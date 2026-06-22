//! Afky God section: the single struct at `X.t` (the AFK god that fires at clones
//! for idle exp). Edits the seven player-confirmed scalars — power, firing speed,
//! clone HP, clone count, experience, clones killed, exp multiplier — driven off
//! the `AfkyGodField` descriptors so labels/paths/kinds stay in one place.
//!
//! No typed model for this struct; values read straight from the raw tree and
//! stage via `set_scalar` by path. Most fields are BigDoubles (validated as a
//! finite number, written verbatim so decimals / scientific notation survive);
//! the two counts validate as non-negative integers.

use std::collections::HashMap;

use eframe::egui::{self, RichText};
use save_parser::labels::{AfkyGodField, FieldKind};

use crate::style;
use crate::views::save_editor::session::EditSession;

#[derive(Default)]
pub struct AfkyGodEditState {
    /// Per-field buffers keyed by the field's dotted raw path.
    buffers: HashMap<String, String>,
    status: Option<(String, bool)>,
}

pub fn show(ui: &mut egui::Ui, session: &mut EditSession, st: &mut AfkyGodEditState) {
    ui.heading("Afky God");

    if session.root().get_path(&["X", "t"]).is_none() {
        ui.label(RichText::new("No Afky God data in this save.").color(style::TEXT_MUTED));
        return;
    }

    ui.label(
        RichText::new("The AFK god that fires at clones for idle experience.")
            .color(style::TEXT_MUTED)
            .size(11.0),
    );
    ui.separator();

    if let Some((msg, err)) = &st.status {
        let color = if *err { style::ERROR } else { style::SUCCESS };
        ui.label(RichText::new(msg).color(color).size(12.0));
    }

    let mut edits: Vec<(Vec<String>, String, String)> = Vec::new();
    egui::Grid::new("afky_god_fields").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        for field in AfkyGodField::ALL {
            // Build the full raw path: X.t + the field's (possibly dotted) key.
            let mut path: Vec<&str> = vec!["X", "t"];
            path.extend(field.key().split('.'));
            scalar_cell(ui, session, st, &path, field.label(), field.kind(), &mut edits);
            ui.end_row();
        }
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
        st.status = Some(("Staged Afky God edit".to_string(), false));
    }
}

/// A labeled, validated editable scalar. `UInt` fields accept a non-negative
/// integer; everything else accepts any finite number (BigDouble, written
/// verbatim). Stages into `edits` on a valid, changed value.
fn scalar_cell(
    ui: &mut egui::Ui,
    session: &EditSession,
    st: &mut AfkyGodEditState,
    path: &[&str],
    label: &str,
    kind: FieldKind,
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    ui.label(label);
    let key = path.join(".");
    let current = session.value(path).unwrap_or_default();
    let buf = st.buffers.entry(key).or_insert_with(|| current.clone());
    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(180.0));
    if resp.lost_focus() {
        let v = buf.trim().to_string();
        let valid = match kind {
            FieldKind::UInt => v.parse::<u64>().is_ok(),
            _ => v.parse::<f64>().is_ok_and(f64::is_finite),
        };
        if v != current && valid {
            edits.push((path.iter().map(|s| s.to_string()).collect(), label.to_string(), v));
        }
    } else if !resp.has_focus() && buf.as_str() != current {
        *buf = current;
    }
}
