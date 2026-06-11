//! Growth-chamber view: configure a set of pets locked into the Growth campaign,
//! run the multi-cycle simulation ([`itrtg_planner::campaign::simulate_growth_chamber`]),
//! and see how many cycles each tracked pet needs to reach its target.
//!
//! This first cut focuses on the "rush a pet to a target" use case. The driver
//! ([`build_roster`]) bridges the real roster to the sim: it pulls each pet's
//! growth-campaign bonus (innate + equipment + class), its passive growth/hour
//! (Moai for all; the Growing Love Pendant when assigned), and tags the special
//! pets (Pandora's Box / Bag). **All unlocked pets** go into the sim — those not
//! in the chamber are benched, so Bag's gift can still reach the global lowest.

use std::collections::BTreeMap;

use eframe::egui::{self, RichText};
use itrtg_models::{
    pgc_growth_mult, CampaignType, Equipment, ExportPet, Loadout, Quality, MAGIC_EGG_GROWTH_MULT,
};
use itrtg_planner::campaign::{
    simulate_growth_chamber, ChamberPet, ChamberResult, ChamberRun, SpecialPet,
};
use itrtg_planner::growth::GrowthRates;
use itrtg_planner::merge::{CampaignContext, MergedPet};
use serde::{Deserialize, Serialize};

use crate::data::DataStore;
use crate::style;

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
    /// Per-pet what-if overrides (canonical name → tweaked loadout + CL). Absent
    /// means "use the live export as-is". Seeded from the export on first edit;
    /// the card's "Refresh from export" button drops the entry to revert.
    pub overrides: BTreeMap<String, PetOverride>,

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
            overrides: BTreeMap::new(),
            search: String::new(),
            result: None,
            last_starts: BTreeMap::new(),
            row_heights: Vec::new(),
        }
    }
}

impl ChamberState {
    pub fn apply_app_state(&mut self, state: &crate::state::AppState) {
        let src = &state.chamber;
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
        self.overrides = src.overrides.clone();
    }

    pub fn write_into(&self, state: &mut crate::state::AppState) {
        // Persisted fields only (drop the ephemeral search/result/last_starts).
        state.chamber = ChamberState {
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
            overrides: self.overrides.clone(),
            ..Default::default()
        };
    }

    /// Auto-fill the chamber's global inputs from a Main-stats export: the UPC
    /// and PGC completion counts, the DPC food multiplier, and Fish Power /
    /// Fishing Level. Returns the filled field labels for the import status.
    pub fn apply_main_stats(&mut self, ms: &itrtg_models::MainStats) -> Vec<&'static str> {
        let mut filled = Vec::new();
        if let Some((done, max)) = ms.ultimate_pet_challenges {
            self.upc_done = done;
            self.upc_max = max;
            filled.push("UPC");
        }
        if let Some((done, max)) = ms.patreon_god_challenges {
            self.pgc_done = done;
            self.pgc_max = max;
            filled.push("PGC");
        }
        if let Some(multi) = ms.day_pet_challenge_multi {
            self.dpc_highest_multi = multi;
            filled.push("DPC multi");
        }
        if let Some(fp) = ms.fish_power {
            self.fish_power = fp;
            filled.push("Fish Power");
        }
        if let Some(level) = ms.fishing_level {
            self.fishing_level = level;
            filled.push("Fishing level");
        }
        filled
    }

    /// The global PGC growth multiplier (×1 with no completions).
    fn pgc_mult(&self) -> f64 {
        pgc_growth_mult(self.pgc_done, self.pgc_max)
    }

    /// The UPC campaign bonus %: `5 ·` completions, capped at 100%.
    fn upc_pct(&self) -> f64 {
        (5.0 * self.upc_done as f64).min(100.0)
    }

    /// The DPC food boost %: `log2(highest pet multiplier)`, capped at 100%. 0 when
    /// there's no multiplier (≤ 1, where the log is ≤ 0).
    fn dpc_boost_pct(&self) -> f64 {
        if !self.dpc_highest_multi.is_finite() || self.dpc_highest_multi <= 1.0 {
            return 0.0;
        }
        self.dpc_highest_multi.log2().clamp(0.0, 100.0)
    }

    /// The steady (no-fishing) growth per feeding for a food type: base × the DPC
    /// boost. Fishing is layered on top by the sim (it decays over the rebirth).
    fn food_value(&self, idx: usize) -> f64 {
        BASE_FOOD.get(idx).copied().unwrap_or(0.0) * (1.0 + self.dpc_boost_pct() / 100.0)
    }

    /// Effective steady growth per feeding for the chosen food.
    fn food_growth(&self) -> f64 {
        self.food_value(self.food_choice)
    }

    /// Average rebirth length in **whole** hours (value × unit, truncated — a
    /// partial hour can't run another campaign, so it's dropped).
    fn rebirth_total_hours(&self) -> u32 {
        let factor = REBIRTH_UNITS.get(self.rebirth_unit).map_or(1, |&(_, h)| h) as f64;
        ((self.rebirth_value * factor).floor() as i64).clamp(1, u32::MAX as i64) as u32
    }

    /// The Fish Power food boost % at a rebirth's start (0 if no Fish Power).
    fn fishing_boost(&self) -> f64 {
        itrtg_planner::campaign::fishing_boost_pct(self.fish_power, self.fishing_level)
    }

    /// Per-feeding growth every pet gets from a Gold Dragon feeding (25% of his
    /// food's growth). 0 if Gold Dragon isn't being fed.
    fn gold_dragon_broadcast(&self) -> f64 {
        self.gold_dragon_food.map_or(0.0, |i| 0.25 * self.food_value(i))
    }
}

