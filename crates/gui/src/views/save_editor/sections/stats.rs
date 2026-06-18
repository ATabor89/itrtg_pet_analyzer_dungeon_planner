//! Generic "count list" editor for the simple root-level blocks that are just a
//! list of entries with one or two editable count fields: **Physical** (`h`,
//! level), **Skills** (`j`, level + usage count), **Monsters** (`k`, defeated).
//!
//! One config-driven section serves all three. Each is keyed by list position
//! (`a`), so edits stage `set_scalar([list_key, <i>, …field])` — a numeric index
//! path (these lists are fixed-size, never added to / deleted from, so indices
//! are stable).

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use save_parser::edit::apply_factor;
use save_parser::items;
use save_parser::raw::Raw;

use super::bulk::{self, OpKind};
use crate::style;
use crate::views::save_editor::session::EditSession;

/// One editable count field within an entry.
pub struct StatField {
    pub label: &'static str,
    /// Key path relative to the element (e.g. `["b"]` or `["e", "b"]`).
    pub sub: &'static [&'static str],
}

/// A block of same-shaped count entries.
pub struct StatConfig {
    pub title: &'static str,
    pub list_key: &'static str,
    pub name_fn: fn(u32) -> Option<&'static str>,
    pub fields: &'static [StatField],
}

pub const PHYSICAL: StatConfig = StatConfig {
    title: "Physical",
    list_key: "h",
    name_fn: items::physical_training_name,
    fields: &[StatField { label: "Level", sub: &["b"] }],
};

pub const SKILLS: StatConfig = StatConfig {
    title: "Skills",
    list_key: "j",
    name_fn: items::skill_name,
    fields: &[
        StatField { label: "Level", sub: &["b"] },
        StatField { label: "Usage", sub: &["e", "b"] },
    ],
};

pub const MONSTERS: StatConfig = StatConfig {
    title: "Monsters",
    list_key: "k",
    name_fn: items::monster_name,
    fields: &[StatField { label: "Defeated", sub: &["b"] }],
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortKey {
    Name,
    Field(usize),
}

#[derive(Default)]
pub struct StatEditState {
    f_name: String,
    selected: HashSet<usize>,
    /// Enabled bulk ops, keyed by field index.
    ops: HashMap<usize, (OpKind, String)>,
    /// Per-(row, field) overrides.
    overrides: HashMap<(usize, usize), String>,
    cell_buffers: HashMap<(usize, usize), String>,
    sort: Option<(SortKey, bool)>,
    apply_requested: bool,
    status: Option<(String, bool)>,
}

struct StatRow {
    index: usize,
    id: u32,
    name: Option<&'static str>,
    /// Raw value string per field (None if the field is absent on this entry).
    values: Vec<Option<String>>,
}

impl StatRow {
    fn display_name(&self) -> String {
        self.name.map(str::to_string).unwrap_or_else(|| format!("#{}", self.id))
    }
}

pub fn show(ui: &mut egui::Ui, session: &mut EditSession, st: &mut StatEditState, cfg: &StatConfig) {
    ui.heading(cfg.title);

    // These blocks always have many entries, so `h`/`j`/`k` parse as real lists;
    // index-path edits below rely on that (a lone struct would need a selector).
    let count = match session.root().get_path(&[cfg.list_key]) {
        Some(Raw::List(items)) => items.len(),
        _ => 0,
    };
    let rows: Vec<StatRow> = (0..count)
        .map(|index| {
            let is = index.to_string();
            let id = session
                .value(&[cfg.list_key, &is, "a"])
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(index as u32);
            let values = cfg
                .fields
                .iter()
                .map(|f| {
                    let mut path = vec![cfg.list_key, is.as_str()];
                    path.extend_from_slice(f.sub);
                    session.value(&path)
                })
                .collect();
            StatRow { index, id, name: (cfg.name_fn)(id), values }
        })
        .collect();

    let mut filtered: Vec<usize> =
        (0..rows.len()).filter(|&i| passes_filter(st, &rows[i])).collect();
    if let Some((key, asc)) = st.sort {
        filtered.sort_by(|&a, &b| {
            let o = cmp_rows(&rows[a], &rows[b], key);
            if asc { o } else { o.reverse() }
        });
    }

    filter_bar(ui, st, rows.len(), filtered.len());
    ui.separator();
    bulk_panel(ui, st, cfg, &filtered);
    ui.separator();

    if let Some((msg, err)) = &st.status {
        let color = if *err { style::ERROR } else { style::SUCCESS };
        ui.label(RichText::new(msg).color(color).size(12.0));
    }

    if st.apply_requested {
        st.status = Some(apply(session, st, cfg, &rows));
        st.apply_requested = false;
        st.selected.clear();
        st.overrides.clear();
        st.ops.clear();
        st.cell_buffers.clear();
    }

    table(ui, st, cfg, &rows, &filtered);
}

fn passes_filter(st: &StatEditState, r: &StatRow) -> bool {
    if st.f_name.trim().is_empty() {
        return true;
    }
    let q = st.f_name.trim().to_lowercase();
    r.name.is_some_and(|n| n.to_lowercase().contains(&q)) || r.id.to_string().contains(&q)
}

fn parse_u64(s: &str) -> Option<u64> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.parse().ok()).flatten()
}

