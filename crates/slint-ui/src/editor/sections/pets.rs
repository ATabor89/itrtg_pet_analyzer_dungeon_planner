//! Pets: filter, multi-select and **staged bulk edits** with per-pet overrides.
//!
//! Filter the roster, select pets, configure bulk ops (× growth, set a level,
//! set a class) and optionally type a different value into an individual pet's
//! cell — that per-pet **override wins** over the bulk op for that field.
//! "Apply" stages the changes into the pending list (paths `X.b.<index>.…`);
//! nothing is written until Save As.
//!
//! Display reads the typed `derived().pets` (in `X.b` order, so the index is the
//! raw index).
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use itrtg_models::{Class, Element};
use save_parser::edit::{apply_delta, apply_factor};
use save_parser::model::EquipmentItem;
use save_parser::session::EditSession;

use super::equip_builder::{self, Builder, Built, Mode};
use crate::editor::bulk::{self, parse_f64, parse_u64, OpKind};
use crate::editor::common::{in_range, matches, owned, pick, refs, tri, F};
use crate::editor::Editor;
use crate::form::{button, check, gap, input, muted, spacer, text, Cell, Mark, Tone};

/// The bulk-editable numeric fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field { Growth, Normal, Dungeon, ClassLevel }

impl Field {
    const ALL: [Field; 4] = [Field::Growth, Field::Normal, Field::Dungeon, Field::ClassLevel];

    fn label(self) -> &'static str {
        match self { Field::Growth => "Growth", Field::Normal => "Normal Lvl", Field::Dungeon => "Dungeon Lvl", Field::ClassLevel => "Class Lvl" }
    }

    fn slug(self) -> &'static str {
        match self { Field::Growth => "growth", Field::Normal => "normal", Field::Dungeon => "dungeon", Field::ClassLevel => "class-level" }
    }

    /// Raw key path relative to `X.b.<i>`.
    fn keys(self) -> &'static [&'static str] {
        match self { Field::Growth => &["E"], Field::Normal => &["g"], Field::Dungeon => &["w", "b"], Field::ClassLevel => &["w", "d", "b"] }
    }

    /// Ops offered for this field (growth can multiply *or* add a flat amount).
    fn ops(self) -> &'static [OpKind] {
        match self { Field::Growth => &[OpKind::Set, OpKind::Mul, OpKind::Add], _ => &[OpKind::Set, OpKind::Add] }
    }

    /// Growth accepts any number, levels whole numbers.
    fn valid(self, value: &str) -> bool {
        match self { Field::Growth => parse_f64(value).is_some(), _ => parse_u64(value).is_some() }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sort { Name, Element, Class, Field(Field) }

/// (label, save class id). Classless is id 0.
const CLASSES: [(&str, u32); 9] = [
    ("Classless", 0), ("Blacksmith", 1), ("Alchemist", 2), ("Adventurer", 3), ("Defender", 4),
    ("Supporter", 5), ("Rogue", 6), ("Assassin", 7), ("Mage", 8),
];
const ELEMENTS: [Element; 5] = [Element::Fire, Element::Water, Element::Wind, Element::Earth, Element::Neutral];

fn classes() -> Vec<(u32, String)> { CLASSES.iter().map(|(label, id)| (*id, label.to_string())).collect() }

fn class_id(class: Class) -> u32 {
    match class {
        Class::Blacksmith => 1, Class::Alchemist => 2, Class::Adventurer => 3, Class::Defender => 4,
        Class::Supporter => 5, Class::Rogue => 6, Class::Assassin => 7, Class::Mage => 8, Class::Wildcard => 0,
    }
}

fn class_label(id: u32) -> &'static str { CLASSES.iter().find(|(_, i)| *i == id).map_or("?", |(label, _)| *label) }

fn element_label(element: Element) -> &'static str {
    match element {
        Element::Fire => "Fire", Element::Water => "Water", Element::Wind => "Wind",
        Element::Earth => "Earth", Element::Neutral => "Neutral", Element::All => "All",
    }
}

