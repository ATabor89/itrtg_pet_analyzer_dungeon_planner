//! Fishing section: root `025`. Edit level/exp, toggle owned rods, grant bait
//! (counts), and adjust fish-caught counts. Fish Power (`025.a`) lives in the
//! Resources section. Selected rod/bait/pond are shown read-only for context.
//!
//! No typed model for these lists, so rows read straight from the raw tree via
//! the `Fishing*Field` descriptors; edits stage by raw list index.

use std::collections::HashMap;

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use save_parser::items;
use save_parser::labels::{FishingBaitField, FishingField, FishingFishField, FishingRodField};
use save_parser::raw::Raw;

use crate::style;
use crate::views::save_editor::session::EditSession;

struct RodRow {
    index: usize,
    name: String,
    owned: bool,
}
struct BaitRow {
    index: usize,
    name: String,
    count: String,
}
struct FishRow {
    index: usize,
    name: String,
    caught: String,
}

#[derive(Default)]
pub struct FishingEditState {
    /// Buffers for the top-level scalar fields (level/exp), keyed by dotted path.
    scalar_buffers: HashMap<String, String>,
    /// Per-row buffers for bait counts (`025.h` index) and fish caught (`025.i`).
    bait_buffers: HashMap<usize, String>,
    fish_buffers: HashMap<usize, String>,
    status: Option<(String, bool)>,
}

fn mat_name(id: u32) -> String {
    items::material_name(id).map_or_else(|| format!("id {id}"), str::to_string)
}

fn read_rods(session: &EditSession) -> Vec<RodRow> {
    let Some(Raw::List(items_list)) = session.root().get_path(&["025", "g"]) else {
        return Vec::new();
    };
    (0..items_list.len())
        .map(|index| {
            let i = index.to_string();
            let id: u32 = session
                .value(&["025", "g", &i, FishingRodField::Rod.key()])
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            let owned = session
                .value(&["025", "g", &i, FishingRodField::Owned.key()])
                .is_some_and(|s| s.trim() != "0" && !s.trim().is_empty());
            RodRow { index, name: mat_name(id), owned }
        })
        .collect()
}

fn read_bait(session: &EditSession) -> Vec<BaitRow> {
    let Some(Raw::List(items_list)) = session.root().get_path(&["025", "h"]) else {
        return Vec::new();
    };
    (0..items_list.len())
        .map(|index| {
            let i = index.to_string();
            let id: u32 = session
                .value(&["025", "h", &i, FishingBaitField::Bait.key()])
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            BaitRow {
                index,
                name: mat_name(id),
                count: session.value(&["025", "h", &i, FishingBaitField::Count.key()]).unwrap_or_default(),
            }
        })
        .collect()
}

fn read_fish(session: &EditSession) -> Vec<FishRow> {
    let Some(Raw::List(items_list)) = session.root().get_path(&["025", "i"]) else {
        return Vec::new();
    };
    (0..items_list.len())
        .map(|index| {
            let i = index.to_string();
            let id: u32 = session
                .value(&["025", "i", &i, FishingFishField::Fish.key()])
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            FishRow {
                index,
                name: mat_name(id),
                caught: session.value(&["025", "i", &i, FishingFishField::Caught.key()]).unwrap_or_default(),
            }
        })
        .collect()
}

