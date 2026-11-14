//! Growth-chamber view: configure a set of pets locked into the Growth campaign,
//! run the multi-cycle simulation, and see how many cycles each tracked pet
//! needs to reach its target.
//!
//! The state, the roster bridge, the per-pet what-if overrides, and the result
//! report all live in [`itrtg_planner::chamber`] (shared with the Slint front
//! end); this file is only the egui rendering.

use eframe::egui::{self, RichText};
use itrtg_models::{CampaignType, ExportPet};
use itrtg_planner::campaign::{ChamberPet, SpecialPet};
pub use itrtg_planner::chamber::ChamberState;
use itrtg_planner::chamber::{
    chamber_entry_issue, chamber_nightmare_malus, chamber_pet, effective_export,
    growth_bonus_breakdown, is_stick, override_mut, picker_entries, recommend_chamber,
    remove_entries, report, set_accessory, set_armor, set_class_level, set_membership,
    set_target, set_weapon, weapon_kind_label, Tone, ACCESSORY_CHOICES, ARMOR_CHOICES, FOODS,
    QUALITIES, REBIRTH_UNITS, WEAPON_CHOICES,
};
use itrtg_planner::growth::GrowthRates;
use itrtg_planner::merge::{CampaignBonusBreakdown, CampaignContext};

use crate::data::DataStore;
use crate::style;
use super::widgets::DragValueExt;

/// Bridges the shared [`ChamberState`] to this app's persisted `AppState`.
pub trait ChamberStateExt {
    fn apply_app_state(&mut self, state: &crate::state::AppState);
    fn write_into(&self, state: &mut crate::state::AppState);
}

impl ChamberStateExt for ChamberState {
    fn apply_app_state(&mut self, state: &crate::state::AppState) {
        self.load_persisted(&state.chamber);
    }

    fn write_into(&self, state: &mut crate::state::AppState) {
        state.chamber = self.persisted();
    }
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
        ui.add(egui::DragValue::new(&mut state.hours).range(1..=12).clearable());
        ui.separator();
        ui.label(RichText::new("UPC:").color(style::TEXT_MUTED).size(12.0))
            .on_hover_text("Ultimate Pet Challenges completed — each adds +5% to every pet's campaign contribution (capped at 100%). Auto-filled when you import a Main-stats export.");
        ui.add(egui::DragValue::new(&mut state.upc_done).range(0..=state.upc_max).clearable());
        ui.label(
            RichText::new(format!("/ {}  \u{2B62} +{:.0}%", state.upc_max, state.upc_pct()))
                .color(style::TEXT_MUTED)
                .size(11.0),
        );
        ui.separator();
        ui.label(RichText::new("PGC:").color(style::TEXT_MUTED).size(12.0))
            .on_hover_text("Patreon God Challenges completed — +1% growth per completion, doubled once all are done (24/25 → ×1.24, 25/25 → ×1.50). Stacks with the Magic Egg. Auto-filled when you import a Main-stats export.");
        ui.add(egui::DragValue::new(&mut state.pgc_done).range(0..=state.pgc_max).clearable());
        ui.label(
            RichText::new(format!("/ {}  \u{2B62} ×{:.2} growth", state.pgc_max, state.pgc_mult()))
                .color(style::TEXT_MUTED)
                .size(11.0),
        );
        ui.separator();
        ui.label(RichText::new("Pandora feedings:").color(style::TEXT_MUTED).size(12.0));
        ui.add(egui::DragValue::new(&mut state.pandora_feedings).range(0..=20).clearable())
            .on_hover_text("Pandora's starting feeding count. It climbs as she's fed each cycle (bonus caps at 20) and resets at the start of each rebirth.");
        // Global counterpart of a card's "Refresh from export": shown when any
        // input auto-filled from a Main-stats import (UPC / PGC / DPC multi /
        // Fish Power / Fishing level) has been edited away from it.
        if state.deviates_from_import() {
            ui.separator();
            ui.label(RichText::new("edited").color(style::WARNING).size(10.0));
            if ui
                .small_button("Reset to import")
                .on_hover_text(
                    "Discard edits — reset UPC, PGC, DPC multi, Fish Power and Fishing level to their imported Main-stats values",
                )
                .clicked()
            {
                state.reset_to_import();
            }
        }
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
                .max_decimals(0)
                .clearable(),
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
                    .max_decimals(2)
                    .clearable(),
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
                .max_decimals(0)
                .clearable(),
        );
        ui.label(RichText::new("Fishing level:").color(style::TEXT_MUTED).size(12.0))
            .on_hover_text("Milestones: +10% at level 15, another +10% at 27.");
        ui.add(egui::DragValue::new(&mut state.fishing_level).range(0..=100).clearable());
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
        ui.add(egui::DragValue::new(&mut state.max_cycles).range(1..=1_000_000).speed(50.0).clearable());
        ui.separator();
        ui.label(RichText::new("Adventurer XP ×:").color(style::TEXT_MUTED).size(12.0))
            .on_hover_text(
                "Multiplier on Adventurer campaign class XP (pet-stone / ChP Adventurer-XP purchases). Base XP per cycle is 250·(1 + total growth/20000)·hours. 1.0 = no purchases; 0 turns class-level modelling off.",
            );
        ui.add(
            egui::DragValue::new(&mut state.adv_xp_mult)
                .range(0.0..=1000.0)
                .speed(0.1)
                .max_decimals(2)
                .clearable(),
        );
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
            recommend_chamber(state, &data.merged, ctx);
        }
        ui.label(
            RichText::new(format!("{n}/10 in chamber{}", if over { " — too many!" } else { "" }))
                .color(if over { style::WARNING } else { style::TEXT_MUTED })
                .size(11.0),
        );

        if run {
            itrtg_planner::chamber::run(state, &data.merged, ctx, rates);
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
    let nightmare_malus = chamber_nightmare_malus(state, &data.merged);

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
                let breakdown = growth_bonus_breakdown(pet, ctx, state);
                let natural = show_pet_card(
                    ui, state, name, &cp, &breakdown, eff.as_ref(), pad_to, nightmare_malus,
                );
                row_max = row_max.max(natural);
            }
        });
        new_heights.push(row_max);
    }

    remove_entries(state, &data.merged, &removed);

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
#[allow(clippy::too_many_arguments)]
fn show_pet_card(
    ui: &mut egui::Ui,
    state: &mut ChamberState,
    name: &str,
    cp: &ChamberPet,
    // The Growth bonus split by source; `cp.campaign_bonus_pct` is its total.
    breakdown: &CampaignBonusBreakdown,
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
                // Where the bonus comes from — only when a flat layer (class /
                // equipment) contributes; otherwise the bonus *is* the innate.
                if breakdown.class.is_some() || breakdown.equipment.is_some() {
                    let innate = breakdown
                        .innate
                        .get(&CampaignType::Growth)
                        .copied()
                        .unwrap_or(0.0);
                    let mut parts = vec![format!("innate {innate:+.0}")];
                    if let Some(v) = breakdown.class {
                        parts.push(format!("class {v:+.0}"));
                    }
                    if let Some(v) = breakdown.equipment {
                        parts.push(format!("equip {v:+.0}"));
                    }
                    ui.label(
                        RichText::new(parts.join("  ·  "))
                            .color(style::TEXT_MUTED)
                            .size(10.0),
                    );
                }
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
                            .add(egui::DragValue::new(&mut cl).range(0..=200).speed(1.0).clearable())
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
                    if ui.add(egui::DragValue::new(&mut target).speed(100.0).clearable()).changed() {
                        set_target(state, name, target);
                    }
                });
            });
            // Report the *natural* content height (independent of the min above),
            // so the row max tracks the real tallest card and shrinks when it does.
            inner.response.rect.height()
        })
        .inner
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
    let cur_name = cur.map_or("none", |e| e.name.as_str());
    // none, the four sticks, Candy Cane, Magic Egg.
    let opts: Vec<(&str, bool)> = WEAPON_CHOICES.iter().map(|&s| (s, cur_name == s)).collect();
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
            if ui.add(egui::DragValue::new(&mut up).range(0..=20).speed(1.0).clearable()).changed()
                && let Some(e) = override_mut(state, name, eff).loadout.weapon.as_mut()
            {
                e.upgrade_level = Some(up);
            }
        });
    }
}