/// Stable order for the element column (`None` sorts last).
fn element_order(element: Option<Element>) -> u8 {
    match element {
        Some(Element::Fire) => 0, Some(Element::Water) => 1, Some(Element::Wind) => 2,
        Some(Element::Earth) => 3, Some(Element::Neutral) => 4, Some(Element::All) => 5, None => 6,
    }
}

/// "Name Quality+Plus" (e.g. "Magic Stick SSS+20"); the `+plus` is omitted at +0.
fn equip_label(item: &EquipmentItem) -> String {
    let mut label = item.type_name().unwrap_or("Equipment").to_string();
    if let Some(quality) = item.quality_name() { label.push(' '); label.push_str(quality); }
    if item.plus > 0 { label.push_str(&format!("+{}", item.plus)); }
    label
}

/// A snapshot of one pet.
struct Pet {
    index: usize,
    name: String,
    element: Option<Element>,
    class_id: u32,
    unlocked: bool,
    growth: f64,
    /// The exact stored growth — display, the multiply base and the skip-compare
    /// all use it, so none of them round-trip through f64.
    raw_growth: String,
    normal: u32,
    dungeon: u32,
    class_level: u32,
    /// Equipped items (weapon, armor, accessory); `None` for an empty slot.
    gear: [Option<String>; 3],
}

impl Pet {
    fn current(&self, field: Field) -> String {
        match field {
            Field::Growth => self.raw_growth.clone(),
            Field::Normal => self.normal.to_string(),
            Field::Dungeon => self.dungeon.to_string(),
            Field::ClassLevel => self.class_level.to_string(),
        }
    }

    fn equipped(&self) -> bool { self.gear.iter().any(Option::is_some) }
}

#[derive(Default)]
pub struct Filters {
    element: Option<Element>,
    class: Option<u32>,
    unlocked: Option<bool>,
    /// A pet has a class iff it is evolved (the only way to get one).
    evolved: Option<bool>,
    equipped: Option<bool>,
    name: String,
    dungeon_min: String,
    dungeon_max: String,
    class_min: String,
    class_max: String,
    growth_min: String,
    growth_max: String,
}

#[derive(Default)]
pub struct State {
    filters: Filters,
    selected: HashSet<usize>,
    /// Enabled bulk ops: (op kind, value text).
    ops: HashMap<Field, (OpKind, String)>,
    /// Bulk "set class" (save class id); `None` leaves the class alone.
    op_class: Option<u32>,
    /// Bulk unlock (`true`) / lock (`false`); `None` leaves it alone.
    op_unlocked: Option<bool>,
    /// Per-pet field overrides (explicit values that beat the bulk op).
    overrides: HashMap<(usize, Field), String>,
    class_overrides: HashMap<usize, u32>,
    sort: Option<(Sort, bool)>,
    /// The shared equipment builder, used here in give-to-pets mode.
    builder: Option<Builder>,
}

fn of(editor: &mut Editor) -> &mut State { &mut editor.pets }

fn roster(session: &EditSession) -> Option<Vec<Pet>> {
    let save = session.derived()?;
    Some(save.pets.iter().enumerate().map(|(index, pet)| {
        // Resolve each equip slot ref to its item via `X.R`.
        let gear = [pet.weapon_id, pet.armor_id, pet.accessory_id].map(|id| id.and_then(|d| save.equipment_by_instance_id(d)).map(equip_label));
        Pet {
            index, name: pet.name.clone(), element: pet.element, class_id: pet.class.map(class_id).unwrap_or(0), unlocked: pet.unlocked,
            growth: pet.growth,
            raw_growth: session.value(&["X", "b", &index.to_string(), "E"]).unwrap_or_else(|| pet.growth.to_string()),
            normal: pet.normal_level, dungeon: pet.dungeon_level, class_level: pet.class_level, gear,
        }
    }).collect())
}

fn passes(filters: &Filters, pet: &Pet) -> bool {
    filters.element.is_none_or(|e| pet.element == Some(e))
        && filters.class.is_none_or(|c| pet.class_id == c)
        && filters.unlocked.is_none_or(|u| pet.unlocked == u)
        && filters.evolved.is_none_or(|e| (pet.class_id != 0) == e)
        && filters.equipped.is_none_or(|e| pet.equipped() == e)
        && matches(&filters.name.trim().to_lowercase(), &pet.name)
        && in_range(pet.dungeon, &filters.dungeon_min, &filters.dungeon_max)
        && in_range(pet.class_level, &filters.class_min, &filters.class_max)
        && parse_f64(&filters.growth_min).is_none_or(|min| pet.growth >= min)
        && parse_f64(&filters.growth_max).is_none_or(|max| pet.growth <= max)
}

