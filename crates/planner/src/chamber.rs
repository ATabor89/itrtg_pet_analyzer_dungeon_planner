//! Growth-chamber state and logic, shared by every front end.
//!
//! The chamber locks a set of pets into the Growth campaign and runs the
//! multi-cycle simulation ([`crate::campaign::simulate_growth_chamber`]). This
//! module owns everything that isn't rendering: the persisted configuration
//! ([`ChamberState`]), the bridge from the real roster to the sim
//! ([`build_roster`]), the per-pet what-if overrides, and the text report of a
//! finished run ([`report`]). **All unlocked pets** go into the sim — those not
//! in the chamber are benched, so Bag's gift can still reach the global lowest.

use std::collections::BTreeMap;

use itrtg_models::{
    pgc_growth_mult, CampaignType, Equipment, ExportPet, Loadout, Quality, MAGIC_EGG_GROWTH_MULT,
};
use serde::{Deserialize, Serialize};

use crate::campaign::{
    simulate_growth_chamber, ChamberClass, ChamberCycle, ChamberPet, ChamberResult, ChamberRun,
    SpecialPet, TargetStatus,
};
use crate::growth::GrowthRates;
use crate::merge::{CampaignBonusBreakdown, CampaignContext, MergedPet};

/// Persisted growth-chamber configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ChamberState {
    /// Canonical names of the pets in the chamber (campaign participants).
    pub chamber: Vec<String>,
    /// Per-pet target growth (canonical name → target). A pet with a target is
    /// "tracked" — the sim reports the cycle it reaches it.
    pub targets: BTreeMap<String, u64>,
    /// Campaign length, 1–12 h.
    pub hours: u32,
    /// Maximum cycles to simulate before giving up.
    pub max_cycles: u32,
    /// Pandora's Box feedings — drives its campaign-total bonus. (The feeding
    /// behaviour over many rounds is still being pinned down; this is the hook.)
    pub pandora_feedings: u32,
    /// Ultimate Pet Challenges completed — each adds +5% to every pet's
    /// campaign contribution (capped at 100%). Auto-filled from a Main-stats
    /// export.
    pub upc_done: u32,
    /// Total Ultimate Pet Challenges available (the export's `/ <max>` side —
    /// 20 today; drives the input's range).
    pub upc_max: u32,
    /// Patreon God Challenges completed — +1% growth per completion, doubled
    /// once all are done (24/25 → ×1.24, 25/25 → ×1.50; stacks with the Magic
    /// Egg). Auto-filled from a Main-stats export.
    pub pgc_done: u32,
    /// Total Patreon God Challenges available (the export's `/ <max>` side —
    /// 25 today; drives the completion doubling and the input's range).
    pub pgc_max: u32,
    /// Which food is fed to every pet — index into [`FOODS`]. Drives per-feeding
    /// growth (every pet is fed `floor(hours/3)` times per cycle).
    pub food_choice: usize,
    /// Day Pet Challenge highest pet multiplier (a percent, e.g. `3.664e9`). Drives
    /// the food DPC boost `log2(multi)` (capped 100%). Auto-filled from the
    /// Main-stats export; 0 = no DPC boost.
    pub dpc_highest_multi: f64,
    /// Food fed to **Gold Dragon** (index into [`FOODS`]); `None` = not fed / not
    /// owned. Feeding him gives **every** pet 25% of the growth he gains — a big,
    /// campaign-independent source. Best fed chocolate.
    pub gold_dragon_food: Option<usize>,
    /// Run until every tracked pet hits its target (vs a fixed cycle count).
    pub run_until_targets: bool,
    /// The export was captured right at a campaign's end (so its growth already
    /// includes that campaign's passive/Moai). When set, the first simulated
    /// cycle adds no passive growth — avoids double-counting it.
    pub exported_after_campaign: bool,
    /// Model rebirths — shorten each rebirth's last cycle so a campaign never
    /// spans a rebirth. Off ⭢ uniform cycles.
    pub rebirth_enabled: bool,
    /// Average rebirth length, in [`REBIRTH_UNITS`] of `rebirth_unit` (decimal —
    /// e.g. 7.5 days).
    pub rebirth_value: f64,
    /// Index into [`REBIRTH_UNITS`] (Hours / Days / Weeks).
    pub rebirth_unit: usize,
    /// Fish Power — drives the food boost that decays over a rebirth's first 30 h.
    /// 0 = no fishing boost.
    pub fish_power: f64,
    /// Fishing level — adds the +10% milestones (at 15 and 27) to the boost.
    pub fishing_level: u32,
    /// Multiplier on Adventurer campaign class XP — the "pet stone / ChP
    /// Adventurer-XP" purchases applied after the base formula. `1.0` = none;
    /// `0.0` turns class-level modelling off entirely. The pet-stone component is
    /// save-file-only (no export carries it), so this is a manual input.
    #[serde(default = "default_adv_xp_mult")]
    pub adv_xp_mult: f64,
    /// Per-pet what-if overrides (canonical name → tweaked loadout + CL). Absent
    /// means "use the live export as-is". Seeded from the export on first edit;
    /// the card's "Refresh from export" button drops the entry to revert.
    pub overrides: BTreeMap<String, PetOverride>,
    /// Snapshot of the global inputs as last auto-filled from a Main-stats
    /// export. Powers the global "Reset to import" button — shown when any of
    /// the imported inputs has been edited away from its snapshot value.
    pub imported: ImportedStats,

    /// Pet-picker search filter — ephemeral.
    #[serde(skip)]
    pub search: String,
    /// Last simulation result — ephemeral.
    #[serde(skip)]
    pub result: Option<ChamberResult>,
    /// In-chamber pets' growth at the last run's start — ephemeral, for the
    /// before→after report.
    #[serde(skip)]
    pub last_starts: BTreeMap<String, f64>,
    /// Last frame's natural card-content height per card-row — ephemeral. Used to
    /// pad shorter cards up to the tallest in their row so a row stays flush even
    /// when one card grows (e.g. an "edited" card gains the Refresh control).
    #[serde(skip)]
    pub row_heights: Vec<f32>,
}

/// A per-pet what-if override of the sim inputs the user can tweak on a card.
/// Seeded from the pet's export on first edit, so an override always carries the
/// pet's full effective loadout + CL; "Refresh from export" removes it entirely
/// (reverting to the live export). Persisted so a tuned chamber survives reloads.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PetOverride {
    /// Effective equipment (overrides the export's loadout). Phase 2a leaves this
    /// equal to the export; the gear editors that mutate it arrive in phase 2b.
    pub loadout: Loadout,
    /// Effective class level (overrides the export's CL).
    pub class_level: u32,
}

/// The chamber inputs a Main-stats import auto-fills, as last imported.
/// `None` per field = that field has never been imported (so user edits to it
/// can't "deviate" and don't trigger the reset button). Kept up to date by
/// [`ChamberState::apply_main_stats`]; later imports only overwrite the fields
/// they carry, matching how the fill itself works.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImportedStats {
    /// `(done, max)` Ultimate Pet Challenges.
    pub upc: Option<(u32, u32)>,
    /// `(done, max)` Patreon God Challenges.
    pub pgc: Option<(u32, u32)>,
    pub dpc_highest_multi: Option<f64>,
    pub fish_power: Option<f64>,
    pub fishing_level: Option<u32>,
}

/// Serde default for [`ChamberState::adv_xp_mult`] — 1.0 (no purchases), so an
/// older persisted state that predates the field models class XP at the base
/// rate rather than disabling it (which 0.0 would do).
fn default_adv_xp_mult() -> f64 {
    1.0
}

/// Food types, lowest→highest growth.
pub const FOODS: [&str; 5] = ["Free", "Puny", "Strong", "Mighty", "Chocolate"];

/// Base growth per feeding for each [`FOODS`] type (before DPC/fishing boosts).
pub const BASE_FOOD: [f64; 5] = [0.75, 1.5, 3.0, 4.5, 6.0];

/// Rebirth-length units `(label, hours)`. Index matches [`ChamberState::rebirth_unit`].
pub const REBIRTH_UNITS: [(&str, u32); 3] = [("Hours", 1), ("Days", 24), ("Weeks", 168)];

impl Default for ChamberState {
    fn default() -> Self {
        Self {
            chamber: Vec::new(),
            targets: BTreeMap::new(),
            hours: 12,
            max_cycles: 5_000,
            pandora_feedings: 0,
            upc_done: 0,
            upc_max: 20,
            pgc_done: 0,
            pgc_max: 25,
            food_choice: 4, // Chocolate
            dpc_highest_multi: 0.0,
            gold_dragon_food: None,
            run_until_targets: false,
            exported_after_campaign: false,
            rebirth_enabled: false,
            rebirth_value: 24.0,
            rebirth_unit: 0, // Hours
            fish_power: 0.0,
            fishing_level: 0,
            adv_xp_mult: 1.0,
            overrides: BTreeMap::new(),
            imported: ImportedStats::default(),
            search: String::new(),
            result: None,
            last_starts: BTreeMap::new(),
            row_heights: Vec::new(),
        }
    }
}

