//! Equipment: filter, multi-select and staged bulk edits over the owned
//! equipment list (`X.R`), plus create / delete / equip / unequip. Mirrors the
//! Pets section; edits write `X.R.<index>.<key>` into the pending list.
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use itrtg_models::Element;
use save_parser::items::{self, EquipCategory};
use save_parser::labels::EquipField;
use save_parser::session::EditSession;

use super::equip_builder::{self, Builder, Built, Mode, CATEGORIES};
use super::gems::{element_name, ELEMENTS};
use crate::editor::bulk::{self, parse_u64, OpKind};
use crate::editor::common::{in_range, matches, pick, tri, F};
use crate::editor::Editor;
use crate::form::{button, check, gap, input, muted, spacer, text, Cell, Mark, Tone};

/// The bulk-editable numeric fields, with their bulk-panel labels. Keys and
/// ranges come from the shared [`EquipField`] descriptors.
const FIELDS: [(EquipField, &str); 4] = [
    (EquipField::Quality, "Quality (F–SSS / 0–8)"),
    (EquipField::Plus, "Upgrade Level"),
    (EquipField::Enchant, "Enchant Level (0–20)"),
    (EquipField::GemLevel, "Gem Level"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sort { Type, Field(EquipField), Equipped }

fn elements() -> Vec<(u32, String)> { ELEMENTS.iter().map(|(label, id)| (*id, label.to_string())).collect() }

fn element_id(element: Element) -> u32 {
    match element { Element::Neutral | Element::All => 0, Element::Fire => 1, Element::Water => 2, Element::Earth => 3, Element::Wind => 4 }
}

/// (slot label, raw pet-slot key under `w`, category).
const SLOTS: [(&str, &str, EquipCategory); 3] = [
    ("weapon", "e", EquipCategory::Weapon), ("armor", "f", EquipCategory::Armor), ("accessory", "g", EquipCategory::Accessory),
];

fn slot_key(category: EquipCategory) -> &'static str { SLOTS.iter().find(|(_, _, c)| *c == category).map_or("e", |(_, key, _)| *key) }

/// A snapshot of one equipment instance.
struct Item {
    index: usize,
    name: String,
    category: Option<EquipCategory>,
    quality: u32,
    plus: u32,
    enchant: u32,
    gem_level: u32,
    gem_element: u32,
    /// Unique id (`h`) — the row's stable display id.
    unique_id: u32,
    /// Who wears it: (pet name, index into [`SLOTS`], pet index in `X.b`).
    worn: Option<(String, usize, usize)>,
}

impl Item {
    fn current(&self, field: EquipField) -> String {
        match field {
            EquipField::Quality => self.quality,
            EquipField::Plus => self.plus,
            EquipField::Enchant => self.enchant,
            _ => self.gem_level,
        }.to_string()
    }
}

#[derive(Default)]
pub struct Filters {
    name: String,
    category: Option<EquipCategory>,
    quality_min: String,
    quality_max: String,
    plus_min: String,
    plus_max: String,
    gem: Option<bool>,
    gem_element: Option<u32>,
    equipped: Option<bool>,
}

#[derive(Default)]
pub struct State {
    filters: Filters,
    selected: HashSet<usize>,
    ops: HashMap<EquipField, (OpKind, String)>,
    /// Bulk "set gem element" (element id); `None` leaves it alone.
    op_gem_element: Option<u32>,
    overrides: HashMap<(usize, EquipField), String>,
    gem_overrides: HashMap<usize, u32>,
    sort: Option<(Sort, bool)>,
    builder: Option<Builder>,
    /// "Equip to pet" in progress: (equipment index, chosen pet index).
    equip: Option<(usize, usize)>,
}

fn of(editor: &mut Editor) -> &mut State { &mut editor.equipment }

fn inventory(session: &EditSession) -> Option<Vec<Item>> {
    let save = session.derived()?;
    // Reverse map: a pet slot id (the item's **equip ref `d`**, 0 = unequipped)
    // → who has it. Pet slots reference `d`, NOT the unique id `h` — they diverge
    // for event gear, so this must be keyed and looked up by `d` or such items
    // wrongly read as unequipped.
    let mut wearers: HashMap<u32, (String, usize, usize)> = HashMap::new();
    for (pet_index, pet) in save.pets.iter().enumerate() {
        for (slot, id) in [pet.weapon_id, pet.armor_id, pet.accessory_id].into_iter().enumerate() {
            if let Some(id) = id { wearers.entry(id).or_insert((pet.name.clone(), slot, pet_index)); }
        }
    }
    Some(save.equipment.iter().enumerate().map(|(index, item)| {
        let i = index.to_string();
        let read = |key: &str, fallback: u32| session.value(&["X", "R", &i, key]).and_then(|s| s.trim().parse().ok()).unwrap_or(fallback);
        // Honor a pending edit to `d` if one is staged.
        let equip_ref = read("d", item.instance_id);
        let worn = (equip_ref != 0).then(|| wearers.get(&equip_ref).cloned()).flatten();
        Item {
            index,
            name: items::equipment_type_name(item.type_id).map_or_else(|| format!("Type {}", item.type_id), str::to_string),
            // Category from the type table, falling back to the equipped slot (so
            // an unknown type that's equipped is still categorized).
            category: items::equipment_category(item.type_id).or_else(|| worn.as_ref().map(|(_, slot, _)| SLOTS[*slot].2)),
            quality: item.quality, plus: item.plus, enchant: item.enchant_level, gem_level: item.gem_level,
            gem_element: item.gem_element.map(element_id).unwrap_or(0),
            unique_id: read("h", item.unique_id),
            worn,
        }
    }).collect())
}

/// A typed quality: a letter grade (F..SSS / Ult) or a number, clamped to the
/// valid 0–8 (SSS) range so an out-of-range number lands on SSS.
fn parse_quality(text: &str) -> Option<u32> {
    let text = text.trim();
    text.parse::<u32>().ok().or_else(|| items::quality_from_str(text)).map(|q| q.min(8))
}

/// Quality range filter; bounds accept letter grades or numbers, blank = open.
fn quality_in_range(value: u32, min: &str, max: &str) -> bool {
    let bound = |text: &str| { let t = text.trim(); t.parse::<u32>().ok().or_else(|| items::quality_from_str(t)) };
    bound(min).is_none_or(|low| value >= low) && bound(max).is_none_or(|high| value <= high)
}

fn passes(filters: &Filters, item: &Item) -> bool {
    matches(&filters.name.trim().to_lowercase(), &item.name)
        && filters.category.is_none_or(|c| item.category == Some(c))
        && quality_in_range(item.quality, &filters.quality_min, &filters.quality_max)
        && in_range(item.plus, &filters.plus_min, &filters.plus_max)
        && filters.gem.is_none_or(|gem| (item.gem_level > 0) == gem)
        && filters.gem_element.is_none_or(|e| item.gem_level > 0 && item.gem_element == e)
        && filters.equipped.is_none_or(|e| item.worn.is_some() == e)
}

fn compare(a: &Item, b: &Item, sort: Sort) -> Ordering {
    match sort {
        Sort::Type => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        Sort::Field(EquipField::Quality) => a.quality.cmp(&b.quality),
        Sort::Field(EquipField::Plus) => a.plus.cmp(&b.plus),
        Sort::Field(EquipField::Enchant) => a.enchant.cmp(&b.enchant),
        Sort::Field(_) => a.gem_level.cmp(&b.gem_level),
        Sort::Equipped => a.worn.is_some().cmp(&b.worn.is_some()),
    }
}

/// The bulk-op result for a numeric field, or `None` if no op is configured.
fn bulk_target(state: &State, item: &Item, field: EquipField) -> Option<String> {
    let (kind, value) = state.ops.get(&field)?;
    let current = item.current(field).parse::<u64>().ok()?;
    let new = match kind {
        // Quality "Set" accepts a letter grade or a number.
        OpKind::Set if field == EquipField::Quality => u64::from(parse_quality(value)?),
        OpKind::Set => parse_u64(value)?,
        OpKind::Add => current.checked_add(parse_u64(value)?)?,
        OpKind::Mul => return None,
    };
    // Bounds come from the shared field descriptor (quality 0–8, plus 0–30,
    // enchant 0–20), so the section, the model and the tree all agree.
    Some(match field.range() { Some((low, high)) => new.clamp(low as u64, high as u64), None => new }.to_string())
}

fn effective_target(state: &State, item: &Item, field: EquipField) -> Option<String> {
    state.overrides.get(&(item.index, field)).cloned().or_else(|| bulk_target(state, item, field))
}

/// Stage every configured change for the selected items.
fn apply(editor: &mut Editor) {
    let Some(session) = editor.session.as_mut() else { return };
    let Some(rows) = inventory(session) else { return };
    let state = &editor.equipment;
    let (mut staged, mut skipped) = (0, 0);
    for item in rows.iter().filter(|i| state.selected.contains(&i.index)) {
        let i = item.index.to_string();
        for (field, label) in FIELDS {
            let Some(target) = effective_target(state, item, field) else { continue };
            // Quality may arrive as a letter grade (typed into an override cell);
            // normalize to a clamped number before validating / staging.
            let target = if field == EquipField::Quality { parse_quality(&target).map(|q| q.to_string()) } else { Some(target) };
            let Some(target) = target.filter(|t| t.trim().parse::<u64>().is_ok()) else { skipped += 1; continue };
            if target.trim() == item.current(field) { continue; }
            if session.set_scalar(&["X", "R", &i, field.key()], format!("{} · {label}", item.name), target.trim()).is_ok() { staged += 1; } else { skipped += 1; }
        }
        if let Some(element) = state.gem_overrides.get(&item.index).copied().or(state.op_gem_element) && element != item.gem_element
            && session.set_scalar(&["X", "R", &i, EquipField::GemElement.key()], format!("{} · Gem Element", item.name), &element.to_string()).is_ok() {
            staged += 1;
        }
    }
    editor.note = Some(if skipped > 0 {
        (format!("Staged {staged} edits ({skipped} skipped — invalid or unreachable)"), staged == 0)
    } else {
        (format!("Staged {staged} edits across selected items"), false)
    });
    let state = &mut editor.equipment;
    state.selected.clear();
    state.overrides.clear();
    state.gem_overrides.clear();
    state.ops.clear();
    state.op_gem_element = None;
}

/// Create the built item `quantity` times, unequipped.
fn create(editor: &mut Editor, built: Built) {
    let Some(session) = editor.session.as_mut() else { return };
    let label = built.label();
    let mut created = 0;
    let mut failure = None;
    for _ in 0..built.quantity {
        match session.add_equipment(built.type_id, built.plus, built.quality, built.gem_level, built.gem_element, label.clone(), None) {
            Ok(_) => created += 1,
            Err(error) => { failure = Some(error.to_string()); break; }
        }
    }
    editor.note = Some(match failure {
        Some(error) => (format!("Created {created}, then failed: {error}"), true),
        None => (format!("Created {created} × {label} — see Pending changes"), false),
    });
}

/// One bulk op: its checkbox plus, when enabled, the op picker and value.
fn op_cells(form: &F, state: &State, field: EquipField, label: &str) -> Vec<Cell> {
    let key = field.key();
    let mut cells = vec![check(form.on(format!("equipment/op/{key}"), move |editor, value| {
        if value == "1" { editor.equipment.ops.insert(field, (OpKind::Set, String::new())); } else { editor.equipment.ops.remove(&field); }
    }), label, state.ops.contains_key(&field)).w(170.0)];
    match state.ops.get(&field) {
        Some((kind, value)) => {
            cells.push(bulk::op_picker(form, format!("equipment/op/{key}/kind"), &[OpKind::Set, OpKind::Add], *kind, move |editor, kind| {
                if let Some(op) = editor.equipment.ops.get_mut(&field) { op.0 = kind; }
            }));
            cells.push(input(form.on(format!("equipment/op/{key}/value"), move |editor, value| {
                if let Some(op) = editor.equipment.ops.get_mut(&field) { op.1 = value.to_string(); }
            }), value.as_str()).w(110.0));
        }
        None => cells.push(gap(204.0)),
    }
    cells.push(gap(16.0));
    cells
}

pub fn build(form: &mut F, session: &EditSession, state: &State) {
    form.title_with("Equipment", vec![
        button(form.on("equipment/add", |editor, _| editor.equipment.builder = Some(Builder::default())), "Add equipment").w(130.0),
        spacer(),
    ]);
    let Some(rows) = inventory(session) else {
        form.note("Typed equipment data unavailable — use the Raw Save Tree.");
        return;
    };
    let mut shown: Vec<&Item> = rows.iter().filter(|i| passes(&state.filters, i)).collect();
    if let Some((sort, ascending)) = state.sort {
        shown.sort_by(|a, b| { let order = compare(a, b, sort); if ascending { order } else { order.reverse() } });
    }

    let filters = &state.filters;
    let label = |text: &str, width: f32| muted(text).size(11.0).w(width);
    let typed = |form: &F, id: &str, value: &str, set: fn(&mut Filters, String)| input(form.on(format!("equipment/filter/{id}"), move |editor, v| set(&mut editor.equipment.filters, v.to_string())), value).live();
    let mut categories: Vec<(Option<EquipCategory>, String)> = vec![(None, "Any".into())];
    categories.extend(CATEGORIES.iter().map(|c| (Some(*c), c.name().to_string())));
    let mut gem_elements: Vec<(Option<u32>, String)> = vec![(None, "Any".into())];
    gem_elements.extend(elements().into_iter().map(|(id, label)| (Some(id), label)));
    form.row(vec![
        muted("Filter:").w(44.0),
        label("Slot", 24.0),
        pick(form, "equipment/filter/category", categories, &filters.category, |_| String::new(), |editor, c| editor.equipment.filters.category = *c).w(110.0),
        label("Name", 32.0), typed(form, "name", &filters.name, |f, v| f.name = v).w(130.0),
        label("Quality", 42.0), typed(form, "quality-min", &filters.quality_min, |f, v| f.quality_min = v).w(50.0),
        muted("–").w(10.0), typed(form, "quality-max", &filters.quality_max, |f, v| f.quality_max = v).w(50.0),
        label("Upgrade", 46.0), typed(form, "plus-min", &filters.plus_min, |f, v| f.plus_min = v).w(46.0),
        muted("–").w(10.0), typed(form, "plus-max", &filters.plus_max, |f, v| f.plus_max = v).w(46.0),
        spacer(),
    ]);
    form.row(vec![
        gap(44.0),
        label("Gem", 24.0),
        tri(form, "equipment/filter/gem", ["Any", "Gemmed", "No gem"], filters.gem, |editor, v| editor.equipment.filters.gem = v).w(110.0),
        label("Gem element", 70.0),
        pick(form, "equipment/filter/gem-element", gem_elements, &filters.gem_element, |_| String::new(), |editor, e| editor.equipment.filters.gem_element = *e).w(110.0),
        label("Equipped", 52.0),
        tri(form, "equipment/filter/equipped", ["Any", "Equipped", "Unequipped"], filters.equipped, |editor, v| editor.equipment.filters.equipped = v).w(120.0),
        button(form.on("equipment/filter/clear", |editor, _| editor.equipment.filters = Filters::default()), "× clear").w(80.0),
        muted(format!("{} / {} items", shown.len(), rows.len())).size(11.0),
    ]);

    // Bulk edit.
    let visible: Vec<usize> = shown.iter().map(|i| i.index).collect();
    let count = state.selected.len();
    form.row(vec![
        text(format!("{count} selected")).tone(Tone::Bright).w(90.0),
        button(form.on("equipment/select-all", move |editor, _| editor.equipment.selected.extend(visible.iter().copied())), "Select all (filtered)").w(170.0),
        button(form.on("equipment/select-none", |editor, _| editor.equipment.selected.clear()), "Clear selection").w(130.0),
        muted("Quality 0–8 = F E D C B A S SS SSS. A typed cell value wins over the bulk op.").size(11.0),
    ]);
    for pair in FIELDS.chunks(2) {
        let mut cells = Vec::new();
        for (field, label) in pair { cells.extend(op_cells(form, state, *field, label)); }
        cells.push(spacer());
        form.row(cells);
    }
    let mut cells = vec![check(form.on("equipment/op/gem-element", |editor, value| editor.equipment.op_gem_element = (value == "1").then_some(1)), "Gem Element", state.op_gem_element.is_some()).w(170.0)];
    match state.op_gem_element {
        Some(element) => {
            cells.push(pick(form, "equipment/op/gem-element/value", elements(), &element, |e| element_name(*e), |editor, e| editor.equipment.op_gem_element = Some(*e)).w(120.0));
            cells.push(gap(76.0));
        }
        None => cells.push(gap(204.0)),
    }
    let has_ops = !state.ops.is_empty() || state.op_gem_element.is_some() || !state.overrides.is_empty() || !state.gem_overrides.is_empty();
    cells.extend([
        gap(16.0),
        button(form.on("equipment/apply", |editor, _| apply(editor)), format!("Apply to {count} items")).primary().enabled(count > 0 && has_ops).w(150.0),
        spacer(),
    ]);
    form.row(cells);
    if let Some(builder) = &state.builder {
        equip_builder::rows(form, builder, Mode::Add, "equipment/builder", |editor| &mut editor.equipment.builder, create);
    }
    if let Some((index, pet)) = state.equip && let Some(item) = rows.iter().find(|i| i.index == index) && let Some(category) = item.category {
        equip_form(form, session, item, category, pet);
    }

    form.header(vec![
        spacer().w(30.0),
        bulk::sort_header(form, "equipment/sort/type", "Name", Sort::Type, state.sort, |e| &mut of(e).sort).w(190.0),
        bulk::sort_header(form, "equipment/sort/quality", "Quality", Sort::Field(EquipField::Quality), state.sort, |e| &mut of(e).sort).w(100.0),
        bulk::sort_header(form, "equipment/sort/plus", "Upgrade Level", Sort::Field(EquipField::Plus), state.sort, |e| &mut of(e).sort).w(110.0),
        bulk::sort_header(form, "equipment/sort/enchant", "Enchant", Sort::Field(EquipField::Enchant), state.sort, |e| &mut of(e).sort).w(90.0),
        bulk::sort_header(form, "equipment/sort/gem", "Gem", Sort::Field(EquipField::GemLevel), state.sort, |e| &mut of(e).sort).w(198.0),
        bulk::sort_header(form, "equipment/sort/equipped", "Equipped", Sort::Equipped, state.sort, |e| &mut of(e).sort).w(258.0),
        text("Inst").w(54.0),
        spacer(),
    ]);
    form.pin();

    for item in shown {
        let selected = state.selected.contains(&item.index);
        let index = item.index;
        let mut cells = vec![
            bulk::select_box(form, format!("equipment/select/{index}"), index, selected, |e| &mut of(e).selected),
            text(item.name.as_str()).w(190.0),
        ];
        for (field, width) in [(EquipField::Quality, 100.0), (EquipField::Plus, 110.0), (EquipField::Enchant, 90.0), (EquipField::GemLevel, 90.0)] {
            let current = item.current(field);
            if selected {
                let default = bulk_target(state, item, field).unwrap_or_else(|| current.clone());
                cells.extend(bulk::override_cells(form, format!("equipment/override/{index}/{}", field.key()), (index, field), &default, &current,
                    &state.overrides, |e| &mut of(e).overrides, width));
            } else {
                // Quality reads as its letter grade.
                let shown = if field == EquipField::Quality { items::quality_name(item.quality).map_or(current, str::to_string) } else { current };
                cells.push(text(shown).mono().size(11.0).w(width));
            }
        }
        if selected {
            // The element picker defaults to the bulk op; anything else is a per-item override.
            let default = state.op_gem_element.unwrap_or(item.gem_element);
            let current = state.gem_overrides.get(&index).copied().unwrap_or(default);
            cells.push(pick(form, format!("equipment/gem-element/{index}"), elements(), &current, |e| element_name(*e), move |editor, element| {
                if *element == default { editor.equipment.gem_overrides.remove(&index); } else { editor.equipment.gem_overrides.insert(index, *element); }
            }).w(100.0));
        } else {
            cells.push(muted(if item.gem_level > 0 { element_name(item.gem_element) } else { String::new() }).size(11.0).w(100.0));
        }
        match &item.worn {
            Some((pet, slot, pet_index)) => {
                let (slot_label, key, _) = SLOTS[*slot];
                let (pet_index, label) = (*pet_index, format!("Unequip {} from {pet}", item.name));
                cells.push(text(format!("{pet} ({slot_label})")).w(170.0));
                // Clear the pet's slot and return the item to the inventory
                // (`d` = 0 ⟺ in inventory) — the inverse of equipping.
                cells.push(button(form.on(format!("equipment/unequip/{index}"), move |editor, _| {
                    let Some(session) = editor.session.as_mut() else { return };
                    let cleared = session.set_scalar(&["X", "b", &pet_index.to_string(), "w", key], label.clone(), "0");
                    let returned = session.set_scalar(&["X", "R", &index.to_string(), "d"], label.clone(), "0");
                    if let Err(error) = cleared.and(returned) { editor.note = Some((format!("Unequip failed: {error}"), true)); }
                }), "unequip").w(80.0).hint("Clear this pet's slot and return the item to inventory"));
            }
            None => {
                cells.push(muted("—").w(170.0));
                // Only categorized items map to a pet slot.
                cells.push(if item.category.is_some() {
                    button(form.on(format!("equipment/equip/{index}"), move |editor, _| editor.equipment.equip = Some((index, 0))), "equip").w(80.0).hint("Equip this item onto a pet")
                } else {
                    gap(80.0)
                });
            }
        }
        cells.push(muted(item.unique_id.to_string()).size(11.0).w(54.0));
        let label = item.name.clone();
        cells.push(button(form.on(format!("equipment/delete/{index}"), move |editor, _| {
            let Some(session) = editor.session.as_mut() else { return };
            if let Err(error) = session.delete_equipment(index, label.clone()) { editor.note = Some((format!("Delete failed: {error}"), true)); }
            // Row indices shift after a delete, so index-keyed picks no longer apply.
            let state = &mut editor.equipment;
            state.selected.clear();
            state.overrides.clear();
            state.gem_overrides.clear();
            state.equip = None;
        }), "×").w(40.0).hint("Delete this item"));
        cells.push(spacer());
        form.row(cells).mark = if selected { Mark::Selected } else { Mark::None };
    }
}

/// The "equip to pet" form. The slot is fixed by the item's category.
fn equip_form(form: &mut F, session: &EditSession, item: &Item, category: EquipCategory, pet: usize) {
    let pets: Vec<(usize, String)> = session.derived().map(|save| save.pets.iter().enumerate().map(|(index, pet)| (index, pet.name.clone())).collect()).unwrap_or_default();
    let (index, name, slot) = (item.index, item.name.clone(), slot_key(category));
    let names = pets.clone();
    form.row(vec![
        text(format!("Equip {}:", item.name)).tone(Tone::Bright).w(220.0),
        muted(format!("Slot: {}", category.name())).size(11.0).w(100.0),
        muted("Pet:").w(30.0),
        pick(form, "equipment/equip/pet", pets, &pet, |_| "—".to_string(), |editor, pet| {
            if let Some(equip) = editor.equipment.equip.as_mut() { equip.1 = *pet; }
        }).w(220.0),
        // Mints a fresh equip ref and is swap-aware (see `EditSession::equip_existing`).
        button(form.on("equipment/equip/confirm", move |editor, _| {
            let (Some(session), Some((_, pet))) = (editor.session.as_mut(), editor.equipment.equip.take()) else { return };
            let pet_name = names.iter().find(|(i, _)| *i == pet).map_or("pet", |(_, n)| n.as_str());
            editor.note = Some(match session.equip_existing(index, pet, slot, format!("Equip {name} on {pet_name}")) {
                Ok(()) => (format!("Equipped {name} on {pet_name}"), false),
                Err(error) => (format!("Equip failed: {error}"), true),
            });
        }), "Equip").primary().w(80.0),
        button(form.on("equipment/equip/cancel", |editor, _| editor.equipment.equip = None), "Cancel").w(80.0),
        muted("If the pet's slot is occupied, that item returns to inventory.").size(10.0),
    ]).mark = Mark::Selected;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::tests::{fire, loaded, open};
    use save_parser::registry::SectionId;

    fn item() -> Item {
        Item { index: 2, name: "Magic Stick".into(), category: Some(EquipCategory::Weapon), quality: 6, plus: 10, enchant: 0, gem_level: 0, gem_element: 0, unique_id: 858, worn: None }
    }

    fn target(state: &mut State, field: EquipField, kind: OpKind, value: &str) -> Option<String> {
        state.ops.insert(field, (kind, value.into()));
        bulk_target(state, &item(), field)
    }

    #[test]
    fn bulk_targets_clamp_to_the_field_ranges() {
        let mut state = State::default();
        assert_eq!(target(&mut state, EquipField::Plus, OpKind::Add, "5").as_deref(), Some("15"));
        // Plus is bounded 0–30 (Candy Cane's cap): +30 survives, absurd pins to 30.
        assert_eq!(target(&mut state, EquipField::Plus, OpKind::Set, "30").as_deref(), Some("30"));
        assert_eq!(target(&mut state, EquipField::Plus, OpKind::Set, "99").as_deref(), Some("30"));
        assert_eq!(target(&mut state, EquipField::Quality, OpKind::Add, "5").as_deref(), Some("8"));
        assert_eq!(target(&mut state, EquipField::Enchant, OpKind::Set, "50").as_deref(), Some("20"));
        state.overrides.insert((2, EquipField::Plus), "13".into());
        assert_eq!(effective_target(&state, &item(), EquipField::Plus).as_deref(), Some("13"));
    }

    #[test]
    fn quality_accepts_letters_and_numbers() {
        let mut state = State::default();
        assert_eq!(target(&mut state, EquipField::Quality, OpKind::Set, "SSS").as_deref(), Some("8"));
        assert_eq!(target(&mut state, EquipField::Quality, OpKind::Set, "a").as_deref(), Some("5"));
        assert_eq!(target(&mut state, EquipField::Quality, OpKind::Set, "12").as_deref(), Some("8"));
        assert_eq!(target(&mut state, EquipField::Quality, OpKind::Set, "Z"), None);
        // The row's quality is 6 (S).
        assert!(quality_in_range(6, "A", "SSS") && !quality_in_range(6, "SS", "SSS"));
        assert!(quality_in_range(6, "", "") && quality_in_range(6, "0", "8"));
    }

    #[test]
    fn create_edit_equip_unequip_and_delete_through_the_form() {
        let mut editor = loaded();
        open(&mut editor, SectionId::Equipment);
        let before = inventory(editor.session.as_ref().unwrap()).unwrap().len();
        fire(&mut editor, "equipment/add", "");
        fire(&mut editor, "equipment/builder/category", "1");
        fire(&mut editor, "equipment/builder/type", "1");
        fire(&mut editor, "equipment/builder/quantity", "2");
        fire(&mut editor, "equipment/builder/confirm", "");
        let rows = inventory(editor.session.as_ref().unwrap()).unwrap();
        assert_eq!(rows.len(), before + 2);
        assert!(editor.note.as_ref().unwrap().0.starts_with("Created 2 × "));
        let new = rows.last().unwrap();
        assert_eq!((new.category, new.quality, new.worn.is_none()), (Some(EquipCategory::Weapon), 8, true));
        let index = new.index;

        // Bulk-edit it: quality by letter, plus through an override.
        fire(&mut editor, &format!("equipment/select/{index}"), "1");
        fire(&mut editor, "equipment/op/c", "1");
        fire(&mut editor, "equipment/op/c/value", "A");
        fire(&mut editor, &format!("equipment/override/{index}/b"), "7");
        fire(&mut editor, "equipment/apply", "");
        let edited = &inventory(editor.session.as_ref().unwrap()).unwrap()[index];
        assert_eq!((edited.quality, edited.plus), (5, 7));

        // Equip it on the first pet, then unequip it again.
        fire(&mut editor, &format!("equipment/equip/{index}"), "");
        fire(&mut editor, "equipment/equip/confirm", "");
        let worn = inventory(editor.session.as_ref().unwrap()).unwrap()[index].worn.clone().unwrap();
        assert_eq!((worn.1, worn.2), (0, 0));
        fire(&mut editor, &format!("equipment/unequip/{index}"), "");
        assert!(inventory(editor.session.as_ref().unwrap()).unwrap()[index].worn.is_none());

        fire(&mut editor, &format!("equipment/delete/{index}"), "");
        assert_eq!(inventory(editor.session.as_ref().unwrap()).unwrap().len(), before + 1);
    }
}
