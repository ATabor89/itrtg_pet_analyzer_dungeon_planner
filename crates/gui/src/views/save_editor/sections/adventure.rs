//! Adventure section: the adventure-mode inventory (`032.d`), cores (`032.G`),
//! the adventurer's per-class progression (`032.b.f`), researches (`032.H.a`),
//! and a read-only view of the equipped battle skills (`032.b.g`).
//!
//! No typed model for these lists, so rows read straight from the raw tree via
//! the `AdventureItemField` / `CoreField` / `ClassProgressionField` /
//! `ResearchField` descriptors; edits stage by raw list index/path. The battle
//! skills (`032.b.g`) are shown read-only — their per-field meanings beyond skill
//! id + level aren't pinned down yet, so nothing there is editable.

use std::collections::HashMap;

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use save_parser::labels::{
    AdvProfessionField, AdventureItemField, AdventurerField, ClassProgressionField, CoreField,
    ResearchField,
};
use save_parser::raw::Raw;
use save_parser::{items, model};

use crate::style;
use crate::views::save_editor::session::EditSession;

struct ItemRow {
    index: usize,
    item_id: u32,
    name: String,
    count: String,
}

/// One per-class progression row (`032.b.f.<index>`).
struct ClassRow {
    index: usize,
    class_id: u32,
    name: String,
    level: String,
    exp: String,
}

/// One research row (`032.H.a.<index>`).
struct ResearchRow {
    index: usize,
    research_id: u32,
    name: String,
    level: String,
    /// Authoritative level cap (`model::research_max`); `None` = uncapped. The
    /// save's own per-entry max field is unreliable, so it isn't used here.
    max_level: Option<u64>,
    in_progress: bool,
}

/// One battle-skill row (`032.b.g.<index>`) — read-only.
struct SkillRow {
    skill_id: u32,
    name: String,
    level: String,
}

/// One side-profession row (`032.j.<index>`).
struct ProfRow {
    index: usize,
    prof_id: u32,
    name: String,
    level: String,
    exp: String,
}

struct CoreRow {
    index: usize,
    enemy_id: u32,
    name: String,
    count: String,
    quality: u32,
}

/// State for the "add item" / "add core" modal.
#[derive(Default)]
struct AddState {
    open: bool,
    /// `false` = add an inventory item, `true` = add a core.
    is_core: bool,
    /// Chosen item id (inventory) or enemy id (core).
    id: u32,
    count: String,
    /// Core quality (cores only).
    quality: u32,
}

/// State for the "Unlock class" modal.
#[derive(Default)]
struct AddClassState {
    open: bool,
    /// Chosen target class id.
    target: u32,
}

#[derive(Default)]
pub struct AdventureEditState {
    /// Adventure-inventory name filter.
    f_name: String,
    /// Research name filter.
    r_name: String,
    /// Per-row count buffers (keyed by `032.d` index).
    item_buffers: HashMap<usize, String>,
    /// Buffers for the class-progression / research / adventurer cells, keyed by
    /// the field's dotted raw path (each cell is unique, so one map suffices).
    cell_buffers: HashMap<String, String>,
    add: AddState,
    add_class: AddClassState,
    status: Option<(String, bool)>,
}

/// A step in a class-unlock plan: set class `class_id` to `level` (tier `d`).
struct UnlockStep {
    class_id: u32,
    level: u32,
    tier: &'static str,
}

/// Compute the class-progression entries needed so `target` becomes selectable:
/// the prerequisite closure (each prereq at its required level, recursively) plus
/// the target itself at level 1. `existing` maps class id → current level. Returns
/// only the classes that are missing or below the needed level, sorted by id.
/// `None` if the target has no known unlock chain.
fn unlock_plan(target: u32, existing: &HashMap<u32, u32>) -> Option<Vec<UnlockStep>> {
    fn visit(class: u32, need: u32, required: &mut HashMap<u32, u32>) -> Option<()> {
        let (_tier, prereqs) = items::adventure_class_unlock(class)?;
        let slot = required.entry(class).or_insert(0);
        // A class's prerequisites are constant (independent of how high *it* is
        // needed), so each class only needs visiting once — we just keep the max
        // required level. `*slot == 0` means not yet visited (needs are always
        // ≥ 1). This dedups diamond chains and is safe against bad-data cycles.
        let first_visit = *slot == 0;
        *slot = need.max(*slot);
        if first_visit {
            for &(p, lvl) in prereqs {
                visit(p, lvl, required)?;
            }
        }
        Some(())
    }
    let mut required: HashMap<u32, u32> = HashMap::new();
    visit(target, 1, &mut required)?;
    let mut steps: Vec<UnlockStep> = required
        .into_iter()
        .filter_map(|(class, need)| {
            let cur = existing.get(&class).copied();
            // Skip classes already present at or above the needed level.
            if cur.is_some_and(|c| c >= need) {
                return None;
            }
            let level = need.max(cur.unwrap_or(0));
            let (tier, _) = items::adventure_class_unlock(class)?;
            Some(UnlockStep { class_id: class, level, tier })
        })
        .collect();
    steps.sort_by_key(|s| s.class_id);
    Some(steps)
}

