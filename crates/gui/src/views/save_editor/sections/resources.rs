//! Resources & currencies — the first structured section. Flat scalar fields
//! (god power, pet stones, divinity, baal power, …) driven entirely by the field
//! registry, so it grows by adding registry entries, not code here.

use std::collections::HashMap;

use eframe::egui::{self, RichText};

use crate::style;
use crate::views::save_editor::registry::{FieldKind, FieldRegistry, SectionId};
use crate::views::save_editor::session::EditSession;

/// `buffers` is the shared per-path text-edit buffer map (keyed by dotted path).
pub fn show(
    ui: &mut egui::Ui,
    session: &mut EditSession,
    registry: &FieldRegistry,
    buffers: &mut HashMap<String, String>,
) {
    ui.heading(SectionId::Resources.title());
    ui.label(
        RichText::new(
            "Large counts are edited as text and written verbatim, so 17-digit \
             values and scientific-notation doubles keep full precision.",
        )
        .color(style::TEXT_MUTED)
        .size(11.0),
    );
    ui.add_space(6.0);

    egui::Grid::new("resources_grid")
        .num_columns(2)
        .spacing([16.0, 6.0])
        .striped(true)
        .show(ui, |ui| {
            for field in registry.for_section(SectionId::Resources) {
                let key = field.path.join(".");
                let current = session.value(field.path);

                // Label column (with the help text as a hover tooltip).
                let mut label = ui.label(RichText::new(field.name).color(style::TEXT_BRIGHT));
                if let Some(help) = field.help {
                    label = label.on_hover_text(help);
                }
                let _ = label;

                // Editor column.
                match current {
                    None => {
                        ui.label(RichText::new("— (absent in this save)").color(style::TEXT_MUTED));
                    }
                    Some(current) => match field.kind {
                        FieldKind::Bool => bool_editor(ui, session, field.path, field.name, &current),
                        FieldKind::Number | FieldKind::Text => text_editor(
                            ui,
                            session,
                            field.path,
                            field.name,
                            field.kind,
                            &current,
                            buffers.entry(key).or_insert_with(|| current.clone()),
                        ),
                    },
                }
                ui.end_row();
            }
        });
}

/// Uniform width for a resource input when it isn't being edited.
const FIELD_WIDTH: f32 = 220.0;

/// A validated text editor for a numeric/text scalar. Commits on focus-out when
/// the value is valid (for `Number`, parseable); reverts otherwise. The box is a
/// uniform width at rest and grows to fit the value while focused, so long
/// values are fully visible when you're editing them.
#[allow(clippy::too_many_arguments)]
fn text_editor(
    ui: &mut egui::Ui,
    session: &mut EditSession,
    path: &[&str],
    name: &str,
    kind: FieldKind,
    current: &str,
    buf: &mut String,
) {
    let id = egui::Id::new(("save_editor_resource", path.join(".")));
    let focused = ui.memory(|m| m.has_focus(id));
    let width = if focused {
        // ~8px per monospace char, clamped so it never gets absurd.
        ((buf.chars().count() as f32 + 2.0) * 8.0).clamp(FIELD_WIDTH, 540.0)
    } else {
        FIELD_WIDTH
    };
    let resp = ui.add(
        egui::TextEdit::singleline(buf)
            .id(id)
            .desired_width(width)
            .font(egui::TextStyle::Monospace),
    );

    let mut committed = false;
    if resp.lost_focus() {
        let trimmed = buf.trim().to_string();
        if trimmed != current {
            let valid = match kind {
                FieldKind::Number => itrtg_models::parse_flexible_number(&trimmed).is_some(),
                _ => !trimmed.is_empty(),
            };
            if valid {
                let _ = session.set_scalar(path, name, &trimmed);
                buf.clone_from(&trimmed);
                committed = true;
            } else {
                // Reject invalid input: snap back to the canonical value.
                *buf = current.to_string();
            }
        }
    }

    // When not being edited, mirror the canonical value so changes made
    // elsewhere (e.g. the raw tree navigator) show up here too.
    if !committed && !resp.has_focus() && buf != current {
        *buf = current.to_string();
    }
}

/// A `True`/`False` checkbox.
fn bool_editor(
    ui: &mut egui::Ui,
    session: &mut EditSession,
    path: &[&str],
    name: &str,
    current: &str,
) {
    let mut on = current.eq_ignore_ascii_case("true");
    if ui.checkbox(&mut on, "").changed() {
        let _ = session.set_scalar(path, name, if on { "True" } else { "False" });
    }
}