fn compare(a: &Pet, b: &Pet, sort: Sort) -> Ordering {
    match sort {
        Sort::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        Sort::Element => element_order(a.element).cmp(&element_order(b.element)),
        Sort::Class => a.class_id.cmp(&b.class_id),
        Sort::Field(Field::Growth) => a.growth.partial_cmp(&b.growth).unwrap_or(Ordering::Equal),
        Sort::Field(Field::Normal) => a.normal.cmp(&b.normal),
        Sort::Field(Field::Dungeon) => a.dungeon.cmp(&b.dungeon),
        Sort::Field(Field::ClassLevel) => a.class_level.cmp(&b.class_level),
    }
}

/// The bulk-op result for a field (no per-pet override), or `None` if no op is configured.
fn bulk_target(state: &State, pet: &Pet, field: Field) -> Option<String> {
    let (kind, value) = state.ops.get(&field)?;
    match (kind, field) {
        (OpKind::Set, _) => Some(value.trim().to_string()),
        // Mul applies only to growth.
        (OpKind::Mul, _) => apply_factor(&pet.raw_growth, parse_f64(value)?).ok(),
        // Growth + a flat amount (fractional ok); levels + a whole number (overflow-safe).
        (OpKind::Add, Field::Growth) => apply_delta(&pet.raw_growth, parse_f64(value)?).ok(),
        (OpKind::Add, _) => pet.current(field).parse::<u64>().ok()?.checked_add(parse_u64(value)?).map(|v| v.to_string()),
    }
}

/// The effective new value for a field: a per-pet override beats the bulk op.
fn effective_target(state: &State, pet: &Pet, field: Field) -> Option<String> {
    state.overrides.get(&(pet.index, field)).cloned().or_else(|| bulk_target(state, pet, field))
}

/// Stage every configured change for the selected pets.
fn apply(editor: &mut Editor) {
    let Some(session) = editor.session.as_mut() else { return };
    let Some(rows) = roster(session) else { return };
    let state = &editor.pets;
    // (path, label, value) per change; an invalid value counts as skipped.
    let mut edits: Vec<(Vec<String>, String, String)> = Vec::new();
    let mut skipped = 0;
    for pet in rows.iter().filter(|p| state.selected.contains(&p.index)) {
        let i = pet.index.to_string();
        for field in Field::ALL {
            let Some(target) = effective_target(state, pet, field) else { continue };
            if target.trim() == pet.current(field).trim() { continue; }
            if !field.valid(&target) { skipped += 1; continue; }
            let mut path = owned(&["X", "b", &i]);
            path.extend(field.keys().iter().map(|k| k.to_string()));
            edits.push((path, format!("{} · {}", pet.name, field.label()), target.trim().to_string()));
        }
        if let Some(class) = state.class_overrides.get(&pet.index).copied().or(state.op_class) && class != pet.class_id {
            edits.push((owned(&["X", "b", &i, "w", "d", "a"]), format!("{} · Class", pet.name), class.to_string()));
        }
        // Unlocked is the `l` bool, stored as "True"/"False".
        if let Some(want) = state.op_unlocked && want != pet.unlocked {
            edits.push((owned(&["X", "b", &i, "l"]), format!("{} · {}", pet.name, if want { "Unlock" } else { "Lock" }), if want { "True" } else { "False" }.to_string()));
        }
    }
    let staged = edits.iter().filter(|(path, label, value)| session.set_scalar(&refs(path), label.as_str(), value).is_ok()).count();
    skipped += edits.len() - staged;
    editor.note = Some(if skipped > 0 {
        (format!("Staged {staged} edits ({skipped} skipped — invalid or unreachable)"), staged == 0)
    } else {
        (format!("Staged {staged} edits across selected pets"), false)
    });
    let state = &mut editor.pets;
    state.selected.clear();
    state.overrides.clear();
    state.class_overrides.clear();
    state.ops.clear();
    state.op_class = None;
    state.op_unlocked = None;
}

