//! Equipment section: filter, multi-select, and staged bulk edits over the owned
//! equipment list (`X.R`). Mirrors the Pets section. Edits write
//! `X.R.<index>.<key>` into the session pending log; creating *new* equipment is
//! Phase 2b (the equipment builder).

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use itrtg_models::Element;
use save_parser::items;

use super::bulk::{self, OpKind};
use crate::style;
use crate::views::save_editor::session::EditSession;

/// Bulk-editable numeric equipment fields.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum EField {
    Quality,
    Plus,
    GemLevel,
}

impl EField {
    const ALL: [EField; 3] = [EField::Quality, EField::Plus, EField::GemLevel];

    fn label(self) -> &'static str {
        match self {
            EField::Quality => "Quality (0–8)",
            EField::Plus => "Plus",
            EField::GemLevel => "Gem Level",
        }
    }

    /// Raw key under `X.R.<i>`.
    fn key(self) -> &'static str {
        match self {
            EField::Quality => "c",
            EField::Plus => "b",
            EField::GemLevel => "f",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ESort {
    Type,
    Quality,
    Plus,
    GemLevel,
    Equipped,
}

/// (label, element id). Save element ids: 0=Neutral … 4=Wind.
const ELEMENT_CHOICES: &[(&str, u32)] = &[
    ("Neutral", 0),
    ("Fire", 1),
    ("Water", 2),
    ("Earth", 3),
    ("Wind", 4),
];

fn element_name(id: u32) -> &'static str {
    ELEMENT_CHOICES
        .iter()
        .find(|(_, i)| *i == id)
        .map_or("?", |(l, _)| *l)
}

fn element_id(e: Element) -> u32 {
    match e {
        Element::Neutral => 0,
        Element::Fire => 1,
        Element::Water => 2,
        Element::Earth => 3,
        Element::Wind => 4,
        Element::All => 0,
    }
}

/// Owned, render-ready snapshot of one equipment instance.
struct EquipRow {
    index: usize,
    name: String,
    quality: u32,
    plus: u32,
    gem_level: u32,
    gem_element_id: u32,
    /// Unique mirror id (`h`) — the one pet slots reference.
    mirror_id: u32,
    equipped_on: Option<(String, &'static str)>,
}

impl EquipRow {
    fn current(&self, field: EField) -> String {
        match field {
            EField::Quality => self.quality.to_string(),
            EField::Plus => self.plus.to_string(),
            EField::GemLevel => self.gem_level.to_string(),
        }
    }
}

#[derive(Default)]
pub struct EquipEditState {
    f_name: String,
    f_quality_min: String,
    f_quality_max: String,
    f_plus_min: String,
    f_plus_max: String,
    f_gem: Option<bool>,
    f_gem_element: Option<u32>,
    f_equipped: Option<bool>,

    selected: HashSet<usize>,
    ops: HashMap<EField, (OpKind, String)>,
    /// Bulk "set gem element" (element id), `None` = leave alone.
    op_gem_element: Option<u32>,
    overrides: HashMap<(usize, EField), String>,
    gem_overrides: HashMap<usize, u32>,
    cell_buffers: HashMap<(usize, EField), String>,
    sort: Option<(ESort, bool)>,

