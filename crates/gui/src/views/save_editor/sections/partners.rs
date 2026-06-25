//! Partners section: pet pairings (`X.b[i].F` = partner's type id, `999` = none;
//! `G` = days partnered).
//!
//! Partnerships are **mutual** (both pets point at each other) and a pet may be
//! partnered with itself. The editor preserves the mutual invariant: setting a
//! partner links both sides (and unlinks any displaced exes), so you can't create
//! a one-sided pairing. The partner picker only offers valid choices — pets that
//! are unlocked and currently single (plus "self" and "None"). "Assign all
//! partners" pairs up every remaining single pet at once (odd one out → self),
//! since in-game you can only make one couple per day.

use std::collections::HashMap;

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use save_parser::items;

use crate::style;
use crate::views::save_editor::session::EditSession;

/// One unlocked-pet row.
struct Row {
    index: usize,
    type_id: u32,
    name: String,
    /// Current partner type id (`None` = single; `Some(type_id)`, may equal
    /// `type_id` for a self-partner).
    partner: Option<u32>,
    days: String,
}

#[derive(Default)]
pub struct PartnerEditState {
    filter: String,
    /// Per-row days buffers, keyed by `X.b` index.
    days_buffers: HashMap<usize, String>,
    status: Option<(String, bool)>,
}

fn pet_name(type_id: u32) -> String {
    items::pet_type_name(type_id).map_or_else(|| format!("type {type_id}"), str::to_string)
}

pub fn show(ui: &mut egui::Ui, session: &mut EditSession, st: &mut PartnerEditState) {
    ui.heading("Partners");

    // Snapshot the unlocked roster + the single-pet options from the typed view,
    // then release the borrow so edits can run.
    let (rows, mut options): (Vec<Row>, Vec<(u32, String)>) = {
        let Some(save) = session.derived() else {
            ui.label(
                RichText::new("Typed pet roster unavailable for this save — use the Raw Save Tree.")
                    .color(style::TEXT_MUTED),
            );
            return;
        };
        let rows: Vec<Row> = save
            .pets
            .iter()
            .enumerate()
            .filter(|(_, p)| p.unlocked)
            .map(|(index, p)| Row {
                index,
                type_id: p.type_id,
                name: pet_name(p.type_id),
                partner: p.partner_type_id,
                days: p.partner_days.to_string(),
            })
            .collect();
        let options: Vec<(u32, String)> =
            rows.iter().filter(|r| r.partner.is_none()).map(|r| (r.type_id, r.name.clone())).collect();
        (rows, options)
    };
    options.sort_by(|a, b| a.1.cmp(&b.1));

    let singles = rows.iter().filter(|r| r.partner.is_none()).count();
    ui.label(
        RichText::new(format!(
            "{} unlocked pets · {singles} single. Partners are mutual; pick from single pets (or \
             self). \u{201c}Assign all\u{201d} pairs the rest at once.",
            rows.len()
        ))
        .color(style::TEXT_MUTED)
        .size(11.0),
    );

    if let Some((msg, err)) = &st.status {
        let color = if *err { style::ERROR } else { style::SUCCESS };
        ui.label(RichText::new(msg).color(color).size(12.0));
    }

    ui.horizontal(|ui| {
        if ui
            .button("➕ Assign all partners")
            .on_hover_text("Randomly pair every single pet (odd one out partners itself)")
            .clicked()
        {
            st.status = Some(match session.assign_all_partners() {
                Ok(n) => (format!("Assigned {n} pet(s)"), false),
                Err(e) => (format!("Assign failed: {e}"), true),
            });
        }
        ui.separator();
        ui.label(RichText::new("Filter:").color(style::TEXT_MUTED));
        ui.add(egui::TextEdit::singleline(&mut st.filter).desired_width(160.0));
        if ui.button("× clear").clicked() {
            st.filter.clear();
        }
    });

    let needle = st.filter.trim().to_lowercase();
    let filtered: Vec<usize> = (0..rows.len())
        .filter(|&i| needle.is_empty() || rows[i].name.to_lowercase().contains(&needle))
        .collect();

    // Collected actions, applied after the render (one partner change + any days edits).
    let mut partner_action: Option<(usize, Option<u32>)> = None;
    let mut days_edits: Vec<(usize, String)> = Vec::new();
    partner_table(ui, st, &rows, &filtered, &options, &mut partner_action, &mut days_edits);

    if let Some((index, choice)) = partner_action {
        let pet = rows.iter().find(|r| r.index == index).map(|r| r.name.clone()).unwrap_or_default();
        let res = match choice {
            None => session.clear_partner(index),
            Some(type_id) => session.set_partner(index, type_id),
        };
        st.status = Some(match res {
            Ok(()) => (format!("Set {pet}'s partner"), false),
            Err(e) => (format!("Edit failed: {e}"), true),
        });
    }
    for (index, value) in days_edits {
        if let Err(e) = session.set_scalar(&["X", "b", &index.to_string(), "G"], "partner days", &value) {
            st.status = Some((format!("Edit failed: {e}"), true));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn partner_table(
    ui: &mut egui::Ui,
    st: &mut PartnerEditState,
    rows: &[Row],
    filtered: &[usize],
    options: &[(u32, String)],
    action: &mut Option<(usize, Option<u32>)>,
    days_edits: &mut Vec<(usize, String)>,
) {
    TableBuilder::new(ui)
        .striped(true)
        .id_salt("partners")
        .column(Column::initial(180.0)) // pet
        .column(Column::initial(220.0)) // partner
        .column(Column::remainder()) // days
        .header(20.0, |mut h| {
            for t in ["Pet", "Partner", "Days"] {
                h.col(|ui| {
                    ui.label(RichText::new(t).strong().size(12.0));
                });
            }
        })
        .body(|body| {
            body.rows(24.0, filtered.len(), |mut tr| {
                let row = &rows[filtered[tr.index()]];
                tr.col(|ui| {
                    ui.label(&row.name).on_hover_text(format!("type id {}", row.type_id));
                });
                tr.col(|ui| {
                    let mut sel = row.partner;
                    let selected_text = match row.partner {
                        None => "None".to_string(),
                        Some(t) if t == row.type_id => format!("{} (self)", row.name),
                        Some(t) => pet_name(t),
                    };
                    egui::ComboBox::from_id_salt(("partner", row.index))
                        .selected_text(selected_text)
                        .width(200.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut sel, None, "None");
                            ui.selectable_value(&mut sel, Some(row.type_id), format!("{} (self)", row.name));
                            for (tid, name) in options {
                                if *tid != row.type_id {
                                    ui.selectable_value(&mut sel, Some(*tid), name);
                                }
                            }
                        });
                    if sel != row.partner {
                        *action = Some((row.index, sel));
                    }
                });
                tr.col(|ui| {
                    let buf = st.days_buffers.entry(row.index).or_insert_with(|| row.days.clone());
                    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(80.0));
                    if resp.changed() {
                        let v = buf.trim();
                        if v.parse::<u64>().is_ok() && v != row.days.trim() {
                            days_edits.push((row.index, v.to_string()));
                        }
                    } else if !resp.has_focus() && buf.trim() != row.days.trim() {
                        *buf = row.days.clone();
                    }
                });
            });
        });
}
