//! The raw save tree navigator: a searchable, name-labeled view of the lossless
//! tree. Editing a scalar leaf stages a change through the same [`EditSession`]
//! as the structured sections, so the two views share one pending log and never
//! drift.
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

pub fn show(
    ui: &mut egui::Ui,
    session: &mut EditSession,
    registry: &FieldRegistry,
    buffers: &mut HashMap<String, String>,
    search: &mut String,
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
    let mut edits: Vec<StagedEdit> = Vec::new();
    {
        let mut walk = Walk {
            registry,
            buffers,
            edits: &mut edits,
            query: &query,
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
    query: &'a str,
}

impl Walk<'_> {
    /// The registry display-name for the current path, if known.
    fn known_name(&self, path: &[String]) -> Option<&'static str> {
        let p: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
        self.registry.lookup(&p).map(|d| d.name)
    }

    /// Render one struct field (a key and its value).
    fn render_field(&mut self, ui: &mut egui::Ui, path: &mut Vec<String>, key: &str, field: &Field) {
        path.push(key.to_string());
        match field {
            Field::EmptyColon | Field::EmptyBare => {
                if self.query.is_empty() || key.to_lowercase().contains(self.query) {
                    let label = NodeName::Key(key);
                    self.empty_row(ui, path, &label);
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
                if self.scalar_visible(path, &name, s) {
                    self.scalar_row(ui, path, &name, s);
                }
            }
            Raw::Struct(fields) => {
                if !self.query.is_empty() && !subtree_matches(value.peel(), self.query) {
                    return;
                }
                let summary = format!("{{{} fields}}", fields.len());
                self.container(ui, path, &name, summary, |w, path, ui| {
                    for (k, f) in fields {
                        w.render_field(ui, path, k, f);
                    }
                });
            }
            Raw::List(items) => {
                if !self.query.is_empty() && !subtree_matches(value.peel(), self.query) {
                    return;
                }
                let summary = format!("[{} items]", items.len());
                self.container(ui, path, &name, summary, |w, path, ui| {
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

    fn scalar_visible(&self, path: &[String], name: &NodeName, value: &str) -> bool {
        if self.query.is_empty() {
            return true;
        }
        let q = self.query;
        name.display().to_lowercase().contains(q)
            || value.to_lowercase().contains(q)
            || self
                .known_name(path)
                .is_some_and(|n| n.to_lowercase().contains(q))
    }

    /// A collapsing container header with a name label and a summary count.
    fn container(
        &mut self,
        ui: &mut egui::Ui,
        path: &mut Vec<String>,
        name: &NodeName,
        summary: String,
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
        let mut header = egui::CollapsingHeader::new(RichText::new(title).color(color))
            .id_salt(path.join("."));
        if !self.query.is_empty() {
            header = header.default_open(true);
        }
        header.show(ui, |ui| build(self, path, ui));
    }

    /// An editable scalar leaf row.
    fn scalar_row(&mut self, ui: &mut egui::Ui, path: &[String], name: &NodeName, current: &str) {
        let known = self.known_name(path).map(|s| s.to_string());
        let key = path.join(".");
        ui.horizontal(|ui| {
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
                    ui.label(
                        RichText::new(name.display())
                            .color(style::TEXT_NORMAL)
                            .monospace(),
                    );
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
            if let Some(v) = newval {
                let label = known.unwrap_or_else(|| path.join("."));
                self.edits.push((path.to_vec(), label, v));
            }
        });
    }

    fn empty_row(&self, ui: &mut egui::Ui, _path: &[String], name: &NodeName) {
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

/// Does any key, scalar value, anywhere in this subtree, contain `q`?
fn subtree_matches(node: &Raw, q: &str) -> bool {
    match node.peel() {
        Raw::Scalar(s) => s.to_lowercase().contains(q),
        Raw::Struct(fields) => fields.iter().any(|(k, f)| {
            k.to_lowercase().contains(q)
                || matches!(f, Field::Value(v) if subtree_matches(v, q))
        }),
        Raw::List(items) => items.iter().any(|it| subtree_matches(it, q)),
        Raw::Base64(_) => false,
    }
}