pub fn show(ui: &mut egui::Ui, session: &mut EditSession, st: &mut FishingEditState) {
    ui.heading("Fishing");

    if session.root().get_path(&["025"]).is_none() {
        ui.label(RichText::new("No fishing data in this save.").color(style::TEXT_MUTED));
        return;
    }

    if let Some((msg, err)) = &st.status {
        let color = if *err { style::ERROR } else { style::SUCCESS };
        ui.label(RichText::new(msg).color(color).size(12.0));
    }

    let rods = read_rods(session);
    let bait = read_bait(session);
    let fish = read_fish(session);
    // Selected rod/bait/pond, resolved for context.
    let sel = |k: &str| session.value(&["025", k]).and_then(|s| s.trim().parse::<u32>().ok());
    let pond = sel(FishingField::CurrentPond.key())
        .and_then(items::pond_name)
        .map_or_else(|| "—".to_string(), str::to_string);
    let rod = sel(FishingField::SelectedRod.key()).map_or_else(|| "—".to_string(), mat_name);
    let bait_sel = sel(FishingField::SelectedBait.key()).map_or_else(|| "—".to_string(), mat_name);

    let mut edits: Vec<(Vec<String>, String, String)> = Vec::new();

    // --- Scalars: level + exp ---
    egui::Grid::new("fishing_scalars").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        scalar_editor(ui, session, st, &["025", FishingField::Level.key()], "Fishing Level", &mut edits);
        ui.end_row();
        scalar_editor(ui, session, st, &["025", FishingField::Exp.key()], "Fishing Exp", &mut edits);
        ui.end_row();
    });
    ui.label(
        RichText::new(format!("Pond: {pond}  ·  Rod: {rod}  ·  Bait: {bait_sel}"))
            .color(style::TEXT_MUTED)
            .size(11.0),
    );
    ui.separator();

    // --- Rods: owned toggle ---
    ui.label(RichText::new("Rods").strong());
    rod_table(ui, &rods, &mut edits);
    ui.add_space(6.0);
    ui.separator();

    // --- Bait: counts ---
    ui.label(RichText::new("Bait").strong());
    count_table(ui, "fishing_bait", &bait, &mut st.bait_buffers, &["025", "h"], FishingBaitField::Count.key(), &mut edits);
    ui.add_space(6.0);
    ui.separator();

    // --- Fish caught ---
    ui.label(RichText::new("Fish Caught").strong());
    let fish_rows: Vec<BaitRow> =
        fish.into_iter().map(|f| BaitRow { index: f.index, name: f.name, count: f.caught }).collect();
    count_table(ui, "fishing_fish", &fish_rows, &mut st.fish_buffers, &["025", "i"], FishingFishField::Caught.key(), &mut edits);

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
        st.status = Some(("Staged fishing edit".to_string(), false));
    }
}

/// A labeled, validated editable scalar; stages an edit into `edits` on change.
fn scalar_editor(
    ui: &mut egui::Ui,
    session: &EditSession,
    st: &mut FishingEditState,
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

fn rod_table(ui: &mut egui::Ui, rows: &[RodRow], edits: &mut Vec<(Vec<String>, String, String)>) {
    TableBuilder::new(ui)
        .striped(true)
        .id_salt("fishing_rods")
        .column(Column::initial(220.0))
        .column(Column::remainder())
        .header(20.0, |mut h| {
            for t in ["Rod", "Owned"] {
                h.col(|ui| {
                    ui.label(RichText::new(t).strong().size(12.0));
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
                    let mut owned = row.owned;
                    if ui.checkbox(&mut owned, "").changed() {
                        edits.push((
                            vec!["025".into(), "g".into(), row.index.to_string(), FishingRodField::Owned.key().into()],
                            format!("{} owned", row.name),
                            if owned { "1" } else { "0" }.to_string(),
                        ));
                    }
                });
            });
        });
}

/// A generic "name + editable count" table (used for bait counts and fish caught).
fn count_table(
    ui: &mut egui::Ui,
    salt: &str,
    rows: &[BaitRow],
    buffers: &mut HashMap<usize, String>,
    list: &[&str; 2],
    key: &'static str,
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    TableBuilder::new(ui)
        .striped(true)
        .id_salt(salt)
        .column(Column::initial(220.0))
        .column(Column::remainder())
        .header(20.0, |mut h| {
            for t in ["Item", "Count"] {
                h.col(|ui| {
                    ui.label(RichText::new(t).strong().size(12.0));
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
                    let buf = buffers.entry(row.index).or_insert_with(|| row.count.clone());
                    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(140.0));
                    if resp.changed() {
                        let v = buf.trim();
                        if v.parse::<u64>().is_ok() && v != row.count.trim() {
                            edits.push((
                                vec![list[0].into(), list[1].into(), row.index.to_string(), key.into()],
                                format!("{} count", row.name),
                                v.to_string(),
                            ));
                        }
                    } else if !resp.has_focus() && buf.trim() != row.count.trim() {
                        *buf = row.count.clone();
                    }
                });
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mat_name_resolves_and_falls_back() {
        // 500 = Wooden Rod (a known fishing material); an unknown id falls back.
        assert!(items::material_name(500).is_some());
        assert_eq!(mat_name(999_999), "id 999999");
    }
}
