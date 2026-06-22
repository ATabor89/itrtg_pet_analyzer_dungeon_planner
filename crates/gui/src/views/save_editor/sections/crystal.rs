//! Crystal Factory section: the single struct at `X.w` (`GKJLJMJLMIB`).
//!
//! Edits the factory's Crystal Power (`X.w.a`) and each module's level
//! (`X.w.b.<i>.b`, one per crystal grade). The production counters (`c`/`d`) and
//! the modules' cost/timer/value BigDoubles are shown read-only — their exact
//! gameplay roles aren't pinned down, so they stay in the Raw Save Tree.
//!
//! No typed model for this struct, so values read straight from the raw tree via
//! the `CrystalFactoryField` / `CrystalModuleField` descriptors; edits stage by
//! raw path / list index.

use std::collections::HashMap;

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use save_parser::items;
use save_parser::labels::{CrystalFactoryField, CrystalModuleField};
use save_parser::raw::Raw;

use crate::style;
use crate::views::save_editor::session::EditSession;

/// One module row (`X.w.b.<index>`).
struct ModuleRow {
    index: usize,
    grade_id: u32,
    name: String,
    level: String,
    cost: String,
    value: String,
}

#[derive(Default)]
pub struct CrystalEditState {
    /// Per-cell buffers keyed by the field's dotted raw path.
    buffers: HashMap<String, String>,
    status: Option<(String, bool)>,
}

fn read_modules(session: &EditSession) -> Vec<ModuleRow> {
    let Some(Raw::List(list)) = session.root().get_path(&["X", "w", "b"]) else {
        return Vec::new();
    };
    (0..list.len())
        .map(|index| {
            let i = index.to_string();
            let at = |k: &str| session.value(&["X", "w", "b", &i, k]).unwrap_or_default();
            let grade_id: u32 = at(CrystalModuleField::Grade.key()).trim().parse().unwrap_or(0);
            ModuleRow {
                index,
                grade_id,
                name: items::crystal_module_name(grade_id)
                    .map_or_else(|| format!("Grade {grade_id}"), str::to_string),
                level: at(CrystalModuleField::Level.key()),
                cost: at(CrystalModuleField::CostC.key()),
                value: at(CrystalModuleField::ValueF.key()),
            }
        })
        .collect()
}

pub fn show(ui: &mut egui::Ui, session: &mut EditSession, st: &mut CrystalEditState) {
    ui.heading("Crystal Factory");

    if session.root().get_path(&["X", "w"]).is_none() {
        ui.label(RichText::new("No Crystal Factory data in this save.").color(style::TEXT_MUTED));
        return;
    }

    if let Some((msg, err)) = &st.status {
        let color = if *err { style::ERROR } else { style::SUCCESS };
        ui.label(RichText::new(msg).color(color).size(12.0));
    }

    let modules = read_modules(session);
    let mut edits: Vec<(Vec<String>, String, String)> = Vec::new();

    // --- Factory scalars ---
    egui::Grid::new("crystal_scalars").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Crystal Power");
        let cur = session.value(&["X", "w", CrystalFactoryField::CrystalPower.key()]).unwrap_or_default();
        big_cell(ui, &mut st.buffers, &["X", "w", "a"], &cur, "Crystal Power".into(), &mut edits);
        ui.end_row();
        for (label, key) in [("Accumulated", "c"), ("Progress", "d")] {
            ui.label(label);
            let v = session.value(&["X", "w", key]).unwrap_or_else(|| "—".into());
            ui.label(RichText::new(v).color(style::TEXT_MUTED).monospace().size(11.0));
            ui.end_row();
        }
    });
    ui.separator();

    // --- Modules ---
    ui.label(RichText::new("Modules").strong());
    if modules.is_empty() {
        ui.label(RichText::new("No modules.").color(style::TEXT_MUTED));
    } else {
        module_table(ui, st, &modules, &mut edits);
    }

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
        st.status = Some(("Staged Crystal Factory edit".to_string(), false));
    }
}