/// Build the full sim roster from every unlocked pet: those in `state.chamber`
/// are participants, the rest are benched (so Bag's gift can reach the global
/// lowest). Pulls each pet's Growth bonus (equipment + class included via `ctx`)
/// and passive growth/hour (Moai + pendant).
fn build_roster(
    data: &DataStore,
    ctx: &CampaignContext,
    rates: &GrowthRates,
    state: &ChamberState,
) -> Vec<ChamberPet> {
    data.merged
        .iter()
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
/// the existing [`MergedPet::campaign_bonus_for`] seam.
fn effective_export(pet: &MergedPet, state: &ChamberState) -> Option<ExportPet> {
    let base = pet.export.as_ref()?;
    let mut eff = base.clone();
    if let Some(ov) = state.overrides.get(&pet.name) {
        eff.loadout = ov.loadout.clone();
        eff.class_level = ov.class_level;
    }
    Some(eff)
}

fn chamber_pet(
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
    // show live. With no override this is the unedited export, so the result
    // matches `pet.campaign_bonus_for` exactly; only build the synthetic pet (and
    // clone the wiki) when an override is actually in play.
    let bonus = if state.overrides.contains_key(&pet.name) {
        let synth =
            MergedPet { name: pet.name.clone(), wiki: pet.wiki.clone(), export: Some(export.clone()) };
        synth.campaign_bonus_for(CampaignType::Growth, ctx).unwrap_or(0.0)
    } else {
        pet.campaign_bonus_for(CampaignType::Growth, ctx).unwrap_or(0.0)
    };
    let mut passive = rates.moai_per_hour;
    // The pendant is just the equipped accessory — no separate toggle.
    if export.loadout.accessory.as_ref().is_some_and(|a| a.name == "Growing Love Pendant") {
        passive += rates.pendant_per_hour();
    }
    let special = match pet.name.as_str() {
        "Pandora's Box" => Some(SpecialPet::Pandora { feedings: state.pandora_feedings }),
        "Bag" => Some(SpecialPet::Bag { token_improved: export.improved }),
        // Reduces every other chamber pet's campaign bonus by (20 − 0.25·CL) pts.
        "Nightmare" => Some(SpecialPet::Nightmare { class_level: export.class_level }),
        _ => None,
    };
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
    })
}

