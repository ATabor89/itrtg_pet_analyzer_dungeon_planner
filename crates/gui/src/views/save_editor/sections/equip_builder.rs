//! Shared equipment builder: a modal window to create a new equipment instance
//! (type, quality, plus, gem). Two modes:
//! - [`BuilderMode::Add`] — the Equipment page: a category filter + a quantity,
//!   creates N unequipped instances.
//! - [`BuilderMode::GiveToPets`] — the Pets page: a slot picker (which also
//!   filters the type list), one instance per selected pet equipped in that slot.

use eframe::egui::{self, RichText};
use save_parser::items::{self, EquipCategory};

use crate::style;

const ELEMENT_CHOICES: &[(&str, u32)] = &[
    ("Neutral", 0),
    ("Fire", 1),
    ("Water", 2),
    ("Earth", 3),
    ("Wind", 4),
];

fn element_name(id: u32) -> &'static str {
    ELEMENT_CHOICES.iter().find(|(_, i)| *i == id).map_or("?", |(l, _)| *l)
}

/// (label, pet slot key, category) — index into this is `st.slot`.
const SLOTS: &[(&str, &str, EquipCategory)] = &[
    ("Weapon", "e", EquipCategory::Weapon),
    ("Armor", "f", EquipCategory::Armor),
    ("Accessory", "g", EquipCategory::Accessory),
];

const CATEGORIES: &[EquipCategory] =
    &[EquipCategory::Weapon, EquipCategory::Armor, EquipCategory::Accessory];

/// What the builder is being used for.
pub enum BuilderMode {
    /// Add standalone equipment to inventory.
    Add,
    /// Give one new item to each of `count` selected pets.
    GiveToPets { count: usize },
}

#[derive(Default)]
pub struct EquipBuilderState {
    pub open: bool,
    type_id: Option<u32>,
    search: String,
    quality: u32,
    plus: u32,
    gem_level: u32,
    gem_element: u32,
    /// Add-mode type-list category filter (`None` = any).
    category: Option<EquipCategory>,
    /// Give-mode slot index into [`SLOTS`].
    slot: usize,
    /// Add-mode quantity.
    quantity: u32,
}

impl EquipBuilderState {
    /// Open the builder, resetting to sensible defaults (SSS, +0, no gem, qty 1).
    pub fn open(&mut self) {
        *self = EquipBuilderState {
            open: true,
            quality: 8,
            quantity: 1,
            ..EquipBuilderState::default()
        };
    }
}

/// What the builder produced when "Create" was clicked.
pub struct BuiltEquip {
    pub type_id: u32,
    pub plus: u32,
    pub quality: u32,
    pub gem_level: u32,
    pub gem_element: u32,
    /// Add mode: how many to create. Give mode: 1.
    pub quantity: u32,
    /// Give mode: the pet slot key to equip into. Add mode: `None`.
    pub slot_key: Option<&'static str>,
}

