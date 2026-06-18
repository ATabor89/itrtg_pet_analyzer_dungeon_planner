//! Gems section: view/filter the gem inventory (`X.002`) and edit counts. A gem
//! stack is keyed by (element, level) — Fire L1 is a different stack from Fire
//! L10. Nearly a mirror of the Inventory (materials) section.
//!
//! Note: gems embedded in equipment are NOT in this list; this is the loose-gem
//! inventory only. Count edits use the numeric index path `X.002.<i>.c`.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use save_parser::edit::apply_factor;

use super::bulk::{self, OpKind};
use crate::style;
use crate::views::save_editor::session::EditSession;

/// (label, element id). Save element ids: 0=Neutral … 4=Wind.
const ELEMENTS: &[(&str, u32)] = &[
    ("Neutral", 0),
    ("Fire", 1),
    ("Water", 2),
    ("Earth", 3),
    ("Wind", 4),
];

fn element_name(id: u32) -> String {
    ELEMENTS.iter().find(|(_, i)| *i == id).map_or_else(|| format!("Element {id}"), |(l, _)| (*l).to_string())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GSort {
    Element,
    Level,
    Count,
}

struct GemRow {
    index: usize,
    element_id: u32,
    level: u32,
    count: u64,
    raw_count: String,
}

#[derive(Default)]
struct AddGemState {
    open: bool,
    element: u32,
    level: u32,
    count: String,
}

#[derive(Default)]
pub struct GemEditState {
    f_element: Option<u32>,
    f_level_min: String,
    f_level_max: String,
    f_count_min: String,

    selected: HashSet<usize>,
    op_count: Option<(OpKind, String)>,
    overrides: HashMap<usize, String>,
    cell_buffers: HashMap<usize, String>,
    sort: Option<(GSort, bool)>,
    add: AddGemState,

    apply_requested: bool,
    status: Option<(String, bool)>,
}

pub fn show(ui: &mut egui::Ui, session: &mut EditSession, st: &mut GemEditState) {
    ui.horizontal(|ui| {
        ui.heading("Gems");
        if ui.button("➕ Add gem").clicked() {
            st.add = AddGemState { open: true, element: 1, level: 1, count: "1".into() };
        }
    });

    let Some(save) = session.derived() else {
        ui.label(RichText::new("Typed gem data unavailable — use the Raw Save Tree.").color(style::TEXT_MUTED));
        return;
    };
    let rows: Vec<GemRow> = (0..save.gems.len())
        .map(|index| {
            let g = &save.gems[index];
            let idx = index.to_string();
            let raw_count = session.value(&["X", "002", &idx, "c"]).unwrap_or_else(|| g.count.to_string());
            GemRow { index, element_id: g.element_id, level: g.level, count: g.count, raw_count }
        })
        .collect();

    let mut filtered: Vec<usize> = (0..rows.len()).filter(|&i| passes_filter(st, &rows[i])).collect();
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
        st.op_count = None;
        st.cell_buffers.clear();
    }

    if let Some((element, level, count)) = add_gem_window(ui.ctx(), &mut st.add) {
        let label = format!("{} L{level}", element_name(element));
        st.status = Some(match session.set_gem(element, level, count.trim(), label.clone()) {
            Ok(true) => (format!("Added {label}"), false),
            Ok(false) => (format!("Updated {label} (already owned)"), false),
            Err(e) => (format!("Add failed: {e}"), true),
        });
    }

    table(ui, st, &rows, &filtered);
}

fn passes_filter(st: &GemEditState, r: &GemRow) -> bool {
    if let Some(e) = st.f_element
        && r.element_id != e
    {
        return false;
    }
    if let Some(lo) = parse_u64(&st.f_level_min)
        && (r.level as u64) < lo
    {
        return false;
    }
    if let Some(hi) = parse_u64(&st.f_level_max)
        && (r.level as u64) > hi
    {
        return false;
    }
    if let Some(min) = parse_u64(&st.f_count_min)
        && r.count < min
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

fn cmp_rows(a: &GemRow, b: &GemRow, col: GSort) -> Ordering {
    match col {
        GSort::Element => a.element_id.cmp(&b.element_id).then(a.level.cmp(&b.level)),
        GSort::Level => a.level.cmp(&b.level),
        GSort::Count => a.count.cmp(&b.count),
    }
}

fn filter_bar(ui: &mut egui::Ui, st: &mut GemEditState, total: usize, shown: usize) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Filter:").color(style::TEXT_MUTED));
        egui::ComboBox::from_id_salt("gem_f_element")
            .selected_text(st.f_element.map_or_else(|| "Any element".to_string(), element_name))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut st.f_element, None, "Any element");
                for &(label, id) in ELEMENTS {
                    ui.selectable_value(&mut st.f_element, Some(id), label);
                }
            });
        ui.label("Level");
        ui.add(egui::TextEdit::singleline(&mut st.f_level_min).desired_width(36.0));
        ui.label("–");
        ui.add(egui::TextEdit::singleline(&mut st.f_level_max).desired_width(36.0));
        ui.label("count ≥");
        ui.add(egui::TextEdit::singleline(&mut st.f_count_min).desired_width(70.0));
        if ui.button("× clear").clicked() {
            let selected = std::mem::take(&mut st.selected);
            *st = GemEditState::default();
            st.selected = selected;
        }
        ui.label(RichText::new(format!("{shown} / {total} stacks")).color(style::TEXT_MUTED).size(11.0));
    });
}