pub fn show(
    ui: &mut egui::Ui,
    state: &mut ChamberState,
    data: &DataStore,
    ctx: &CampaignContext,
    rates: &GrowthRates,
) {
    ui.heading(RichText::new("Growth Chamber").color(style::TEXT_BRIGHT));
    ui.label(
        RichText::new(
            "Lock pets into the Growth campaign and project how many cycles a \
             tracked pet needs to hit its target.",
        )
        .color(style::TEXT_MUTED)
        .size(11.0),
    );
    ui.add_space(4.0);

    // --- Global run parameters ---
    ui.horizontal(|ui| {
        ui.label(RichText::new("Hours:").color(style::TEXT_MUTED).size(12.0));
        ui.add(egui::DragValue::new(&mut state.hours).range(1..=12));
        ui.separator();
        ui.label(RichText::new("UPC:").color(style::TEXT_MUTED).size(12.0))
            .on_hover_text("Ultimate Pet Challenges completed — each adds +5% to every pet's campaign contribution (capped at 100%). Auto-filled when you import a Main-stats export.");
        ui.add(egui::DragValue::new(&mut state.upc_done).range(0..=state.upc_max));
        ui.label(
            RichText::new(format!("/ {}  \u{2B62} +{:.0}%", state.upc_max, state.upc_pct()))
                .color(style::TEXT_MUTED)
                .size(11.0),
        );
        ui.separator();
        ui.label(RichText::new("PGC:").color(style::TEXT_MUTED).size(12.0))
            .on_hover_text("Patreon God Challenges completed — +1% growth per completion, doubled once all are done (24/25 → ×1.24, 25/25 → ×1.50). Stacks with the Magic Egg. Auto-filled when you import a Main-stats export.");
        ui.add(egui::DragValue::new(&mut state.pgc_done).range(0..=state.pgc_max));
        ui.label(
            RichText::new(format!("/ {}  \u{2B62} ×{:.2} growth", state.pgc_max, state.pgc_mult()))
                .color(style::TEXT_MUTED)
                .size(11.0),
        );
        ui.separator();
        ui.label(RichText::new("Pandora feedings:").color(style::TEXT_MUTED).size(12.0));
        ui.add(egui::DragValue::new(&mut state.pandora_feedings).range(0..=20))
            .on_hover_text("Pandora's starting feeding count. It climbs as she's fed each cycle (bonus caps at 20) and resets at the start of each rebirth.");
    });

    // --- Food (per-feeding growth, fed to every pet) ---
    ui.horizontal(|ui| {
        ui.label(RichText::new("Food:").color(style::TEXT_MUTED).size(12.0));
        for (i, label) in FOODS.iter().enumerate() {
            if ui.selectable_label(state.food_choice == i, *label).clicked() {
                state.food_choice = i;
            }
        }
        ui.separator();
        // Computed from the base food value × the DPC boost (fishing layered on in
        // the sim). No longer a manual input.
        ui.label(
            RichText::new(format!("{:.2} growth/feeding", state.food_growth()))
                .color(style::TEXT_NORMAL)
                .size(11.0),
        );
        ui.label(
            RichText::new(format!("× {} feedings/cycle", state.hours / 3))
                .color(style::TEXT_MUTED)
                .size(10.0),
        );
    });

    // --- DPC (food growth boost: log2(highest pet multiplier), capped 100%) ---
    ui.horizontal(|ui| {
        ui.label(RichText::new("DPC highest multi:").color(style::TEXT_MUTED).size(12.0))
            .on_hover_text("Day Pet Challenge highest pet multiplier (a %). Boosts food growth by log2(multi), capped 100%. Auto-filled from a Main-stats export.");
        ui.add(
            egui::DragValue::new(&mut state.dpc_highest_multi)
                .range(0.0..=1e18)
                .speed(1e5)
                .max_decimals(0),
        );
        ui.label(
            RichText::new(format!("⭢ +{:.2}% to food", state.dpc_boost_pct()))
                .color(style::TEXT_MUTED)
                .size(10.0),
        );
    });

    // --- Gold Dragon (25% of his food growth goes to every pet) ---
    ui.horizontal(|ui| {
        ui.label(RichText::new("Gold Dragon food:").color(style::TEXT_MUTED).size(12.0))
            .on_hover_text("Feeding Gold Dragon gives every pet 25% of the growth he gains.");
        if ui.selectable_label(state.gold_dragon_food.is_none(), "None").clicked() {
            state.gold_dragon_food = None;
        }
        for (i, label) in FOODS.iter().enumerate() {
            if ui.selectable_label(state.gold_dragon_food == Some(i), *label).clicked() {
                state.gold_dragon_food = Some(i);
            }
        }
        if state.gold_dragon_food.is_some() {
            ui.label(
                RichText::new(format!("(+{:.2}/feeding to all)", state.gold_dragon_broadcast()))
                    .color(style::TEXT_MUTED)
                    .size(10.0),
            );
        }
    });

    // --- Rebirths (cycle scheduling) ---
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.rebirth_enabled, RichText::new("Model rebirths").size(12.0))
            .on_hover_text(
                "Campaigns can't span a rebirth, so each rebirth runs full cycles then one shorter cycle for the leftover time.",
            );
        ui.add_enabled_ui(state.rebirth_enabled, |ui| {
            ui.label(RichText::new("every").color(style::TEXT_MUTED).size(12.0));
            ui.add(
                egui::DragValue::new(&mut state.rebirth_value)
                    .range(0.1..=100_000.0)
                    .speed(0.25)
                    .max_decimals(2),
            );
            let unit = REBIRTH_UNITS.get(state.rebirth_unit).map_or("Hours", |&(l, _)| l);
            egui::ComboBox::from_id_salt("rebirth_unit").selected_text(unit).show_ui(ui, |ui| {
                for (i, (label, _)) in REBIRTH_UNITS.iter().enumerate() {
                    ui.selectable_value(&mut state.rebirth_unit, i, *label);
                }
            });
            // Effective whole-hour length — shown when it's a conversion or a
            // partial hour gets truncated (e.g. 7.03 d ⭢ 168 h, 7.5 h ⭢ 7 h).
            if state.rebirth_unit != 0 || state.rebirth_value.fract() != 0.0 {
                ui.label(
                    RichText::new(format!("= {} h", state.rebirth_total_hours()))
                        .color(style::TEXT_MUTED)
                        .size(11.0),
                );
            }
        });
        if state.rebirth_enabled {
            let cycle = state.hours.clamp(1, 12);
            let rb = state.rebirth_total_hours();
            // Compact schedule: collapse the full cycles into a count and only show
            // the remainder when there is one (e.g. "14 × 12 h + 1 h").
            let full = cycle.min(rb);
            let n_full = rb / full;
            let rem = rb % full;
            let mut pattern =
                if n_full <= 1 { format!("{full} h") } else { format!("{n_full} × {full} h") };
            if rem > 0 {
                pattern += &format!(" + {rem} h");
            }
            let (text, color) = if rb < cycle {
                (format!("⚠ rebirth ({rb} h) < cycle — cycles clamped to {pattern}"), style::WARNING)
            } else {
                (format!("⭢ {pattern} per rebirth"), style::TEXT_MUTED)
            };
            ui.label(RichText::new(text).color(color).size(10.0));
        }
    });

    // --- Fishing (Fish Power food boost, decays over a rebirth's first 30 h) ---
    ui.horizontal(|ui| {
        ui.label(RichText::new("Fish Power:").color(style::TEXT_MUTED).size(12.0))
            .on_hover_text("Boosts all food growth by FishPower^0.25 × milestone, decaying to 0 over the first 30 h of each rebirth. 0 = off.");
        ui.add(
            egui::DragValue::new(&mut state.fish_power)
                .range(0.0..=1e15)
                .speed(1000.0)
                .max_decimals(0),
        );
        ui.label(RichText::new("Fishing level:").color(style::TEXT_MUTED).size(12.0))
            .on_hover_text("Milestones: +10% at level 15, another +10% at 27.");
        ui.add(egui::DragValue::new(&mut state.fishing_level).range(0..=100));
        let boost = state.fishing_boost();
        if boost > 0.0 {
            let (text, color) = if state.rebirth_enabled {
                (format!("⭢ +{boost:.1}% food at rebirth start, fading over 30 h"), style::TEXT_MUTED)
            } else {
                ("needs rebirth modeling to apply".to_string(), style::WARNING)
            };
            ui.label(RichText::new(text).color(color).size(10.0));
        }
    });

    // --- Run mode + actions ---
    ui.horizontal(|ui| {
        ui.label(RichText::new("Max cycles:").color(style::TEXT_MUTED).size(12.0));
        ui.add(egui::DragValue::new(&mut state.max_cycles).range(1..=1_000_000).speed(50.0));
        ui.separator();
        ui.checkbox(
            &mut state.run_until_targets,
            RichText::new("Stop early when all targets reached").size(12.0),
        )
        .on_hover_text("Off: always run the full max cycles. On: stop as soon as every target is hit (max cycles still caps it).");
        ui.separator();
        ui.checkbox(
            &mut state.exported_after_campaign,
            RichText::new("Export taken at a campaign's end").size(12.0),
        )
        .on_hover_text(
            "On: the first cycle adds no passive (Moai) growth, since an export captured right when a campaign finished already includes it. Avoids double-counting that ~one cycle of Moai.",
        );
    });

    ui.horizontal(|ui| {
        let n = state.chamber.len();
        let over = n > 10;
        let run = ui
            .add_enabled(!over, egui::Button::new(RichText::new("\u{25B6} Run").size(13.0)))
            .clicked();
        if ui
            .button(RichText::new("Recommend chamber").size(12.0))
            .on_hover_text("Fill the chamber with the top 10 by Growth bonus (tiebreak: growth)")
            .clicked()
        {
            recommend_chamber(state, data, ctx);
        }
        ui.label(
            RichText::new(format!("{n}/10 in chamber{}", if over { " — too many!" } else { "" }))
                .color(if over { style::WARNING } else { style::TEXT_MUTED })
                .size(11.0),
        );

        if run {
            // Lock in the in-chamber pets' starting growth (the report reads this,
            // so it won't shift when the selection changes before the next run).
            state.last_starts = data
                .merged
                .iter()
                .filter(|p| state.chamber.iter().any(|c| c == &p.name))
                .filter_map(|p| p.export.as_ref().map(|e| (p.name.clone(), e.effective_growth() as f64)))
                .collect();
            let mut roster = build_roster(data, ctx, rates, state);
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
                },
            ));
        }
    });

    ui.add_space(4.0);
    // The run controls above stay pinned; everything below (results, the pet
    // cards — which get tall once equipment editors and overrides are in play —
    // and the picker list) scrolls, so the picker stays reachable at any window
    // size. One scroll region only: the picker no longer scrolls on its own.
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        show_results(ui, state);
        ui.separator();
        show_pet_cards(ui, state, data, ctx, rates);
        ui.separator();
        show_pet_picker(ui, state, data, ctx);
    });
}

