use eframe::egui::{self, Color32, RichText, CornerRadius, Stroke, StrokeKind, Ui, Vec2};
use itrtg_models::dungeon::PartyEquipment;
use itrtg_models::Dungeon;
use itrtg_planner::solver::{self, Assignment, CoverageKind, DungeonPlan, MatchQuality};

use crate::data::DataStore;
use crate::style;
use super::widgets;

// =============================================================================
// State
// =============================================================================

/// Per-dungeon selection state.
#[derive(Clone)]
struct DungeonEntry {
    dungeon: Dungeon,
    label: &'static str,
    enabled: bool,
    depth: u8,
}

#[derive(Default)]
pub struct DungeonState {
    /// Selection and depth for each dungeon. Initialized on first frame.
    entries: Vec<DungeonEntry>,
    /// Solved plans, keyed by dungeon. Regenerated on Solve.
    plans: Vec<DungeonPlan>,
    initialized: bool,
}

const DUNGEONS: &[(Dungeon, &str)] = &[
    (Dungeon::Scrapyard, "Scrapyard"),
    (Dungeon::WaterTemple, "Water Temple"),
    (Dungeon::Volcano, "Volcano"),
    (Dungeon::Mountain, "Mountain"),
    (Dungeon::Forest, "Forest"),
];

impl DungeonState {
    fn ensure_init(&mut self) {
        if self.initialized {
            return;
        }
        self.entries = DUNGEONS
            .iter()
            .map(|(d, label)| DungeonEntry {
                dungeon: *d,
                label,
                enabled: false,
                depth: 1,
            })
            .collect();
        self.initialized = true;
    }
}

// =============================================================================
// Rendering
// =============================================================================

pub fn show(ui: &mut Ui, state: &mut DungeonState, data: &DataStore) {
    state.ensure_init();

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

    // Dungeon selection panel: one row per dungeon with checkbox + depth buttons
    ui.label(
        RichText::new("Select Dungeons")
            .color(style::TEXT_BRIGHT)
            .size(14.0)
            .strong(),
    );
    ui.add_space(2.0);

    for entry in &mut state.entries {
        ui.horizontal(|ui| {
            ui.checkbox(&mut entry.enabled, "");
            ui.label(
                RichText::new(entry.label)
                    .color(if entry.enabled { style::TEXT_BRIGHT } else { style::TEXT_MUTED })
                    .size(13.0),
            );

            ui.add_space(8.0);

            // Depth buttons
            for depth in 1..=3u8 {
                let selected = entry.depth == depth;
                let text = RichText::new(format!("D{depth}")).color(
                    if !entry.enabled {
                        style::TEXT_MUTED
                    } else if selected {
                        style::ACCENT
                    } else {
                        style::TEXT_NORMAL
                    },
                );
                let btn = egui::Button::new(text).fill(if selected && entry.enabled {
                    Color32::from_rgb(0x2a, 0x20, 0x40)
                } else {
                    style::BG_SURFACE
                });
                if ui.add_enabled(entry.enabled, btn).clicked() {
                    entry.depth = depth;
                }
            }
        });
    }

    ui.add_space(4.0);

    // Action buttons
    ui.horizontal(|ui| {
        let any_enabled = state.entries.iter().any(|e| e.enabled);
        let can_solve = any_enabled && !data.merged.is_empty();

        if ui
            .add_enabled(
                can_solve,
                egui::Button::new(RichText::new("  Solve All  ").color(style::TEXT_BRIGHT))
                    .fill(Color32::from_rgb(0x30, 0x20, 0x50)),
            )
            .clicked()
        {
            solve_all(state, data);
        }

        if ui
            .add(egui::Button::new(RichText::new("Clear").color(style::TEXT_MUTED)))
            .clicked()
        {
            state.plans.clear();
        }

        // Summary of selection
        let selected_count = state.entries.iter().filter(|e| e.enabled).count();
        if selected_count > 0 {
            ui.label(
                RichText::new(format!("{selected_count} dungeon(s) selected"))
                    .color(style::TEXT_MUTED)
                    .size(12.0),
            );
        }
    });

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    // Show results
    if state.plans.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(
                RichText::new("Select dungeons and depths, then click Solve All.")
                    .color(style::TEXT_MUTED),
            );
        });
    } else {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, plan) in state.plans.iter().enumerate() {
                if i > 0 {
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);
                }
                show_plan(ui, plan, data);
            }
        });
    }
}