impl ChamberState {
    /// Load the persisted fields from a stored copy, sanitizing indices and
    /// counts the way every loaded value is (the ephemeral fields are left alone).
    pub fn load_persisted(&mut self, src: &ChamberState) {
        self.chamber = src.chamber.clone();
        self.targets = src.targets.clone();
        self.hours = src.hours;
        self.max_cycles = src.max_cycles;
        self.pandora_feedings = src.pandora_feedings;
        // Sanitize like the other loaded indices: done can't exceed max (for
        // PGC a hand-edited value would otherwise apply a huge multiplier; for
        // UPC the % caps anyway, so this just keeps the input range honest).
        self.upc_max = src.upc_max;
        self.upc_done = src.upc_done.min(src.upc_max);
        self.pgc_max = src.pgc_max;
        self.pgc_done = src.pgc_done.min(src.pgc_max);
        self.food_choice = src.food_choice.min(FOODS.len() - 1);
        self.dpc_highest_multi = src.dpc_highest_multi;
        self.gold_dragon_food = src.gold_dragon_food.filter(|&i| i < FOODS.len());
        self.run_until_targets = src.run_until_targets;
        self.exported_after_campaign = src.exported_after_campaign;
        self.rebirth_enabled = src.rebirth_enabled;
        self.rebirth_value = src.rebirth_value;
        self.rebirth_unit = src.rebirth_unit.min(REBIRTH_UNITS.len() - 1);
        self.fish_power = src.fish_power;
        self.fishing_level = src.fishing_level;
        self.adv_xp_mult = src.adv_xp_mult;
        self.overrides = src.overrides.clone();
        self.imported = src.imported.clone();
    }

    /// A copy holding only the persisted fields (drops the ephemeral
    /// search/result/last_starts).
    pub fn persisted(&self) -> ChamberState {
        ChamberState {
            chamber: self.chamber.clone(),
            targets: self.targets.clone(),
            hours: self.hours,
            max_cycles: self.max_cycles,
            pandora_feedings: self.pandora_feedings,
            upc_done: self.upc_done,
            upc_max: self.upc_max,
            pgc_done: self.pgc_done,
            pgc_max: self.pgc_max,
            food_choice: self.food_choice,
            dpc_highest_multi: self.dpc_highest_multi,
            gold_dragon_food: self.gold_dragon_food,
            run_until_targets: self.run_until_targets,
            exported_after_campaign: self.exported_after_campaign,
            rebirth_enabled: self.rebirth_enabled,
            rebirth_value: self.rebirth_value,
            rebirth_unit: self.rebirth_unit,
            fish_power: self.fish_power,
            fishing_level: self.fishing_level,
            adv_xp_mult: self.adv_xp_mult,
            overrides: self.overrides.clone(),
            imported: self.imported.clone(),
            ..Default::default()
        }
    }


    /// Auto-fill the chamber's global inputs from a Main-stats export: the UPC
    /// and PGC completion counts, the DPC food multiplier, and Fish Power /
    /// Fishing Level. Returns the filled field labels for the import status.
    pub fn apply_main_stats(&mut self, ms: &itrtg_models::MainStats) -> Vec<&'static str> {
        let mut filled = Vec::new();
        if let Some((done, max)) = ms.ultimate_pet_challenges {
            self.upc_done = done;
            self.upc_max = max;
            self.imported.upc = Some((done, max));
            filled.push("UPC");
        }
        if let Some((done, max)) = ms.patreon_god_challenges {
            self.pgc_done = done;
            self.pgc_max = max;
            self.imported.pgc = Some((done, max));
            filled.push("PGC");
        }
        if let Some(multi) = ms.day_pet_challenge_multi {
            self.dpc_highest_multi = multi;
            self.imported.dpc_highest_multi = Some(multi);
            filled.push("DPC multi");
        }
        if let Some(fp) = ms.fish_power {
            self.fish_power = fp;
            self.imported.fish_power = Some(fp);
            filled.push("Fish Power");
        }
        if let Some(level) = ms.fishing_level {
            self.fishing_level = level;
            self.imported.fishing_level = Some(level);
            filled.push("Fishing level");
        }
        filled
    }

    /// True when any global input auto-filled from a Main-stats import has been
    /// edited away from its imported value. Never-imported fields don't count.
    pub fn deviates_from_import(&self) -> bool {
        self.imported.upc.is_some_and(|v| v != (self.upc_done, self.upc_max))
            || self.imported.pgc.is_some_and(|v| v != (self.pgc_done, self.pgc_max))
            || self.imported.dpc_highest_multi.is_some_and(|v| v != self.dpc_highest_multi)
            || self.imported.fish_power.is_some_and(|v| v != self.fish_power)
            || self.imported.fishing_level.is_some_and(|v| v != self.fishing_level)
    }

    /// Restore every imported global input to its imported value (the global
    /// counterpart of a card's "Refresh from export"). Never-imported fields
    /// keep their current values.
    pub fn reset_to_import(&mut self) {
        if let Some((done, max)) = self.imported.upc {
            self.upc_done = done;
            self.upc_max = max;
        }
        if let Some((done, max)) = self.imported.pgc {
            self.pgc_done = done;
            self.pgc_max = max;
        }
        if let Some(multi) = self.imported.dpc_highest_multi {
            self.dpc_highest_multi = multi;
        }
        if let Some(fp) = self.imported.fish_power {
            self.fish_power = fp;
        }
        if let Some(level) = self.imported.fishing_level {
            self.fishing_level = level;
        }
    }

    /// The global PGC growth multiplier (×1 with no completions).
    pub fn pgc_mult(&self) -> f64 {
        pgc_growth_mult(self.pgc_done, self.pgc_max)
    }

    /// The UPC campaign bonus %: `5 ·` completions, capped at 100%.
    pub fn upc_pct(&self) -> f64 {
        (5.0 * self.upc_done as f64).min(100.0)
    }

    /// The DPC food boost %: `log2(highest pet multiplier)`, capped at 100%. 0 when
    /// there's no multiplier (≤ 1, where the log is ≤ 0).
    pub fn dpc_boost_pct(&self) -> f64 {
        if !self.dpc_highest_multi.is_finite() || self.dpc_highest_multi <= 1.0 {
            return 0.0;
        }
        self.dpc_highest_multi.log2().clamp(0.0, 100.0)
    }

    /// The steady (no-fishing) growth per feeding for a food type: base × the DPC
    /// boost. Fishing is layered on top by the sim (it decays over the rebirth).
    pub fn food_value(&self, idx: usize) -> f64 {
        BASE_FOOD.get(idx).copied().unwrap_or(0.0) * (1.0 + self.dpc_boost_pct() / 100.0)
    }

    /// Effective steady growth per feeding for the chosen food.
    pub fn food_growth(&self) -> f64 {
        self.food_value(self.food_choice)
    }

    /// Average rebirth length in **whole** hours (value × unit, truncated — a
    /// partial hour can't run another campaign, so it's dropped).
    pub fn rebirth_total_hours(&self) -> u32 {
        let factor = REBIRTH_UNITS.get(self.rebirth_unit).map_or(1, |&(_, h)| h) as f64;
        ((self.rebirth_value * factor).floor() as i64).clamp(1, u32::MAX as i64) as u32
    }

    /// The Fish Power food boost % at a rebirth's start (0 if no Fish Power).
    pub fn fishing_boost(&self) -> f64 {
        crate::campaign::fishing_boost_pct(self.fish_power, self.fishing_level)
    }

    /// Per-feeding growth every pet gets from a Gold Dragon feeding (25% of his
    /// food's growth). 0 if Gold Dragon isn't being fed.
    pub fn gold_dragon_broadcast(&self) -> f64 {
        self.gold_dragon_food.map_or(0.0, |i| 0.25 * self.food_value(i))
    }
}

/// Build the full sim roster from every unlocked pet: those in `state.chamber`
/// are participants, the rest are benched (so Bag's gift can reach the global
/// lowest). Pulls each pet's Growth bonus (equipment + class included via `ctx`)
/// and passive growth/hour (Moai + pendant).
pub fn build_roster(
    pets: &[MergedPet],
    ctx: &CampaignContext,
    rates: &GrowthRates,
    state: &ChamberState,
) -> Vec<ChamberPet> {
    pets.iter()
        .filter(|p| p.is_unlocked())
        .filter_map(|p| chamber_pet(p, ctx, rates, state))
        .collect()
}

