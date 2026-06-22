//! Planet / Ultimate Beings section: the planet's top-level scalars (`T.d`
//! level, `T.h` unspent Baal Power) plus two distinct boss lists.
//!
//! There are two parallel UB lists in the save, and they are NOT the same bosses:
//!  - `T.f` (`CEFAAPALBMD`) — the regular, **respawning** Ultimate Beings that
//!    attack on staggered spawn timers (kill count, god power gained, countdown).
//!  - `T.k` (`FPBMNCNKPHN`) — the **Ultimate Being V2** bosses, each defeatable
//!    **once per rebirth**; their cumulative defeats drive the "Multi from
//!    Ultimate Beings" bonus. The C# gates this list on the `UBV2C` challenge.
//!
//! They share the same id space / names (the V2s just append " V2"), so this
//! section renders them as two separate tables rather than merging by id.
//!
//! No typed model for the UB lists, so rows read straight from the raw tree via
//! the `UbField` / `UbV2Field` descriptors; edits stage by raw list index.

use std::collections::HashMap;

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use save_parser::items;
use save_parser::labels::{PlanetField, UbField, UbV2Field};
use save_parser::raw::Raw;

use crate::style;
use crate::views::save_editor::session::EditSession;

/// One regular (respawning) UB row, from `T.f`.
struct UbRow {
    ub_id: u32,
    ub_name: String,
    /// Index into `T.f`.
    f_index: usize,
    kill_count: String,
    /// God power gained (`T.f.f`) — a BigDouble; kept as raw text (can exceed f64).
    god_power: String,
    spawn_ms: f64,
}

/// One Ultimate Being V2 row, from `T.k`.
struct UbV2Row {
    ub_id: u32,
    ub_name: String,
    /// Index into `T.k`.
    k_index: usize,
    defeats: String,
    state: String,
}

#[derive(Default)]
pub struct PlanetEditState {
    /// Buffers for the top-level scalar fields, keyed by dotted path.
    scalar_buffers: HashMap<String, String>,
    /// Buffers for the per-UB kill-count cells (`T.f` index).
    ub_buffers: HashMap<usize, String>,
    /// Buffers for the per-UBv2 defeat cells (`T.k` index).
    ubv2_buffers: HashMap<usize, String>,
    status: Option<(String, bool)>,
}

fn ub_name(id: u32) -> String {
    items::ultimate_being_name(id).map_or_else(|| format!("UB {id}"), str::to_string)
}

/// Read the regular UB rows from `T.f`.
fn read_ub_rows(session: &EditSession) -> Vec<UbRow> {
    let Some(Raw::List(fs)) = session.root().get_path(&["T", "f"]) else {
        return Vec::new();
    };
    (0..fs.len())
        .map(|fi| {
            let i = fi.to_string();
            let fget = |k: &str| session.value(&["T", "f", &i, k]).unwrap_or_default();
            let ub_id: u32 = fget(UbField::Ub.key()).trim().parse().unwrap_or(0);
            UbRow {
                ub_id,
                ub_name: ub_name(ub_id),
                f_index: fi,
                kill_count: fget(UbField::KillCount.key()),
                god_power: fget(UbField::GodPowerGained.key()),
                spawn_ms: fget(UbField::SpawnCountdown.key()).trim().parse().unwrap_or(0.0),
            }
        })
        .collect()
}

/// Read the Ultimate Being V2 rows from `T.k`.
fn read_ubv2_rows(session: &EditSession) -> Vec<UbV2Row> {
    let Some(Raw::List(ks)) = session.root().get_path(&["T", "k"]) else {
        return Vec::new();
    };
    (0..ks.len())
        .map(|ki| {
            let i = ki.to_string();
            let kget = |k: &str| session.value(&["T", "k", &i, k]).unwrap_or_default();
            let ub_id: u32 = kget(UbV2Field::Ub.key()).trim().parse().unwrap_or(0);
            UbV2Row {
                ub_id,
                ub_name: format!("{} V2", ub_name(ub_id)),
                k_index: ki,
                defeats: kget(UbV2Field::Defeats.key()),
                state: kget(UbV2Field::State.key()),
            }
        })
        .collect()
}

