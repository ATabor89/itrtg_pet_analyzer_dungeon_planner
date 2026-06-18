//! Inventory section: view/filter the material inventory (`X.Q`), bulk-edit
//! quantities, and add items (known or arbitrary id). Mirrors the Equipment
//! page, but with a single editable field (quantity).
//!
//! Quantity edits stage `set_scalar(["X","Q","a=<id>","b"], …)` (by item id, so
//! the numeric index doesn't matter); "Add item" upserts via
//! `session.set_material` (edit the stack if owned, else create it).

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use save_parser::edit::apply_factor;
use save_parser::items;

use super::bulk::{self, OpKind};
use crate::style;
use crate::views::save_editor::session::EditSession;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ISort {
    Item,
    Id,
    Quantity,
}

/// Owned, render-ready snapshot of one material stack.
struct InvRow {
    index: usize,
    item_id: u32,
    name: Option<&'static str>,
    count: u64,
    /// Exact stored count string (`b`), for ×Mul and the skip-compare.
    raw_count: String,
}

#[derive(Default)]
struct AddItemState {
    open: bool,
    id_text: String,
    search: String,
    qty: String,
}

#[derive(Default)]
pub struct InventoryEditState {
    f_name: String,
    f_known: Option<bool>,
    f_count_min: String,

    selected: HashSet<usize>,
    op_qty: Option<(OpKind, String)>,
    overrides: HashMap<usize, String>,
    cell_buffers: HashMap<usize, String>,
    sort: Option<(ISort, bool)>,
    add: AddItemState,
    pending_delete: Option<usize>,

    apply_requested: bool,
    status: Option<(String, bool)>,
}

pub fn show(ui: &mut egui::Ui, session: &mut EditSession, st: &mut InventoryEditState) {
    ui.horizontal(|ui| {
        ui.heading("Inventory");
        if ui.button("➕ Add item").clicked() {
            st.add = AddItemState { open: true, qty: "1".into(), ..AddItemState::default() };
        }
    });

    let Some(save) = session.derived() else {
        ui.label(
            RichText::new("Typed inventory data unavailable — use the Raw Save Tree.")
                .color(style::TEXT_MUTED),
        );
        return;
    };
    let rows: Vec<InvRow> = (0..save.materials.len())
        .map(|index| {
            let m = &save.materials[index];
            let idx = index.to_string();
            let raw_count = session
                .value(&["X", "Q", &idx, "b"])
                .unwrap_or_else(|| m.count.to_string());
            InvRow {
                index,
                item_id: m.item_id,
                name: items::material_name(m.item_id),
                count: m.count,
                raw_count,
            }
        })
        .collect();
    let owned: HashMap<u32, u64> = rows.iter().map(|r| (r.item_id, r.count)).collect();

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
        st.op_qty = None;
        st.cell_buffers.clear();
    }

    // Add-item modal (upsert).
    if let Some((id, qty)) = add_item_window(ui.ctx(), &mut st.add, &owned) {
        let label = items::material_name(id).map(str::to_string).unwrap_or_else(|| format!("Item {id}"));
        st.status = Some(match session.set_material(id, qty.trim(), label.clone()) {
            Ok(true) => (format!("Added {label} (id {id})"), false),
            Ok(false) => (format!("Updated {label} (id {id}) — already owned"), false),
            Err(e) => (format!("Add failed: {e}"), true),
        });
    }

    table(ui, st, &rows, &filtered);

    if let Some(idx) = st.pending_delete.take()
        && let Some(row) = rows.iter().find(|r| r.index == idx)
    {
        let label = row_name(row);
        if let Err(e) = session.delete_material(idx, label) {
            st.status = Some((format!("Delete failed: {e}"), true));
        }
    }
}

