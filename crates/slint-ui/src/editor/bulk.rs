//! Shared mechanics of the table-style bulk-edit sections (pets, equipment,
//! inventory, gems, stats): the op kinds, sortable column headers, row
//! selection, and the editable per-row override cell. Filters, columns and
//! field sets stay per-section.
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use super::common::{pick, F};
use super::Editor;
use crate::form::{check, gap, input, link, text, Cell, Tone};

/// How a bulk numeric op transforms the current value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OpKind {
    #[default]
    Set,
    /// Multiply (growth-like values only).
    Mul,
    Add,
}

pub fn op_label(kind: OpKind) -> &'static str {
    match kind { OpKind::Set => "Set", OpKind::Mul => "× Mul", OpKind::Add => "+ Add" }
}

/// A picker over `kinds`; choosing one hands it to `set`.
pub fn op_picker(form: &F, id: impl Into<String>, kinds: &[OpKind], current: OpKind, set: impl Fn(&mut Editor, OpKind) + 'static) -> Cell {
    let choices = kinds.iter().map(|k| (*k, op_label(*k).to_string())).collect();
    pick(form, id, choices, &current, |k| op_label(*k).to_string(), move |editor, kind| set(editor, *kind)).w(86.0)
}

/// Cycle a column's sort: none → ascending → descending → none.
pub fn cycle_sort<C: PartialEq + Copy>(sort: &mut Option<(C, bool)>, column: C) {
    *sort = match *sort {
        Some((c, true)) if c == column => Some((column, false)),
        Some((c, false)) if c == column => None,
        _ => Some((column, true)),
    };
}

/// A clickable column title showing the current sort arrow.
pub fn sort_header<C: PartialEq + Copy + 'static>(form: &F, id: impl Into<String>, title: &str, column: C, current: Option<(C, bool)>,
    sort: fn(&mut Editor) -> &mut Option<(C, bool)>) -> Cell {
    let arrow = match current {
        Some((c, true)) if c == column => " ▲",
        Some((c, false)) if c == column => " ▼",
        _ => "",
    };
    link(form.on(id, move |editor, _| cycle_sort(sort(editor), column)), format!("{title}{arrow}")).hint("Click to sort (asc → desc → off)")
}

/// A row's selection checkbox.
pub fn select_box<K: Clone + Eq + Hash + 'static>(form: &F, id: impl Into<String>, key: K, selected: bool, set: fn(&mut Editor) -> &mut HashSet<K>) -> Cell {
    check(form.on(id, move |editor, value| {
        if value == "1" { set(editor).insert(key.clone()); } else { set(editor).remove(&key); }
    }), "", selected).w(30.0)
}

/// An editable override cell plus its change marker. Shows `default` (the bulk-op
/// result) until something else is typed, which becomes a per-row override;
/// typing `default` again clears it. The ● marks a value that differs from
/// `current` (the stored one).
pub fn override_cells<K: Clone + Eq + Hash + 'static>(form: &F, id: impl Into<String>, key: K, default: &str, current: &str,
    overrides: &HashMap<K, String>, map: fn(&mut Editor) -> &mut HashMap<K, String>, width: f32) -> [Cell; 2] {
    let shown = overrides.get(&key).cloned().unwrap_or_else(|| default.to_string());
    let fallback = default.to_string();
    let id = form.on(id, move |editor, value| {
        if value.trim() == fallback.trim() { map(editor).remove(&key); } else { map(editor).insert(key.clone(), value.to_string()); }
    });
    let marker = if shown.trim() != current.trim() { text("●").tone(Tone::Success).w(14.0).hint(format!("{current} → {shown}")) } else { gap(14.0) };
    [input(id, shown).mono().w(width), marker]
}

/// A blank or unparsable filter bound matches everything.
pub fn parse_u64(text: &str) -> Option<u64> {
    let t = text.trim();
    (!t.is_empty()).then(|| t.parse().ok()).flatten()
}

pub fn parse_f64(text: &str) -> Option<f64> {
    let t = text.trim();
    (!t.is_empty()).then(|| itrtg_models::parse_flexible_number(t)).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_cycles_and_switches_columns() {
        let mut sort = None;
        cycle_sort(&mut sort, 1);
        assert_eq!(sort, Some((1, true)));
        cycle_sort(&mut sort, 1);
        assert_eq!(sort, Some((1, false)));
        cycle_sort(&mut sort, 2);
        assert_eq!(sort, Some((2, true)));
        cycle_sort(&mut sort, 2);
        cycle_sort(&mut sort, 2);
        assert_eq!(sort, None);
        assert_eq!(parse_u64(" 12 "), Some(12));
        assert_eq!(parse_u64(""), None);
        assert_eq!(parse_f64("1.5e3"), Some(1500.0));
    }
}
