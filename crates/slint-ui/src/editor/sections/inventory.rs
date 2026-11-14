//! Inventory: view/filter the material inventory (`X.Q`), bulk-edit quantities
//! and add items (known or arbitrary id).
//!
//! Quantity edits stage by item id (`X.Q.a=<id>.b`), so the numeric index doesn't
//! matter; "Add item" upserts via `set_material` (edit the stack if owned, else
//! create it).
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use save_parser::edit::apply_factor;
use save_parser::items;
use save_parser::session::EditSession;

use crate::editor::bulk::{self, parse_f64, parse_u64, OpKind};
use crate::editor::common::{pick, F};
use crate::editor::Editor;
use crate::form::{button, check, gap, input, muted, spacer, text, Mark, Tone};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sort { Item, Id, Quantity }

struct Item {
    index: usize,
    id: u32,
    name: Option<&'static str>,
    count: u64,
    /// Exact stored count, for × Mul and the skip-compare.
    raw_count: String,
}

impl Item {
    fn label(&self) -> String { item_label(self.id) }
}

fn item_label(id: u32) -> String { items::material_name(id).map_or_else(|| format!("Item {id}"), str::to_string) }

#[derive(Default)]
pub struct Add { id: String, search: String, quantity: String }

#[derive(Default)]
pub struct State {
    name: String,
    known: Option<bool>,
    count_min: String,
    selected: HashSet<usize>,
    op: Option<(OpKind, String)>,
    overrides: HashMap<usize, String>,
    sort: Option<(Sort, bool)>,
    add: Option<Add>,
}

fn of(editor: &mut Editor) -> &mut State { &mut editor.inventory }

fn stacks(session: &EditSession) -> Option<Vec<Item>> {
    let save = session.derived()?;
    Some(save.materials.iter().enumerate().map(|(index, material)| Item {
        index, id: material.item_id, name: items::material_name(material.item_id), count: material.count,
        raw_count: session.value(&["X", "Q", &index.to_string(), "b"]).unwrap_or_else(|| material.count.to_string()),
    }).collect())
}

fn passes(state: &State, item: &Item) -> bool {
    let needle = state.name.trim().to_lowercase();
    (needle.is_empty() || item.name.is_some_and(|n| n.to_lowercase().contains(&needle)) || item.id.to_string().contains(&needle))
        && state.known.is_none_or(|known| item.name.is_some() == known)
        && parse_u64(&state.count_min).is_none_or(|min| item.count >= min)
}

fn compare(a: &Item, b: &Item, sort: Sort) -> Ordering {
    match sort {
        Sort::Item => a.label().to_lowercase().cmp(&b.label().to_lowercase()),
        Sort::Id => a.id.cmp(&b.id),
        Sort::Quantity => a.count.cmp(&b.count),
    }
}

fn bulk_target(state: &State, item: &Item) -> Option<String> {
    let (kind, value) = state.op.as_ref()?;
    match kind {
        OpKind::Set => Some(value.trim().to_string()),
        OpKind::Add => item.count.checked_add(parse_u64(value)?).map(|v| v.to_string()),
        OpKind::Mul => apply_factor(&item.raw_count, parse_f64(value)?).ok(),
    }
}

/// Stage the quantity change for every selected stack.
fn apply(editor: &mut Editor) {
    let Some(session) = editor.session.as_mut() else { return };
    let Some(rows) = stacks(session) else { return };
    let state = &editor.inventory;
    let (mut staged, mut skipped) = (0, 0);
    for item in rows.iter().filter(|i| state.selected.contains(&i.index)) {
        let Some(target) = state.overrides.get(&item.index).cloned().or_else(|| bulk_target(state, item)) else { continue };
        if target.trim() == item.raw_count.trim() { continue; }
        // Edit by item id (selector path) so a lone-struct `X.Q` still works.
        let selector = format!("a={}", item.id);
        if target.trim().parse::<u64>().is_ok() && session.set_scalar(&["X", "Q", &selector, "b"], item.label(), target.trim()).is_ok() { staged += 1; } else { skipped += 1; }
    }
    editor.note = Some(if skipped > 0 { (format!("Staged {staged} edits ({skipped} skipped)"), staged == 0) } else { (format!("Staged {staged} quantity edits"), false) });
    let state = &mut editor.inventory;
    state.selected.clear();
    state.overrides.clear();
    state.op = None;
}

