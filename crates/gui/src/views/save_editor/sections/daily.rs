//! Daily section: the once-a-day timers, pack counts, and Bonus Points.
//!
//! The three timers (`p.L` free draw, `p.013` bonus pack, `p.S` daily pack) are
//! stored **countdowns in ms** — not wall-clock anchors. They tick down and reset
//! to +24h when claimed, so a value of 0 means "ready now" (the daily pack `p.S`
//! is a signed long, so ≤0 also = ready). The "Set ready" buttons just write that
//! value. Bonus Points are shown on the Daily screen in-game but actually live on
//! the pet object at `X.q`.
//!
//! Values read/stage straight from the raw tree by path; numbers are validated as
//! finite and written verbatim so BigDoubles survive.

use std::collections::HashMap;

use eframe::egui::{self, RichText};

use crate::style;
use crate::views::save_editor::session::EditSession;

#[derive(Default)]
pub struct DailyEditState {
    /// Per-field buffers keyed by the field's dotted raw path.
    buffers: HashMap<String, String>,
    status: Option<(String, bool)>,
}

pub fn show(ui: &mut egui::Ui, session: &mut EditSession, st: &mut DailyEditState) {
    ui.heading("Daily");

    if session.root().get_path(&["p"]).is_none() {
        ui.label(RichText::new("No daily data in this save.").color(style::TEXT_MUTED));
        return;
    }

    ui.label(
        RichText::new(
            "Timers are countdowns in milliseconds — \u{201c}Set ready\u{201d} makes one claimable \
             now; to shift the tick time, set it to (next tick \u{2212} now) in ms.",
        )
        .color(style::TEXT_MUTED)
        .size(11.0),
    );
    ui.separator();

    if let Some((msg, err)) = &st.status {
        let color = if *err { style::ERROR } else { style::SUCCESS };
        ui.label(RichText::new(msg).color(color).size(12.0));
    }

    let mut edits: Vec<(Vec<String>, String, String)> = Vec::new();

    egui::Grid::new("daily_fields").num_columns(3).spacing([12.0, 6.0]).show(ui, |ui| {
        // Free draw + bonus pack timers clamp at 0; the daily pack is signed (<0 =
        // ready), so its "ready" value is -1.
        timer_row(ui, session, st, &["p", "L"], "Free Draw Timer (ms)", "0", &mut edits);
        timer_row(ui, session, st, &["p", "013"], "Bonus Pack Timer (ms)", "0", &mut edits);
        scalar_row(ui, session, st, &["p", "012"], "Bonus Packs Left", &mut edits);
        timer_row(ui, session, st, &["p", "S"], "Daily Pack Timer (ms)", "-1", &mut edits);
        scalar_row(ui, session, st, &["p", "T"], "Daily Packs Left", &mut edits);
        scalar_row(ui, session, st, &["X", "q"], "Bonus Points", &mut edits);
    });

    // Apply after the read borrow is released.
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
        st.status = Some(("Staged daily edit".to_string(), false));
    }
}

/// A timer row: a label, an editable ms cell, and a "Set ready" button that
/// stages `ready_value` (0 for clamped timers, -1 for the signed daily pack).
fn timer_row(
    ui: &mut egui::Ui,
    session: &EditSession,
    st: &mut DailyEditState,
    path: &[&str],
    label: &str,
    ready_value: &str,
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    scalar_cell(ui, session, st, path, label, edits);
    if ui
        .button("Set ready")
        .on_hover_text("Make this claimable now (sets the countdown to ready)")
        .clicked()
    {
        st.buffers.insert(path.join("."), ready_value.to_string());
        edits.push((
            path.iter().map(|s| s.to_string()).collect(),
            label.to_string(),
            ready_value.to_string(),
        ));
    }
    ui.end_row();
}

/// A plain editable scalar row (label + cell, empty 3rd column).
fn scalar_row(
    ui: &mut egui::Ui,
    session: &EditSession,
    st: &mut DailyEditState,
    path: &[&str],
    label: &str,
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    scalar_cell(ui, session, st, path, label, edits);
    ui.label("");
    ui.end_row();
}

/// The label + validated editable cell shared by both row kinds. Stages on a
/// valid (finite number), changed value; reverts the buffer on blur.
fn scalar_cell(
    ui: &mut egui::Ui,
    session: &EditSession,
    st: &mut DailyEditState,
    path: &[&str],
    label: &str,
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    ui.label(label);
    let key = path.join(".");
    let current = session.value(path).unwrap_or_default();
    let buf = st.buffers.entry(key).or_insert_with(|| current.clone());
    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(160.0));
    if resp.lost_focus() {
        let v = buf.trim().to_string();
        if v != current && v.parse::<f64>().is_ok_and(f64::is_finite) {
            edits.push((path.iter().map(|s| s.to_string()).collect(), label.to_string(), v));
        }
    } else if !resp.has_focus() && buf.as_str() != current {
        *buf = current;
    }
}