/// Cards for the selected chamber pets — their computed stats, read-only
/// equipment/CL, and the editable growth target.
fn show_pet_cards(
    ui: &mut egui::Ui,
    state: &mut ChamberState,
    data: &DataStore,
    ctx: &CampaignContext,
    rates: &GrowthRates,
) {
    if state.chamber.is_empty() {
        ui.label(
            RichText::new("No pets in the chamber yet — add some below.")
                .color(style::TEXT_MUTED)
                .size(11.0),
        );
        return;
    }
    ui.label(RichText::new("Chamber").color(style::TEXT_BRIGHT).size(12.0));

    // Render in the chamber's order; clone names so we can mutate `state`.
    let names: Vec<String> = state.chamber.clone();

    // Nightmare's team malus, if he's in the chamber — the points he docks from
    // every *other* pet's campaign bonus. Display-only here (the sim applies it
    // to the bonuses itself); recomputed each frame from his *effective* class
    // level, so the cards track add/remove and the CL editor live.
    let nightmare_malus: Option<f32> = if names.iter().any(|n| n == "Nightmare") {
        data.merged
            .iter()
            .find(|p| p.name == "Nightmare")
            .and_then(|p| effective_export(p, state))
            .map(|e| itrtg_planner::campaign::nightmare_malus(e.class_level) as f32)
    } else {
        None
    };

    // Lay the cards out in a *balanced* grid: pick the most columns that fit the
    // available width, then even the rows so the last one isn't a lonely single
    // card (e.g. 10 ⭢ 5+5, 9 ⭢ 5+4, 7 ⭢ 4+3 — never 9+1).
    let gap = ui.spacing().item_spacing.x;
    let avail = ui.available_width();
    // One card's footprint: 232 inner + 8·2 inner_margin + ~2 stroke.
    let footprint = CARD_WIDTH + 18.0;
    let max_cols = (((avail + gap) / (footprint + gap)).floor() as usize).max(1);
    let n = names.len();
    let rows = n.div_ceil(max_cols);
    let cols = n.div_ceil(rows.max(1)).max(1);

    // Equalize card heights within each row: pad every card up to the tallest
    // card's *natural* content height in that row. We pad to last frame's measured
    // max (`prev_heights`) and record this frame's measured max for next frame —
    // a one-frame lag that's imperceptible, and we request a repaint while the
    // measurements are still settling so it converges immediately.
    let prev_heights = state.row_heights.clone();
    let mut new_heights: Vec<f32> = Vec::with_capacity(prev_heights.len());
    // Chamber entries removed via a stale-entry stub this frame (applied after
    // the loop — `names` is a snapshot of `state.chamber`).
    let mut removed: Vec<String> = Vec::new();

    for (row_idx, chunk) in names.chunks(cols).enumerate() {
        // Center the row: pad the left by half the unused width so the row's
        // midpoint sits at the available width's midpoint (odd ⭢ middle card
        // centered, even ⭢ centered on the seam between the two middle cards).
        let k = chunk.len() as f32;
        let row_w = k * footprint + (k - 1.0).max(0.0) * gap;
        let pad = ((avail - row_w) * 0.5).max(0.0);
        let pad_to = prev_heights.get(row_idx).copied().unwrap_or(0.0);
        let mut row_max = 0.0_f32;
        ui.horizontal(|ui| {
            if pad > 0.0 {
                ui.add_space(pad);
            }
            for name in chunk {
                // An entry can go stale after a re-import (renamed/missing pet,
                // locked, no export). Show a removable stub instead of skipping
                // silently — a skipped ghost still counts toward the 10-slot
                // cap, the picker can't list it, and a locked pet would render
                // as a participant while `build_roster` drops it from the sim.
                let pet = data.merged.iter().find(|p| &p.name == name);
                if let Some(reason) = chamber_entry_issue(pet) {
                    if show_ghost_card(ui, name, reason) {
                        removed.push(name.clone());
                    }
                    continue;
                }
                let Some(pet) = pet else { continue };
                let Some(cp) = chamber_pet(pet, ctx, rates, state) else { continue };
                // The card shows (and edits) the *effective* export: the live
                // export with any per-pet override applied.
                let eff = effective_export(pet, state);
                let natural =
                    show_pet_card(ui, state, name, &cp, eff.as_ref(), pad_to, nightmare_malus);
                row_max = row_max.max(natural);
            }
        });
        new_heights.push(row_max);
    }

    if !removed.is_empty() {
        state.chamber.retain(|n| !removed.contains(n));
        // A roster-missing pet's target/override are equally unreachable from
        // the UI — drop them with it so they don't linger in the app state.
        for n in &removed {
            if !data.merged.iter().any(|p| &p.name == n) {
                state.targets.remove(n);
                state.overrides.remove(n);
            }
        }
    }

    // Repaint until the per-row heights stop changing (within ~half a pixel), so a
    // card growing/shrinking settles its row-mates without waiting for the next
    // interaction.
    let settled = prev_heights.len() == new_heights.len()
        && prev_heights
            .iter()
            .zip(&new_heights)
            .all(|(a, b)| (a - b).abs() < 0.5);
    if !settled {
        ui.ctx().request_repaint();
    }
    state.row_heights = new_heights;
}

