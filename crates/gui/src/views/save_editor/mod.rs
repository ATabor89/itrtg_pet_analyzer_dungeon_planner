//! Save editor view.
//!
//! Loads a raw ITRTG save into an [`EditSession`] (the single mutable source of
//! truth — the lossless tree) and presents it through structured sections and a
//! raw tree navigator. Both read and write the same session, so they can't
//! drift. Edits are staged as a pending list and written to a *new* file; the
//! loaded save is never overwritten unless the user explicitly picks it.
//!
//! The session holds the player's real identity fields in memory, so it is
//! deliberately **not** part of the persisted `AppState` — nothing about a
//! loaded save reaches `app_state.yaml`.

mod registry;
mod session;
mod sections;

use std::collections::HashMap;

use eframe::egui::{self, RichText};

use crate::style;
use registry::{FieldRegistry, SectionId};
use session::EditSession;
use sections::{raw_tree, resources};

#[derive(Default)]
pub struct SaveEditorState {
    session: Option<EditSession>,
    registry: FieldRegistry,
    current: SectionId,
    tree_search: String,
    /// Raw tree search mode: reveal-in-place (true) vs filter (false).
    tree_reveal: bool,
    /// The query we last auto-scrolled to in Reveal mode, so we scroll once per
    /// query rather than yanking the viewport back every frame.
    tree_scrolled_query: Option<String>,
    /// Browse-mode collapsing-id generation; "Collapse all" bumps it.
    tree_generation: u64,
    /// Shared per-path text-edit buffers (dotted path → in-progress text),
    /// used by every section so edits keep their cursor across frames. Assumes
    /// one editor per path per frame (only one section renders at a time).
    buffers: HashMap<String, String>,
    status: Option<(String, bool)>,
}

impl SaveEditorState {
    /// Install a freshly-loaded session, clearing buffers from any prior save.
    fn set_session(&mut self, session: EditSession) {
        let name = session.source_name.clone();
        self.session = Some(session);
        self.buffers.clear();
        self.status = Some((
            match name {
                Some(n) => format!("Loaded {n}"),
                None => "Save loaded".to_string(),
            },
            false,
        ));
    }

    /// Attempt to load raw save text. Returns `true` if it parsed as a save —
    /// used by the drag-drop router in `app.rs` to decide whether a dropped file
    /// was a save. Leaves any existing session untouched on failure.
    pub fn try_load(&mut self, text: &str, source_name: Option<String>) -> bool {
        match EditSession::load(text, source_name) {
            Ok(s) => {
                self.set_session(s);
                true
            }
            Err(_) => false,
        }
    }
}

pub fn show(ui: &mut egui::Ui, state: &mut SaveEditorState) {
    if let Some(s) = state.session.as_mut() {
        s.rederive_if_needed();
    }

    header_bar(ui, state);
    if let Some((msg, err)) = &state.status {
        let color = if *err { style::ERROR } else { style::SUCCESS };
        ui.label(RichText::new(msg).color(color).size(11.0));
    }
    ui.separator();

    if state.session.is_none() {
        empty_prompt(ui);
        return;
    }

    // Disjoint borrows of the individual fields for the body.
    let SaveEditorState {
        session,
        registry,
        current,
        tree_search,
        tree_reveal,
        tree_scrolled_query,
        tree_generation,
        buffers,
        ..
    } = state;
    let session = session.as_mut().unwrap();

    pending_panel(ui, session);
    ui.separator();

    ui.horizontal_top(|ui| {
        // Section nav.
        ui.allocate_ui_with_layout(
            egui::vec2(190.0, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.label(RichText::new("SECTIONS").color(style::TEXT_MUTED).size(10.0));
                for &section in SectionId::ALL {
                    if ui
                        .selectable_label(*current == section, section.title())
                        .clicked()
                    {
                        *current = section;
                    }
                }
            },
        );
        ui.separator();
        // Active section content.
        ui.vertical(|ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match *current {
                    SectionId::Resources => resources::show(ui, session, registry, buffers),
                    SectionId::RawTree => raw_tree::show(
                        ui,
                        session,
                        registry,
                        buffers,
                        tree_search,
                        tree_reveal,
                        tree_scrolled_query,
                        tree_generation,
                    ),
                });
        });
    });
}

