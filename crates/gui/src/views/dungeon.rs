use std::collections::{BTreeMap, HashMap, HashSet};

use eframe::egui::{self, Color32, RichText, CornerRadius, Stroke, StrokeKind, Ui, Vec2};
use itrtg_models::dungeon::{DungeonRecommendations, EquipmentCatalog};
use itrtg_models::Quality;
use itrtg_models::{Dungeon, Element};
use itrtg_planner::equipment::{self, EquipmentSource};
use itrtg_planner::solver::{
    self, Assignment, CoverageKind, DungeonPlan, DungeonRequest, MatchQuality, SolverConstraints,
};

use crate::data::DataStore;
use crate::state::{
    AppState, ConstraintsState, DungeonSelection, EquipmentStandardOverride as StateStandardOverride,
    ForcedEntry,
};
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
    /// Whether the requirements/party preview is expanded.
    show_preview: bool,
}

#[derive(Default)]
pub struct DungeonState {
    /// Selection and depth for each dungeon. Initialized on first frame.
    entries: Vec<DungeonEntry>,
    /// Solved plans, keyed by dungeon. Regenerated on Solve.
    plans: Vec<DungeonPlan>,
    initialized: bool,
    /// When false the solver ignores all constraints, producing fresh
    /// unconstrained recommendations. The constraints themselves are still
    /// visible in the UI so the user can re-enable without re-entering.
    constraints_enabled: bool,
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
    /// Equipment inventory: catalog_key → owned quantity.
    /// Only tracks limited/premium equipment. Craftable equipment is unlimited.
    pub equipment_inventory: HashMap<String, u8>,
    /// Per-dungeon equipment standard overrides from app state.
    equipment_standard_overrides: HashMap<Dungeon, EquipmentStandard>,
    // -- Constraints import/export UI state --
    /// Whether the constraints import dialog is open.
    show_constraints_import: bool,
    /// Text buffer for the constraints import dialog.
    constraints_import_text: String,
    /// Status message from the last import/export attempt.
    constraints_status: Option<(String, bool)>, // (message, is_error)
}

