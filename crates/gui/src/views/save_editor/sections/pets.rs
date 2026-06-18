//! Pets section: filter, multi-select, and **staged bulk edits** with per-pet
//! overrides.
//!
//! You filter the roster, select pets, configure bulk ops (× growth, set a
//! level, set a class), and optionally type a different value into an individual
//! pet's cell — that per-pet **override wins** over the bulk op for that field.
//! "Apply" stages the resulting changes into the session's pending log (paths
//! `X.b.<index>.…`); nothing is written to disk until you Save-As.
//!
//! Display reads the typed `derived().pets` (in `X.b` order, so the Vec index is
//! the raw index). To keep edits and reads from fighting over the session borrow,
//! a lightweight owned snapshot (`PetRow`) is built first, then the session is
//! free to be mutated on Apply.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use itrtg_models::{Class, Element};
use save_parser::edit::{apply_delta, apply_factor};

use super::bulk::{self, OpKind};
use super::equip_builder::{self, EquipBuilderState};
use crate::style;
use crate::views::save_editor::session::EditSession;

/// The bulk-editable numeric fields.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Field {
    Growth,
    Normal,
    Dungeon,
    ClassLvl,
}

impl Field {
    const ALL: [Field; 4] = [Field::Growth, Field::Normal, Field::Dungeon, Field::ClassLvl];

    fn label(self) -> &'static str {
        match self {
            Field::Growth => "Growth",
            Field::Normal => "Normal Lvl",
            Field::Dungeon => "Dungeon Lvl",
            Field::ClassLvl => "Class Lvl",
        }
    }

    /// Raw key path relative to `X.b.<i>`.
    fn keys(self) -> &'static [&'static str] {
        match self {
            Field::Growth => &["E"],
            Field::Normal => &["g"],
            Field::Dungeon => &["w", "b"],
            Field::ClassLvl => &["w", "d", "b"],
        }
    }

    /// Ops offered for this field (growth can multiply *or* add a flat amount).
    fn allowed_ops(self) -> &'static [OpKind] {
        match self {
            Field::Growth => &[OpKind::Set, OpKind::Mul, OpKind::Add],
            _ => &[OpKind::Set, OpKind::Add],
        }
    }
}

/// Sortable table columns.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SortCol {
    Name,
    Element,
    Class,
    Growth,
    Normal,
    Dungeon,
    ClassLvl,
}

/// Stable order for the element column (None sorts last).
fn element_order(e: Option<Element>) -> u8 {
    match e {
        Some(Element::Fire) => 0,
        Some(Element::Water) => 1,
        Some(Element::Wind) => 2,
        Some(Element::Earth) => 3,
        Some(Element::Neutral) => 4,
        Some(Element::All) => 5,
        None => 6,
    }
}

fn cmp_rows(a: &PetRow, b: &PetRow, col: SortCol) -> Ordering {
    match col {
        SortCol::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        SortCol::Element => element_order(a.element).cmp(&element_order(b.element)),
        SortCol::Class => a.class_id.cmp(&b.class_id),
        SortCol::Growth => a.growth.partial_cmp(&b.growth).unwrap_or(Ordering::Equal),
        SortCol::Normal => a.normal.cmp(&b.normal),
        SortCol::Dungeon => a.dungeon.cmp(&b.dungeon),
        SortCol::ClassLvl => a.class_lvl.cmp(&b.class_lvl),
    }
}

/// (label, save class id). Classless is id 0.
const CLASS_CHOICES: &[(&str, u32)] = &[
    ("Classless", 0),
    ("Blacksmith", 1),
    ("Alchemist", 2),
    ("Adventurer", 3),
    ("Defender", 4),
    ("Supporter", 5),
    ("Rogue", 6),
    ("Assassin", 7),
    ("Mage", 8),
];

const ELEMENTS: &[Element] = &[
    Element::Fire,
    Element::Water,
    Element::Wind,
    Element::Earth,
    Element::Neutral,
];