fn header_bar(ui: &mut egui::Ui, state: &mut SaveEditorState) {
    // Toolbar row: left-aligned so the action buttons are always visible (a
    // long summary row used to squeeze them off the right edge).
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Save Editor")
                .color(style::ACCENT)
                .strong()
                .size(16.0),
        );
        ui.separator();

        let has_session = state.session.is_some();

        #[cfg(not(target_arch = "wasm32"))]
        if ui.button(RichText::new("📂 Load Save…").size(12.0)).clicked() {
            load_from_file(state);
        }

        if has_session {
            if ui
                .button(RichText::new("💾 Save As… (full)").size(12.0))
                .on_hover_text(
                    "Write the full edited save — including your real identity — to a new \
                     file. This is the one that loads back into the game.",
                )
                .clicked()
            {
                save_to_file(state, false);
            }
            if ui
                .button(RichText::new("Save Redacted Copy…").size(12.0))
                .on_hover_text("Write a copy with account identifiers scrubbed — for sharing.")
                .clicked()
            {
                save_to_file(state, true);
            }
        }

        #[cfg(target_arch = "wasm32")]
        if !has_session {
            ui.label(
                RichText::new("Drag a save file onto the window to load.")
                    .color(style::TEXT_MUTED)
                    .size(11.0),
            );
        }
    });

    // Summary row.
    if let Some(s) = state.session.as_ref() {
        ui.horizontal(|ui| {
            if let Some(name) = &s.source_name {
                ui.label(RichText::new(name).color(style::TEXT_BRIGHT));
            }
            ui.label(
                RichText::new(s.format_label())
                    .color(style::TEXT_MUTED)
                    .size(11.0),
            );
            let typed_ok = s.derived().is_some();
            let pets = s
                .derived()
                .map(|d| d.pets.len().to_string())
                .unwrap_or_else(|| "—".into());
            let gp = s.value(&["p", "j"]).unwrap_or_else(|| "—".into());
            let stones = s.value(&["X", "y"]).unwrap_or_else(|| "—".into());
            let summary = ui.label(
                RichText::new(format!("{pets} pets · GP {gp} · stones {stones}"))
                    .color(style::TEXT_MUTED)
                    .size(12.0),
            );
            if !typed_ok {
                summary
                    .on_hover_text("Typed view unavailable for this save; raw editing still works.");
            }
            if s.is_dirty() {
                ui.label(
                    RichText::new(format!("● {} pending", s.pending().len()))
                        .color(style::WARNING),
                );
            }
        });

        ui.label(
            RichText::new(
                "⚠ A loaded save holds your real account identifiers in memory; it is never \
                 written to app state. “Save As” keeps them (so it loads in-game); use \
                 “Save Redacted Copy” to share.",
            )
            .color(style::WARNING)
            .size(11.0),
        );
    }
}

fn empty_prompt(ui: &mut egui::Ui) {
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new("No save loaded")
                .color(style::TEXT_BRIGHT)
                .size(16.0),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new("Drag an ITRTG save file onto the window, or use “Load Save…”.")
                .color(style::TEXT_MUTED),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "Edits are staged as a pending list and written to a new file — your loaded \
                 save is never overwritten unless you pick it.",
            )
            .color(style::TEXT_MUTED)
            .size(11.0),
        );
    });
}

