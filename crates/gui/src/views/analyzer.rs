use eframe::egui::{self, RichText, Ui};
use egui_extras::{Column, TableBuilder};
use itrtg_models::{Class, Element, RecommendedClass};
use itrtg_planner::merge::MergedPet;

use crate::data::DataStore;
use crate::style;
use super::widgets;

// =============================================================================
// Filter state
// =============================================================================

#[derive(Default)]
pub struct AnalyzerState {
    pub search: String,
    pub filter_element: Option<Element>,
    pub filter_unlocked: Option<bool>,
    pub filter_evolved: Option<bool>,
    pub filter_improvable: Option<bool>,
    pub sort_column: SortColumn,
    pub sort_ascending: bool,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    #[default]
    Name,
    Element,
    RecClass,
    EvoDifficulty,
    Growth,
    DungeonLevel,
    Class,
}

// =============================================================================
// Rendering
// =============================================================================

pub fn show(ui: &mut Ui, state: &mut AnalyzerState, data: &DataStore) {
    // Stats bar
    show_stats_bar(ui, data);

    ui.add_space(4.0);

    // Filter bar
    show_filters(ui, state);

    ui.add_space(4.0);
    ui.separator();

    // Pet table
    let filtered = filter_and_sort(&data.merged, state);
    show_table(ui, &filtered, state);
}

fn show_stats_bar(ui: &mut Ui, data: &DataStore) {
    ui.horizontal(|ui| {
        let total = data.merged.len();
        let unlocked = data.merged.iter().filter(|p| p.is_unlocked()).count();
        let evolved = data.merged.iter().filter(|p| p.is_evolved()).count();

        ui.label(
            RichText::new(format!("Total: {total}"))
                .color(style::TEXT_BRIGHT)
                .size(13.0),
        );
        ui.separator();
        ui.label(
            RichText::new(format!("Unlocked: {unlocked}"))
                .color(style::SUCCESS)
                .size(13.0),
        );
        ui.separator();
        ui.label(
            RichText::new(format!("Evolved: {evolved}"))
                .color(style::ACCENT)
                .size(13.0),
        );

        if data.export_pets.is_empty() {
            ui.separator();
            ui.label(
                RichText::new("No export loaded")
                    .color(style::WARNING)
                    .size(13.0),
            );
        }
    });
}

fn show_filters(ui: &mut Ui, state: &mut AnalyzerState) {
    ui.horizontal(|ui| {
        // Search
        ui.label(RichText::new("Search:").color(style::TEXT_MUTED));
        ui.add(
            egui::TextEdit::singleline(&mut state.search)
                .desired_width(150.0)
                .hint_text("Pet name..."),
        );

        ui.separator();

        // Element filter
        ui.label(RichText::new("Element:").color(style::TEXT_MUTED));
        egui::ComboBox::from_id_salt("elem_filter")
            .selected_text(match &state.filter_element {
                None => "All",
                Some(Element::Fire) => "Fire",
                Some(Element::Water) => "Water",
                Some(Element::Wind) => "Wind",
                Some(Element::Earth) => "Earth",
                Some(Element::Neutral) => "Neutral",
                Some(Element::All) => "All (Chameleon)",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.filter_element, None, "All");
                ui.selectable_value(&mut state.filter_element, Some(Element::Fire), "Fire");
                ui.selectable_value(&mut state.filter_element, Some(Element::Water), "Water");
                ui.selectable_value(&mut state.filter_element, Some(Element::Wind), "Wind");
                ui.selectable_value(&mut state.filter_element, Some(Element::Earth), "Earth");
                ui.selectable_value(&mut state.filter_element, Some(Element::Neutral), "Neutral");
            });

        ui.separator();

        // Unlocked filter
        ui.label(RichText::new("Unlocked:").color(style::TEXT_MUTED));
        egui::ComboBox::from_id_salt("unlock_filter")
            .selected_text(match state.filter_unlocked {
                None => "All",
                Some(true) => "Yes",
                Some(false) => "No",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.filter_unlocked, None, "All");
                ui.selectable_value(&mut state.filter_unlocked, Some(true), "Yes");
                ui.selectable_value(&mut state.filter_unlocked, Some(false), "No");
            });

        // Evolved filter
        ui.label(RichText::new("Evolved:").color(style::TEXT_MUTED));
        egui::ComboBox::from_id_salt("evolved_filter")
            .selected_text(match state.filter_evolved {
                None => "All",
                Some(true) => "Yes",
                Some(false) => "No",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.filter_evolved, None, "All");
                ui.selectable_value(&mut state.filter_evolved, Some(true), "Yes");
                ui.selectable_value(&mut state.filter_evolved, Some(false), "No");
            });
    });
}

