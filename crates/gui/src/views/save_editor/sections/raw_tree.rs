//! The raw save tree navigator: a searchable, name-labeled view of the lossless
//! tree. Editing a scalar leaf stages a change through the same [`EditSession`]
//! as the structured sections, so the two views share one pending log and never
//! drift.
//!
//! Search has two modes. **Filter** (default) hides everything that doesn't
//! match. **Reveal in place** keeps the whole tree visible but expands the path
//! to each match and scrolls to the first — useful for poking at values near a
//! known field. Matching considers raw keys, scalar values, *and* the registry
//! display name at each node, so searching "Shadow Clones" finds the labeled
//! field, not just the literal string elsewhere.
//!
//! Edits are collected during the (immutable) walk and applied afterwards, so we
//! never hold a `&Raw` borrow of the tree while calling `&mut` `set_scalar`.

use std::collections::HashMap;

use eframe::egui::{self, RichText};
use save_parser::raw::{Field, Raw};

use crate::style;
use crate::views::save_editor::registry::FieldRegistry;
use crate::views::save_editor::session::EditSession;

/// A staged edit gathered during the walk: (path, label, new value).
type StagedEdit = (Vec<String>, String, String);

/// How an active search affects the tree.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// No active query.
    None,
    /// Hide everything that doesn't match.
    Filter,
    /// Keep the whole tree; expand the path to matches and scroll to the first.
    Reveal,
}

pub fn show(
    ui: &mut egui::Ui,
    session: &mut EditSession,
    registry: &FieldRegistry,
    buffers: &mut HashMap<String, String>,
    search: &mut String,
    reveal: &mut bool,
) {
    ui.heading("Raw Save Tree");
    ui.horizontal(|ui| {
        ui.label("Search:");
        ui.add(
            egui::TextEdit::singleline(search)
                .desired_width(240.0)
                .hint_text("key, field name, or value"),
        );
        if ui.button("Clear").clicked() {
            search.clear();
        }
        ui.separator();
        ui.checkbox(reveal, "Reveal in place").on_hover_text(
            "Jump to matches and expand the path to them, without hiding the rest of the tree.",
        );
    });
    ui.label(
        RichText::new(
            "Known fields show their name. Editing a scalar stages a change in the \
             pending list; values are written verbatim.",
        )
        .color(style::TEXT_MUTED)
        .size(11.0),
    );
    ui.separator();

    let query = search.trim().to_lowercase();
    let mode = if query.is_empty() {
        Mode::None
    } else if *reveal {
        Mode::Reveal
    } else {
        Mode::Filter
    };

    let mut edits: Vec<StagedEdit> = Vec::new();
    let mut scrolled = false;
    {
        let mut walk = Walk {
            registry,
            buffers,
            edits: &mut edits,
            query: &query,
            mode,
            scrolled: &mut scrolled,
        };
        let mut path: Vec<String> = Vec::new();
        if let Raw::Struct(fields) = session.root().peel() {
            for (key, field) in fields {
                walk.render_field(ui, &mut path, key, field);
            }
        }
    }

    // Apply staged edits now that the read-only borrow of the tree is released.
    for (path, label, value) in edits {
        let p: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
        let _ = session.set_scalar(&p, label, &value);
    }
}

/// How a node is named under its parent: a struct key or a list index.
enum NodeName<'a> {
    Key(&'a str),
    Index(usize),
}

impl NodeName<'_> {
    fn display(&self) -> String {
        match self {
            NodeName::Key(k) => (*k).to_string(),
            NodeName::Index(i) => format!("[{i}]"),
        }
    }
}

/// Mutable walk context (kept separate from the borrowed `&Raw` tree).
struct Walk<'a> {
    registry: &'a FieldRegistry,
    buffers: &'a mut HashMap<String, String>,
    edits: &'a mut Vec<StagedEdit>,
    /// Lowercased search query (empty when `mode == None`).
    query: &'a str,
    mode: Mode,
    /// Whether we've already scrolled to a match this frame (Reveal mode).
    scrolled: &'a mut bool,
}

