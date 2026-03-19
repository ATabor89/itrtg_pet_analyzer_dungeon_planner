use eframe::egui::{self, Color32, RichText, Ui};
use egui_extras::{Column, TableBuilder};
use itrtg_models::{Class, Element, RecommendedClass, UnlockCondition};
use itrtg_planner::merge::MergedPet;

use crate::data::DataStore;
use crate::style;
use super::widgets;

// =============================================================================
// Filter enums
// =============================================================================

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum UnlockTypeFilter {
    #[default]
    All,
    DefeatGods,
    PBaal,
    PBaalVersion,
    PetToken,
    Milestones,
    SpecialTask,
    Secret,
    TavernQuest,
    StrategyRoom,
    DungeonBoss,
    PetCount,
    ItemGift,
    AncientMimic,
    Special,
}

impl UnlockTypeFilter {
    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::DefeatGods => "Defeat Gods",
            Self::PBaal => "P.Baal",
            Self::PBaalVersion => "P.Baal Ver",
            Self::PetToken => "Pet Token",
            Self::Milestones => "Milestones",
            Self::SpecialTask => "Special Task",
            Self::Secret => "Secret",
            Self::TavernQuest => "Tavern Quest",
            Self::StrategyRoom => "Strategy Room",
            Self::DungeonBoss => "Dungeon Boss",
            Self::PetCount => "Pet Count",
            Self::ItemGift => "Item Gift",
            Self::AncientMimic => "Ancient Mimic",
            Self::Special => "Special",
        }
    }

    fn matches(self, cond: &UnlockCondition) -> bool {
        match self {
            Self::All => true,
            Self::DefeatGods => matches!(cond, UnlockCondition::DefeatGods),
            Self::PBaal => matches!(cond, UnlockCondition::DefeatPBaal(_)),
            Self::PBaalVersion => matches!(cond, UnlockCondition::DefeatPBaalVersion(_)),
            Self::PetToken => matches!(cond, UnlockCondition::PetToken),
            Self::Milestones => matches!(cond, UnlockCondition::Milestones | UnlockCondition::MilestonesOrPetToken),
            Self::SpecialTask => matches!(cond, UnlockCondition::SpecialTask),
            Self::Secret => matches!(cond, UnlockCondition::Secret),
            Self::TavernQuest => matches!(cond, UnlockCondition::TavernQuest(_)),
            Self::StrategyRoom => matches!(cond, UnlockCondition::StrategyRoom(_)),
            Self::DungeonBoss => matches!(cond, UnlockCondition::DungeonBoss(_)),
            Self::PetCount => matches!(cond, UnlockCondition::PetCount(_)),
            Self::ItemGift => matches!(cond, UnlockCondition::ItemGift(_)),
            Self::AncientMimic => matches!(cond, UnlockCondition::AncientMimicPoints(_)),
            Self::Special => matches!(cond, UnlockCondition::Special),
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum RecClassFilter {
    #[default]
    All,
    Adventurer,
    Blacksmith,
    Alchemist,
    Defender,
    Supporter,
    Rogue,
    Assassin,
    Mage,
    Wildcard,
    DungeonWildcard,
    AllClasses,
    Village,
    Special,
}

impl RecClassFilter {
    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Adventurer => "Adventurer",
            Self::Blacksmith => "Blacksmith",
            Self::Alchemist => "Alchemist",
            Self::Defender => "Defender",
            Self::Supporter => "Supporter",
            Self::Rogue => "Rogue",
            Self::Assassin => "Assassin",
            Self::Mage => "Mage",
            Self::Wildcard => "Wildcard",
            Self::DungeonWildcard => "Dng Wildcard",
            Self::AllClasses => "All Classes",
            Self::Village => "Village",
            Self::Special => "Special/Alt",
        }
    }

    fn matches(self, rec: &RecommendedClass) -> bool {
        match self {
            Self::All => true,
            Self::Adventurer => rec_includes_class(rec, Class::Adventurer),
            Self::Blacksmith => rec_includes_class(rec, Class::Blacksmith),
            Self::Alchemist => rec_includes_class(rec, Class::Alchemist),
            Self::Defender => rec_includes_class(rec, Class::Defender),
            Self::Supporter => rec_includes_class(rec, Class::Supporter),
            Self::Rogue => rec_includes_class(rec, Class::Rogue),
            Self::Assassin => rec_includes_class(rec, Class::Assassin),
            Self::Mage => rec_includes_class(rec, Class::Mage),
            Self::Wildcard => matches!(rec, RecommendedClass::Wildcard),
            Self::DungeonWildcard => matches!(rec, RecommendedClass::DungeonWildcard),
            Self::AllClasses => matches!(rec, RecommendedClass::AllClasses),
            Self::Village => matches!(rec, RecommendedClass::Village(_)),
            Self::Special => matches!(rec, RecommendedClass::Special | RecommendedClass::Alternates),
        }
    }
}

