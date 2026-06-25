//! Convert a parsed [`SaveFile`] into [`MainStats`] — the account-level values
//! the in-game *Main Stats* export provides — so a save can feed the analyzer's
//! campaign inputs and the growth chamber through the existing
//! `apply_main_stats` consumers, instead of a pasted stats export.
//!
//! Every field below was cross-checked against the same-session Main Stats
//! export paired with the `2026-06-09` reference save (see `tests/to_main_stats.rs`).
//!
//! Two deliberate omissions (left `None`, so selective-fill never clobbers a
//! prior/manual value):
//!
//! - **`challenge_points`** — the export's "Challenge Points" is the *available*
//!   (spendable) balance = `total − used`, and `total` includes Day-challenge
//!   score contributions whose formulas aren't decoded yet. The flat
//!   `Σ(completions × challenge_chp)` is a different (and wrong) quantity for
//!   this field, so we don't auto-fill it from the save.
//! - **`base_growth_per_hour`** — the Main Stats export only exposes the Moai
//!   sum for an all-or-nothing `== 2` inference. We instead read the exact Moai
//!   levels via [`moai_levels`] and set them directly, so this stays `None`.

use itrtg_models::MainStats;

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
        // Available-ChP balance isn't cleanly derivable from the save — see docs.
        challenge_points: None,
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