/// Quality letter for an id (F…SSS), falling back to the number.
fn quality_label(q: u32) -> String {
    items::quality_name(q).map_or_else(|| q.to_string(), str::to_string)
}

fn read_items(session: &EditSession) -> Vec<ItemRow> {
    let Some(Raw::List(items_list)) = session.root().get_path(&["032", "d"]) else {
        return Vec::new();
    };
    (0..items_list.len())
        .map(|index| {
            let i = index.to_string();
            let item_id: u32 = session
                .value(&["032", "d", &i, AdventureItemField::Item.key()])
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            ItemRow {
                index,
                item_id,
                name: items::adventure_item_name(item_id)
                    .map_or_else(|| format!("Item {item_id}"), str::to_string),
                count: session
                    .value(&["032", "d", &i, AdventureItemField::Count.key()])
                    .unwrap_or_default(),
            }
        })
        .collect()
}

fn read_cores(session: &EditSession) -> Vec<CoreRow> {
    let Some(Raw::List(cores)) = session.root().get_path(&["032", "G"]) else {
        return Vec::new();
    };
    (0..cores.len())
        .map(|index| {
            let i = index.to_string();
            let enemy_id: u32 = session
                .value(&["032", "G", &i, CoreField::Enemy.key()])
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            CoreRow {
                index,
                enemy_id,
                name: items::adventure_enemy_name(enemy_id)
                    .map_or_else(|| format!("Enemy {enemy_id}"), str::to_string),
                count: session.value(&["032", "G", &i, CoreField::Count.key()]).unwrap_or_default(),
                quality: session
                    .value(&["032", "G", &i, CoreField::Quality.key()])
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0),
            }
        })
        .collect()
}

fn read_class_progression(session: &EditSession) -> Vec<ClassRow> {
    let Some(Raw::List(list)) = session.root().get_path(&["032", "b", "f"]) else {
        return Vec::new();
    };
    (0..list.len())
        .map(|index| {
            let i = index.to_string();
            let class_id: u32 = session
                .value(&["032", "b", "f", &i, ClassProgressionField::Class.key()])
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            ClassRow {
                index,
                class_id,
                name: items::adventure_class_name(class_id)
                    .map_or_else(|| format!("Class {class_id}"), str::to_string),
                level: session
                    .value(&["032", "b", "f", &i, ClassProgressionField::Level.key()])
                    .unwrap_or_default(),
                exp: session
                    .value(&["032", "b", "f", &i, ClassProgressionField::Exp.key()])
                    .unwrap_or_default(),
            }
        })
        .collect()
}

fn read_research(session: &EditSession) -> Vec<ResearchRow> {
    let Some(Raw::List(list)) = session.root().get_path(&["032", "H", "a"]) else {
        return Vec::new();
    };
    (0..list.len())
        .filter_map(|index| {
            let i = index.to_string();
            let research_id: u32 = session
                .value(&["032", "H", "a", &i, ResearchField::Research.key()])
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            // id 0 is an unused placeholder slot in the save — skip it.
            let name = model::research_name(research_id)?;
            Some(ResearchRow {
                index,
                research_id,
                name: name.to_string(),
                level: session
                    .value(&["032", "H", "a", &i, ResearchField::Level.key()])
                    .unwrap_or_default(),
                max_level: model::research_max(research_id),
                in_progress: session
                    .value(&["032", "H", "a", &i, ResearchField::InProgress.key()])
                    .is_some_and(|s| s.trim() == "1"),
            })
        })
        .collect()
}

fn read_battle_skills(session: &EditSession) -> Vec<SkillRow> {
    let Some(Raw::List(list)) = session.root().get_path(&["032", "b", "g"]) else {
        return Vec::new();
    };
    (0..list.len())
        .map(|index| {
            let i = index.to_string();
            // `a` = skill id, `b` = level (best-effort; the remaining fields
            // aren't pinned down, so they stay in the raw tree only).
            let skill_id: u32 = session
                .value(&["032", "b", "g", &i, "a"])
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            SkillRow {
                skill_id,
                name: items::adventure_skill_name(skill_id)
                    .map_or_else(|| format!("Skill {skill_id}"), str::to_string),
                level: session.value(&["032", "b", "g", &i, "b"]).unwrap_or_default(),
            }
        })
        .collect()
}

fn read_professions(session: &EditSession) -> Vec<ProfRow> {
    let Some(Raw::List(list)) = session.root().get_path(&["032", "j"]) else {
        return Vec::new();
    };
    (0..list.len())
        .map(|index| {
            let i = index.to_string();
            let at = |k: &str| session.value(&["032", "j", &i, k]).unwrap_or_default();
            let prof_id: u32 = at(AdvProfessionField::Profession.key()).trim().parse().unwrap_or(0);
            ProfRow {
                index,
                prof_id,
                name: items::adventure_profession_name(prof_id)
                    .map_or_else(|| format!("Profession {prof_id}"), str::to_string),
                level: at(AdvProfessionField::Level.key()),
                exp: at(AdvProfessionField::Exp.key()),
            }
        })
        .collect()
}