fn pending_panel(ui: &mut egui::Ui, session: &mut EditSession) {
    let n = session.pending().len();
    let color = if n > 0 {
        style::WARNING
    } else {
        style::TEXT_MUTED
    };
    egui::CollapsingHeader::new(RichText::new(format!("Pending changes ({n})")).color(color))
        .id_salt("save_editor_pending")
        .default_open(n > 0 && n <= 12)
        .show(ui, |ui| {
            if n == 0 {
                ui.label(RichText::new("No staged changes.").color(style::TEXT_MUTED));
                return;
            }
            let mut undo: Option<usize> = None;
            egui::Grid::new("save_editor_pending_grid")
                .num_columns(4)
                .spacing([12.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    for (i, e) in session.pending().iter().enumerate() {
                        ui.label(RichText::new(&e.label).color(style::TEXT_BRIGHT));
                        ui.label(
                            RichText::new(e.path.join("."))
                                .color(style::TEXT_MUTED)
                                .monospace()
                                .size(11.0),
                        );
                        ui.label(
                            RichText::new(format!("{} → {}", trunc(&e.original), trunc(&e.new)))
                                .monospace()
                                .size(11.0),
                        );
                        if ui.small_button("undo").clicked() {
                            undo = Some(i);
                        }
                        ui.end_row();
                    }
                });
            if let Some(i) = undo {
                let _ = session.undo(i);
            }
        });
}

/// Truncate a long value for compact display in the pending list.
fn trunc(s: &str) -> String {
    if s.chars().count() > 24 {
        format!("{}…", s.chars().take(23).collect::<String>())
    } else {
        s.to_string()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn load_from_file(state: &mut SaveEditorState) {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("ITRTG save", &["txt"])
        .pick_file()
    else {
        return;
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let name = path.file_name().map(|n| n.to_string_lossy().to_string());
            if !state.try_load(&text, name) {
                state.status = Some(("Not a recognized ITRTG save file".to_string(), true));
            }
        }
        Err(e) => state.status = Some((format!("Read failed: {e}"), true)),
    }
}

/// Encode the (optionally redacted) save, round-trip validate it, and write it
/// out — to a file via the native dialog, or as a browser download on wasm.
fn save_to_file(state: &mut SaveEditorState, redacted: bool) {
    let default_name = if redacted {
        "edited_redacted_save.txt"
    } else {
        "edited_save.txt"
    };

    let status = {
        let session = state.session.as_ref().unwrap();
        let encoded: anyhow::Result<String> = if redacted {
            session.encode_redacted().map(|(enc, _)| enc)
        } else {
            Ok(session.encode())
        };
        match encoded {
            Err(e) => Some((format!("Encode failed: {e}"), true)),
            Ok(enc) => {
                let validated = if redacted {
                    session.validate_encoded_redacted(&enc)
                } else {
                    session.validate_encoded(&enc)
                };
                match validated {
                    Err(e) => Some((format!("Validation failed — not written: {e}"), true)),
                    Ok(()) => output_save(default_name, &enc),
                }
            }
        }
    };
    // `output_save` returns `None` when the user cancels the dialog (leave the
    // previous status untouched).
    if let Some(status) = status {
        state.status = Some(status);
    }
}

/// Native: prompt for a path and write the file. Returns `None` if cancelled.
#[cfg(not(target_arch = "wasm32"))]
fn output_save(default_name: &str, encoded: &str) -> Option<(String, bool)> {
    let path = rfd::FileDialog::new()
        .set_file_name(default_name)
        .save_file()?;
    Some(match std::fs::write(&path, encoded) {
        Ok(()) => (format!("Wrote {}", path.display()), false),
        Err(e) => (format!("Write failed: {e}"), true),
    })
}

/// WASM: trigger a browser download of the encoded save.
#[cfg(target_arch = "wasm32")]
fn output_save(default_name: &str, encoded: &str) -> Option<(String, bool)> {
    Some(match crate::platform::download_text(default_name, encoded) {
        Ok(()) => (format!("Downloaded {default_name}"), false),
        Err(e) => (format!("Download failed: {e}"), true),
    })
}
