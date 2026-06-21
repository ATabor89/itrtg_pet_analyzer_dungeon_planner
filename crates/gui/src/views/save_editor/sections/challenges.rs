//! Challenges section: view + edit challenge completions (`root.x.242`) and ADD
//! new completion entries — the latter isn't cleanly possible via the raw tree.
//!
//! Challenge Points are *recomputed* by the game from this list (normal
//! challenges: completions × a fixed per-challenge value; Day challenges: by
//! highest score), so editing completions here is how you grant/adjust ChP.
//!
//! There's no typed model for challenges, so rows are read straight from the raw
//! tree. Edits go through `set_scalar` (`x.242.<i>.b`/`.c`); adds go through
//! `session.set_challenge` (upsert by challenge id).

use std::collections::HashMap;

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use save_parser::items;
use save_parser::raw::Raw;

use crate::style;
use crate::views::save_editor::session::EditSession;

/// `HOLHIHDKBKA` difficulty ids (challenge field `c`).
const DIFFICULTIES: &[(&str, u32)] = &[("Normal", 0), ("Hard", 1), ("Root", 2), ("Mixed", 3)];

/// Highest challenge enum id (`OIDDHCOBPLG.BCC`); 49 = `UNUSED`.
const MAX_CHALLENGE_ID: u32 = 76;
const UNUSED_CHALLENGE_ID: u32 = 49;

fn difficulty_name(id: u32) -> String {
    DIFFICULTIES
        .iter()
        .find(|(_, i)| *i == id)
        .map_or_else(|| format!("Difficulty {id}"), |(l, _)| (*l).to_string())
}

fn challenge_label(id: u32) -> String {
    items::challenge_name(id).map_or_else(|| format!("Challenge {id}"), str::to_string)
}

struct ChalRow {
    index: usize,
    challenge_id: u32,
    completions: String,
    difficulty: u32,
}

#[derive(Default)]
struct AddChalState {
    open: bool,
    challenge_id: u32,
    completions: String,
    difficulty: u32,
}

#[derive(Default)]
pub struct ChallengeEditState {
    f_name: String,
    add: AddChalState,
    /// Per-row in-progress completions text (keyed by list index).
    cell_buffers: HashMap<usize, String>,
    status: Option<(String, bool)>,
}

pub fn show(ui: &mut egui::Ui, session: &mut EditSession, st: &mut ChallengeEditState) {
    ui.horizontal(|ui| {
        ui.heading("Challenges");
        if ui.button("➕ Add challenge").clicked() {
            st.add = AddChalState {
                open: true,
                challenge_id: first_unused_challenge(session),
                completions: "1".into(),
                difficulty: 0,
            };
        }
    });
    ui.label(
        RichText::new(
            "Challenge Points are recomputed from these completions (normal challenges: \
             completions × a fixed value; Day challenges score-based, so editing their count \
             won't change ChP).",
        )
        .color(style::TEXT_MUTED)
        .size(11.0),
    );
    ui.separator();

    let rows = read_rows(session);

    ui.horizontal(|ui| {
        ui.label(RichText::new("Filter name:").color(style::TEXT_MUTED));
        ui.add(egui::TextEdit::singleline(&mut st.f_name).desired_width(160.0));
        if ui.button("× clear").clicked() {
            st.f_name.clear();
        }
        ui.label(
            RichText::new(format!("{} challenges", rows.len()))
                .color(style::TEXT_MUTED)
                .size(11.0),
        );
    });
    ui.separator();

    if let Some((msg, err)) = &st.status {
        let color = if *err { style::ERROR } else { style::SUCCESS };
        ui.label(RichText::new(msg).color(color).size(12.0));
    }

    // Add modal.
    if let Some((id, completions, diff)) = add_window(ui.ctx(), &mut st.add) {
        let label = format!("{} completions", challenge_label(id));
        st.status = Some(match session.set_challenge(id, completions.trim(), diff, label) {
            Ok(true) => (format!("Added {}", challenge_label(id)), false),
            Ok(false) => (format!("Updated {} (already present)", challenge_label(id)), false),
            Err(e) => (format!("Add failed: {e}"), true),
        });
    }

    if rows.is_empty() {
        ui.label(
            RichText::new("No challenge completions in this save. Use \u{201c}Add challenge\u{201d}.")
                .color(style::TEXT_MUTED),
        );
        return;
    }

    let needle = st.f_name.trim().to_lowercase();
    let filtered: Vec<usize> = (0..rows.len())
        .filter(|&i| {
            needle.is_empty() || challenge_label(rows[i].challenge_id).to_lowercase().contains(&needle)
        })
        .collect();

    table(ui, session, st, &rows, &filtered);
}

fn read_rows(session: &EditSession) -> Vec<ChalRow> {
    let Some(Raw::List(items)) = session.root().get_path(&["x", "242"]) else {
        return Vec::new();
    };
    (0..items.len())
        .map(|index| {
            let idx = index.to_string();
            let challenge_id = session
                .value(&["x", "242", &idx, "a"])
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            let completions = session.value(&["x", "242", &idx, "b"]).unwrap_or_default();
            let difficulty = session
                .value(&["x", "242", &idx, "c"])
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            ChalRow { index, challenge_id, completions, difficulty }
        })
        .collect()
}

