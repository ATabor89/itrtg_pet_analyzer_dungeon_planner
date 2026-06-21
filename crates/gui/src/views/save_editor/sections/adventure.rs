//! Adventure section: the adventure-mode inventory (`032.d`) and cores (`032.G`).
//!
//! This is the first slice of the Adventure subsystem — grant adventure items
//! (edit counts) and upgrade core quality. Research (`032.H.a`) and the
//! adventurer's own stats (`032.b`) are a planned follow-up.
//!
//! No typed model for these lists, so rows read straight from the raw tree via
//! the `AdventureItemField` / `CoreField` descriptors; edits stage by raw list
//! index (`032.d.<i>.b`, `032.G.<i>.d`).

use std::collections::HashMap;

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use save_parser::items;
use save_parser::labels::{AdventureItemField, CoreField};
use save_parser::raw::Raw;

use crate::style;
use crate::views::save_editor::session::EditSession;

struct ItemRow {
    index: usize,
    item_id: u32,
    name: String,
    count: String,
}

struct CoreRow {
    index: usize,
    enemy_id: u32,
    name: String,
    count: String,
    quality: u32,
}

#[derive(Default)]
pub struct AdventureEditState {
    /// Adventure-inventory name filter.
    f_name: String,
    /// Per-row count buffers (keyed by `032.d` index).
    item_buffers: HashMap<usize, String>,
    status: Option<(String, bool)>,
}

/// Quality letter for an id (F…SSS), falling back to the number.
fn quality_label(q: u32) -> String {
    items::quality_name(q).map_or_else(|| q.to_string(), str::to_string)
}

fn read_items(session: &EditSession) -> Vec<ItemRow> {
    let Some(Raw::List(items_list)) = session.root().get_path(&["032", "d"]) else {
        return Vec::new();
    };
    (0..items_list.len())
        .map(|index| {
            let i = index.to_string();
            let item_id: u32 = session
                .value(&["032", "d", &i, AdventureItemField::Item.key()])
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            ItemRow {
                index,
                item_id,
                name: items::adventure_item_name(item_id)
                    .map_or_else(|| format!("Item {item_id}"), str::to_string),
                count: session
                    .value(&["032", "d", &i, AdventureItemField::Count.key()])
                    .unwrap_or_default(),
            }
        })
        .collect()
}

fn read_cores(session: &EditSession) -> Vec<CoreRow> {
    let Some(Raw::List(cores)) = session.root().get_path(&["032", "G"]) else {
        return Vec::new();
    };
    (0..cores.len())
        .map(|index| {
            let i = index.to_string();
            let enemy_id: u32 = session
                .value(&["032", "G", &i, CoreField::Enemy.key()])
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            CoreRow {
                index,
                enemy_id,
                name: items::adventure_enemy_name(enemy_id)
                    .map_or_else(|| format!("Enemy {enemy_id}"), str::to_string),
                count: session.value(&["032", "G", &i, CoreField::Count.key()]).unwrap_or_default(),
                quality: session
                    .value(&["032", "G", &i, CoreField::Quality.key()])
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0),
            }
        })
        .collect()
}

pub fn show(ui: &mut egui::Ui, session: &mut EditSession, st: &mut AdventureEditState) {
    ui.heading("Adventure");

    if session.root().get_path(&["032"]).is_none() {
        ui.label(RichText::new("No adventure data in this save.").color(style::TEXT_MUTED));
        return;
    }

    if let Some((msg, err)) = &st.status {
        let color = if *err { style::ERROR } else { style::SUCCESS };
        ui.label(RichText::new(msg).color(color).size(12.0));
    }

    let items_rows = read_items(session);
    let cores = read_cores(session);

    // Edits collected during the read-only render, applied after.
    let mut edits: Vec<(Vec<String>, String, String)> = Vec::new();

    // --- Adventure inventory ---
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Adventure Inventory").strong());
        ui.label(RichText::new("· edit item counts").color(style::TEXT_MUTED).size(11.0));
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("Filter:").color(style::TEXT_MUTED));
        ui.add(egui::TextEdit::singleline(&mut st.f_name).desired_width(160.0));
        if ui.button("× clear").clicked() {
            st.f_name.clear();
        }
        ui.label(
            RichText::new(format!("{} items", items_rows.len()))
                .color(style::TEXT_MUTED)
                .size(11.0),
        );
    });

    let needle = st.f_name.trim().to_lowercase();
    let filtered: Vec<usize> = (0..items_rows.len())
        .filter(|&i| needle.is_empty() || items_rows[i].name.to_lowercase().contains(&needle))
        .collect();
    item_table(ui, st, &items_rows, &filtered, &mut edits);

    // --- Cores ---
    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(RichText::new("Cores").strong());
        ui.label(RichText::new("· upgrade core quality").color(style::TEXT_MUTED).size(11.0));
    });
    if cores.is_empty() {
        ui.label(RichText::new("No cores in this save.").color(style::TEXT_MUTED));
    } else {
        core_table(ui, &cores, &mut edits);
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
        st.status = Some(("Staged adventure edit".to_string(), false));
    }
}