fn passes_filter(st: &InventoryEditState, r: &InvRow) -> bool {
    if !st.f_name.trim().is_empty() {
        let q = st.f_name.trim().to_lowercase();
        let name_hit = r.name.is_some_and(|n| n.to_lowercase().contains(&q));
        let id_hit = r.item_id.to_string().contains(&q);
        if !name_hit && !id_hit {
            return false;
        }
    }
    if let Some(known) = st.f_known
        && r.name.is_some() != known
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

fn cmp_rows(a: &InvRow, b: &InvRow, col: ISort) -> Ordering {
    match col {
        ISort::Item => row_name(a).to_lowercase().cmp(&row_name(b).to_lowercase()),
        ISort::Id => a.item_id.cmp(&b.item_id),
        ISort::Quantity => a.count.cmp(&b.count),
    }
}

fn row_name(r: &InvRow) -> String {
    r.name.map(str::to_string).unwrap_or_else(|| format!("Item {}", r.item_id))
}

fn filter_bar(ui: &mut egui::Ui, st: &mut InventoryEditState, total: usize, shown: usize) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Filter:").color(style::TEXT_MUTED));
        ui.label("name / id");
        ui.add(egui::TextEdit::singleline(&mut st.f_name).desired_width(120.0));
        egui::ComboBox::from_id_salt("inv_f_known")
            .selected_text(match st.f_known {
                None => "Any",
                Some(true) => "Named",
                Some(false) => "Unknown id",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut st.f_known, None, "Any");
                ui.selectable_value(&mut st.f_known, Some(true), "Named");
                ui.selectable_value(&mut st.f_known, Some(false), "Unknown id");
            });
        ui.label("count ≥");
        ui.add(egui::TextEdit::singleline(&mut st.f_count_min).desired_width(70.0));
        if ui.button("× clear").clicked() {
            let selected = std::mem::take(&mut st.selected);
            *st = InventoryEditState::default();
            st.selected = selected;
        }
        ui.label(
            RichText::new(format!("{shown} / {total} items"))
                .color(style::TEXT_MUTED)
                .size(11.0),
        );
    });
}

fn bulk_panel(ui: &mut egui::Ui, st: &mut InventoryEditState, filtered: &[usize]) {
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
        let mut enabled = st.op_qty.is_some();
        if ui.checkbox(&mut enabled, "Quantity").changed() {
            if enabled {
                st.op_qty = Some((OpKind::Set, String::new()));
            } else {
                st.op_qty = None;
            }
        }
        if let Some((kind, value)) = &mut st.op_qty {
            egui::ComboBox::from_id_salt("inv_op_kind")
                .selected_text(bulk::op_label(*kind))
                .width(70.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(kind, OpKind::Set, "Set");
                    ui.selectable_value(kind, OpKind::Add, "+ Add");
                    ui.selectable_value(kind, OpKind::Mul, "× Mul");
                });
            ui.add(egui::TextEdit::singleline(value).desired_width(120.0));
        }
        let has_ops = st.op_qty.is_some() || !st.overrides.is_empty();
        if ui
            .add_enabled(
                !st.selected.is_empty() && has_ops,
                egui::Button::new(format!("Apply to {}", st.selected.len())),
            )
            .clicked()
        {
            st.apply_requested = true;
        }
    });
}

/// The bulk-op result for a row, or `None` if no op configured.
fn bulk_target(st: &InventoryEditState, row: &InvRow) -> Option<String> {
    let (kind, value) = st.op_qty.as_ref()?;
    match kind {
        OpKind::Set => Some(value.trim().to_string()),
        OpKind::Add => row.count.checked_add(parse_u64(value)?).map(|v| v.to_string()),
        OpKind::Mul => apply_factor(&row.raw_count, parse_f64(value)?).ok(),
    }
}

fn effective_target(st: &InventoryEditState, row: &InvRow) -> Option<String> {
    if let Some(v) = st.overrides.get(&row.index) {
        return Some(v.clone());
    }
    bulk_target(st, row)
}