fn bulk_panel(ui: &mut egui::Ui, st: &mut GemEditState, filtered: &[usize]) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{} selected", st.selected.len())).color(style::TEXT_BRIGHT));
        if ui.button("Select all (filtered)").clicked() {
            st.selected.extend(filtered.iter().copied());
        }
        if ui.button("Clear selection").clicked() {
            st.selected.clear();
        }
    });
    ui.horizontal(|ui| {
        let mut enabled = st.op_count.is_some();
        if ui.checkbox(&mut enabled, "Count").changed() {
            if enabled {
                st.op_count = Some((OpKind::Set, String::new()));
            } else {
                st.op_count = None;
            }
        }
        if let Some((kind, value)) = &mut st.op_count {
            egui::ComboBox::from_id_salt("gem_op_kind")
                .selected_text(bulk::op_label(*kind))
                .width(70.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(kind, OpKind::Set, "Set");
                    ui.selectable_value(kind, OpKind::Add, "+ Add");
                    ui.selectable_value(kind, OpKind::Mul, "× Mul");
                });
            ui.add(egui::TextEdit::singleline(value).desired_width(120.0));
        }
        let has_ops = st.op_count.is_some() || !st.overrides.is_empty();
        if ui
            .add_enabled(!st.selected.is_empty() && has_ops, egui::Button::new(format!("Apply to {}", st.selected.len())))
            .clicked()
        {
            st.apply_requested = true;
        }
    });
}

fn bulk_target(st: &GemEditState, row: &GemRow) -> Option<String> {
    let (kind, value) = st.op_count.as_ref()?;
    match kind {
        OpKind::Set => Some(value.trim().to_string()),
        OpKind::Add => row.count.checked_add(parse_u64(value)?).map(|v| v.to_string()),
        OpKind::Mul => apply_factor(&row.raw_count, parse_f64(value)?).ok(),
    }
}

fn effective_target(st: &GemEditState, row: &GemRow) -> Option<String> {
    if let Some(v) = st.overrides.get(&row.index) {
        return Some(v.clone());
    }
    bulk_target(st, row)
}

fn table(ui: &mut egui::Ui, st: &mut GemEditState, rows: &[GemRow], filtered: &[usize]) {
    let current_sort = st.sort;
    let mut sort_click: Option<GSort> = None;
    TableBuilder::new(ui)
        .striped(true)
        .column(Column::auto())
        .column(Column::initial(120.0)) // element
        .column(Column::initial(80.0)) // level
        .column(Column::remainder()) // count
        .header(20.0, |mut h| {
            h.col(|_| {});
            for (title, col) in [("Element", GSort::Element), ("Level", GSort::Level), ("Count", GSort::Count)] {
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
                    ui.label(element_name(row.element_id));
                });
                tr.col(|ui| {
                    ui.label(RichText::new(row.level.to_string()).monospace());
                });
                tr.col(|ui| {
                    if selected {
                        let default = bulk_target(st, row).unwrap_or_else(|| row.raw_count.clone());
                        bulk::override_cell(ui, row.index, &default, &row.raw_count, &mut st.cell_buffers, &mut st.overrides);
                    } else {
                        ui.label(RichText::new(&row.raw_count).monospace().size(11.0));
                    }
                });
            });
        });

    if let Some(col) = sort_click {
        bulk::cycle_sort(&mut st.sort, col);
    }
}

fn apply(session: &mut EditSession, st: &GemEditState, rows: &[GemRow]) -> (String, bool) {
    let by_index: HashMap<usize, &GemRow> = rows.iter().map(|r| (r.index, r)).collect();
    let mut selected: Vec<usize> = st.selected.iter().copied().collect();
    selected.sort_unstable();
    let mut staged = 0;
    let mut skipped = 0;
    for i in selected {
        let Some(row) = by_index.get(&i) else { continue };
        let Some(target) = effective_target(st, row) else { continue };
        if target.trim() == row.raw_count.trim() {
            continue;
        }
        if target.trim().parse::<u64>().is_err() {
            skipped += 1;
            continue;
        }
        // Route through set_gem (upsert by element+level): it normalizes a
        // lone-struct X.002 to a list, so the numeric-index edit path is valid
        // even when the player has a single gem stack.
        let label = format!("{} L{} count", element_name(row.element_id), row.level);
        if session.set_gem(row.element_id, row.level, target.trim(), label).is_ok() {
            staged += 1;
        } else {
            skipped += 1;
        }
    }
    if skipped > 0 {
        (format!("Staged {staged} edits ({skipped} skipped)"), staged == 0)
    } else {
        (format!("Staged {staged} count edits"), false)
    }
}

/// The add-gem modal. Returns `Some((element, level, count))` on Add.
fn add_gem_window(ctx: &egui::Context, st: &mut AddGemState) -> Option<(u32, u32, String)> {
    if !st.open {
        return None;
    }
    let mut result = None;
    let mut close = false;
    let mut window_open = true;
    egui::Window::new("Add Gem")
        .collapsible(false)
        .resizable(false)
        .open(&mut window_open)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Element:");
                egui::ComboBox::from_id_salt("gem_add_elem")
                    .selected_text(element_name(st.element))
                    .show_ui(ui, |ui| {
                        for &(label, id) in ELEMENTS {
                            ui.selectable_value(&mut st.element, id, label);
                        }
                    });
                ui.label("Level:");
                ui.add(egui::DragValue::new(&mut st.level).range(0..=20));
                ui.label("Count:");
                ui.add(egui::TextEdit::singleline(&mut st.count).desired_width(120.0));
            });
            ui.separator();
            ui.horizontal(|ui| {
                let ok = st.count.trim().parse::<u64>().is_ok();
                if ui.add_enabled(ok, egui::Button::new("Add")).clicked() {
                    result = Some((st.element, st.level, st.count.trim().to_string()));
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