/// The pet's *effective* export for the sim: the real export with any per-pet
/// override (loadout + CL) applied. `None` if the pet has no export data.
///
/// The override is what powers the card's what-if editing — swapping the loadout
/// and CL here means every downstream read (Magic Egg multiplier, pendant
/// passive, and the recomputed Growth bonus) reflects the edits, with no engine
/// change: the bonus is recomputed by feeding this synthetic export back through
/// the existing [`MergedPet::campaign_bonus_breakdown`] seam.
pub fn effective_export(pet: &MergedPet, state: &ChamberState) -> Option<ExportPet> {
    let base = pet.export.as_ref()?;
    let mut eff = base.clone();
    if let Some(ov) = state.overrides.get(&pet.name) {
        eff.loadout = ov.loadout.clone();
        eff.class_level = ov.class_level;
    }
    Some(eff)
}

pub fn chamber_pet(
    pet: &MergedPet,
    ctx: &CampaignContext,
    rates: &GrowthRates,
    state: &ChamberState,
) -> Option<ChamberPet> {
    let export = effective_export(pet, state)?;
    // Base growth is the accumulator; the Magic Egg (+30%) and the global PGC
    // (+1% per completion, doubled at full completion) multiply it into the
    // total growth the campaign reads.
    let growth = export.growth as f64;
    let growth_multiplier =
        if export.has_magic_egg() { MAGIC_EGG_GROWTH_MULT } else { 1.0 } * state.pgc_mult();
    // Recompute the Growth bonus from the *effective* export so loadout/CL edits
    // show live. Goes through the same breakdown seam the card's source line
    // uses, so the total and its split can never disagree.
    let bd = growth_bonus_breakdown(pet, ctx, state);
    let bonus = bd.total().get(&CampaignType::Growth).copied().unwrap_or(0.0);
    let mut passive = rates.moai_per_hour;
    // The pendant is just the equipped accessory — no separate toggle.
    if export.loadout.accessory.as_ref().is_some_and(|a| a.name == "Growing Love Pendant") {
        passive += rates.pendant_per_hour();
    }
    let special = match pet.name.as_str() {
        "Pandora's Box" => Some(SpecialPet::Pandora { feedings: state.pandora_feedings }),
        // The flat layers of Bag's Growth bonus (equipment + class) ride along;
        // the sim re-evaluates his innate lowest-growth term each cycle, since
        // the global lowest pet keeps growing during the run.
        "Bag" => Some(SpecialPet::Bag {
            token_improved: export.improved,
            flat_bonus_pct: bd.equipment.unwrap_or(0.0) + bd.class.unwrap_or(0.0),
        }),
        // Reduces every other chamber pet's campaign bonus by (20 − 0.25·CL) pts.
        "Nightmare" => Some(SpecialPet::Nightmare { class_level: export.class_level }),
        _ => None,
    };
    // Class-XP state — only Adventurers earn campaign class XP. The per-level
    // slope (2 + evo) comes from the same merge seam the bonus split uses, so the
    // mid-run level-ups raise the bonus by exactly what the card would show. The
    // starting level/exp come from the (override-aware) effective export; exp is
    // 0 until a save import populates it.
    let class = pet.adventurer_per_level_bonus(ctx).map(|per_cl| ChamberClass {
        level: export.class_level,
        exp: export.class_exp,
        bonus_per_cl: per_cl,
    });
    Some(ChamberPet {
        name: pet.name.clone(),
        growth,
        growth_multiplier,
        campaign_bonus_pct: bonus,
        passive_per_hour: passive,
        food_per_feeding: state.food_growth(),
        gold_dragon_per_feeding: state.gold_dragon_broadcast(),
        target: state.targets.get(&pet.name).map(|&t| t as f64),
        in_chamber: state.chamber.iter().any(|n| n == &pet.name),
        special,
        class,
    })
}

/// The pet's Growth-bonus source split (innate / class / equipment), computed
/// from the same override-aware effective export as the rest of the card. With
/// no override this is just the pet's own breakdown; the synthetic pet (and the
/// wiki clone) is only built when an override is actually in play.
pub fn growth_bonus_breakdown(
    pet: &MergedPet,
    ctx: &CampaignContext,
    state: &ChamberState,
) -> CampaignBonusBreakdown {
    if state.overrides.contains_key(&pet.name) {
        let synth = MergedPet {
            name: pet.name.clone(),
            wiki: pet.wiki.clone(),
            export: effective_export(pet, state),
        };
        synth.campaign_bonus_breakdown(ctx)
    } else {
        pet.campaign_bonus_breakdown(ctx)
    }
}

/// Run the simulation with the current settings, storing the result (and the
/// in-chamber pets' starting growth, which the report reads) on `state`.
pub fn run(state: &mut ChamberState, pets: &[MergedPet], ctx: &CampaignContext, rates: &GrowthRates) {
    // Lock in the in-chamber pets' starting growth (the report reads this,
    // so it won't shift when the selection changes before the next run).
    state.last_starts = pets
        .iter()
        .filter(|p| state.chamber.iter().any(|c| c == &p.name))
        .filter_map(|p| p.export.as_ref().map(|e| (p.name.clone(), e.effective_growth() as f64)))
        .collect();
    let mut roster = build_roster(pets, ctx, rates, state);
    state.result = Some(simulate_growth_chamber(
        &mut roster,
        &ChamberRun {
            hours: state.hours,
            upc_pct: state.upc_pct(),
            max_cycles: state.max_cycles,
            stop_at_targets: state.run_until_targets,
            skip_first_cycle_passive: state.exported_after_campaign,
            rebirth_hours: state.rebirth_enabled.then(|| state.rebirth_total_hours()),
            fishing_boost_pct: state.fishing_boost(),
            adv_xp_mult: state.adv_xp_mult,
        },
    ));
}

/// Nightmare's team malus, if he's in the chamber — the points he docks from
/// every *other* pet's campaign bonus. Display-only (the sim applies it to the
/// bonuses itself); computed from his *effective* class level, so it tracks
/// add/remove and the CL editor live.
pub fn chamber_nightmare_malus(state: &ChamberState, pets: &[MergedPet]) -> Option<f32> {
    if !state.chamber.iter().any(|n| n == "Nightmare") {
        return None;
    }
    pets.iter()
        .find(|p| p.name == "Nightmare")
        .and_then(|p| effective_export(p, state))
        .map(|e| crate::campaign::nightmare_malus(e.class_level) as f32)
}

/// Drop stale chamber entries (see [`chamber_entry_issue`]). A roster-missing
/// pet's target/override are equally unreachable from the UI — they go with it
/// so they don't linger in the stored state.
pub fn remove_entries(state: &mut ChamberState, pets: &[MergedPet], removed: &[String]) {
    if removed.is_empty() {
        return;
    }
    state.chamber.retain(|n| !removed.contains(n));
    for n in removed {
        if !pets.iter().any(|p| &p.name == n) {
            state.targets.remove(n);
            state.overrides.remove(n);
        }
    }
}

/// Why a chamber entry can't participate in a run, if it can't (`None` = fine).
/// `pet` is the roster lookup for the entry's name — `None` when no merged pet
/// has that name anymore (re-import from another account, wiki rename).
pub fn chamber_entry_issue(pet: Option<&MergedPet>) -> Option<&'static str> {
    match pet {
        None => Some("not in the imported roster"),
        // Before the lock check: is_unlocked() is also false with no export.
        Some(p) if p.export.is_none() => Some("no export data"),
        Some(p) if !p.is_unlocked() => Some("locked — excluded from the sim"),
        Some(_) => None,
    }
}

/// Get (seeding if absent) the mutable override for a pet. Seeded from the given
/// (effective) export so a fresh override carries the pet's full loadout + CL;
/// every card editor funnels through here so the first edit of any field
/// captures the rest of the export unchanged.
pub fn override_mut<'a>(
    state: &'a mut ChamberState,
    name: &str,
    eff: &ExportPet,
) -> &'a mut PetOverride {
    state.overrides.entry(name.to_string()).or_insert_with(|| PetOverride {
        loadout: eff.loadout.clone(),
        class_level: eff.class_level,
    })
}

/// Set a pet's effective class level via its override.
pub fn set_class_level(state: &mut ChamberState, name: &str, eff: &ExportPet, cl: u32) {
    override_mut(state, name, eff).class_level = cl;
}

/// The four campaign sticks, weakest→strongest (the order their caps imply).
pub const STICKS: [&str; 4] =
    ["Walking Stick", "Journeying Stick", "Magic Stick", "Legendary Stick"];

/// Quality grades, weakest→strongest, for the stick quality picker.
pub const QUALITIES: [Quality; 9] = [
    Quality::F,
    Quality::E,
    Quality::D,
    Quality::C,
    Quality::B,
    Quality::A,
    Quality::S,
    Quality::SS,
    Quality::SSS,
];