/// First challenge id (1..=MAX, skipping `UNUSED`) not already present, for the
/// Add dialog's default. Falls back to 1 if everything is somehow present.
fn first_unused_challenge(session: &EditSession) -> u32 {
    let present: std::collections::HashSet<u32> = read_rows(session)
        .iter()
        .map(|r| r.challenge_id)
        .collect();
    (1..=MAX_CHALLENGE_ID)
        .find(|id| *id != UNUSED_CHALLENGE_ID && !present.contains(id))
        .unwrap_or(1)
}

fn table(
    ui: &mut egui::Ui,
    session: &mut EditSession,
    st: &mut ChallengeEditState,
    rows: &[ChalRow],
    filtered: &[usize],
) {
    let mut edit: Option<(usize, &'static str, u32, String)> = None; // (index, key, challenge_id, value)
    TableBuilder::new(ui)
        .striped(true)
        .column(Column::initial(240.0)) // challenge
        .column(Column::initial(120.0)) // completions
        .column(Column::initial(140.0)) // difficulty
        .column(Column::remainder())
        .header(20.0, |mut h| {
            for title in ["Challenge", "Completions", "Difficulty", ""] {
                h.col(|ui| {
                    ui.label(RichText::new(title).strong().size(12.0));
                });
            }
        })
        .body(|body| {
            body.rows(24.0, filtered.len(), |mut tr| {
                let row = &rows[filtered[tr.index()]];
                tr.col(|ui| {
                    ui.label(challenge_label(row.challenge_id));
                });
                tr.col(|ui| {
                    let buf = st
                        .cell_buffers
                        .entry(row.index)
                        .or_insert_with(|| row.completions.clone());
                    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(100.0));
                    if resp.changed() {
                        let v = buf.trim();
                        if v.parse::<u64>().is_ok() && v != row.completions.trim() {
                            edit = Some((row.index, "b", row.challenge_id, v.to_string()));
                        }
                    } else if !resp.has_focus() && buf.trim() != row.completions.trim() {
                        // Idle cell drifted from the tree value (e.g. the Add modal
                        // upserted this id, or an undo shifted list indices) —
                        // re-seed it from the save so it shows the current count.
                        *buf = row.completions.clone();
                    }
                });
                tr.col(|ui| {
                    let mut diff = row.difficulty;
                    egui::ComboBox::from_id_salt(("chal_diff", row.index))
                        .selected_text(difficulty_name(diff))
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            for &(label, id) in DIFFICULTIES {
                                ui.selectable_value(&mut diff, id, label);
                            }
                        });
                    if diff != row.difficulty {
                        edit = Some((row.index, "c", row.challenge_id, diff.to_string()));
                    }
                });
                tr.col(|ui| {
                    ui.label(
                        RichText::new(format!("id {}", row.challenge_id))
                            .color(style::TEXT_MUTED)
                            .monospace()
                            .size(10.0),
                    );
                });
            });
        });

    if let Some((index, key, challenge_id, value)) = edit {
        let field = if key == "b" { "completions" } else { "difficulty" };
        let label = format!("{} {field}", challenge_label(challenge_id));
        if let Err(e) = session.set_scalar(&["x", "242", &index.to_string(), key], label, &value) {
            st.status = Some((format!("Edit failed: {e}"), true));
        }
    }
}

/// The add-challenge modal. Returns `Some((challenge_id, completions, difficulty))`
/// on Add.
fn add_window(ctx: &egui::Context, st: &mut AddChalState) -> Option<(u32, String, u32)> {
    if !st.open {
        return None;
    }
    let mut result = None;
    let mut close = false;
    let mut window_open = true;
    egui::Window::new("Add Challenge")
        .collapsible(false)
        .resizable(false)
        .open(&mut window_open)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Challenge:");
                egui::ComboBox::from_id_salt("chal_add_pick")
                    .selected_text(challenge_label(st.challenge_id))
                    .width(240.0)
                    .show_ui(ui, |ui| {
                        // Sorted by name so the ~75-entry list is easy to scan.
                        let mut opts: Vec<(u32, String)> = (1..=MAX_CHALLENGE_ID)
                            .filter(|id| *id != UNUSED_CHALLENGE_ID)
                            .map(|id| (id, challenge_label(id)))
                            .collect();
                        opts.sort_by(|a, b| a.1.cmp(&b.1));
                        for (id, label) in opts {
                            ui.selectable_value(&mut st.challenge_id, id, label);
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Completions:");
                ui.add(egui::TextEdit::singleline(&mut st.completions).desired_width(100.0));
                ui.label("Difficulty:");
                egui::ComboBox::from_id_salt("chal_add_diff")
                    .selected_text(difficulty_name(st.difficulty))
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        for &(label, id) in DIFFICULTIES {
                            ui.selectable_value(&mut st.difficulty, id, label);
                        }
                    });
            });
            ui.label(
                RichText::new(
                    "If the challenge already has an entry, this updates its completion count.",
                )
                .color(style::TEXT_MUTED)
                .size(10.0),
            );
            ui.separator();
            ui.horizontal(|ui| {
                let ok = st.completions.trim().parse::<u64>().is_ok();
                if ui.add_enabled(ok, egui::Button::new("Add")).clicked() {
                    result = Some((st.challenge_id, st.completions.trim().to_string(), st.difficulty));
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
