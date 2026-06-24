//! Dungeons section: the **active dungeon runs** (`X.P`) — force-complete a run
//! so its loot is ready to collect on next load.
//!
//! A run counts elapsed ms (`b`) up to a target (`c`, typically 12 h) and
//! completes when `b` ≥ `c` (the universal elapsed-timer pattern). This section
//! lists each in-progress run with its dungeon/depth/progress and lets you set
//! elapsed directly or force-complete it.
//!
//! Team *composition* (`X.S`) is managed in the Dungeon Planner view, not here.
//! Active runs have no typed model, so rows are read straight from the raw tree
//! (keys via the `ActiveDungeonField` descriptor); edits stage `X.P.<i>.b`.

use std::collections::HashMap;

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use save_parser::items;
use save_parser::labels::ActiveDungeonField as F;
use save_parser::raw::Raw;

use crate::style;
use crate::views::save_editor::session::EditSession;

/// Owned, render-ready snapshot of one active run.
struct RunRow {
    index: usize,
    dungeon_name: String,
    dungeon_id: u32,
    depth: u32,
    elapsed_ms: f64,
    target_ms: u64,
    team_index: String,
}

#[derive(Default)]
pub struct DungeonEditState {
    /// Per-row elapsed-edit buffer (keyed by list index).
    cell_buffers: HashMap<usize, String>,
    status: Option<(String, bool)>,
}

/// Format a millisecond span as hours (e.g. "11.5h").
fn fmt_hours(ms: f64) -> String {
    format!("{:.1}h", ms / 3_600_000.0)
}

/// Completion percentage (0–100), guarding a zero target.
fn percent(elapsed_ms: f64, target_ms: u64) -> f64 {
    if target_ms == 0 {
        0.0
    } else {
        (elapsed_ms / target_ms as f64 * 100.0).clamp(0.0, 100.0)
    }
}

fn read_runs(session: &EditSession) -> Vec<RunRow> {
    let Some(Raw::List(items)) = session.root().get_path(&["X", "P"]) else {
        return Vec::new();
    };
    (0..items.len())
        .map(|index| {
            let i = index.to_string();
            let get = |k: &str| session.value(&["X", "P", &i, k]).unwrap_or_default();
            let dungeon_id: u32 = get(F::DungeonId.key()).trim().parse().unwrap_or(0);
            // Elapsed/target are the game's float type; parse leniently.
            let elapsed_ms: f64 = get(F::Elapsed.key()).trim().parse().unwrap_or(0.0);
            let target_ms: u64 = get(F::TargetDuration.key())
                .trim()
                .parse::<f64>()
                .map(|f| f as u64)
                .unwrap_or(0);
            let depth: u32 = get(F::Depth.key()).trim().parse().unwrap_or(0);
            RunRow {
                index,
                dungeon_name: items::dungeon_name(dungeon_id)
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("Dungeon {dungeon_id}")),
                dungeon_id,
                depth,
                elapsed_ms,
                target_ms,
                team_index: get(F::TeamIndex.key()),
            }
        })
        .collect()
}

pub fn show(ui: &mut egui::Ui, session: &mut EditSession, st: &mut DungeonEditState) {
    ui.heading("Dungeons");
    ui.label(
        RichText::new(
            "Active dungeon runs. Force-complete a run (set elapsed = target) so its loot is \
             ready to collect on next load. Team composition lives in the Dungeon Planner.",
        )
        .color(style::TEXT_MUTED)
        .size(11.0),
    );
    ui.separator();

    if let Some((msg, err)) = &st.status {
        let color = if *err { style::ERROR } else { style::SUCCESS };
        ui.label(RichText::new(msg).color(color).size(12.0));
    }

    let rows = read_runs(session);
    if rows.is_empty() {
        ui.label(
            RichText::new("No active dungeon runs in this save.").color(style::TEXT_MUTED),
        );
        return;
    }

    if ui
        .button("Complete all runs")
        .on_hover_text("Force-complete every active run (set elapsed = target)")
        .clicked()
    {
        let mut n = 0;
        for row in &rows {
            if row.target_ms > 0 {
                let v = row.target_ms.to_string();
                st.cell_buffers.insert(row.index, v.clone());
                let label = format!("{} run elapsed", row.dungeon_name);
                if session.set_scalar(&["X", "P", &row.index.to_string(), "b"], label, &v).is_ok() {
                    n += 1;
                }
            }
        }
        st.status = Some((format!("Completed {n} run(s)"), false));
    }

    table(ui, session, st, &rows);
}

