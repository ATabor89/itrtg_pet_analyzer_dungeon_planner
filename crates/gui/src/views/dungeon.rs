use eframe::egui::{self, Color32, RichText, CornerRadius, Stroke, StrokeKind, Ui, Vec2};
use itrtg_models::Dungeon;
use itrtg_planner::solver::{self, Assignment, CoverageKind, DungeonPlan, MatchQuality};

use crate::data::DataStore;
use crate::style;
use super::widgets;

// =============================================================================
// State
// =============================================================================

#[derive(Default)]
pub struct DungeonState {
    pub selected_dungeon: Option<Dungeon>,
    pub selected_depth: u8,
    pub plan: Option<DungeonPlan>,
}

const DUNGEONS: &[(Dungeon, &str)] = &[
    (Dungeon::Scrapyard, "Scrapyard"),
    (Dungeon::WaterTemple, "Water Temple"),
    (Dungeon::Volcano, "Volcano"),
    (Dungeon::Mountain, "Mountain"),
    (Dungeon::Forest, "Forest"),
];

// =============================================================================
// Rendering
// =============================================================================

pub fn show(ui: &mut Ui, state: &mut DungeonState, data: &DataStore) {
    if data.dungeon_recs.is_none() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(
                RichText::new("No dungeon recommendations loaded.")
                    .color(style::WARNING)
                    .size(16.0),
            );
            ui.label(
                RichText::new("Place dungeon_recommendations.yaml in the references directory.")
                    .color(style::TEXT_MUTED),
            );
        });
        return;
    }

    ui.horizontal(|ui| {
        // Dungeon selector
        ui.label(RichText::new("Dungeon:").color(style::TEXT_MUTED));
        let current_name = state
            .selected_dungeon
            .as_ref()
            .and_then(|d| DUNGEONS.iter().find(|(dd, _)| dd == d).map(|(_, n)| *n))
            .unwrap_or("Select...");

        egui::ComboBox::from_id_salt("dungeon_select")
            .selected_text(current_name)
            .show_ui(ui, |ui| {
                for (d, name) in DUNGEONS {
                    ui.selectable_value(&mut state.selected_dungeon, Some(*d), *name);
                }
            });

        ui.separator();

        // Depth selector
        ui.label(RichText::new("Depth:").color(style::TEXT_MUTED));
        for depth in 1..=3u8 {
            let selected = state.selected_depth == depth;
            let text = RichText::new(format!("D{depth}"))
                .color(if selected { style::ACCENT } else { style::TEXT_NORMAL });
            if ui
                .add(egui::Button::new(text).fill(if selected {
                    Color32::from_rgb(0x2a, 0x20, 0x40)
                } else {
                    style::BG_SURFACE
                }))
                .clicked()
            {
                state.selected_depth = depth;
                state.plan = None; // clear stale plan
            }
        }

        ui.separator();

        // Solve button
        let can_solve =
            state.selected_dungeon.is_some() && state.selected_depth > 0 && !data.merged.is_empty();
        if ui
            .add_enabled(
                can_solve,
                egui::Button::new(RichText::new("  Solve  ").color(style::TEXT_BRIGHT))
                    .fill(Color32::from_rgb(0x30, 0x20, 0x50)),
            )
            .clicked()
        {
            solve_plan(state, data);
        }

        if ui
            .add(egui::Button::new(RichText::new("Clear").color(style::TEXT_MUTED)))
            .clicked()
        {
            state.plan = None;
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    // Show plan results
    if let Some(plan) = &state.plan {
        show_plan(ui, plan);
    } else {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(
                RichText::new("Select a dungeon and depth, then click Solve.")
                    .color(style::TEXT_MUTED),
            );
        });
    }
}

fn solve_plan(state: &mut DungeonState, data: &DataStore) {
    let Some(dungeon) = state.selected_dungeon else { return };
    let Some(recs) = &data.dungeon_recs else { return };
    let Some(dungeon_data) = recs.dungeons.get(&dungeon) else { return };

    state.plan = Some(solver::solve(
        dungeon,
        state.selected_depth,
        dungeon_data,
        &data.merged,
    ));
}