/// Resolved minimum equipment standards for a dungeon.
#[derive(Debug, Clone, Copy)]
pub struct EquipmentStandard {
    pub min_tier: u8,
    pub min_quality: Quality,
    pub min_upgrade: u8,
}

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
                show_preview: false,
            })
            .collect();
        self.force_dungeon = None; // Default: solver picks best team
        self.constraints_enabled = true; // Default: constraints active
        self.initialized = true;
    }

    /// Resolve equipment standards for a dungeon at a given depth.
    /// Returns depth-based defaults (tier=depth, S+10) merged with any overrides.
    fn standards_for(&self, dungeon: Dungeon, depth: u8) -> EquipmentStandard {
        let default = EquipmentStandard {
            min_tier: depth.clamp(1, 3),
            min_quality: Quality::S,
            min_upgrade: 10,
        };
        match self.equipment_standard_overrides.get(&dungeon) {
            Some(ovr) => EquipmentStandard {
                min_tier: ovr.min_tier.max(default.min_tier),
                min_quality: ovr.min_quality.max(default.min_quality),
                min_upgrade: ovr.min_upgrade.max(default.min_upgrade),
            },
            None => default,
        }
    }

    /// Build SolverConstraints from the current UI state.
    ///
    /// When `constraints_enabled` is false, returns empty constraints so
    /// the solver produces a fresh unconstrained recommendation.
    fn build_constraints(&self) -> SolverConstraints {
        if !self.constraints_enabled {
            return SolverConstraints::default();
        }

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

    /// Absorb the persisted `AppState` into this in-memory dungeon state.
    /// Replaces constraint sets, inventory, standards, and default-dungeon
    /// selections — the file is the sole source of truth.
    pub fn apply_app_state(&mut self, state: &AppState) {
        self.ensure_init();

        // Reset all dungeon selections before reapplying.
        for entry in &mut self.entries {
            entry.enabled = false;
            entry.depth = 1;
        }
        for selection in &state.default_dungeons {
            if let Some(entry) = self
                .entries
                .iter_mut()
                .find(|e| e.dungeon == selection.dungeon)
            {
                entry.enabled = true;
                entry.depth = selection.depth.clamp(1, 3);
            }
        }

        // Inventory is copied wholesale — order is irrelevant to the solver.
        self.equipment_inventory = state.inventory.iter().map(|(k, v)| (k.clone(), *v)).collect();

        // Standards: convert from the persistable override form (all Option
        // fields) to the in-memory form (concrete defaults).
        self.equipment_standard_overrides.clear();
        for (dungeon, ovr) in &state.equipment_standards {
            self.equipment_standard_overrides.insert(
                *dungeon,
                EquipmentStandard {
                    min_tier: ovr.min_tier.unwrap_or(1),
                    min_quality: ovr.min_quality.unwrap_or(Quality::S),
                    min_upgrade: ovr.min_upgrade.unwrap_or(10),
                },
            );
        }

        // Constraints: replace all three sets + toggle state.
        self.constraints_enabled = state.constraints.enabled;
        self.forbidden_pets.clear();
        self.forbidden_pets.extend(state.constraints.forbidden.iter().cloned());
        self.whitelisted_pets.clear();
        self.whitelisted_pets.extend(state.constraints.whitelisted.iter().cloned());
        self.forced_pets.clear();
        self.forced_pets.extend(
            state
                .constraints
                .forced
                .iter()
                .map(|f| (f.dungeon, f.pet.clone())),
        );
    }

    /// Fill an `AppState` with the persistable bits of the current dungeon state.
    /// Output ordering is deterministic so frame-to-frame diff detection is stable.
    pub fn write_into(&self, state: &mut AppState) {
        // Default dungeons: iterate in the canonical `DUNGEONS` order so that
        // toggling selections doesn't reorder the list in the saved file.
        state.default_dungeons = self
            .entries
            .iter()
            .filter(|e| e.enabled)
            .map(|e| DungeonSelection { dungeon: e.dungeon, depth: e.depth })
            .collect();

        // Inventory → BTreeMap for sorted iteration on serialize.
        state.inventory = self
            .equipment_inventory
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect::<BTreeMap<_, _>>();

        // Standards → BTreeMap keyed by Dungeon. Only emit fields that deviate
        // from the depth-based defaults; the in-memory form keeps concrete values.
        state.equipment_standards = self
            .equipment_standard_overrides
            .iter()
            .map(|(d, s)| {
                (
                    *d,
                    StateStandardOverride {
                        min_tier: Some(s.min_tier),
                        min_quality: Some(s.min_quality),
                        min_upgrade: Some(s.min_upgrade),
                    },
                )
            })
            .collect();

        // Constraints: sort forbidden/whitelisted alphabetically for stable YAML.
        let mut forbidden: Vec<String> = self.forbidden_pets.iter().cloned().collect();
        forbidden.sort();
        let mut whitelisted: Vec<String> = self.whitelisted_pets.iter().cloned().collect();
        whitelisted.sort();

        state.constraints = ConstraintsState {
            enabled: self.constraints_enabled,
            forbidden,
            whitelisted,
            forced: self
                .forced_pets
                .iter()
                .map(|(dungeon, pet)| ForcedEntry { pet: pet.clone(), dungeon: *dungeon })
                .collect(),
        };
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
            // Re-enrich equipment with updated pet data. The config is
            // passed through as `Option` so that static (non-generic)
            // equipment keeps getting tagged even when the planner config
            // failed to load.
            if let Some(recs) = &data.dungeon_recs {
                equipment::enrich_equipment(
                    plan,
                    &recs.equipment,
                    data.planner_config.as_ref(),
                );
            }
        }
    }

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
        let equip_catalog = data.dungeon_recs.as_ref().map(|r| &r.equipment);

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

                // Preview toggle
                let preview_label = if entry.show_preview { "▼ Preview" } else { "▶ Preview" };
                if ui
                    .add(egui::Button::new(
                        RichText::new(preview_label)
                            .color(if entry.show_preview { style::ACCENT } else { style::TEXT_MUTED })
                            .size(11.0),
                    ).fill(Color32::TRANSPARENT))
                    .clicked()
                {
                    entry.show_preview = !entry.show_preview;
                }
            });

            // Preview panel
            if entry.show_preview
                && let Some(recs) = &data.dungeon_recs {
                    show_dungeon_preview(ui, entry.dungeon, entry.depth, recs, equip_catalog);
                }
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
                show_plan(ui, plan, state, data);
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
    let enabled = state.constraints_enabled;
    let header_text = if total_constraints > 0 {
        let status = if enabled { "active" } else { "paused" };
        format!("Pet Constraints ({total_constraints} {status})")
    } else {
        "Pet Constraints".to_string()
    };

    egui::CollapsingHeader::new(
        RichText::new(header_text)
            .color(if enabled { style::TEXT_MUTED } else { Color32::from_rgb(0x66, 0x66, 0x66) })
            .size(13.0),
    )
    .id_salt("pet_constraints")
    .default_open(false)
    .show(ui, |ui| {
        // Muted overlay alpha for constraint pills when disabled.
        let pill_alpha = if enabled { 255 } else { 90 };

        // Toolbar: toggle + import/export
        ui.horizontal(|ui| {
            ui.checkbox(&mut state.constraints_enabled, "Enabled");

            ui.separator();

            // Export → clipboard
            if ui
                .button(RichText::new("\u{1F4CB} Export").size(11.0))
                .on_hover_text("Copy constraints to clipboard as YAML")
                .clicked()
            {
                export_constraints_to_clipboard(state);
            }

            // Import ← textbox
            if ui
                .button(RichText::new("\u{1F4DD} Import").size(11.0))
                .on_hover_text("Import constraints from YAML text")
                .clicked()
            {
                state.show_constraints_import = !state.show_constraints_import;
            }

            // Status flash from last import/export
            if let Some((msg, is_err)) = &state.constraints_status {
                let color = if *is_err { style::ERROR } else { style::SUCCESS };
                ui.label(RichText::new(msg).color(color).size(11.0));
            }
        });

        ui.add_space(2.0);

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
                state.constraints_status = None;
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
                state.constraints_status = None;
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
                state.constraints_status = None;
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

        // Show active constraints (visually muted when toggle is off)
        let has_any = !state.forbidden_pets.is_empty()
            || !state.forced_pets.is_empty()
            || !state.whitelisted_pets.is_empty();

        if has_any {
            ui.add_space(4.0);

            // Forbidden pets
            if !state.forbidden_pets.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    let c = style::ERROR.linear_multiply(pill_alpha as f32 / 255.0);
                    ui.label(RichText::new("Forbidden:").color(c).size(11.0));
                    let mut to_remove = Vec::new();
                    let mut sorted: Vec<&String> = state.forbidden_pets.iter().collect();
                    sorted.sort();
                    for name in sorted {
                        let btn = egui::Button::new(
                            RichText::new(format!("{name} ×")).color(c).size(11.0),
                        )
                        .fill(Color32::from_rgba_premultiplied(0x30, 0x15, 0x15, pill_alpha));
                        if ui.add(btn).clicked() {
                            to_remove.push(name.clone());
                        }
                    }
                    if !to_remove.is_empty() {
                        state.constraints_status = None;
                    }
                    for name in to_remove {
                        state.forbidden_pets.remove(&name);
                    }
                });
            }

            // Whitelisted pets
            if !state.whitelisted_pets.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    let c = Color32::from_rgba_premultiplied(0x88, 0xcc, 0xff, pill_alpha);
                    ui.label(
                        RichText::new("Whitelisted:").color(c).size(11.0),
                    );
                    let mut to_remove = Vec::new();
                    let mut sorted: Vec<&String> = state.whitelisted_pets.iter().collect();
                    sorted.sort();
                    for name in sorted {
                        let btn = egui::Button::new(
                            RichText::new(format!("{name} ×")).color(c).size(11.0),
                        )
                        .fill(Color32::from_rgba_premultiplied(0x15, 0x20, 0x30, pill_alpha));
                        if ui.add(btn).clicked() {
                            to_remove.push(name.clone());
                        }
                    }
                    if !to_remove.is_empty() {
                        state.constraints_status = None;
                    }
                    for name in to_remove {
                        state.whitelisted_pets.remove(&name);
                    }
                });
            }

            // Forced pets
            if !state.forced_pets.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    let c = style::SUCCESS.linear_multiply(pill_alpha as f32 / 255.0);
                    ui.label(RichText::new("Forced:").color(c).size(11.0));
                    let mut to_remove = Vec::new();
                    for (i, (dungeon, name)) in state.forced_pets.iter().enumerate() {
                        let target = match dungeon {
                            Some(d) => dungeon_label(*d),
                            None => "Any",
                        };
                        let btn = egui::Button::new(
                            RichText::new(format!("{name} → {target} ×")).color(c).size(11.0),
                        )
                        .fill(Color32::from_rgba_premultiplied(0x15, 0x30, 0x15, pill_alpha));
                        if ui.add(btn).clicked() {
                            to_remove.push(i);
                        }
                    }
                    if !to_remove.is_empty() {
                        state.constraints_status = None;
                    }
                    for i in to_remove.into_iter().rev() {
                        state.forced_pets.remove(i);
                    }
                });
            }
        }
    });

    // Import dialog (floating window, outside the collapsible so it doesn't
    // get clipped — the button that opens it lives inside the section above).
    show_constraints_import_dialog(ui.ctx(), state, data);
}