fn class_id(c: Class) -> u32 {
    match c {
        Class::Blacksmith => 1,
        Class::Alchemist => 2,
        Class::Adventurer => 3,
        Class::Defender => 4,
        Class::Supporter => 5,
        Class::Rogue => 6,
        Class::Assassin => 7,
        Class::Mage => 8,
        Class::Wildcard => 0,
    }
}

fn class_label(id: u32) -> &'static str {
    CLASS_CHOICES
        .iter()
        .find(|(_, i)| *i == id)
        .map_or("?", |(l, _)| *l)
}

fn element_label(e: Element) -> &'static str {
    match e {
        Element::Fire => "Fire",
        Element::Water => "Water",
        Element::Wind => "Wind",
        Element::Earth => "Earth",
        Element::Neutral => "Neutral",
        Element::All => "All",
    }
}

/// An owned, render-ready snapshot of one pet (so the session borrow is released
/// before Apply mutates it).
struct PetRow {
    index: usize,
    name: String,
    element: Option<Element>,
    class_id: u32,
    unlocked: bool,
    growth: f64,
    /// The exact stored growth (`E`) string — used for display, the multiply
    /// base, and the skip-compare, so none of them round-trip through f64.
    raw_growth: String,
    normal: u32,
    dungeon: u32,
    class_lvl: u32,
}

impl PetRow {
    fn current(&self, field: Field) -> String {
        match field {
            Field::Growth => self.raw_growth.clone(),
            Field::Normal => self.normal.to_string(),
            Field::Dungeon => self.dungeon.to_string(),
            Field::ClassLvl => self.class_lvl.to_string(),
        }
    }
}

#[derive(Default)]
pub struct PetEditState {
    // Filters.
    f_element: Option<Element>,
    f_class: Option<u32>,
    f_unlocked: Option<bool>,
    /// A pet has a class iff it is evolved (the only way to get one).
    f_evolved: Option<bool>,
    f_name: String,
    f_dungeon_min: String,
    f_dungeon_max: String,
    f_class_min: String,
    f_class_max: String,
    f_growth_min: String,
    f_growth_max: String,

    // Selection + staged batch.
    selected: HashSet<usize>,
    /// Enabled bulk ops, keyed by field: (op kind, value text).
    ops: HashMap<Field, (OpKind, String)>,
    /// Bulk "set class" (save class id), `None` = leave class alone.
    op_class: Option<u32>,
    /// Per-pet field overrides (explicit values that beat the bulk op).
    overrides: HashMap<(usize, Field), String>,
    /// Per-pet class overrides.
    class_overrides: HashMap<usize, u32>,
    /// In-progress text for editable override cells.
    cell_buffers: HashMap<(usize, Field), String>,

    /// Column sort: `(column, ascending)`; `None` = save order.
    sort: Option<(SortCol, bool)>,
    /// The shared equipment builder, used here in give-to-pets mode.
    builder: EquipBuilderState,

    /// Set by the Apply button, consumed next in `show`.
    apply_requested: bool,
    status: Option<(String, bool)>,
}

