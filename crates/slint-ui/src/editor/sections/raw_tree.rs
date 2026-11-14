//! The raw save tree navigator: a searchable, name-labeled view of the lossless
//! tree. Editing a scalar leaf stages a change through the same `EditSession` as
//! the structured sections, so the two views share one pending list.
//!
//! Search has two modes. **Filter** (default) hides everything that doesn't
//! match. **Reveal in place** keeps the whole tree visible but expands the path
//! to each match and scrolls to the first — useful for poking at values near a
//! known field. Matching considers raw keys, scalar values *and* the registry
//! display name at each node, so searching "Shadow Clones" finds the labeled
//! field, not just the literal string elsewhere.
//!
//! The tree is flattened into rows: a container contributes its children only
//! while it is open, so a render costs what is visible, not the whole save.
use std::collections::HashSet;

use save_parser::labels::Resolve;
use save_parser::navigate::{build_matches, core_label, equip_label, equipment_instance_index, prefix_set, resolve_name};
use save_parser::raw::{Field, Raw};
use save_parser::registry::{FieldKind, FieldRegistry};
use save_parser::session::EditSession;

use crate::editor::common::{refs, F};
use crate::editor::{Editor, Effect};
use crate::form::{button, check, editor as text_editor, input, link, muted, spacer, text, Mark, RowKind, Tone};

/// Rows rendered at most; a broader view asks for a narrower search.
const MAX_ROWS: usize = 3000;
const INDENT: f32 = 16.0;

/// The "Paste over node" editor.
pub struct Paste {
    /// Dotted (all-index) path of the node being replaced.
    path: String,
    /// The raw text to apply (prefilled with the node's current subtree).
    text: String,
    error: Option<String>,
}

#[derive(Default)]
pub struct State {
    search: String,
    reveal: bool,
    /// Containers expanded while browsing.
    open: HashSet<String>,
    /// Containers expanded by hand during a search or jump; dropped with it, so
    /// poking around mid-search doesn't leak into the browse view.
    transient: HashSet<String>,
    /// A node to reveal (set by "↪ tree" and cross-reference links).
    jump: Option<String>,
    /// The node whose copy / paste actions are showing.
    menu: Option<String>,
    paste: Option<Paste>,
}

impl Editor {
    /// Show the node at `target` (a dotted all-index path) in the tree. This
    /// replaces any search, so the jump — not a stale query — drives the view.
    pub fn reveal_in_tree(&mut self, target: String) {
        self.section = save_parser::registry::SectionId::RawTree;
        self.note = None;
        let state = &mut self.raw_tree;
        state.jump = Some(target);
        state.search.clear();
        state.transient.clear();
        self.jump_token = self.jump_token.wrapping_add(1);
    }

    /// Clipboard text arrived for the paste editor.
    pub fn pasted(&mut self, text: String) {
        if let Some(paste) = self.raw_tree.paste.as_mut() { (paste.text, paste.error) = (text, None); }
    }
}

/// How an active search affects the tree.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// No active query: browse.
    Browse,
    /// Hide everything that doesn't match.
    Filter,
    /// Keep the whole tree; expand the path to matches and scroll to the first.
    Reveal,
}

