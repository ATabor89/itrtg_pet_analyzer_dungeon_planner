//! Rust side of the data-driven form renderer (`ui/form.slint`).
//!
//! A view describes itself as [`Row`]s of [`Cell`]s and registers a closure per
//! interactive cell. [`FormModel`] pushes the rows into a Slint model by diffing
//! against what is already shown, so untouched controls keep their focus and
//! in-progress text while the rest of the screen updates.
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use slint::{Model, ModelRc, SharedString, VecModel};

use crate::{FormCell, FormRow};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone { #[default] Normal, Muted, Accent, Warning, Error, Success, Bright }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind { #[default] Text, Input, Check, Combo, Button, Spacer, Link, Editor }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RowKind { #[default] Normal, Header, Title, Separator, Note, Editor }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mark { #[default] None, Selected, Match }

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cell {
    pub kind: Kind,
    pub id: String,
    pub text: String,
    pub options: Rc<Vec<String>>,
    pub index: i32,
    pub checked: bool,
    pub width: f32,
    pub tone: Tone,
    pub disabled: bool,
    pub hint: String,
    pub live: bool,
    pub primary: bool,
    pub mono: bool,
    pub size: f32,
}

impl Cell {
    pub fn w(mut self, width: f32) -> Self { self.width = width; self }
    pub fn tone(mut self, tone: Tone) -> Self { self.tone = tone; self }
    pub fn hint(mut self, hint: impl Into<String>) -> Self { self.hint = hint.into(); self }
    pub fn live(mut self) -> Self { self.live = true; self }
    pub fn primary(mut self) -> Self { self.primary = true; self }
    pub fn mono(mut self) -> Self { self.mono = true; self }
    pub fn size(mut self, size: f32) -> Self { self.size = size; self }
    pub fn enabled(mut self, enabled: bool) -> Self { self.disabled = !enabled; self }
}

pub fn text(text: impl Into<String>) -> Cell { Cell { text: text.into(), ..Default::default() } }
pub fn muted(text: impl Into<String>) -> Cell { self::text(text).tone(Tone::Muted) }
pub fn spacer() -> Cell { Cell { kind: Kind::Spacer, ..Default::default() } }
pub fn gap(width: f32) -> Cell { spacer().w(width) }
pub fn input(id: String, value: impl Into<String>) -> Cell { Cell { kind: Kind::Input, id, text: value.into(), ..Default::default() } }
pub fn check(id: String, label: impl Into<String>, checked: bool) -> Cell { Cell { kind: Kind::Check, id, text: label.into(), checked, ..Default::default() } }
pub fn button(id: String, label: impl Into<String>) -> Cell { Cell { kind: Kind::Button, id, text: label.into(), ..Default::default() } }
pub fn link(id: String, label: impl Into<String>) -> Cell { Cell { kind: Kind::Link, id, text: label.into(), ..Default::default() } }
pub fn editor(id: String, value: impl Into<String>) -> Cell { Cell { kind: Kind::Editor, id, text: value.into(), ..Default::default() } }
pub fn combo(id: String, options: Rc<Vec<String>>, index: usize) -> Cell {
    Cell { kind: Kind::Combo, id, options, index: index as i32, ..Default::default() }
}
/// Option list for a [`combo`]; share one `Rc` between rows that offer the same choices.
pub fn options<S: AsRef<str>>(items: impl IntoIterator<Item = S>) -> Rc<Vec<String>> {
    Rc::new(items.into_iter().map(|s| s.as_ref().to_string()).collect())
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    pub cells: Vec<Cell>,
    pub kind: RowKind,
    pub indent: f32,
    pub mark: Mark,
}

/// What an interactive cell does: receives the context and the committed value
/// (text for inputs, `"1"`/`"0"` for checkboxes, the index for pickers).
pub type Act<C> = Rc<dyn Fn(&mut C, &str)>;

/// Rows plus their actions, as built by one render pass over context `C`.
pub struct Form<C> {
    pub rows: Vec<Row>,
    /// Interior-mutable so a cell can register its action inside the argument
    /// list of the row it belongs to (`form.row(vec![button(form.on(..), ..)])`).
    acts: RefCell<HashMap<String, Act<C>>>,
    /// Rows before this index stay pinned above the scrolling list (see [`Form::pin`]).
    pub pinned: usize,
    /// Row (in the scrolling part) to bring into view, if any.
    pub jump: Option<usize>,
}

impl<C> Default for Form<C> {
    fn default() -> Self { Self { rows: Vec::new(), acts: RefCell::new(HashMap::new()), pinned: 0, jump: None } }
}

impl<C> Form<C> {
    /// Register an action under a stable id and return that id for the cell.
    pub fn on(&self, id: impl Into<String>, act: impl Fn(&mut C, &str) + 'static) -> String {
        let id = id.into();
        self.acts.borrow_mut().insert(id.clone(), Rc::new(act));
        id
    }
    /// Everything added so far stays pinned; rows added from here on scroll.
    pub fn pin(&mut self) { self.pinned = self.rows.len(); }
    /// Bring the next row to be added into view.
    pub fn jump_here(&mut self) { self.jump = Some(self.rows.len().saturating_sub(self.pinned)); }
    /// Take the rows built so far (to split one pass into several panels).
    pub fn take_rows(&mut self) -> Vec<Row> { std::mem::take(&mut self.rows) }
    pub fn into_acts(self) -> HashMap<String, Act<C>> { self.acts.into_inner() }
    pub fn row(&mut self, cells: Vec<Cell>) -> &mut Row { self.push(RowKind::Normal, cells) }
    pub fn header(&mut self, cells: Vec<Cell>) -> &mut Row {
        let cells = cells.into_iter().map(|c| if c.kind == Kind::Text && c.tone == Tone::Normal { c.tone(Tone::Muted) } else { c }).collect();
        self.push(RowKind::Header, cells)
    }
    pub fn title(&mut self, title: impl Into<String>) -> &mut Row { self.push(RowKind::Title, vec![text(title).tone(Tone::Accent).size(18.0)]) }
    /// A section title followed by cells on the same line (buttons, counters).
    pub fn title_with(&mut self, title: impl Into<String>, mut cells: Vec<Cell>) -> &mut Row {
        let title: String = title.into();
        let width = (title.chars().count() as f32 * 10.5 + 16.0).max(80.0);
        cells.insert(0, text(title).tone(Tone::Accent).size(18.0).w(width));
        self.push(RowKind::Title, cells)
    }
    pub fn subtitle(&mut self, title: impl Into<String>) -> &mut Row { self.push(RowKind::Normal, vec![text(title).tone(Tone::Bright).size(13.0)]) }
    pub fn note(&mut self, body: impl Into<String>) -> &mut Row { self.push(RowKind::Note, vec![muted(body).size(11.0)]) }
    pub fn note_tone(&mut self, body: impl Into<String>, tone: Tone) -> &mut Row { self.push(RowKind::Note, vec![text(body).tone(tone).size(11.0)]) }
    pub fn sep(&mut self) -> &mut Row { self.push(RowKind::Separator, Vec::new()) }
    pub fn push(&mut self, kind: RowKind, cells: Vec<Cell>) -> &mut Row {
        self.rows.push(Row { cells, kind, ..Default::default() });
        self.rows.last_mut().unwrap()
    }
    /// A status line: green for success, red for an error.
    pub fn status(&mut self, status: &Option<(String, bool)>) {
        if let Some((message, error)) = status {
            self.note_tone(message.clone(), if *error { Tone::Error } else { Tone::Success });
        }
    }
}

/// Routes `action(id, value)` callbacks from a form to the registered actions.
///
/// Inputs report in-progress text as `draft:<id>`; the draft is committed before
/// any *other* action runs, so a value typed into a field takes effect when a
/// button is clicked even though the field never lost focus.
pub struct Dispatcher<C> {
    acts: HashMap<String, Act<C>>,
    draft: Option<(String, String)>,
}

impl<C> Default for Dispatcher<C> {
    fn default() -> Self { Self { acts: HashMap::new(), draft: None } }
}

impl<C> Dispatcher<C> {
    /// Install the actions of the latest render.
    pub fn set(&mut self, acts: HashMap<String, Act<C>>) { self.acts = acts; }

    /// Returns true when an action ran (the caller should re-render).
    pub fn dispatch(&mut self, context: &mut C, id: &str, value: &str) -> bool {
        if let Some(id) = id.strip_prefix("draft:") {
            self.draft = Some((id.to_string(), value.to_string()));
            return false;
        }
        let mut ran = false;
        if let Some((draft, text)) = self.draft.take() && draft != id && let Some(act) = self.acts.get(&draft).cloned() {
            act(context, &text);
            ran = true;
        }
        if let Some(act) = self.acts.get(id).cloned() {
            act(context, value);
            ran = true;
        }
        ran
    }
}

/// The Slint model for one form, updated in place.
pub struct FormModel {
    rows: Rc<VecModel<FormRow>>,
    shadow: Vec<Row>,
    cells: Vec<Rc<VecModel<FormCell>>>,
}

impl Default for FormModel {
    fn default() -> Self { Self { rows: Rc::new(VecModel::default()), shadow: Vec::new(), cells: Vec::new() } }
}

type OptionCache = HashMap<*const Vec<String>, ModelRc<SharedString>>;

fn slint_cell(cell: &Cell, cache: &mut OptionCache) -> FormCell {
    let options = if cell.options.is_empty() { ModelRc::default() } else {
        cache.entry(Rc::as_ptr(&cell.options)).or_insert_with(|| {
            Rc::new(VecModel::from(cell.options.iter().map(SharedString::from).collect::<Vec<_>>())).into()
        }).clone()
    };
    FormCell {
        kind: cell.kind as i32, id: cell.id.as_str().into(), text: cell.text.as_str().into(), options,
        index: cell.index, checked: cell.checked, width: cell.width, tone: cell.tone as i32,
        enabled: !cell.disabled, hint: cell.hint.as_str().into(), live: cell.live, primary: cell.primary,
        mono: cell.mono, size: cell.size,
    }
}

/// Same control in the same place: its instance can be updated instead of
/// rebuilt. A picker whose options changed still counts — rebuilding the row
/// would take the focus from a search box beside it mid-typing.
fn same_shape(a: &Cell, b: &Cell) -> bool {
    a.kind == b.kind && a.width == b.width
}

impl FormModel {
    pub fn model(&self) -> ModelRc<FormRow> { self.rows.clone().into() }
    #[cfg(test)]
    pub fn len(&self) -> usize { self.shadow.len() }

    pub fn set(&mut self, rows: Vec<Row>) {
        let mut cache = OptionCache::new();
        let slint_row = |row: &Row, cells: &Rc<VecModel<FormCell>>| FormRow {
            cells: cells.clone().into(), kind: row.kind as i32, indent: row.indent, mark: row.mark as i32,
        };
        for (index, row) in rows.iter().enumerate() {
            if index >= self.shadow.len() {
                let cells = Rc::new(VecModel::from(row.cells.iter().map(|c| slint_cell(c, &mut cache)).collect::<Vec<_>>()));
                self.rows.push(slint_row(row, &cells));
                self.cells.push(cells);
                continue;
            }
            let old = &self.shadow[index];
            if old == row { continue; }
            let reuse = old.cells.len() == row.cells.len() && old.cells.iter().zip(&row.cells).all(|(a, b)| same_shape(a, b));
            if reuse {
                for (i, (a, b)) in old.cells.iter().zip(&row.cells).enumerate() {
                    if a != b { self.cells[index].set_row_data(i, slint_cell(b, &mut cache)); }
                }
                if old.kind != row.kind || old.indent != row.indent || old.mark != row.mark {
                    self.rows.set_row_data(index, slint_row(row, &self.cells[index]));
                }
            } else {
                let cells = Rc::new(VecModel::from(row.cells.iter().map(|c| slint_cell(c, &mut cache)).collect::<Vec<_>>()));
                self.rows.set_row_data(index, slint_row(row, &cells));
                self.cells[index] = cells;
            }
        }
        while self.rows.row_count() > rows.len() {
            self.rows.remove(self.rows.row_count() - 1);
            self.cells.pop();
        }
        self.shadow = rows;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(model: &FormModel) -> Vec<Vec<String>> {
        model.rows.iter().map(|r| r.cells.iter().map(|c| c.text.to_string()).collect()).collect()
    }

    #[test]
    fn set_diffs_in_place_and_tracks_length() {
        let mut model = FormModel::default();
        model.set(vec![Row { cells: vec![text("a"), input("x".into(), "1")], ..Default::default() }, Row { cells: vec![text("b")], ..Default::default() }]);
        assert_eq!(texts(&model), [vec!["a", "1"], vec!["b"]]);
        let first = model.cells[0].clone();

        // Same shape: the cell model is reused (controls keep their state).
        model.set(vec![Row { cells: vec![text("a"), input("x".into(), "2")], ..Default::default() }]);
        assert_eq!(texts(&model), [vec!["a", "2"]]);
        assert!(Rc::ptr_eq(&first, &model.cells[0]));
        assert_eq!(model.len(), 1);

        // Different shape: the row is rebuilt.
        model.set(vec![Row { cells: vec![button("go".into(), "Go")], ..Default::default() }, Row::default()]);
        assert!(!Rc::ptr_eq(&first, &model.cells[0]));
        assert_eq!(model.rows.row_count(), 2);
    }

    #[test]
    fn dispatcher_commits_a_pending_draft_before_another_action() {
        let form: Form<Vec<String>> = Form::default();
        form.on("value", |log, v| log.push(format!("value={v}")));
        form.on("apply", |log, _| log.push("apply".into()));
        let mut dispatcher = Dispatcher::default();
        dispatcher.set(form.into_acts());
        let mut log = Vec::new();
        assert!(!dispatcher.dispatch(&mut log, "draft:value", "4"));
        assert!(!dispatcher.dispatch(&mut log, "draft:value", "42"));
        assert!(dispatcher.dispatch(&mut log, "apply", ""));
        assert_eq!(log, ["value=42", "apply"]);
        // Committing the field itself (Enter / focus loss) consumes the draft once.
        dispatcher.dispatch(&mut log, "draft:value", "7");
        dispatcher.dispatch(&mut log, "value", "7");
        dispatcher.dispatch(&mut log, "apply", "");
        assert_eq!(log[2..], ["value=7", "apply"]);
        assert!(!dispatcher.dispatch(&mut log, "unknown", ""));
    }

    #[test]
    fn form_registers_actions_by_id() {
        let mut form: Form<Vec<String>> = Form::default();
        form.row(vec![button(form.on("add", |log, value| log.push(value.to_string())), "Add")]);
        assert_eq!(form.rows.len(), 1);
        let acts = form.into_acts();
        let mut log = Vec::new();
        acts["add"](&mut log, "v");
        assert_eq!(log, ["v"]);
    }
}