fn show_table(ui: &mut Ui, pets: &[&MergedPet], state: &mut AnalyzerState) {
    let available = ui.available_size();

    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .min_scrolled_height(available.y - 20.0)
        .max_scroll_height(available.y - 20.0)
        .column(Column::initial(30.0).at_least(30.0))   // Status dot
        .column(Column::initial(170.0).at_least(100.0))  // Name
        .column(Column::initial(70.0).at_least(50.0))    // Element
        .column(Column::initial(140.0).at_least(80.0))   // Rec Class
        .column(Column::initial(50.0).at_least(40.0))    // Evo
        .column(Column::initial(85.0).at_least(60.0))    // Growth
        .column(Column::initial(55.0).at_least(40.0))    // Dng Lv
        .column(Column::initial(100.0).at_least(70.0))   // Class
        .column(Column::initial(50.0).at_least(40.0))    // CL
        .column(Column::initial(50.0).at_least(40.0))    // Imp
        .column(Column::remainder())                      // Action / Special
        .header(22.0, |mut header| {
            header.col(|_| {}); // status dot column, no header
            sortable_header(&mut header, "Name", SortColumn::Name, state);
            sortable_header(&mut header, "Element", SortColumn::Element, state);
            sortable_header(&mut header, "Rec Class", SortColumn::RecClass, state);
            sortable_header(&mut header, "Evo", SortColumn::EvoDifficulty, state);
            sortable_header(&mut header, "Growth", SortColumn::Growth, state);
            sortable_header(&mut header, "Dng Lv", SortColumn::DungeonLevel, state);
            sortable_header(&mut header, "Class", SortColumn::Class, state);
            header.col(|ui| { ui.label(RichText::new("CL").color(style::TEXT_MUTED).strong()); });
            header.col(|ui| { ui.label(RichText::new("Imp").color(style::TEXT_MUTED).strong()); });
            header.col(|ui| { ui.label(RichText::new("Action / Special").color(style::TEXT_MUTED).strong()); });
        })
        .body(|body| {
            body.rows(22.0, pets.len(), |mut row| {
                let pet = pets[row.index()];
                let unlocked = pet.is_unlocked();
                let text_color = if unlocked { style::TEXT_NORMAL } else { style::TEXT_MUTED };

                // Status dot
                row.col(|ui| {
                    widgets::status_dot(ui, unlocked);
                });

                // Name
                row.col(|ui| {
                    ui.label(RichText::new(&pet.name).color(
                        if unlocked { style::TEXT_BRIGHT } else { style::TEXT_MUTED },
                    ));
                });

                // Element
                row.col(|ui| {
                    if let Some(el) = pet.element() {
                        widgets::element_badge(ui, &el);
                    }
                });

                // Rec Class
                row.col(|ui| {
                    if let Some(rec) = pet.recommended_class() {
                        ui.horizontal(|ui| {
                            widgets::recommended_class_label(ui, rec);
                        });
                    }
                });

                // Evo Difficulty
                row.col(|ui| {
                    if let Some(wiki) = &pet.wiki {
                        let evo = &wiki.evo_difficulty;
                        ui.label(
                            RichText::new(format!("{}({})", evo.base, evo.with_conditions))
                                .color(text_color)
                                .size(12.0),
                        );
                    }
                });

                // Growth
                row.col(|ui| {
                    if let Some(export) = &pet.export {
                        ui.label(
                            RichText::new(format_number(export.growth))
                                .color(text_color)
                                .size(12.0)
                                .family(egui::FontFamily::Monospace),
                        );
                    }
                });

                // Dungeon Level
                row.col(|ui| {
                    if let Some(export) = &pet.export {
                        ui.label(
                            RichText::new(export.dungeon_level.to_string())
                                .color(text_color)
                                .size(12.0)
                                .family(egui::FontFamily::Monospace),
                        );
                    }
                });

                // Class
                row.col(|ui| {
                    if let Some(class) = pet.evolved_class() {
                        widgets::class_label(ui, &class);
                    } else if unlocked {
                        ui.label(RichText::new("—").color(style::TEXT_MUTED).size(12.0));
                    }
                });

                // Class Level
                row.col(|ui| {
                    if let Some(export) = &pet.export {
                        if export.class.is_some() {
                            ui.label(
                                RichText::new(export.class_level.to_string())
                                    .color(text_color)
                                    .size(12.0)
                                    .family(egui::FontFamily::Monospace),
                            );
                        }
                    }
                });

                // Improvable / Improved
                row.col(|ui| {
                    let improved = pet.export.as_ref().is_some_and(|e| e.improved);
                    let improvable = pet.wiki.as_ref().is_some_and(|w| w.token_improvable);
                    if improved {
                        ui.label(RichText::new("✓").color(style::SUCCESS).size(12.0));
                    } else if improvable {
                        ui.label(RichText::new("○").color(style::WARNING).size(12.0));
                    }
                });

                // Action / Special
                row.col(|ui| {
                    if let Some(export) = &pet.export {
                        let action_str = format!("{:?}", export.action);
                        ui.label(
                            RichText::new(action_str)
                                .color(style::TEXT_MUTED)
                                .size(11.0),
                        );
                    } else if let Some(wiki) = &pet.wiki {
                        if let Some(special) = &wiki.special_ability {
                            ui.label(
                                RichText::new(special)
                                    .color(style::TEXT_MUTED)
                                    .italics()
                                    .size(11.0),
                            );
                        }
                    }
                });
            });
        });
}

