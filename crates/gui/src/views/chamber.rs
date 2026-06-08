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
use itrtg_models::{CampaignType, ExportPet, Loadout, MAGIC_EGG_GROWTH_MULT};
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

/// Food types, lowest→highest growth. Index matches [`ChamberState::food_values`].
pub const FOODS: [&str; 5] = ["Free", "Puny", "Strong", "Mighty", "Chocolate"];

impl Default for ChamberState {
    fn default() -> Self {
        Self {
            chamber: Vec::new(),
            targets: BTreeMap::new(),
            hours: 12,
            max_cycles: 5_000,
            pandora_feedings: 0,
            upc_pct: 0.0,
            food_choice: 4, // Chocolate
            food_values: [1.3, 2.6, 5.19, 7.79, 10.38],
            gold_dragon_food: None,
            run_until_targets: false,
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
        self.upc_pct = src.upc_pct;
        self.food_choice = src.food_choice.min(FOODS.len() - 1);
        self.food_values = src.food_values;
        self.gold_dragon_food = src.gold_dragon_food.filter(|&i| i < FOODS.len());
        self.run_until_targets = src.run_until_targets;
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
            upc_pct: self.upc_pct,
            food_choice: self.food_choice,
            food_values: self.food_values,
            gold_dragon_food: self.gold_dragon_food,
            run_until_targets: self.run_until_targets,
            overrides: self.overrides.clone(),
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
    // Base growth is the accumulator; the Magic Egg (+30%) makes total growth that
    // the campaign reads. (Patreon-God-Challenge would multiply here too once we
    // track it — the player has none yet.)
    let growth = export.growth as f64;
    let growth_multiplier = if export.has_magic_egg() { MAGIC_EGG_GROWTH_MULT } else { 1.0 };
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
    show_pet_cards(ui, state, data, ctx, rates);
    ui.separator();
    show_pet_picker(ui, state, data, ctx);
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
                let Some(pet) = data.merged.iter().find(|p| &p.name == name) else { continue };
                let Some(cp) = chamber_pet(pet, ctx, rates, state) else { continue };
                // The card shows (and edits) the *effective* export: the live
                // export with any per-pet override applied.
                let eff = effective_export(pet, state);
                let natural = show_pet_card(ui, state, name, &cp, eff.as_ref(), pad_to);
                row_max = row_max.max(natural);
            }
        });
        new_heights.push(row_max);
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
) -> f32 {
    egui::Frame::new()
        .fill(style::BG_SURFACE)
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(0x33, 0x33, 0x44)))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_width(CARD_WIDTH);
            // Top of the content area — measure the card's natural height against it.
            let content_top = ui.min_rect().bottom();
            ui.vertical(|ui| {
                // Name + special tag.
                ui.horizontal(|ui| {
                    ui.label(RichText::new(name).color(style::TEXT_BRIGHT).size(13.0).strong());
                    let tag = match cp.special {
                        Some(SpecialPet::Pandora { .. }) => Some("Pandora"),
                        Some(SpecialPet::Bag { .. }) => Some("Bag"),
                        None => None,
                    };
                    if let Some(tag) = tag {
                        ui.label(RichText::new(tag).color(style::ACCENT).size(10.0));
                    }
                });
                // Total growth + Growth bonus.
                ui.label(
                    RichText::new(format!(
                        "total {:.0}   +{:.0}% growth",
                        cp.growth * cp.growth_multiplier,
                        cp.campaign_bonus_pct
                    ))
                    .color(style::TEXT_NORMAL)
                    .size(11.0),
                );
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
                // Read-only equipment (W / A / Ac) — editors arrive in phase 2b.
                if let Some(e) = export {
                    ui.label(
                        RichText::new(format!(
                            "W: {}\nA: {}\nAc: {}",
                            equip_label(e.loadout.weapon.as_ref()),
                            equip_label(e.loadout.armor.as_ref()),
                            equip_label(e.loadout.accessory.as_ref()),
                        ))
                        .color(style::TEXT_MUTED)
                        .size(10.0),
                    );
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
            // The natural content height, then pad up to the row's tallest.
            let natural = ui.min_rect().bottom() - content_top;
            if pad_to > natural {
                ui.add_space(pad_to - natural);
            }
            natural
        })
        .inner
}

/// Set a pet's effective class level via its override, seeding the override from
/// the given (effective) export the first time so it carries the pet's full
/// loadout — later gear edits then mutate the same entry.
fn set_class_level(state: &mut ChamberState, name: &str, eff: &ExportPet, cl: u32) {
    state
        .overrides
        .entry(name.to_string())
        .or_insert_with(|| PetOverride {
            loadout: eff.loadout.clone(),
            class_level: eff.class_level,
        })
        .class_level = cl;
}

/// Compact label for an equipment slot: name plus quality/upgrade for gear that
/// carries an upgrade level (e.g. `Magic Stick SSS+10`); `—` when empty.
fn equip_label(item: Option<&itrtg_models::Equipment>) -> String {
    match item {
        None => "—".to_string(),
        Some(e) => match e.upgrade_level {
            Some(u) => format!("{} {:?}+{u}", e.name, e.quality),
            None => e.name.clone(),
        },
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
            });
        }
    });
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
        use itrtg_models::{CampaignInputs, CampaignOverrides, Class};

        // An Adventurer's all-campaign bonus is (2 + evo)%·CL; a generic pet has
        // evo 0, so 2%/CL. The override must flow CL through campaign_bonus_for.
        let mut adv = export_with(1000, 5);
        adv.class = Some(Class::Adventurer);
        let pet = merged("Advy", adv);

        let roster = vec![pet.clone()];
        let overrides = CampaignOverrides::default();
        let inputs = CampaignInputs::default();
        let ctx = CampaignContext {
            overrides: &overrides,
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
}
