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
//! When a search is active we compute the matching/ancestor-of-match node set in
//! a single O(N) pre-pass (`build_matches`) and the render walk just consults it
//! — so typing in the search box doesn't re-walk the subtree at every container.
//!
//! Edits are collected during the (immutable) walk and applied afterwards, so we
//! never hold a `&Raw` borrow of the tree while calling `&mut` `set_scalar`.

use std::collections::{HashMap, HashSet};

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

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    session: &mut EditSession,
    registry: &FieldRegistry,
    buffers: &mut HashMap<String, String>,
    search: &mut String,
    reveal: &mut bool,
    scrolled_query: &mut Option<String>,
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

    // One O(N) pre-pass: which nodes match or are an ancestor of a match.
    let matches = if mode == Mode::None {
        HashSet::new()
    } else {
        build_matches(session.root(), &query, registry)
    };

    // Scroll to the first match once per (query, reveal) — not every frame, or
    // the viewport would snap back and the user could never scroll away.
    if mode != Mode::Reveal {
        *scrolled_query = None;
    }
    let want_scroll = mode == Mode::Reveal && scrolled_query.as_deref() != Some(query.as_str());

    let mut edits: Vec<StagedEdit> = Vec::new();
    let mut scrolled = false;
    {
        let mut walk = Walk {
            registry,
            buffers,
            edits: &mut edits,
            query: &query,
            mode,
            matches: &matches,
            want_scroll,
            scrolled: &mut scrolled,
        };
        let mut path: Vec<String> = Vec::new();
        if let Raw::Struct(fields) = session.root().peel() {
            for (key, field) in fields {
                walk.render_field(ui, &mut path, key, field);
            }
        }
    }
    if want_scroll {
        *scrolled_query = Some(query);
    }

    // Apply staged edits now that the read-only borrow of the tree is released.
    for (path, label, value) in edits {
        let p: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
        let _ = session.set_scalar(&p, label, &value);
    }
}

/// The registry display-name at `path`, lowercased-contains the query?
fn name_matches(registry: &FieldRegistry, path: &[String], query: &str) -> bool {
    let p: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
    registry
        .lookup(&p)
        .is_some_and(|d| d.name.to_lowercase().contains(query))
}

/// Build the set of node paths (dotted) that match the query or are an ancestor
/// of a match — a single pass over the whole tree.
fn build_matches(root: &Raw, query: &str, registry: &FieldRegistry) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut path: Vec<String> = Vec::new();
    if let Raw::Struct(fields) = root.peel() {
        for (k, f) in fields {
            if let Field::Value(v) = f {
                path.push(k.clone());
                collect_matches(v, &mut path, query, registry, &mut out);
                path.pop();
            }
        }
    }
    out
}

/// Returns whether `node` (at `path`) matches or contains a match, inserting
/// every such node's dotted path into `out`.
fn collect_matches(
    node: &Raw,
    path: &mut Vec<String>,
    query: &str,
    registry: &FieldRegistry,
    out: &mut HashSet<String>,
) -> bool {
    // This node matches if its key (last path segment) or registry name matches.
    let mut found = name_matches(registry, path, query)
        || path
            .last()
            .is_some_and(|seg| seg.to_lowercase().contains(query));

    match node.peel() {
        Raw::Scalar(s) => {
            if s.to_lowercase().contains(query) {
                found = true;
            }
        }
        Raw::Struct(fields) => {
            for (k, f) in fields {
                if let Field::Value(v) = f {
                    path.push(k.clone());
                    found |= collect_matches(v, path, query, registry, out);
                    path.pop();
                }
            }
        }
        Raw::List(items) => {
            for (i, it) in items.iter().enumerate() {
                path.push(i.to_string());
                found |= collect_matches(it, path, query, registry, out);
                path.pop();
            }
        }
        Raw::Base64(_) => {}
    }

    if found {
        out.insert(path.join("."));
    }
    found
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
    /// Node paths that match or are an ancestor of a match (empty in `None`).
    matches: &'a HashSet<String>,
    /// Whether we should scroll to the first match this frame (Reveal only).
    want_scroll: bool,
    /// Whether we've already scrolled this frame.
    scrolled: &'a mut bool,
}

impl Walk<'_> {
    /// The registry display-name for the current path, if known.
    fn known_name(&self, path: &[String]) -> Option<&'static str> {
        let p: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
        self.registry.lookup(&p).map(|d| d.name)
    }

    /// Is this node a match or ancestor-of-match (from the pre-pass)?
    fn on_match(&self, path: &[String]) -> bool {
        self.mode != Mode::None && self.matches.contains(&path.join("."))
    }

    /// Render one struct field (a key and its value).
    fn render_field(&mut self, ui: &mut egui::Ui, path: &mut Vec<String>, key: &str, field: &Field) {
        path.push(key.to_string());
        match field {
            Field::EmptyColon | Field::EmptyBare => {
                let show = self.mode != Mode::Filter || key.to_lowercase().contains(self.query);
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
                let is_match = self.on_match(path);
                let visible = self.mode != Mode::Filter || is_match;
                if visible {
                    self.scalar_row(ui, path, &name, s, is_match);
                }
            }
            Raw::Struct(fields) => {
                let force_open = self.container_open(path);
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
                let force_open = self.container_open(path);
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

    /// Force a container open when an active search has a match inside it.
    fn container_open(&self, path: &[String]) -> Option<bool> {
        if self.on_match(path) { Some(true) } else { None }
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
    /// scrolls the first match into view (once per query).
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

        // Bring the first match into view in Reveal mode, once per query.
        if is_match && self.mode == Mode::Reveal && self.want_scroll && !*self.scrolled {
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

#[cfg(test)]
mod tests {
    use super::*;
    use save_parser::raw::{Field, Raw};

    fn scalar(s: &str) -> Field {
        Field::Value(Raw::Scalar(s.into()))
    }

    /// Searching a registry display name finds the labeled node (and its
    /// ancestors), not just literal occurrences of the text elsewhere.
    #[test]
    fn search_matches_registry_labels() {
        // `e` is labeled "Shadow Clones"; `e.a`/`e.b` "Shadow Clones (current/max)".
        let root = Raw::Struct(vec![(
            "e".into(),
            Field::Value(Raw::Base64(Box::new(Raw::Struct(vec![
                ("a".into(), scalar("5")),
                ("b".into(), scalar("9")),
            ])))),
        )]);
        let reg = FieldRegistry::new();
        let m = build_matches(&root, "shadow", &reg);
        assert!(m.contains("e"), "ancestor block marked");
        assert!(m.contains("e.a"), "leaf matched by its label");
        assert!(m.contains("e.b"));
    }

    /// A value/key match is included; unrelated siblings are not.
    #[test]
    fn search_matches_value_and_excludes_unrelated() {
        let root = Raw::Struct(vec![("W".into(), scalar("Adam")), ("c".into(), scalar("123"))]);
        let reg = FieldRegistry::new();
        let m = build_matches(&root, "adam", &reg);
        assert!(m.contains("W"), "matched by scalar value");
        assert!(!m.contains("c"), "unrelated field excluded");
    }
}