fn module_table(
    ui: &mut egui::Ui,
    st: &mut CrystalEditState,
    rows: &[ModuleRow],
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    TableBuilder::new(ui)
        .striped(true)
        .id_salt("crystal_modules")
        .column(Column::initial(140.0)) // grade
        .column(Column::initial(100.0)) // level
        .column(Column::initial(140.0)) // cost
        .column(Column::remainder()) // value
        .header(20.0, |mut h| {
            for title in ["Grade", "Level", "Cost", "Value"] {
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
                    ui.label(&row.name).on_hover_text(format!("grade id {}", row.grade_id));
                });
                tr.col(|ui| {
                    let p = ["X", "w", "b", &idx, "b"];
                    uint_cell(ui, &mut st.buffers, &p, &row.level, format!("{} level", row.name), edits);
                });
                tr.col(|ui| {
                    ui.label(RichText::new(&row.cost).color(style::TEXT_MUTED).monospace().size(11.0));
                });
                tr.col(|ui| {
                    ui.label(RichText::new(&row.value).color(style::TEXT_MUTED).monospace().size(11.0));
                });
            });
        });
}

/// Editable non-negative-integer cell.
fn uint_cell(
    ui: &mut egui::Ui,
    buffers: &mut HashMap<String, String>,
    path: &[&str],
    current: &str,
    label: String,
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    let buf = buffers.entry(path.join(".")).or_insert_with(|| current.to_string());
    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(90.0));
    if resp.changed() {
        let v = buf.trim();
        if v.parse::<u64>().is_ok() && v != current.trim() {
            edits.push((path.iter().map(|s| s.to_string()).collect(), label, v.to_string()));
        }
    } else if !resp.has_focus() && buf.trim() != current.trim() {
        *buf = current.to_string();
    }
}

/// Editable number cell that accepts a finite value, written verbatim (BigDouble).
fn big_cell(
    ui: &mut egui::Ui,
    buffers: &mut HashMap<String, String>,
    path: &[&str],
    current: &str,
    label: String,
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    let buf = buffers.entry(path.join(".")).or_insert_with(|| current.to_string());
    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(160.0));
    if resp.changed() {
        let v = buf.trim();
        if v.parse::<f64>().is_ok_and(f64::is_finite) && v != current.trim() {
            edits.push((path.iter().map(|s| s.to_string()).collect(), label, v.to_string()));
        }
    } else if !resp.has_focus() && buf.trim() != current.trim() {
        *buf = current.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use save_parser::container::encode_container;
    use save_parser::raw::Field;

    fn sc(s: &str) -> Field {
        Field::Value(Raw::Scalar(s.into()))
    }

    #[test]
    fn reads_modules_with_resolved_grades() {
        let modules = Raw::List(vec![
            Raw::Struct(vec![("a".into(), sc("0")), ("b".into(), sc("0")), ("c".into(), sc("0")), ("f".into(), sc("0"))]),
            Raw::Struct(vec![("a".into(), sc("2")), ("b".into(), sc("12")), ("c".into(), sc("115200")), ("f".into(), sc("12"))]),
        ]);
        let w = Raw::Struct(vec![
            ("a".into(), sc("400000")),
            ("b".into(), Field::Value(modules)),
            ("c".into(), sc("912")),
        ]);
        let x = Raw::Struct(vec![("w".into(), Field::Value(Raw::Base64(Box::new(w))))]);
        let root = Raw::Struct(vec![("X".into(), Field::Value(Raw::Base64(Box::new(x))))]);
        let s = EditSession::load(&encode_container(&root.serialize(), "V2"), None).unwrap();

        let rows = read_modules(&s);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "Physical");
        assert_eq!(rows[1].name, "Battle");
        assert_eq!(rows[1].level, "12");
        assert_eq!(rows[1].cost, "115200");
        assert_eq!(s.value(&["X", "w", "a"]).as_deref(), Some("400000"));
    }
}
