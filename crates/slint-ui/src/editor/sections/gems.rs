//! Gems: view/filter the loose-gem inventory (`X.002`) and edit counts. A stack
//! is keyed by (element, level). Gems embedded in equipment are not in this list.
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use save_parser::edit::apply_factor;
use save_parser::session::EditSession;

use crate::editor::bulk::{self, parse_f64, parse_u64, OpKind};
use crate::editor::common::{pick, F};
use crate::editor::Editor;
use crate::form::{button, check, input, muted, spacer, text, Mark, Tone};

/// (label, element id). Save element ids: 0 = Neutral … 4 = Wind.
pub const ELEMENTS: [(&str, u32); 5] = [("Neutral", 0), ("Fire", 1), ("Water", 2), ("Earth", 3), ("Wind", 4)];

pub fn element_name(id: u32) -> String {
    ELEMENTS.iter().find(|(_, i)| *i == id).map_or_else(|| format!("Element {id}"), |(label, _)| (*label).to_string())
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sort { Element, Level, Count }

struct Gem { index: usize, element: u32, level: u32, count: u64, raw_count: String }

pub struct Add { element: u32, level: String, count: String }

#[derive(Default)]
pub struct State {
    element: Option<u32>,
    level_min: String,
    level_max: String,
    count_min: String,
    selected: HashSet<usize>,
    op: Option<(OpKind, String)>,
    overrides: HashMap<usize, String>,
    sort: Option<(Sort, bool)>,
    add: Option<Add>,
}

fn of(editor: &mut Editor) -> &mut State { &mut editor.gems }

fn gems(session: &EditSession) -> Option<Vec<Gem>> {
    let save = session.derived()?;
    Some(save.gems.iter().enumerate().map(|(index, gem)| Gem {
        index, element: gem.element_id, level: gem.level, count: gem.count,
        raw_count: session.value(&["X", "002", &index.to_string(), "c"]).unwrap_or_else(|| gem.count.to_string()),
    }).collect())
}

fn passes(state: &State, gem: &Gem) -> bool {
    state.element.is_none_or(|e| gem.element == e)
        && parse_u64(&state.level_min).is_none_or(|low| u64::from(gem.level) >= low)
        && parse_u64(&state.level_max).is_none_or(|high| u64::from(gem.level) <= high)
        && parse_u64(&state.count_min).is_none_or(|min| gem.count >= min)
}

fn compare(a: &Gem, b: &Gem, sort: Sort) -> Ordering {
    match sort {
        Sort::Element => a.element.cmp(&b.element).then(a.level.cmp(&b.level)),
        Sort::Level => a.level.cmp(&b.level),
        Sort::Count => a.count.cmp(&b.count),
    }
}

fn bulk_target(state: &State, gem: &Gem) -> Option<String> {
    let (kind, value) = state.op.as_ref()?;
    match kind {
        OpKind::Set => Some(value.trim().to_string()),
        OpKind::Add => gem.count.checked_add(parse_u64(value)?).map(|v| v.to_string()),
        OpKind::Mul => apply_factor(&gem.raw_count, parse_f64(value)?).ok(),
    }
}

/// Stage the bulk op / overrides for every selected stack.
fn apply(editor: &mut Editor) {
    let Some(session) = editor.session.as_mut() else { return };
    let Some(rows) = gems(session) else { return };
    let state = &editor.gems;
    let (mut staged, mut skipped) = (0, 0);
    for gem in rows.iter().filter(|g| state.selected.contains(&g.index)) {
        let Some(target) = state.overrides.get(&gem.index).cloned().or_else(|| bulk_target(state, gem)) else { continue };
        if target.trim() == gem.raw_count.trim() { continue; }
        // `set_gem` upserts by element + level and normalizes a lone-struct
        // `X.002` to a list, so this works even with a single gem stack.
        let label = format!("{} L{} count", element_name(gem.element), gem.level);
        if target.trim().parse::<u64>().is_ok() && session.set_gem(gem.element, gem.level, target.trim(), label).is_ok() { staged += 1; } else { skipped += 1; }
    }
    editor.note = Some(if skipped > 0 { (format!("Staged {staged} edits ({skipped} skipped)"), staged == 0) } else { (format!("Staged {staged} count edits"), false) });
    let state = &mut editor.gems;
    state.selected.clear();
    state.overrides.clear();
    state.op = None;
}

pub fn build(form: &mut F, session: &EditSession, state: &State) {
    form.title_with("Gems", vec![
        button(form.on("gems/add", |editor, _| editor.gems.add = Some(Add { element: 1, level: "1".into(), count: "1".into() })), "Add gem").w(100.0),
        spacer(),
    ]);
    let Some(rows) = gems(session) else {
        form.note("Typed gem data unavailable — use the Raw Save Tree.");
        return;
    };
    let mut shown: Vec<&Gem> = rows.iter().filter(|g| passes(state, g)).collect();
    if let Some((sort, ascending)) = state.sort {
        shown.sort_by(|a, b| { let order = compare(a, b, sort); if ascending { order } else { order.reverse() } });
    }

    // Filters.
    let mut elements: Vec<(Option<u32>, String)> = vec![(None, "Any element".into())];
    elements.extend(ELEMENTS.iter().map(|(label, id)| (Some(*id), label.to_string())));
    let filter = |form: &F, id: &str, value: &str, set: fn(&mut State, String)| input(form.on(id, move |editor, v| set(&mut editor.gems, v.to_string())), value).live();
    form.row(vec![
        muted("Filter:").w(44.0),
        pick(form, "gems/filter/element", elements, &state.element, |e| element_name(e.unwrap_or(0)), |editor, e| editor.gems.element = *e).w(140.0),
        muted("Level").w(36.0),
        filter(form, "gems/filter/level-min", &state.level_min, |s, v| s.level_min = v).w(56.0),
        muted("–").w(10.0),
        filter(form, "gems/filter/level-max", &state.level_max, |s, v| s.level_max = v).w(56.0),
        muted("count ≥").w(52.0),
        filter(form, "gems/filter/count-min", &state.count_min, |s, v| s.count_min = v).w(90.0),
        button(form.on("gems/filter/clear", |editor, _| {
            let selected = std::mem::take(&mut editor.gems.selected);
            editor.gems = State { selected, ..Default::default() };
        }), "× clear").w(80.0),
        muted(format!("{} / {} stacks", shown.len(), rows.len())).size(11.0),
    ]);

    // Bulk edit.
    let visible: Vec<usize> = shown.iter().map(|g| g.index).collect();
    let has_ops = state.op.is_some() || !state.overrides.is_empty();
    let mut cells = vec![
        text(format!("{} selected", state.selected.len())).tone(Tone::Bright).w(90.0),
        button(form.on("gems/select-all", move |editor, _| editor.gems.selected.extend(visible.iter().copied())), "Select all (filtered)").w(170.0),
        button(form.on("gems/select-none", |editor, _| editor.gems.selected.clear()), "Clear selection").w(130.0),
        check(form.on("gems/op", |editor, value| editor.gems.op = (value == "1").then(|| (OpKind::Set, String::new()))), "Count", state.op.is_some()).w(80.0),
    ];
    if let Some((kind, value)) = &state.op {
        cells.push(bulk::op_picker(form, "gems/op/kind", &[OpKind::Set, OpKind::Add, OpKind::Mul], *kind, |editor, kind| {
            if let Some(op) = editor.gems.op.as_mut() { op.0 = kind; }
        }));
        cells.push(input(form.on("gems/op/value", |editor, value| {
            if let Some(op) = editor.gems.op.as_mut() { op.1 = value.to_string(); }
        }), value.as_str()).w(130.0));
    }
    cells.push(button(form.on("gems/apply", |editor, _| apply(editor)), format!("Apply to {}", state.selected.len())).primary().enabled(!state.selected.is_empty() && has_ops).w(120.0));
    cells.push(spacer());
    form.row(cells);
    if let Some(add) = &state.add { add_form(form, add); }

    form.header(vec![
        spacer().w(30.0),
        bulk::sort_header(form, "gems/sort/element", "Element", Sort::Element, state.sort, |e| &mut of(e).sort).w(130.0),
        bulk::sort_header(form, "gems/sort/level", "Level", Sort::Level, state.sort, |e| &mut of(e).sort).w(80.0),
        bulk::sort_header(form, "gems/sort/count", "Count", Sort::Count, state.sort, |e| &mut of(e).sort).w(174.0),
        spacer(),
    ]);
    form.pin();
    for gem in shown {
        let selected = state.selected.contains(&gem.index);
        let mut cells = vec![
            bulk::select_box(form, format!("gems/select/{}", gem.index), gem.index, selected, |e| &mut of(e).selected),
            text(element_name(gem.element)).w(130.0),
            text(gem.level.to_string()).mono().w(80.0),
        ];
        if selected {
            let default = bulk_target(state, gem).unwrap_or_else(|| gem.raw_count.clone());
            cells.extend(bulk::override_cells(form, format!("gems/override/{}", gem.index), gem.index, &default, &gem.raw_count, &state.overrides, |e| &mut of(e).overrides, 160.0));
        } else {
            cells.push(text(gem.raw_count.as_str()).mono().size(11.0).w(174.0));
        }
        let (index, label) = (gem.index, format!("{} L{}", element_name(gem.element), gem.level));
        cells.push(button(form.on(format!("gems/delete/{index}"), move |editor, _| {
            let Some(session) = editor.session.as_mut() else { return };
            if let Err(error) = session.delete_gem(index, label.clone()) { editor.note = Some((format!("Delete failed: {error}"), true)); }
            // Row indices shift after a delete, so index-keyed picks no longer apply.
            editor.gems.selected.clear();
            editor.gems.overrides.clear();
        }), "×").w(40.0).hint("Delete this stack"));
        cells.push(spacer());
        form.row(cells).mark = if selected { Mark::Selected } else { Mark::None };
    }
}

fn add_form(form: &mut F, add: &Add) {
    let elements = ELEMENTS.iter().map(|(label, id)| (*id, label.to_string())).collect();
    let level = add.level.trim().parse::<u32>().ok().filter(|l| *l <= 20);
    let valid = level.is_some() && add.count.trim().parse::<u64>().is_ok();
    let field = |form: &F, id: &str, value: &str, set: fn(&mut Add, String)| input(form.on(id, move |editor, v| {
        if let Some(add) = editor.gems.add.as_mut() { set(add, v.to_string()); }
    }), value).live();
    form.row(vec![
        text("Add gem:").tone(Tone::Bright).w(70.0),
        muted("Element:").w(56.0),
        pick(form, "gems/add/element", elements, &add.element, |e| element_name(*e), |editor, element| {
            if let Some(add) = editor.gems.add.as_mut() { add.element = *element; }
        }).w(120.0),
        muted("Level (0–20):").w(86.0),
        field(form, "gems/add/level", &add.level, |add, v| add.level = v).w(60.0),
        muted("Count:").w(44.0),
        field(form, "gems/add/count", &add.count, |add, v| add.count = v).w(130.0),
        button(form.on("gems/add/confirm", |editor, _| {
            let Some(add) = editor.gems.add.take() else { return };
            let (Some(session), Ok(level)) = (editor.session.as_mut(), add.level.trim().parse::<u32>()) else { return };
            let label = format!("{} L{level}", element_name(add.element));
            editor.note = Some(match session.set_gem(add.element, level, add.count.trim(), label.clone()) {
                Ok(true) => (format!("Added {label}"), false),
                Ok(false) => (format!("Updated {label} (already owned)"), false),
                Err(error) => (format!("Add failed: {error}"), true),
            });
        }), "Add").primary().enabled(valid).w(70.0),
        button(form.on("gems/add/cancel", |editor, _| editor.gems.add = None), "Cancel").w(80.0),
        spacer(),
    ]).mark = Mark::Selected;
}
