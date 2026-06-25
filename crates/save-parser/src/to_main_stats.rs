//! Convert a parsed [`SaveFile`] into [`MainStats`] — the account-level values
//! the in-game *Main Stats* export provides — so a save can feed the analyzer's
//! campaign inputs and the growth chamber through the existing
//! `apply_main_stats` consumers, instead of a pasted stats export.
//!
//! Every field below was cross-checked against the same-session Main Stats
//! export paired with the `2026-06-09` reference save (see `tests/to_main_stats.rs`).
//!
//! One deliberate omission (left `None`, so selective-fill never clobbers a
//! prior/manual value):
//!
//! - **`base_growth_per_hour`** — the Main Stats export only exposes the Moai
//!   sum for an all-or-nothing `== 2` inference. We instead read the exact Moai
//!   levels via [`moai_levels`] and set them directly, so this stays `None`.
//!
//! `challenge_points` *is* derived — it's the **total ChP earned** (the quantity
//! the export reports and the Unicorn / Aether formulas consume), computed as
//! `Σ(non-Day completions × per-completion ChP) + Σ(Day-challenge capped score
//! formula)`. See [`total_challenge_points`].

use itrtg_models::MainStats;

use crate::items;
use crate::model::{SaveFile, trackers};
use crate::tree::Node;

/// `root.x.242` challenge ids (`items::challenge_name`) whose lifetime completion
/// counts feed campaign formulas.
const CHALLENGE_UPC: u32 = 4; // Ultimate Pet Challenge
const CHALLENGE_UCC: u32 = 19; // Ultimate Challenge Challenge → Goblin's UCC term
const CHALLENGE_OC: u32 = 29; // Overflow Challenge → Goblin's OC evo term
const CHALLENGE_PGC: u32 = 31; // Patreon Gods Challenge

/// Per-challenge **max** completion counts — game constants (`aIBCEIHNNME` in
/// `KPLPGPEOFNB`), not stored in the save. The chamber needs the max to know
/// when a challenge is fully complete (PGC at max → the ×1.5 growth jump; UPC's
/// bonus caps well before its max either way).
const UPC_MAX: u32 = 20;
const PGC_MAX: u32 = 25;

/// `X.Q` inventory id (`items::material_name`) for Ants — Ant Queen / Anteater's
/// campaign-bonus input.
const ANT_MATERIAL_ID: u32 = 117;

/// Museum statue id (`items::statue_name`) for the Moai (the Easter-2026
/// commemorative). Each maxed (level 20) Moai grants +1 base growth/hour; the
/// player can own two. Museum statues live at `root.024.f.a` (`a` = level,
/// `b` = statue id).
const MOAI_STATUE_ID: u32 = 11;

/// Build a [`MainStats`] from a parsed save. Fields the save can't faithfully
/// supply are left `None` (see the module docs); a partial result fills what it
/// can, exactly like a sparse text export.
pub fn save_to_main_stats(save: &SaveFile) -> MainStats {
    MainStats {
        pet_stones: save.pet_stones,
        ants: material_count(save, ANT_MATERIAL_ID),
        honey_consumed_by_bear: save
            .global_tracker(trackers::HONEY_CONSUMED_BY_BEAR)
            .map(|v| v.round() as u64),
        challenge_points: Some(total_challenge_points(save)),
        goblin_ucc: Some(challenge_completions(save, CHALLENGE_UCC) as u32),
        goblin_oc: Some(challenge_completions(save, CHALLENGE_OC) as u32),
        ultimate_pet_challenges: Some((challenge_completions(save, CHALLENGE_UPC) as u32, UPC_MAX)),
        patreon_god_challenges: Some((challenge_completions(save, CHALLENGE_PGC) as u32, PGC_MAX)),
        day_pet_challenge_multi: save.global_tracker(trackers::DAY_PET_CHALLENGE_MULTI),
        fish_power: save.root.get_path(&["025", "a"]).and_then(Node::as_f64),
        fishing_level: save
            .root
            .get_path(&["025", "c"])
            .and_then(Node::as_u64)
            .map(|v| v as u32),
        // ChP "Stone Pet improvement" upgrade — stored as "0"/"1" at root.X.035,
        // not "True"/"False", so read it numerically.
        stone_campaign_upgrade: save
            .root
            .get_path(&["X", "035"])
            .and_then(Node::as_u64)
            .map(|v| v != 0),
        earth_eater_planets_text: save
            .global_tracker(trackers::EARTH_EATER_PLANETS_TOTAL)
            .map(|v| (v.round() as u64).to_string()),
        // Read exact Moai levels separately (see `moai_levels`); the text-export
        // `== 2` inference is superseded.
        base_growth_per_hour: None,
    }
}