fn show_plan(ui: &mut Ui, plan: &DungeonPlan) {
    // Party grid: 2 rows of 3
    ui.label(
        RichText::new("Recommended Party")
            .color(style::TEXT_BRIGHT)
            .size(16.0),
    );
    ui.label(
        RichText::new("Positions 1-3: Front Row  |  Positions 4-6: Back Row")
            .color(style::TEXT_MUTED)
            .size(11.0),
    );
    ui.add_space(4.0);

    let slot_width = ui.available_width().min(800.0) / 3.0 - 8.0;

    for row_label in ["Front Row", "Back Row"] {
        ui.label(
            RichText::new(row_label)
                .color(style::TEXT_MUTED)
                .strong()
                .size(12.0),
        );
        ui.horizontal(|ui| {
            let start = if row_label == "Front Row" { 0 } else { 3 };
            let end = start + 3;
            for i in start..end.min(plan.assignments.len()) {
                show_slot_card(ui, &plan.assignments[i], slot_width);
            }
        });
        ui.add_space(4.0);
    }

    // Warnings
    if !plan.warnings.is_empty() {
        ui.add_space(8.0);
        ui.separator();
        ui.label(
            RichText::new("Coverage Warnings")
                .color(style::WARNING)
                .size(14.0),
        );
        for warning in &plan.warnings {
            let kind_str = match warning.kind {
                CoverageKind::Trap => "Trap",
                CoverageKind::Event => "Event",
            };
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("  D{} {kind_str}:", warning.source_depth))
                        .color(style::WARNING)
                        .size(12.0),
                );
                ui.label(
                    RichText::new(&warning.name)
                        .color(style::TEXT_BRIGHT)
                        .size(12.0),
                );
                ui.label(
                    RichText::new(format!("— {}", warning.detail))
                        .color(style::TEXT_MUTED)
                        .size(12.0),
                );
            });
        }
    }
}

fn show_slot_card(ui: &mut Ui, slot: &solver::SlotAssignment, width: f32) {
    let height = 80.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());

    // Card background
    let bg = style::BG_SURFACE;
    ui.painter().rect_filled(rect, CornerRadius::same(6), bg);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(1.0, Color32::from_rgb(0x33, 0x33, 0x44)),
        StrokeKind::Outside,
    );

    let inner = rect.shrink(8.0);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::LEFT)));

    // Slot header: position + requirements
    child.horizontal(|ui| {
        ui.label(
            RichText::new(format!("#{}", slot.position + 1))
                .color(style::TEXT_MUTED)
                .size(11.0),
        );
        // Required class
        if let Some(class) = &slot.slot.class {
            widgets::class_label(ui, class);
        } else {
            ui.label(RichText::new("Any").color(style::TEXT_MUTED).size(12.0));
        }
        // Required element
        if let Some(el) = slot.slot.element {
            widgets::element_badge(ui, &el);
        }
    });

    // Assignment
    match &slot.assignment {
        Assignment::Filled { pet, quality } => {
            let (quality_text, quality_color) = match quality {
                MatchQuality::Exact => ("Exact", style::SUCCESS),
                MatchQuality::Evolvable => ("Evolvable", style::WARNING),
                MatchQuality::Reclassable => ("Reclass?", Color32::from_rgb(0xdd, 0x88, 0x44)),
                MatchQuality::Fallback => ("Fallback", style::ERROR),
            };

            child.horizontal(|ui| {
                ui.label(
                    RichText::new(&pet.name)
                        .color(style::TEXT_BRIGHT)
                        .size(13.0),
                );
                if let Some(el) = pet.element() {
                    widgets::element_badge(ui, &el);
                }
            });
            child.horizontal(|ui| {
                if let Some(class) = pet.evolved_class() {
                    widgets::class_label(ui, &class);
                } else {
                    ui.label(RichText::new("Unevolved").color(style::TEXT_MUTED).size(11.0));
                }
                ui.label(
                    RichText::new(format!("[{quality_text}]"))
                        .color(quality_color)
                        .size(10.0),
                );
            });
        }
        Assignment::Empty { suggestions } => {
            child.label(RichText::new("No pet available").color(style::ERROR).size(12.0));
            if !suggestions.is_empty() {
                let first = &suggestions[0];
                child.label(
                    RichText::new(format!("Unlock: {}", first.pet.name))
                        .color(style::TEXT_MUTED)
                        .italics()
                        .size(10.0),
                );
            }
        }
    }
}