/// Show the builder window while `st.open`. Returns `Some` on Create.
pub fn builder_window(
    ctx: &egui::Context,
    st: &mut EquipBuilderState,
    mode: BuilderMode,
) -> Option<BuiltEquip> {
    if !st.open {
        return None;
    }
    let give = matches!(mode, BuilderMode::GiveToPets { .. });
    let title = match mode {
        BuilderMode::Add => "Create Equipment".to_string(),
        BuilderMode::GiveToPets { count } => format!("Give Equipment to {count} pets"),
    };
    // The type list is filtered by the chosen category (Add) or slot (Give).
    let lock = if give {
        Some(SLOTS[st.slot].2)
    } else {
        st.category
    };

    let mut built = None;
    let mut close = false;
    let mut window_open = true;

    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .open(&mut window_open)
        .show(ctx, |ui| {
            // Slot (give) or category (add) filter.
            ui.horizontal(|ui| {
                if give {
                    ui.label("Slot:");
                    egui::ComboBox::from_id_salt("eqbuild_slot")
                        .selected_text(SLOTS[st.slot].0)
                        .show_ui(ui, |ui| {
                            for (i, (label, _, _)) in SLOTS.iter().enumerate() {
                                ui.selectable_value(&mut st.slot, i, *label);
                            }
                        });
                } else {
                    ui.label("Category:");
                    egui::ComboBox::from_id_salt("eqbuild_cat")
                        .selected_text(st.category.map_or("Any", EquipCategory::name))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut st.category, None, "Any");
                            for &c in CATEGORIES {
                                ui.selectable_value(&mut st.category, Some(c), c.name());
                            }
                        });
                }
            });

            // Type picker (filtered by `lock`, searchable). Clear the selection if
            // it no longer matches the filter.
            if let Some(l) = lock
                && st.type_id.and_then(items::equipment_category) != Some(l)
            {
                st.type_id = None;
            }
            ui.horizontal(|ui| {
                ui.label("Type:");
                let current =
                    st.type_id.and_then(items::equipment_type_name).unwrap_or("— pick a type —");
                egui::ComboBox::from_id_salt("eqbuild_type")
                    .selected_text(current)
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut st.search)
                                .hint_text("search")
                                .desired_width(180.0),
                        );
                        let q = st.search.trim().to_lowercase();
                        // Sorted by name so a long type list is easy to scan.
                        let mut opts: Vec<&(u32, &str, EquipCategory)> = items::EQUIPMENT_TYPES
                            .iter()
                            .filter(|(_, name, cat)| {
                                !lock.is_some_and(|l| *cat != l)
                                    && (q.is_empty() || name.to_lowercase().contains(&q))
                            })
                            .collect();
                        opts.sort_by(|a, b| a.1.cmp(b.1));
                        for (id, name, cat) in opts {
                            ui.selectable_value(
                                &mut st.type_id,
                                Some(*id),
                                format!("{name}  ({})", cat.name()),
                            );
                        }
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Quality:");
                egui::ComboBox::from_id_salt("eqbuild_quality")
                    .selected_text(items::quality_name(st.quality).unwrap_or("?"))
                    .show_ui(ui, |ui| {
                        for q in 0..=8 {
                            ui.selectable_value(&mut st.quality, q, items::quality_name(q).unwrap());
                        }
                    });
                ui.label("Plus:");
                ui.add(egui::DragValue::new(&mut st.plus).range(0..=20));
            });

            ui.horizontal(|ui| {
                ui.label("Gem level:");
                ui.add(egui::DragValue::new(&mut st.gem_level).range(0..=20));
                if st.gem_level > 0 {
                    ui.label("element:");
                    egui::ComboBox::from_id_salt("eqbuild_gem")
                        .selected_text(element_name(st.gem_element))
                        .show_ui(ui, |ui| {
                            for &(label, id) in ELEMENT_CHOICES {
                                ui.selectable_value(&mut st.gem_element, id, label);
                            }
                        });
                }
            });

            // Quantity (Add mode only — Give mode is one per selected pet).
            if !give {
                ui.horizontal(|ui| {
                    ui.label("Quantity:");
                    ui.add(egui::DragValue::new(&mut st.quantity).range(1..=99));
                });
            }

            ui.separator();
            ui.horizontal(|ui| {
                let can_create = st.type_id.is_some();
                let action = if give { "Give" } else { "Create" };
                if ui.add_enabled(can_create, egui::Button::new(action)).clicked() {
                    built = st.type_id.map(|type_id| BuiltEquip {
                        type_id,
                        plus: st.plus,
                        quality: st.quality,
                        gem_level: st.gem_level,
                        gem_element: st.gem_element,
                        quantity: if give { 1 } else { st.quantity.max(1) },
                        slot_key: give.then(|| SLOTS[st.slot].1),
                    });
                    close = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
                if !can_create {
                    ui.label(RichText::new("pick a type").color(style::TEXT_MUTED).size(11.0));
                }
            });
        });

    // The window stays open unless the X was clicked, Create, or Cancel.
    st.open = window_open && !close;
    built
}