/// Inner content width of a chamber pet card (excludes the frame's margin/stroke).
const CARD_WIDTH: f32 = 232.0;

/// Why a chamber entry can't participate in a run, if it can't (`None` = fine).
/// `pet` is the roster lookup for the entry's name — `None` when no merged pet
/// has that name anymore (re-import from another account, wiki rename).
fn chamber_entry_issue(pet: Option<&MergedPet>) -> Option<&'static str> {
    match pet {
        None => Some("not in the imported roster"),
        // Before the lock check: is_unlocked() is also false with no export.
        Some(p) if p.export.is_none() => Some("no export data"),
        Some(p) if !p.is_unlocked() => Some("locked — excluded from the sim"),
        Some(_) => None,
    }
}

/// Stub card for a stale chamber entry ([`chamber_entry_issue`]): shows the
/// name, the reason it can't run, and a remove button to free the slot.
/// Returns true when remove is clicked.
fn show_ghost_card(ui: &mut egui::Ui, name: &str, reason: &str) -> bool {
    let mut remove = false;
    egui::Frame::new()
        .fill(style::BG_SURFACE)
        .stroke(egui::Stroke::new(1.0, style::WARNING))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_width(CARD_WIDTH);
            ui.vertical(|ui| {
                ui.label(RichText::new(name).color(style::TEXT_BRIGHT).size(13.0).strong());
                ui.label(RichText::new(reason).color(style::WARNING).size(11.0));
                remove = ui.button(RichText::new("Remove from chamber").size(11.0)).clicked();
            });
        });
    remove
}