pub fn build(form: &mut F, session: &EditSession, state: &State) {
    form.title_with("Inventory", vec![
        button(form.on("inventory/add", |editor, _| editor.inventory.add = Some(Add { quantity: "1".into(), ..Default::default() })), "Add item").w(100.0),
        spacer(),
    ]);
    let Some(rows) = stacks(session) else {
        form.note("Typed inventory data unavailable — use the Raw Save Tree.");
        return;
    };
    let mut shown: Vec<&Item> = rows.iter().filter(|i| passes(state, i)).collect();
    if let Some((sort, ascending)) = state.sort {
        shown.sort_by(|a, b| { let order = compare(a, b, sort); if ascending { order } else { order.reverse() } });
    }

    let known = vec![(None, "Any".to_string()), (Some(true), "Named".to_string()), (Some(false), "Unknown id".to_string())];
    form.row(vec![
        muted("Filter:").w(44.0),
        text("name / id").w(64.0),
        input(form.on("inventory/filter/name", |editor, value| editor.inventory.name = value.to_string()), state.name.as_str()).live().w(150.0),
        pick(form, "inventory/filter/known", known, &state.known, |_| "Any".to_string(), |editor, known| editor.inventory.known = *known).w(120.0),
        muted("count ≥").w(52.0),
        input(form.on("inventory/filter/count-min", |editor, value| editor.inventory.count_min = value.to_string()), state.count_min.as_str()).live().w(90.0),
        button(form.on("inventory/filter/clear", |editor, _| {
            let state = &mut editor.inventory;
            (state.name, state.known, state.count_min) = (String::new(), None, String::new());
        }), "× clear").w(80.0),
        muted(format!("{} / {} items", shown.len(), rows.len())).size(11.0),
    ]);

    let visible: Vec<usize> = shown.iter().map(|i| i.index).collect();
    let has_ops = state.op.is_some() || !state.overrides.is_empty();
    let mut cells = vec![
        text(format!("{} selected", state.selected.len())).tone(Tone::Bright).w(90.0),
        button(form.on("inventory/select-all", move |editor, _| editor.inventory.selected.extend(visible.iter().copied())), "Select all (filtered)").w(170.0),
        button(form.on("inventory/select-none", |editor, _| editor.inventory.selected.clear()), "Clear selection").w(130.0),
        check(form.on("inventory/op", |editor, value| editor.inventory.op = (value == "1").then(|| (OpKind::Set, String::new()))), "Quantity", state.op.is_some()).w(90.0),
    ];
    if let Some((kind, value)) = &state.op {
        cells.push(bulk::op_picker(form, "inventory/op/kind", &[OpKind::Set, OpKind::Add, OpKind::Mul], *kind, |editor, kind| {
            if let Some(op) = editor.inventory.op.as_mut() { op.0 = kind; }
        }));
        cells.push(input(form.on("inventory/op/value", |editor, value| {
            if let Some(op) = editor.inventory.op.as_mut() { op.1 = value.to_string(); }
        }), value.as_str()).w(130.0));
    }
    cells.push(button(form.on("inventory/apply", |editor, _| apply(editor)), format!("Apply to {}", state.selected.len())).primary().enabled(!state.selected.is_empty() && has_ops).w(120.0));
    cells.push(spacer());
    form.row(cells);
    if let Some(add) = &state.add { add_form(form, add, &rows); }

    form.header(vec![
        spacer().w(30.0),
        bulk::sort_header(form, "inventory/sort/item", "Item", Sort::Item, state.sort, |e| &mut of(e).sort).w(240.0),
        bulk::sort_header(form, "inventory/sort/id", "Id", Sort::Id, state.sort, |e| &mut of(e).sort).w(70.0),
        bulk::sort_header(form, "inventory/sort/quantity", "Quantity", Sort::Quantity, state.sort, |e| &mut of(e).sort).w(174.0),
        spacer(),
    ]);
    form.pin();
    for item in shown {
        let selected = state.selected.contains(&item.index);
        let mut cells = vec![
            bulk::select_box(form, format!("inventory/select/{}", item.index), item.index, selected, |e| &mut of(e).selected),
            text(item.label()).tone(if item.name.is_some() { Tone::Normal } else { Tone::Warning }).w(240.0),
            muted(item.id.to_string()).mono().size(11.0).w(70.0),
        ];
        if selected {
            let default = bulk_target(state, item).unwrap_or_else(|| item.raw_count.clone());
            cells.extend(bulk::override_cells(form, format!("inventory/override/{}", item.index), item.index, &default, &item.raw_count, &state.overrides, |e| &mut of(e).overrides, 174.0));
        } else {
            cells.push(text(item.raw_count.as_str()).mono().size(11.0).w(174.0));
        }
        let (index, label) = (item.index, item.label());
        cells.push(button(form.on(format!("inventory/delete/{index}"), move |editor, _| {
            let Some(session) = editor.session.as_mut() else { return };
            if let Err(error) = session.delete_material(index, label.clone()) { editor.note = Some((format!("Delete failed: {error}"), true)); }
            // Row indices shift after a delete, so index-keyed picks no longer apply.
            editor.inventory.selected.clear();
            editor.inventory.overrides.clear();
        }), "×").w(40.0).hint("Delete this stack"));
        cells.push(spacer());
        form.row(cells).mark = if selected { Mark::Selected } else { Mark::None };
    }
}