/// Give one new equipment instance to each selected pet, equipped in the chosen slot.
fn give(editor: &mut Editor, built: Built) {
    let (Some(session), Some(slot)) = (editor.session.as_mut(), built.slot_key) else { return };
    let Some(rows) = roster(session) else { return };
    let mut selected: Vec<usize> = editor.pets.selected.iter().copied().collect();
    selected.sort_unstable();
    let (total, label) = (selected.len(), built.label());
    let given = selected.into_iter().filter(|index| {
        let pet = rows.get(*index).map_or("pet", |p| p.name.as_str());
        session.add_equipment(built.type_id, built.plus, built.quality, built.gem_level, built.gem_element, format!("{label} → {pet}"), Some((*index, slot))).is_ok()
    }).count();
    let name = save_parser::items::equipment_type_name(built.type_id).unwrap_or("Equipment");
    editor.note = Some(if given < total {
        (format!("Gave {name} to {given}/{total} pets ({} failed)", total - given), true)
    } else {
        (format!("Gave {name} to {given} pets — see Pending changes"), false)
    });
}

/// One bulk op: its checkbox plus, when enabled, the op picker and value (padded
/// to a fixed width so two ops sit side by side).
fn op_cells(form: &F, state: &State, field: Field) -> Vec<Cell> {
    let slug = field.slug();
    let mut cells = vec![check(form.on(format!("pets/op/{slug}"), move |editor, value| {
        if value == "1" { editor.pets.ops.insert(field, (OpKind::Set, String::new())); } else { editor.pets.ops.remove(&field); }
    }), field.label(), state.ops.contains_key(&field)).w(110.0)];
    match state.ops.get(&field) {
        Some((kind, value)) => {
            cells.push(bulk::op_picker(form, format!("pets/op/{slug}/kind"), field.ops(), *kind, move |editor, kind| {
                if let Some(op) = editor.pets.ops.get_mut(&field) { op.0 = kind; }
            }));
            cells.push(input(form.on(format!("pets/op/{slug}/value"), move |editor, value| {
                if let Some(op) = editor.pets.ops.get_mut(&field) { op.1 = value.to_string(); }
            }), value.as_str()).w(110.0));
        }
        None => cells.push(gap(204.0)),
    }
    cells.push(gap(16.0));
    cells
}