// =============================================================================
// Constraints import / export
// =============================================================================

/// Export the current constraints to the system clipboard as YAML.
fn export_constraints_to_clipboard(state: &mut DungeonState) {
    use crate::state::{ConstraintsState, ForcedEntry};

    let mut forbidden: Vec<String> = state.forbidden_pets.iter().cloned().collect();
    forbidden.sort();
    let mut whitelisted: Vec<String> = state.whitelisted_pets.iter().cloned().collect();
    whitelisted.sort();

    let export = ConstraintsState {
        enabled: state.constraints_enabled,
        forbidden,
        whitelisted,
        forced: state
            .forced_pets
            .iter()
            .map(|(dungeon, pet)| ForcedEntry {
                pet: pet.clone(),
                dungeon: *dungeon,
            })
            .collect(),
    };

    match serde_yaml::to_string(&export) {
        Ok(yaml) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&yaml)) {
                    Ok(()) => {
                        state.constraints_status =
                            Some(("Copied to clipboard".to_string(), false));
                    }
                    Err(e) => {
                        state.constraints_status =
                            Some((format!("Clipboard error: {e}"), true));
                    }
                }
            }
            #[cfg(target_arch = "wasm32")]
            {
                if let Some(window) = web_sys::window() {
                    let clipboard = window.navigator().clipboard();
                    let promise = clipboard.write_text(&yaml);
                    // Await the clipboard Promise so we don't claim success
                    // on a rejected write. The success/failure status can't
                    // easily be communicated back to DungeonState from the
                    // async closure (no channel wired up), so we set an
                    // optimistic message and log failures. In practice
                    // writeText rarely fails — it runs on a user gesture in
                    // a secure context (GitHub Pages).
                    state.constraints_status =
                        Some(("Copied to clipboard".to_string(), false));
                    wasm_bindgen_futures::spawn_local(async move {
                        if wasm_bindgen_futures::JsFuture::from(promise)
                            .await
                            .is_err()
                        {
                            log::warn!("WASM clipboard write_text rejected");
                        }
                    });
                } else {
                    state.constraints_status =
                        Some(("Clipboard not available".to_string(), true));
                }
            }
        }
        Err(e) => {
            state.constraints_status = Some((format!("Serialize error: {e}"), true));
        }
    }
}