fn parse_f64(s: &str) -> Option<f64> {
    let t = s.trim();
    (!t.is_empty()).then(|| itrtg_models::parse_flexible_number(t)).flatten()
}

fn cmp_rows(a: &StatRow, b: &StatRow, key: SortKey) -> Ordering {
    match key {
        SortKey::Name => a.display_name().to_lowercase().cmp(&b.display_name().to_lowercase()),
        SortKey::Field(fi) => field_u64(a, fi).cmp(&field_u64(b, fi)),
    }
}

fn field_u64(r: &StatRow, fi: usize) -> u64 {
    r.values.get(fi).and_then(|v| v.as_ref()).and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}

fn filter_bar(ui: &mut egui::Ui, st: &mut StatEditState, total: usize, shown: usize) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Filter:").color(style::TEXT_MUTED));
        ui.label("name / id");
        ui.add(egui::TextEdit::singleline(&mut st.f_name).desired_width(140.0));
        if ui.button("× clear").clicked() {
            st.f_name.clear();
        }
        ui.label(RichText::new(format!("{shown} / {total}")).color(style::TEXT_MUTED).size(11.0));
    });
}

fn bulk_panel(ui: &mut egui::Ui, st: &mut StatEditState, cfg: &StatConfig, filtered: &[usize]) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{} selected", st.selected.len())).color(style::TEXT_BRIGHT));
        if ui.button("Select all (filtered)").clicked() {
            st.selected.extend(filtered.iter().copied());
        }
        if ui.button("Clear selection").clicked() {
            st.selected.clear();
        }
    });

    egui::Grid::new("stat_bulk_ops").num_columns(3).spacing([10.0, 4.0]).show(ui, |ui| {
        for (fi, field) in cfg.fields.iter().enumerate() {
            let mut enabled = st.ops.contains_key(&fi);
            if ui.checkbox(&mut enabled, field.label).changed() {
                if enabled {
                    st.ops.insert(fi, (OpKind::Set, String::new()));
                } else {
                    st.ops.remove(&fi);
                }
            }
            if let Some((kind, value)) = st.ops.get_mut(&fi) {
                egui::ComboBox::from_id_salt(("stat_op_kind", cfg.list_key, fi))
                    .selected_text(bulk::op_label(*kind))
                    .width(70.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(kind, OpKind::Set, "Set");
                        ui.selectable_value(kind, OpKind::Add, "+ Add");
                        ui.selectable_value(kind, OpKind::Mul, "× Mul");
                    });
                ui.add(egui::TextEdit::singleline(value).desired_width(120.0));
            } else {
                ui.label("");
                ui.label("");
            }
            ui.end_row();
        }
    });

    let has_ops = !st.ops.is_empty() || !st.overrides.is_empty();
    if ui
        .add_enabled(
            !st.selected.is_empty() && has_ops,
            egui::Button::new(format!("Apply to {}", st.selected.len())),
        )
        .clicked()
    {
        st.apply_requested = true;
    }
}

fn bulk_target(st: &StatEditState, row: &StatRow, fi: usize) -> Option<String> {
    let (kind, value) = st.ops.get(&fi)?;
    let cur = row.values.get(fi)?.as_ref()?;
    match kind {
        OpKind::Set => Some(value.trim().to_string()),
        OpKind::Add => cur.trim().parse::<u64>().ok()?.checked_add(parse_u64(value)?).map(|v| v.to_string()),
        OpKind::Mul => apply_factor(cur, parse_f64(value)?).ok(),
    }
}

fn effective_target(st: &StatEditState, row: &StatRow, fi: usize) -> Option<String> {
    if let Some(v) = st.overrides.get(&(row.index, fi)) {
        return Some(v.clone());
    }
    bulk_target(st, row, fi)
}