pub fn build(form: &mut F, session: &EditSession, registry: &FieldRegistry, state: &State) {
    form.title("Raw Save Tree");
    form.row(vec![
        text("Search:").w(50.0),
        input(form.on("tree/search", |editor, value| {
            let state = &mut editor.raw_tree;
            state.search = value.to_string();
            state.transient.clear();
            // A search supersedes (and consumes) an active navigation jump.
            if !state.search.trim().is_empty() { state.jump = None; }
            // Each new query scrolls to its first match once.
            if state.reveal { editor.jump_token = editor.jump_token.wrapping_add(1); }
        }), state.search.as_str()).live().hint("key, field name, or value").w(260.0),
        button(form.on("tree/search/clear", |editor, _| {
            let state = &mut editor.raw_tree;
            state.search.clear();
            state.jump = None;
            state.transient.clear();
        }), "Clear").w(70.0),
        check(form.on("tree/reveal", |editor, value| {
            editor.raw_tree.reveal = value == "1";
            if value == "1" { editor.jump_token = editor.jump_token.wrapping_add(1); }
        }), "Reveal in place", state.reveal).w(140.0),
        button(form.on("tree/collapse-all", |editor, _| {
            editor.raw_tree.open.clear();
            editor.raw_tree.transient.clear();
        }), "Collapse all").w(110.0).hint("Collapse the entire tree"),
        muted("Reveal in place jumps to matches and expands the path to them, without hiding the rest of the tree.").size(11.0),
    ]);
    form.note("Known fields show their name. Editing a scalar stages a change in the pending list; values are written verbatim. “⋯” on a row copies its path or value, or pastes over a node.");

    let query = state.search.trim().to_lowercase();
    let jump = state.jump.as_deref().filter(|_| query.is_empty());
    if let Some(target) = jump {
        form.row(vec![
            text("▶ Navigated to").tone(Tone::Accent).size(11.0).w(100.0),
            text(target).tone(Tone::Bright).mono().size(11.0),
            button(form.on("tree/jump/clear", |editor, _| {
                editor.raw_tree.jump = None;
                editor.raw_tree.transient.clear();
            }), "Clear").w(70.0),
        ]);
    }
    if let Some(paste) = &state.paste { paste_form(form, paste); }
    form.pin();

    let (mode, matches) = if !query.is_empty() {
        (if state.reveal { Mode::Reveal } else { Mode::Filter }, build_matches(session.root(), &query, registry))
    } else if let Some(target) = jump {
        (Mode::Reveal, prefix_set(target))
    } else {
        (Mode::Browse, HashSet::new())
    };
    let root = session.root();
    let mut walk = Walk { form, registry, root, state, mode, matches: &matches, query: &query, jump, scrolled: false, rows: 0, hidden: 0 };
    let mut path = Vec::new();
    if let Raw::Struct(fields) = root.peel() {
        for (key, field) in fields { walk.field(&mut path, key, field); }
    }
    let hidden = walk.hidden;
    if hidden > 0 {
        form.note_tone(format!("… {hidden} more rows not shown. Narrow the search or collapse a container to see them."), Tone::Warning);
    }
}

/// The paste editor: a prefilled raw-text box that, on Apply, replaces the
/// node's whole subtree (validated by a full encode round-trip, so malformed
/// input is rejected).
fn paste_form(form: &mut F, paste: &Paste) {
    form.row(vec![
        text("Paste over node").tone(Tone::Bright).w(120.0),
        muted(format!("Replacing: {}", paste.path)).mono().size(11.0),
    ]).mark = Mark::Selected;
    form.note("Paste raw save text (the format from “Copy node (raw)”). It is validated by a full re-encode before staging — a malformed paste is rejected.");
    form.push(RowKind::Editor, vec![text_editor(form.on("tree/paste/text", |editor, value| {
        if let Some(paste) = editor.raw_tree.paste.as_mut() { paste.text = value.to_string(); }
    }), paste.text.as_str())]);
    if let Some(error) = &paste.error { form.note_tone(error.clone(), Tone::Error); }
    form.row(vec![
        button(form.on("tree/paste/apply", |editor, _| {
            let (Some(session), Some(paste)) = (editor.session.as_mut(), editor.raw_tree.paste.as_mut()) else { return };
            let path: Vec<&str> = paste.path.split('.').collect();
            match session.replace_node(&path, &paste.text) {
                Ok(()) => editor.raw_tree.paste = None,
                Err(error) => paste.error = Some(error.to_string()),
            }
        }), "Apply").primary().w(80.0),
        button(form.on("tree/paste/clipboard", |editor, _| editor.effects.push(Effect::PasteClipboard)), "Paste from clipboard").w(170.0),
        button(form.on("tree/paste/cancel", |editor, _| editor.raw_tree.paste = None), "Cancel").w(80.0),
        spacer(),
    ]).mark = Mark::Selected;
}

/// How a node is named under its parent.
fn display(name: &str, in_list: bool) -> String { if in_list { format!("[{name}]") } else { name.to_string() } }

/// A rough pixel width for `text`, so label cells don't stretch.
fn width(text: &str, mono: bool) -> f32 { (text.chars().count() as f32 * if mono { 7.4 } else { 6.8 } + 14.0).clamp(30.0, 520.0) }