/// Import dialog window for pasting constraints YAML.
fn show_constraints_import_dialog(
    ctx: &egui::Context,
    state: &mut DungeonState,
    data: &DataStore,
) {
    if !state.show_constraints_import {
        return;
    }

    egui::Window::new("Import Pet Constraints")
        .collapsible(false)
        .resizable(true)
        .default_size([500.0, 300.0])
        .show(ctx, |ui| {
            ui.label(
                RichText::new("Paste constraints YAML below. This will replace all current constraints.")
                    .color(style::TEXT_MUTED),
            );
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut state.constraints_import_text)
                            .desired_width(f32::INFINITY)
                            .desired_rows(10)
                            .font(egui::TextStyle::Monospace)
                            .hint_text(CONSTRAINTS_HINT_TEXT),
                    );
                });

            // Error display from last parse attempt
            if let Some((msg, true)) = &state.constraints_status {
                ui.label(RichText::new(msg).color(style::ERROR).size(11.0));
            }

            ui.horizontal(|ui| {
                if ui.button("Import").clicked()
                    && !state.constraints_import_text.is_empty()
                {
                    import_constraints(state, data);
                }
                if ui.button("Cancel").clicked() {
                    state.show_constraints_import = false;
                    state.constraints_status = None;
                }
            });
        });
}

const CONSTRAINTS_HINT_TEXT: &str = "\
forbidden:
  - Hourglass
