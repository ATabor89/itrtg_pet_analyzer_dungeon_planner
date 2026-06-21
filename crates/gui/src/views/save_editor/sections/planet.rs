//! Planet / Ultimate Beings section: the planet's top-level scalars (`T.d`
//! level, `T.h` unspent Baal Power) plus the 5 Ultimate Beings.
//!
//! Each UB has two parallel records: `T.f` (live state — alive flag + spawn
//! countdown) and `T.k` (the kill count that DRIVES the "Multi from Ultimate
//! Beings" bonus). This section merges them per UB so you can bump the
//! multiplier-driving kill count or force a UB to spawn (set its countdown to 0).
//!
//! No typed model for the UB lists, so rows read straight from the raw tree via
//! the `UbField` / `UbMultField` descriptors; edits stage by raw list index.

use std::collections::HashMap;

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use save_parser::items;
use save_parser::labels::{PlanetField, UbField, UbMultField};
use save_parser::raw::Raw;

use crate::style;
use crate::views::save_editor::session::EditSession;

/// One UB row, merging its `T.f` (live) and `T.k` (multiplier) records by UB id.
struct UbRow {
    ub_id: u32,
    ub_name: String,
    /// Index into `T.f` (live state).
    f_index: usize,
    /// Index into `T.k` (multiplier), if a matching record exists.
    k_index: Option<usize>,
    /// Multiplier-driving kill count (`T.k.b`).
    mult_kills: String,
    alive: bool,
    spawn_ms: f64,
}

#[derive(Default)]
pub struct PlanetEditState {
    /// Buffers for the top-level scalar fields, keyed by dotted path.
    scalar_buffers: HashMap<String, String>,
    /// Buffers for the per-UB multiplier-kill cells, keyed by `T.k` index.
    ub_buffers: HashMap<usize, String>,
    status: Option<(String, bool)>,
}

fn ub_name(id: u32) -> String {
    items::ultimate_being_name(id).map_or_else(|| format!("UB {id}"), str::to_string)
}

/// Read and merge the UB rows from `T.f` (live) + `T.k` (multiplier).
fn read_ub_rows(session: &EditSession) -> Vec<UbRow> {
    let Some(Raw::List(fs)) = session.root().get_path(&["T", "f"]) else {
        return Vec::new();
    };
    // Map UB id -> T.k index (the multiplier record).
    let k_by_id: HashMap<u32, usize> = match session.root().get_path(&["T", "k"]) {
        Some(Raw::List(ks)) => (0..ks.len())
            .filter_map(|i| {
                let id = session.value(&["T", "k", &i.to_string(), UbMultField::Ub.key()])?;
                Some((id.trim().parse().ok()?, i))
            })
            .collect(),
        _ => HashMap::new(),
    };
    (0..fs.len())
        .map(|fi| {
            let i = fi.to_string();
            let fget = |k: &str| session.value(&["T", "f", &i, k]).unwrap_or_default();
            let ub_id: u32 = fget(UbField::Ub.key()).trim().parse().unwrap_or(0);
            let k_index = k_by_id.get(&ub_id).copied();
            let mult_kills = k_index
                .and_then(|ki| session.value(&["T", "k", &ki.to_string(), UbMultField::KillCount.key()]))
                .unwrap_or_default();
            UbRow {
                ub_id,
                ub_name: ub_name(ub_id),
                f_index: fi,
                k_index,
                mult_kills,
                alive: fget(UbField::Alive.key()).trim().eq_ignore_ascii_case("true"),
                spawn_ms: fget(UbField::SpawnCountdown.key()).trim().parse().unwrap_or(0.0),
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
            "The planet level and unspent Baal Power are top-level values. Each Ultimate Being's \
             kill count drives the \u{201c}Multi from Ultimate Beings\u{201d} bonus; \u{201c}Force \
             spawn\u{201d} sets its spawn countdown to 0.",
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

    // --- Ultimate Beings ---
    let rows = read_ub_rows(session);
    if rows.is_empty() {
        ui.label(RichText::new("No Ultimate Beings in this save.").color(style::TEXT_MUTED));
    } else {
        ub_table(ui, st, &rows, &mut edits);
    }

    // Apply.
    let mut ok = false;
    for (path, label, value) in edits {
        let p: Vec<&str> = path.iter().map(String::as_str).collect();
        match session.set_scalar(&p, label, &value) {
            Ok(_) => ok = true,
            Err(e) => st.status = Some((format!("Edit failed: {e}"), true)),
        }
    }
    if ok {
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

fn ub_table(
    ui: &mut egui::Ui,
    st: &mut PlanetEditState,
    rows: &[UbRow],
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    TableBuilder::new(ui)
        .striped(true)
        .column(Column::initial(150.0)) // UB
        .column(Column::initial(140.0)) // multiplier kills
        .column(Column::initial(60.0)) // alive
        .column(Column::initial(130.0)) // spawn countdown
        .column(Column::remainder()) // force spawn
        .header(20.0, |mut h| {
            for title in ["Ultimate Being", "Kill count (bonus)", "Alive", "Spawn in", ""] {
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
                // Multiplier-driving kill count (T.k.b), editable when matched.
                tr.col(|ui| {
                    let Some(ki) = row.k_index else {
                        ui.label(RichText::new("—").color(style::TEXT_MUTED));
                        return;
                    };
                    let buf = st.ub_buffers.entry(ki).or_insert_with(|| row.mult_kills.clone());
                    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(110.0));
                    if resp.changed() {
                        let v = buf.trim();
                        if v.parse::<u64>().is_ok() && v != row.mult_kills.trim() {
                            edits.push((
                                vec!["T".into(), "k".into(), ki.to_string(), UbMultField::KillCount.key().into()],
                                format!("{} kill count", row.ub_name),
                                v.to_string(),
                            ));
                        }
                    } else if !resp.has_focus() && buf.trim() != row.mult_kills.trim() {
                        *buf = row.mult_kills.clone();
                    }
                });
                tr.col(|ui| {
                    ui.label(if row.alive { "yes" } else { "no" });
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