impl Walk<'_> {
    /// The registry display-name for the current path, if known.
    fn known_name(&self, path: &[String]) -> Option<&'static str> {
        let p: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
        self.registry.lookup(&p).map(|d| d.name)
    }

    fn name_matches(&self, path: &[String]) -> bool {
        self.known_name(path)
            .is_some_and(|n| n.to_lowercase().contains(self.query))
    }

    /// Render one struct field (a key and its value).
    fn render_field(&mut self, ui: &mut egui::Ui, path: &mut Vec<String>, key: &str, field: &Field) {
        path.push(key.to_string());
        match field {
            Field::EmptyColon | Field::EmptyBare => {
                let show = self.mode != Mode::Filter
                    || key.to_lowercase().contains(self.query)
                    || self.name_matches(path);
                if show {
                    self.empty_row(ui, &NodeName::Key(key));
                }
            }
            Field::Value(v) => self.render_value(ui, path, NodeName::Key(key), v),
        }
        path.pop();
    }

    /// Render a value (scalar leaf, or a struct/list container).
    fn render_value(&mut self, ui: &mut egui::Ui, path: &mut Vec<String>, name: NodeName, value: &Raw) {
        match value.peel() {
            Raw::Scalar(s) => {
                let is_match = self.mode != Mode::None && self.scalar_matches(path, &name, s);
                let visible = self.mode != Mode::Filter || is_match;
                if visible {
                    self.scalar_row(ui, path, &name, s, is_match);
                }
            }
            Raw::Struct(fields) => {
                let force_open = self.container_open(value.peel(), path);
                if self.mode == Mode::Filter && force_open != Some(true) {
                    return;
                }
                let summary = format!("{{{} fields}}", fields.len());
                self.container(ui, path, &name, summary, force_open, |w, path, ui| {
                    for (k, f) in fields {
                        w.render_field(ui, path, k, f);
                    }
                });
            }
            Raw::List(items) => {
                let force_open = self.container_open(value.peel(), path);
                if self.mode == Mode::Filter && force_open != Some(true) {
                    return;
                }
                let summary = format!("[{} items]", items.len());
                self.container(ui, path, &name, summary, force_open, |w, path, ui| {
                    for (i, item) in items.iter().enumerate() {
                        path.push(i.to_string());
                        w.render_value(ui, path, NodeName::Index(i), item);
                        path.pop();
                    }
                });
            }
            // peel() already removed any Base64 wrapper.
            Raw::Base64(_) => {}
        }
    }

    /// Whether (and how) to force a container open: `Some(true)` when an active
    /// search has a match inside, else `None` (use the stored open state).
    fn container_open(&self, node: &Raw, path: &mut Vec<String>) -> Option<bool> {
        if self.mode == Mode::None {
            return None;
        }
        if self.subtree_matches(node, path) {
            Some(true)
        } else {
            None
        }
    }

    /// Does this scalar leaf match the query (by name, key, or value)?
    fn scalar_matches(&self, path: &[String], name: &NodeName, value: &str) -> bool {
        let q = self.query;
        name.display().to_lowercase().contains(q)
            || value.to_lowercase().contains(q)
            || self.name_matches(path)
    }

    /// Does any name, key, or scalar value anywhere in this subtree match?
    fn subtree_matches(&self, node: &Raw, path: &mut Vec<String>) -> bool {
        if self.name_matches(path) {
            return true;
        }
        match node.peel() {
            Raw::Scalar(s) => s.to_lowercase().contains(self.query),
            Raw::Struct(fields) => fields.iter().any(|(k, f)| {
                if k.to_lowercase().contains(self.query) {
                    return true;
                }
                match f {
                    Field::Value(v) => {
                        path.push(k.clone());
                        let m = self.subtree_matches(v, path);
                        path.pop();
                        m
                    }
                    _ => false,
                }
            }),
            Raw::List(items) => items.iter().enumerate().any(|(i, it)| {
                path.push(i.to_string());
                let m = self.subtree_matches(it, path);
                path.pop();
                m
            }),
            Raw::Base64(_) => false,
        }
    }

    /// A collapsing container header with a name label and a summary count.
    fn container(
        &mut self,
        ui: &mut egui::Ui,
        path: &mut Vec<String>,
        name: &NodeName,
        summary: String,
        force_open: Option<bool>,
        build: impl FnOnce(&mut Walk, &mut Vec<String>, &mut egui::Ui),
    ) {
        let known = self.known_name(path);
        let title = match known {
            Some(n) => format!("{}  ·  {}   {}", n, name.display(), summary),
            None => format!("{}   {}", name.display(), summary),
        };
        let color = if known.is_some() {
            style::ACCENT
        } else {
            style::TEXT_NORMAL
        };
        egui::CollapsingHeader::new(RichText::new(title).color(color))
            .id_salt(path.join("."))
            .open(force_open)
            .show(ui, |ui| build(self, path, ui));
    }

    /// An editable scalar leaf row. `is_match` highlights it and, in Reveal mode,
    /// scrolls the first match into view.
    fn scalar_row(
        &mut self,
        ui: &mut egui::Ui,
        path: &[String],
        name: &NodeName,
        current: &str,
        is_match: bool,
    ) {
        let known = self.known_name(path).map(|s| s.to_string());
        let key = path.join(".");
        let row = ui.horizontal(|ui| {
            match &known {
                Some(n) => {
                    ui.label(RichText::new(n).color(style::ACCENT).strong());
                    ui.label(
                        RichText::new(format!("· {}", name.display()))
                            .color(style::TEXT_MUTED)
                            .monospace(),
                    );
                }
                None => {
                    let color = if is_match {
                        style::WARNING
                    } else {
                        style::TEXT_NORMAL
                    };
                    ui.label(RichText::new(name.display()).color(color).monospace());
                }
            }

            let mut newval: Option<String> = None;
            {
                let buf = self.buffers.entry(key).or_insert_with(|| current.to_string());
                let resp = ui.add(
                    egui::TextEdit::singleline(buf)
                        .desired_width(260.0)
                        .font(egui::TextStyle::Monospace),
                );
                if resp.lost_focus() {
                    let t = buf.trim().to_string();
                    if t != current {
                        buf.clone_from(&t);
                        newval = Some(t);
                    }
                } else if !resp.has_focus() && buf.as_str() != current {
                    // Mirror changes made elsewhere (structured sections, undo).
                    *buf = current.to_string();
                }
            }
            if is_match {
                ui.label(RichText::new("◀").color(style::WARNING).small());
            }
            if let Some(v) = newval {
                let label = known.unwrap_or_else(|| path.join("."));
                self.edits.push((path.to_vec(), label, v));
            }
        });

        // Bring the first match into view in Reveal mode.
        if is_match && self.mode == Mode::Reveal && !*self.scrolled {
            row.response.scroll_to_me(Some(egui::Align::Center));
            *self.scrolled = true;
        }
    }

    fn empty_row(&self, ui: &mut egui::Ui, name: &NodeName) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(name.display())
                    .color(style::TEXT_MUTED)
                    .monospace(),
            );
            ui.label(RichText::new("(empty)").color(style::TEXT_MUTED).size(11.0));
        });
    }
}