whitelisted:
  - Bee
forced:
  - pet: Frog
  - pet: Cat
    dungeon: WaterTemple";

/// Parse the import text as YAML, validate pet names against the roster,
/// and replace the current constraints if everything checks out.
fn import_constraints(state: &mut DungeonState, data: &DataStore) {
    use crate::state::ConstraintsState;

    let text = &state.constraints_import_text;

    let parsed: ConstraintsState = match serde_yaml::from_str(text) {
        Ok(c) => c,
        Err(e) => {
            state.constraints_status = Some((format!("YAML parse error: {e}"), true));
            return;
        }
    };

    // Build a set of known pet names for validation
    let known_names: HashSet<String> = data
        .merged
        .iter()
        .map(|p| p.name.clone())
        .collect();

    // Validate all pet names
    let mut errors: Vec<String> = Vec::new();
    let all_names = parsed
        .forbidden
        .iter()
        .chain(parsed.whitelisted.iter())
        .chain(parsed.forced.iter().map(|f| &f.pet));

    for name in all_names {
        if !known_names.contains(name) {
            let suggestion = find_closest_name(name, &known_names);
            let msg = if let Some(closest) = suggestion {
                format!("Unknown pet: \"{name}\" — did you mean \"{closest}\"?")
            } else {
                format!("Unknown pet: \"{name}\"")
            };
            errors.push(msg);
        }
    }

    if !errors.is_empty() {
        state.constraints_status = Some((errors.join("\n"), true));
        return;
    }

    // All names valid — replace current constraints.
    state.constraints_enabled = parsed.enabled;
    state.forbidden_pets = parsed.forbidden.into_iter().collect();
    state.whitelisted_pets = parsed.whitelisted.into_iter().collect();
    state.forced_pets = parsed
        .forced
        .into_iter()
        .map(|f| (f.dungeon, f.pet))
        .collect();

    let count = state.forbidden_pets.len()
        + state.whitelisted_pets.len()
        + state.forced_pets.len();
    state.constraints_status = Some((format!("Imported {count} constraints"), false));
    state.constraints_import_text.clear();
    state.show_constraints_import = false;
}

/// Find the closest known pet name to an unknown input, for error
/// suggestions. Uses case-insensitive prefix matching and a simple
/// edit distance check (Levenshtein ≤ 2).
fn find_closest_name(unknown: &str, known: &HashSet<String>) -> Option<String> {
    let lower = unknown.to_lowercase();

    // Precompute lowercase forms once to avoid repeated allocation per candidate.
    let known_lower: Vec<(&String, String)> = known
        .iter()
        .map(|k| (k, k.to_lowercase()))
        .collect();

    // Exact case-insensitive match (shouldn't happen, but safety net)
    if let Some((original, _)) = known_lower.iter().find(|(_, kl)| *kl == lower) {
        return Some((*original).clone());
    }

    // Prefix match: "Hourgl" → "Hourglass"
    let prefix_matches: Vec<&String> = known_lower
        .iter()
        .filter(|(_, kl)| kl.starts_with(&lower) || lower.starts_with(kl.as_str()))
        .map(|(original, _)| *original)
        .collect();
    if prefix_matches.len() == 1 {
        return Some(prefix_matches[0].clone());
    }

    // Simple edit distance: compute once per candidate, keep the closest
    // within distance 2.
    known_lower
        .iter()
        .map(|(original, kl)| (original, levenshtein(&lower, kl)))
        .filter(|(_, dist)| *dist <= 2)
        .min_by_key(|(_, dist)| *dist)
        .map(|(original, _)| (*original).clone())
}

/// Simple Levenshtein distance. Fine for short pet names (max ~25 chars).
#[allow(clippy::needless_range_loop)]
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[m][n]
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

    // Solve all dungeons simultaneously — no pet reuse across teams.
    // The planner config is passed as Option so the solver falls back to
    // the generic wiki-driven behavior if the config failed to load.
    state.plans = solver::solve_multi(
        &requests,
        &data.merged,
        &constraints,
        data.planner_config.as_ref(),
    );

    // Enrich with equipment suggestions. Static (non-generic) gear is
    // always tagged; computed suggestions for generic/missing slots need
    // the planner config, which is passed through as `Option` so a
    // missing config still lets the static path run.
    for plan in &mut state.plans {
        equipment::enrich_equipment(
            plan,
            &recs.equipment,
            data.planner_config.as_ref(),
        );
    }

    // Mark plans as current
    state.last_data_version = data.data_version;
}