/// One chamber pet card (~240px). Reuses `ChamberPet` (`cp`) for the numbers.
///
/// `pad_to` is the row's tallest natural content height (from last frame); the
/// card pads itself up to it so its row stays flush. Returns this card's own
/// natural content height (pre-padding) so the caller can track the row max.
fn show_pet_card(
    ui: &mut egui::Ui,
    state: &mut ChamberState,
    name: &str,
    cp: &ChamberPet,
    export: Option<&itrtg_models::ExportPet>,
    pad_to: f32,
    // Nightmare's malus (points) if he's in the chamber. Subtracted from every
    // *other* pet's displayed bonus; shown as the strength on Nightmare's own card.
    nightmare_malus: Option<f32>,
) -> f32 {
    egui::Frame::new()
        .fill(style::BG_SURFACE)
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(0x33, 0x33, 0x44)))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_width(CARD_WIDTH);
            // Force the card up to the row's tallest natural height (measured last
            // frame) so its row stays flush even when a sibling grows taller.
            if pad_to > 0.0 {
                ui.set_min_height(pad_to);
            }
            let is_nightmare = matches!(cp.special, Some(SpecialPet::Nightmare { .. }));
            let inner = ui.vertical(|ui| {
                // Name + a special tag describing the pet's campaign role.
                ui.horizontal(|ui| {
                    ui.label(RichText::new(name).color(style::TEXT_BRIGHT).size(13.0).strong());
                    let tag: Option<(String, egui::Color32)> = match cp.special {
                        Some(SpecialPet::Pandora { .. }) => {
                            Some(("amplifies deposit".into(), style::ACCENT))
                        }
                        Some(SpecialPet::Bag { .. }) => Some(("gifts the lowest".into(), style::ACCENT)),
                        Some(SpecialPet::Nightmare { .. }) => Some((
                            format!("−{:.2} to others", nightmare_malus.unwrap_or(0.0)),
                            style::WARNING,
                        )),
                        None => None,
                    };
                    if let Some((tag, color)) = tag {
                        ui.label(RichText::new(tag).color(color).size(10.0));
                    }
                });
                // Total growth + Growth bonus — reduced live by a chamber Nightmare's
                // malus (Nightmare's own bonus is not reduced).
                let display_bonus = if is_nightmare {
                    cp.campaign_bonus_pct
                } else {
                    cp.campaign_bonus_pct - nightmare_malus.unwrap_or(0.0)
                };
                ui.label(
                    RichText::new(format!(
                        "total {:.0}   +{:.0}% growth",
                        cp.growth * cp.growth_multiplier,
                        display_bonus
                    ))
                    .color(style::TEXT_NORMAL)
                    .size(11.0),
                );
                // Why the bonus is reduced (only on the affected pets).
                if !is_nightmare
                    && let Some(m) = nightmare_malus
                {
                    ui.label(
                        RichText::new(format!("Nightmare −{m:.2}"))
                            .color(style::WARNING)
                            .size(10.0),
                    );
                }
                // Passive/hr (read-only) + editable CL.
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("passive {:.1}/hr   ·   CL", cp.passive_per_hour))
                            .color(style::TEXT_MUTED)
                            .size(10.0),
                    );
                    if let Some(e) = export {
                        let mut cl = e.class_level;
                        if ui
                            .add(egui::DragValue::new(&mut cl).range(0..=200).speed(1.0))
                            .on_hover_text("Class level — drives the Adventurer campaign bonus")
                            .changed()
                        {
                            set_class_level(state, name, e, cl);
                        }
                    }
                });
                // Editable equipment (W / A / Ac).
                if let Some(e) = export {
                    weapon_editor(ui, state, name, e);
                    armor_editor(ui, state, name, e);
                    accessory_editor(ui, state, name, e);
                }
                // Override status + refresh (only when this pet has been edited).
                if state.overrides.contains_key(name) {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("edited").color(style::WARNING).size(10.0));
                        if ui
                            .small_button("Refresh from export")
                            .on_hover_text(
                                "Discard edits — reset equipment + CL to the current export",
                            )
                            .clicked()
                        {
                            state.overrides.remove(name);
                        }
                    });
                }
                // Editable target.
                ui.horizontal(|ui| {
                    ui.label(RichText::new("target:").color(style::TEXT_MUTED).size(10.0));
                    let mut target = state.targets.get(name).copied().unwrap_or(0);
                    if ui.add(egui::DragValue::new(&mut target).speed(100.0)).changed() {
                        if target == 0 {
                            state.targets.remove(name);
                        } else {
                            state.targets.insert(name.to_string(), target);
                        }
                    }
                });
            });
            // Report the *natural* content height (independent of the min above),
            // so the row max tracks the real tallest card and shrinks when it does.
            inner.response.rect.height()
        })
        .inner
}

/// Get (seeding if absent) the mutable override for a pet. Seeded from the given
/// (effective) export so a fresh override carries the pet's full loadout + CL;
/// every card editor funnels through here so the first edit of any field
/// captures the rest of the export unchanged.
fn override_mut<'a>(
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
fn set_class_level(state: &mut ChamberState, name: &str, eff: &ExportPet, cl: u32) {
    override_mut(state, name, eff).class_level = cl;
}

/// The four campaign sticks, weakest→strongest (the order their caps imply).
const STICKS: [&str; 4] =
    ["Walking Stick", "Journeying Stick", "Magic Stick", "Legendary Stick"];

/// Quality grades, weakest→strongest, for the stick quality picker.
const QUALITIES: [Quality; 9] = [
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
fn equip(name: &str, quality: Quality, upgrade: Option<u8>) -> Equipment {
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
fn is_stick(item: &Equipment) -> bool {
    STICKS.contains(&item.name.as_str())
}

/// The weapon a slot should hold when the user picks `name` from the weapon
/// menu, given what's currently equipped. Switching *between* sticks preserves
/// the current quality/upgrade; everything else gets its effective level (event
/// gear only does anything at SSS+20; the Magic Egg's level is irrelevant).
/// `name == "none"` clears the slot.
fn weapon_for(name: &str, current: Option<&Equipment>) -> Option<Equipment> {
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
fn weapon_kind_label(item: Option<&Equipment>) -> String {
    match item {
        None => "none".to_string(),
        Some(e) => e.name.clone(),
    }
}

/// Apply a weapon-menu pick to a pet's override.
fn set_weapon(state: &mut ChamberState, name: &str, eff: &ExportPet, choice: &str) {
    let new = weapon_for(choice, eff.loadout.weapon.as_ref());
    override_mut(state, name, eff).loadout.weapon = new;
}

/// A small `<label> <picker>` row. The picker's `clicked` value, if any, is the
/// option the user just chose this frame.
fn slot_picker(
    ui: &mut egui::Ui,
    id: (&str, &str),
    label: &str,
    selected_text: &str,
    options: &[(&str, bool)],
) -> Option<usize> {
    let mut chosen = None;
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(style::TEXT_MUTED).size(10.0).monospace());
        egui::ComboBox::from_id_salt(id)
            .width(150.0)
            .selected_text(RichText::new(selected_text).size(10.0))
            .show_ui(ui, |ui| {
                for (i, (opt, selected)) in options.iter().enumerate() {
                    if ui.selectable_label(*selected, RichText::new(*opt).size(10.0)).clicked() {
                        chosen = Some(i);
                    }
                }
            });
    });
    chosen
}