fn solve_all(state: &mut DungeonState, data: &DataStore) {
    let Some(recs) = &data.dungeon_recs else { return };

    state.plans.clear();

    for entry in &state.entries {
        if !entry.enabled {
            continue;
        }
        let Some(dungeon_data) = recs.dungeons.get(&entry.dungeon) else {
            continue;
        };

        let plan = solver::solve(entry.dungeon, entry.depth, dungeon_data, &data.merged);
        state.plans.push(plan);
    }
}

fn show_plan(ui: &mut Ui, plan: &DungeonPlan, data: &DataStore) {
    let dungeon_name = DUNGEONS
        .iter()
        .find(|(d, _)| *d == plan.dungeon)
        .map(|(_, n)| *n)
        .unwrap_or("Unknown");

    // Dungeon header
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{dungeon_name} D{}", plan.depth))
                .color(style::ACCENT)
                .size(16.0)
                .strong(),
        );
    });

    ui.label(
        RichText::new("Positions 1-3: Front Row  |  Positions 4-6: Back Row")
            .color(style::TEXT_MUTED)
            .size(11.0),
    );
    ui.add_space(4.0);

    let slot_width = ui.available_width().min(800.0) / 3.0 - 8.0;

    // Look up equipment catalog for this dungeon
    let equip_catalog = data.dungeon_recs.as_ref().map(|r| &r.equipment);

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
                show_slot_card(ui, &plan.assignments[i], slot_width, equip_catalog);
            }
        });
        ui.add_space(4.0);
    }

    // Warnings
    if !plan.warnings.is_empty() {
        ui.add_space(4.0);
        ui.label(
            RichText::new("Coverage Warnings")
                .color(style::WARNING)
                .size(13.0),
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

fn show_slot_card(
    ui: &mut Ui,
    slot: &solver::SlotAssignment,
    width: f32,
    equip_catalog: Option<&itrtg_models::dungeon::EquipmentCatalog>,
) {
    // Dynamically size: taller if we have equipment to show
    let has_equip = slot.slot.equipment.is_some();
    let height = if has_equip { 105.0 } else { 80.0 };
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
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::LEFT)),
    );

    // Slot header: position + requirements
    child.horizontal(|ui| {
        ui.label(
            RichText::new(format!("#{}", slot.position + 1))
                .color(style::TEXT_MUTED)
                .size(11.0),
        );
        if let Some(class) = &slot.slot.class {
            widgets::class_label(ui, class);
        } else {
            ui.label(RichText::new("Any").color(style::TEXT_MUTED).size(12.0));
        }
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

    // Equipment recommendations
    if let Some(equip) = &slot.slot.equipment {
        show_equipment_line(&mut child, equip, equip_catalog);
    }
}

/// Show a compact equipment recommendation line inside a slot card.
fn show_equipment_line(
    ui: &mut Ui,
    equip: &PartyEquipment,
    catalog: Option<&itrtg_models::dungeon::EquipmentCatalog>,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Gear:").color(style::TEXT_MUTED).size(10.0));

        let parts: Vec<String> = [
            equip.weapon.as_deref().map(|k| resolve_equip_name(k, catalog)),
            equip.armor.as_deref().map(|k| resolve_equip_name(k, catalog)),
            equip.accessory.as_deref().map(|k| resolve_equip_name(k, catalog)),
        ]
        .iter()
        .filter_map(|x| x.clone())
        .collect();

        if parts.is_empty() {
            ui.label(RichText::new("—").color(style::TEXT_MUTED).size(10.0));
        } else {
            ui.label(
                RichText::new(parts.join(" / "))
                    .color(style::TEXT_NORMAL)
                    .size(10.0),
            );
        }
    });

    // Gem recommendations
    if let Some(gems) = &equip.gems {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Gems:").color(style::TEXT_MUTED).size(10.0));
            let gem_parts: Vec<String> = [
                gems.weapon.as_ref().map(|e| format!("W:{e:?}")),
                gems.armor.as_ref().map(|e| format!("A:{e:?}")),
                gems.accessory.as_ref().map(|e| format!("Ac:{e:?}")),
            ]
            .iter()
            .filter_map(|x| x.clone())
            .collect();

            if !gem_parts.is_empty() {
                ui.label(
                    RichText::new(gem_parts.join(" "))
                        .color(style::TEXT_MUTED)
                        .size(10.0),
                );
            }
        });
    }
}

/// Resolve a catalog key to a display name.
fn resolve_equip_name(key: &str, catalog: Option<&itrtg_models::dungeon::EquipmentCatalog>) -> String {
    if let Some(cat) = catalog {
        if let Some(entry) = cat.lookup(key) {
            return entry.name.clone();
        }
    }
    // Fallback: humanize the key
    key.replace('_', " ")
}