fn item_table(
    ui: &mut egui::Ui,
    st: &mut AdventureEditState,
    rows: &[ItemRow],
    filtered: &[usize],
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    TableBuilder::new(ui)
        .striped(true)
        .id_salt("adv_items")
        .column(Column::initial(220.0)) // item
        .column(Column::initial(160.0)) // count
        .column(Column::remainder())
        .header(20.0, |mut h| {
            for title in ["Item", "Count", ""] {
                h.col(|ui| {
                    ui.label(RichText::new(title).strong().size(12.0));
                });
            }
        })
        .body(|body| {
            body.rows(24.0, filtered.len(), |mut tr| {
                let row = &rows[filtered[tr.index()]];
                tr.col(|ui| {
                    ui.label(&row.name).on_hover_text(format!("item id {}", row.item_id));
                });
                tr.col(|ui| {
                    let buf = st.item_buffers.entry(row.index).or_insert_with(|| row.count.clone());
                    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(140.0));
                    if resp.changed() {
                        let v = buf.trim();
                        if v.parse::<u64>().is_ok() && v != row.count.trim() {
                            edits.push((
                                vec!["032".into(), "d".into(), row.index.to_string(), AdventureItemField::Count.key().into()],
                                format!("{} count", row.name),
                                v.to_string(),
                            ));
                        }
                    } else if !resp.has_focus() && buf.trim() != row.count.trim() {
                        *buf = row.count.clone();
                    }
                });
                tr.col(|_| {});
            });
        });
}

fn core_table(
    ui: &mut egui::Ui,
    rows: &[CoreRow],
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    TableBuilder::new(ui)
        .striped(true)
        .id_salt("adv_cores")
        .column(Column::initial(220.0)) // enemy
        .column(Column::initial(120.0)) // quality
        .column(Column::initial(100.0)) // count
        .column(Column::remainder())
        .header(20.0, |mut h| {
            for title in ["Core (enemy)", "Quality", "Count", ""] {
                h.col(|ui| {
                    ui.label(RichText::new(title).strong().size(12.0));
                });
            }
        })
        .body(|body| {
            body.rows(24.0, rows.len(), |mut tr| {
                let row = &rows[tr.index()];
                tr.col(|ui| {
                    ui.label(&row.name).on_hover_text(format!("enemy id {}", row.enemy_id));
                });
                tr.col(|ui| {
                    let mut q = row.quality;
                    egui::ComboBox::from_id_salt(("core_q", row.index))
                        .selected_text(quality_label(q))
                        .width(90.0)
                        .show_ui(ui, |ui| {
                            for id in 0..=8u32 {
                                ui.selectable_value(&mut q, id, quality_label(id));
                            }
                        });
                    if q != row.quality {
                        edits.push((
                            vec!["032".into(), "G".into(), row.index.to_string(), CoreField::Quality.key().into()],
                            format!("{} quality", row.name),
                            q.to_string(),
                        ));
                    }
                });
                tr.col(|ui| {
                    ui.label(RichText::new(&row.count).monospace().size(11.0));
                });
                tr.col(|_| {});
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_label_letters() {
        assert_eq!(quality_label(8), "SSS");
        assert_eq!(quality_label(0), "F");
        // Unknown ids fall back to the number.
        assert_eq!(quality_label(99), "99");
    }
}
