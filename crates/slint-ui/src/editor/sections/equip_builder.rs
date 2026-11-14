//! Shared equipment builder: an inline form that creates a new equipment
//! instance (type, quality, plus, gem). Two modes:
//! - [`Mode::Add`] — the Equipment page: a category filter and a quantity;
//!   creates N unequipped instances.
//! - [`Mode::Give`] — the Pets page: a slot picker (which also filters the type
//!   list); one instance per selected pet, equipped in that slot.
use save_parser::items::{self, EquipCategory};

use super::gems::{element_name, ELEMENTS};
use crate::editor::common::{pick, F};
use crate::editor::Editor;
use crate::form::{button, gap, input, muted, spacer, text, Mark, Tone};

/// (label, pet slot key, category) — `Builder::slot` indexes this.
const SLOTS: [(&str, &str, EquipCategory); 3] = [
    ("Weapon", "e", EquipCategory::Weapon),
    ("Armor", "f", EquipCategory::Armor),
    ("Accessory", "g", EquipCategory::Accessory),
];
pub const CATEGORIES: [EquipCategory; 3] = [EquipCategory::Weapon, EquipCategory::Armor, EquipCategory::Accessory];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Add standalone equipment to the inventory.
    Add,
    /// Give one new item to each of `count` selected pets.
    Give { count: usize },
}

pub struct Builder {
    type_id: Option<u32>,
    search: String,
    quality: u32,
    plus: String,
    gem_level: String,
    gem_element: u32,
    /// Add mode: the type list's category filter (`None` = any).
    category: Option<EquipCategory>,
    /// Give mode: index into [`SLOTS`].
    slot: usize,
    /// Add mode: how many to create.
    quantity: String,
}

impl Default for Builder {
    /// Sensible defaults: SSS, +0, no gem, quantity 1.
    fn default() -> Self {
        Builder { type_id: None, search: String::new(), quality: 8, plus: "0".into(), gem_level: "0".into(), gem_element: 0, category: None, slot: 0, quantity: "1".into() }
    }
}

/// What the builder produced when it was confirmed.
pub struct Built {
    pub type_id: u32,
    pub plus: u32,
    pub quality: u32,
    pub gem_level: u32,
    pub gem_element: u32,
    /// Add mode: how many to create. Give mode: 1.
    pub quantity: u32,
    /// Give mode: the pet slot key to equip into.
    pub slot_key: Option<&'static str>,
}

impl Built {
    /// "Magic Stick SSS+20" (the `+plus` is omitted at +0).
    pub fn label(&self) -> String {
        let plus = if self.plus > 0 { format!("+{}", self.plus) } else { String::new() };
        format!("{} {}{plus}", items::equipment_type_name(self.type_id).unwrap_or("Equipment"), items::quality_name(self.quality).unwrap_or(""))
    }
}

fn bounded(text: &str, low: u32, high: u32) -> Option<u32> {
    text.trim().parse::<u32>().ok().filter(|n| (low..=high).contains(n))
}

impl Builder {
    /// The category the type list is locked to: the chosen slot (give) or filter (add).
    fn lock(&self, give: bool) -> Option<EquipCategory> {
        if give { Some(SLOTS[self.slot.min(2)].2) } else { self.category }
    }

    /// The chosen type, unless it no longer matches the slot/category filter.
    fn chosen(&self, give: bool) -> Option<u32> {
        let lock = self.lock(give);
        self.type_id.filter(|id| lock.is_none_or(|l| items::equipment_category(*id) == Some(l)))
    }

    fn built(&self, give: bool) -> Option<Built> {
        Some(Built {
            type_id: self.chosen(give)?,
            plus: bounded(&self.plus, 0, 20)?,
            quality: self.quality,
            gem_level: bounded(&self.gem_level, 0, 20)?,
            gem_element: self.gem_element,
            quantity: if give { 1 } else { bounded(&self.quantity, 1, 99)? },
            slot_key: give.then(|| SLOTS[self.slot.min(2)].1),
        })
    }
}