/// Build a piece of equipment with just the fields the growth sim cares about
/// (name + quality + upgrade); enchant/gem are irrelevant to campaign bonuses.
pub fn equip(name: &str, quality: Quality, upgrade: Option<u8>) -> Equipment {
    Equipment {
        name: name.to_string(),
        upgrade_level: upgrade,
        quality,
        enchant_level: None,
        gem: None,
        gem_level: None,
    }
}

/// Is this equipment one of the four campaign sticks?
pub fn is_stick(item: &Equipment) -> bool {
    STICKS.contains(&item.name.as_str())
}

/// The weapon a slot should hold when the user picks `name` from the weapon
/// menu, given what's currently equipped. Switching *between* sticks preserves
/// the current quality/upgrade; everything else gets its effective level (event
/// gear only does anything at SSS+20; the Magic Egg's level is irrelevant).
/// `name == "none"` clears the slot.
pub fn weapon_for(name: &str, current: Option<&Equipment>) -> Option<Equipment> {
    match name {
        "none" => None,
        "Magic Egg" => Some(equip("Magic Egg", Quality::SSS, None)),
        "Candy Cane" => Some(equip("Candy Cane", Quality::SSS, Some(20))),
        s if STICKS.contains(&s) => {
            // Carry quality/upgrade across stick swaps; otherwise default to a
            // maxed SSS+20 (the common event-stick state and the natural ceiling).
            let (q, u) = current
                .filter(|c| is_stick(c))
                .map(|c| (c.quality, c.upgrade_level.unwrap_or(20)))
                .unwrap_or((Quality::SSS, 20));
            Some(equip(s, q, Some(u)))
        }
        _ => current.cloned(),
    }
}

/// The label shown for a weapon in the picker header: the known kind, the raw
/// name for anything unrecognized, or `none`.
pub fn weapon_kind_label(item: Option<&Equipment>) -> String {
    match item {
        None => "none".to_string(),
        Some(e) => e.name.clone(),
    }
}

/// Apply a weapon-menu pick to a pet's override.
pub fn set_weapon(state: &mut ChamberState, name: &str, eff: &ExportPet, choice: &str) {
    let new = weapon_for(choice, eff.loadout.weapon.as_ref());
    override_mut(state, name, eff).loadout.weapon = new;
}

/// Weapon-menu choices, in display order: none, the four sticks, Candy Cane,
/// Magic Egg.
pub const WEAPON_CHOICES: [&str; 7] = [
    "none",
    "Walking Stick",
    "Journeying Stick",
    "Magic Stick",
    "Legendary Stick",
    "Candy Cane",
    "Magic Egg",
];

/// Armor-menu choices: Merry Mantle (SSS+20) or none.
pub const ARMOR_CHOICES: [&str; 2] = ["none", "Merry Mantle"];

/// Accessory-menu choices: Growing Love Pendant (passive growth), Christmas
/// Boots (SSS+20 campaign boost), or none.
pub const ACCESSORY_CHOICES: [&str; 3] = ["none", "Growing Love Pendant", "Christmas Boots"];

/// Apply an armor-menu pick to a pet's override.
pub fn set_armor(state: &mut ChamberState, name: &str, eff: &ExportPet, choice: &str) {
    override_mut(state, name, eff).loadout.armor = match choice {
        "Merry Mantle" => Some(equip("Merry Mantle", Quality::SSS, Some(20))),
        _ => None,
    };
}

/// Apply an accessory-menu pick to a pet's override.
pub fn set_accessory(state: &mut ChamberState, name: &str, eff: &ExportPet, choice: &str) {
    override_mut(state, name, eff).loadout.accessory = match choice {
        // The pendant gives passive growth, not a campaign boost — its level
        // is irrelevant, so a plain SSS is fine.
        "Growing Love Pendant" => Some(equip("Growing Love Pendant", Quality::SSS, None)),
        "Christmas Boots" => Some(equip("Christmas Boots", Quality::SSS, Some(20))),
        _ => None,
    };
}

/// Set a stick weapon's quality via the pet's override (no-op without a weapon).
pub fn set_weapon_quality(state: &mut ChamberState, name: &str, eff: &ExportPet, quality: Quality) {
    if let Some(e) = override_mut(state, name, eff).loadout.weapon.as_mut() {
        e.quality = quality;
    }
}

/// Set a stick weapon's upgrade level via the pet's override (no-op without a weapon).
pub fn set_weapon_upgrade(state: &mut ChamberState, name: &str, eff: &ExportPet, upgrade: u8) {
    if let Some(e) = override_mut(state, name, eff).loadout.weapon.as_mut() {
        e.upgrade_level = Some(upgrade);
    }
}

/// Set (or, with 0, clear) a pet's growth target.
pub fn set_target(state: &mut ChamberState, name: &str, target: u64) {
    if target == 0 {
        state.targets.remove(name);
    } else {
        state.targets.insert(name.to_string(), target);
    }
}

/// Tick a pet into or out of the chamber. Adding respects the 10-pet cap;
/// returns whether the membership changed.
pub fn set_membership(state: &mut ChamberState, name: &str, in_chamber: bool) -> bool {
    let was = state.chamber.iter().any(|n| n == name);
    if in_chamber && !was && state.chamber.len() < 10 {
        state.chamber.push(name.to_string());
        true
    } else if !in_chamber && was {
        state.chamber.retain(|n| n != name);
        true
    } else {
        false
    }
}

/// One pet-picker entry: an unlocked pet with export data.
pub struct PickerEntry<'a> {
    pub pet: &'a MergedPet,
    /// Growth-campaign bonus %.
    pub bonus: f32,
    /// Effective (total) growth.
    pub growth: u64,
}

/// The pet picker's rows: unlocked pets whose name contains `needle`
/// (case-insensitive), highest Growth bonus first — what you'd want in a mature
/// chamber — with growth as the tiebreak.
pub fn picker_entries<'a>(
    pets: &'a [MergedPet],
    ctx: &CampaignContext,
    needle: &str,
) -> Vec<PickerEntry<'a>> {
    let needle = needle.to_lowercase();
    let mut out: Vec<PickerEntry<'a>> = pets
        .iter()
        .filter(|p| p.is_unlocked() && p.export.is_some())
        .filter(|p| needle.is_empty() || p.name.to_lowercase().contains(&needle))
        .map(|p| PickerEntry {
            pet: p,
            bonus: p.campaign_bonus_for(CampaignType::Growth, ctx).unwrap_or(0.0),
            growth: p.export.as_ref().map(|e| e.effective_growth()).unwrap_or(0),
        })
        .collect();
    out.sort_by(|a, b| {
        b.bonus
            .partial_cmp(&a.bonus)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.growth.cmp(&a.growth))
    });
    out
}

/// Fill the chamber with the top 10 unlocked pets by Growth-campaign bonus,
/// breaking ties by raw growth (both highest-first).
pub fn recommend_chamber(state: &mut ChamberState, pets: &[MergedPet], ctx: &CampaignContext) {
    let mut ranked: Vec<(&str, f32, u64)> = pets
        .iter()
        .filter(|p| p.is_unlocked() && p.export.is_some())
        .map(|p| {
            (
                p.name.as_str(),
                p.campaign_bonus_for(CampaignType::Growth, ctx).unwrap_or(0.0),
                p.export.as_ref().map(|e| e.effective_growth()).unwrap_or(0),
            )
        })
        .collect();
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.2.cmp(&a.2))
    });
    state.chamber = ranked.iter().take(10).map(|(n, _, _)| n.to_string()).collect();
}

/// Friendly hours: plain hours, a day approximation once it's a few days, and
/// years once it's a long way out (for target estimates).
pub fn fmt_hours(h: u32) -> String {
    let days = h as f64 / 24.0;
    if days >= 365.0 {
        format!("~{:.1} yr", days / 365.0)
    } else if h >= 72 {
        format!("~{h} h \u{2248} {days:.1} d")
    } else {
        format!("~{h} h")
    }
}