fn rec_includes_class(rec: &RecommendedClass, target: Class) -> bool {
    match rec {
        RecommendedClass::Single(c) => *c == target,
        RecommendedClass::Dual(a, b) => *a == target || *b == target,
        _ => false,
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum MyClassFilter {
    #[default]
    All,
    Unevolved,
    Adventurer,
    Blacksmith,
    Alchemist,
    Defender,
    Supporter,
    Rogue,
    Assassin,
    Mage,
    Wildcard,
}

impl MyClassFilter {
    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Unevolved => "Unevolved",
            Self::Adventurer => "Adventurer",
            Self::Blacksmith => "Blacksmith",
            Self::Alchemist => "Alchemist",
            Self::Defender => "Defender",
            Self::Supporter => "Supporter",
            Self::Rogue => "Rogue",
            Self::Assassin => "Assassin",
            Self::Mage => "Mage",
            Self::Wildcard => "Wildcard",
        }
    }

    fn matches(self, class: Option<Class>) -> bool {
        match self {
            Self::All => true,
            Self::Unevolved => class.is_none(),
            Self::Adventurer => class == Some(Class::Adventurer),
            Self::Blacksmith => class == Some(Class::Blacksmith),
            Self::Alchemist => class == Some(Class::Alchemist),
            Self::Defender => class == Some(Class::Defender),
            Self::Supporter => class == Some(Class::Supporter),
            Self::Rogue => class == Some(Class::Rogue),
            Self::Assassin => class == Some(Class::Assassin),
            Self::Mage => class == Some(Class::Mage),
            Self::Wildcard => class == Some(Class::Wildcard),
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ImprovableFilter {
    #[default]
    All,
    Improvable,
    Improved,
    NotImproved,
}

impl ImprovableFilter {
    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Improvable => "Improvable",
            Self::Improved => "Improved",
            Self::NotImproved => "Not Improved",
        }
    }
}

// =============================================================================
// State
// =============================================================================

#[derive(Default)]
pub struct AnalyzerState {
    pub search: String,
    pub filter_element: Option<Element>,
    pub filter_unlocked: Option<bool>,
    pub filter_evolved: Option<bool>,
    pub filter_unlock_type: UnlockTypeFilter,
    pub filter_rec_class: RecClassFilter,
    pub filter_my_class: MyClassFilter,
    pub filter_improvable: ImprovableFilter,
    pub sort_column: SortColumn,
    pub sort_ascending: bool,
    sort_initialized: bool,
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
    ClassLevel,
}

impl SortColumn {
    /// Default sort direction: true = ascending.
    /// Text/categorical columns default ascending; numeric columns default descending.
    fn default_ascending(self) -> bool {
        match self {
            Self::Name | Self::Element | Self::RecClass | Self::Class => true,
            Self::EvoDifficulty | Self::Growth | Self::DungeonLevel | Self::ClassLevel => false,
        }
    }
}

// =============================================================================
// Rendering
// =============================================================================

pub fn show(ui: &mut Ui, state: &mut AnalyzerState, data: &DataStore) {
    // Initialize default sort direction on first frame
    if !state.sort_initialized {
        state.sort_ascending = state.sort_column.default_ascending();
        state.sort_initialized = true;
    }

    // Stats bar
    show_stats_bar(ui, data);

    ui.add_space(4.0);

    // Filter bars (two rows)
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

        // Total growth of unlocked pets
        let total_growth: u64 = data
            .merged
            .iter()
            .filter_map(|p| p.export.as_ref())
            .filter(|e| e.unlocked)
            .map(|e| e.growth)
            .sum();
        if total_growth > 0 {
            ui.separator();
            ui.label(
                RichText::new(format!("Growth: {}", format_number(total_growth)))
                    .color(style::TEXT_NORMAL)
                    .size(13.0),
            );
        }

        // Top-50 average dungeon level
        let mut dungeon_levels: Vec<u32> = data
            .merged
            .iter()
            .filter_map(|p| p.export.as_ref())
            .filter(|e| e.unlocked && e.dungeon_level > 0)
            .map(|e| e.dungeon_level)
            .collect();
        if !dungeon_levels.is_empty() {
            dungeon_levels.sort_unstable_by(|a, b| b.cmp(a));
            let top_n = dungeon_levels.len().min(50);
            let top_sum: u64 = dungeon_levels[..top_n].iter().map(|&x| x as u64).sum();
            let avg = top_sum as f64 / top_n as f64;
            ui.separator();
            ui.label(
                RichText::new(format!("Top-{top_n} Dng Avg: {avg:.1}"))
                    .color(style::TEXT_NORMAL)
                    .size(13.0),
            );
        }

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
    // Row 1: Search, Element, Unlocked, Evolved
    ui.horizontal(|ui| {
        ui.label(RichText::new("Search:").color(style::TEXT_MUTED));
        ui.add(
            egui::TextEdit::singleline(&mut state.search)
                .desired_width(150.0)
                .hint_text("Name or ability..."),
        );

        ui.separator();

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

    // Row 2: Unlock Type, Rec Class, My Class, Improvable
    ui.horizontal(|ui| {
        ui.label(RichText::new("Unlock:").color(style::TEXT_MUTED));
        egui::ComboBox::from_id_salt("unlock_type_filter")
            .selected_text(state.filter_unlock_type.label())
            .show_ui(ui, |ui| {
                for f in [
                    UnlockTypeFilter::All,
                    UnlockTypeFilter::DefeatGods,
                    UnlockTypeFilter::PBaal,
                    UnlockTypeFilter::PBaalVersion,
                    UnlockTypeFilter::PetToken,
                    UnlockTypeFilter::Milestones,
                    UnlockTypeFilter::SpecialTask,
                    UnlockTypeFilter::Secret,
                    UnlockTypeFilter::TavernQuest,
                    UnlockTypeFilter::StrategyRoom,
                    UnlockTypeFilter::DungeonBoss,
                    UnlockTypeFilter::PetCount,
                    UnlockTypeFilter::ItemGift,
                    UnlockTypeFilter::AncientMimic,
                    UnlockTypeFilter::Special,
                ] {
                    ui.selectable_value(&mut state.filter_unlock_type, f, f.label());
                }
            });

        ui.separator();

        ui.label(RichText::new("Rec Class:").color(style::TEXT_MUTED));
        egui::ComboBox::from_id_salt("rec_class_filter")
            .selected_text(state.filter_rec_class.label())
            .show_ui(ui, |ui| {
                for f in [
                    RecClassFilter::All,
                    RecClassFilter::Adventurer,
                    RecClassFilter::Blacksmith,
                    RecClassFilter::Alchemist,
                    RecClassFilter::Defender,
                    RecClassFilter::Supporter,
                    RecClassFilter::Rogue,
                    RecClassFilter::Assassin,
                    RecClassFilter::Mage,
                    RecClassFilter::Wildcard,
                    RecClassFilter::DungeonWildcard,
                    RecClassFilter::AllClasses,
                    RecClassFilter::Village,
                    RecClassFilter::Special,
                ] {
                    ui.selectable_value(&mut state.filter_rec_class, f, f.label());
                }
            });

        ui.separator();

        ui.label(RichText::new("My Class:").color(style::TEXT_MUTED));
        egui::ComboBox::from_id_salt("my_class_filter")
            .selected_text(state.filter_my_class.label())
            .show_ui(ui, |ui| {
                for f in [
                    MyClassFilter::All,
                    MyClassFilter::Unevolved,
                    MyClassFilter::Adventurer,
                    MyClassFilter::Blacksmith,
                    MyClassFilter::Alchemist,
                    MyClassFilter::Defender,
                    MyClassFilter::Supporter,
                    MyClassFilter::Rogue,
                    MyClassFilter::Assassin,
                    MyClassFilter::Mage,
                    MyClassFilter::Wildcard,
                ] {
                    ui.selectable_value(&mut state.filter_my_class, f, f.label());
                }
            });

        ui.separator();

        ui.label(RichText::new("Improved:").color(style::TEXT_MUTED));
        egui::ComboBox::from_id_salt("improvable_filter")
            .selected_text(state.filter_improvable.label())
            .show_ui(ui, |ui| {
                for f in [
                    ImprovableFilter::All,
                    ImprovableFilter::Improvable,
                    ImprovableFilter::Improved,
                    ImprovableFilter::NotImproved,
                ] {
                    ui.selectable_value(&mut state.filter_improvable, f, f.label());
                }
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
            sortable_header(&mut header, "CL", SortColumn::ClassLevel, state);
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

                // Evo Difficulty (color-coded)
                row.col(|ui| {
                    if let Some(wiki) = &pet.wiki {
                        let evo = &wiki.evo_difficulty;
                        let color = evo_difficulty_color(evo.base);
                        ui.label(
                            RichText::new(format!("{}({})", evo.base, evo.with_conditions))
                                .color(color)
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
                state.sort_ascending = column.default_ascending();
            }
        }
    });
}

/// Color-code evo difficulty: low = green, mid = yellow, high = red.
fn evo_difficulty_color(base: u8) -> Color32 {
    match base {
        1 => style::SUCCESS,
        2 => Color32::from_rgb(0x88, 0xdd, 0x88),
        3 => Color32::from_rgb(0xbb, 0xdd, 0x66),
        4 => Color32::from_rgb(0xdd, 0xcc, 0x44),
        5 => style::WARNING,
        6 => Color32::from_rgb(0xdd, 0x88, 0x44),
        7 => Color32::from_rgb(0xdd, 0x66, 0x44),
        _ => style::ERROR,
    }
}

// =============================================================================
// Filtering & Sorting
// =============================================================================

fn filter_and_sort<'a>(pets: &'a [MergedPet], state: &AnalyzerState) -> Vec<&'a MergedPet> {
    let search_lower = state.search.to_lowercase();

    let mut filtered: Vec<&MergedPet> = pets
        .iter()
        .filter(|pet| {
            // Search: matches name OR special ability
            if !search_lower.is_empty() {
                let name_match = pet.name.to_lowercase().contains(&search_lower);
                let ability_match = pet
                    .wiki
                    .as_ref()
                    .and_then(|w| w.special_ability.as_ref())
                    .is_some_and(|a| a.to_lowercase().contains(&search_lower));
                if !name_match && !ability_match {
                    return false;
                }
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

            // Unlock type
            if state.filter_unlock_type != UnlockTypeFilter::All {
                match pet.wiki.as_ref() {
                    Some(wiki) => {
                        if !state.filter_unlock_type.matches(&wiki.unlock_condition) {
                            return false;
                        }
                    }
                    None => return false,
                }
            }

            // Recommended class
            if state.filter_rec_class != RecClassFilter::All {
                match pet.recommended_class() {
                    Some(rec) => {
                        if !state.filter_rec_class.matches(rec) {
                            return false;
                        }
                    }
                    None => return false,
                }
            }

            // My class (actual evolved class)
            if state.filter_my_class != MyClassFilter::All {
                if !state.filter_my_class.matches(pet.evolved_class()) {
                    return false;
                }
            }

            // Improvable filter
            match state.filter_improvable {
                ImprovableFilter::All => {}
                ImprovableFilter::Improvable => {
                    if !pet.wiki.as_ref().is_some_and(|w| w.token_improvable) {
                        return false;
                    }
                }
                ImprovableFilter::Improved => {
                    if !pet.export.as_ref().is_some_and(|e| e.improved) {
                        return false;
                    }
                }
                ImprovableFilter::NotImproved => {
                    let improvable = pet.wiki.as_ref().is_some_and(|w| w.token_improvable);
                    let improved = pet.export.as_ref().is_some_and(|e| e.improved);
                    if !improvable || improved {
                        return false;
                    }
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
                let da = a
                    .wiki
                    .as_ref()
                    .map(|w| (w.evo_difficulty.base, w.evo_difficulty.with_conditions))
                    .unwrap_or((99, 99));
                let db = b
                    .wiki
                    .as_ref()
                    .map(|w| (w.evo_difficulty.base, w.evo_difficulty.with_conditions))
                    .unwrap_or((99, 99));
                da.cmp(&db).then_with(|| {
                    // Growth tiebreaker in same direction as primary sort
                    let ga = a.export.as_ref().map(|e| e.growth).unwrap_or(0);
                    let gb = b.export.as_ref().map(|e| e.growth).unwrap_or(0);
                    gb.cmp(&ga)
                })
            }
            SortColumn::Growth => {
                let ga = a.export.as_ref().map(|e| e.growth).unwrap_or(0);
                let gb = b.export.as_ref().map(|e| e.growth).unwrap_or(0);
                ga.cmp(&gb).then_with(|| a.name.cmp(&b.name))
            }
            SortColumn::DungeonLevel => {
                let da = a.export.as_ref().map(|e| e.dungeon_level).unwrap_or(0);
                let db = b.export.as_ref().map(|e| e.dungeon_level).unwrap_or(0);
                da.cmp(&db).then_with(|| a.name.cmp(&b.name))
            }
            SortColumn::Class => {
                let ca = a.evolved_class().unwrap_or(Class::Wildcard);
                let cb = b.evolved_class().unwrap_or(Class::Wildcard);
                ca.cmp(&cb).then_with(|| a.name.cmp(&b.name))
            }
            SortColumn::ClassLevel => {
                let la = a
                    .export
                    .as_ref()
                    .filter(|e| e.class.is_some())
                    .map(|e| e.class_level)
                    .unwrap_or(0);
                let lb = b
                    .export
                    .as_ref()
                    .filter(|e| e.class.is_some())
                    .map(|e| e.class_level)
                    .unwrap_or(0);
                la.cmp(&lb).then_with(|| a.name.cmp(&b.name))
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