/// The builder's rows. `id` prefixes its actions, `access` finds its state on
/// the editor, and `done` receives the result when it is confirmed.
pub fn rows(form: &mut F, builder: &Builder, mode: Mode, id: &'static str, access: fn(&mut Editor) -> &mut Option<Builder>, done: fn(&mut Editor, Built)) {
    let give = matches!(mode, Mode::Give { .. });
    let lock = builder.lock(give);
    let chosen = builder.chosen(give);
    let edit = move |apply: fn(&mut Builder, &str)| move |editor: &mut Editor, value: &str| {
        if let Some(builder) = access(editor).as_mut() { apply(builder, value); }
    };
    let field = |form: &F, name: &str, value: &str, apply: fn(&mut Builder, &str)| input(form.on(format!("{id}/{name}"), edit(apply)), value).live();

    // The type list, filtered by slot/category and the search box, sorted by
    // name so a long list is easy to scan.
    let needle = builder.search.trim().to_lowercase();
    let mut types: Vec<&(u32, &str, EquipCategory)> = items::EQUIPMENT_TYPES.iter()
        .filter(|(_, name, category)| lock.is_none_or(|l| *category == l) && (needle.is_empty() || name.to_lowercase().contains(&needle)))
        .collect();
    types.sort_by(|a, b| a.1.cmp(b.1));
    let mut choices: Vec<(Option<u32>, String)> = vec![(None, "— pick a type —".into())];
    choices.extend(types.into_iter().map(|(id, name, category)| (Some(*id), format!("{name}  ({})", category.name()))));
    let shown = chosen.filter(|id| choices.iter().any(|(choice, _)| *choice == Some(*id)));

    let mut cells = vec![text(match mode {
        Mode::Add => "Create equipment:".to_string(),
        Mode::Give { count } => format!("Give equipment to {count} pets:"),
    }).tone(Tone::Bright).w(190.0)];
    if give {
        let slots = SLOTS.iter().enumerate().map(|(index, (label, _, _))| (index, label.to_string())).collect();
        cells.push(muted("Slot:").w(34.0));
        cells.push(pick(form, format!("{id}/slot"), slots, &builder.slot.min(2), |_| String::new(), move |editor, slot| {
            if let Some(builder) = access(editor).as_mut() { builder.slot = *slot; }
        }).w(120.0));
    } else {
        let mut categories: Vec<(Option<EquipCategory>, String)> = vec![(None, "Any".into())];
        categories.extend(CATEGORIES.iter().map(|c| (Some(*c), c.name().to_string())));
        cells.push(muted("Category:").w(60.0));
        cells.push(pick(form, format!("{id}/category"), categories, &builder.category, |_| String::new(), move |editor, category| {
            if let Some(builder) = access(editor).as_mut() { builder.category = *category; }
        }).w(120.0));
    }
    cells.extend([
        muted("Type:").w(36.0),
        field(form, "search", &builder.search, |b, v| b.search = v.to_string()).hint("search").w(110.0),
        pick(form, format!("{id}/type"), choices, &shown, |_| "— pick a type —".to_string(), move |editor, choice| {
            if let Some(builder) = access(editor).as_mut() { builder.type_id = *choice; }
        }).w(280.0),
        spacer(),
    ]);
    form.row(cells).mark = Mark::Selected;

    let qualities = (0..=8u32).map(|q| (q, items::quality_name(q).unwrap_or("?").to_string())).collect();
    let mut cells = vec![
        gap(190.0),
        muted("Quality:").w(50.0),
        pick(form, format!("{id}/quality"), qualities, &builder.quality, |q| q.to_string(), move |editor, quality| {
            if let Some(builder) = access(editor).as_mut() { builder.quality = *quality; }
        }).w(80.0),
        muted("Plus (0–20):").w(76.0),
        field(form, "plus", &builder.plus, |b, v| b.plus = v.to_string()).w(54.0),
        muted("Gem level (0–20):").w(108.0),
        field(form, "gem-level", &builder.gem_level, |b, v| b.gem_level = v.to_string()).w(54.0),
    ];
    if bounded(&builder.gem_level, 1, 20).is_some() {
        let elements = ELEMENTS.iter().map(|(label, id)| (*id, label.to_string())).collect();
        cells.push(pick(form, format!("{id}/gem-element"), elements, &builder.gem_element, |e| element_name(*e), move |editor, element| {
            if let Some(builder) = access(editor).as_mut() { builder.gem_element = *element; }
        }).w(110.0));
    }
    if !give {
        cells.push(muted("Quantity (1–99):").w(100.0));
        cells.push(field(form, "quantity", &builder.quantity, |b, v| b.quantity = v.to_string()).w(54.0));
    }
    cells.extend([
        button(form.on(format!("{id}/confirm"), move |editor, _| {
            let Some(builder) = access(editor).take() else { return };
            if let Some(built) = builder.built(give) { done(editor, built); }
        }), if give { "Give" } else { "Create" }).primary().enabled(builder.built(give).is_some()).w(80.0),
        button(form.on(format!("{id}/cancel"), move |editor, _| *access(editor) = None), "Cancel").w(80.0),
        muted(if chosen.is_none() { "pick a type" } else { "" }).size(11.0),
    ]);
    form.row(cells).mark = Mark::Selected;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_validates_ranges_and_the_slot_filter() {
        let weapon = items::EQUIPMENT_TYPES.iter().find(|(_, _, c)| *c == EquipCategory::Weapon).unwrap().0;
        let mut builder = Builder { type_id: Some(weapon), ..Default::default() };
        let built = builder.built(false).unwrap();
        assert_eq!((built.quality, built.plus, built.quantity, built.slot_key), (8, 0, 1, None));
        assert!(!built.label().contains('+'));
        builder.plus = "21".into();
        assert!(builder.built(false).is_none(), "plus is capped at 20");
        builder.plus = "20".into();
        assert!(builder.built(false).unwrap().label().ends_with("SSS+20"));
        // Give mode equips into the chosen slot, and drops a type from another slot.
        assert_eq!(builder.built(true).unwrap().slot_key, Some("e"));
        builder.slot = 1;
        assert!(builder.built(true).is_none());
        builder.quantity = "0".into();
        assert!(builder.built(false).is_none());
    }
}