fn table(ui: &mut egui::Ui, st: &mut StatEditState, cfg: &StatConfig, rows: &[StatRow], filtered: &[usize]) {
    let current_sort = st.sort;
    let mut sort_click: Option<SortKey> = None;
    let mut builder = TableBuilder::new(ui)
        .striped(true)
        .column(Column::auto()) // checkbox
        .column(Column::initial(200.0)) // name
        .column(Column::initial(54.0)); // id
    for _ in cfg.fields {
        builder = builder.column(Column::initial(140.0));
    }
    builder = builder.column(Column::remainder()); // trailing pad

    builder
        .header(20.0, |mut h| {
            h.col(|_| {});
            h.col(|ui| {
                if bulk::sort_header(ui, current_sort, "Name", SortKey::Name) {
                    sort_click = Some(SortKey::Name);
                }
            });
            h.col(|ui| {
                ui.label(RichText::new("Id").strong().size(12.0));
            });
            for (fi, field) in cfg.fields.iter().enumerate() {
                h.col(|ui| {
                    if bulk::sort_header(ui, current_sort, field.label, SortKey::Field(fi)) {
                        sort_click = Some(SortKey::Field(fi));
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
                    ui.label(RichText::new(row.display_name()).color(color));
                });
                tr.col(|ui| {
                    ui.label(RichText::new(row.id.to_string()).color(style::TEXT_MUTED).monospace().size(11.0));
                });
                for fi in 0..cfg.fields.len() {
                    tr.col(|ui| field_cell(ui, st, row, fi, selected));
                }
                tr.col(|_| {});
            });
        });

    if let Some(key) = sort_click {
        bulk::cycle_sort(&mut st.sort, key);
    }
}

fn field_cell(ui: &mut egui::Ui, st: &mut StatEditState, row: &StatRow, fi: usize, selected: bool) {
    let Some(current) = row.values.get(fi).and_then(|v| v.clone()) else {
        ui.label(RichText::new("—").color(style::TEXT_MUTED));
        return;
    };
    if !selected {
        ui.label(RichText::new(current).monospace().size(11.0));
        return;
    }
    let default = bulk_target(st, row, fi).unwrap_or_else(|| current.clone());
    bulk::override_cell(ui, (row.index, fi), &default, &current, &mut st.cell_buffers, &mut st.overrides);
}

fn apply(session: &mut EditSession, st: &StatEditState, cfg: &StatConfig, rows: &[StatRow]) -> (String, bool) {
    let by_index: HashMap<usize, &StatRow> = rows.iter().map(|r| (r.index, r)).collect();
    let mut selected: Vec<usize> = st.selected.iter().copied().collect();
    selected.sort_unstable();
    let mut staged = 0;
    let mut skipped = 0;
    for i in selected {
        let Some(row) = by_index.get(&i) else { continue };
        let is = i.to_string();
        for (fi, field) in cfg.fields.iter().enumerate() {
            let Some(target) = effective_target(st, row, fi) else { continue };
            let current = row.values.get(fi).and_then(|v| v.as_deref()).unwrap_or("");
            if target.trim() == current.trim() {
                continue;
            }
            if target.trim().parse::<u64>().is_err() {
                skipped += 1;
                continue;
            }
            let mut path = vec![cfg.list_key, is.as_str()];
            path.extend_from_slice(field.sub);
            let label = format!("{} {}", row.display_name(), field.label);
            if session.set_scalar(&path, label, target.trim()).is_ok() {
                staged += 1;
            } else {
                skipped += 1;
            }
        }
    }
    if skipped > 0 {
        (format!("Staged {staged} edits ({skipped} skipped)"), staged == 0)
    } else {
        (format!("Staged {staged} edits"), false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(level: &str, usage: Option<&str>) -> StatRow {
        StatRow {
            index: 0,
            id: 3,
            name: Some("Skill"),
            values: vec![Some(level.to_string()), usage.map(str::to_string)],
        }
    }

    #[test]
    fn bulk_ops_per_field() {
        let mut st = StatEditState::default();
        st.ops.insert(0, (OpKind::Add, "5".into())); // level
        st.ops.insert(1, (OpKind::Mul, "1000".into())); // usage
        let r = row("10", Some("2"));
        assert_eq!(bulk_target(&st, &r, 0).as_deref(), Some("15"));
        assert_eq!(bulk_target(&st, &r, 1).as_deref(), Some("2000"));
    }

    #[test]
    fn override_beats_bulk_and_absent_field() {
        let mut st = StatEditState::default();
        st.ops.insert(0, (OpKind::Set, "99".into()));
        st.overrides.insert((0, 0), "50".into());
        let r = row("10", None);
        assert_eq!(effective_target(&st, &r, 0).as_deref(), Some("50"));
        // Field 1 absent on this entry → no target.
        assert_eq!(bulk_target(&st, &r, 1), None);
    }
}