/// Armor slot editor: Merry Mantle (SSS+20) or none.
fn armor_editor(ui: &mut egui::Ui, state: &mut ChamberState, name: &str, eff: &ExportPet) {
    let cur_name = eff.loadout.armor.as_ref().map_or("none", |e| e.name.as_str());
    let opts: Vec<(&str, bool)> = ARMOR_CHOICES.iter().map(|&s| (s, cur_name == s)).collect();
    if let Some(i) = slot_picker(ui, (name, "armor"), "A", cur_name, &opts) {
        set_armor(state, name, eff, opts[i].0);
    }
}

/// Accessory slot editor: Growing Love Pendant (passive growth), Christmas Boots
/// (SSS+20 campaign boost), or none.
fn accessory_editor(ui: &mut egui::Ui, state: &mut ChamberState, name: &str, eff: &ExportPet) {
    let cur_name = eff.loadout.accessory.as_ref().map_or("none", |e| e.name.as_str());
    let opts: Vec<(&str, bool)> = ACCESSORY_CHOICES.iter().map(|&s| (s, cur_name == s)).collect();
    if let Some(i) = slot_picker(ui, (name, "acc"), "Ac", cur_name, &opts) {
        set_accessory(state, name, eff, opts[i].0);
    }
}

/// The last run's report (built by [`itrtg_planner::chamber::report`]).
fn show_results(ui: &mut egui::Ui, state: &ChamberState) {
    let lines = report(state);
    if lines.is_empty() {
        return;
    }
    egui::Frame::new()
        .fill(style::BG_SURFACE)
        .inner_margin(6.0)
        .show(ui, |ui| {
            for line in lines {
                let color = match line.tone {
                    Tone::Bright => style::TEXT_BRIGHT,
                    Tone::Normal => style::TEXT_NORMAL,
                    Tone::Muted => style::TEXT_MUTED,
                    Tone::Accent => style::ACCENT,
                    Tone::Success => style::SUCCESS,
                    Tone::Warning => style::WARNING,
                };
                let resp = ui.label(RichText::new(line.text).color(color).size(line.size));
                if let Some(hint) = line.hint {
                    resp.on_hover_text(hint);
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

    // No inner scroll — the whole view scrolls as one region (see `show`).
    for entry in picker_entries(&data.merged, ctx, &state.search) {
        let name = entry.pet.name.clone();
        ui.horizontal(|ui| {
            let mut in_chamber = state.chamber.iter().any(|n| n == &name);
            let was = in_chamber;
            // Enforce the 10-pet cap: can't tick an 11th.
            let toggled = ui
                .add_enabled(was || state.chamber.len() < 10, egui::Checkbox::new(&mut in_chamber, ""))
                .changed();
            if toggled {
                set_membership(state, &name, in_chamber);
            }
            ui.label(RichText::new(&name).color(style::TEXT_NORMAL).size(12.0));
            ui.label(
                RichText::new(format!("+{:.0}%  ({})", entry.bonus, entry.growth))
                    .color(style::TEXT_MUTED)
                    .size(10.0),
            );
        });
    }
}
