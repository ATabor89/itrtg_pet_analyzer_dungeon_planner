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
use itrtg_models::{CampaignType, MAGIC_EGG_GROWTH_MULT};
use itrtg_planner::campaign::{
    simulate_growth_chamber, ChamberPet, ChamberResult, SpecialPet,
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
    /// Canonical names of pets wearing a Growing Love Pendant.
    pub pendant: Vec<String>,
    /// Campaign length, 1–12 h.
    pub hours: u32,
    /// Maximum cycles to simulate before giving up.
    pub max_cycles: u32,
    /// Pandora's Box feedings — drives its campaign-total bonus. (The feeding
    /// behaviour over many rounds is still being pinned down; this is the hook.)
    pub pandora_feedings: u32,
    /// UPC bonus % (`5 · Ultimate Pet Challenges`). Manual for now.
    pub upc_pct: f64,
    /// Which food is fed to every pet — index into [`FOODS`]. Drives per-feeding
    /// growth (every pet is fed `floor(hours/3)` times per cycle).
    pub food_choice: usize,
    /// Effective growth-per-feeding for each food type (DPC + fishing baked in;
    /// see `food_and_feedings.md`). Editable until we auto-derive them.
    pub food_values: [f64; 5],
    /// Food fed to **Gold Dragon** (index into [`FOODS`]); `None` = not fed / not
    /// owned. Feeding him gives **every** pet 25% of the growth he gains — a big,
    /// campaign-independent source. Best fed chocolate.
    pub gold_dragon_food: Option<usize>,
    /// Run until every tracked pet hits its target (vs a fixed cycle count).
    pub run_until_targets: bool,

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
}

/// Food types, lowest→highest growth. Index matches [`ChamberState::food_values`].
pub const FOODS: [&str; 5] = ["Free", "Puny", "Strong", "Mighty", "Chocolate"];

impl Default for ChamberState {
    fn default() -> Self {
        Self {
            chamber: Vec::new(),
            targets: BTreeMap::new(),
            pendant: Vec::new(),
            hours: 12,
            max_cycles: 5_000,
            pandora_feedings: 0,
            upc_pct: 0.0,
            food_choice: 4, // Chocolate
            food_values: [1.3, 2.6, 5.19, 7.79, 10.38],
            gold_dragon_food: None,
            run_until_targets: false,
            search: String::new(),
            result: None,
            last_starts: BTreeMap::new(),
        }
    }
}

impl ChamberState {
    pub fn apply_app_state(&mut self, state: &crate::state::AppState) {
        let src = &state.chamber;
        self.chamber = src.chamber.clone();
        self.targets = src.targets.clone();
        self.pendant = src.pendant.clone();
        self.hours = src.hours;
        self.max_cycles = src.max_cycles;
        self.pandora_feedings = src.pandora_feedings;
        self.upc_pct = src.upc_pct;
        self.food_choice = src.food_choice.min(FOODS.len() - 1);
        self.food_values = src.food_values;
        self.gold_dragon_food = src.gold_dragon_food.filter(|&i| i < FOODS.len());
        self.run_until_targets = src.run_until_targets;
    }

    pub fn write_into(&self, state: &mut crate::state::AppState) {
        // Persisted fields only (drop the ephemeral search/result/last_starts).
        state.chamber = ChamberState {
            chamber: self.chamber.clone(),
            targets: self.targets.clone(),
            pendant: self.pendant.clone(),
            hours: self.hours,
            max_cycles: self.max_cycles,
            pandora_feedings: self.pandora_feedings,
            upc_pct: self.upc_pct,
            food_choice: self.food_choice,
            food_values: self.food_values,
            gold_dragon_food: self.gold_dragon_food,
            run_until_targets: self.run_until_targets,
            ..Default::default()
        };
    }

    /// Effective growth per feeding for the chosen food.
    fn food_growth(&self) -> f64 {
        self.food_values.get(self.food_choice).copied().unwrap_or(0.0)
    }

    /// Per-feeding growth every pet gets from a Gold Dragon feeding (25% of his
    /// food's growth). 0 if Gold Dragon isn't being fed.
    fn gold_dragon_broadcast(&self) -> f64 {
        self.gold_dragon_food
            .and_then(|i| self.food_values.get(i))
            .map_or(0.0, |&v| 0.25 * v)
    }

