use std::collections::{HashMap, HashSet};

use eframe::egui::{self, Color32, RichText, CornerRadius, Stroke, StrokeKind, Ui, Vec2};
use itrtg_models::dungeon::EquipmentCatalog;
use itrtg_models::{Dungeon, Element};
use itrtg_planner::equipment::{self, EquipmentSource};
use itrtg_planner::solver::{
    self, Assignment, CoverageKind, DungeonPlan, DungeonRequest, MatchQuality, SolverConstraints,
};
use serde::Deserialize;

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
    /// Pet names forbidden from all dungeon teams.
    forbidden_pets: HashSet<String>,
    /// Pets forced into dungeon teams: (optional dungeon, pet_name).
    /// None dungeon = solver picks the best team.
    forced_pets: Vec<(Option<Dungeon>, String)>,
    /// Whitelisted pets: bypass non-dungeon class filter without being forced.
    whitelisted_pets: HashSet<String>,
    /// Search text for adding constraints.
    constraint_search: String,
    /// Selected dungeon for the "Force" action. None = Any team.
    force_dungeon: Option<Dungeon>,
    /// Data version when plans were last refreshed.
    last_data_version: u64,
}

const CONSTRAINTS_PATH: &str = "data/pet_constraints.yaml";

const DUNGEONS: &[(Dungeon, &str)] = &[
    (Dungeon::Scrapyard, "Scrapyard"),
    (Dungeon::WaterTemple, "Water Temple"),
    (Dungeon::Volcano, "Volcano"),
    (Dungeon::Mountain, "Mountain"),
    (Dungeon::Forest, "Forest"),
];