    apply_requested: bool,
    status: Option<(String, bool)>,
}

pub fn show(ui: &mut egui::Ui, session: &mut EditSession, st: &mut EquipEditState) {
    ui.heading("Equipment");

    let Some(save) = session.derived() else {
        ui.label(
            RichText::new("Typed equipment data unavailable — use the Raw Save Tree.")
                .color(style::TEXT_MUTED),
        );
        return;
    };

    // Reverse map: a pet slot id (== the item's unique mirror id `h`) → who has it.
    let mut equipped: HashMap<u32, (String, &'static str)> = HashMap::new();
    for p in &save.pets {
        for (id, slot) in [
            (p.weapon_id, "weapon"),
            (p.armor_id, "armor"),
            (p.accessory_id, "accessory"),
        ] {
            if let Some(id) = id {
                equipped.entry(id).or_insert((p.name.clone(), slot));
            }
        }
    }

    let rows: Vec<EquipRow> = (0..save.equipment.len())
        .map(|index| {
            let e = &save.equipment[index];
            let idx = index.to_string();
            let mirror_id = session
                .value(&["X", "R", &idx, "h"])
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(e.instance_id);
            EquipRow {
                index,
                name: items::equipment_type_name(e.type_id)
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("Type {}", e.type_id)),
                quality: e.quality,
                plus: e.plus,
                gem_level: e.gem_level,
                gem_element_id: e.gem_element.map(element_id).unwrap_or(0),
                mirror_id,
                equipped_on: equipped.get(&mirror_id).cloned(),
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

    if st.apply_requested {
        st.status = Some(apply(session, st, &rows));
        st.apply_requested = false;
        st.selected.clear();
        st.overrides.clear();
        st.gem_overrides.clear();
        st.ops.clear();
        st.op_gem_element = None;
        st.cell_buffers.clear();
    }

    table(ui, st, &rows, &filtered);
}

fn passes_filter(st: &EquipEditState, r: &EquipRow) -> bool {
    if !st.f_name.trim().is_empty()
        && !r.name.to_lowercase().contains(&st.f_name.trim().to_lowercase())
    {
        return false;
    }
    if !in_range(r.quality, &st.f_quality_min, &st.f_quality_max) {
        return false;
    }
    if !in_range(r.plus, &st.f_plus_min, &st.f_plus_max) {
        return false;
    }
    if let Some(gem) = st.f_gem
        && (r.gem_level > 0) != gem
    {
        return false;
    }
    if let Some(eid) = st.f_gem_element
        && (r.gem_level == 0 || r.gem_element_id != eid)
    {
        return false;
    }
    if let Some(eq) = st.f_equipped
        && r.equipped_on.is_some() != eq
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

fn cmp_rows(a: &EquipRow, b: &EquipRow, col: ESort) -> Ordering {
    match col {
        ESort::Type => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        ESort::Quality => a.quality.cmp(&b.quality),
        ESort::Plus => a.plus.cmp(&b.plus),
        ESort::GemLevel => a.gem_level.cmp(&b.gem_level),
        ESort::Equipped => a.equipped_on.is_some().cmp(&b.equipped_on.is_some()),
    }
}

fn filter_bar(ui: &mut egui::Ui, st: &mut EquipEditState, total: usize, shown: usize) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Filter:").color(style::TEXT_MUTED));
        ui.label("type");
        ui.add(egui::TextEdit::singleline(&mut st.f_name).desired_width(120.0));
        ui.label("Quality");
        ui.add(egui::TextEdit::singleline(&mut st.f_quality_min).desired_width(36.0));
        ui.label("–");
        ui.add(egui::TextEdit::singleline(&mut st.f_quality_max).desired_width(36.0));
        ui.label("Plus");
        ui.add(egui::TextEdit::singleline(&mut st.f_plus_min).desired_width(36.0));
        ui.label("–");
        ui.add(egui::TextEdit::singleline(&mut st.f_plus_max).desired_width(36.0));
    });
    ui.horizontal_wrapped(|ui| {
        egui::ComboBox::from_id_salt("eq_f_gem")
            .selected_text(match st.f_gem {
                None => "Any gem",
                Some(true) => "Gemmed",
                Some(false) => "No gem",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut st.f_gem, None, "Any gem");
                ui.selectable_value(&mut st.f_gem, Some(true), "Gemmed");
                ui.selectable_value(&mut st.f_gem, Some(false), "No gem");
            });
        egui::ComboBox::from_id_salt("eq_f_gem_elem")
            .selected_text(st.f_gem_element.map_or("Any element", element_name))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut st.f_gem_element, None, "Any element");
                for &(label, id) in ELEMENT_CHOICES {
                    ui.selectable_value(&mut st.f_gem_element, Some(id), label);
                }
            });
        egui::ComboBox::from_id_salt("eq_f_equipped")
            .selected_text(match st.f_equipped {
                None => "Any",
                Some(true) => "Equipped",
                Some(false) => "Unequipped",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut st.f_equipped, None, "Any");
                ui.selectable_value(&mut st.f_equipped, Some(true), "Equipped");
                ui.selectable_value(&mut st.f_equipped, Some(false), "Unequipped");
            });
        if ui.button("× clear").clicked() {
            let selected = std::mem::take(&mut st.selected);
            *st = EquipEditState::default();
            st.selected = selected;
        }
        ui.label(
            RichText::new(format!("{shown} / {total} items"))
                .color(style::TEXT_MUTED)
                .size(11.0),
        );
    });
}