/// Weapon slot editor: the four sticks (with quality/upgrade), Candy Cane, Magic
/// Egg, or none. Writes the chosen weapon into the pet's override.
fn weapon_editor(ui: &mut egui::Ui, state: &mut ChamberState, name: &str, eff: &ExportPet) {
    let cur = eff.loadout.weapon.as_ref();
    let cur_name = cur.map(|e| e.name.as_str());
    // none, the four sticks, Candy Cane, Magic Egg.
    let opts: Vec<(&str, bool)> = std::iter::once(("none", cur_name.is_none()))
        .chain(STICKS.iter().map(|&s| (s, cur_name == Some(s))))
        .chain([("Candy Cane", cur_name == Some("Candy Cane"))])
        .chain([("Magic Egg", cur_name == Some("Magic Egg"))])
        .collect();
    let header = weapon_kind_label(cur);
    if let Some(i) = slot_picker(ui, (name, "weapon"), "W", &header, &opts) {
        set_weapon(state, name, eff, opts[i].0);
    }

    // Quality + upgrade, only for a stick.
    if let Some(w) = eff.loadout.weapon.as_ref().filter(|w| is_stick(w)) {
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            egui::ComboBox::from_id_salt((name, "wq"))
                .width(48.0)
                .selected_text(RichText::new(format!("{:?}", w.quality)).size(10.0))
                .show_ui(ui, |ui| {
                    for q in QUALITIES {
                        if ui
                            .selectable_label(w.quality == q, RichText::new(format!("{q:?}")).size(10.0))
                            .clicked()
                            && let Some(e) = override_mut(state, name, eff).loadout.weapon.as_mut()
                        {
                            e.quality = q;
                        }
                    }
                });
            ui.label(RichText::new("+").color(style::TEXT_MUTED).size(10.0));
            let mut up = w.upgrade_level.unwrap_or(0);
            if ui.add(egui::DragValue::new(&mut up).range(0..=20).speed(1.0)).changed()
                && let Some(e) = override_mut(state, name, eff).loadout.weapon.as_mut()
            {
                e.upgrade_level = Some(up);
            }
        });
    }
}

/// Armor slot editor: Merry Mantle (SSS+20) or none.
fn armor_editor(ui: &mut egui::Ui, state: &mut ChamberState, name: &str, eff: &ExportPet) {
    let cur_name = eff.loadout.armor.as_ref().map(|e| e.name.as_str());
    let opts = [
        ("none", cur_name.is_none()),
        ("Merry Mantle", cur_name == Some("Merry Mantle")),
    ];
    let header = eff.loadout.armor.as_ref().map_or("none", |e| e.name.as_str());
    if let Some(i) = slot_picker(ui, (name, "armor"), "A", header, &opts) {
        override_mut(state, name, eff).loadout.armor = match opts[i].0 {
            "Merry Mantle" => Some(equip("Merry Mantle", Quality::SSS, Some(20))),
            _ => None,
        };
    }
}

/// Accessory slot editor: Growing Love Pendant (passive growth), Christmas Boots
/// (SSS+20 campaign boost), or none.
fn accessory_editor(ui: &mut egui::Ui, state: &mut ChamberState, name: &str, eff: &ExportPet) {
    let cur_name = eff.loadout.accessory.as_ref().map(|e| e.name.as_str());
    let opts = [
        ("none", cur_name.is_none()),
        ("Growing Love Pendant", cur_name == Some("Growing Love Pendant")),
        ("Christmas Boots", cur_name == Some("Christmas Boots")),
    ];
    let header = eff.loadout.accessory.as_ref().map_or("none", |e| e.name.as_str());
    if let Some(i) = slot_picker(ui, (name, "acc"), "Ac", header, &opts) {
        override_mut(state, name, eff).loadout.accessory = match opts[i].0 {
            // The pendant gives passive growth, not a campaign boost — its level
            // is irrelevant, so a plain SSS is fine.
            "Growing Love Pendant" => Some(equip("Growing Love Pendant", Quality::SSS, None)),
            "Christmas Boots" => Some(equip("Christmas Boots", Quality::SSS, Some(20))),
            _ => None,
        };
    }
}