/// A pet's estimated growth per cycle over the run's **recent** cycles, in
/// total terms. The whole-run average lags the true rate — campaign rewards
/// grow as the roster grows — so extrapolating from it lands the ETA late
/// (~3 cycles over a long run, per validation). `None` when the run is too
/// short to have a meaningfully "recent" window (caller falls back to the
/// whole-run average).
///
/// Recent rate = the pet's campaign income (recipient deposits + Bag gifts)
/// over the last `window` cycles, plus its per-cycle non-campaign income
/// (passive/feeding/Gold Dragon — near-uniform across cycles). The trace
/// records campaign income in *base* terms; the run-wide campaign breakdown
/// (total terms) rescales it exactly, since the pet's multiplier is constant.
pub fn recent_rate_per_cycle(result: &ChamberResult, name: &str) -> Option<f64> {
    let n = result.trace.len();
    let idx = result.final_growth.iter().position(|(p, _)| p == name)?;
    let breakdown = &result.breakdown.get(idx)?.1;

    // Window: two recipient rotations (the chamber size), so it spans at least
    // one full deposit cadence even as the recipient leapfrogs around. If that
    // window doesn't fit in half the run (so "recent" would neither span a
    // rotation nor be distinct from the whole-run average), bail to the
    // fallback — a truncated window could miss a pet's deposits entirely and
    // report a misleading campaign-free rate.
    let chamber_size = result.trace.first().map_or(0, |c| c.contributions.len());
    let window = (2 * chamber_size).max(1);
    if window > n / 2 {
        return None;
    }

    // Campaign income to this pet in one cycle, in base terms.
    let base_income = |c: &ChamberCycle| -> f64 {
        let deposit = if c.recipient == idx { c.recipient_gain } else { 0.0 };
        let gift = match c.bag_gift {
            Some((to, amt)) if to == idx => amt,
            _ => 0.0,
        };
        deposit + gift
    };
    let run_base: f64 = result.trace.iter().map(base_income).sum();
    let recent_base: f64 = result.trace[n - window..].iter().map(base_income).sum();
    let campaign = if run_base > 0.0 {
        breakdown.campaign * (recent_base / run_base) / window as f64
    } else {
        0.0
    };
    let non_campaign = (breakdown.passive + breakdown.feeding + breakdown.gold_dragon) / n as f64;
    Some(campaign + non_campaign)
}

/// How a [`ReportLine`] should be colored (each front end maps it to its theme).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Bright,
    Normal,
    Muted,
    Accent,
    Success,
    Warning,
}

/// One line of the run report.
#[derive(Debug, Clone, PartialEq)]
pub struct ReportLine {
    pub text: String,
    pub tone: Tone,
    /// Suggested font size in points.
    pub size: f32,
    /// Optional explanatory hover/hint text.
    pub hint: Option<&'static str>,
}

fn line(text: String, tone: Tone, size: f32) -> ReportLine {
    ReportLine { text, tone, size, hint: None }
}

/// The last run's report, line by line: the headline, the averages and reward
/// trend, growth by source, the special pets' share, then one block per pet that
/// was in the chamber for the run. Empty when nothing has been run yet.
///
/// Targets are classified against the *live* target values, re-derived from each
/// pet's recorded trajectory — so editing a target updates the report without a
/// re-run.
pub fn report(state: &ChamberState) -> Vec<ReportLine> {
    let Some(result) = &state.result else { return Vec::new() };
    let mut out = Vec::new();

    // Actual elapsed hours summed from the (possibly varying) cycle lengths.
    let total_hours: u32 = result.trace.iter().map(|c| c.hours).sum();
    // Count targets reached against the *live* target values (already-above
    // counts as reached).
    let reached_count = result
        .tracks
        .iter()
        .filter(|t| {
            state.targets.get(&t.name).is_some_and(|&target| {
                !matches!(t.status(target as f64), TargetStatus::NotReached)
            })
        })
        .count();
    out.push(line(
        format!(
            "Ran {} cycles ({}) — {} target(s) reached.  (by growth gained)",
            result.cycles,
            fmt_hours(total_hours),
            reached_count
        ),
        Tone::Bright,
        12.0,
    ));

    // Report the pets that were in the chamber *for this run* (captured in
    // `last_starts`), so it doesn't shift if the selection changes before
    // the next run. Sorted by growth gained, most first.
    let mut rows: Vec<(&String, f64, f64)> = state
        .last_starts
        .iter()
        .filter_map(|(name, &start)| {
            let final_g = result.final_growth.iter().find(|(n, _)| n == name)?.1;
            Some((name, start, final_g))
        })
        .collect();
    rows.sort_by(|a, b| (b.2 - b.1).partial_cmp(&(a.2 - a.1)).unwrap_or(std::cmp::Ordering::Equal));

    // Per-pet total campaign contribution over the run (by name; trace
    // contributions are keyed by roster index, matching `final_growth`).
    let mut contrib: BTreeMap<&str, f64> = BTreeMap::new();
    for cyc in &result.trace {
        for &(idx, amount) in &cyc.contributions {
            if let Some((n, _)) = result.final_growth.get(idx) {
                *contrib.entry(n.as_str()).or_insert(0.0) += amount;
            }
        }
    }
    let cycle_base = |c: &ChamberCycle| -> f64 { c.contributions.iter().map(|(_, a)| a).sum() };

    // Summary stats: average growth/pet (per hour always; per cycle too when
    // every cycle is the same length) and the reward trend.
    if !rows.is_empty() && result.cycles > 0 {
        let total_gain: f64 = rows.iter().map(|(_, s, f)| f - s).sum();
        let per_pet = total_gain / rows.len() as f64;
        let per_hour = per_pet / total_hours.max(1) as f64;
        // Cycles are uniform unless a rebirth's remainder shortened some.
        let uniform = result.trace.windows(2).all(|w| w[0].hours == w[1].hours);
        let avg = if uniform {
            format!(
                "avg +{:.1}/pet/cycle  ·  +{:.2}/pet/hr",
                per_pet / result.cycles as f64,
                per_hour
            )
        } else {
            format!("avg +{per_hour:.2}/pet/hr")
        };
        let mut summary = format!("  {avg}  (chamber total +{total_gain:.0})");
        // Reward trend: average the first/last `window` cycles rather than
        // single cycles, so the recipient rotation (a big contributor being
        // the recipient that cycle → 0 contribution) doesn't skew it. The
        // window is ≈ one rotation (the chamber size), capped at half the
        // cycles so the two windows don't overlap.
        let n = result.trace.len();
        if n >= 2 {
            let window = rows.len().max(1).min(n / 2);
            let first: f64 =
                result.trace[..window].iter().map(&cycle_base).sum::<f64>() / window as f64;
            let last: f64 =
                result.trace[n - window..].iter().map(&cycle_base).sum::<f64>() / window as f64;
            summary += &format!("  ·  reward/cycle {first:.0} \u{2B62} {last:.0}");
        }
        out.push(line(summary, Tone::Muted, 11.0));
    }

    // Growth by source across the reported pets (each pet's gain splits
    // into campaign + passive + feeding + Gold Dragon — see `breakdown`).
    let bd = |name: &str| result.breakdown.iter().find(|(n, _)| n == name).map(|(_, b)| b);
    if !rows.is_empty() {
        let (mut camp, mut pass, mut feed, mut gd) = (0.0, 0.0, 0.0, 0.0);
        for (name, _, _) in &rows {
            if let Some(b) = bd(name) {
                camp += b.campaign;
                pass += b.passive;
                feed += b.feeding;
                gd += b.gold_dragon;
            }
        }
        out.push(line(
            format!(
                "  by source — campaign +{camp:.0}  ·  passive +{pass:.0}  ·  feeding +{feed:.0}  ·  Gold Dragon +{gd:.0}"
            ),
            Tone::Accent,
            11.0,
        ));
    }

    // The special pets' share of the campaign income — not a fifth
    // source. Pandora's boost is inside the "by source" campaign figure
    // (its recipients are chamber pets); Bag's gift counts toward its
    // *target's* campaign income, which is outside this report whenever
    // the global lowest pet is benched (the usual case).
    let sp = &result.specials;
    if sp.pandora_bonus > 0.0 || sp.bag_gift > 0.0 {
        let mut parts = Vec::new();
        if sp.pandora_bonus > 0.0 {
            parts.push(format!("Pandora's Box +{:.0} to recipients", sp.pandora_bonus));
        }
        if sp.bag_gift > 0.0 {
            let stolen = if sp.bag_stolen > 0.0 { " (stolen from deposits)" } else { "" };
            parts.push(format!("Bag +{:.0} gifted to the lowest{stolen}", sp.bag_gift));
        }
        out.push(ReportLine {
            text: format!("  specials — {}", parts.join("  ·  ")),
            tone: Tone::Accent,
            size: 11.0,
            hint: Some(
                "Bonus growth from the special pets' abilities, not a fifth source. Pandora's Box boosts each cycle's recipient deposit — included in the campaign figures above. Bag gifts a share to the global lowest-growth pet, counted in that pet's campaign income — outside this report when the pet is benched.",
            ),
        });
    }

    for (name, start, final_g) in rows {
        let delta = final_g - start;
        let avg_contrib =
            contrib.get(name.as_str()).copied().unwrap_or(0.0) / result.cycles.max(1) as f64;
        // Classify the *live* target against this pet's recorded
        // trajectory, so editing the target updates the row immediately —
        // no re-run needed (membership/bonuses stay frozen; only the
        // target↔trajectory comparison is re-derived).
        let track = result.tracks.iter().find(|t| &t.name == name);
        let (status, tone) = match (state.targets.get(name), track) {
            (None, _) => (String::new(), Tone::Muted),
            (Some(&t), Some(tr)) => match tr.status(t as f64) {
                // Already at/above the target before the run — didn't grow
                // into it.
                TargetStatus::AlreadyAbove => {
                    ("\u{2713} already above target".to_string(), Tone::Success)
                }
                TargetStatus::Reached(cycle) => {
                    // Elapsed hours = sum of the first `cycle` cycle lengths.
                    let h: u32 = result.trace.iter().take(cycle as usize).map(|c| c.hours).sum();
                    (format!("\u{2713} target at cycle {cycle} ({})", fmt_hours(h)), Tone::Success)
                }
                TargetStatus::NotReached => {
                    // Linear-extrapolate from this pet's *recent* growth/cycle
                    // (rewards trend up, so the whole-run average lands late) to
                    // estimate when it would reach the target (total from the run
                    // start, so it lines up with the reached pets above).
                    let per_cycle = recent_rate_per_cycle(result, name)
                        .unwrap_or(delta / result.cycles.max(1) as f64);
                    let remaining = t as f64 - final_g;
                    if per_cycle > 0.0 && remaining > 0.0 {
                        let more = (remaining / per_cycle).ceil();
                        let avg_cycle_h = total_hours as f64 / result.cycles.max(1) as f64;
                        let eta_h = (total_hours as f64 + more * avg_cycle_h).min(u32::MAX as f64);
                        let est_cycle = result.cycles as f64 + more;
                        (
                            format!(
                                "{final_g:.0}/{t} — est. cycle ~{est_cycle:.0} ({})",
                                fmt_hours(eta_h as u32)
                            ),
                            Tone::Warning,
                        )
                    } else {
                        (format!("{final_g:.0}/{t} (not reached)"), Tone::Warning)
                    }
                }
            },
            // Target set on a pet that wasn't in this run (no trajectory to
            // read) — prompt a re-run rather than guess.
            (Some(&t), None) => (format!("{final_g:.0}/{t} — run to evaluate"), Tone::Warning),
        };
        out.push(line(
            format!(
                "  {name}: {start:.0} \u{2B62} {final_g:.0}  (+{delta:.0})  contrib ~{avg_contrib:.0}/cyc{}{status}",
                if status.is_empty() { "" } else { "  " }
            ),
            tone,
            11.0,
        ));
        // Per-pet split: what made up this pet's gain.
        if let Some(b) = bd(name) {
            out.push(line(
                format!(
                    "      campaign +{:.0}  passive +{:.0}  feeding +{:.0}  GD +{:.0}",
                    b.campaign, b.passive, b.feeding, b.gold_dragon
                ),
                Tone::Muted,
                10.0,
            ));
        }
        // Class progression (Adventurers only). Show the level move and the
        // XP earned; highlight when the pet actually leveled.
        if let Some(cp) = result.class_progress.iter().find(|c| &c.name == name) {
            let leveled = cp.end_level > cp.start_level;
            let level_txt = if leveled {
                format!("CL {} \u{2B62} {}", cp.start_level, cp.end_level)
            } else {
                format!("CL {}", cp.end_level)
            };
            out.push(line(
                format!(
                    "      class — {level_txt}  (+{:.0} class XP, {:.0} toward next)",
                    cp.exp_gained, cp.final_exp
                ),
                if leveled { Tone::Success } else { Tone::Muted },
                10.0,
            ));
        }
    }
    out
}