pub fn show(ui: &mut egui::Ui, session: &mut EditSession, st: &mut PlanetEditState) {
    ui.heading("Planet / Ultimate Beings");

    if session.root().get_path(&["T"]).is_none() {
        ui.label(RichText::new("No planet data in this save.").color(style::TEXT_MUTED));
        return;
    }

    ui.label(
        RichText::new(
            "The planet level and unspent Baal Power are top-level values. The Ultimate Beings \
             respawn on a timer (\u{201c}Force spawn\u{201d} sets the countdown to 0); the Ultimate \
             Being V2 bosses are defeated once per rebirth, and their cumulative defeats drive the \
             \u{201c}Multi from Ultimate Beings\u{201d} bonus.",
        )
        .color(style::TEXT_MUTED)
        .size(11.0),
    );
    ui.separator();

    if let Some((msg, err)) = &st.status {
        let color = if *err { style::ERROR } else { style::SUCCESS };
        ui.label(RichText::new(msg).color(color).size(12.0));
    }

    // Collected edits applied after the read-only render borrow is released.
    let mut edits: Vec<(Vec<String>, String, String)> = Vec::new();

    // --- Top-level scalars ---
    egui::Grid::new("planet_scalars").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        scalar_editor(ui, session, st, &["T", PlanetField::Level.key()], "Planet Level", &mut edits);
        ui.end_row();
        scalar_editor(
            ui,
            session,
            st,
            &["T", PlanetField::BaalPower.key()],
            "Unspent Baal Power",
            &mut edits,
        );
        ui.end_row();
    });
    ui.separator();

    // --- Ultimate Beings (regular, respawning) ---
    ui.label(RichText::new("Ultimate Beings").strong());
    let rows = read_ub_rows(session);
    if rows.is_empty() {
        ui.label(RichText::new("No Ultimate Beings in this save.").color(style::TEXT_MUTED));
    } else {
        ub_table(ui, st, &rows, &mut edits);
    }

    ui.add_space(8.0);
    ui.separator();

    // --- Ultimate Beings V2 ---
    ui.label(RichText::new("Ultimate Beings V2").strong());
    ui.label(
        RichText::new("Defeated once per rebirth; defeats accumulate across rebirths.")
            .color(style::TEXT_MUTED)
            .size(11.0),
    );
    let v2_rows = read_ubv2_rows(session);
    if v2_rows.is_empty() {
        ui.label(RichText::new("No Ultimate Being V2 data in this save.").color(style::TEXT_MUTED));
    } else {
        ubv2_table(ui, st, &v2_rows, &mut edits);
    }

    // Apply. A failure's status must not be overwritten by a later success in
    // the same batch.
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
        st.status = Some(("Staged planet edit".to_string(), false));
    }
}

/// A labeled, validated editable scalar; stages an edit into `edits` on change.
fn scalar_editor(
    ui: &mut egui::Ui,
    session: &EditSession,
    st: &mut PlanetEditState,
    path: &[&str],
    label: &str,
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    ui.label(label);
    let key = path.join(".");
    let current = session.value(path).unwrap_or_default();
    let buf = st.scalar_buffers.entry(key).or_insert_with(|| current.clone());
    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(160.0));
    if resp.lost_focus() {
        let v = buf.trim().to_string();
        if v != current && v.parse::<f64>().is_ok() {
            edits.push((path.iter().map(|s| s.to_string()).collect(), label.to_string(), v));
        }
    } else if !resp.has_focus() && buf.as_str() != current {
        *buf = current;
    }
}

/// An editable unsigned-integer cell backed by `buffers[index]`; stages into
/// `edits` on change. `path` is the full raw path of the field being edited.
fn uint_cell(
    ui: &mut egui::Ui,
    buffers: &mut HashMap<usize, String>,
    index: usize,
    current: &str,
    path: Vec<String>,
    label: String,
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    let buf = buffers.entry(index).or_insert_with(|| current.to_string());
    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(100.0));
    if resp.changed() {
        let v = buf.trim();
        if v.parse::<u64>().is_ok() && v != current.trim() {
            edits.push((path, label, v.to_string()));
        }
    } else if !resp.has_focus() && buf.trim() != current.trim() {
        *buf = current.to_string();
    }
}

