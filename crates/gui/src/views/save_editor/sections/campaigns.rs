//! Campaigns section: view the campaign slots (`X.x`, one per campaign type) and
//! adjust an active campaign's progress.
//!
//! A campaign runs its assigned pets for a target duration (`e`, typically 12 h);
//! elapsed time (`c`) counts up to it and the campaign completes when `c` ≥ `e`
//! (the universal elapsed-timer pattern). This section lets you **force-complete**
//! an active campaign (set elapsed = target) or set elapsed directly, so rewards
//! are ready to collect on next load.
//!
//! The campaign *type* (`a`) is the slot's identity, not an editable field, so
//! it's shown read-only. Pet assignments (`d`) are display-only here — edit a
//! pet's team/campaign elsewhere. Reads the typed `derived().campaigns` (in `X.x`
//! order, so the Vec index is the raw index); edits stage `X.x.<i>.c`.

use std::collections::HashMap;

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};

use crate::style;
use crate::views::save_editor::session::EditSession;

/// Owned, render-ready snapshot of one campaign slot.
struct CampRow {
    index: usize,
    type_id: u32,
    type_name: String,
    elapsed_ms: f64,
    duration_ms: u64,
    bonus: u64,
    pets: Vec<String>,
    /// A slot with a non-zero target duration is running a campaign.
    active: bool,
}

#[derive(Default)]
pub struct CampaignEditState {
    /// Per-row elapsed-edit buffer (keyed by list index).
    cell_buffers: HashMap<usize, String>,
    status: Option<(String, bool)>,
}

/// Format a millisecond span as hours (e.g. "11.5h").
fn fmt_hours(ms: f64) -> String {
    format!("{:.1}h", ms / 3_600_000.0)
}

/// Completion percentage (0–100), guarding against a zero/!finite target.
fn percent(elapsed_ms: f64, duration_ms: u64) -> f64 {
    if duration_ms == 0 {
        0.0
    } else {
        (elapsed_ms / duration_ms as f64 * 100.0).clamp(0.0, 100.0)
    }
}

pub fn show(ui: &mut egui::Ui, session: &mut EditSession, st: &mut CampaignEditState) {
    ui.heading("Campaigns");

    // Owned snapshot, so the session borrow is free for edits below.
    let Some(save) = session.derived() else {
        ui.label(
            RichText::new("Typed campaign data unavailable for this save — use the Raw Save Tree.")
                .color(style::TEXT_MUTED),
        );
        return;
    };
    let rows: Vec<CampRow> = save
        .campaigns
        .iter()
        .enumerate()
        .map(|(index, c)| {
            let pets = c
                .pet_type_ids
                .iter()
                .map(|id| {
                    save.pet_by_type_id(*id)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| format!("type {id}"))
                })
                .collect();
            CampRow {
                index,
                type_id: c.campaign_type_id,
                type_name: c
                    .campaign_type_name()
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("Type {}", c.campaign_type_id)),
                elapsed_ms: c.elapsed_ms,
                duration_ms: c.duration_ms,
                bonus: c.bonus,
                pets,
                active: c.duration_ms > 0,
            }
        })
        .collect();

    ui.label(
        RichText::new(
            "A campaign completes when elapsed ≥ target. Force-complete an active campaign \
             (or set its elapsed directly) so rewards are ready on next load. The campaign \
             type is the slot's identity, not editable here.",
        )
        .color(style::TEXT_MUTED)
        .size(11.0),
    );
    ui.separator();

    if let Some((msg, err)) = &st.status {
        let color = if *err { style::ERROR } else { style::SUCCESS };
        ui.label(RichText::new(msg).color(color).size(12.0));
    }

    if rows.is_empty() {
        ui.label(RichText::new("No campaign slots in this save.").color(style::TEXT_MUTED));
        return;
    }

    table(ui, session, st, &rows);
}