pub fn show(ui: &mut egui::Ui, session: &mut EditSession, st: &mut PetEditState) {
    ui.heading("Pets");

    // Build the owned snapshot, releasing the session borrow before any edits.
    let Some(save) = session.derived() else {
        ui.label(
            RichText::new("Typed pet data unavailable for this save — use the Raw Save Tree.")
                .color(style::TEXT_MUTED),
        );
        return;
    };
    let rows: Vec<PetRow> = (0..save.pets.len())
        .map(|index| {
            let p = &save.pets[index];
            // The exact stored growth string (not the round-tripped f64), so
            // display and the skip-compare match the real `E` value.
            let idx = index.to_string();
            let raw_growth = session
                .value(&["X", "b", &idx, "E"])
                .unwrap_or_else(|| format!("{}", p.growth));
            PetRow {
                index,
                name: p.name.clone(),
                element: p.element,
                class_id: p.class.map(class_id).unwrap_or(0),
                unlocked: p.unlocked,
                growth: p.growth,
                raw_growth,
                normal: p.normal_level,
                dungeon: p.dungeon_level,
                class_lvl: p.class_level,
            }
        })
        .collect();

    let mut filtered: Vec<usize> =
        (0..rows.len()).filter(|&i| passes_filter(st, &rows[i])).collect();
    if let Some((col, asc)) = st.sort {
        filtered.sort_by(|&a, &b| {
            let o = cmp_rows(&rows[a], &rows[b], col);
            if asc { o } else { o.reverse() }
        });
    }

    filter_bar(ui, st, rows.len(), filtered.len());
    ui.separator();
    bulk_panel(ui, st, &filtered);
    ui.separator();

    if let Some((msg, err)) = &st.status {
        let color = if *err { style::ERROR } else { style::SUCCESS };
        ui.label(RichText::new(msg).color(color).size(12.0));
    }

    // Apply (collect intended edits, then stage them on the session).
    if st.apply_requested {
        let staged = apply(session, st, &rows);
        st.status = Some(staged);
        st.apply_requested = false;
        st.selected.clear();
        st.overrides.clear();
        st.class_overrides.clear();
        st.ops.clear();
        st.op_class = None;
        st.cell_buffers.clear();
    }

    // Give equipment to the selected pets (one new instance each).
    if let Some(built) = equip_builder::builder_window(
        ui.ctx(),
        &mut st.builder,
        equip_builder::BuilderMode::GiveToPets { count: st.selected.len() },
    ) {
        st.status = Some(give_equipment(session, st, &rows, &built));
    }

    table(ui, st, &rows, &filtered);
}

fn passes_filter(st: &PetEditState, r: &PetRow) -> bool {
    if let Some(e) = st.f_element
        && r.element != Some(e)
    {
        return false;
    }
    if let Some(c) = st.f_class
        && r.class_id != c
    {
        return false;
    }
    if let Some(u) = st.f_unlocked
        && r.unlocked != u
    {
        return false;
    }
    if let Some(ev) = st.f_evolved
        && (r.class_id != 0) != ev
    {
        return false;
    }
    if !st.f_name.trim().is_empty()
        && !r.name.to_lowercase().contains(&st.f_name.trim().to_lowercase())
    {
        return false;
    }
    if !in_range(r.dungeon, &st.f_dungeon_min, &st.f_dungeon_max) {
        return false;
    }
    if !in_range(r.class_lvl, &st.f_class_min, &st.f_class_max) {
        return false;
    }
    if let Some(min) = parse_f64(&st.f_growth_min)
        && r.growth < min
    {
        return false;
    }
    if let Some(max) = parse_f64(&st.f_growth_max)
        && r.growth > max
    {
        return false;
    }
    true
}

fn in_range(v: u32, min: &str, max: &str) -> bool {
    if let Some(lo) = parse_u64(min)
        && (v as u64) < lo
    {
        return false;
    }
    if let Some(hi) = parse_u64(max)
        && (v as u64) > hi
    {
        return false;
    }
    true
}

fn parse_u64(s: &str) -> Option<u64> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.parse().ok()).flatten()
}

fn parse_f64(s: &str) -> Option<f64> {
    let t = s.trim();
    (!t.is_empty()).then(|| itrtg_models::parse_flexible_number(t)).flatten()
}