fn dungeon_label(d: Dungeon) -> &'static str {
    DUNGEONS
        .iter()
        .find(|(dd, _)| *dd == d)
        .map(|(_, l)| *l)
        .unwrap_or("Unknown")
}

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
        self.force_dungeon = None; // Default: solver picks best team
        self.initialized = true;
    }

    /// Build SolverConstraints from the current UI state.
    fn build_constraints(&self) -> SolverConstraints {
        let mut forced: HashMap<Dungeon, Vec<String>> = HashMap::new();
        let mut forced_any: Vec<String> = Vec::new();
        for (dungeon, name) in &self.forced_pets {
            // Skip if the pet is also forbidden (forbidden takes priority)
            if self.forbidden_pets.contains(name) {
                continue;
            }
            match dungeon {
                Some(d) => forced.entry(*d).or_default().push(name.clone()),
                None => forced_any.push(name.clone()),
            }
        }
        SolverConstraints {
            forbidden: self.forbidden_pets.clone(),
            forced,
            forced_any,
            whitelisted: self.whitelisted_pets.clone(),
        }
    }

    /// Load constraints from a YAML string (pet_constraints.yaml).
    /// Merges into current state — existing manual constraints are preserved.
    pub fn load_constraints_yaml(&mut self, yaml: &str) -> Result<(), String> {
        let file: ConstraintsFile =
            serde_yaml::from_str(yaml).map_err(|e| format!("Constraints YAML error: {e}"))?;

        for name in file.forbidden.unwrap_or_default() {
            self.forbidden_pets.insert(name);
        }
        for entry in file.forced.unwrap_or_default() {
            let already = self
                .forced_pets
                .iter()
                .any(|(d, n)| n == &entry.pet && *d == entry.dungeon);
            if !already {
                self.forced_pets.push((entry.dungeon, entry.pet));
            }
        }
        for name in file.whitelisted.unwrap_or_default() {
            self.whitelisted_pets.insert(name);
        }

        Ok(())
    }

    /// Refresh pet data in existing plans without re-solving.
    /// Updates stats (DL, CL, growth, equipment) for already-assigned pets.
    fn refresh_plans(&mut self, data: &DataStore) {
        if self.plans.is_empty() || self.last_data_version == data.data_version {
            return;
        }
        self.last_data_version = data.data_version;

        for plan in &mut self.plans {
            for sa in &mut plan.assignments {
                if let Assignment::Filled { pet, .. } = &mut sa.assignment {
                    // Find updated data for this pet
                    if let Some(updated) = data.merged.iter().find(|m| m.name == pet.name) {
                        pet.export = updated.export.clone();
                        pet.wiki = updated.wiki.clone();
                    }
                }
            }
            // Re-enrich equipment with updated pet data
            if let Some(recs) = &data.dungeon_recs {
                equipment::enrich_equipment(plan, &recs.equipment);
            }
        }
    }

    /// Clear all constraints and reload from the YAML file.
    pub fn reset_constraints_from_file(&mut self) -> Result<(), String> {
        self.forbidden_pets.clear();
        self.forced_pets.clear();
        self.whitelisted_pets.clear();

        let path = std::path::Path::new(CONSTRAINTS_PATH);
        if path.exists() {
            let yaml =
                std::fs::read_to_string(path).map_err(|e| format!("Read error: {e}"))?;
            self.load_constraints_yaml(&yaml)?;
        }
        Ok(())
    }

    /// Save current constraints to the YAML file with nice comments.
    pub fn save_constraints_to_file(&self) -> Result<(), String> {
        let yaml = self.serialize_constraints_yaml();
        std::fs::write(CONSTRAINTS_PATH, yaml).map_err(|e| format!("Write error: {e}"))
    }

    /// Serialize current constraints to a YAML string with explanatory comments.
    fn serialize_constraints_yaml(&self) -> String {
        let mut out = String::new();
        out.push_str("# Pet Constraints for the Dungeon Planner\n");
        out.push_str("#\n");
        out.push_str("# Forbidden: pets excluded from ALL dungeon teams.\n");
        out.push_str("# Use this for pets you want to keep on campaigns, village jobs,\n");
        out.push_str("# or otherwise don't want the solver touching.\n");
        out.push_str("#\n");
        out.push_str("# Forced: pets that MUST appear in a dungeon team.\n");
        out.push_str("# Optionally specify a dungeon to pin them to a specific team.\n");
        out.push_str("# Omit the dungeon to let the solver pick the best fit.\n");
        out.push_str("#\n");
        out.push_str("# Whitelisted: pets that bypass the non-dungeon class filter.\n");
        out.push_str("# Use for Blacksmiths/Alchemists/Adventurers that actually benefit\n");
        out.push_str("# from dungeon runs (e.g. special abilities, class XP exceptions).\n");
        out.push_str("#\n");
        out.push_str("# Dungeon names: Scrapyard, WaterTemple, Volcano, Mountain, Forest\n");
        out.push_str("\n");

        // Forbidden
        out.push_str("forbidden:\n");
        if self.forbidden_pets.is_empty() {
            out.push_str("  # - Pet Name\n");
        } else {
            let mut sorted: Vec<&String> = self.forbidden_pets.iter().collect();
            sorted.sort();
            for name in sorted {
                out.push_str(&format!("  - {name}\n"));
            }
        }
        out.push_str("\n");

        // Forced
        out.push_str("forced:\n");
        if self.forced_pets.is_empty() {
            out.push_str("  # - pet: Pet Name\n");
            out.push_str("  #   dungeon: Forest\n");
        } else {
            for (dungeon, name) in &self.forced_pets {
                out.push_str(&format!("  - pet: {name}\n"));
                if let Some(d) = dungeon {
                    out.push_str(&format!("    dungeon: {d:?}\n"));
                }
            }
        }
        out.push_str("\n");

        // Whitelisted
        out.push_str("whitelisted:\n");
        if self.whitelisted_pets.is_empty() {
            out.push_str("  # - Pet Name\n");
        } else {
            let mut sorted: Vec<&String> = self.whitelisted_pets.iter().collect();
            sorted.sort();
            for name in sorted {
                out.push_str(&format!("  - {name}\n"));
            }
        }

        out
    }
}

// =============================================================================
// Constraints YAML format
// =============================================================================

#[derive(Deserialize)]
struct ConstraintsFile {
    forbidden: Option<Vec<String>>,
    forced: Option<Vec<ForcedEntry>>,
    whitelisted: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct ForcedEntry {
    pet: String,
    dungeon: Option<Dungeon>,
}

// =============================================================================
// Rendering
// =============================================================================

pub fn show(ui: &mut Ui, state: &mut DungeonState, data: &DataStore) {
    state.ensure_init();

    // Auto-refresh plans when pet data changes (import/wiki refresh)
    state.refresh_plans(data);

    if data.dungeon_recs.is_none() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(
                RichText::new("No dungeon recommendations loaded.")
                    .color(style::WARNING)
                    .size(16.0),
            );
            ui.label(
                RichText::new("Place dungeon_recommendations.yaml in the data directory.")
                    .color(style::TEXT_MUTED),
            );
        });
        return;
    }

    // Dungeon selection (collapsible)
    let selected_count = state.entries.iter().filter(|e| e.enabled).count();
    let sel_header = if selected_count > 0 {
        format!("Select Dungeons ({selected_count} selected)")
    } else {
        "Select Dungeons".to_string()
    };

    egui::CollapsingHeader::new(
        RichText::new(sel_header)
            .color(style::TEXT_BRIGHT)
            .size(14.0)
            .strong(),
    )
    .default_open(state.plans.is_empty())
    .show(ui, |ui| {
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
    });

    // Action buttons (always visible)
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
    });

    ui.add_space(4.0);

    // Pet constraints (collapsible)
    show_constraints(ui, state, data);

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

            // Shopping list: aggregate missing items across all plans
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            show_shopping_list(ui, &state.plans, data);
        });
    }
}