fn add_form(form: &mut F, add: &Add, rows: &[Item]) {
    let owned: HashMap<u32, u64> = rows.iter().map(|i| (i.id, i.count)).collect();
    let needle = add.search.trim().to_lowercase();
    let id = add.id.trim().parse::<u32>().ok();
    // The known-item list, narrowed by the search box; picking one fills in the id.
    let mut choices: Vec<(Option<u32>, String)> = vec![(None, "pick…".into())];
    choices.extend(items::known_materials().into_iter().filter(|(_, name)| needle.is_empty() || name.to_lowercase().contains(&needle)).map(|(id, name)| {
        (Some(id), match owned.get(&id) { Some(count) => format!("{name}  (id {id}, owned {count})"), None => format!("{name}  (id {id})") })
    }));
    let current = id.filter(|id| choices.iter().any(|(choice, _)| *choice == Some(*id)));
    let field = |form: &F, id: &str, value: &str, set: fn(&mut Add, String)| input(form.on(id, move |editor, v| {
        if let Some(add) = editor.inventory.add.as_mut() { set(add, v.to_string()); }
    }), value).live();
    form.row(vec![
        text("Add item:").tone(Tone::Bright).w(70.0),
        muted("Known item:").w(74.0),
        field(form, "inventory/add/search", &add.search, |add, v| add.search = v).hint("search").w(120.0),
        pick(form, "inventory/add/known", choices, &current, |_| "pick…".to_string(), |editor, choice| {
            if let (Some(add), Some(id)) = (editor.inventory.add.as_mut(), choice) { add.id = id.to_string(); }
        }).w(300.0),
        spacer(),
    ]).mark = Mark::Selected;
    let valid = id.is_some() && add.quantity.trim().parse::<u64>().is_ok();
    let resolved = match id {
        Some(id) => format!("→ {}{}", items::material_name(id).unwrap_or("unknown id"), owned.get(&id).map(|c| format!(" — already own {c}")).unwrap_or_default()),
        None => "enter a numeric id".to_string(),
    };
    form.row(vec![
        gap(70.0),
        muted("Item id:").w(74.0),
        field(form, "inventory/add/id", &add.id, |add, v| add.id = v).w(120.0),
        muted("Quantity:").w(60.0),
        field(form, "inventory/add/quantity", &add.quantity, |add, v| add.quantity = v).w(130.0),
        button(form.on("inventory/add/confirm", |editor, _| {
            let Some(add) = editor.inventory.add.take() else { return };
            let (Some(session), Ok(id)) = (editor.session.as_mut(), add.id.trim().parse::<u32>()) else { return };
            let label = item_label(id);
            editor.note = Some(match session.set_material(id, add.quantity.trim(), label.clone()) {
                Ok(true) => (format!("Added {label} (id {id})"), false),
                Ok(false) => (format!("Updated {label} (id {id}) — already owned"), false),
                Err(error) => (format!("Add failed: {error}"), true),
            });
        }), "Add").primary().enabled(valid).w(70.0),
        button(form.on("inventory/add/cancel", |editor, _| editor.inventory.add = None), "Cancel").w(80.0),
        muted(resolved).size(11.0),
    ]).mark = Mark::Selected;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::tests::{fire, loaded, open};
    use save_parser::registry::SectionId;

    #[test]
    fn bulk_multiply_and_add_item_upsert() {
        let mut editor = loaded();
        open(&mut editor, SectionId::Inventory);
        let rows = stacks(editor.session.as_ref().unwrap()).unwrap();
        let (first, id, before) = (rows[0].index, rows[0].id, rows[0].count);
        fire(&mut editor, &format!("inventory/select/{first}"), "1");
        fire(&mut editor, "inventory/op", "1");
        fire(&mut editor, "inventory/op/kind", "2");
        fire(&mut editor, "inventory/op/value", "2");
        fire(&mut editor, "inventory/apply", "");
        let rows = stacks(editor.session.as_ref().unwrap()).unwrap();
        assert_eq!(rows[first].count, before * 2);
        assert_eq!(editor.note.as_ref().unwrap().0, "Staged 1 quantity edits");

        // Adding an owned item updates its stack rather than duplicating it.
        fire(&mut editor, "inventory/add", "");
        fire(&mut editor, "inventory/add/id", &id.to_string());
        fire(&mut editor, "inventory/add/quantity", "12345");
        fire(&mut editor, "inventory/add/confirm", "");
        let after = stacks(editor.session.as_ref().unwrap()).unwrap();
        assert_eq!(after.len(), rows.len());
        assert_eq!(after[first].count, 12345);
        assert!(editor.note.as_ref().unwrap().0.contains("already owned"));
        assert!(editor.inventory.add.is_none());

        fire(&mut editor, "inventory/filter/name", "zzz-nothing");
        assert!(crate::editor::build(&editor).rows.is_empty());
    }
}