fn table(ui: &mut egui::Ui, session: &mut EditSession, st: &mut CampaignEditState, rows: &[CampRow]) {
    // A staged elapsed edit: (list index, new value, campaign type name for label).
    let mut edit: Option<(usize, String, String)> = None;
    TableBuilder::new(ui)
        .striped(true)
        .column(Column::initial(120.0)) // type
        .column(Column::initial(120.0)) // elapsed (editable)
        .column(Column::initial(110.0)) // target
        .column(Column::initial(70.0)) // percent
        .column(Column::initial(110.0)) // force-complete
        .column(Column::remainder()) // pets
        .header(20.0, |mut h| {
            for title in ["Campaign", "Elapsed (ms)", "Target", "%", "", "Pets"] {
                h.col(|ui| {
                    ui.label(RichText::new(title).strong().size(12.0));
                });
            }
        })
        .body(|body| {
            body.rows(24.0, rows.len(), |mut tr| {
                let row = &rows[tr.index()];
                tr.col(|ui| {
                    ui.label(&row.type_name)
                        .on_hover_text(format!("type id {} · bonus {}", row.type_id, row.bonus));
                });
                // Elapsed: editable for active slots, "—" otherwise.
                tr.col(|ui| {
                    if !row.active {
                        ui.label(RichText::new("inactive").color(style::TEXT_MUTED).size(11.0));
                        return;
                    }
                    let current = format!("{}", row.elapsed_ms);
                    let buf = st.cell_buffers.entry(row.index).or_insert_with(|| current.clone());
                    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(104.0));
                    if resp.changed() {
                        let v = buf.trim();
                        if v.parse::<f64>().is_ok() && v != current.trim() {
                            edit = Some((row.index, v.to_string(), row.type_name.clone()));
                        }
                    } else if !resp.has_focus() && buf.trim() != current.trim() {
                        // Re-seed an idle cell that drifted (e.g. force-complete or undo).
                        *buf = current;
                    }
                });
                tr.col(|ui| {
                    if row.active {
                        ui.label(
                            RichText::new(format!("{} ({})", row.duration_ms, fmt_hours(row.duration_ms as f64)))
                                .monospace()
                                .size(11.0),
                        );
                    } else {
                        ui.label(RichText::new("—").color(style::TEXT_MUTED));
                    }
                });
                tr.col(|ui| {
                    if row.active {
                        ui.label(format!("{:.0}%", percent(row.elapsed_ms, row.duration_ms)));
                    }
                });
                tr.col(|ui| {
                    if row.active
                        && ui
                            .button("Force complete")
                            .on_hover_text("Set elapsed = target so the campaign is ready to collect")
                            .clicked()
                    {
                        edit = Some((row.index, row.duration_ms.to_string(), row.type_name.clone()));
                    }
                });
                tr.col(|ui| {
                    if row.pets.is_empty() {
                        ui.label(RichText::new("—").color(style::TEXT_MUTED));
                    } else {
                        let summary = row.pets.join(", ");
                        ui.label(RichText::new(&summary).size(11.0)).on_hover_text(&summary);
                    }
                });
            });
        });

    if let Some((index, value, type_name)) = edit {
        let label = format!("{type_name} campaign elapsed");
        // Keep the edit buffer in step so the cell shows the staged value.
        st.cell_buffers.insert(index, value.clone());
        if let Err(e) = session.set_scalar(&["X", "x", &index.to_string(), "c"], label, &value) {
            st.status = Some((format!("Edit failed: {e}"), true));
        } else {
            st.status = Some(("Staged campaign elapsed".to_string(), false));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_hours_and_percent() {
        assert_eq!(fmt_hours(43_200_000.0), "12.0h");
        assert_eq!(fmt_hours(0.0), "0.0h");
        // ~95.6% of a 12 h target.
        let p = percent(41_300_000.0, 43_200_000);
        assert!((p - 95.6).abs() < 0.1, "{p}");
        // Zero target never divides by zero, and over-target clamps to 100.
        assert_eq!(percent(100.0, 0), 0.0);
        assert_eq!(percent(99_999_999.0, 43_200_000), 100.0);
    }
}