fn filter_bar(ui: &mut egui::Ui, st: &mut PetEditState, total: usize, shown: usize) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Filter:").color(style::TEXT_MUTED));
        // Element.
        egui::ComboBox::from_id_salt("pet_f_element")
            .selected_text(st.f_element.map_or("Any element", element_label))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut st.f_element, None, "Any element");
                for &e in ELEMENTS {
                    ui.selectable_value(&mut st.f_element, Some(e), element_label(e));
                }
            });
        // Class.
        egui::ComboBox::from_id_salt("pet_f_class")
            .selected_text(st.f_class.map_or("Any class", class_label))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut st.f_class, None, "Any class");
                for &(label, id) in CLASS_CHOICES {
                    ui.selectable_value(&mut st.f_class, Some(id), label);
                }
            });
        // Unlocked.
        egui::ComboBox::from_id_salt("pet_f_unlocked")
            .selected_text(match st.f_unlocked {
                None => "Any",
                Some(true) => "Unlocked",
                Some(false) => "Locked",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut st.f_unlocked, None, "Any");
                ui.selectable_value(&mut st.f_unlocked, Some(true), "Unlocked");
                ui.selectable_value(&mut st.f_unlocked, Some(false), "Locked");
            });
        // Evolved (a pet has a class iff it's evolved).
        egui::ComboBox::from_id_salt("pet_f_evolved")
            .selected_text(match st.f_evolved {
                None => "Any",
                Some(true) => "Evolved",
                Some(false) => "Not evolved",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut st.f_evolved, None, "Any");
                ui.selectable_value(&mut st.f_evolved, Some(true), "Evolved");
                ui.selectable_value(&mut st.f_evolved, Some(false), "Not evolved");
            });
        ui.label("name");
        ui.add(egui::TextEdit::singleline(&mut st.f_name).desired_width(90.0));
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Ranges:").color(style::TEXT_MUTED));
        range_input(ui, "Dungeon", &mut st.f_dungeon_min, &mut st.f_dungeon_max);
        range_input(ui, "Class", &mut st.f_class_min, &mut st.f_class_max);
        ui.label("Growth ≥");
        ui.add(egui::TextEdit::singleline(&mut st.f_growth_min).desired_width(70.0));
        ui.label("≤");
        ui.add(egui::TextEdit::singleline(&mut st.f_growth_max).desired_width(70.0));
        if ui.button("× clear").clicked() {
            let selected = std::mem::take(&mut st.selected);
            *st = PetEditState::default();
            st.selected = selected;
        }
        ui.label(
            RichText::new(format!("{shown} / {total} pets"))
                .color(style::TEXT_MUTED)
                .size(11.0),
        );
    });
}

fn range_input(ui: &mut egui::Ui, label: &str, min: &mut String, max: &mut String) {
    ui.label(format!("{label} min"));
    ui.add(egui::TextEdit::singleline(min).desired_width(48.0));
    ui.label("max");
    ui.add(egui::TextEdit::singleline(max).desired_width(48.0));
}

fn bulk_panel(ui: &mut egui::Ui, st: &mut PetEditState, filtered: &[usize]) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{} selected", st.selected.len()))
                .color(style::TEXT_BRIGHT),
        );
        if ui.button("Select all (filtered)").clicked() {
            st.selected.extend(filtered.iter().copied());
        }
        if ui.button("Clear selection").clicked() {
            st.selected.clear();
        }
    });

    ui.label(
        RichText::new(
            "Bulk ops apply to selected pets; a value you type into a pet's own cell wins.",
        )
        .color(style::TEXT_MUTED)
        .size(11.0),
    );

    egui::Grid::new("pet_bulk_ops")
        .num_columns(3)
        .spacing([10.0, 4.0])
        .show(ui, |ui| {
            for field in Field::ALL {
                let mut enabled = st.ops.contains_key(&field);
                if ui.checkbox(&mut enabled, field.label()).changed() {
                    if enabled {
                        st.ops.insert(field, (OpKind::Set, String::new()));
                    } else {
                        st.ops.remove(&field);
                    }
                }
                if let Some((kind, value)) = st.ops.get_mut(&field) {
                    egui::ComboBox::from_id_salt(("pet_op_kind", field.label()))
                        .selected_text(bulk::op_label(*kind))
                        .width(70.0)
                        .show_ui(ui, |ui| {
                            for &k in field.allowed_ops() {
                                ui.selectable_value(kind, k, bulk::op_label(k));
                            }
                        });
                    ui.add(egui::TextEdit::singleline(value).desired_width(110.0));
                } else {
                    ui.label("");
                    ui.label("");
                }
                ui.end_row();
            }

            // Class op.
            let mut class_enabled = st.op_class.is_some();
            if ui.checkbox(&mut class_enabled, "Class").changed() {
                st.op_class = class_enabled.then_some(0);
            }
            if let Some(id) = &mut st.op_class {
                egui::ComboBox::from_id_salt("pet_op_class")
                    .selected_text(class_label(*id))
                    .show_ui(ui, |ui| {
                        for &(label, cid) in CLASS_CHOICES {
                            ui.selectable_value(id, cid, label);
                        }
                    });
                ui.label("");
            } else {
                ui.label("");
                ui.label("");
            }
            ui.end_row();
        });

    let has_ops = !st.ops.is_empty() || st.op_class.is_some() || !st.overrides.is_empty()
        || !st.class_overrides.is_empty();
    let n = st.selected.len();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                !st.selected.is_empty() && has_ops,
                egui::Button::new(format!("Apply to {n} pets")),
            )
            .clicked()
        {
            st.apply_requested = true;
        }
        if ui
            .add_enabled(
                !st.selected.is_empty(),
                egui::Button::new(format!("Give equipment to {n}…")),
            )
            .on_hover_text("Create one new item per selected pet and equip it")
            .clicked()
        {
            st.builder.open();
        }
    });
}

