//! Crystal Factory section: the single struct at `X.w` (`GKJLJMJLMIB`).
//!
//! Edits the factory's Crystal Power (`X.w.a`), **Energy** (`X.w.c`, spent on
//! module upgrades), each module's level + its stored grade-1 crystal count
//! (`X.w.b.<i>.e.c`), and the equipped crystals' grades (`X.w.e`). The module
//! cost/timer BigDoubles stay read-only (roles not fully pinned).
//!
//! No typed model for this struct, so values read straight from the raw tree via
//! the `CrystalFactoryField` / `CrystalModuleField` / `CrystalStackField`
//! descriptors; edits stage by raw path / list index.

use std::collections::HashMap;

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use save_parser::items;
use save_parser::labels::{CrystalFactoryField, CrystalModuleField, CrystalStackField};
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
    /// Stored grade-1 crystal count (`X.w.b.<i>.e.c`); `None` for an unbuilt
    /// module (no stored stack).
    stored: Option<String>,
}

/// One equipped-crystal row (`X.w.e.<index>`).
struct EquippedRow {
    /// Full raw path of the grade (`b`) field — list-index or lone-struct form.
    grade_path: Vec<String>,
    name: String,
    grade: String,
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
                // Stored grade-1 crystals live in the module's `e` stack (`e.c`).
                stored: session.value(&["X", "w", "b", &i, "e", CrystalStackField::Count.key()]),
            }
        })
        .collect()
}

fn read_equipped(session: &EditSession) -> Vec<EquippedRow> {
    // `X.w.e` is a list of equipped crystals; a single one collapses to a lone
    // struct, so handle both forms.
    let prefixes: Vec<Vec<String>> = match session.root().get_path(&["X", "w", "e"]) {
        Some(Raw::List(items)) => (0..items.len())
            .map(|i| vec!["X".to_string(), "w".into(), "e".into(), i.to_string()])
            .collect(),
        Some(Raw::Struct(_)) => vec![vec!["X".to_string(), "w".into(), "e".into()]],
        _ => return Vec::new(),
    };
    prefixes
        .into_iter()
        .map(|prefix| {
            let read = |k: &str| {
                let mut path = prefix.clone();
                path.push(k.to_string());
                let p: Vec<&str> = path.iter().map(String::as_str).collect();
                session.value(&p).unwrap_or_default()
            };
            let type_id: u32 = read(CrystalStackField::Crystal.key()).trim().parse().unwrap_or(0);
            let mut grade_path = prefix.clone();
            grade_path.push(CrystalStackField::Grade.key().to_string());
            EquippedRow {
                grade_path,
                name: items::crystal_module_name(type_id)
                    .map_or_else(|| format!("Type {type_id}"), str::to_string),
                grade: read(CrystalStackField::Grade.key()),
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
    let equipped = read_equipped(session);
    let mut edits: Vec<(Vec<String>, String, String)> = Vec::new();

    // --- Factory scalars ---
    egui::Grid::new("crystal_scalars").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Crystal Power");
        let cur = session.value(&["X", "w", CrystalFactoryField::CrystalPower.key()]).unwrap_or_default();
        big_cell(ui, &mut st.buffers, &["X", "w", "a"], &cur, "Crystal Power".into(), &mut edits);
        ui.end_row();
        // Energy (X.w.c) is spent on module upgrades — editable.
        ui.label("Energy");
        let energy = session.value(&["X", "w", CrystalFactoryField::Energy.key()]).unwrap_or_default();
        big_cell(ui, &mut st.buffers, &["X", "w", "c"], &energy, "Energy".into(), &mut edits);
        ui.end_row();
        ui.label("Progress");
        let prog = session.value(&["X", "w", "d"]).unwrap_or_else(|| "—".into());
        ui.label(RichText::new(prog).color(style::TEXT_MUTED).monospace().size(11.0));
        ui.end_row();
    });
    ui.separator();

    // --- Modules ---
    ui.label(RichText::new("Modules").strong());
    if modules.is_empty() {
        ui.label(RichText::new("No modules.").color(style::TEXT_MUTED));
    } else {
        module_table(ui, st, &modules, &mut edits);
    }

    // --- Equipped crystals ---
    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new("Equipped Crystals").strong());
    if equipped.is_empty() {
        ui.label(RichText::new("No equipped crystals.").color(style::TEXT_MUTED));
    } else {
        equipped_table(ui, st, &equipped, &mut edits);
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
        .column(Column::initial(140.0)) // type
        .column(Column::initial(90.0)) // level
        .column(Column::initial(130.0)) // stored grade-1
        .column(Column::remainder()) // cost
        .header(20.0, |mut h| {
            for title in ["Type", "Level", "Stored (g1)", "Cost"] {
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
                    ui.label(&row.name).on_hover_text(format!("type id {}", row.grade_id));
                });
                tr.col(|ui| {
                    let p = ["X", "w", "b", &idx, "b"];
                    uint_cell(ui, &mut st.buffers, &p, &row.level, format!("{} level", row.name), edits);
                });
                tr.col(|ui| match &row.stored {
                    // Editable only when the module has a stored stack (`e.c`).
                    Some(stored) => {
                        let p = ["X", "w", "b", &idx, "e", "c"];
                        uint_cell(ui, &mut st.buffers, &p, stored, format!("{} stored", row.name), edits);
                    }
                    None => {
                        ui.label(RichText::new("—").color(style::TEXT_MUTED).size(11.0));
                    }
                });
                tr.col(|ui| {
                    ui.label(RichText::new(&row.cost).color(style::TEXT_MUTED).monospace().size(11.0));
                });
            });
        });
}

