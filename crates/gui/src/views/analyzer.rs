use std::cell::RefCell;

use eframe::egui::{self, Color32, RichText, Ui};
use egui_extras::{Column, TableBuilder};
use itrtg_models::{
    CampaignType, Class, Dungeon, Element, PetAction, RecommendedClass, UnlockCondition,
    VillageJob,
};
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
    PetToken,
    Milestones,
    Special,
    Secret,
    TavernQuest,
    StrategyRoom,
    DungeonBoss,
    PetCount,
    ItemGift,
    AncientMimic,
}

impl UnlockTypeFilter {
    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::DefeatGods => "Defeat Gods",
            Self::PBaal => "P.Baal",
            Self::PetToken => "Pet Token",
            Self::Milestones => "Milestones",
            Self::Special => "Special",
            Self::Secret => "Secret",
            Self::TavernQuest => "Tavern Quest",
            Self::StrategyRoom => "Strategy Room",
            Self::DungeonBoss => "Dungeon Boss",
            Self::PetCount => "Pet Count",
            Self::ItemGift => "Item Gift",
            Self::AncientMimic => "Ancient Mimic",
        }
    }

    fn matches(self, cond: &UnlockCondition) -> bool {
        match self {
            Self::All => true,
            Self::DefeatGods => matches!(cond, UnlockCondition::DefeatGods),
            Self::PBaal => matches!(cond, UnlockCondition::DefeatPBaal(_) | UnlockCondition::DefeatPBaalVersion(_)),
            Self::PetToken => matches!(cond, UnlockCondition::PetToken),
            Self::Milestones => matches!(cond, UnlockCondition::Milestones | UnlockCondition::MilestonesOrPetToken),
            Self::Special => matches!(cond, UnlockCondition::SpecialTask | UnlockCondition::Special),
            Self::Secret => matches!(cond, UnlockCondition::Secret),
            Self::TavernQuest => matches!(cond, UnlockCondition::TavernQuest(_)),
            Self::StrategyRoom => matches!(cond, UnlockCondition::StrategyRoom(_)),
            Self::DungeonBoss => matches!(cond, UnlockCondition::DungeonBoss(_)),
            Self::PetCount => matches!(cond, UnlockCondition::PetCount(_)),
            Self::ItemGift => matches!(cond, UnlockCondition::ItemGift(_)),
            Self::AncientMimic => matches!(cond, UnlockCondition::AncientMimicPoints(_)),
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
    /// Name of the currently selected pet for the detail card.
    pub selected_pet: Option<String>,
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
    Action,
}

impl SortColumn {
    /// Default sort direction: true = ascending.
    /// Text/categorical columns default ascending; numeric columns default descending.
    fn default_ascending(self) -> bool {
        match self {
            Self::Name | Self::Element | Self::RecClass | Self::Class | Self::Action => true,
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

    // Pet detail window (rendered before table so it floats above)
    show_detail_window(ui, state, data);

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

fn show_detail_window(ui: &mut Ui, state: &mut AnalyzerState, data: &DataStore) {
    if let Some(pet_name) = state.selected_pet.clone() {
        let pet = data.merged.iter().find(|p| p.name == pet_name);
        let mut open = true;

        egui::Window::new(format!("Pet: {pet_name}"))
            .open(&mut open)
            .collapsible(true)
            .resizable(true)
            .default_size([400.0, 350.0])
            .show(ui.ctx(), |ui| {
                if let Some(pet) = pet {
                    show_pet_details(ui, pet);
                } else {
                    ui.label(
                        RichText::new("Pet not found in current data.")
                            .color(style::WARNING),
                    );
                }
            });

        if !open {
            state.selected_pet = None;
        }
    }
}

fn show_pet_details(ui: &mut Ui, pet: &MergedPet) {
    // Wiki data section
    if let Some(wiki) = &pet.wiki {
        // Wiki link
        if !wiki.wiki_url.is_empty() {
            ui.hyperlink_to(
                RichText::new("View on Wiki →").color(style::ACCENT).size(12.0),
                &wiki.wiki_url,
            );
        }

        ui.add_space(4.0);

        egui::Grid::new("pet_wiki_grid")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.label(RichText::new("Element:").color(style::TEXT_MUTED).size(12.0));
                widgets::element_badge(ui, &wiki.element);
                ui.end_row();

                ui.label(RichText::new("Rec Class:").color(style::TEXT_MUTED).size(12.0));
                ui.horizontal(|ui| {
                    widgets::recommended_class_label(ui, &wiki.recommended_class);
                });
                ui.end_row();

                ui.label(RichText::new("Class Bonus:").color(style::TEXT_MUTED).size(12.0));
                ui.label(RichText::new(&wiki.class_bonus).color(style::TEXT_NORMAL).size(12.0));
                ui.end_row();

                ui.label(RichText::new("Unlock:").color(style::TEXT_MUTED).size(12.0));
                ui.label(
                    RichText::new(format_unlock_condition(&wiki.unlock_condition))
                        .color(style::TEXT_NORMAL)
                        .size(12.0),
                );
                ui.end_row();

                ui.label(RichText::new("Evo Difficulty:").color(style::TEXT_MUTED).size(12.0));
                let evo = &wiki.evo_difficulty;
                ui.label(
                    RichText::new(format!("{} ({})", evo.base, evo.with_conditions))
                        .color(evo_difficulty_color(evo.base))
                        .size(12.0),
                );
                ui.end_row();

                ui.label(RichText::new("Improvable:").color(style::TEXT_MUTED).size(12.0));
                ui.label(
                    RichText::new(if wiki.token_improvable { "Yes" } else { "No" })
                        .color(if wiki.token_improvable { style::SUCCESS } else { style::TEXT_MUTED })
                        .size(12.0),
                );
                ui.end_row();

                if let Some(special) = &wiki.special_ability {
                    ui.label(RichText::new("Special:").color(style::TEXT_MUTED).size(12.0));
                    ui.label(
                        RichText::new(special)
                            .color(style::ACCENT)
                            .italics()
                            .size(12.0),
                    );
                    ui.end_row();
                }
            });
    }

    // Export data section
    if let Some(export) = &pet.export {
        ui.add_space(8.0);
        ui.separator();
        ui.label(
            RichText::new("Your Pet Data")
                .color(style::TEXT_BRIGHT)
                .size(13.0)
                .strong(),
        );
        ui.add_space(2.0);

        egui::Grid::new("pet_export_grid")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.label(RichText::new("Growth:").color(style::TEXT_MUTED).size(12.0));
                ui.label(
                    RichText::new(format_number(export.growth))
                        .color(style::TEXT_NORMAL)
                        .size(12.0)
                        .family(egui::FontFamily::Monospace),
                );
                ui.end_row();

                ui.label(
                    RichText::new("w/ Magic Egg:")
                        .color(style::TEXT_MUTED)
                        .size(12.0),
                );
                let egg_growth = (export.growth as f64 * 1.3).round() as u64;
                ui.label(
                    RichText::new(format_number(egg_growth))
                        .color(style::TEXT_NORMAL)
                        .size(12.0)
                        .family(egui::FontFamily::Monospace),
                );
                ui.end_row();

                ui.label(RichText::new("Dungeon Lv:").color(style::TEXT_MUTED).size(12.0));
                ui.label(
                    RichText::new(export.dungeon_level.to_string())
                        .color(style::TEXT_NORMAL)
                        .size(12.0)
                        .family(egui::FontFamily::Monospace),
                );
                ui.end_row();

                ui.label(RichText::new("Class:").color(style::TEXT_MUTED).size(12.0));
                ui.horizontal(|ui| {
                    if let Some(class) = &export.class {
                        widgets::class_label(ui, class);
                        ui.label(
                            RichText::new(format!("(CL {})", export.class_level))
                                .color(style::TEXT_MUTED)
                                .size(12.0),
                        );
                    } else {
                        ui.label(RichText::new("Unevolved").color(style::TEXT_MUTED).size(12.0));
                    }
                });
                ui.end_row();

                ui.label(RichText::new("Stats:").color(style::TEXT_MUTED).size(12.0));
                ui.label(
                    RichText::new(format!(
                        "HP {} / ATK {} / DEF {} / SPD {}",
                        format_number(export.combat_stats.hp as u64),
                        format_number(export.combat_stats.attack as u64),
                        format_number(export.combat_stats.defense as u64),
                        format_number(export.combat_stats.speed as u64),
                    ))
                    .color(style::TEXT_NORMAL)
                    .size(11.0)
                    .family(egui::FontFamily::Monospace),
                );
                ui.end_row();

                ui.label(RichText::new("Action:").color(style::TEXT_MUTED).size(12.0));
                ui.label(
                    RichText::new(format_action(&export.action))
                        .color(style::TEXT_NORMAL)
                        .size(12.0),
                );
                ui.end_row();

                if export.improved {
                    ui.label(RichText::new("Improved:").color(style::TEXT_MUTED).size(12.0));
                    ui.label(RichText::new("✓ Yes").color(style::SUCCESS).size(12.0));
                    ui.end_row();
                }

                // Equipment
                let has_equip = export.loadout.weapon.is_some()
                    || export.loadout.armor.is_some()
                    || export.loadout.accessory.is_some();
                if has_equip {
                    ui.label(RichText::new("Equipment:").color(style::TEXT_MUTED).size(12.0));
                    ui.vertical(|ui| {
                        if let Some(w) = &export.loadout.weapon {
                            ui.label(
                                RichText::new(format!("W: {} ({:?})", w.name, w.quality))
                                    .color(style::TEXT_NORMAL)
                                    .size(11.0),
                            );
                        }
                        if let Some(a) = &export.loadout.armor {
                            ui.label(
                                RichText::new(format!("A: {} ({:?})", a.name, a.quality))
                                    .color(style::TEXT_NORMAL)
                                    .size(11.0),
                            );
                        }
                        if let Some(ac) = &export.loadout.accessory {
                            ui.label(
                                RichText::new(format!("Ac: {} ({:?})", ac.name, ac.quality))
                                    .color(style::TEXT_NORMAL)
                                    .size(11.0),
                            );
                        }
                    });
                    ui.end_row();
                }
            });
    }
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

        // Top-50 combined dungeon level (game unlocks based on this total)
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
            ui.separator();
            ui.label(
                RichText::new(format!("Top-{top_n} Dng Lv: {}", format_number(top_sum)))
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
                    UnlockTypeFilter::PetToken,
                    UnlockTypeFilter::Milestones,
                    UnlockTypeFilter::Special,
                    UnlockTypeFilter::Secret,
                    UnlockTypeFilter::TavernQuest,
                    UnlockTypeFilter::StrategyRoom,
                    UnlockTypeFilter::DungeonBoss,
                    UnlockTypeFilter::PetCount,
                    UnlockTypeFilter::ItemGift,
                    UnlockTypeFilter::AncientMimic,
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

    // Track clicks via RefCell so the closure can mutate it
    let clicked_pet: RefCell<Option<String>> = RefCell::new(None);

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
            sortable_header(&mut header, "Action", SortColumn::Action, state);
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

                // Name (clickable for detail card)
                row.col(|ui| {
                    let text = RichText::new(&pet.name).color(
                        if unlocked { style::TEXT_BRIGHT } else { style::TEXT_MUTED },
                    );
                    if ui.add(egui::Label::new(text).sense(egui::Sense::click())).clicked() {
                        *clicked_pet.borrow_mut() = Some(pet.name.clone());
                    }
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
                        ui.label(
                            RichText::new(format_action(&export.action))
                                .color(action_color(&export.action))
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

    // Process click after table rendering
    if let Some(name) = clicked_pet.into_inner() {
        // Toggle: clicking the same pet again closes the detail card
        if state.selected_pet.as_ref() == Some(&name) {
            state.selected_pet = None;
        } else {
            state.selected_pet = Some(name);
        }
    }
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
// Action formatting
// =============================================================================

fn format_action(action: &PetAction) -> String {
    match action {
        PetAction::Idle => "Idle".to_string(),
        PetAction::Crafting => "Crafting".to_string(),
        PetAction::Campaign(ct) => {
            let name = match ct {
                CampaignType::Growth => "Growth",
                CampaignType::Divinity => "Divinity",
                CampaignType::Food => "Food",
                CampaignType::Item => "Item",
                CampaignType::Level => "Level",
                CampaignType::Multiplier => "Multiplier",
                CampaignType::GodPower => "God Power",
            };
            format!("C: {name}")
        }
        PetAction::Dungeon(d) => {
            let name = match d {
                Dungeon::NewbieGround => "Newbie",
                Dungeon::Scrapyard => "Scrapyard",
                Dungeon::WaterTemple => "Water",
                Dungeon::Volcano => "Volcano",
                Dungeon::Mountain => "Mountain",
                Dungeon::Forest => "Forest",
            };
            format!("D: {name}")
        }
        PetAction::Village(vj) => {
            let detail = match vj {
                VillageJob::Fishing(sub) => match sub {
                    Some(s) => format!("Fish ({s})"),
                    None => "Fishing".to_string(),
                },
                VillageJob::MaterialFactory(sub) => match sub {
                    Some(s) => format!("Mat ({s})"),
                    None => "Material".to_string(),
                },
                VillageJob::AlchemyHut => "Alchemy".to_string(),
                VillageJob::Dojo => "Dojo".to_string(),
                VillageJob::StrategyRoom => "Strategy".to_string(),
                VillageJob::Questing(sub) => match sub {
                    Some(s) => format!("Quest ({s})"),
                    None => "Questing".to_string(),
                },
            };
            format!("V: {detail}")
        }
    }
}

fn action_color(action: &PetAction) -> Color32 {
    match action {
        PetAction::Idle => style::TEXT_MUTED,
        PetAction::Campaign(_) => Color32::from_rgb(0x88, 0xcc, 0xff),
        PetAction::Dungeon(_) => Color32::from_rgb(0xff, 0xaa, 0x88),
        PetAction::Crafting => Color32::from_rgb(0xff, 0xcc, 0x66),
        PetAction::Village(_) => Color32::from_rgb(0x88, 0xdd, 0x88),
    }
}

/// Sort key for actions: groups by type, then by sub-variant.
fn action_sort_key(action: &PetAction) -> u16 {
    match action {
        PetAction::Idle => 0,
        PetAction::Campaign(ct) => 10 + match ct {
            CampaignType::Growth => 0,
            CampaignType::Divinity => 1,
            CampaignType::Food => 2,
            CampaignType::Item => 3,
            CampaignType::Level => 4,
            CampaignType::Multiplier => 5,
            CampaignType::GodPower => 6,
        },
        PetAction::Dungeon(d) => 20 + match d {
            Dungeon::NewbieGround => 0,
            Dungeon::Scrapyard => 1,
            Dungeon::WaterTemple => 2,
            Dungeon::Volcano => 3,
            Dungeon::Mountain => 4,
            Dungeon::Forest => 5,
        },
        PetAction::Crafting => 30,
        PetAction::Village(_) => 40,
    }
}

// =============================================================================
// Unlock condition formatting
// =============================================================================

fn format_unlock_condition(cond: &UnlockCondition) -> String {
    match cond {
        UnlockCondition::DefeatGods => "Defeat Gods".to_string(),
        UnlockCondition::DefeatPBaal(n) => format!("Defeat P.Baal {n}"),
        UnlockCondition::DefeatPBaalVersion(n) => format!("Defeat P.Baal v{n}"),
        UnlockCondition::SpecialTask => "Special Task".to_string(),
        UnlockCondition::PetToken => "Pet Token".to_string(),
        UnlockCondition::MilestonesOrPetToken => "Milestones or Pet Token".to_string(),
        UnlockCondition::Milestones => "Milestones".to_string(),
        UnlockCondition::Secret => "Secret".to_string(),
        UnlockCondition::Special => "Special".to_string(),
        UnlockCondition::TavernQuest(rank) => format!("Tavern Quest ({rank})"),
        UnlockCondition::StrategyRoom(level) => format!("Strategy Room Lv.{level}"),
        UnlockCondition::AncientMimicPoints(pts) => format!("Ancient Mimic ({pts} pts)"),
        UnlockCondition::PetCount(n) => format!("{n} Pets Unlocked"),
        UnlockCondition::DungeonBoss(boss) => format!("Dungeon Boss: {boss}"),
        UnlockCondition::ItemGift(item) => format!("Item Gift: {item}"),
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

    // Sort — growth descending is the universal tiebreaker (strongest first in ties)
    let asc = state.sort_ascending;
    filtered.sort_by(|a, b| {
        let ga = a.export.as_ref().map(|e| e.growth).unwrap_or(0);
        let gb = b.export.as_ref().map(|e| e.growth).unwrap_or(0);

        let ord = match state.sort_column {
            SortColumn::Name => a.name.cmp(&b.name),
            SortColumn::Element => {
                let ea = a.element().unwrap_or(Element::Neutral);
                let eb = b.element().unwrap_or(Element::Neutral);
                ea.cmp(&eb).then_with(|| gb.cmp(&ga))
            }
            SortColumn::RecClass => {
                let ra = rec_class_sort_key(a.recommended_class());
                let rb = rec_class_sort_key(b.recommended_class());
                ra.cmp(&rb).then_with(|| gb.cmp(&ga))
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
                da.cmp(&db).then_with(|| gb.cmp(&ga))
            }
            SortColumn::Growth => {
                ga.cmp(&gb).then_with(|| a.name.cmp(&b.name))
            }
            SortColumn::DungeonLevel => {
                let da = a.export.as_ref().map(|e| e.dungeon_level).unwrap_or(0);
                let db = b.export.as_ref().map(|e| e.dungeon_level).unwrap_or(0);
                da.cmp(&db).then_with(|| gb.cmp(&ga))
            }
            SortColumn::Class => {
                let ca = a.evolved_class().unwrap_or(Class::Wildcard);
                let cb = b.evolved_class().unwrap_or(Class::Wildcard);
                ca.cmp(&cb).then_with(|| gb.cmp(&ga))
            }
            SortColumn::ClassLevel => {
                let la = a.export.as_ref().filter(|e| e.class.is_some()).map(|e| e.class_level).unwrap_or(0);
                let lb = b.export.as_ref().filter(|e| e.class.is_some()).map(|e| e.class_level).unwrap_or(0);
                // Class level → class → growth
                let ca = a.evolved_class().unwrap_or(Class::Wildcard);
                let cb = b.evolved_class().unwrap_or(Class::Wildcard);
                la.cmp(&lb).then_with(|| ca.cmp(&cb)).then_with(|| gb.cmp(&ga))
            }
            SortColumn::Action => {
                let ka = a.export.as_ref().map(|e| action_sort_key(&e.action)).unwrap_or(999);
                let kb = b.export.as_ref().map(|e| action_sort_key(&e.action)).unwrap_or(999);
                ka.cmp(&kb).then_with(|| gb.cmp(&ga))
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