fn bulk_panel(ui: &mut egui::Ui, st: &mut EquipEditState, filtered: &[usize]) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{} selected", st.selected.len())).color(style::TEXT_BRIGHT));
        if ui.button("Select all (filtered)").clicked() {
            st.selected.extend(filtered.iter().copied());
        }
        if ui.button("Clear selection").clicked() {
            st.selected.clear();
        }
    });
    ui.label(
        RichText::new("Quality 0–8 = F E D C B A S SS SSS. A typed cell value wins over the bulk op.")
            .color(style::TEXT_MUTED)
            .size(11.0),
    );

    egui::Grid::new("eq_bulk_ops")
        .num_columns(3)
        .spacing([10.0, 4.0])
        .show(ui, |ui| {
            for field in EField::ALL {
                let mut enabled = st.ops.contains_key(&field);
                if ui.checkbox(&mut enabled, field.label()).changed() {
                    if enabled {
                        st.ops.insert(field, (OpKind::Set, String::new()));
                    } else {
                        st.ops.remove(&field);
                    }
                }
                if let Some((kind, value)) = st.ops.get_mut(&field) {
                    egui::ComboBox::from_id_salt(("eq_op_kind", field.label()))
                        .selected_text(bulk::op_label(*kind))
                        .width(70.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(kind, OpKind::Set, "Set");
                            ui.selectable_value(kind, OpKind::Add, "+ Add");
                        });
                    ui.add(egui::TextEdit::singleline(value).desired_width(110.0));
                } else {
                    ui.label("");
                    ui.label("");
                }
                ui.end_row();
            }

            // Gem element (choice op).
            let mut enabled = st.op_gem_element.is_some();
            if ui.checkbox(&mut enabled, "Gem Element").changed() {
                st.op_gem_element = enabled.then_some(1);
            }
            if let Some(id) = &mut st.op_gem_element {
                egui::ComboBox::from_id_salt("eq_op_gem_elem")
                    .selected_text(element_name(*id))
                    .show_ui(ui, |ui| {
                        for &(label, eid) in ELEMENT_CHOICES {
                            ui.selectable_value(id, eid, label);
                        }
                    });
                ui.label("");
            } else {
                ui.label("");
                ui.label("");
            }
            ui.end_row();
        });

    let has_ops = !st.ops.is_empty()
        || st.op_gem_element.is_some()
        || !st.overrides.is_empty()
        || !st.gem_overrides.is_empty();
    if ui
        .add_enabled(
            !st.selected.is_empty() && has_ops,
            egui::Button::new(format!("Apply to {} items", st.selected.len())),
        )
        .clicked()
    {
        st.apply_requested = true;
    }
}

/// The bulk-op result for a numeric field, or `None` if no op is configured.
fn bulk_target(st: &EquipEditState, row: &EquipRow, field: EField) -> Option<String> {
    let (kind, value) = st.ops.get(&field)?;
    let cur = row.current(field).parse::<u64>().ok()?;
    let new = match kind {
        OpKind::Set => parse_u64(value)?,
        OpKind::Add => cur.checked_add(parse_u64(value)?)?,
        OpKind::Mul => return None,
    };
    // Quality is bounded 0–8.
    let new = if field == EField::Quality { new.min(8) } else { new };
    Some(new.to_string())
}