/// Give one new equipment instance to each selected pet (equipped in the chosen
/// slot). Returns a status message.
fn give_equipment(
    session: &mut EditSession,
    st: &PetEditState,
    rows: &[PetRow],
    built: &equip_builder::BuiltEquip,
) -> (String, bool) {
    let Some(slot) = built.slot_key else {
        return ("No slot chosen".into(), true);
    };
    let names: HashMap<usize, &str> = rows.iter().map(|r| (r.index, r.name.as_str())).collect();
    let eq_name = save_parser::items::equipment_type_name(built.type_id).unwrap_or("Equipment");
    let qual = save_parser::items::quality_name(built.quality).unwrap_or("");
    let plus = if built.plus > 0 { format!("+{}", built.plus) } else { String::new() };

    let mut selected: Vec<usize> = st.selected.iter().copied().collect();
    selected.sort_unstable();
    let mut given = 0;
    for i in selected {
        let pet = names.get(&i).copied().unwrap_or("pet");
        let label = format!("{eq_name} {qual}{plus} → {pet}");
        if session
            .add_equipment(
                built.type_id,
                built.plus,
                built.quality,
                built.gem_level,
                built.gem_element,
                label,
                Some((i, slot)),
            )
            .is_ok()
        {
            given += 1;
        }
    }
    (format!("Gave {eq_name} to {given} pets — see Pending changes"), false)
}

/// The bulk-op result for a field (no per-pet override), or `None` if no op is
/// configured. Returns the new raw value string.
fn bulk_target(st: &PetEditState, row: &PetRow, field: Field) -> Option<String> {
    let (kind, value) = st.ops.get(&field)?;
    match (kind, field) {
        (OpKind::Set, _) => Some(value.trim().to_string()),
        // Mul applies only to growth.
        (OpKind::Mul, _) => apply_factor(&row.raw_growth, parse_f64(value)?).ok(),
        // Growth + flat amount (fractional ok); levels + integer (overflow-safe).
        (OpKind::Add, Field::Growth) => apply_delta(&row.raw_growth, parse_f64(value)?).ok(),
        (OpKind::Add, _) => {
            let add = parse_u64(value)?;
            let cur = row.current(field).parse::<u64>().ok()?;
            cur.checked_add(add).map(|v| v.to_string())
        }
    }
}

/// The effective new value for a field: a per-pet override beats the bulk op.
fn effective_target(st: &PetEditState, row: &PetRow, field: Field) -> Option<String> {
    if let Some(v) = st.overrides.get(&(row.index, field)) {
        return Some(v.clone());
    }
    bulk_target(st, row, field)
}