pub fn show(ui: &mut egui::Ui, session: &mut EditSession, st: &mut AdventureEditState) {
    ui.heading("Adventure");

    if session.root().get_path(&["032"]).is_none() {
        ui.label(RichText::new("No adventure data in this save.").color(style::TEXT_MUTED));
        return;
    }

    if let Some((msg, err)) = &st.status {
        let color = if *err { style::ERROR } else { style::SUCCESS };
        ui.label(RichText::new(msg).color(color).size(12.0));
    }

    let items_rows = read_items(session);
    let cores = read_cores(session);
    let class_rows = read_class_progression(session);
    let prof_rows = read_professions(session);
    let research_rows = read_research(session);
    let skill_rows = read_battle_skills(session);
    // Adventurer summary scalars (032.b): a = active class, b = level, c = exp.
    let active_class_id: u32 =
        session.value(&["032", "b", AdventurerField::Class.key()]).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    let active_class = items::adventure_class_name(active_class_id)
        .map_or_else(|| format!("Class {active_class_id}"), str::to_string);

    // Edits collected during the read-only render, applied after.
    let mut edits: Vec<(Vec<String>, String, String)> = Vec::new();

    // --- Adventure inventory ---
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Adventure Inventory").strong());
        ui.label(RichText::new("· edit item counts").color(style::TEXT_MUTED).size(11.0));
        if ui.button("➕ Add item").clicked() {
            let id = items::known_adventure_items().first().map_or(0, |(id, _)| *id);
            st.add = AddState { open: true, is_core: false, id, count: "1".into(), quality: 0 };
        }
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("Filter:").color(style::TEXT_MUTED));
        ui.add(egui::TextEdit::singleline(&mut st.f_name).desired_width(160.0));
        if ui.button("× clear").clicked() {
            st.f_name.clear();
        }
        ui.label(
            RichText::new(format!("{} items", items_rows.len()))
                .color(style::TEXT_MUTED)
                .size(11.0),
        );
    });

    let needle = st.f_name.trim().to_lowercase();
    let filtered: Vec<usize> = (0..items_rows.len())
        .filter(|&i| needle.is_empty() || items_rows[i].name.to_lowercase().contains(&needle))
        .collect();
    item_table(ui, st, &items_rows, &filtered, &mut edits);

    // --- Cores ---
    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(RichText::new("Cores").strong());
        ui.label(RichText::new("· upgrade core quality").color(style::TEXT_MUTED).size(11.0));
        if ui.button("➕ Add core").clicked() {
            let id = items::known_adventure_enemies().first().map_or(0, |(id, _)| *id);
            st.add = AddState { open: true, is_core: true, id, count: "1".into(), quality: 8 };
        }
    });
    if cores.is_empty() {
        ui.label(RichText::new("No cores in this save.").color(style::TEXT_MUTED));
    } else {
        core_table(ui, &cores, &mut edits);
    }

    // --- Adventurer ---
    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new("Adventurer").strong());
    egui::Grid::new("adv_adventurer").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Active class");
        ui.label(RichText::new(&active_class).color(style::TEXT_MUTED));
        ui.end_row();
        scalar_cell(ui, st, &["032", "b", AdventurerField::Level.key()], "Level", None, &mut edits, session);
        ui.end_row();
        scalar_cell(ui, st, &["032", "b", AdventurerField::Exp.key()], "Exp", None, &mut edits, session);
        ui.end_row();
    });

    // --- Professions (Crafting / Smithing / Alchemy / …) ---
    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new("Professions").strong());
    ui.label(
        RichText::new("Side-profession level & exp. The next-level threshold is computed, so only level and exp are stored.")
            .color(style::TEXT_MUTED)
            .size(11.0),
    );
    if prof_rows.is_empty() {
        ui.label(RichText::new("No professions in this save.").color(style::TEXT_MUTED));
    } else {
        prof_table(ui, st, &prof_rows, &mut edits);
    }

    // --- Class progression ---
    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(RichText::new("Class Progression").strong());
        if ui
            .button("➕ Unlock class")
            .on_hover_text("Unlock a class by materializing its full prerequisite chain")
            .clicked()
        {
            // Default to the first unlockable class not already present.
            let present: std::collections::HashSet<u32> =
                class_rows.iter().map(|r| r.class_id).collect();
            let first = unlockable_classes()
                .into_iter()
                .find(|(id, _)| !present.contains(id))
                .map_or(2, |(id, _)| id);
            st.add_class = AddClassState { open: true, target: first };
        }
    });
    ui.label(
        RichText::new("Per-class level & exp; classes advance independently.")
            .color(style::TEXT_MUTED)
            .size(11.0),
    );
    if class_rows.is_empty() {
        ui.label(RichText::new("No class progression in this save.").color(style::TEXT_MUTED));
    } else {
        class_table(ui, st, &class_rows, &mut edits);
    }

    // --- Research ---
    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(RichText::new("Research").strong());
        ui.label(RichText::new("Filter:").color(style::TEXT_MUTED));
        ui.add(egui::TextEdit::singleline(&mut st.r_name).desired_width(160.0));
        if ui.button("× clear").clicked() {
            st.r_name.clear();
        }
    });
    let r_needle = st.r_name.trim().to_lowercase();
    let r_filtered: Vec<usize> = (0..research_rows.len())
        .filter(|&i| r_needle.is_empty() || research_rows[i].name.to_lowercase().contains(&r_needle))
        .collect();
    if research_rows.is_empty() {
        ui.label(RichText::new("No research in this save.").color(style::TEXT_MUTED));
    } else {
        research_table(ui, st, &research_rows, &r_filtered, &mut edits);
    }

    // --- Battle skills (read-only; fields beyond id+level not yet identified) ---
    ui.add_space(8.0);
    ui.separator();
    egui::CollapsingHeader::new(format!("Battle Skills ({}) — read-only", skill_rows.len()))
        .default_open(false)
        .show(ui, |ui| {
            ui.label(
                RichText::new(
                    "Equipped battle skills. Only the skill id and level are identified; \
                     edit the rest in the Raw Save Tree (032.b.g).",
                )
                .color(style::TEXT_MUTED)
                .size(11.0),
            );
            skill_table(ui, &skill_rows);
        });

    // Apply.
    let mut ok = false;
    let mut had_err = false;
    for (path, label, value) in edits {
        let p: Vec<&str> = path.iter().map(String::as_str).collect();
        match session.set_scalar(&p, label, &value) {
            Ok(_) => ok = true,
            Err(e) => {
                had_err = true;
                st.status = Some((format!("Edit failed: {e}"), true));
            }
        }
    }
    if ok && !had_err {
        st.status = Some(("Staged adventure edit".to_string(), false));
    }

    // Add-entry modal (item or core).
    if let Some((is_core, id, count, quality)) = add_window(ui.ctx(), &mut st.add) {
        if is_core {
            let label = format!(
                "{} core",
                items::adventure_enemy_name(id).unwrap_or("Enemy")
            );
            st.status = Some(match session.set_core(id, count.trim(), quality, label) {
                Ok(true) => ("Added core".to_string(), false),
                Ok(false) => ("Updated existing core".to_string(), false),
                Err(e) => (format!("Add failed: {e}"), true),
            });
        } else {
            let label = format!(
                "{} count",
                items::adventure_item_name(id).unwrap_or("Item")
            );
            st.status = Some(match session.set_adventure_item(id, count.trim(), label) {
                Ok(true) => ("Added adventure item".to_string(), false),
                Ok(false) => ("Updated existing item".to_string(), false),
                Err(e) => (format!("Add failed: {e}"), true),
            });
        }
    }

    // Unlock-class modal. `existing` reflects the loaded class levels; the plan
    // uses non-lowering sets, so a stale read can only under-raise, never harm.
    let existing: HashMap<u32, u32> = class_rows
        .iter()
        .filter_map(|r| Some((r.class_id, r.level.trim().parse().ok()?)))
        .collect();
    if let Some(target) = unlock_class_window(ui.ctx(), &mut st.add_class, &existing) {
        let cname = items::adventure_class_name(target).unwrap_or("class");
        match unlock_plan(target, &existing) {
            Some(steps) => {
                let mut created = 0;
                let mut raised = 0;
                let mut err = None;
                for step in &steps {
                    let sname = items::adventure_class_name(step.class_id).unwrap_or("class");
                    let label = format!("unlock {cname}: {sname} L{}", step.level);
                    match session.set_class_progression(step.class_id, step.level, step.tier, label) {
                        Ok(true) => created += 1,
                        Ok(false) => raised += 1,
                        Err(e) => {
                            err = Some(e);
                            break;
                        }
                    }
                }
                st.status = Some(match err {
                    Some(e) => (format!("Unlock failed: {e}"), true),
                    None => {
                        (format!("Unlocked {cname} ({created} created, {raised} raised)"), false)
                    }
                });
            }
            None => {
                st.status = Some(("That class has no known unlock chain.".to_string(), true));
            }
        }
    }
}