// =============================================================================
// Dungeon preview (static requirements & party composition)
// =============================================================================

/// Show a compact preview of a dungeon's requirements and recommended party.
/// This is a quick-reference view — no solving/assignment, just the static YAML data.
fn show_dungeon_preview(
    ui: &mut Ui,
    dungeon: Dungeon,
    depth: u8,
    recs: &DungeonRecommendations,
    catalog: Option<&EquipmentCatalog>,
) {
    let Some(dungeon_data) = recs.dungeons.get(&dungeon) else {
        return;
    };
    let Some(depth_data) = dungeon_data.depths.get(&depth) else {
        ui.label(
            RichText::new(format!("  No data for D{depth}"))
                .color(style::TEXT_MUTED)
                .italics()
                .size(11.0),
        );
        return;
    };

    ui.indent(format!("preview_{dungeon:?}_{depth}"), |ui| {
        // Requirements
        let reqs = &depth_data.requirements;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Requirements:")
                    .color(style::TEXT_MUTED)
                    .size(11.0)
                    .strong(),
            );
            ui.label(
                RichText::new(format!(
                    "Avg DL {}  |  CL {}  |  {} rooms × {} monsters",
                    reqs.dungeon_level_avg, reqs.class_level,
                    depth_data.rooms, depth_data.monsters_per_room,
                ))
                    .color(style::TEXT_NORMAL)
                    .size(11.0),
            );
        });

        // Party composition — compact table
        ui.add_space(2.0);
        ui.label(
            RichText::new("Recommended Party:")
                .color(style::TEXT_MUTED)
                .size(11.0)
                .strong(),
        );

        for (i, slot) in depth_data.party.iter().enumerate() {
            let row_label = if i < 3 { "F" } else { "B" };
            let pos = i + 1;

            ui.horizontal(|ui| {
                // Position
                ui.label(
                    RichText::new(format!("  {row_label}{pos}"))
                        .color(style::TEXT_MUTED)
                        .size(10.0)
                        .family(egui::FontFamily::Monospace),
                );

                // Class
                match &slot.class {
                    Some(class) => widgets::class_label(ui, class),
                    None => {
                        ui.label(RichText::new("Any").color(style::TEXT_MUTED).size(12.0));
                    }
                }

                // Element
                match &slot.element {
                    Some(el) => widgets::element_badge(ui, el),
                    None => {
                        ui.label(RichText::new("any").color(style::TEXT_MUTED).size(10.0));
                    }
                }

                // Equipment (if specified and not generic)
                if let Some(equip) = &slot.equipment {
                    let parts: Vec<String> = [
                        equip.weapon.as_deref().map(|k| resolve_equip_name(k, catalog)),
                        equip.armor.as_deref().map(|k| resolve_equip_name(k, catalog)),
                        equip.accessory.as_deref().map(|k| resolve_equip_name(k, catalog)),
                    ]
                    .into_iter()
                    .flatten()
                    .collect();

                    if !parts.is_empty() {
                        ui.label(
                            RichText::new(format!("  {}", parts.join(" / ")))
                                .color(Color32::from_rgb(0x88, 0x99, 0xcc))
                                .size(10.0),
                        );
                    }

                    // Gem recommendations
                    if let Some(gems) = &equip.gems {
                        let gem_parts: Vec<String> = [
                            gems.weapon.as_ref().map(|e| format!("{e:?}")),
                            gems.armor.as_ref().map(|e| format!("{e:?}")),
                            gems.accessory.as_ref().map(|e| format!("{e:?}")),
                        ]
                        .into_iter()
                        .flatten()
                        .collect();

                        if !gem_parts.is_empty() {
                            ui.label(
                                RichText::new(format!("💎{}", gem_parts.join("/")))
                                    .color(Color32::from_rgb(0xcc, 0x99, 0xff))
                                    .size(9.0),
                            );
                        }
                    }
                }
            });
        }

        // Items needed
        if !depth_data.party_items.is_empty() {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Items:")
                        .color(style::TEXT_MUTED)
                        .size(11.0)
                        .strong(),
                );
                let item_strs: Vec<String> = depth_data
                    .party_items
                    .iter()
                    .map(|pi| {
                        let name = recs
                            .items
                            .get(&pi.item)
                            .map(|i| i.name.as_str())
                            .unwrap_or(&pi.item);
                        format!("{}×{}", pi.quantity, name)
                    })
                    .collect();
                ui.label(
                    RichText::new(item_strs.join("  "))
                        .color(style::TEXT_NORMAL)
                        .size(11.0),
                );
            });
        }

        ui.add_space(4.0);
    });
}