fn effective_target(st: &EquipEditState, row: &EquipRow, field: EField) -> Option<String> {
    if let Some(v) = st.overrides.get(&(row.index, field)) {
        return Some(v.clone());
    }
    bulk_target(st, row, field)
}

fn table(ui: &mut egui::Ui, st: &mut EquipEditState, rows: &[EquipRow], filtered: &[usize]) {
    let current_sort = st.sort;
    let mut sort_click: Option<ESort> = None;
    TableBuilder::new(ui)
        .striped(true)
        .column(Column::auto()) // checkbox
        .column(Column::initial(170.0)) // type
        .column(Column::initial(120.0)) // quality
        .column(Column::initial(110.0)) // plus
        .column(Column::initial(150.0)) // gem
        .column(Column::initial(150.0)) // equipped
        .column(Column::remainder()) // instance id
        .header(20.0, |mut h| {
            h.col(|_| {});
            let cols = [
                ("Type", ESort::Type),
                ("Quality", ESort::Quality),
                ("Plus", ESort::Plus),
                ("Gem", ESort::GemLevel),
                ("Equipped", ESort::Equipped),
            ];
            for (title, col) in cols {
                h.col(|ui| {
                    if bulk::sort_header(ui, current_sort, title, col) {
                        sort_click = Some(col);
                    }
                });
            }
            h.col(|ui| {
                ui.label(RichText::new("Inst").strong().size(12.0));
            });
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
                    ui.label(&row.name);
                });
                tr.col(|ui| field_cell(ui, st, row, EField::Quality, selected, true));
                tr.col(|ui| field_cell(ui, st, row, EField::Plus, selected, false));
                tr.col(|ui| gem_cell(ui, st, row, selected));
                tr.col(|ui| {
                    match &row.equipped_on {
                        Some((pet, slot)) => ui.label(
                            RichText::new(format!("{pet} ({slot})")).color(style::TEXT_NORMAL),
                        ),
                        None => ui.label(RichText::new("—").color(style::TEXT_MUTED)),
                    };
                });
                tr.col(|ui| {
                    ui.label(RichText::new(row.mirror_id.to_string()).color(style::TEXT_MUTED).size(11.0));
                });
            });
        });

    if let Some(col) = sort_click {
        bulk::cycle_sort(&mut st.sort, col);
    }
}