#[cfg(test)]
mod tests {
    use super::*;
    use itrtg_models::{
        CombatStats, Element, ElementalAffinities, Equipment, PetAction, Quality,
    };

    fn export_with(growth: u64, cl: u32) -> ExportPet {
        ExportPet {
            export_name: "Test".into(),
            element: Element::Fire,
            growth,
            dungeon_level: 1,
            class: None,
            class_level: cl,
            class_exp: 0.0,
            combat_stats: CombatStats { hp: 1, attack: 1, defense: 1, speed: 1 },
            elemental_affinities: ElementalAffinities {
                water: 0,
                fire: 0,
                wind: 0,
                earth: 0,
                dark: 0,
                light: 0,
            },
            loadout: Loadout { weapon: None, armor: None, accessory: None },
            action: PetAction::Idle,
            unlocked: true,
            improved: false,
            other: None,
            has_partner: false,
        }
    }

    fn merged(name: &str, export: ExportPet) -> MergedPet {
        MergedPet { name: name.into(), wiki: None, export: Some(export) }
    }

    #[test]
    fn effective_export_without_override_matches_the_export() {
        let pet = merged("Foo", export_with(1000, 5));
        let state = ChamberState::default();
        let eff = effective_export(&pet, &state).unwrap();
        assert_eq!(eff.class_level, 5);
        assert!(eff.loadout.weapon.is_none());
    }

    #[test]
    fn main_stats_auto_fill_the_chamber_inputs() {
        use itrtg_models::MainStats;
        let mut state = ChamberState::default();
        let filled = state.apply_main_stats(&MainStats {
            ultimate_pet_challenges: Some((8, 20)), // 5 × 8 = 40%
            day_pet_challenge_multi: Some(3.664e9),
            fish_power: Some(1.05e6),
            fishing_level: Some(14),
            ..Default::default()
        });
        assert_eq!(filled, vec!["UPC", "DPC multi", "Fish Power", "Fishing level"]);
        assert_eq!((state.upc_done, state.upc_max), (8, 20));
        assert_eq!(state.upc_pct(), 40.0);
        assert_eq!(state.dpc_highest_multi, 3.664e9);
        assert_eq!(state.fish_power, 1.05e6);
        assert_eq!(state.fishing_level, 14);

        // The derived bonus caps at 100% even past 20 completions.
        state.apply_main_stats(&MainStats {
            ultimate_pet_challenges: Some((30, 30)),
            ..Default::default()
        });
        assert_eq!(state.upc_pct(), 100.0);
        // An empty export fills nothing and leaves values untouched.
        state.upc_done = 3;
        assert!(state.apply_main_stats(&MainStats::default()).is_empty());
        assert_eq!(state.upc_done, 3);

        // PGC fills both the completed count and the max.
        let filled = state
            .apply_main_stats(&MainStats { patreon_god_challenges: Some((24, 25)), ..Default::default() });
        assert_eq!(filled, vec!["PGC"]);
        assert_eq!((state.pgc_done, state.pgc_max), (24, 25));
    }

    #[test]
    fn reset_to_import_restores_only_imported_inputs() {
        use itrtg_models::MainStats;
        // Nothing imported yet: edits never count as deviating.
        let mut state = ChamberState { upc_done: 7, ..ChamberState::default() };
        assert!(!state.deviates_from_import());

        state.apply_main_stats(&MainStats {
            ultimate_pet_challenges: Some((8, 20)),
            fish_power: Some(1.05e6),
            ..Default::default()
        });
        assert!(!state.deviates_from_import());

        // Drift two imported fields → deviates; reset restores both.
        state.upc_done = 3;
        state.fish_power = 0.0;
        assert!(state.deviates_from_import());
        state.reset_to_import();
        assert_eq!((state.upc_done, state.upc_max), (8, 20));
        assert_eq!(state.fish_power, 1.05e6);
        assert!(!state.deviates_from_import());

        // A never-imported field (PGC here) neither deviates nor gets clobbered.
        state.pgc_done = 10;
        assert!(!state.deviates_from_import());
        state.reset_to_import();
        assert_eq!(state.pgc_done, 10);

        // A later partial import refreshes only its own field's snapshot — the
        // Fish Power baseline survives and still catches drift.
        state.apply_main_stats(&MainStats {
            ultimate_pet_challenges: Some((9, 20)),
            ..Default::default()
        });
        assert_eq!(state.imported.fish_power, Some(1.05e6));
        state.fish_power = 1.0;
        assert!(state.deviates_from_import());
    }

    #[test]
    fn chamber_entry_issue_classifies_stale_entries() {
        // Missing from the roster entirely (re-import / wiki rename): the old
        // code skipped these silently — they held a chamber slot forever.
        assert_eq!(chamber_entry_issue(None), Some("not in the imported roster"));
        // Locked after a re-import: a card used to render while build_roster
        // silently dropped the pet from the sim.
        let mut locked = export_with(1000, 5);
        locked.unlocked = false;
        assert_eq!(
            chamber_entry_issue(Some(&merged("Foo", locked))),
            Some("locked — excluded from the sim")
        );
        // Wiki-only pet (no export at all).
        let wiki_only = MergedPet { name: "Bar".into(), wiki: None, export: None };
        assert_eq!(chamber_entry_issue(Some(&wiki_only)), Some("no export data"));
        // A normal unlocked pet is fine.
        assert_eq!(chamber_entry_issue(Some(&merged("Baz", export_with(1000, 5)))), None);
    }