// =============================================================================
// Plan display
// =============================================================================

fn show_plan(ui: &mut Ui, plan: &DungeonPlan, state: &DungeonState, data: &DataStore) {
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

    // Look up equipment catalog and standards for this dungeon
    let equip_catalog = data.dungeon_recs.as_ref().map(|r| &r.equipment);
    let standards = state.standards_for(plan.dungeon, plan.depth);

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
                show_slot_card(ui, &plan.assignments[i], slot_width, equip_catalog, standards);
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
                CoverageKind::Equipment => "Gear",
                CoverageKind::Synergy => "Team",
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
    // Always divide by 6 (full team size) so empty slots count as DL 0.
    let avg_dungeon_level = assigned_exports.iter().map(|e| e.dungeon_level as u64).sum::<u64>() / 6;
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
            let growth_ok = reqs.total_growth.is_none_or(|req| min_growth >= req);

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
                                if let Some(needed) = rec
                                    && cur != Some(*needed) {
                                        *gems_needed.entry(*needed).or_insert(0) += 1;
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

        let matches = cur_equip
            .as_ref()
            .is_some_and(|cur| equip_matches_rec(&cur.name, key, catalog));

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
    standards: EquipmentStandard,
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
                show_equipment_comparison(&mut child, suggestion, current_loadout, equip_catalog, standards);
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
/// ✓ if current gear matches (with quality/upgrade coloring), or the current gear name if different.
/// Also shows gem status: ◆ in element color if matched, ◆Element in warning if missing.
fn show_equipment_comparison(
    ui: &mut Ui,
    suggestion: &equipment::EquipmentSuggestion,
    current_loadout: Option<&itrtg_models::Loadout>,
    catalog: Option<&EquipmentCatalog>,
    standards: EquipmentStandard,
) {
    let equip = &suggestion.equipment;
    let gems = equip.gems.as_ref();
    let is_computed = suggestion.source == EquipmentSource::Computed;

    let rec_color = if is_computed {
        Color32::from_rgb(0x88, 0x99, 0xcc)
    } else {
        style::TEXT_NORMAL
    };

    type EquipLine<'a> = (&'a str, Option<&'a str>, Option<&'a Element>, Option<&'a itrtg_models::Equipment>);
    let lines: [EquipLine<'_>; 3] = [
        ("W:", equip.weapon.as_deref(), gems.and_then(|g| g.weapon.as_ref()),
         current_loadout.and_then(|l| l.weapon.as_ref())),
        ("A:", equip.armor.as_deref(), gems.and_then(|g| g.armor.as_ref()),
         current_loadout.and_then(|l| l.armor.as_ref())),
        ("Ac:", equip.accessory.as_deref(), gems.and_then(|g| g.accessory.as_ref()),
         current_loadout.and_then(|l| l.accessory.as_ref())),
    ];

    for (prefix, rec_key, rec_gem, current) in &lines {
        if let Some(key) = rec_key {
            let rec_name = resolve_equip_name(key, catalog);
            let gem_str = match rec_gem {
                Some(el) => format!(" [{el:?}]"),
                None => String::new(),
            };

            // Check if current gear matches the recommendation (or is an upgrade)
            let (matches_line, cur_equip) = match current {
                Some(cur) => (equip_matches_rec(&cur.name, key, catalog), Some(*cur)),
                None => (false, None),
            };

            ui.horizontal(|ui| {
                // Recommendation label
                let mut rec_text = RichText::new(format!("{prefix} {rec_name}{gem_str}"))
                    .color(rec_color)
                    .size(10.0);
                if is_computed {
                    rec_text = rec_text.italics();
                }
                ui.label(rec_text);

                // Current status indicator
                match (matches_line, cur_equip) {
                    (true, Some(cur)) => {
                        // Equipment matches — check tier, quality, upgrade against standards
                        let cur_tier = catalog.and_then(|cat| {
                            cat.find_key_by_name_exact(&cur.name)
                                .and_then(|k| cat.lookup(k))
                                .map(|e| e.tier)
                        });

                        let tier_ok = cur_tier.is_none_or(|t| t >= standards.min_tier);
                        let quality_ok = cur.quality >= standards.min_quality;
                        let upgrade_ok = cur.upgrade_level.unwrap_or(0) >= standards.min_upgrade;

                        if tier_ok {
                            // Tier is fine — show ✓ with granular quality/upgrade coloring
                            ui.label(
                                RichText::new("✓")
                                    .color(style::SUCCESS)
                                    .size(9.0),
                            );
                            ui.label(
                                RichText::new(format!("{:?}", cur.quality))
                                    .color(if quality_ok { style::SUCCESS } else { style::WARNING })
                                    .size(9.0),
                            );
                            if let Some(lv) = cur.upgrade_level {
                                ui.label(
                                    RichText::new(format!("+{lv}"))
                                        .color(if upgrade_ok { style::SUCCESS } else { style::WARNING })
                                        .size(9.0),
                                );
                            }
                        } else {
                            // Tier too low — flag everything in warning
                            let grade = match cur.upgrade_level {
                                Some(lv) => format!("✓ T{} {:?}+{lv}", cur_tier.unwrap_or(0), cur.quality),
                                None => format!("✓ T{} {:?}", cur_tier.unwrap_or(0), cur.quality),
                            };
                            ui.label(
                                RichText::new(grade)
                                    .color(style::WARNING)
                                    .size(9.0),
                            );
                        }
                    }
                    (false, Some(cur)) => {
                        // Different equipment
                        ui.label(
                            RichText::new(format!("✗ {}", cur.name))
                                .color(Color32::from_rgb(0xdd, 0x88, 0x44))
                                .size(9.0),
                        );
                    }
                    (_, None) => {
                        // No equipment
                        ui.label(
                            RichText::new("—")
                                .color(style::TEXT_MUTED)
                                .size(9.0),
                        );
                    }
                }

                // Gem status
                let current_gem = cur_equip.and_then(|c| c.gem);
                if let Some(needed_elem) = rec_gem {
                    let gem_color = style::element_color(needed_elem);
                    if current_gem == Some(**needed_elem) {
                        // Gem matches recommendation
                        ui.label(
                            RichText::new("◆")
                                .color(gem_color)
                                .size(8.0),
                        );
                    } else {
                        // Gem missing or wrong element
                        ui.label(
                            RichText::new(format!("◆{needed_elem:?}"))
                                .color(style::WARNING)
                                .size(8.0),
                        );
                    }
                } else if let Some(cur_gem) = current_gem {
                    // No recommendation, but pet has a gem — show it informally
                    ui.label(
                        RichText::new("◆")
                            .color(style::element_color(&cur_gem))
                            .size(8.0),
                    );
                }
            });
        }
    }
}

/// Check if a pet's current equipment matches a recommendation (by catalog key),
/// including higher-tier upgrades in the same crafting chain.
fn equip_matches_rec(
    current_name: &str,
    rec_key: &str,
    catalog: Option<&EquipmentCatalog>,
) -> bool {
    let Some(cat) = catalog else {
        // No catalog: fall back to name substring match
        let cur_lower = current_name.to_lowercase();
        let rec_name = rec_key.replace('_', " ").to_lowercase();
        return cur_lower.contains(&rec_name) || rec_name.contains(&cur_lower);
    };

    // Find the current equipment's catalog key by name
    if let Some(cur_key) = cat.find_key_by_name_exact(current_name) {
        // Check if it's the same item or an upgrade of the recommendation
        if cat.is_same_line(cur_key, rec_key) {
            return true;
        }
    }

    // Fallback: name substring match (handles items not in catalog)
    let rec_name = resolve_equip_name(rec_key, catalog).to_lowercase();
    let cur_lower = current_name.to_lowercase();
    cur_lower.contains(&rec_name) || rec_name.contains(&cur_lower)
}

/// Resolve a catalog key to a display name.
fn resolve_equip_name(key: &str, catalog: Option<&EquipmentCatalog>) -> String {
    if let Some(cat) = catalog
        && let Some(entry) = cat.lookup(key) {
            return entry.name.clone();
        }
    // Humanize generic keys: "generic_t2_s10" → "Generic T2"
    if let Some(rest) = key.strip_prefix("generic_t") {
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