/// Fill the chamber with the top 10 unlocked pets by Growth-campaign bonus,
/// breaking ties by raw growth (both highest-first).
fn recommend_chamber(state: &mut ChamberState, data: &DataStore, ctx: &CampaignContext) {
    let mut ranked: Vec<(&str, f32, u64)> = data
        .merged
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
fn fmt_hours(h: u32) -> String {
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
fn recent_rate_per_cycle(result: &ChamberResult, name: &str) -> Option<f64> {
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
    let base_income = |c: &itrtg_planner::campaign::ChamberCycle| -> f64 {
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

fn show_results(ui: &mut egui::Ui, state: &ChamberState) {
    let Some(result) = &state.result else { return };
    egui::Frame::new()
        .fill(style::BG_SURFACE)
        .inner_margin(6.0)
        .show(ui, |ui| {
            // Actual elapsed hours summed from the (possibly varying) cycle lengths.
            let total_hours: u32 = result.trace.iter().map(|c| c.hours).sum();
            ui.label(
                RichText::new(format!(
                    "Ran {} cycles ({}) — {} target(s) reached.  (by growth gained)",
                    result.cycles,
                    fmt_hours(total_hours),
                    result.reached.len()
                ))
                .color(style::TEXT_BRIGHT)
                .size(12.0),
            );

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
            rows.sort_by(|a, b| {
                (b.2 - b.1).partial_cmp(&(a.2 - a.1)).unwrap_or(std::cmp::Ordering::Equal)
            });

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
            let cycle_base = |c: &itrtg_planner::campaign::ChamberCycle| -> f64 {
                c.contributions.iter().map(|(_, a)| a).sum()
            };

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
                ui.label(RichText::new(summary).color(style::TEXT_MUTED).size(11.0));
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
                ui.label(
                    RichText::new(format!(
                        "  by source — campaign +{camp:.0}  ·  passive +{pass:.0}  ·  feeding +{feed:.0}  ·  Gold Dragon +{gd:.0}"
                    ))
                    .color(style::ACCENT)
                    .size(11.0),
                );
            }

            for (name, start, final_g) in rows {
                let delta = final_g - start;
                let avg_contrib =
                    contrib.get(name.as_str()).copied().unwrap_or(0.0) / result.cycles.max(1) as f64;
                let reached = result.reached.iter().find(|(n, _)| n == name);
                let (status, color) = match (reached, state.targets.get(name)) {
                    (Some((_, cycle)), _) => {
                        // Elapsed hours = sum of the first `cycle` cycle lengths.
                        let h: u32 = result.trace.iter().take(*cycle as usize).map(|c| c.hours).sum();
                        (format!("\u{2713} target at cycle {cycle} ({})", fmt_hours(h)), style::SUCCESS)
                    }
                    (None, Some(t)) => {
                        // Linear-extrapolate from this pet's *recent* growth/cycle
                        // (rewards trend up, so the whole-run average lands late) to
                        // estimate when it would reach the target (total from the run
                        // start, so it lines up with the reached pets above).
                        let per_cycle = recent_rate_per_cycle(result, name)
                            .unwrap_or(delta / result.cycles.max(1) as f64);
                        let remaining = *t as f64 - final_g;
                        if per_cycle > 0.0 && remaining > 0.0 {
                            let more = (remaining / per_cycle).ceil();
                            let avg_cycle_h = total_hours as f64 / result.cycles.max(1) as f64;
                            let eta_h = (total_hours as f64 + more * avg_cycle_h).min(u32::MAX as f64);
                            let est_cycle = result.cycles as f64 + more;
                            (
                                format!("{final_g:.0}/{t} — est. cycle ~{est_cycle:.0} ({})", fmt_hours(eta_h as u32)),
                                style::WARNING,
                            )
                        } else {
                            (format!("{final_g:.0}/{t} (not reached)"), style::WARNING)
                        }
                    }
                    (None, None) => (String::new(), style::TEXT_MUTED),
                };
                ui.label(
                    RichText::new(format!(
                        "  {name}: {start:.0} \u{2B62} {final_g:.0}  (+{delta:.0})  contrib ~{avg_contrib:.0}/cyc{}{status}",
                        if status.is_empty() { "" } else { "  " }
                    ))
                    .color(color)
                    .size(11.0),
                );
                // Per-pet split: what made up this pet's gain.
                if let Some(b) = bd(name) {
                    ui.label(
                        RichText::new(format!(
                            "      campaign +{:.0}  passive +{:.0}  feeding +{:.0}  GD +{:.0}",
                            b.campaign, b.passive, b.feeding, b.gold_dragon
                        ))
                        .color(style::TEXT_MUTED)
                        .size(10.0),
                    );
                }
            }
        });
}

fn show_pet_picker(
    ui: &mut egui::Ui,
    state: &mut ChamberState,
    data: &DataStore,
    ctx: &CampaignContext,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Pets (by Growth bonus):").color(style::TEXT_MUTED).size(12.0));
        ui.add(egui::TextEdit::singleline(&mut state.search).hint_text("filter…").desired_width(140.0));
    });
    let needle = state.search.to_lowercase();

    // No inner scroll — the whole view scrolls as one region (see `show`).
    let mut pets: Vec<(&MergedPet, f32)> = data
        .merged
        .iter()
        .filter(|p| p.is_unlocked() && p.export.is_some())
        .filter(|p| needle.is_empty() || p.name.to_lowercase().contains(&needle))
        .map(|p| (p, p.campaign_bonus_for(CampaignType::Growth, ctx).unwrap_or(0.0)))
        .collect();
    // Highest Growth bonus first — what you'd want in a mature chamber.
    pets.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                b.0.export
                    .as_ref()
                    .map(|e| e.effective_growth())
                    .cmp(&a.0.export.as_ref().map(|e| e.effective_growth())),
            )
    });

    for (pet, bonus) in pets {
        let name = pet.name.clone();
        let growth = pet.export.as_ref().map(|e| e.effective_growth()).unwrap_or(0);
        ui.horizontal(|ui| {
            let mut in_chamber = state.chamber.iter().any(|n| n == &name);
            let was = in_chamber;
            // Enforce the 10-pet cap: can't tick an 11th.
            let toggled = ui
                .add_enabled(was || state.chamber.len() < 10, egui::Checkbox::new(&mut in_chamber, ""))
                .changed();
            if toggled {
                if in_chamber {
                    state.chamber.push(name.clone());
                } else {
                    state.chamber.retain(|n| n != &name);
                }
            }
            ui.label(RichText::new(&name).color(style::TEXT_NORMAL).size(12.0));
            ui.label(
                RichText::new(format!("+{bonus:.0}%  ({growth})"))
                    .color(style::TEXT_MUTED)
                    .size(10.0),
            );
        });
    }
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
        use itrtg_planner::campaign::{ChamberCycle, GrowthBreakdown};

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