    #[test]
    fn recent_rate_tracks_the_rising_reward_curve() {
        use crate::campaign::GrowthBreakdown;

        // Solo chamber, 4 cycles, deposits rising 10 → 40 (base terms, mult 1).
        let cycle = |gain: f64| ChamberCycle {
            recipient: 0,
            recipient_gain: gain,
            bag_gift: None,
            contributions: vec![(0, 0.0)],
            hours: 12,
        };
        let result = ChamberResult {
            cycles: 4,
            reached: vec![],
            trace: vec![cycle(10.0), cycle(20.0), cycle(30.0), cycle(40.0)],
            final_growth: vec![("A".into(), 1108.0)],
            breakdown: vec![(
                "A".into(),
                GrowthBreakdown { campaign: 100.0, passive: 8.0, feeding: 0.0, gold_dragon: 0.0 },
            )],
            specials: Default::default(),
            tracks: vec![],
            class_progress: vec![],
        };
        // Window = min(2 × chamber size, n/2) = 2: campaign (30+40)/2 = 35,
        // plus passive 8/4 = 2 → 37/cycle. The whole-run average is only
        // 108/4 = 27/cycle — the lag this estimator fixes.
        let rate = recent_rate_per_cycle(&result, "A").unwrap();
        assert!((rate - 37.0).abs() < 1e-9, "got {rate}");

        // Too short for a distinct recent window → None (caller falls back to
        // the whole-run average), and an unknown pet is None too.
        let short =
            ChamberResult { cycles: 1, trace: vec![cycle(10.0)], ..result.clone() };
        assert!(recent_rate_per_cycle(&short, "A").is_none());
        assert!(recent_rate_per_cycle(&result, "B").is_none());

        // A window that can't fit (2 rotations > half the run) falls back too,
        // rather than truncating and possibly missing a pet's deposits: the
        // same 4 cycles with a 2-pet chamber would want a window of 4 > 2.
        let mut wide = result.clone();
        for c in &mut wide.trace {
            c.contributions.push((1, 0.0));
        }
        assert!(recent_rate_per_cycle(&wide, "A").is_none());
    }

    #[test]
    fn pgc_completions_scale_total_growth() {
        use itrtg_models::{CampaignBonusRules, CampaignInputs};

        let pet = merged("Foo", export_with(1000, 5));
        let roster = vec![pet.clone()];
        let bonuses = CampaignBonusRules::default();
        let inputs = CampaignInputs::default();
        let ctx = CampaignContext {
            bonuses: &bonuses,
            roster: &roster,
            inputs: &inputs,
            include_equipment: true,
            include_class: true,
        };
        let rates = GrowthRates { evolved_pets: 0, moai_per_hour: 0.0, pendant_cap: 0 };

        let mut state = ChamberState::default();
        assert_eq!(chamber_pet(&pet, &ctx, &rates, &state).unwrap().growth_multiplier, 1.0);
        // Gradual: +1% per completion below the max…
        state.pgc_done = 10;
        let mult = chamber_pet(&pet, &ctx, &rates, &state).unwrap().growth_multiplier;
        assert!((mult - 1.10).abs() < 1e-12, "10/25 → {mult}");
        // …doubled at full completion.
        state.pgc_done = 25;
        assert_eq!(chamber_pet(&pet, &ctx, &rates, &state).unwrap().growth_multiplier, 1.5);

        // Stacks multiplicatively with the Magic Egg: 1.5 × 1.3 = 1.95.
        let base = pet.export.clone().unwrap();
        set_class_level(&mut state, "Foo", &base, 5); // seed the override entry
        state.overrides.get_mut("Foo").unwrap().loadout.weapon = Some(Equipment {
            name: "Magic Egg".into(),
            upgrade_level: None,
            quality: Quality::SSS,
            enchant_level: None,
            gem: None,
            gem_level: None,
        });
        let mult = chamber_pet(&pet, &ctx, &rates, &state).unwrap().growth_multiplier;
        assert!((mult - 1.95).abs() < 1e-12, "egg × PGC = {mult}");
    }

    #[test]
    fn dpc_boost_matches_the_in_game_food_values() {
        // in-game shows "Pet food growth boost: 32%" for this multi.
        let mut state = ChamberState { dpc_highest_multi: 3.664e9, ..Default::default() };
        // Accurate log2 ≈ 31.77% (the display rounds to 32).
        assert!((state.dpc_boost_pct() - 31.77).abs() < 0.05, "{}", state.dpc_boost_pct());
        // Chocolate (base 6) → 6 × 1.3177 ≈ 7.906, matching the in-game 7.91.
        state.food_choice = 4;
        assert!((state.food_growth() - 7.906).abs() < 0.01, "{}", state.food_growth());
        // No DPC multi → base food, no boost.
        state.dpc_highest_multi = 0.0;
        assert_eq!(state.dpc_boost_pct(), 0.0);
        assert_eq!(state.food_growth(), 6.0);
    }

    #[test]
    fn override_swaps_cl_and_loadout() {
        let pet = merged("Foo", export_with(1000, 5));
        let mut state = ChamberState::default();
        let base = pet.export.clone().unwrap();

        // CL override flows through.
        set_class_level(&mut state, "Foo", &base, 22);
        assert_eq!(effective_export(&pet, &state).unwrap().class_level, 22);

        // A loadout override flows through too (the phase-2b edit path).
        state.overrides.get_mut("Foo").unwrap().loadout.weapon = Some(Equipment {
            name: "Magic Egg".into(),
            upgrade_level: None,
            quality: Quality::SSS,
            enchant_level: None,
            gem: None,
            gem_level: None,
        });
        let eff = effective_export(&pet, &state).unwrap();
        assert!(eff.has_magic_egg());
        assert_eq!(eff.class_level, 22, "CL override survives a later loadout edit");
    }

    #[test]
    fn set_class_level_seeds_then_updates_in_place() {
        let pet = merged("Foo", export_with(1000, 5));
        let mut state = ChamberState::default();
        let base = pet.export.clone().unwrap();

        set_class_level(&mut state, "Foo", &base, 10);
        assert_eq!(state.overrides["Foo"].class_level, 10);
        // A second edit mutates the same entry rather than adding another.
        set_class_level(&mut state, "Foo", &base, 30);
        assert_eq!(state.overrides["Foo"].class_level, 30);
        assert_eq!(state.overrides.len(), 1);

        // "Refresh from export" is just dropping the entry.
        state.overrides.remove("Foo");
        assert_eq!(effective_export(&pet, &state).unwrap().class_level, 5);
    }

    #[test]
    fn cl_override_raises_adventurer_growth_bonus() {
        use itrtg_models::{CampaignBonusRules, CampaignInputs, Class};

        // An Adventurer's all-campaign bonus is (2 + evo)%·CL; a generic pet has
        // evo 0, so 2%/CL. The override must flow CL through campaign_bonus_for.
        let mut adv = export_with(1000, 5);
        adv.class = Some(Class::Adventurer);
        let pet = merged("Advy", adv);

        let roster = vec![pet.clone()];
        let bonuses = CampaignBonusRules::default();
        let inputs = CampaignInputs::default();
        let ctx = CampaignContext {
            bonuses: &bonuses,
            roster: &roster,
            inputs: &inputs,
            include_equipment: true,
            include_class: true,
        };
        let rates = GrowthRates { evolved_pets: 0, moai_per_hour: 0.0, pendant_cap: 0 };

        let mut state = ChamberState::default();
        let base = pet.export.clone().unwrap();

        // No override (fast path): 2%·5 = 10%.
        let before = chamber_pet(&pet, &ctx, &rates, &state).unwrap().campaign_bonus_pct;
        // CL 5 → 22 (synthetic path): 2%·22 = 44%.
        set_class_level(&mut state, "Advy", &base, 22);
        let after = chamber_pet(&pet, &ctx, &rates, &state).unwrap().campaign_bonus_pct;

        assert!(
            after > before,
            "raising CL should raise the Adventurer Growth bonus ({before} → {after})"
        );
        assert!(
            (after - before - 34.0).abs() < 0.5,
            "2%/CL over a 17-level bump ≈ +34% ({before} → {after})"
        );
    }