/// Levels of the player's owned Moai (Easter-2026 Museum statues), in save order
/// — typically up to two. Each is `0..=20`; level 20 = +1 base growth/hour.
///
/// The analyzer models exactly two Moai slots, so callers fill those from the
/// front of this list. Returns empty when none are owned.
pub fn moai_levels(save: &SaveFile) -> Vec<u32> {
    let Some(list) = save.root.get_path(&["024", "f", "a"]) else {
        return Vec::new();
    };
    list.list_or_single()
        .iter()
        .filter(|s| s.get("b").and_then(Node::as_u32) == Some(MOAI_STATUE_ID))
        .filter_map(|s| s.get("a").and_then(Node::as_u32))
        .collect()
}

/// Held count of a material/item id from the `X.Q` inventory. Always `Some`:
/// a material absent from the inventory means a real count of `0` (you own
/// none), not unknown data — matching what the text export reports.
fn material_count(save: &SaveFile, item_id: u32) -> Option<u64> {
    save.materials
        .iter()
        .find(|m| m.item_id == item_id)
        .map(|m| m.count)
        .or(Some(0))
}

/// Lifetime completion count of a challenge id from the `root.x.242` list
/// (`a` = challenge id, `b` = count). `0` when the challenge isn't in the list.
fn challenge_completions(save: &SaveFile, challenge_id: u32) -> u64 {
    let Some(list) = save.root.get_path(&["x", "242"]) else {
        return 0;
    };
    list.list_or_single()
        .iter()
        .filter(|e| e.get("a").and_then(Node::as_u32) == Some(challenge_id))
        .filter_map(|e| e.get("b").and_then(Node::as_u64))
        .sum()
}

/// Total Challenge Points **earned** — the value the export's "Challenge Points"
/// line reports and the Unicorn campaign bonus / Aether plan consume. ChP is a
/// derived sum (never stored): the flat-rate challenges pay per completion, the
/// Day challenges pay a single capped amount based on their best score. The
/// result is the *total earned*, not the spendable balance — spending ChP on
/// upgrades doesn't reduce it. Cross-checked to the export's 751 for the
/// 2026-06-09 save (`tests/to_main_stats.rs`).
fn total_challenge_points(save: &SaveFile) -> u64 {
    // Our transcribed Day-challenge constants (e.g. 6.67) carry sub-ChP error
    // vs. the game's exact internals, so round the derived sum rather than floor.
    (non_day_challenge_chp(save) + day_challenge_chp(save)).round() as u64
}

/// Σ over the `x.242` list of `completions × per-completion ChP` for the
/// flat-rate challenges. [`items::challenge_chp`] returns `None` for the
/// score-based Day challenges, so they're skipped here (see [`day_challenge_chp`]).
fn non_day_challenge_chp(save: &SaveFile) -> f64 {
    let Some(list) = save.root.get_path(&["x", "242"]) else {
        return 0.0;
    };
    list.list_or_single()
        .iter()
        .filter_map(|e| {
            let id = e.get("a").and_then(Node::as_u32)?;
            let count = e.get("b").and_then(Node::as_u64)?;
            Some(count as f64 * items::challenge_chp(id)? as f64)
        })
        .sum()
}

/// Σ of each Day challenge's score-based ChP. Each Day challenge pays a single
/// amount from its **highest score** (a stat at `root.x.<key>`) run through the
/// game's per-challenge formula and cap — completion *count* is irrelevant.
/// A 0/absent score contributes 0.
///
/// Three Day challenges whose score key isn't located yet — No Rebirth (41),
/// God Power (52), Multiverse (54) — are omitted and contribute 0 until a save
/// with non-zero completions surfaces their keys.
fn day_challenge_chp(save: &SaveFile) -> f64 {
    // Highest-score stat for a Day challenge, clamped non-negative.
    let score = |key: &str| save.global_tracker(key).unwrap_or(0.0).max(0.0);
    let capped = |v: f64, cap: f64| v.clamp(0.0, cap);
    // log2 is only meaningful for scores ≥ 1; below that the challenge is untouched.
    let log2 = |x: f64| if x >= 1.0 { x.log2() } else { 0.0 };

    // (challenge name, score key) — formulas transcribed from the game, each capped.
    capped(3.0 * score("045"), 666.0)               // Day Baal: 3 × strongest P.Baal
        + capped(log2(score("047")) * 8.0, 666.0)   // Day Universe: log2(universes) × 8
        + capped(log2(score("049")) * 6.67, 666.0)  // Day Pet: log2(pet multi) × 6.67
        + capped(score("065").sqrt() / 1.6, 666.0)  // Day Might: sqrt(might) / 1.6
        + capped(score("068").sqrt() / 100.0, 666.0) // Day No Divinity: sqrt(points) / 100
        + capped(5.0 * score("134"), 2000.0)        // Road to Infinity: 5 × strongest P.Baal
        + capped(score("304").powf(0.15) * 35.0, 666.0) // Day Extreme Building: points^0.15 × 35
}