struct Walk<'a> {
    form: &'a mut F,
    registry: &'a FieldRegistry,
    /// The whole tree, for cross-references (equipment-instance id → item).
    root: &'a Raw,
    state: &'a State,
    mode: Mode,
    /// Node paths that match or are an ancestor of a match (empty while browsing).
    matches: &'a HashSet<String>,
    query: &'a str,
    /// The node a navigation jump targets.
    jump: Option<&'a str>,
    /// Whether the scroll target has been placed.
    scrolled: bool,
    rows: usize,
    hidden: usize,
}

impl Walk<'_> {
    fn on_match(&self, dotted: &str) -> bool { self.mode != Mode::Browse && self.matches.contains(dotted) }

    /// Whether a row may still be added; counts the ones that may not.
    fn room(&mut self) -> bool {
        if self.rows >= MAX_ROWS { self.hidden += 1; return false; }
        self.rows += 1;
        true
    }

    fn indent(path: &[String]) -> f32 { (path.len().saturating_sub(1)) as f32 * INDENT }

    /// Scroll to this row if it is the jump target or, when revealing a search,
    /// the first matching leaf.
    fn mark_scroll(&mut self, dotted: &str, leaf_match: bool) {
        let target = match self.jump { Some(target) => target == dotted, None => leaf_match && self.mode == Mode::Reveal };
        if target && !self.scrolled {
            self.form.jump_here();
            self.scrolled = true;
        }
    }

    fn field(&mut self, path: &mut Vec<String>, key: &str, field: &Field) {
        path.push(key.to_string());
        match field {
            Field::EmptyColon | Field::EmptyBare => {
                if (self.mode != Mode::Filter || key.to_lowercase().contains(self.query)) && self.room() {
                    self.form.row(vec![muted(key).mono().w(width(key, true)), muted("(empty)").size(11.0)]).indent = Self::indent(path);
                }
            }
            Field::Value(value) => self.value(path, false, value),
        }
        path.pop();
    }

    fn value(&mut self, path: &mut Vec<String>, in_list: bool, value: &Raw) {
        let dotted = path.join(".");
        let on_match = self.on_match(&dotted);
        match value.peel() {
            Raw::Scalar(current) => {
                if self.mode != Mode::Filter || on_match { self.scalar(path, &dotted, in_list, current, on_match); }
            }
            Raw::Struct(fields) => {
                if self.mode == Mode::Filter && !on_match { return; }
                if self.container(path, &dotted, in_list, value, format!("{{{} fields}}", fields.len()), on_match) {
                    for (key, field) in fields { self.field(path, key, field); }
                }
            }
            Raw::List(items) => {
                if self.mode == Mode::Filter && !on_match { return; }
                if self.container(path, &dotted, in_list, value, format!("[{} items]", items.len()), on_match) {
                    for (index, item) in items.iter().enumerate() {
                        path.push(index.to_string());
                        self.value(path, true, item);
                        path.pop();
                    }
                }
            }
            // `peel` already removed any Base64 wrapper.
            Raw::Base64(_) => {}
        }
    }

    /// A human title for an element container from one of its children (a pet
    /// by its name, a monument by its id), if the schema says how.
    fn element_label(&self, path: &[String], value: &Raw) -> Option<String> {
        let (key, resolve) = self.registry.lookup(&refs(path))?.element_name?;
        // Node-based titles read the whole element, not a single child scalar.
        match resolve {
            Resolve::EquipmentNode => equip_label(value),
            Resolve::CoreNode => core_label(value),
            _ => match value.get(key)? { Raw::Scalar(child) => resolve_name(resolve, child, self.root), _ => None },
        }
    }

    /// A container's header row. Returns whether its children should follow.
    fn container(&mut self, path: &[String], dotted: &str, in_list: bool, value: &Raw, summary: String, on_match: bool) -> bool {
        // While searching, a container on the path to a match is forced open;
        // anything else follows what was toggled by hand.
        let open = match self.mode {
            Mode::Browse => self.state.open.contains(dotted),
            _ => on_match || self.state.transient.contains(dotted),
        };
        if !self.room() { return false; }
        self.mark_scroll(dotted, false);
        let known = self.registry.lookup(&refs(path)).map(|d| d.name);
        let extra = self.element_label(path, value);
        let name = display(path.last().map_or("", String::as_str), in_list);
        let title = match (&extra, known) {
            // resolved name (e.g. "Robot") · type label · node · summary
            (Some(extra), Some(known)) => format!("{extra}  ·  {known} {name}   {summary}"),
            (Some(extra), None) => format!("{extra}  ·  {name}   {summary}"),
            (None, Some(known)) => format!("{known}  ·  {name}   {summary}"),
            (None, None) => format!("{name}   {summary}"),
        };
        let tone = if extra.is_some() { Tone::Bright } else if known.is_some() { Tone::Accent } else { Tone::Normal };
        let browsing = self.mode == Mode::Browse;
        let key = dotted.to_string();
        let toggle = self.form.on(format!("tree/toggle/{dotted}"), move |editor, _| {
            let state = &mut editor.raw_tree;
            let set = if browsing { &mut state.open } else { &mut state.transient };
            if !set.remove(&key) { set.insert(key.clone()); }
        });
        let cells = vec![link(toggle, format!("{} {title}", if open { "▾" } else { "▸" })).tone(tone), self.menu_button(dotted)];
        let row = self.form.row(cells);
        row.indent = Self::indent(path);
        if self.jump == Some(dotted) { row.mark = Mark::Match; }
        if self.state.menu.as_deref() == Some(dotted) { self.menu(path, dotted, false); }
        open
    }

    fn menu_button(&self, dotted: &str) -> crate::form::Cell {
        let key = dotted.to_string();
        button(self.form.on(format!("tree/menu/{dotted}"), move |editor, _| {
            let menu = &mut editor.raw_tree.menu;
            *menu = if menu.as_deref() == Some(key.as_str()) { None } else { Some(key.clone()) };
        }), "⋯").w(34.0).hint("Copy path / value, or paste over this node")
    }

    /// The copy / paste actions of one node, as a row under it.
    fn menu(&mut self, path: &[String], dotted: &str, scalar: bool) {
        if !self.room() { return; }
        let form = &*self.form;
        let key = dotted.to_string();
        let mut cells = vec![button(form.on("tree/menu/copy-path", move |editor, _| {
            editor.effects.push(Effect::Copy(key.clone()));
            editor.raw_tree.menu = None;
        }), "Copy path").w(100.0)];
        // The node's text is read when the button is pressed, not on every render.
        let node_text = |editor: &Editor, path: &[String]| editor.session.as_ref()
            .and_then(|session| session.root().get_path(&refs(path)).map(|node| node.peel().serialize()));
        if scalar {
            let path = path.to_vec();
            cells.push(button(form.on("tree/menu/copy-value", move |editor, _| {
                if let Some(text) = node_text(editor, &path) { editor.effects.push(Effect::Copy(text)); }
                editor.raw_tree.menu = None;
            }), "Copy value").w(100.0));
        } else {
            let (copy, paste, key) = (path.to_vec(), path.to_vec(), dotted.to_string());
            cells.push(button(form.on("tree/menu/copy-node", move |editor, _| {
                if let Some(text) = node_text(editor, &copy) { editor.effects.push(Effect::Copy(text)); }
                editor.raw_tree.menu = None;
            }), "Copy node (raw)").w(130.0).hint("Copy this node and its whole subtree as raw save text"));
            cells.push(button(form.on("tree/menu/paste", move |editor, _| {
                if let Some(text) = node_text(editor, &paste) { editor.raw_tree.paste = Some(Paste { path: key.clone(), text, error: None }); }
                editor.raw_tree.menu = None;
            }), "Paste over node…").w(140.0).hint("Replace this node's whole subtree with raw save text (validated)"));
        }
        cells.push(spacer());
        let row = self.form.row(cells);
        row.indent = Self::indent(path) + INDENT;
        row.mark = Mark::Selected;
    }

    /// An editable scalar leaf.
    fn scalar(&mut self, path: &[String], dotted: &str, in_list: bool, current: &str, is_match: bool) {
        if !self.room() { return; }
        self.mark_scroll(dotted, is_match);
        let form = &*self.form;
        let field = self.registry.lookup(&refs(path));
        let known = field.map(|d| d.name);
        let name = display(path.last().map_or("", String::as_str), in_list);
        let mut cells = match known {
            Some(known) => vec![text(known).tone(Tone::Accent).w(width(known, false)), muted(format!("· {name}")).mono().w(width(&name, true) + 12.0)],
            None => vec![text(name.as_str()).mono().tone(if is_match { Tone::Warning } else { Tone::Normal }).w(width(&name, true))],
        };
        // Values are written verbatim, so the tree stays fully unrestricted.
        let (target, label) = (path.to_vec(), known.map_or_else(|| dotted.to_string(), str::to_string));
        let set = move |editor: &mut Editor, value: &str| {
            let Some(session) = editor.session.as_mut() else { return };
            if session.value(&refs(&target)).as_deref() != Some(value) { let _ = session.set_scalar(&refs(&target), label.as_str(), value); }
        };
        let commit = set.clone();
        cells.push(input(form.on(format!("tree/set/{dotted}"), move |editor, value| commit(editor, value.trim())), current).mono().w(260.0));
        // Bool fields get quick True/False buttons (the text box still works).
        if field.is_some_and(|d| d.kind == FieldKind::Bool) {
            for value in ["True", "False"] {
                let set = set.clone();
                cells.push(button(form.on(format!("tree/bool/{dotted}/{value}"), move |editor, _| set(editor, value)), value).w(60.0));
            }
        }
        // Ranged numeric fields show the bound and warn (never block) when the
        // current value is outside it.
        if let Some((low, high)) = field.and_then(|d| d.range) {
            let outside = current.trim().parse::<u32>().is_ok_and(|v| v < low || v > high);
            cells.push(text(format!("({low}–{high})")).size(11.0).w(70.0)
                .tone(if outside { Tone::Warning } else { Tone::Muted })
                .hint(if outside { "value is outside the expected range" } else { "expected range" }));
        }
        if let Some(resolve) = field.and_then(|d| d.resolve) && let Some(annotation) = resolve_name(resolve, current, self.root) {
            let shown = format!("→ {annotation}");
            // An equipment-instance reference (a pet's weapon / armor / accessory
            // slot) links to the item's node; other ids are plain name labels.
            let item = (resolve == Resolve::EquipmentInstance).then(|| equipment_instance_index(current, self.root)).flatten();
            cells.push(match item {
                Some(index) => link(form.on(format!("tree/goto/{dotted}"), move |editor, _| editor.reveal_in_tree(format!("X.R.{index}"))), shown.as_str())
                    .hint("Go to this item in the tree"),
                None => text(shown.as_str()).tone(Tone::Success),
            }.w(width(&shown, false)));
        }
        if is_match { cells.push(text("◀").tone(Tone::Warning).w(14.0)); }
        cells.push(spacer());
        cells.push(self.menu_button(dotted));
        let row = self.form.row(cells);
        row.indent = Self::indent(path);
        if is_match { row.mark = Mark::Match; }
        if self.state.menu.as_deref() == Some(dotted) { self.menu(path, dotted, true); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::tests::{fire, loaded, open};
    use crate::editor::build as render;
    use save_parser::registry::SectionId;

    fn labels(editor: &Editor) -> Vec<String> {
        render(editor).rows.iter().map(|row| row.cells.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join(" ")).collect()
    }

    #[test]
    fn containers_open_on_demand_and_collapse_all_resets() {
        let mut editor = loaded();
        open(&mut editor, SectionId::RawTree);
        let top = render(&editor).rows.len();
        fire(&mut editor, "tree/toggle/X", "");
        assert!(render(&editor).rows.len() > top);
        fire(&mut editor, "tree/toggle/X.b", "");
        // A pet element is titled by its name.
        assert!(labels(&editor).iter().any(|l| l.contains("[0]") && l.contains("  ·  ")));
        fire(&mut editor, "tree/collapse-all", "");
        assert_eq!(render(&editor).rows.len(), top);
    }

    #[test]
    fn filter_hides_non_matches_and_reveal_keeps_the_tree() {
        let mut editor = loaded();
        open(&mut editor, SectionId::RawTree);
        let top = render(&editor).rows.len();
        fire(&mut editor, "tree/search", "shadow clones");
        let filtered = render(&editor);
        assert!(!filtered.rows.is_empty() && filtered.rows.len() < top);
        assert!(filtered.rows.iter().any(|r| r.mark == Mark::Match));
        assert_eq!(filtered.jump, None, "filtering does not scroll");
        // Reveal keeps every top-level node and scrolls to the first match.
        let token = editor.jump_token;
        fire(&mut editor, "tree/reveal", "1");
        let revealed = render(&editor);
        assert!(revealed.rows.len() >= top);
        assert!(revealed.jump.is_some());
        assert_ne!(editor.jump_token, token);
        fire(&mut editor, "tree/search/clear", "");
        assert_eq!(render(&editor).rows.len(), top, "forced-open matches don't leak into browsing");
    }

    #[test]
    fn scalars_stage_verbatim_and_pending_jumps_to_them() {
        let mut editor = loaded();
        open(&mut editor, SectionId::RawTree);
        fire(&mut editor, "tree/toggle/X", "");
        fire(&mut editor, "tree/set/X.y", " 4242 ");
        assert_eq!(editor.session.as_ref().unwrap().value(&["X", "y"]).unwrap(), "4242");
        // The pending row's "tree" button reveals the edited field.
        open(&mut editor, SectionId::Resources);
        fire(&mut editor, "pending/tree/0", "");
        assert_eq!(editor.section, SectionId::RawTree);
        let forms = render(&editor);
        let row = &forms.rows[forms.jump.expect("scrolls to the field")];
        assert_eq!(row.mark, Mark::Match);
        assert!(row.cells.iter().any(|c| c.text == "4242"));
        assert!(forms.top.iter().any(|r| r.cells.iter().any(|c| c.text == "X.y")));
        fire(&mut editor, "tree/jump/clear", "");
        assert_eq!(render(&editor).jump, None);
    }

    #[test]
    fn slot_references_link_to_their_item() {
        let mut editor = loaded();
        open(&mut editor, SectionId::RawTree);
        let save = editor.session.as_ref().unwrap().derived().unwrap();
        let pet = save.pets.iter().position(|p| p.weapon_id.is_some()).expect("a pet with a weapon");
        let item = save.equipment.iter().position(|e| Some(e.instance_id) == save.pets[pet].weapon_id).unwrap();
        for path in ["X".to_string(), "X.b".into(), format!("X.b.{pet}"), format!("X.b.{pet}.w")] { fire(&mut editor, &format!("tree/toggle/{path}"), ""); }
        fire(&mut editor, &format!("tree/goto/X.b.{pet}.w.e"), "");
        assert_eq!(editor.raw_tree.jump.as_deref(), Some(format!("X.R.{item}").as_str()));
        assert!(render(&editor).jump.is_some());
    }

    #[test]
    fn menu_copies_and_paste_replaces_a_validated_subtree() {
        let mut editor = loaded();
        open(&mut editor, SectionId::RawTree);
        fire(&mut editor, "tree/toggle/X", "");
        fire(&mut editor, "tree/toggle/X.002", "");
        fire(&mut editor, "tree/menu/X.002.0", "");
        fire(&mut editor, "tree/menu/copy-path", "");
        assert!(matches!(editor.effects.pop(), Some(Effect::Copy(path)) if path == "X.002.0"));
        fire(&mut editor, "tree/menu/X.002.0", "");
        fire(&mut editor, "tree/menu/copy-node", "");
        let Some(Effect::Copy(node)) = editor.effects.pop() else { panic!("nothing copied") };
        assert!(node.contains("a:") && node.contains("c:"), "{node}");

        fire(&mut editor, "tree/menu/X.002.0", "");
        fire(&mut editor, "tree/menu/paste", "");
        assert_eq!(editor.raw_tree.paste.as_ref().unwrap().text, node);
        fire(&mut editor, "tree/paste/clipboard", "");
        assert!(matches!(editor.effects.pop(), Some(Effect::PasteClipboard)));
        let count = editor.session.as_ref().unwrap().value(&["X", "002", "0", "c"]).unwrap();
        editor.pasted(node.replace(&format!("c:{count};"), "c:987654;"));
        fire(&mut editor, "tree/paste/apply", "");
        assert!(editor.raw_tree.paste.is_none(), "{:?}", editor.raw_tree.paste.as_ref().and_then(|p| p.error.clone()));
        let session = editor.session.as_ref().unwrap();
        assert_eq!(session.value(&["X", "002", "0", "c"]).unwrap(), "987654");
        assert_eq!(session.tree_edits().len(), 1);
    }

    #[test]
    fn a_broad_view_is_capped() {
        let mut editor = loaded();
        open(&mut editor, SectionId::RawTree);
        fire(&mut editor, "tree/search", "a");
        let forms = render(&editor);
        assert!(forms.rows.len() <= MAX_ROWS + 1);
        assert!(forms.rows.last().unwrap().cells[0].text.contains("more rows not shown"));
    }
}