fn table(ui: &mut egui::Ui, session: &mut EditSession, st: &mut DungeonEditState, rows: &[RunRow]) {
    // A staged elapsed edit: (list index, new value, dungeon name for the label).
    let mut edit: Option<(usize, String, String)> = None;
    TableBuilder::new(ui)
        .striped(true)
        .column(Column::initial(140.0)) // dungeon
        .column(Column::initial(56.0)) // depth
        .column(Column::initial(120.0)) // elapsed (editable)
        .column(Column::initial(110.0)) // target
        .column(Column::initial(64.0)) // percent
        .column(Column::initial(110.0)) // force-complete
        .column(Column::remainder()) // team
        .header(20.0, |mut h| {
            for title in ["Dungeon", "Depth", "Elapsed (ms)", "Target", "%", "", "Team"] {
                h.col(|ui| {
                    ui.label(RichText::new(title).strong().size(12.0));
                });
            }
        })
        .body(|body| {
            body.rows(24.0, rows.len(), |mut tr| {
                let row = &rows[tr.index()];
                tr.col(|ui| {
                    ui.label(&row.dungeon_name)
                        .on_hover_text(format!("dungeon id {}", row.dungeon_id));
                });
                tr.col(|ui| {
                    ui.label(format!("D{}", row.depth));
                });
                tr.col(|ui| {
                    let current = format!("{}", row.elapsed_ms);
                    let buf = st.cell_buffers.entry(row.index).or_insert_with(|| current.clone());
                    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(104.0));
                    if resp.changed() {
                        let v = buf.trim();
                        if v.parse::<f64>().is_ok() && v != current.trim() {
                            edit = Some((row.index, v.to_string(), row.dungeon_name.clone()));
                        }
                    } else if !resp.has_focus() && buf.trim() != current.trim() {
                        // Re-seed an idle cell that drifted (force-complete / undo).
                        *buf = current;
                    }
                });
                tr.col(|ui| {
                    ui.label(
                        RichText::new(format!("{} ({})", row.target_ms, fmt_hours(row.target_ms as f64)))
                            .monospace()
                            .size(11.0),
                    );
                });
                tr.col(|ui| {
                    ui.label(format!("{:.0}%", percent(row.elapsed_ms, row.target_ms)));
                });
                tr.col(|ui| {
                    if row.target_ms > 0
                        && ui
                            .button("Force complete")
                            .on_hover_text("Set elapsed = target so the run is ready to collect")
                            .clicked()
                    {
                        edit = Some((row.index, row.target_ms.to_string(), row.dungeon_name.clone()));
                    }
                });
                tr.col(|ui| {
                    ui.label(
                        RichText::new(format!("#{}", row.team_index))
                            .color(style::TEXT_MUTED)
                            .monospace()
                            .size(11.0),
                    );
                });
            });
        });

    if let Some((index, value, dungeon_name)) = edit {
        let label = format!("{dungeon_name} run elapsed");
        st.cell_buffers.insert(index, value.clone());
        if let Err(e) = session.set_scalar(&["X", "P", &index.to_string(), "b"], label, &value) {
            st.status = Some((format!("Edit failed: {e}"), true));
        } else {
            st.status = Some(("Staged dungeon-run elapsed".to_string(), false));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_hours_and_percent() {
        assert_eq!(fmt_hours(43_200_000.0), "12.0h");
        let p = percent(41_300_000.0, 43_200_000);
        assert!((p - 95.6).abs() < 0.1, "{p}");
        assert_eq!(percent(100.0, 0), 0.0);
        assert_eq!(percent(99_999_999.0, 43_200_000), 100.0);
    }
}