fn equipped_table(
    ui: &mut egui::Ui,
    st: &mut CrystalEditState,
    rows: &[EquippedRow],
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    TableBuilder::new(ui)
        .striped(true)
        .id_salt("crystal_equipped")
        .column(Column::initial(140.0)) // type
        .column(Column::remainder()) // grade
        .header(20.0, |mut h| {
            for title in ["Crystal", "Grade"] {
                h.col(|ui| {
                    ui.label(RichText::new(title).strong().size(12.0));
                });
            }
        })
        .body(|body| {
            body.rows(24.0, rows.len(), |mut tr| {
                let row = &rows[tr.index()];
                tr.col(|ui| {
                    ui.label(&row.name);
                });
                tr.col(|ui| {
                    let p: Vec<&str> = row.grade_path.iter().map(String::as_str).collect();
                    uint_cell(ui, &mut st.buffers, &p, &row.grade, format!("{} grade", row.name), edits);
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

    fn b64(r: Raw) -> Field {
        Field::Value(Raw::Base64(Box::new(r)))
    }

    /// A crystal stack `{a:type, b:grade, c:count}` (used for stored & equipped).
    fn stack(ty: &str, grade: &str, count: &str) -> Raw {
        Raw::Struct(vec![("a".into(), sc(ty)), ("b".into(), sc(grade)), ("c".into(), sc(count))])
    }

    /// `X.w` with: an unbuilt Physical module, a built Battle module (level 13,
    /// stored 1814 grade-1), Energy 2058, and one equipped Battle crystal (g23).
    fn crystal_session() -> EditSession {
        let modules = Raw::List(vec![
            Raw::Struct(vec![("a".into(), sc("0")), ("b".into(), sc("0"))]),
            Raw::Struct(vec![
                ("a".into(), sc("2")),
                ("b".into(), sc("13")),
                ("c".into(), sc("124800")),
                // `e` = the stored grade-1 stack (a 1-element list → lone struct).
                ("e".into(), Field::Value(Raw::List(vec![stack("2", "1", "1814")]))),
            ]),
        ]);
        // Two equipped crystals so X.w.e stays a list (Battle g23, God g12).
        let equipped = Raw::List(vec![stack("2", "23", "0"), stack("5", "12", "0")]);
        let w = Raw::Struct(vec![
            ("a".into(), sc("400000")),
            ("b".into(), Field::Value(modules)),
            ("c".into(), sc("2058")),
            ("e".into(), Field::Value(equipped)),
        ]);
        let x = Raw::Struct(vec![("w".into(), b64(w))]);
        let root = Raw::Struct(vec![("X".into(), b64(x))]);
        EditSession::load(&encode_container(&root.serialize(), "V2"), None).unwrap()
    }

    #[test]
    fn reads_modules_stored_and_energy() {
        let s = crystal_session();
        let rows = read_modules(&s);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "Physical");
        assert_eq!(rows[0].stored, None, "unbuilt module has no stored stack");
        assert_eq!(rows[1].name, "Battle");
        assert_eq!(rows[1].level, "13");
        assert_eq!(rows[1].stored.as_deref(), Some("1814"));
        // Energy is X.w.c.
        assert_eq!(s.value(&["X", "w", "c"]).as_deref(), Some("2058"));
    }

    #[test]
    fn reads_equipped_crystal_grades() {
        let s = crystal_session();
        let rows = read_equipped(&s);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "Battle");
        assert_eq!(rows[0].grade, "23");
        assert_eq!(rows[0].grade_path, vec!["X", "w", "e", "0", "b"]);
        assert_eq!(rows[1].name, "God");
        assert_eq!(rows[1].grade, "12");
    }
}