fn table(ui: &mut egui::Ui, st: &mut PetEditState, rows: &[PetRow], filtered: &[usize]) {
    // The header reads the sort by value and reports a click into a local, so it
    // doesn't borrow `st` (the body needs `&mut st`).
    let current_sort = st.sort;
    let mut sort_click: Option<SortCol> = None;
    TableBuilder::new(ui)
        .striped(true)
        .column(Column::auto()) // checkbox
        .column(Column::initial(150.0)) // name
        .column(Column::initial(64.0)) // element
        .column(Column::initial(90.0)) // class
        .column(Column::initial(160.0)) // growth
        .column(Column::initial(120.0)) // normal
        .column(Column::initial(120.0)) // dungeon
        .column(Column::remainder()) // class lvl
        .header(20.0, |mut h| {
            h.col(|_| {}); // checkbox column
            let cols = [
                ("Name", SortCol::Name),
                ("Elem", SortCol::Element),
                ("Class", SortCol::Class),
                ("Growth", SortCol::Growth),
                ("Normal", SortCol::Normal),
                ("Dungeon", SortCol::Dungeon),
                ("Class Lvl", SortCol::ClassLvl),
            ];
            for (title, col) in cols {
                h.col(|ui| {
                    if bulk::sort_header(ui, current_sort, title, col) {
                        sort_click = Some(col);
                    }
                });
            }
        })
        .body(|body| {
            body.rows(22.0, filtered.len(), |mut tr| {
                let row = &rows[filtered[tr.index()]];
                let selected = st.selected.contains(&row.index);

                tr.col(|ui| {
                    let mut on = selected;
                    if ui.checkbox(&mut on, "").changed() {
                        if on {
                            st.selected.insert(row.index);
                        } else {
                            st.selected.remove(&row.index);
                        }
                    }
                });
                tr.col(|ui| {
                    let color = if row.unlocked { style::TEXT_NORMAL } else { style::TEXT_MUTED };
                    ui.label(RichText::new(&row.name).color(color));
                });
                tr.col(|ui| {
                    ui.label(row.element.map_or("—", element_label));
                });
                tr.col(|ui| {
                    if selected {
                        class_cell(ui, st, row);
                    } else {
                        ui.label(class_label(row.class_id));
                    }
                });
                tr.col(|ui| field_cell(ui, st, row, Field::Growth, selected));
                tr.col(|ui| field_cell(ui, st, row, Field::Normal, selected));
                tr.col(|ui| field_cell(ui, st, row, Field::Dungeon, selected));
                tr.col(|ui| field_cell(ui, st, row, Field::ClassLvl, selected));
            });
        });

    if let Some(col) = sort_click {
        bulk::cycle_sort(&mut st.sort, col);
    }
}

/// A field cell. Read-only current value for unselected rows; an editable
/// override box (defaulting to the bulk-op result) for selected rows.
fn field_cell(ui: &mut egui::Ui, st: &mut PetEditState, row: &PetRow, field: Field, selected: bool) {
    let current = row.current(field);
    if !selected {
        ui.label(RichText::new(current).monospace().size(11.0));
        return;
    }
    // The displayed default is the bulk-op result, or the current value if no op.
    let default = bulk_target(st, row, field).unwrap_or_else(|| current.clone());
    bulk::override_cell(
        ui,
        (row.index, field),
        &default,
        &current,
        &mut st.cell_buffers,
        &mut st.overrides,
    );
}

fn class_cell(ui: &mut egui::Ui, st: &mut PetEditState, row: &PetRow) {
    let default = st.op_class.unwrap_or(row.class_id);
    let mut id = st.class_overrides.get(&row.index).copied().unwrap_or(default);
    let before = id;
    egui::ComboBox::from_id_salt(("pet_class_cell", row.index))
        .selected_text(class_label(id))
        .width(84.0)
        .show_ui(ui, |ui| {
            for &(label, cid) in CLASS_CHOICES {
                ui.selectable_value(&mut id, cid, label);
            }
        });
    if id != before {
        if id == default {
            st.class_overrides.remove(&row.index);
        } else {
            st.class_overrides.insert(row.index, id);
        }
    }
}