// =============================================================================
// Constraints UI
// =============================================================================

fn show_constraints(ui: &mut Ui, state: &mut DungeonState, data: &DataStore) {
    let total_constraints = state.forbidden_pets.len()
        + state.forced_pets.len()
        + state.whitelisted_pets.len();
    let header_text = if total_constraints > 0 {
        format!("Pet Constraints ({total_constraints} active)")
    } else {
        "Pet Constraints".to_string()
    };

    egui::CollapsingHeader::new(
        RichText::new(header_text)
            .color(style::TEXT_MUTED)
            .size(13.0),
    )
    .default_open(false)
    .show(ui, |ui| {
        // Search + add controls
        ui.horizontal(|ui| {
            ui.label(RichText::new("Pet:").color(style::TEXT_MUTED).size(12.0));

            // Pet selector: ComboBox listing unlocked pets filtered by search
            let search = &state.constraint_search;
            egui::ComboBox::from_id_salt("constraint_pet")
                .selected_text(if search.is_empty() {
                    "Select pet..."
                } else {
                    search.as_str()
                })
                .width(160.0)
                .show_ui(ui, |ui| {
                    let search_lower = state.constraint_search.to_lowercase();
                    let mut names: Vec<&str> = data.merged.iter()
                        .filter(|p| p.is_unlocked())
                        .filter(|p| !state.forbidden_pets.contains(&p.name))
                        .filter(|p| !state.forced_pets.iter().any(|(_, n)| n == &p.name))
                        .filter(|p| !state.whitelisted_pets.contains(&p.name))
                        .filter(|p| search_lower.is_empty() || p.name.to_lowercase().contains(&search_lower))
                        .map(|p| p.name.as_str())
                        .collect();
                    names.sort_unstable();
                    for name in names {
                        if ui.selectable_label(false, name).clicked() {
                            state.constraint_search = name.to_string();
                        }
                    }
                });

            ui.add_space(4.0);

            let pet_valid = data.merged.iter().any(|p| p.name == state.constraint_search && p.is_unlocked());

            // Forbid button
            if ui
                .add_enabled(
                    pet_valid,
                    egui::Button::new(RichText::new("Forbid").color(style::ERROR).size(12.0)),
                )
                .clicked()
            {
                state.forbidden_pets.insert(state.constraint_search.clone());
                state.constraint_search.clear();
            }

            // Whitelist button
            if ui
                .add_enabled(
                    pet_valid,
                    egui::Button::new(
                        RichText::new("Whitelist")
                            .color(Color32::from_rgb(0x88, 0xcc, 0xff))
                            .size(12.0),
                    ),
                )
                .on_hover_text("Allow this pet in dungeon teams even if it has a non-dungeon class")
                .clicked()
            {
                state.whitelisted_pets.insert(state.constraint_search.clone());
                state.constraint_search.clear();
            }

            // Force button
            if ui
                .add_enabled(
                    pet_valid,
                    egui::Button::new(RichText::new("Force").color(style::SUCCESS).size(12.0)),
                )
                .clicked()
            {
                state
                    .forced_pets
                    .push((state.force_dungeon, state.constraint_search.clone()));
                state.constraint_search.clear();
            }

            // Optional dungeon selector for Force (defaults to Any)
            let force_label = match state.force_dungeon {
                None => "Any",
                Some(d) => dungeon_label(d),
            };
            ui.label(RichText::new("→").color(style::TEXT_MUTED).size(11.0));
            egui::ComboBox::from_id_salt("constraint_dungeon")
                .selected_text(force_label)
                .width(110.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.force_dungeon, None, "Any");
                    for (d, label) in DUNGEONS {
                        ui.selectable_value(&mut state.force_dungeon, Some(*d), *label);
                    }
                });
        });

        // File management buttons
        ui.horizontal(|ui| {
            if ui
                .add(egui::Button::new(
                    RichText::new("Open File").color(style::TEXT_MUTED).size(11.0),
                ))
                .on_hover_text("Open pet_constraints.yaml in your default editor")
                .clicked()
            {
                open_constraints_file();
            }

            if ui
                .add(egui::Button::new(
                    RichText::new("Save to File").color(style::TEXT_MUTED).size(11.0),
                ))
                .on_hover_text("Save current constraints to data/pet_constraints.yaml")
                .clicked()
            {
                if let Err(e) = state.save_constraints_to_file() {
                    eprintln!("Failed to save constraints: {e}");
                }
            }

            if ui
                .add(egui::Button::new(
                    RichText::new("Reset from File").color(style::TEXT_MUTED).size(11.0),
                ))
                .on_hover_text("Clear all constraints and reload from data/pet_constraints.yaml")
                .clicked()
            {
                let _ = state.reset_constraints_from_file();
            }
        });

        // Show active constraints
        let has_any = !state.forbidden_pets.is_empty()
            || !state.forced_pets.is_empty()
            || !state.whitelisted_pets.is_empty();

        if has_any {
            ui.add_space(4.0);

            // Forbidden pets
            if !state.forbidden_pets.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Forbidden:").color(style::ERROR).size(11.0));
                    let mut to_remove = Vec::new();
                    let mut sorted: Vec<&String> = state.forbidden_pets.iter().collect();
                    sorted.sort();
                    for name in sorted {
                        let btn = egui::Button::new(
                            RichText::new(format!("{name} ×")).color(style::ERROR).size(11.0),
                        )
                        .fill(Color32::from_rgb(0x30, 0x15, 0x15));
                        if ui.add(btn).clicked() {
                            to_remove.push(name.clone());
                        }
                    }
                    for name in to_remove {
                        state.forbidden_pets.remove(&name);
                    }
                });
            }

            // Whitelisted pets
            if !state.whitelisted_pets.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Whitelisted:")
                            .color(Color32::from_rgb(0x88, 0xcc, 0xff))
                            .size(11.0),
                    );
                    let mut to_remove = Vec::new();
                    let mut sorted: Vec<&String> = state.whitelisted_pets.iter().collect();
                    sorted.sort();
                    for name in sorted {
                        let btn = egui::Button::new(
                            RichText::new(format!("{name} ×"))
                                .color(Color32::from_rgb(0x88, 0xcc, 0xff))
                                .size(11.0),
                        )
                        .fill(Color32::from_rgb(0x15, 0x20, 0x30));
                        if ui.add(btn).clicked() {
                            to_remove.push(name.clone());
                        }
                    }
                    for name in to_remove {
                        state.whitelisted_pets.remove(&name);
                    }
                });
            }

            // Forced pets
            if !state.forced_pets.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Forced:").color(style::SUCCESS).size(11.0));
                    let mut to_remove = Vec::new();
                    for (i, (dungeon, name)) in state.forced_pets.iter().enumerate() {
                        let target = match dungeon {
                            Some(d) => dungeon_label(*d),
                            None => "Any",
                        };
                        let btn = egui::Button::new(
                            RichText::new(format!("{name} → {target} ×"))
                                .color(style::SUCCESS)
                                .size(11.0),
                        )
                        .fill(Color32::from_rgb(0x15, 0x30, 0x15));
                        if ui.add(btn).clicked() {
                            to_remove.push(i);
                        }
                    }
                    for i in to_remove.into_iter().rev() {
                        state.forced_pets.remove(i);
                    }
                });
            }
        }
    });
}