fn ub_table(
    ui: &mut egui::Ui,
    st: &mut PlanetEditState,
    rows: &[UbRow],
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    TableBuilder::new(ui)
        .striped(true)
        .id_salt("planet_ub")
        .column(Column::initial(150.0)) // UB
        .column(Column::initial(120.0)) // kill count
        .column(Column::initial(130.0)) // god power gained
        .column(Column::initial(110.0)) // spawn countdown
        .column(Column::remainder()) // force spawn
        .header(20.0, |mut h| {
            for title in ["Ultimate Being", "Kill count", "God power gained", "Spawn in", ""] {
                h.col(|ui| {
                    ui.label(RichText::new(title).strong().size(12.0));
                });
            }
        })
        .body(|body| {
            body.rows(24.0, rows.len(), |mut tr| {
                let row = &rows[tr.index()];
                tr.col(|ui| {
                    ui.label(&row.ub_name).on_hover_text(format!("UB id {}", row.ub_id));
                });
                tr.col(|ui| {
                    uint_cell(
                        ui,
                        &mut st.ub_buffers,
                        row.f_index,
                        &row.kill_count,
                        vec!["T".into(), "f".into(), row.f_index.to_string(), UbField::KillCount.key().into()],
                        format!("{} kill count", row.ub_name),
                        edits,
                    );
                });
                tr.col(|ui| {
                    ui.label(RichText::new(&row.god_power).size(11.0));
                });
                tr.col(|ui| {
                    let mins = row.spawn_ms / 60_000.0;
                    ui.label(RichText::new(format!("{mins:.1} min")).size(11.0));
                });
                tr.col(|ui| {
                    if row.spawn_ms > 0.0
                        && ui
                            .button("Force spawn")
                            .on_hover_text("Set the spawn countdown to 0 so this UB spawns next tick")
                            .clicked()
                    {
                        edits.push((
                            vec!["T".into(), "f".into(), row.f_index.to_string(), UbField::SpawnCountdown.key().into()],
                            format!("{} spawn countdown", row.ub_name),
                            "0".to_string(),
                        ));
                    }
                });
            });
        });
}

fn ubv2_table(
    ui: &mut egui::Ui,
    st: &mut PlanetEditState,
    rows: &[UbV2Row],
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    TableBuilder::new(ui)
        .striped(true)
        .id_salt("planet_ubv2")
        .column(Column::initial(170.0)) // UB V2
        .column(Column::initial(120.0)) // defeats
        .column(Column::remainder()) // state
        .header(20.0, |mut h| {
            for title in ["Ultimate Being V2", "Defeats", "State"] {
                h.col(|ui| {
                    ui.label(RichText::new(title).strong().size(12.0));
                });
            }
        })
        .body(|body| {
            body.rows(24.0, rows.len(), |mut tr| {
                let row = &rows[tr.index()];
                tr.col(|ui| {
                    ui.label(&row.ub_name).on_hover_text(format!("UB id {}", row.ub_id));
                });
                tr.col(|ui| {
                    uint_cell(
                        ui,
                        &mut st.ubv2_buffers,
                        row.k_index,
                        &row.defeats,
                        vec!["T".into(), "k".into(), row.k_index.to_string(), UbV2Field::Defeats.key().into()],
                        format!("{} defeats", row.ub_name),
                        edits,
                    );
                });
                tr.col(|ui| {
                    ui.label(RichText::new(&row.state).color(style::TEXT_MUTED).size(11.0));
                });
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ub_name_resolves_and_falls_back() {
        assert_eq!(ub_name(1), "Planet Eater");
        assert_eq!(ub_name(5), "ITRTG");
        assert_eq!(ub_name(99), "UB 99");
    }
}