fn table(ui: &mut egui::Ui, st: &mut InventoryEditState, rows: &[InvRow], filtered: &[usize]) {
    let current_sort = st.sort;
    let mut sort_click: Option<ISort> = None;
    TableBuilder::new(ui)
        .striped(true)
        .column(Column::auto()) // checkbox
        .column(Column::initial(220.0)) // item
        .column(Column::initial(70.0)) // id
        .column(Column::initial(140.0)) // quantity
        .column(Column::remainder()) // delete
        .header(20.0, |mut h| {
            h.col(|_| {});
            for (title, col) in [("Item", ISort::Item), ("Id", ISort::Id), ("Quantity", ISort::Quantity)] {
                h.col(|ui| {
                    if bulk::sort_header(ui, current_sort, title, col) {
                        sort_click = Some(col);
                    }
                });
            }
            h.col(|_| {});
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
                    let color = if row.name.is_some() { style::TEXT_NORMAL } else { style::WARNING };
                    ui.label(RichText::new(row_name(row)).color(color));
                });
                tr.col(|ui| {
                    ui.label(RichText::new(row.item_id.to_string()).color(style::TEXT_MUTED).monospace().size(11.0));
                });
                tr.col(|ui| {
                    if selected {
                        let default = bulk_target(st, row).unwrap_or_else(|| row.raw_count.clone());
                        bulk::override_cell(ui, row.index, &default, &row.raw_count, &mut st.cell_buffers, &mut st.overrides);
                    } else {
                        ui.label(RichText::new(&row.raw_count).monospace().size(11.0));
                    }
                });
                tr.col(|ui| {
                    if ui.small_button("×").on_hover_text("Delete this stack").clicked() {
                        st.pending_delete = Some(row.index);
                    }
                });
            });
        });

    if let Some(col) = sort_click {
        bulk::cycle_sort(&mut st.sort, col);
    }
}

/// Stage all quantity changes for the selected rows.
fn apply(session: &mut EditSession, st: &InventoryEditState, rows: &[InvRow]) -> (String, bool) {
    let by_index: HashMap<usize, &InvRow> = rows.iter().map(|r| (r.index, r)).collect();
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
        // Edit by item id (selector path) so a lone-struct X.Q still works.
        let sel = format!("a={}", row.item_id);
        let label = row_name(row);
        if session.set_scalar(&["X", "Q", &sel, "b"], label, target.trim()).is_ok() {
            staged += 1;
        } else {
            skipped += 1;
        }
    }
    if skipped > 0 {
        (format!("Staged {staged} edits ({skipped} skipped)"), staged == 0)
    } else {
        (format!("Staged {staged} quantity edits"), false)
    }
}

/// The add-item modal. Returns `Some((item_id, quantity))` on Add.
fn add_item_window(
    ctx: &egui::Context,
    st: &mut AddItemState,
    owned: &HashMap<u32, u64>,
) -> Option<(u32, String)> {
    if !st.open {
        return None;
    }
    let mut result = None;
    let mut close = false;
    let mut window_open = true;

    egui::Window::new("Add Item")
        .collapsible(false)
        .resizable(false)
        .open(&mut window_open)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Known item:");
                egui::ComboBox::from_id_salt("inv_add_known")
                    .selected_text("pick…")
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut st.search)
                                .hint_text("search")
                                .desired_width(200.0),
                        );
                        let q = st.search.trim().to_lowercase();
                        for (id, name) in items::known_materials() {
                            if !q.is_empty() && !name.to_lowercase().contains(&q) {
                                continue;
                            }
                            let label = match owned.get(&id) {
                                Some(c) => format!("{name}  (id {id}, owned {c})"),
                                None => format!("{name}  (id {id})"),
                            };
                            if ui.selectable_label(false, label).clicked() {
                                st.id_text = id.to_string();
                            }
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Item id:");
                ui.add(egui::TextEdit::singleline(&mut st.id_text).desired_width(80.0));
                ui.label("Quantity:");
                ui.add(egui::TextEdit::singleline(&mut st.qty).desired_width(120.0));
            });
            // Resolved name + owned hint.
            if let Ok(id) = st.id_text.trim().parse::<u32>() {
                let name = items::material_name(id).unwrap_or("unknown id");
                let owned_txt = owned.get(&id).map(|c| format!(" — already own {c}")).unwrap_or_default();
                ui.label(RichText::new(format!("→ {name}{owned_txt}")).color(style::TEXT_MUTED).size(11.0));
            }

            ui.separator();
            ui.horizontal(|ui| {
                let id_ok = st.id_text.trim().parse::<u32>().is_ok();
                let qty_ok = st.qty.trim().parse::<u64>().is_ok();
                if ui.add_enabled(id_ok && qty_ok, egui::Button::new("Add")).clicked()
                    && let Ok(id) = st.id_text.trim().parse::<u32>()
                {
                    result = Some((id, st.qty.trim().to_string()));
                    close = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
                if !id_ok {
                    ui.label(RichText::new("enter a numeric id").color(style::TEXT_MUTED).size(11.0));
                }
            });
        });

    st.open = window_open && !close;
    result
}