/// Stage all configured changes for the selected pets. Returns a status message.
fn apply(session: &mut EditSession, st: &PetEditState, rows: &[PetRow]) -> (String, bool) {
    let by_index: HashMap<usize, &PetRow> = rows.iter().map(|r| (r.index, r)).collect();
    let mut staged = 0usize;
    let mut skipped = 0usize;

    let mut selected: Vec<usize> = st.selected.iter().copied().collect();
    selected.sort_unstable();

    for i in selected {
        let Some(row) = by_index.get(&i) else { continue };
        let i_str = i.to_string();

        for field in Field::ALL {
            let Some(target) = effective_target(st, row, field) else { continue };
            if target.trim() == row.current(field).trim() {
                continue;
            }
            if !valid_value(field, &target) {
                skipped += 1;
                continue;
            }
            let mut path = vec!["X", "b", i_str.as_str()];
            path.extend_from_slice(field.keys());
            if session.set_scalar(&path, format!("{} · {}", row.name, field.label()), target.trim()).is_ok() {
                staged += 1;
            } else {
                skipped += 1;
            }
        }

        // Class.
        let class_target = st.class_overrides.get(&i).copied().or(st.op_class);
        if let Some(cid) = class_target
            && cid != row.class_id
        {
            let id_s = cid.to_string();
            if session
                .set_scalar(&["X", "b", &i_str, "w", "d", "a"], format!("{} · Class", row.name), &id_s)
                .is_ok()
            {
                staged += 1;
            } else {
                skipped += 1;
            }
        }
    }

    let msg = if skipped > 0 {
        format!("Staged {staged} edits ({skipped} skipped — invalid or unreachable)")
    } else {
        format!("Staged {staged} edits across selected pets")
    };
    (msg, skipped > 0 && staged == 0)
}

/// Validate a value before staging: growth accepts any number, levels integers.
fn valid_value(field: Field, value: &str) -> bool {
    match field {
        Field::Growth => parse_f64(value).is_some(),
        _ => parse_u64(value).is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> PetRow {
        PetRow {
            index: 3,
            name: "Robot".into(),
            element: Some(Element::Fire),
            class_id: 2,
            unlocked: true,
            growth: 100.0,
            raw_growth: "100".into(),
            normal: 10,
            dungeon: 20,
            class_lvl: 5,
        }
    }

    #[test]
    fn bulk_multiply_uses_apply_factor() {
        let mut st = PetEditState::default();
        st.ops.insert(Field::Growth, (OpKind::Mul, "3".into()));
        assert_eq!(bulk_target(&st, &row(), Field::Growth).as_deref(), Some("300"));
    }

    #[test]
    fn bulk_add_levels() {
        let mut st = PetEditState::default();
        st.ops.insert(Field::Dungeon, (OpKind::Add, "5".into()));
        assert_eq!(bulk_target(&st, &row(), Field::Dungeon).as_deref(), Some("25"));
    }

    #[test]
    fn bulk_add_growth_flat() {
        let mut st = PetEditState::default();
        st.ops.insert(Field::Growth, (OpKind::Add, "50".into()));
        // raw_growth "100" + 50 → "150" (integer-preserving via apply_delta).
        assert_eq!(bulk_target(&st, &row(), Field::Growth).as_deref(), Some("150"));
    }

    #[test]
    fn override_beats_bulk_op() {
        let mut st = PetEditState::default();
        st.ops.insert(Field::Growth, (OpKind::Mul, "3".into()));
        st.overrides.insert((3, Field::Growth), "777".into());
        assert_eq!(effective_target(&st, &row(), Field::Growth).as_deref(), Some("777"));
    }

    #[test]
    fn set_op_and_no_op() {
        let mut st = PetEditState::default();
        assert_eq!(bulk_target(&st, &row(), Field::Normal), None);
        st.ops.insert(Field::Normal, (OpKind::Set, "50".into()));
        assert_eq!(bulk_target(&st, &row(), Field::Normal).as_deref(), Some("50"));
    }

    #[test]
    fn class_id_round_trips() {
        assert_eq!(class_id(Class::Mage), 8);
        assert_eq!(class_label(8), "Mage");
        assert_eq!(class_label(0), "Classless");
    }
}