/// Open the constraints YAML file in the OS default editor.
fn open_constraints_file() {
    let path = CONSTRAINTS_PATH;
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", path])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}

// =============================================================================
// Solver
// =============================================================================

fn solve_all(state: &mut DungeonState, data: &DataStore) {
    let Some(recs) = &data.dungeon_recs else { return };

    // Build requests for all enabled dungeons
    let requests: Vec<DungeonRequest> = state
        .entries
        .iter()
        .filter(|e| e.enabled)
        .filter_map(|entry| {
            recs.dungeons.get(&entry.dungeon).map(|dd| DungeonRequest {
                dungeon: entry.dungeon,
                depth: entry.depth,
                data: dd,
            })
        })
        .collect();

    if requests.is_empty() {
        state.plans.clear();
        return;
    }

    // Build constraints from UI state
    let constraints = state.build_constraints();

    // Solve all dungeons simultaneously — no pet reuse across teams
    state.plans = solver::solve_multi(&requests, &data.merged, &constraints);

    // Enrich with equipment suggestions (resolves generic keys to real gear)
    for plan in &mut state.plans {
        equipment::enrich_equipment(plan, &recs.equipment);
    }

    // Mark plans as current
    state.last_data_version = data.data_version;
}

// =============================================================================
// Plan display
// =============================================================================

fn show_plan(ui: &mut Ui, plan: &DungeonPlan, data: &DataStore) {
    let dungeon_name = dungeon_label(plan.dungeon);

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

    // Team stats & difficulty recommendations
    show_team_stats(ui, plan, data);

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

fn show_team_stats(ui: &mut Ui, plan: &DungeonPlan, data: &DataStore) {
    // Collect stats from assigned pets
    let assigned_exports: Vec<_> = plan
        .assignments
        .iter()
        .filter_map(|sa| match &sa.assignment {
            Assignment::Filled { pet, .. } => pet.export.as_ref(),
            _ => None,
        })
        .collect();

    if assigned_exports.is_empty() {
        return;
    }

    let pet_count = assigned_exports.len();
    let avg_dungeon_level = assigned_exports.iter().map(|e| e.dungeon_level as u64).sum::<u64>() / pet_count as u64;
    let min_class_level = assigned_exports.iter().map(|e| e.class_level).min().unwrap_or(0);
    let min_growth = assigned_exports.iter().map(|e| e.growth).min().unwrap_or(0);

    // Team stats summary
    ui.horizontal(|ui| {
        ui.label(RichText::new("Team:").color(style::TEXT_MUTED).size(12.0).strong());
        ui.label(
            RichText::new(format!("Avg DL {avg_dungeon_level}"))
                .color(style::TEXT_NORMAL)
                .size(12.0),
        );
        ui.label(RichText::new("|").color(style::TEXT_MUTED).size(12.0));
        ui.label(
            RichText::new(format!("Min CL {min_class_level}"))
                .color(style::TEXT_NORMAL)
                .size(12.0),
        );
        ui.label(RichText::new("|").color(style::TEXT_MUTED).size(12.0));
        ui.label(
            RichText::new(format!("{pet_count}/6 filled"))
                .color(if pet_count == 6 { style::TEXT_NORMAL } else { style::WARNING })
                .size(12.0),
        );
    });

    // Difficulty recommendations per sub-depth
    let Some(recs) = &data.dungeon_recs else { return };
    let Some(dungeon_data) = recs.dungeons.get(&plan.dungeon) else { return };

    ui.horizontal(|ui| {
        ui.label(RichText::new("Difficulty:").color(style::TEXT_MUTED).size(12.0).strong());

        for depth in 1..=plan.depth {
            let Some(dd) = dungeon_data.depths.get(&depth) else { continue };
            let reqs = &dd.requirements;

            // Dungeon level difficulty: how many difficulty levels can we afford?
            // Use the higher (conservative) levels_per_difficulty value
            let per_diff = reqs.levels_per_difficulty.last().copied().unwrap_or(5) as u64;
            let dl_diff = if avg_dungeon_level > reqs.dungeon_level_avg as u64 {
                ((avg_dungeon_level - reqs.dungeon_level_avg as u64) / per_diff).min(10)
            } else {
                0
            };

            // Class level check: binary pass/fail for the depth
            let cl_ok = min_class_level >= reqs.class_level;

            // Per-pet growth check: "total growth" is per-pet, not team sum
            let growth_ok = reqs.total_growth.map_or(true, |req| min_growth >= req);

            let max_diff = dl_diff.min(10);

            let diff_color = if max_diff >= 8 {
                style::SUCCESS
            } else if max_diff >= 4 {
                style::WARNING
            } else if max_diff >= 1 {
                Color32::from_rgb(0xdd, 0x88, 0x44)
            } else {
                style::ERROR
            };

            if depth > 1 {
                ui.label(RichText::new("|").color(style::TEXT_MUTED).size(12.0));
            }
            ui.label(
                RichText::new(format!("D{depth} → {max_diff}"))
                    .color(diff_color)
                    .size(12.0),
            );

            // Show warnings for unmet requirements
            if !cl_ok {
                ui.label(
                    RichText::new(format!("(need CL {})", reqs.class_level))
                        .color(style::ERROR)
                        .size(10.0),
                );
            }
            if !growth_ok {
                ui.label(
                    RichText::new("(growth)")
                        .color(style::ERROR)
                        .size(10.0),
                );
            }
        }
    });

    ui.add_space(2.0);
}

// =============================================================================
// Shopping list
// =============================================================================

/// Aggregate missing items across all solved dungeon plans into a single checklist.
fn show_shopping_list(ui: &mut Ui, plans: &[DungeonPlan], data: &DataStore) {
    let catalog = data.dungeon_recs.as_ref().map(|r| &r.equipment);

    // Collect missing pets, equipment differences, and gem needs
    let mut pets_to_unlock: Vec<String> = Vec::new();
    let mut pets_to_evolve: Vec<(String, String)> = Vec::new(); // (pet, target class)
    let mut equip_needed: Vec<String> = Vec::new();
    let mut gems_needed: std::collections::BTreeMap<Element, u32> = std::collections::BTreeMap::new();

    for plan in plans {
        let dng_name = dungeon_label(plan.dungeon);

        for sa in &plan.assignments {
            match &sa.assignment {
                Assignment::Empty { suggestions } => {
                    // Missing pet
                    let suggest_str = suggestions
                        .first()
                        .map(|s| format!(" (unlock {})", s.pet.name))
                        .unwrap_or_default();
                    let class_str = sa.slot.class.map(|c| format!("{c:?}")).unwrap_or("Any".into());
                    pets_to_unlock.push(format!(
                        "{dng_name} #{}: {class_str}{suggest_str}",
                        sa.position + 1
                    ));
                }
                Assignment::Filled { pet, quality } => {
                    // Track pets that need evolving
                    if *quality == MatchQuality::Evolvable {
                        let target = sa.slot.class.map(|c| format!("{c:?}")).unwrap_or("?".into());
                        pets_to_evolve.push((pet.name.clone(), target));
                    }

                    // Compare equipment
                    if let Some(suggestion) = &sa.equipment_suggestion {
                        let current = pet.export.as_ref().map(|e| &e.loadout);
                        collect_equip_diffs(
                            &suggestion.equipment,
                            current,
                            catalog,
                            &mut equip_needed,
                        );

                        // Collect gem needs
                        if let Some(gem_slots) = &suggestion.equipment.gems {
                            let current_weapon_gem = current.and_then(|l| l.weapon.as_ref()).and_then(|e| e.gem);
                            let current_armor_gem = current.and_then(|l| l.armor.as_ref()).and_then(|e| e.gem);
                            let current_acc_gem = current.and_then(|l| l.accessory.as_ref()).and_then(|e| e.gem);

                            for (rec, cur) in [
                                (&gem_slots.weapon, current_weapon_gem),
                                (&gem_slots.armor, current_armor_gem),
                                (&gem_slots.accessory, current_acc_gem),
                            ] {
                                if let Some(needed) = rec {
                                    if cur != Some(*needed) {
                                        *gems_needed.entry(*needed).or_insert(0) += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Deduplicate evolve list
    pets_to_evolve.sort();
    pets_to_evolve.dedup();
    // Deduplicate equipment
    equip_needed.sort();
    equip_needed.dedup();

    let has_items = !pets_to_unlock.is_empty()
        || !pets_to_evolve.is_empty()
        || !equip_needed.is_empty()
        || !gems_needed.is_empty();

    if !has_items {
        ui.label(
            RichText::new("✓ All teams fully equipped!")
                .color(style::SUCCESS)
                .size(13.0),
        );
        return;
    }

    ui.label(
        RichText::new("Shopping List")
            .color(style::ACCENT)
            .size(15.0)
            .strong(),
    );
    ui.add_space(4.0);

    // Pets to unlock
    if !pets_to_unlock.is_empty() {
        ui.label(
            RichText::new(format!("Pets to Unlock ({})", pets_to_unlock.len()))
                .color(style::ERROR)
                .size(12.0)
                .strong(),
        );
        for item in &pets_to_unlock {
            ui.label(
                RichText::new(format!("  • {item}"))
                    .color(style::TEXT_NORMAL)
                    .size(11.0),
            );
        }
        ui.add_space(4.0);
    }

    // Pets to evolve
    if !pets_to_evolve.is_empty() {
        ui.label(
            RichText::new(format!("Pets to Evolve ({})", pets_to_evolve.len()))
                .color(style::WARNING)
                .size(12.0)
                .strong(),
        );
        for (name, class) in &pets_to_evolve {
            ui.label(
                RichText::new(format!("  • {name} → {class}"))
                    .color(style::TEXT_NORMAL)
                    .size(11.0),
            );
        }
        ui.add_space(4.0);
    }

    // Equipment to forge/obtain
    if !equip_needed.is_empty() {
        ui.label(
            RichText::new(format!("Equipment Needed ({})", equip_needed.len()))
                .color(Color32::from_rgb(0x88, 0x99, 0xcc))
                .size(12.0)
                .strong(),
        );
        for item in &equip_needed {
            ui.label(
                RichText::new(format!("  • {item}"))
                    .color(style::TEXT_NORMAL)
                    .size(11.0),
            );
        }
        ui.add_space(4.0);
    }

    // Gems needed
    if !gems_needed.is_empty() {
        let total_gems: u32 = gems_needed.values().sum();
        ui.label(
            RichText::new(format!("Gems Needed ({total_gems})"))
                .color(Color32::from_rgb(0xcc, 0x99, 0xff))
                .size(12.0)
                .strong(),
        );
        for (element, count) in &gems_needed {
            ui.label(
                RichText::new(format!("  • {count}x {element:?}"))
                    .color(style::TEXT_NORMAL)
                    .size(11.0),
            );
        }
    }
}

/// Collect equipment differences between recommendation and current loadout.
fn collect_equip_diffs(
    rec: &itrtg_models::dungeon::PartyEquipment,
    current: Option<&itrtg_models::Loadout>,
    catalog: Option<&EquipmentCatalog>,
    out: &mut Vec<String>,
) {
    let slots: [(&str, Option<&str>, Option<&itrtg_models::Equipment>); 3] = [
        ("Weapon", rec.weapon.as_deref(), current.and_then(|l| l.weapon.as_ref())),
        ("Armor", rec.armor.as_deref(), current.and_then(|l| l.armor.as_ref())),
        ("Accessory", rec.accessory.as_deref(), current.and_then(|l| l.accessory.as_ref())),
    ];

    for (slot_name, rec_key, cur_equip) in &slots {
        let Some(key) = rec_key else { continue };
        let rec_name = resolve_equip_name(key, catalog);

        let matches = cur_equip.as_ref().is_some_and(|cur| {
            let cur_lower = cur.name.to_lowercase();
            let rec_lower = rec_name.to_lowercase();
            cur_lower.contains(&rec_lower) || rec_lower.contains(&cur_lower)
        });

        if !matches {
            let current_str = cur_equip
                .map(|c| format!(" (have: {})", c.name))
                .unwrap_or_default();
            out.push(format!("{slot_name}: {rec_name}{current_str}"));
        }
    }
}

fn show_slot_card(
    ui: &mut Ui,
    slot: &solver::SlotAssignment,
    width: f32,
    equip_catalog: Option<&EquipmentCatalog>,
) {
    // Dynamically size based on content
    let has_equip = slot.equipment_suggestion.is_some();
    let is_filled = matches!(&slot.assignment, Assignment::Filled { .. });
    let height = match (has_equip, is_filled) {
        (true, true) => 140.0,  // Full card: header + pet + stats + 3 equip lines
        (false, true) => 80.0,  // No equipment
        _ => 65.0,              // Empty slot
    };
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

            // Pet name + element
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

            // Class + match quality + DL + CL
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
                // DL, CL, Growth
                if let Some(export) = &pet.export {
                    ui.label(
                        RichText::new(format!("DL:{} CL:{} G:{}", export.dungeon_level, export.class_level, format_compact_number(export.growth)))
                            .color(style::TEXT_MUTED)
                            .size(10.0)
                            .family(egui::FontFamily::Monospace),
                    );
                }
            });

            // Equipment: recommended vs current
            if let Some(suggestion) = &slot.equipment_suggestion {
                let current_loadout = pet.export.as_ref().map(|e| &e.loadout);
                show_equipment_comparison(&mut child, suggestion, current_loadout, equip_catalog);
            }
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

/// Show equipment with comparison against pet's current gear.
/// Each line shows the recommendation and a status indicator:
/// ✓ if current gear matches, or the current gear name if different.
fn show_equipment_comparison(
    ui: &mut Ui,
    suggestion: &equipment::EquipmentSuggestion,
    current_loadout: Option<&itrtg_models::Loadout>,
    catalog: Option<&EquipmentCatalog>,
) {
    let equip = &suggestion.equipment;
    let gems = equip.gems.as_ref();
    let is_computed = suggestion.source == EquipmentSource::Computed;

    let rec_color = if is_computed {
        Color32::from_rgb(0x88, 0x99, 0xcc)
    } else {
        style::TEXT_NORMAL
    };

    // (prefix, recommended catalog key, gem, current equipment)
    let lines: [(&str, Option<&str>, Option<&Element>, Option<&itrtg_models::Equipment>); 3] = [
        ("W:", equip.weapon.as_deref(), gems.and_then(|g| g.weapon.as_ref()),
         current_loadout.and_then(|l| l.weapon.as_ref())),
        ("A:", equip.armor.as_deref(), gems.and_then(|g| g.armor.as_ref()),
         current_loadout.and_then(|l| l.armor.as_ref())),
        ("Ac:", equip.accessory.as_deref(), gems.and_then(|g| g.accessory.as_ref()),
         current_loadout.and_then(|l| l.accessory.as_ref())),
    ];

    for (prefix, rec_key, gem, current) in &lines {
        if let Some(key) = rec_key {
            let rec_name = resolve_equip_name(key, catalog);
            let gem_str = match gem {
                Some(el) => format!(" [{el:?}]"),
                None => String::new(),
            };

            // Check if current gear matches the recommendation
            let status = match current {
                Some(cur) => {
                    let cur_lower = cur.name.to_lowercase();
                    let rec_lower = rec_name.to_lowercase();
                    if cur_lower.contains(&rec_lower) || rec_lower.contains(&cur_lower) {
                        // Match — show quality/upgrade info
                        let grade = match cur.upgrade_level {
                            Some(lv) => format!("{:?}+{lv}", cur.quality),
                            None => format!("{:?}", cur.quality),
                        };
                        EquipStatus::Match(grade)
                    } else {
                        // Different equipment
                        EquipStatus::Diff(cur.name.clone())
                    }
                }
                None => EquipStatus::None,
            };

            ui.horizontal(|ui| {
                // Recommendation
                let mut rec_text = RichText::new(format!("{prefix} {rec_name}{gem_str}"))
                    .color(rec_color)
                    .size(10.0);
                if is_computed {
                    rec_text = rec_text.italics();
                }
                ui.label(rec_text);

                // Current status indicator
                match &status {
                    EquipStatus::Match(grade) => {
                        ui.label(
                            RichText::new(format!("✓ {grade}"))
                                .color(style::SUCCESS)
                                .size(9.0),
                        );
                    }
                    EquipStatus::Diff(name) => {
                        ui.label(
                            RichText::new(format!("✗ {name}"))
                                .color(Color32::from_rgb(0xdd, 0x88, 0x44))
                                .size(9.0),
                        );
                    }
                    EquipStatus::None => {
                        ui.label(
                            RichText::new("—")
                                .color(style::TEXT_MUTED)
                                .size(9.0),
                        );
                    }
                }
            });
        }
    }
}

enum EquipStatus {
    /// Current gear matches recommendation — show quality+upgrade.
    Match(String),
    /// Current gear differs — show what's equipped.
    Diff(String),
    /// No equipment in this slot.
    None,
}

/// Resolve a catalog key to a display name.
fn resolve_equip_name(key: &str, catalog: Option<&EquipmentCatalog>) -> String {
    if let Some(cat) = catalog {
        if let Some(entry) = cat.lookup(key) {
            return entry.name.clone();
        }
    }
    // Humanize generic keys: "generic_t2_s10" → "Generic T2"
    if key.starts_with("generic_t") {
        let rest = &key["generic_t".len()..];
        let tier: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !tier.is_empty() {
            return format!("Generic T{tier}");
        }
    }
    // Fallback: humanize the key
    key.replace('_', " ")
}

/// Format a number compactly for card display (e.g. 1500 → "1.5k", 2300000 → "2.3M").
fn format_compact_number(n: u64) -> String {
    if n >= 1_000_000 {
        let m = n as f64 / 1_000_000.0;
        format!("{m:.1}M")
    } else if n >= 1_000 {
        let k = n as f64 / 1_000.0;
        format!("{k:.1}k")
    } else {
        n.to_string()
    }
}