    /// Total growth each pet gains per feeding: its own food plus Gold Dragon's
    /// 25% broadcast.
    fn per_feeding_growth(&self) -> f64 {
        self.food_growth() + self.gold_dragon_broadcast()
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

fn chamber_pet(
    pet: &MergedPet,
    ctx: &CampaignContext,
    rates: &GrowthRates,
    state: &ChamberState,
) -> Option<ChamberPet> {
    let export = pet.export.as_ref()?;
    // Base growth is the accumulator; the Magic Egg (+30%) makes total growth that
    // the campaign reads. (Patreon-God-Challenge would multiply here too once we
    // track it — the player has none yet.)
    let growth = export.growth as f64;
    let growth_multiplier = if export.has_magic_egg() { MAGIC_EGG_GROWTH_MULT } else { 1.0 };
    let bonus = pet.campaign_bonus_for(CampaignType::Growth, ctx).unwrap_or(0.0);
    let mut passive = rates.moai_per_hour;
    if state.pendant.iter().any(|n| n == &pet.name) {
        passive += rates.pendant_per_hour();
    }
    let special = match pet.name.as_str() {
        "Pandora's Box" => Some(SpecialPet::Pandora { feedings: state.pandora_feedings }),
        "Bag" => Some(SpecialPet::Bag { token_improved: export.improved }),
        _ => None,
    };
    Some(ChamberPet {
        name: pet.name.clone(),
        growth,
        growth_multiplier,
        campaign_bonus_pct: bonus,
        passive_per_hour: passive,
        food_per_feeding: state.per_feeding_growth(),
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
        ui.label(RichText::new("UPC %:").color(style::TEXT_MUTED).size(12.0));
        ui.add(egui::DragValue::new(&mut state.upc_pct).range(0.0..=100.0).speed(1.0));
        ui.separator();
        ui.label(RichText::new("Pandora feedings:").color(style::TEXT_MUTED).size(12.0));
        ui.add(egui::DragValue::new(&mut state.pandora_feedings).range(0..=20));
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
        ui.label(RichText::new("growth/feeding:").color(style::TEXT_MUTED).size(11.0));
        let choice = state.food_choice.min(FOODS.len() - 1);
        ui.add(egui::DragValue::new(&mut state.food_values[choice]).speed(0.1));
        ui.label(
            RichText::new(format!("({} feedings/cycle)", state.hours / 3))
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
                state.hours,
                state.upc_pct,
                state.max_cycles,
                state.run_until_targets,
            ));
        }
    });

    ui.add_space(4.0);
    show_results(ui, state);
    ui.separator();
    show_pet_picker(ui, state, data, ctx);
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

fn show_results(ui: &mut egui::Ui, state: &ChamberState) {
    let Some(result) = &state.result else { return };
    egui::Frame::new()
        .fill(style::BG_SURFACE)
        .inner_margin(6.0)
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!(
                    "Ran {} cycles (~{:.0} h) — {} target(s) reached.  (by growth gained)",
                    result.cycles,
                    result.cycles as f64 * state.hours as f64,
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

            // Summary stats: average growth/pet/cycle and the reward trend.
            if !rows.is_empty() && result.cycles > 0 {
                let total_gain: f64 = rows.iter().map(|(_, s, f)| f - s).sum();
                let mut summary = format!(
                    "  avg +{:.1}/pet/cycle  (chamber total +{:.0})",
                    total_gain / rows.len() as f64 / result.cycles as f64,
                    total_gain
                );
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

            for (name, start, final_g) in rows {
                let delta = final_g - start;
                let avg_contrib =
                    contrib.get(name.as_str()).copied().unwrap_or(0.0) / result.cycles.max(1) as f64;
                let reached = result.reached.iter().find(|(n, _)| n == name);
                let (status, color) = match (reached, state.targets.get(name)) {
                    (Some((_, cycle)), _) => (format!("\u{2713} target at cycle {cycle}"), style::SUCCESS),
                    (None, Some(t)) => (format!("{final_g:.0}/{t} (not reached)"), style::WARNING),
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

    egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
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

                let mut pendant = state.pendant.iter().any(|n| n == &name);
                let had_pendant = pendant;
                // At most two pendants exist in-game.
                if ui
                    .add_enabled(
                        had_pendant || state.pendant.len() < 2,
                        egui::Checkbox::new(&mut pendant, RichText::new("pendant").size(10.0)),
                    )
                    .changed()
                {
                    if pendant {
                        state.pendant.push(name.clone());
                    } else {
                        state.pendant.retain(|n| n != &name);
                    }
                }

                ui.label(RichText::new("target:").color(style::TEXT_MUTED).size(10.0));
                let mut target = state.targets.get(&name).copied().unwrap_or(0);
                if ui.add(egui::DragValue::new(&mut target).speed(100.0)).changed() {
                    if target == 0 {
                        state.targets.remove(&name);
                    } else {
                        state.targets.insert(name.clone(), target);
                    }
                }
            });
        }
    });
}
