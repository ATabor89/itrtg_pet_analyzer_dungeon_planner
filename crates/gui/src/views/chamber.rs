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
use itrtg_models::CampaignType;
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

    /// Pet-picker search filter — ephemeral.
    #[serde(skip)]
    pub search: String,
    /// Last simulation result — ephemeral.
    #[serde(skip)]
    pub result: Option<ChamberResult>,
}

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
            search: String::new(),
            result: None,
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
    }

    pub fn write_into(&self, state: &mut crate::state::AppState) {
        // Clone only the persisted fields (drop the ephemeral search/result).
        state.chamber = ChamberState {
            chamber: self.chamber.clone(),
            targets: self.targets.clone(),
            pendant: self.pendant.clone(),
            hours: self.hours,
            max_cycles: self.max_cycles,
            pandora_feedings: self.pandora_feedings,
            upc_pct: self.upc_pct,
            search: String::new(),
            result: None,
        };
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
    let growth = pet.export.as_ref()?.growth as f64;
    let bonus = pet.campaign_bonus_for(CampaignType::Growth, ctx).unwrap_or(0.0);
    let mut passive = rates.moai_per_hour;
    if state.pendant.iter().any(|n| n == &pet.name) {
        passive += rates.pendant_per_hour();
    }
    let special = match pet.name.as_str() {
        "Pandora's Box" => Some(SpecialPet::Pandora { feedings: state.pandora_feedings }),
        "Bag" => Some(SpecialPet::Bag {
            token_improved: pet.export.as_ref().is_some_and(|e| e.improved),
        }),
        _ => None,
    };
    Some(ChamberPet {
        name: pet.name.clone(),
        growth,
        campaign_bonus_pct: bonus,
        passive_per_hour: passive,
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
        ui.label(RichText::new("Max cycles:").color(style::TEXT_MUTED).size(12.0));
        ui.add(egui::DragValue::new(&mut state.max_cycles).range(1..=1_000_000).speed(50.0));
        ui.separator();
        ui.label(RichText::new("UPC %:").color(style::TEXT_MUTED).size(12.0));
        ui.add(egui::DragValue::new(&mut state.upc_pct).range(0.0..=100.0).speed(1.0));
        ui.separator();
        ui.label(RichText::new("Pandora feedings:").color(style::TEXT_MUTED).size(12.0));
        ui.add(egui::DragValue::new(&mut state.pandora_feedings).range(0..=20));
    });

    ui.horizontal(|ui| {
        let in_chamber = state.chamber.len();
        if ui
            .button(RichText::new("\u{25B6} Run simulation").size(13.0))
            .clicked()
        {
            let mut roster = build_roster(data, ctx, rates, state);
            state.result =
                Some(simulate_growth_chamber(&mut roster, state.hours, state.upc_pct, state.max_cycles));
        }
        ui.label(
            RichText::new(format!(
                "{in_chamber}/10 in chamber{}",
                if in_chamber > 10 { " (over the 10-pet cap!)" } else { "" }
            ))
            .color(if in_chamber > 10 { style::WARNING } else { style::TEXT_MUTED })
            .size(11.0),
        );
    });

    ui.add_space(4.0);
    show_results(ui, state);
    ui.separator();
    show_pet_picker(ui, state, data, ctx);
}

fn show_results(ui: &mut egui::Ui, state: &ChamberState) {
    let Some(result) = &state.result else { return };
    egui::Frame::new()
        .fill(style::BG_SURFACE)
        .inner_margin(6.0)
        .show(ui, |ui| {
            let hit = result.reached.len();
            ui.label(
                RichText::new(format!("Ran {} cycles — {hit} target(s) reached.", result.cycles))
                    .color(style::TEXT_BRIGHT)
                    .size(12.0),
            );
            for (name, cycle) in &result.reached {
                let wall = *cycle as f64 * state.hours as f64;
                let final_growth = result
                    .final_growth
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, g)| *g)
                    .unwrap_or(0.0);
                ui.label(
                    RichText::new(format!(
                        "  {name}: reached at cycle {cycle} (~{:.0} h), final growth {final_growth:.0}",
                        wall
                    ))
                    .color(style::SUCCESS)
                    .size(11.0),
                );
            }
            // Tracked-but-unreached pets.
            for (name, target) in &state.targets {
                if !result.reached.iter().any(|(n, _)| n == name) {
                    let final_growth = result
                        .final_growth
                        .iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, g)| *g);
                    if let Some(g) = final_growth {
                        ui.label(
                            RichText::new(format!(
                                "  {name}: not reached — {g:.0} / {target} after {} cycles",
                                result.cycles
                            ))
                            .color(style::WARNING)
                            .size(11.0),
                        );
                    }
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
        ui.label(RichText::new("Pets:").color(style::TEXT_MUTED).size(12.0));
        ui.add(egui::TextEdit::singleline(&mut state.search).hint_text("filter…").desired_width(140.0));
    });
    let needle = state.search.to_lowercase();

    egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
        let mut pets: Vec<&MergedPet> = data
            .merged
            .iter()
            .filter(|p| p.is_unlocked() && p.export.is_some())
            .filter(|p| needle.is_empty() || p.name.to_lowercase().contains(&needle))
            .collect();
        // Lowest growth first — the recipients and rush candidates.
        pets.sort_by_key(|p| p.export.as_ref().map(|e| e.growth).unwrap_or(0));

        for pet in pets {
            let name = pet.name.clone();
            let growth = pet.export.as_ref().map(|e| e.growth).unwrap_or(0);
            let bonus = pet.campaign_bonus_for(CampaignType::Growth, ctx).unwrap_or(0.0);
            ui.horizontal(|ui| {
                let mut in_chamber = state.chamber.iter().any(|n| n == &name);
                if ui.checkbox(&mut in_chamber, "").changed() {
                    if in_chamber {
                        state.chamber.push(name.clone());
                    } else {
                        state.chamber.retain(|n| n != &name);
                    }
                }
                ui.label(RichText::new(&name).color(style::TEXT_NORMAL).size(12.0));
                ui.label(
                    RichText::new(format!("{growth} (+{bonus:.0}%)"))
                        .color(style::TEXT_MUTED)
                        .size(10.0),
                );

                let mut pendant = state.pendant.iter().any(|n| n == &name);
                if ui.checkbox(&mut pendant, RichText::new("pendant").size(10.0)).changed() {
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