/// A numeric field cell: read-only current for unselected rows (quality shows the
/// letter); an editable override box for selected rows.
fn field_cell(
    ui: &mut egui::Ui,
    st: &mut EquipEditState,
    row: &EquipRow,
    field: EField,
    selected: bool,
    quality: bool,
) {
    let current = row.current(field);
    if !selected {
        let text = if quality {
            items::quality_name(row.quality).unwrap_or(&current).to_string()
        } else {
            current.clone()
        };
        ui.label(RichText::new(text).monospace().size(11.0));
        return;
    }
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

/// The gem column: shows level + element; selected rows get an element dropdown
/// override (paired with the gem-level field cell to its own column? gem level is
/// edited via the Gem Level field; here we edit element).
fn gem_cell(ui: &mut egui::Ui, st: &mut EquipEditState, row: &EquipRow, selected: bool) {
    ui.horizontal(|ui| {
        // Gem level via the numeric field cell.
        field_cell(ui, st, row, EField::GemLevel, selected, false);
        if !selected {
            if row.gem_level > 0 {
                ui.label(
                    RichText::new(element_name(row.gem_element_id))
                        .color(style::TEXT_MUTED)
                        .size(11.0),
                );
            }
            return;
        }
        // Element override dropdown.
        let default = st.op_gem_element.unwrap_or(row.gem_element_id);
        let mut id = st.gem_overrides.get(&row.index).copied().unwrap_or(default);
        let before = id;
        egui::ComboBox::from_id_salt(("eq_gem_cell", row.index))
            .selected_text(element_name(id))
            .width(76.0)
            .show_ui(ui, |ui| {
                for &(label, eid) in ELEMENT_CHOICES {
                    ui.selectable_value(&mut id, eid, label);
                }
            });
        if id != before {
            if id == default {
                st.gem_overrides.remove(&row.index);
            } else {
                st.gem_overrides.insert(row.index, id);
            }
        }
    });
}

/// Stage all configured changes for the selected items.
fn apply(session: &mut EditSession, st: &EquipEditState, rows: &[EquipRow]) -> (String, bool) {
    let by_index: HashMap<usize, &EquipRow> = rows.iter().map(|r| (r.index, r)).collect();
    let mut staged = 0usize;
    let mut skipped = 0usize;

    let mut selected: Vec<usize> = st.selected.iter().copied().collect();
    selected.sort_unstable();

    for i in selected {
        let Some(row) = by_index.get(&i) else { continue };
        let i_str = i.to_string();

        for field in EField::ALL {
            let Some(target) = effective_target(st, row, field) else { continue };
            if target.trim() == row.current(field).trim() {
                continue;
            }
            if target.trim().parse::<u64>().is_err() {
                skipped += 1;
                continue;
            }
            if session
                .set_scalar(
                    &["X", "R", &i_str, field.key()],
                    format!("{} · {}", row.name, field.label()),
                    target.trim(),
                )
                .is_ok()
            {
                staged += 1;
            } else {
                skipped += 1;
            }
        }

        // Gem element.
        let elem = st.gem_overrides.get(&i).copied().or(st.op_gem_element);
        if let Some(eid) = elem
            && eid != row.gem_element_id
            && session
                .set_scalar(&["X", "R", &i_str, "g"], format!("{} · Gem Element", row.name), &eid.to_string())
                .is_ok()
        {
            staged += 1;
        }
    }

    let msg = if skipped > 0 {
        format!("Staged {staged} edits ({skipped} skipped — invalid or unreachable)")
    } else {
        format!("Staged {staged} edits across selected items")
    };
    (msg, skipped > 0 && staged == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> EquipRow {
        EquipRow {
            index: 2,
            name: "Magic Stick".into(),
            quality: 6,
            plus: 10,
            gem_level: 0,
            gem_element_id: 0,
            mirror_id: 858,
            equipped_on: None,
        }
    }

    #[test]
    fn bulk_set_and_add() {
        let mut st = EquipEditState::default();
        st.ops.insert(EField::Plus, (OpKind::Add, "5".into()));
        assert_eq!(bulk_target(&st, &row(), EField::Plus).as_deref(), Some("15"));
        st.ops.insert(EField::Plus, (OpKind::Set, "20".into()));
        assert_eq!(bulk_target(&st, &row(), EField::Plus).as_deref(), Some("20"));
    }

    #[test]
    fn quality_clamps_to_8() {
        let mut st = EquipEditState::default();
        st.ops.insert(EField::Quality, (OpKind::Add, "5".into()));
        // 6 + 5 = 11 → clamped to 8 (SSS).
        assert_eq!(bulk_target(&st, &row(), EField::Quality).as_deref(), Some("8"));
        st.ops.insert(EField::Quality, (OpKind::Set, "12".into()));
        assert_eq!(bulk_target(&st, &row(), EField::Quality).as_deref(), Some("8"));
    }

    #[test]
    fn override_beats_bulk() {
        let mut st = EquipEditState::default();
        st.ops.insert(EField::Plus, (OpKind::Set, "20".into()));
        st.overrides.insert((2, EField::Plus), "13".into());
        assert_eq!(effective_target(&st, &row(), EField::Plus).as_deref(), Some("13"));
    }
}