pub fn build(form: &mut F, session: &EditSession, state: &State) {
    form.title("Pets");
    let Some(rows) = roster(session) else {
        form.note("Typed pet data unavailable for this save — use the Raw Save Tree.");
        return;
    };
    let mut shown: Vec<&Pet> = rows.iter().filter(|p| passes(&state.filters, p)).collect();
    if let Some((sort, ascending)) = state.sort {
        shown.sort_by(|a, b| { let order = compare(a, b, sort); if ascending { order } else { order.reverse() } });
    }

    // Filters. Each picker gets a label — without one the Unlocked and Evolved
    // filters would both read "Any".
    let filters = &state.filters;
    let label = |text: &str, width: f32| muted(text).size(11.0).w(width);
    let typed = |form: &F, id: &str, value: &str, set: fn(&mut Filters, String)| input(form.on(format!("pets/filter/{id}"), move |editor, v| set(&mut editor.pets.filters, v.to_string())), value).live();
    let mut elements: Vec<(Option<Element>, String)> = vec![(None, "Any".into())];
    elements.extend(ELEMENTS.iter().map(|e| (Some(*e), element_label(*e).to_string())));
    let mut class_choices: Vec<(Option<u32>, String)> = vec![(None, "Any".into())];
    class_choices.extend(CLASSES.iter().map(|(label, id)| (Some(*id), label.to_string())));
    form.row(vec![
        muted("Filter:").w(44.0),
        label("Element", 46.0),
        pick(form, "pets/filter/element", elements, &filters.element, |_| String::new(), |editor, e| editor.pets.filters.element = *e).w(100.0),
        label("Class", 30.0),
        pick(form, "pets/filter/class", class_choices, &filters.class, |_| String::new(), |editor, c| editor.pets.filters.class = *c).w(116.0),
        label("Unlocked", 50.0),
        tri(form, "pets/filter/unlocked", ["Any", "Unlocked", "Locked"], filters.unlocked, |editor, v| editor.pets.filters.unlocked = v).w(106.0),
        label("Evolved", 44.0),
        tri(form, "pets/filter/evolved", ["Any", "Evolved", "Not evolved"], filters.evolved, |editor, v| editor.pets.filters.evolved = v).w(116.0),
        label("Equip", 32.0),
        tri(form, "pets/filter/equipped", ["Any", "Equipped", "None"], filters.equipped, |editor, v| editor.pets.filters.equipped = v).w(106.0),
        label("Name", 32.0),
        typed(form, "name", &filters.name, |f, v| f.name = v).w(120.0),
        spacer(),
    ]);
    form.row(vec![
        muted("Ranges:").w(44.0),
        label("Dungeon min", 68.0), typed(form, "dungeon-min", &filters.dungeon_min, |f, v| f.dungeon_min = v).w(56.0),
        label("max", 24.0), typed(form, "dungeon-max", &filters.dungeon_max, |f, v| f.dungeon_max = v).w(56.0),
        label("Class min", 54.0), typed(form, "class-min", &filters.class_min, |f, v| f.class_min = v).w(56.0),
        label("max", 24.0), typed(form, "class-max", &filters.class_max, |f, v| f.class_max = v).w(56.0),
        label("Growth ≥", 52.0), typed(form, "growth-min", &filters.growth_min, |f, v| f.growth_min = v).w(86.0),
        label("≤", 10.0), typed(form, "growth-max", &filters.growth_max, |f, v| f.growth_max = v).w(86.0),
        button(form.on("pets/filter/clear", |editor, _| editor.pets.filters = Filters::default()), "× clear").w(80.0),
        muted(format!("{} / {} pets", shown.len(), rows.len())).size(11.0),
    ]);

    // Bulk edit.
    let visible: Vec<usize> = shown.iter().map(|p| p.index).collect();
    let count = state.selected.len();
    form.row(vec![
        text(format!("{count} selected")).tone(Tone::Bright).w(90.0),
        button(form.on("pets/select-all", move |editor, _| editor.pets.selected.extend(visible.iter().copied())), "Select all (filtered)").w(170.0),
        button(form.on("pets/select-none", |editor, _| editor.pets.selected.clear()), "Clear selection").w(130.0),
        muted("Bulk ops apply to selected pets; a value you type into a pet's own cell wins.").size(11.0),
    ]);
    for pair in Field::ALL.chunks(2) {
        let mut cells = Vec::new();
        for field in pair { cells.extend(op_cells(form, state, *field)); }
        cells.push(spacer());
        form.row(cells);
    }
    let mut cells = vec![check(form.on("pets/op/class", |editor, value| editor.pets.op_class = (value == "1").then_some(0)), "Class", state.op_class.is_some()).w(110.0)];
    match state.op_class {
        Some(class) => {
            cells.push(pick(form, "pets/op/class/value", classes(), &class, |id| format!("Class {id}"), |editor, class| editor.pets.op_class = Some(*class)).w(130.0));
            cells.push(gap(66.0));
        }
        None => cells.push(gap(204.0)),
    }
    cells.push(gap(16.0));
    cells.push(check(form.on("pets/op/unlocked", |editor, value| editor.pets.op_unlocked = (value == "1").then_some(true)), "Unlocked", state.op_unlocked.is_some()).w(110.0));
    match state.op_unlocked {
        Some(unlock) => {
            cells.push(pick(form, "pets/op/unlocked/value", vec![(true, "Unlock".to_string()), (false, "Lock".to_string())], &unlock, |_| String::new(),
                |editor, unlock| editor.pets.op_unlocked = Some(*unlock)).w(100.0));
            cells.push(gap(96.0));
        }
        None => cells.push(gap(204.0)),
    }
    let has_ops = !state.ops.is_empty() || state.op_class.is_some() || state.op_unlocked.is_some() || !state.overrides.is_empty() || !state.class_overrides.is_empty();
    cells.extend([
        gap(16.0),
        button(form.on("pets/apply", |editor, _| apply(editor)), format!("Apply to {count} pets")).primary().enabled(count > 0 && has_ops).w(140.0),
        button(form.on("pets/give", |editor, _| editor.pets.builder = Some(Builder::default())), format!("Give equipment to {count}…")).enabled(count > 0).w(190.0)
            .hint("Create one new item per selected pet and equip it"),
        spacer(),
    ]);
    form.row(cells);
    if let Some(builder) = &state.builder {
        equip_builder::rows(form, builder, Mode::Give { count }, "pets/builder", |editor| &mut editor.pets.builder, give);
    }

    let mut header = vec![
        spacer().w(30.0),
        bulk::sort_header(form, "pets/sort/name", "Name", Sort::Name, state.sort, |e| &mut of(e).sort).w(150.0),
        bulk::sort_header(form, "pets/sort/element", "Elem", Sort::Element, state.sort, |e| &mut of(e).sort).w(64.0),
        bulk::sort_header(form, "pets/sort/class", "Class", Sort::Class, state.sort, |e| &mut of(e).sort).w(116.0),
    ];
    let widths = [170.0, 110.0, 110.0, 100.0];
    for (field, width) in Field::ALL.into_iter().zip(widths) {
        let title = match field { Field::Growth => "Growth", Field::Normal => "Normal", Field::Dungeon => "Dungeon", Field::ClassLevel => "Class Lvl" };
        header.push(bulk::sort_header(form, format!("pets/sort/{}", field.slug()), title, Sort::Field(field), state.sort, |e| &mut of(e).sort).w(width));
    }
    header.push(text("Equipment"));
    form.header(header);
    form.pin();

    for pet in shown {
        let selected = state.selected.contains(&pet.index);
        let mut cells = vec![
            bulk::select_box(form, format!("pets/select/{}", pet.index), pet.index, selected, |e| &mut of(e).selected),
            text(pet.name.as_str()).tone(if pet.unlocked { Tone::Normal } else { Tone::Muted }).w(150.0),
            text(pet.element.map_or("—", element_label)).w(64.0),
        ];
        if selected {
            // The class picker defaults to the bulk op; choosing anything else is
            // a per-pet override.
            let default = state.op_class.unwrap_or(pet.class_id);
            let current = state.class_overrides.get(&pet.index).copied().unwrap_or(default);
            let index = pet.index;
            cells.push(pick(form, format!("pets/class/{index}"), classes(), &current, |id| format!("Class {id}"), move |editor, class| {
                if *class == default { editor.pets.class_overrides.remove(&index); } else { editor.pets.class_overrides.insert(index, *class); }
            }).w(116.0));
        } else {
            cells.push(text(class_label(pet.class_id)).w(116.0));
        }
        for (field, width) in Field::ALL.into_iter().zip(widths) {
            let current = pet.current(field);
            if selected {
                let default = bulk_target(state, pet, field).unwrap_or_else(|| current.clone());
                cells.extend(bulk::override_cells(form, format!("pets/override/{}/{}", pet.index, field.slug()), (pet.index, field), &default, &current,
                    &state.overrides, |e| &mut of(e).overrides, width));
            } else {
                cells.push(text(current).mono().size(11.0).w(width));
            }
        }
        // A compact one-line summary of the filled slots; the hint has every slot.
        cells.push(if pet.equipped() {
            let slot = |index: usize| pet.gear[index].as_deref().unwrap_or("—");
            text(pet.gear.iter().flatten().map(String::as_str).collect::<Vec<_>>().join(" · ")).size(11.0)
                .hint(format!("Weapon: {} · Armor: {} · Accessory: {}", slot(0), slot(1), slot(2)))
        } else {
            muted("—").size(11.0)
        });
        form.row(cells).mark = if selected { Mark::Selected } else { Mark::None };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::tests::{fire, loaded, open};
    use save_parser::registry::SectionId;

    fn pet() -> Pet {
        Pet {
            index: 3, name: "Robot".into(), element: Some(Element::Fire), class_id: 2, unlocked: true, growth: 100.0,
            raw_growth: "100".into(), normal: 10, dungeon: 20, class_level: 5, gear: [None, None, None],
        }
    }

    #[test]
    fn bulk_ops_and_overrides() {
        let mut state = State::default();
        assert_eq!(bulk_target(&state, &pet(), Field::Normal), None);
        state.ops.insert(Field::Normal, (OpKind::Set, "50".into()));
        assert_eq!(bulk_target(&state, &pet(), Field::Normal).as_deref(), Some("50"));
        state.ops.insert(Field::Dungeon, (OpKind::Add, "5".into()));
        assert_eq!(bulk_target(&state, &pet(), Field::Dungeon).as_deref(), Some("25"));
        state.ops.insert(Field::Growth, (OpKind::Add, "50".into()));
        assert_eq!(bulk_target(&state, &pet(), Field::Growth).as_deref(), Some("150"));
        state.ops.insert(Field::Growth, (OpKind::Mul, "3".into()));
        assert_eq!(bulk_target(&state, &pet(), Field::Growth).as_deref(), Some("300"));
        state.overrides.insert((3, Field::Growth), "777".into());
        assert_eq!(effective_target(&state, &pet(), Field::Growth).as_deref(), Some("777"));
        assert_eq!((class_id(Class::Mage), class_label(8), class_label(0)), (8, "Mage", "Classless"));
    }

    #[test]
    fn equip_label_formats_quality_and_plus() {
        let item = EquipmentItem { type_id: 51, plus: 20, quality: 8, instance_id: 1, unique_id: 858, enchant_level: 0, gem_level: 0, gem_element: None };
        assert_eq!(equip_label(&item), "Magic Stick SSS+20");
        assert!(!equip_label(&EquipmentItem { plus: 0, ..item }).contains('+'));
    }

    #[test]
    fn select_apply_and_give_through_the_form() {
        let mut editor = loaded();
        open(&mut editor, SectionId::Pets);
        let rows = roster(editor.session.as_ref().unwrap()).unwrap();
        let target = rows.iter().find(|p| p.unlocked).unwrap();
        let (index, level, class) = (target.index, target.dungeon, target.class_id);
        fire(&mut editor, &format!("pets/select/{index}"), "1");
        fire(&mut editor, "pets/op/dungeon", "1");
        fire(&mut editor, "pets/op/dungeon/kind", "1");
        fire(&mut editor, "pets/op/dungeon/value", "3");
        fire(&mut editor, "pets/op/unlocked", "1");
        fire(&mut editor, "pets/op/unlocked/value", "1");
        let other = (class + 1) % 9;
        fire(&mut editor, &format!("pets/class/{index}"), &other.to_string());
        fire(&mut editor, &format!("pets/override/{index}/normal"), "not a number");
        fire(&mut editor, "pets/apply", "");
        let after = &roster(editor.session.as_ref().unwrap()).unwrap()[index];
        assert_eq!((after.dungeon, after.class_id, after.unlocked), (level + 3, other, false));
        assert!(editor.note.as_ref().unwrap().0.contains("1 skipped"), "{:?}", editor.note);
        assert!(editor.pets.selected.is_empty() && editor.pets.ops.is_empty());

        // Give a weapon to that pet: one new instance, equipped.
        let before = editor.session.as_ref().unwrap().derived().unwrap().equipment.len();
        fire(&mut editor, &format!("pets/select/{index}"), "1");
        fire(&mut editor, "pets/give", "");
        fire(&mut editor, "pets/builder/type", "1");
        fire(&mut editor, "pets/builder/confirm", "");
        let save = editor.session.as_ref().unwrap().derived().unwrap();
        assert_eq!(save.equipment.len(), before + 1);
        assert!(save.pets[index].weapon_id.is_some());
        assert!(editor.note.as_ref().unwrap().0.starts_with("Gave "));
        assert!(editor.pets.builder.is_none());

        fire(&mut editor, "pets/filter/name", "zzz-nothing");
        assert!(crate::editor::build(&editor).rows.is_empty());
        fire(&mut editor, "pets/filter/clear", "");
        assert_eq!(crate::editor::build(&editor).rows.len(), rows.len());
    }
}
