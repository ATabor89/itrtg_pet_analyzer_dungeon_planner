//! Shared equipment builder: a modal window to create a new equipment instance
//! (type, quality, plus, gem). Used by the Equipment page ("Add equipment") and
//! (later) the Pets page ("give to a pet", slot-locked via `lock`).

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

#[derive(Default)]
pub struct EquipBuilderState {
    pub open: bool,
    type_id: Option<u32>,
    search: String,
    quality: u32,
    plus: u32,
    gem_level: u32,
    gem_element: u32,
}

impl EquipBuilderState {
    /// Open the builder, resetting to sensible defaults (SSS, +0, no gem).
    pub fn open(&mut self) {
        *self = EquipBuilderState {
            open: true,
            quality: 8,
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
}

/// Show the builder window while `st.open`. Returns `Some` on Create.
/// `lock` restricts the type list to one slot category (for the give-to-pet use).
pub fn builder_window(
    ctx: &egui::Context,
    st: &mut EquipBuilderState,
    lock: Option<EquipCategory>,
) -> Option<BuiltEquip> {
    if !st.open {
        return None;
    }
    let mut built = None;
    let mut close = false;
    let mut window_open = true;

    egui::Window::new("Create Equipment")
        .collapsible(false)
        .resizable(false)
        .open(&mut window_open)
        .show(ctx, |ui| {
            // Type picker (filtered by `lock`, searchable).
            ui.horizontal(|ui| {
                ui.label("Type:");
                let current = st.type_id.and_then(items::equipment_type_name).unwrap_or("— pick a type —");
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
                        for (id, name, cat) in items::EQUIPMENT_TYPES {
                            if lock.is_some_and(|l| *cat != l) {
                                continue;
                            }
                            if !q.is_empty() && !name.to_lowercase().contains(&q) {
                                continue;
                            }
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

            ui.separator();
            ui.horizontal(|ui| {
                let can_create = st.type_id.is_some();
                if ui
                    .add_enabled(can_create, egui::Button::new("Create"))
                    .clicked()
                {
                    built = st.type_id.map(|type_id| BuiltEquip {
                        type_id,
                        plus: st.plus,
                        quality: st.quality,
                        gem_level: st.gem_level,
                        gem_element: st.gem_element,
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