fn sortable_header(
    header: &mut egui_extras::TableRow,
    label: &str,
    column: SortColumn,
    state: &mut AnalyzerState,
) {
    header.col(|ui| {
        let is_active = state.sort_column == column;
        let arrow = if is_active {
            if state.sort_ascending { " ▲" } else { " ▼" }
        } else {
            ""
        };
        let text = RichText::new(format!("{label}{arrow}"))
            .color(if is_active { style::ACCENT } else { style::TEXT_MUTED })
            .strong();
        if ui.add(egui::Label::new(text).sense(egui::Sense::click())).clicked() {
            if is_active {
                state.sort_ascending = !state.sort_ascending;
            } else {
                state.sort_column = column;
                state.sort_ascending = true;
            }
        }
    });
}

// =============================================================================
// Filtering & Sorting
// =============================================================================

fn filter_and_sort<'a>(pets: &'a [MergedPet], state: &AnalyzerState) -> Vec<&'a MergedPet> {
    let search_lower = state.search.to_lowercase();

    let mut filtered: Vec<&MergedPet> = pets
        .iter()
        .filter(|pet| {
            // Search
            if !search_lower.is_empty() && !pet.name.to_lowercase().contains(&search_lower) {
                return false;
            }
            // Element
            if let Some(ref filter_el) = state.filter_element {
                if pet.element().as_ref() != Some(filter_el) {
                    return false;
                }
            }
            // Unlocked
            if let Some(filter_unlock) = state.filter_unlocked {
                if pet.is_unlocked() != filter_unlock {
                    return false;
                }
            }
            // Evolved
            if let Some(filter_evo) = state.filter_evolved {
                if pet.is_evolved() != filter_evo {
                    return false;
                }
            }
            true
        })
        .collect();

    // Sort
    let asc = state.sort_ascending;
    filtered.sort_by(|a, b| {
        let ord = match state.sort_column {
            SortColumn::Name => a.name.cmp(&b.name),
            SortColumn::Element => {
                let ea = a.element().unwrap_or(Element::Neutral);
                let eb = b.element().unwrap_or(Element::Neutral);
                ea.cmp(&eb).then_with(|| a.name.cmp(&b.name))
            }
            SortColumn::RecClass => {
                let ra = rec_class_sort_key(a.recommended_class());
                let rb = rec_class_sort_key(b.recommended_class());
                ra.cmp(&rb).then_with(|| a.name.cmp(&b.name))
            }
            SortColumn::EvoDifficulty => {
                let da = a.wiki.as_ref().map(|w| w.evo_difficulty.base).unwrap_or(99);
                let db = b.wiki.as_ref().map(|w| w.evo_difficulty.base).unwrap_or(99);
                da.cmp(&db).then_with(|| a.name.cmp(&b.name))
            }
            SortColumn::Growth => {
                let ga = a.export.as_ref().map(|e| e.growth).unwrap_or(0);
                let gb = b.export.as_ref().map(|e| e.growth).unwrap_or(0);
                gb.cmp(&ga) // Default: highest growth first
            }
            SortColumn::DungeonLevel => {
                let da = a.export.as_ref().map(|e| e.dungeon_level).unwrap_or(0);
                let db = b.export.as_ref().map(|e| e.dungeon_level).unwrap_or(0);
                db.cmp(&da) // Default: highest level first
            }
            SortColumn::Class => {
                let ca = a.evolved_class().unwrap_or(Class::Wildcard);
                let cb = b.evolved_class().unwrap_or(Class::Wildcard);
                ca.cmp(&cb).then_with(|| a.name.cmp(&b.name))
            }
        };
        if asc { ord } else { ord.reverse() }
    });

    filtered
}

fn rec_class_sort_key(rec: Option<&RecommendedClass>) -> u8 {
    match rec {
        Some(RecommendedClass::Single(c)) => class_order(c),
        Some(RecommendedClass::Dual(c, _)) => class_order(c),
        Some(RecommendedClass::AllClasses) => 50,
        Some(RecommendedClass::DungeonWildcard) => 51,
        Some(RecommendedClass::Wildcard) => 52,
        Some(RecommendedClass::Village(_)) => 60,
        Some(RecommendedClass::Special) => 70,
        Some(RecommendedClass::Alternates) => 71,
        None => 99,
    }
}

fn class_order(c: &Class) -> u8 {
    match c {
        Class::Adventurer => 0,
        Class::Blacksmith => 1,
        Class::Alchemist => 2,
        Class::Defender => 3,
        Class::Supporter => 4,
        Class::Rogue => 5,
        Class::Assassin => 6,
        Class::Mage => 7,
        Class::Wildcard => 8,
    }
}

fn format_number(n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let s = n.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}