/// The classes that have a known unlock chain (for the picker), `(id, name)`.
fn unlockable_classes() -> Vec<(u32, &'static str)> {
    (0..=110u32)
        .filter_map(|id| {
            items::adventure_class_unlock(id)?;
            items::adventure_class_name(id).map(|n| (id, n))
        })
        .collect()
}

/// The "Unlock class" modal: pick a target class, preview the entries that will
/// be created/raised (its full prerequisite chain), and confirm. Returns the
/// chosen target id on Unlock.
fn unlock_class_window(
    ctx: &egui::Context,
    st: &mut AddClassState,
    existing: &HashMap<u32, u32>,
) -> Option<u32> {
    if !st.open {
        return None;
    }
    let mut result = None;
    let mut close = false;
    let mut window_open = true;
    egui::Window::new("Unlock Class")
        .collapsible(false)
        .resizable(false)
        .open(&mut window_open)
        .show(ctx, |ui| {
            let opts = unlockable_classes();
            let selected = opts
                .iter()
                .find(|(id, _)| *id == st.target)
                .map_or_else(|| format!("id {}", st.target), |(_, n)| (*n).to_string());
            ui.horizontal(|ui| {
                ui.label("Class:");
                egui::ComboBox::from_id_salt("adv_unlock_class")
                    .selected_text(selected)
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for (id, name) in &opts {
                            ui.selectable_value(&mut st.target, *id, *name);
                        }
                    });
            });
            let plan = unlock_plan(st.target, existing);
            match &plan {
                Some(steps) if steps.is_empty() => {
                    ui.label(
                        RichText::new("Already unlocked — nothing to do.")
                            .color(style::TEXT_MUTED)
                            .size(11.0),
                    );
                }
                Some(steps) => {
                    ui.label(RichText::new("Will create / raise:").size(11.0));
                    for s in steps {
                        let n = items::adventure_class_name(s.class_id).unwrap_or("?");
                        ui.label(
                            RichText::new(format!("• {n} → level {}", s.level))
                                .color(style::TEXT_MUTED)
                                .size(11.0),
                        );
                    }
                }
                None => {
                    ui.label(
                        RichText::new("No known unlock chain for this class.")
                            .color(style::ERROR)
                            .size(11.0),
                    );
                }
            }
            if st.target == 32 {
                ui.label(
                    RichText::new(
                        "Alchemist also needs the Alchemy profession ≥ 30 (set it under Professions).",
                    )
                    .color(style::WARNING)
                    .size(10.0),
                );
            }
            ui.separator();
            ui.horizontal(|ui| {
                let can_apply = plan.as_ref().is_some_and(|s| !s.is_empty());
                if ui.add_enabled(can_apply, egui::Button::new("Unlock")).clicked() {
                    result = Some(st.target);
                    close = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
    st.open = window_open && !close;
    result
}

/// The add-item / add-core modal. Returns `Some((is_core, id, count, quality))`
/// on Add. Pickers are alphabetized like the challenges add dialog.
fn add_window(ctx: &egui::Context, st: &mut AddState) -> Option<(bool, u32, String, u32)> {
    if !st.open {
        return None;
    }
    let is_core = st.is_core;
    let mut result = None;
    let mut close = false;
    let mut window_open = true;
    let title = if is_core { "Add Core" } else { "Add Item" };
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .open(&mut window_open)
        .show(ctx, |ui| {
            let mut opts = if is_core {
                items::known_adventure_enemies()
            } else {
                items::known_adventure_items()
            };
            opts.sort_by(|a, b| a.1.cmp(b.1));
            let selected = opts
                .iter()
                .find(|(id, _)| *id == st.id)
                .map_or_else(|| format!("id {}", st.id), |(_, n)| (*n).to_string());
            ui.horizontal(|ui| {
                ui.label(if is_core { "Enemy:" } else { "Item:" });
                egui::ComboBox::from_id_salt("adv_add_pick")
                    .selected_text(selected)
                    .width(240.0)
                    .show_ui(ui, |ui| {
                        for (id, name) in &opts {
                            ui.selectable_value(&mut st.id, *id, *name);
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Count:");
                ui.add(egui::TextEdit::singleline(&mut st.count).desired_width(100.0));
                if is_core {
                    ui.label("Quality:");
                    egui::ComboBox::from_id_salt("adv_add_quality")
                        .selected_text(quality_label(st.quality))
                        .width(80.0)
                        .show_ui(ui, |ui| {
                            for q in 0..=8u32 {
                                ui.selectable_value(&mut st.quality, q, quality_label(q));
                            }
                        });
                }
            });
            ui.label(
                RichText::new("If the entry already exists, this updates it instead.")
                    .color(style::TEXT_MUTED)
                    .size(10.0),
            );
            ui.separator();
            ui.horizontal(|ui| {
                let ok = st.count.trim().parse::<u64>().is_ok();
                if ui.add_enabled(ok, egui::Button::new("Add")).clicked() {
                    result = Some((is_core, st.id, st.count.trim().to_string(), st.quality));
                    close = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
    st.open = window_open && !close;
    result
}

fn item_table(
    ui: &mut egui::Ui,
    st: &mut AdventureEditState,
    rows: &[ItemRow],
    filtered: &[usize],
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    TableBuilder::new(ui)
        .striped(true)
        .id_salt("adv_items")
        .column(Column::initial(220.0)) // item
        .column(Column::initial(160.0)) // count
        .column(Column::remainder())
        .header(20.0, |mut h| {
            for title in ["Item", "Count", ""] {
                h.col(|ui| {
                    ui.label(RichText::new(title).strong().size(12.0));
                });
            }
        })
        .body(|body| {
            body.rows(24.0, filtered.len(), |mut tr| {
                let row = &rows[filtered[tr.index()]];
                tr.col(|ui| {
                    ui.label(&row.name).on_hover_text(format!("item id {}", row.item_id));
                });
                tr.col(|ui| {
                    let buf = st.item_buffers.entry(row.index).or_insert_with(|| row.count.clone());
                    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(140.0));
                    if resp.changed() {
                        let v = buf.trim();
                        if v.parse::<u64>().is_ok() && v != row.count.trim() {
                            edits.push((
                                vec!["032".into(), "d".into(), row.index.to_string(), AdventureItemField::Count.key().into()],
                                format!("{} count", row.name),
                                v.to_string(),
                            ));
                        }
                    } else if !resp.has_focus() && buf.trim() != row.count.trim() {
                        *buf = row.count.clone();
                    }
                });
                tr.col(|_| {});
            });
        });
}

fn core_table(
    ui: &mut egui::Ui,
    rows: &[CoreRow],
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    TableBuilder::new(ui)
        .striped(true)
        .id_salt("adv_cores")
        .column(Column::initial(220.0)) // enemy
        .column(Column::initial(120.0)) // quality
        .column(Column::initial(100.0)) // count
        .column(Column::remainder())
        .header(20.0, |mut h| {
            for title in ["Core (enemy)", "Quality", "Count", ""] {
                h.col(|ui| {
                    ui.label(RichText::new(title).strong().size(12.0));
                });
            }
        })
        .body(|body| {
            body.rows(24.0, rows.len(), |mut tr| {
                let row = &rows[tr.index()];
                tr.col(|ui| {
                    ui.label(&row.name).on_hover_text(format!("enemy id {}", row.enemy_id));
                });
                tr.col(|ui| {
                    let mut q = row.quality;
                    egui::ComboBox::from_id_salt(("core_q", row.index))
                        .selected_text(quality_label(q))
                        .width(90.0)
                        .show_ui(ui, |ui| {
                            for id in 0..=8u32 {
                                ui.selectable_value(&mut q, id, quality_label(id));
                            }
                        });
                    if q != row.quality {
                        edits.push((
                            vec!["032".into(), "G".into(), row.index.to_string(), CoreField::Quality.key().into()],
                            format!("{} quality", row.name),
                            q.to_string(),
                        ));
                    }
                });
                tr.col(|ui| {
                    ui.label(RichText::new(&row.count).monospace().size(11.0));
                });
                tr.col(|_| {});
            });
        });
}

/// An editable text cell backed by `buffers[path]`. Every field routed through
/// here (level / exp) is a non-negative integer in-game, so it stages an edit
/// only when the value parses as a `u64` (and is ≤ `max` when `max` is set) and
/// differs — rejecting floats, negatives, and `NaN`/`inf`.
fn edit_cell(
    ui: &mut egui::Ui,
    buffers: &mut HashMap<String, String>,
    path: &[&str],
    current: &str,
    label: String,
    max: Option<u64>,
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    let key = path.join(".");
    let buf = buffers.entry(key).or_insert_with(|| current.to_string());
    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(110.0));
    if resp.changed() {
        let v = buf.trim();
        let valid = v.parse::<u64>().is_ok_and(|n| max.is_none_or(|m| n <= m));
        if valid && v != current.trim() {
            edits.push((path.iter().map(|s| s.to_string()).collect(), label, v.to_string()));
        }
    } else if !resp.has_focus() && buf.trim() != current.trim() {
        *buf = current.to_string();
    }
}

/// Like [`edit_cell`] but for a BigDouble field (e.g. profession exp): stages on
/// any finite number, written verbatim so decimals / large magnitudes survive.
fn big_cell(
    ui: &mut egui::Ui,
    buffers: &mut HashMap<String, String>,
    path: &[&str],
    current: &str,
    label: String,
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    let key = path.join(".");
    let buf = buffers.entry(key).or_insert_with(|| current.to_string());
    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(140.0));
    if resp.changed() {
        let v = buf.trim();
        if v.parse::<f64>().is_ok_and(f64::is_finite) && v != current.trim() {
            edits.push((path.iter().map(|s| s.to_string()).collect(), label, v.to_string()));
        }
    } else if !resp.has_focus() && buf.trim() != current.trim() {
        *buf = current.to_string();
    }
}

/// A labeled editable scalar in a grid; reads its current value from `session`.
fn scalar_cell(
    ui: &mut egui::Ui,
    st: &mut AdventureEditState,
    path: &[&str],
    label: &str,
    max: Option<u64>,
    edits: &mut Vec<(Vec<String>, String, String)>,
    session: &EditSession,
) {
    ui.label(label);
    let current = session.value(path).unwrap_or_default();
    edit_cell(ui, &mut st.cell_buffers, path, &current, label.to_string(), max, edits);
}

fn class_table(
    ui: &mut egui::Ui,
    st: &mut AdventureEditState,
    rows: &[ClassRow],
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    TableBuilder::new(ui)
        .striped(true)
        .id_salt("adv_classes")
        .column(Column::initial(180.0)) // class
        .column(Column::initial(100.0)) // level
        .column(Column::remainder()) // exp
        .header(20.0, |mut h| {
            for title in ["Class", "Level", "Exp"] {
                h.col(|ui| {
                    ui.label(RichText::new(title).strong().size(12.0));
                });
            }
        })
        .body(|body| {
            body.rows(24.0, rows.len(), |mut tr| {
                let row = &rows[tr.index()];
                let idx = row.index.to_string();
                tr.col(|ui| {
                    ui.label(&row.name).on_hover_text(format!("class id {}", row.class_id));
                });
                tr.col(|ui| {
                    edit_cell(
                        ui,
                        &mut st.cell_buffers,
                        &["032", "b", "f", &idx, ClassProgressionField::Level.key()],
                        &row.level,
                        format!("{} level", row.name),
                        None,
                        edits,
                    );
                });
                tr.col(|ui| {
                    edit_cell(
                        ui,
                        &mut st.cell_buffers,
                        &["032", "b", "f", &idx, ClassProgressionField::Exp.key()],
                        &row.exp,
                        format!("{} exp", row.name),
                        None,
                        edits,
                    );
                });
            });
        });
}

fn prof_table(
    ui: &mut egui::Ui,
    st: &mut AdventureEditState,
    rows: &[ProfRow],
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    TableBuilder::new(ui)
        .striped(true)
        .id_salt("adv_professions")
        .column(Column::initial(160.0)) // profession
        .column(Column::initial(100.0)) // level
        .column(Column::remainder()) // exp
        .header(20.0, |mut h| {
            for title in ["Profession", "Level", "Exp"] {
                h.col(|ui| {
                    ui.label(RichText::new(title).strong().size(12.0));
                });
            }
        })
        .body(|body| {
            body.rows(24.0, rows.len(), |mut tr| {
                let row = &rows[tr.index()];
                let idx = row.index.to_string();
                tr.col(|ui| {
                    ui.label(&row.name).on_hover_text(format!("profession id {}", row.prof_id));
                });
                tr.col(|ui| {
                    edit_cell(
                        ui,
                        &mut st.cell_buffers,
                        &["032", "j", &idx, AdvProfessionField::Level.key()],
                        &row.level,
                        format!("{} level", row.name),
                        None,
                        edits,
                    );
                });
                tr.col(|ui| {
                    big_cell(
                        ui,
                        &mut st.cell_buffers,
                        &["032", "j", &idx, AdvProfessionField::Exp.key()],
                        &row.exp,
                        format!("{} exp", row.name),
                        edits,
                    );
                });
            });
        });
}

fn research_table(
    ui: &mut egui::Ui,
    st: &mut AdventureEditState,
    rows: &[ResearchRow],
    filtered: &[usize],
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    TableBuilder::new(ui)
        .striped(true)
        .id_salt("adv_research")
        .column(Column::initial(200.0)) // research
        .column(Column::initial(100.0)) // level
        .column(Column::initial(90.0)) // max
        .column(Column::remainder()) // in progress
        .header(20.0, |mut h| {
            for title in ["Research", "Level", "Max", "Researching"] {
                h.col(|ui| {
                    ui.label(RichText::new(title).strong().size(12.0));
                });
            }
        })
        .body(|body| {
            body.rows(24.0, filtered.len(), |mut tr| {
                let row = &rows[filtered[tr.index()]];
                let idx = row.index.to_string();
                tr.col(|ui| {
                    ui.label(&row.name).on_hover_text(format!("research id {}", row.research_id));
                });
                tr.col(|ui| {
                    edit_cell(
                        ui,
                        &mut st.cell_buffers,
                        &["032", "H", "a", &idx, ResearchField::Level.key()],
                        &row.level,
                        format!("{} level", row.name),
                        row.max_level,
                        edits,
                    );
                });
                tr.col(|ui| {
                    let txt = row.max_level.map_or_else(|| "—".to_string(), |m| m.to_string());
                    ui.label(RichText::new(txt).color(style::TEXT_MUTED).size(11.0));
                });
                tr.col(|ui| {
                    if row.in_progress {
                        ui.label(RichText::new("yes").color(style::WARNING).size(11.0));
                    } else {
                        ui.label(RichText::new("—").color(style::TEXT_MUTED).size(11.0));
                    }
                });
            });
        });
}

fn skill_table(ui: &mut egui::Ui, rows: &[SkillRow]) {
    if rows.is_empty() {
        ui.label(RichText::new("No battle skills.").color(style::TEXT_MUTED));
        return;
    }
    TableBuilder::new(ui)
        .striped(true)
        .id_salt("adv_skills")
        .column(Column::initial(200.0)) // skill
        .column(Column::remainder()) // level
        .header(20.0, |mut h| {
            for title in ["Skill", "Level"] {
                h.col(|ui| {
                    ui.label(RichText::new(title).strong().size(12.0));
                });
            }
        })
        .body(|body| {
            body.rows(22.0, rows.len(), |mut tr| {
                let row = &rows[tr.index()];
                tr.col(|ui| {
                    ui.label(&row.name).on_hover_text(format!("skill id {}", row.skill_id));
                });
                tr.col(|ui| {
                    ui.label(RichText::new(&row.level).monospace().size(11.0));
                });
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use save_parser::container::encode_container;
    use save_parser::raw::Field;

    #[test]
    fn quality_label_letters() {
        assert_eq!(quality_label(8), "SSS");
        assert_eq!(quality_label(0), "F");
        // Unknown ids fall back to the number.
        assert_eq!(quality_label(99), "99");
    }

    fn sc(s: &str) -> Field {
        Field::Value(Raw::Scalar(s.into()))
    }
    fn b64(r: Raw) -> Field {
        Field::Value(Raw::Base64(Box::new(r)))
    }

    /// A `032` block (base64-wrapped like a real save) with research (`H.a`),
    /// class progression (`b.f`), and battle skills (`b.g`). Two entries per list
    /// so they don't collapse to a lone struct.
    fn adv_session() -> EditSession {
        let research = Raw::List(vec![
            // id 0 is the unused placeholder slot.
            Raw::Struct(vec![("a".into(), sc("0")), ("b".into(), sc("0")), ("f".into(), sc("0"))]),
            // id 1 (God HP) — uncapped; `f` is intentionally bogus to prove the
            // editor ignores the save's max field in favor of model::research_max.
            Raw::Struct(vec![
                ("a".into(), sc("1")),
                ("b".into(), sc("5")),
                ("f".into(), sc("10")),
                ("c".into(), sc("1")),
            ]),
            // id 7 (Core Quality) — capped at 800; `f` reads 0 (the bug we fixed).
            Raw::Struct(vec![
                ("a".into(), sc("7")),
                ("b".into(), sc("40")),
                ("f".into(), sc("0")),
                ("c".into(), sc("0")),
            ]),
        ]);
        let classes = Raw::List(vec![
            Raw::Struct(vec![("a".into(), sc("1")), ("b".into(), sc("12")), ("c".into(), sc("644"))]),
            Raw::Struct(vec![("a".into(), sc("4")), ("b".into(), sc("35")), ("c".into(), sc("11164"))]),
        ]);
        let skills = Raw::List(vec![
            Raw::Struct(vec![("a".into(), sc("1")), ("b".into(), sc("19"))]),
            Raw::Struct(vec![("a".into(), sc("11")), ("b".into(), sc("11"))]),
        ]);
        let b = Raw::Struct(vec![
            ("e".into(), sc("20")),
            ("f".into(), Field::Value(classes)),
            ("g".into(), Field::Value(skills)),
        ]);
        let professions = Raw::List(vec![
            Raw::Struct(vec![("a".into(), sc("1")), ("b".into(), sc("20")), ("c".into(), sc("11.89"))]),
            Raw::Struct(vec![("a".into(), sc("5")), ("b".into(), sc("14")), ("c".into(), sc("934.54"))]),
        ]);
        let h = Raw::Struct(vec![("a".into(), Field::Value(research))]);
        // Nested structs are base64-wrapped in real saves.
        let blk = Raw::Struct(vec![
            ("H".into(), b64(h)),
            ("b".into(), b64(b)),
            ("j".into(), Field::Value(professions)),
        ]);
        let root = Raw::Struct(vec![("032".into(), b64(blk))]);
        EditSession::load(&encode_container(&root.serialize(), "V2"), None).unwrap()
    }

    #[test]
    fn research_skips_placeholder_and_resolves_names() {
        let s = adv_session();
        let rows = read_research(&s);
        assert_eq!(rows.len(), 2, "id 0 placeholder should be filtered out");
        // God HP is uncapped — the save's bogus `f` (10) must be ignored.
        assert_eq!(rows[0].name, "God HP");
        assert_eq!(rows[0].max_level, None);
        assert!(rows[0].in_progress);
        // Core Quality is capped at 800 from model::research_max, even though the
        // save's `f` field reads 0 (the editing-blocked bug we fixed).
        assert_eq!(rows[1].name, "Core Quality");
        assert_eq!(rows[1].max_level, Some(800));
    }

    #[test]
    fn class_progression_reads_levels_and_names() {
        let s = adv_session();
        let rows = read_class_progression(&s);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "Newbie");
        assert_eq!(rows[0].level, "12");
        assert_eq!(rows[1].name, "Student");
        assert_eq!(rows[1].exp, "11164");
    }

    #[test]
    fn battle_skills_resolve_ids() {
        let s = adv_session();
        let rows = read_battle_skills(&s);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "Basic Attack");
        assert_eq!(rows[1].name, "Magic Arrow");
        assert_eq!(rows[1].level, "11");
    }

    #[test]
    fn unlock_plan_builds_full_prereq_chain_from_empty() {
        let existing = HashMap::new();
        let steps = unlock_plan(20, &existing).expect("Rogue has a known chain");
        // Rogue needs Newbie10 → Adventurer35 → Thief55 & Archer55 → Rogue1.
        let got: Vec<(u32, u32)> = steps.iter().map(|s| (s.class_id, s.level)).collect();
        assert_eq!(got, vec![(1, 10), (2, 35), (5, 55), (6, 55), (20, 1)]);
    }

    #[test]
    fn unlock_plan_skips_satisfied_prereqs_and_raises_low_ones() {
        let existing = HashMap::from([(1, 99), (2, 40), (5, 30)]);
        let steps = unlock_plan(20, &existing).unwrap();
        let got: Vec<(u32, u32)> = steps.iter().map(|s| (s.class_id, s.level)).collect();
        // Newbie(99≥10) and Adventurer(40≥35) skipped; Thief raised 30→55;
        // Archer created at 55; Rogue created at 1.
        assert_eq!(got, vec![(5, 55), (6, 55), (20, 1)]);
    }

    #[test]
    fn unlock_plan_unknown_class_is_none() {
        // Samurai (41) has no confirmed unlock chain.
        assert!(unlock_plan(41, &HashMap::new()).is_none());
    }

    #[test]
    fn professions_resolve_names_levels_and_decimal_exp() {
        let s = adv_session();
        let rows = read_professions(&s);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "Crafting");
        assert_eq!(rows[0].level, "20");
        assert_eq!(rows[1].name, "Alchemy");
        assert_eq!(rows[1].level, "14");
        assert_eq!(rows[1].exp, "934.54"); // decimal BigDouble preserved
    }
}