    #[test]
    fn weapon_for_builds_the_right_equipment() {
        // none clears the slot.
        assert!(weapon_for("none", None).is_none());

        // Magic Egg: name only (level irrelevant to the +30% multiplier).
        let egg = weapon_for("Magic Egg", None).unwrap();
        assert_eq!(egg.name, "Magic Egg");
        assert_eq!(egg.upgrade_level, None);

        // Candy Cane: pinned to SSS+20 (its only level with a bonus).
        let cane = weapon_for("Candy Cane", None).unwrap();
        assert_eq!((cane.quality, cane.upgrade_level), (Quality::SSS, Some(20)));

        // A fresh stick defaults to SSS+20.
        let stick = weapon_for("Magic Stick", None).unwrap();
        assert_eq!((stick.name.as_str(), stick.quality, stick.upgrade_level), ("Magic Stick", Quality::SSS, Some(20)));

        // Switching between sticks preserves quality/upgrade.
        let cur = equip("Walking Stick", Quality::B, Some(7));
        let swapped = weapon_for("Legendary Stick", Some(&cur)).unwrap();
        assert_eq!((swapped.name.as_str(), swapped.quality, swapped.upgrade_level), ("Legendary Stick", Quality::B, Some(7)));

        // Switching from a non-stick (egg) to a stick does NOT carry levels — it
        // defaults fresh.
        let from_egg = weapon_for("Magic Stick", Some(&equip("Magic Egg", Quality::SSS, None))).unwrap();
        assert_eq!(from_egg.upgrade_level, Some(20));
    }

    #[test]
    fn equipping_a_stick_raises_the_growth_bonus() {
        use itrtg_models::{CampaignBonusRules, CampaignInputs};

        // A generic pet (no wiki innate, not Adventurer) has a 0% Growth bonus;
        // a Legendary Stick SSS+20 is the cap, +100%.
        let pet = merged("Foo", export_with(1000, 5));
        let roster = vec![pet.clone()];
        let bonuses = CampaignBonusRules::default();
        let inputs = CampaignInputs::default();
        let ctx = CampaignContext {
            bonuses: &bonuses,
            roster: &roster,
            inputs: &inputs,
            include_equipment: true,
            include_class: true,
        };
        let rates = GrowthRates { evolved_pets: 0, moai_per_hour: 0.0, pendant_cap: 0 };
        let base = pet.export.clone().unwrap();

        let mut state = ChamberState::default();
        assert_eq!(chamber_pet(&pet, &ctx, &rates, &state).unwrap().campaign_bonus_pct, 0.0);

        set_weapon(&mut state, "Foo", &base, "Legendary Stick");
        let bonus = chamber_pet(&pet, &ctx, &rates, &state).unwrap().campaign_bonus_pct;
        assert!((bonus - 100.0).abs() < 0.01, "Legendary SSS+20 caps at +100% (got {bonus})");
    }

    #[test]
    fn merry_mantle_adds_then_removing_it_clears_the_bonus() {
        use itrtg_models::{CampaignBonusRules, CampaignInputs};

        let pet = merged("Foo", export_with(1000, 5));
        let roster = vec![pet.clone()];
        let bonuses = CampaignBonusRules::default();
        let inputs = CampaignInputs::default();
        let ctx = CampaignContext {
            bonuses: &bonuses,
            roster: &roster,
            inputs: &inputs,
            include_equipment: true,
            include_class: true,
        };
        let rates = GrowthRates { evolved_pets: 0, moai_per_hour: 0.0, pendant_cap: 0 };
        let base = pet.export.clone().unwrap();

        let mut state = ChamberState::default();
        // Merry Mantle SSS+20 → +150%.
        override_mut(&mut state, "Foo", &base).loadout.armor =
            Some(equip("Merry Mantle", Quality::SSS, Some(20)));
        let bonus = chamber_pet(&pet, &ctx, &rates, &state).unwrap().campaign_bonus_pct;
        assert!((bonus - 150.0).abs() < 0.01, "Merry Mantle SSS+20 is +150% (got {bonus})");

        // Clearing the slot drops the bonus back to 0.
        override_mut(&mut state, "Foo", &base).loadout.armor = None;
        let cleared = chamber_pet(&pet, &ctx, &rates, &state).unwrap().campaign_bonus_pct;
        assert_eq!(cleared, 0.0, "removing the armor clears its contribution");
    }

    #[test]
    fn equipping_the_pendant_adds_passive_growth() {
        use itrtg_models::{CampaignBonusRules, CampaignInputs};

        let pet = merged("Foo", export_with(1000, 5));
        let roster = vec![pet.clone()];
        let bonuses = CampaignBonusRules::default();
        let inputs = CampaignInputs::default();
        let ctx = CampaignContext {
            bonuses: &bonuses,
            roster: &roster,
            inputs: &inputs,
            include_equipment: true,
            include_class: true,
        };
        // evolved_pets drives pendant_per_hour; cap high enough to apply.
        let rates = GrowthRates { evolved_pets: 10, moai_per_hour: 1.0, pendant_cap: u64::MAX };
        let base = pet.export.clone().unwrap();

        let mut state = ChamberState::default();
        let without = chamber_pet(&pet, &ctx, &rates, &state).unwrap().passive_per_hour;
        override_mut(&mut state, "Foo", &base).loadout.accessory =
            Some(equip("Growing Love Pendant", Quality::SSS, None));
        let with = chamber_pet(&pet, &ctx, &rates, &state).unwrap().passive_per_hour;
        assert!(with > without, "pendant should add passive growth ({without} → {with})");
    }
}

#[cfg(test)]
mod shared_tests {
    use super::*;
    use crate::campaign::{GrowthBreakdown, GrowthTrack};

    #[test]
    fn membership_respects_the_ten_pet_cap() {
        let mut state = ChamberState::default();
        for i in 0..10 {
            assert!(set_membership(&mut state, &format!("P{i}"), true));
        }
        // An 11th is refused; re-adding a member is a no-op.
        assert!(!set_membership(&mut state, "P10", true));
        assert!(!set_membership(&mut state, "P3", true));
        assert_eq!(state.chamber.len(), 10);
        assert!(set_membership(&mut state, "P3", false));
        assert!(set_membership(&mut state, "P10", true));
        assert!(!set_membership(&mut state, "Nobody", false));
    }

    #[test]
    fn target_of_zero_clears_the_entry() {
        let mut state = ChamberState::default();
        set_target(&mut state, "Foo", 5000);
        assert_eq!(state.targets.get("Foo"), Some(&5000));
        set_target(&mut state, "Foo", 0);
        assert!(state.targets.is_empty());
    }

    #[test]
    fn persisted_round_trip_keeps_settings_and_drops_ephemera() {
        let mut state = ChamberState {
            hours: 6,
            adv_xp_mult: 2.5,
            search: "abc".into(),
            pgc_done: 99, // above pgc_max: sanitized on load
            ..Default::default()
        };
        state.last_starts.insert("Foo".into(), 1.0);
        let stored = state.persisted();
        assert!(stored.search.is_empty() && stored.last_starts.is_empty());
        let mut loaded = ChamberState::default();
        loaded.load_persisted(&stored);
        assert_eq!(loaded.hours, 6);
        assert_eq!(loaded.adv_xp_mult, 2.5);
        assert_eq!(loaded.pgc_done, loaded.pgc_max);
    }

    #[test]
    fn report_is_empty_before_a_run_and_tracks_live_targets() {
        let mut state = ChamberState::default();
        assert!(report(&state).is_empty());

        state.last_starts.insert("A".into(), 1000.0);
        state.result = Some(ChamberResult {
            cycles: 2,
            reached: vec![],
            trace: vec![
                ChamberCycle { recipient: 0, recipient_gain: 50.0, bag_gift: None, contributions: vec![(0, 0.0)], hours: 12 },
                ChamberCycle { recipient: 0, recipient_gain: 50.0, bag_gift: None, contributions: vec![(0, 0.0)], hours: 12 },
            ],
            final_growth: vec![("A".into(), 1100.0)],
            breakdown: vec![(
                "A".into(),
                GrowthBreakdown { campaign: 100.0, passive: 0.0, feeding: 0.0, gold_dragon: 0.0 },
            )],
            specials: Default::default(),
            tracks: vec![GrowthTrack { name: "A".into(), start: 1000.0, per_cycle: vec![1050.0, 1100.0] }],
            class_progress: vec![],
        });
        let lines = report(&state);
        assert!(lines[0].text.starts_with("Ran 2 cycles (~24 h) — 0 target(s) reached."), "{}", lines[0].text);
        assert!(lines.iter().any(|l| l.text.contains("A: 1000 \u{2B62} 1100  (+100)")));

        // Setting a target after the run re-classifies without a re-run.
        set_target(&mut state, "A", 1040);
        let lines = report(&state);
        assert!(lines[0].text.contains("1 target(s) reached"), "{}", lines[0].text);
        assert!(lines.iter().any(|l| l.text.contains("target at cycle 1") && l.tone == Tone::Success));
    }
}
