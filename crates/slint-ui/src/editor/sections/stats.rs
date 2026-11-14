//! The "count list" blocks that are just a list of entries with one or two
//! editable counts: **Physical** (`h`, level), **Skills** (`j`, level + usage
//! count), **Monsters** (`k`, defeated).
//!
//! One config-driven section serves all three. Each is keyed by list position,
//! so edits stage `set_scalar([list_key, <i>, …field])` — these lists are
//! fixed-size (never added to / deleted from), so indices are stable.
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use save_parser::edit::apply_factor;
use save_parser::items;
use save_parser::raw::Raw;
use save_parser::session::EditSession;

use crate::editor::bulk::{self, parse_f64, parse_u64, OpKind};
use crate::editor::common::{matches, F};
use crate::editor::Editor;
use crate::form::{button, check, input, muted, spacer, text, Mark, Tone};

/// Which block a section shows (also its slot in `Editor::stats`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Block { Physical, Skills, Monsters }

/// One editable count within an entry: its label and key path relative to the element.
type Field = (&'static str, &'static [&'static str]);

struct Config {
    title: &'static str,
    list_key: &'static str,
    name: fn(u32) -> Option<&'static str>,
    fields: &'static [Field],
}

fn config(block: Block) -> Config {
    match block {
        Block::Physical => Config { title: "Physical", list_key: "h", name: items::physical_training_name, fields: &[("Level", &["b"])] },
        Block::Skills => Config { title: "Skills", list_key: "j", name: items::skill_name, fields: &[("Level", &["b"]), ("Usage", &["e", "b"])] },
        Block::Monsters => Config { title: "Monsters", list_key: "k", name: items::monster_name, fields: &[("Defeated", &["b"])] },
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sort { Name, Field(usize) }

#[derive(Default)]
pub struct State {
    filter: String,
    selected: HashSet<usize>,
    /// Enabled bulk ops, keyed by field index.
    ops: HashMap<usize, (OpKind, String)>,
    /// Per-(row, field) overrides.
    overrides: HashMap<(usize, usize), String>,
    sort: Option<(Sort, bool)>,
}

fn of(editor: &mut Editor, block: Block) -> &mut State { &mut editor.stats[block as usize] }

struct Entry {
    index: usize,
    id: u32,
    name: Option<&'static str>,
    /// Raw value per field (`None` when the field is absent on this entry).
    values: Vec<Option<String>>,
}

impl Entry {
    fn display_name(&self) -> String { self.name.map_or_else(|| format!("#{}", self.id), str::to_string) }
    fn number(&self, field: usize) -> u64 {
        self.values.get(field).and_then(|v| v.as_ref()).and_then(|s| s.trim().parse().ok()).unwrap_or(0)
    }
}

fn entries(session: &EditSession, config: &Config) -> Vec<Entry> {
    // These blocks always have many entries, so they parse as real lists;
    // index-path edits rely on that (a lone struct would need a selector).
    let count = match session.root().get_path(&[config.list_key]) { Some(Raw::List(items)) => items.len(), _ => 0 };
    (0..count).map(|index| {
        let i = index.to_string();
        let id = session.value(&[config.list_key, &i, "a"]).and_then(|s| s.trim().parse().ok()).unwrap_or(index as u32);
        let values = config.fields.iter().map(|(_, sub)| {
            let mut path = vec![config.list_key, i.as_str()];
            path.extend_from_slice(sub);
            session.value(&path)
        }).collect();
        Entry { index, id, name: (config.name)(id), values }
    }).collect()
}

fn compare(a: &Entry, b: &Entry, sort: Sort) -> Ordering {
    match sort {
        Sort::Name => a.display_name().to_lowercase().cmp(&b.display_name().to_lowercase()),
        Sort::Field(field) => a.number(field).cmp(&b.number(field)),
    }
}

fn bulk_target(state: &State, entry: &Entry, field: usize) -> Option<String> {
    let (kind, value) = state.ops.get(&field)?;
    let current = entry.values.get(field)?.as_ref()?;
    match kind {
        OpKind::Set => Some(value.trim().to_string()),
        OpKind::Add => current.trim().parse::<u64>().ok()?.checked_add(parse_u64(value)?).map(|v| v.to_string()),
        OpKind::Mul => apply_factor(current, parse_f64(value)?).ok(),
    }
}

/// A per-row override wins over the bulk op.
fn effective_target(state: &State, entry: &Entry, field: usize) -> Option<String> {
    state.overrides.get(&(entry.index, field)).cloned().or_else(|| bulk_target(state, entry, field))
}

/// Stage the bulk ops / overrides for every selected entry.
fn apply(editor: &mut Editor, block: Block) {
    let config = config(block);
    let Some(session) = editor.session.as_mut() else { return };
    let rows = entries(session, &config);
    let state = &editor.stats[block as usize];
    let (mut staged, mut skipped) = (0, 0);
    for entry in rows.iter().filter(|e| state.selected.contains(&e.index)) {
        let i = entry.index.to_string();
        for (field, (label, sub)) in config.fields.iter().enumerate() {
            let Some(target) = effective_target(state, entry, field) else { continue };
            let current = entry.values.get(field).and_then(|v| v.as_deref()).unwrap_or("");
            if target.trim() == current.trim() { continue; }
            let mut path = vec![config.list_key, i.as_str()];
            path.extend_from_slice(sub);
            if target.trim().parse::<u64>().is_ok() && session.set_scalar(&path, format!("{} {label}", entry.display_name()), target.trim()).is_ok() {
                staged += 1;
            } else {
                skipped += 1;
            }
        }
    }
    editor.note = Some(if skipped > 0 { (format!("Staged {staged} edits ({skipped} skipped)"), staged == 0) } else { (format!("Staged {staged} edits"), false) });
    let state = of(editor, block);
    state.selected.clear();
    state.overrides.clear();
    state.ops.clear();
}

pub fn build(form: &mut F, session: &EditSession, state: &State, block: Block) {
    let config = config(block);
    let key = config.list_key;
    form.title(config.title);
    let rows = entries(session, &config);
    let needle = state.filter.trim().to_lowercase();
    let mut shown: Vec<&Entry> = rows.iter()
        .filter(|e| matches(&needle, e.name.unwrap_or("")) || (!needle.is_empty() && e.id.to_string().contains(&needle)))
        .collect();
    if let Some((sort, ascending)) = state.sort {
        shown.sort_by(|a, b| { let order = compare(a, b, sort); if ascending { order } else { order.reverse() } });
    }

    form.row(vec![
        muted("Filter:").w(44.0),
        text("name / id").w(64.0),
        input(form.on(format!("stats/{key}/filter"), move |editor, value| of(editor, block).filter = value.to_string()), state.filter.as_str()).live().w(160.0),
        button(form.on(format!("stats/{key}/filter-clear"), move |editor, _| of(editor, block).filter.clear()), "× clear").w(80.0),
        muted(format!("{} / {}", shown.len(), rows.len())).size(11.0),
    ]);

    let visible: Vec<usize> = shown.iter().map(|e| e.index).collect();
    let has_ops = !state.ops.is_empty() || !state.overrides.is_empty();
    form.row(vec![
        text(format!("{} selected", state.selected.len())).tone(Tone::Bright).w(90.0),
        button(form.on(format!("stats/{key}/select-all"), move |editor, _| of(editor, block).selected.extend(visible.iter().copied())), "Select all (filtered)").w(170.0),
        button(form.on(format!("stats/{key}/select-none"), move |editor, _| of(editor, block).selected.clear()), "Clear selection").w(130.0),
        button(form.on(format!("stats/{key}/apply"), move |editor, _| apply(editor, block)), format!("Apply to {}", state.selected.len()))
            .primary().enabled(!state.selected.is_empty() && has_ops).w(120.0),
        spacer(),
    ]);
    for (field, (label, _)) in config.fields.iter().enumerate() {
        let mut cells = vec![
            check(form.on(format!("stats/{key}/op/{field}"), move |editor, value| {
                let ops = &mut of(editor, block).ops;
                if value == "1" { ops.insert(field, (OpKind::Set, String::new())); } else { ops.remove(&field); }
            }), *label, state.ops.contains_key(&field)).w(110.0),
        ];
        if let Some((kind, value)) = state.ops.get(&field) {
            cells.push(bulk::op_picker(form, format!("stats/{key}/op/{field}/kind"), &[OpKind::Set, OpKind::Add, OpKind::Mul], *kind, move |editor, kind| {
                if let Some(op) = of(editor, block).ops.get_mut(&field) { op.0 = kind; }
            }));
            cells.push(input(form.on(format!("stats/{key}/op/{field}/value"), move |editor, value| {
                if let Some(op) = of(editor, block).ops.get_mut(&field) { op.1 = value.to_string(); }
            }), value.as_str()).w(130.0));
        }
        cells.push(spacer());
        form.row(cells);
    }

    let mut header = vec![
        spacer().w(30.0),
        bulk::sort_header(form, format!("stats/{key}/sort/name"), "Name", Sort::Name, state.sort, move |e| &mut of(e, block).sort).w(220.0),
        text("Id").w(54.0),
    ];
    for (field, (label, _)) in config.fields.iter().enumerate() {
        header.push(bulk::sort_header(form, format!("stats/{key}/sort/{field}"), label, Sort::Field(field), state.sort, move |e| &mut of(e, block).sort).w(174.0));
    }
    header.push(spacer());
    form.header(header);
    form.pin();

    for entry in shown {
        let selected = state.selected.contains(&entry.index);
        let mut cells = vec![
            bulk::select_box(form, format!("stats/{key}/select/{}", entry.index), entry.index, selected, move |e| &mut of(e, block).selected),
            text(entry.display_name()).tone(if entry.name.is_some() { Tone::Normal } else { Tone::Warning }).w(220.0),
            muted(entry.id.to_string()).mono().size(11.0).w(54.0),
        ];
        for field in 0..config.fields.len() {
            match entry.values.get(field).and_then(|v| v.as_ref()) {
                None => cells.push(muted("—").w(174.0)),
                Some(current) if selected => {
                    let default = bulk_target(state, entry, field).unwrap_or_else(|| current.clone());
                    cells.extend(bulk::override_cells(form, format!("stats/{key}/override/{}/{field}", entry.index), (entry.index, field),
                        &default, current, &state.overrides, move |e| &mut of(e, block).overrides, 174.0));
                }
                Some(current) => cells.push(text(current.as_str()).mono().size(11.0).w(174.0)),
            }
        }
        cells.push(spacer());
        form.row(cells).mark = if selected { Mark::Selected } else { Mark::None };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::tests::{fire, loaded, open};
    use save_parser::registry::SectionId;

    fn entry(level: &str, usage: Option<&str>) -> Entry {
        Entry { index: 0, id: 3, name: Some("Skill"), values: vec![Some(level.to_string()), usage.map(str::to_string)] }
    }

    #[test]
    fn bulk_ops_apply_per_field_and_overrides_win() {
        let mut state = State::default();
        state.ops.insert(0, (OpKind::Add, "5".into()));
        state.ops.insert(1, (OpKind::Mul, "1000".into()));
        let both = entry("10", Some("2"));
        assert_eq!(bulk_target(&state, &both, 0).as_deref(), Some("15"));
        assert_eq!(bulk_target(&state, &both, 1).as_deref(), Some("2000"));
        state.overrides.insert((0, 0), "50".into());
        let lone = entry("10", None);
        assert_eq!(effective_target(&state, &lone, 0).as_deref(), Some("50"));
        // Field 1 is absent on this entry → no target.
        assert_eq!(bulk_target(&state, &lone, 1), None);
    }

    #[test]
    fn select_set_and_apply_stages_through_the_form() {
        let mut editor = loaded();
        open(&mut editor, SectionId::Physical);
        fire(&mut editor, "stats/h/select/0", "1");
        fire(&mut editor, "stats/h/select/1", "1");
        fire(&mut editor, "stats/h/op/0", "1");
        fire(&mut editor, "stats/h/op/0/value", "4242");
        // A typed per-row value beats the bulk op for that row.
        fire(&mut editor, "stats/h/override/1/0", "7");
        fire(&mut editor, "stats/h/apply", "");
        let session = editor.session.as_ref().unwrap();
        assert_eq!(session.value(&["h", "0", "b"]).unwrap(), "4242");
        assert_eq!(session.value(&["h", "1", "b"]).unwrap(), "7");
        assert_eq!(editor.note.as_ref().unwrap().0, "Staged 2 edits");
        assert!(editor.stats[Block::Physical as usize].selected.is_empty());
        // The other blocks keep their own state.
        open(&mut editor, SectionId::Skills);
        fire(&mut editor, "stats/j/filter", "zzz-nothing");
        assert!(crate::editor::build(&editor).rows.is_empty());
        open(&mut editor, SectionId::Monsters);
        assert!(!crate::editor::build(&editor).rows.is_empty());
    }
}
