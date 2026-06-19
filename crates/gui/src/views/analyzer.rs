use std::cell::RefCell;

use eframe::egui::{self, Color32, RichText, Ui};
use egui_extras::{Column, TableBuilder};
use itrtg_models::{
    parse_flexible_number, CampaignInputs, CampaignType, Class, Dungeon, Element,
    GrowthRequirement, MainStats, MAGIC_EGG_GROWTH_MULT, PetAction, RecommendedClass,
    UnlockCondition, VillageJob,
};
use itrtg_planner::growth::{format_duration, CapRelation, GrowthRates};
use itrtg_planner::merge::{CampaignContext, EvoReadiness, MergedPet};
use serde::{Deserialize, Serialize};

use crate::data::DataStore;
use crate::state::AppState;
use crate::style;
use super::widgets;
use super::widgets::DragValueExt;

// =============================================================================
// Filter enums
// =============================================================================

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// One Moai statue (Easter 2026 event): whether the player owns it and its
/// level (1–20). Only two exist in-game, so the UI shows a fixed pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MoaiStatue {
    pub owned: bool,
    pub level: u8,
}

impl Default for MoaiStatue {
    fn default() -> Self {
        // Level defaults to the max (20); only counts once `owned` is ticked.
        Self { owned: false, level: 20 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AnalyzerState {
    /// Search text — intentionally not persisted; feels stale on relaunch.
    #[serde(skip)]
    pub search: String,
    pub filter_element: Option<Element>,
    pub filter_unlocked: Option<bool>,
    pub filter_evolved: Option<bool>,
    pub filter_unlock_type: UnlockTypeFilter,
    pub filter_rec_class: RecClassFilter,
    pub filter_my_class: MyClassFilter,
    pub filter_improvable: ImprovableFilter,
    /// Show only pets with a (positive) parsed bonus to this campaign. Also the
    /// campaign the `CampaignBonus` sort ranks by. Persisted.
    pub filter_campaign: Option<CampaignType>,
    pub sort_column: SortColumn,
    pub sort_ascending: bool,
    /// The two Moai statues (Easter 2026) used for growth-time estimates —
    /// owned flag + level each. Persisted (player config, not in the export).
    /// Renamed from the earlier free-list `moai_statues`; the old key is simply
    /// ignored on load.
    pub moai: [MoaiStatue; 2],
    /// Global growth target (base growth) used for the "time to target" table
    /// sort. Persisted; `0` means unset. Distinct from the pet card's ephemeral
    /// `custom_target` scratch input.
    pub global_growth_target: u64,
    /// Whether the "time to evolve" sort uses egg-boosted targets (base =
    /// threshold / 1.3) for total-growth thresholds. Persisted. Base-growth
    /// pets (Baby Carno) ignore this either way.
    pub evolve_sort_use_egg: bool,
    /// Secondary key for the time-based sorts when times tie. Persisted.
    pub time_sort_tiebreak: TimeSortTiebreak,
    /// Player-entered values for formula-based campaign bonuses (pet stones,
    /// challenge points, honey, …). Persisted.
    pub campaign_inputs: CampaignInputs,
    /// Include each pet's campaign-boost *equipment* (sticks) in its effective
    /// bonus, on top of innate. Off by default. Persisted.
    pub include_equipment_bonus: bool,
    /// Include each pet's *class* campaign bonus (Adventurer 2%·CL) in its
    /// effective bonus. Off by default. Persisted.
    pub include_class_bonus: bool,
    /// Editable text for Earth Eater's "total Earthlike Planets eaten" input.
    /// Accepts engineering/scientific notation (e.g. `32.4e6`); parsed into
    /// `campaign_inputs.earth_eater_total_planets` each frame. Persisted so the
    /// typed form is preserved across launches.
    pub earth_eater_planets_text: String,
    /// Name of the currently selected pet for the detail card —
    /// not persisted; a detail window reopening on launch feels stale.
    #[serde(skip)]
    pub selected_pet: Option<String>,
    /// Scratch input for the detail card's growth-time calculator — ephemeral,
    /// shared across pets so a target can be compared between them.
    #[serde(skip)]
    pub custom_target: String,
}

impl Default for AnalyzerState {
    fn default() -> Self {
        // `sort_ascending` must match `SortColumn::default().default_ascending()`
        // so that fresh state and deserialized state agree.
        Self {
            search: String::new(),
            filter_element: None,
            filter_unlocked: None,
            filter_evolved: None,
            filter_unlock_type: UnlockTypeFilter::default(),
            filter_rec_class: RecClassFilter::default(),
            filter_my_class: MyClassFilter::default(),
            filter_improvable: ImprovableFilter::default(),
            filter_campaign: None,
            sort_column: SortColumn::default(),
            sort_ascending: SortColumn::default().default_ascending(),
            moai: [MoaiStatue::default(), MoaiStatue::default()],
            global_growth_target: 0,
            evolve_sort_use_egg: false,
            time_sort_tiebreak: TimeSortTiebreak::default(),
            campaign_inputs: CampaignInputs::default(),
            include_equipment_bonus: false,
            include_class_bonus: false,
            earth_eater_planets_text: String::new(),
            selected_pet: None,
            custom_target: String::new(),
        }
    }
}

impl AnalyzerState {
    /// Absorb persisted analyzer state from the unified `AppState`.
    /// Preserves ephemeral fields (`search`, `selected_pet`).
    pub fn apply_app_state(&mut self, state: &AppState) {
        let src = &state.analyzer;
        self.filter_element = src.filter_element;
        self.filter_unlocked = src.filter_unlocked;
        self.filter_evolved = src.filter_evolved;
        self.filter_unlock_type = src.filter_unlock_type;
        self.filter_rec_class = src.filter_rec_class;
        self.filter_my_class = src.filter_my_class;
        self.filter_improvable = src.filter_improvable;
        self.filter_campaign = src.filter_campaign;
        self.sort_column = src.sort_column;
        self.sort_ascending = src.sort_ascending;
        self.moai = src.moai.clone();
        self.global_growth_target = src.global_growth_target;
        self.evolve_sort_use_egg = src.evolve_sort_use_egg;
        self.time_sort_tiebreak = src.time_sort_tiebreak;
        self.campaign_inputs = src.campaign_inputs.clone();
        self.include_equipment_bonus = src.include_equipment_bonus;
        self.include_class_bonus = src.include_class_bonus;
        self.earth_eater_planets_text = src.earth_eater_planets_text.clone();
    }

    /// Copy persistable analyzer state into the unified `AppState`.
    pub fn write_into(&self, state: &mut AppState) {
        state.analyzer = self.clone();
    }

    /// Auto-fill campaign inputs (and the Moai statues) from a parsed Main-stats
    /// export. Returns short labels for the fields that were filled (for a status
    /// message). Only values present in the export are applied; the rest are left
    /// untouched, so importing never clears a field the export didn't carry.
    pub fn apply_main_stats(&mut self, ms: &MainStats) -> Vec<&'static str> {
        let ci = &mut self.campaign_inputs;
        let mut applied = Vec::new();
        if let Some(v) = ms.pet_stones {
            ci.pet_stones = v;
            applied.push("pet stones");
        }
        if let Some(v) = ms.ants {
            ci.ants = v;
            applied.push("ants");
        }
        if let Some(v) = ms.honey_consumed_by_bear {
            ci.honey = v;
            applied.push("Bear honey");
        }
        if let Some(v) = ms.challenge_points {
            ci.challenge_points = v;
            applied.push("challenge points");
        }
        if let Some(v) = ms.goblin_ucc {
            ci.goblin_ucc = v;
            applied.push("Goblin UCC");
        }
        if let Some(v) = ms.goblin_oc {
            ci.goblin_oc = v;
            applied.push("Goblin OC");
        }
        if let Some(v) = ms.stone_campaign_upgrade {
            ci.stone_campaign_upgrade = v;
            applied.push("Stone upgrade");
        }
        // Only adopt the raw string if it actually parses, so a malformed export
        // can't overwrite the field with something that reads back as 0.
        if let Some(text) = &ms.earth_eater_planets_text
            && parse_flexible_number(text).is_some()
        {
            self.earth_eater_planets_text = text.clone();
            applied.push("Earth Eater planets");
        }
        // Base growth/hour of exactly 2 ⇒ both Moai owned at level 20 (unambiguous).
        if ms.base_growth_per_hour == Some(2) {
            self.moai = [
                MoaiStatue { owned: true, level: 20 },
                MoaiStatue { owned: true, level: 20 },
            ];
            applied.push("Moai (both, L20)");
        }
        applied
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Estimated time to grow each pet to its evolution threshold. Not a table
    /// column — triggered from the growth-settings panel.
    TimeToEvolve,
    /// Estimated time to grow each pet to the global custom target. Not a table
    /// column — triggered from the growth-settings panel.
    TimeToTarget,
    /// Pets ranked by their bonus to `filter_campaign`. Not a table column —
    /// triggered alongside the campaign filter.
    CampaignBonus,
}

impl SortColumn {
    /// Default sort direction: true = ascending.
    /// Text/categorical columns default ascending; numeric columns default descending.
    fn default_ascending(self) -> bool {
        match self {
            Self::Name | Self::Element | Self::RecClass | Self::Class | Self::Action => true,
            Self::EvoDifficulty | Self::Growth | Self::DungeonLevel | Self::ClassLevel => false,
            // Time sorts: soonest first.
            Self::TimeToEvolve | Self::TimeToTarget => true,
            // Campaign bonus: biggest boost first.
            Self::CampaignBonus => false,
        }
    }
}

/// Display label for a campaign type.
fn campaign_label(c: CampaignType) -> &'static str {
    match c {
        CampaignType::Growth => "Growth",
        CampaignType::Divinity => "Divinity",
        CampaignType::Food => "Food",
        CampaignType::Item => "Item",
        CampaignType::Level => "Level",
        CampaignType::Multiplier => "Multiplier",
        CampaignType::GodPower => "God Power",
    }
}

const ALL_CAMPAIGNS: [CampaignType; 7] = [
    CampaignType::Growth,
    CampaignType::Divinity,
    CampaignType::Food,
    CampaignType::Item,
    CampaignType::Level,
    CampaignType::Multiplier,
    CampaignType::GodPower,
];

/// Secondary sort key for the time-based sorts, applied when two pets have the
/// same estimated time (common when several already meet the target).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeSortTiebreak {
    /// Higher effective growth first — matches every other column's tiebreaker;
    /// favors pets that add more to total-growth milestones and stats.
    #[default]
    Growth,
    /// Easier-to-evolve first (by wiki evo difficulty) — accounts for the
    /// material and third-condition grind beyond growth.
    EvoDifficulty,
}

// =============================================================================
// Rendering
// =============================================================================

pub fn show(ui: &mut Ui, state: &mut AnalyzerState, data: &DataStore) {
    // Base-growth rates from the roster + the player's owned Moai statues.
    // Computed once per frame; edits to Moai below take effect next frame.
    let moai_levels: Vec<u8> = state
        .moai
        .iter()
        .filter(|m| m.owned)
        .map(|m| m.level)
        .collect();
    let rates = GrowthRates::compute(&data.merged, &moai_levels);

    // Effective-campaign-bonus context (curated overrides applied to the parsed
    // baseline). Cheap to build; borrows the loaded overrides.
    // Clone the (tiny) inputs so the context doesn't borrow `state`, which is
    // mutated by the panels below. Edits take effect next frame.
    let mut campaign_inputs = state.campaign_inputs.clone();
    // Earth Eater's "total planets" is entered as flexible-notation text; parse
    // it here so the value is current regardless of the inputs panel being open.
    campaign_inputs.earth_eater_total_planets =
        parse_flexible_number(&state.earth_eater_planets_text).unwrap_or(0.0).max(0.0) as u64;
    let camp_ctx = CampaignContext {
        bonuses: &data.campaign_bonuses,
        roster: &data.merged,
        inputs: &campaign_inputs,
        include_equipment: state.include_equipment_bonus,
        include_class: state.include_class_bonus,
    };

    // Pet detail window (rendered before table so it floats above)
    show_detail_window(ui, state, data, &rates, &camp_ctx);

    // Stats bar
    show_stats_bar(ui, data);

    ui.add_space(4.0);

    // Filter bars (two rows)
    show_filters(ui, state);

    // Growth-estimate settings (pendant/cap readout + Moai statue editor)
    show_growth_settings(ui, state, &rates);

    // Campaign-bonus inputs (values the export can't provide)
    show_campaign_inputs(ui, state);

    // If a value-dependent sort is active but its value is unset, it would
    // collapse every pet onto the tiebreak — revert to the default sort instead.
    let sort_needs_revert = (state.sort_column == SortColumn::TimeToTarget
        && state.global_growth_target == 0)
        || (state.sort_column == SortColumn::CampaignBonus && state.filter_campaign.is_none());
    if sort_needs_revert {
        state.sort_column = SortColumn::default();
        state.sort_ascending = SortColumn::default().default_ascending();
    }

    ui.add_space(4.0);
    ui.separator();

    // Echo an active non-column sort above the table: its only other cue is a
    // button in a panel/filter row the user may have scrolled past.
    let soonest = if state.sort_ascending { "soonest first" } else { "longest first" };
    let active_sort: Option<(String, &str)> = match state.sort_column {
        SortColumn::TimeToEvolve => Some((
            if state.evolve_sort_use_egg {
                "time to evolve (with egg)".to_string()
            } else {
                "time to evolve (no egg)".to_string()
            },
            soonest,
        )),
        SortColumn::TimeToTarget => Some(("time to custom target".to_string(), soonest)),
        SortColumn::CampaignBonus => state.filter_campaign.map(|c| {
            let dir = if state.sort_ascending { "smallest first" } else { "biggest first" };
            (format!("{} campaign bonus", campaign_label(c)), dir)
        }),
        _ => None,
    };
    if let Some((label, dir)) = active_sort {
        ui.label(
            RichText::new(format!("Sorted by {label} ({dir})"))
                .color(style::ACCENT)
                .size(11.0),
        );
    }

    // Pet table
    let filtered = filter_and_sort(&data.merged, state, &rates, &camp_ctx);
    show_table(ui, &filtered, state);
}

fn show_detail_window(
    ui: &mut Ui,
    state: &mut AnalyzerState,
    data: &DataStore,
    rates: &GrowthRates,
    camp_ctx: &CampaignContext,
) {
    if let Some(pet_name) = state.selected_pet.clone() {
        let pet = data.merged.iter().find(|p| p.name == pet_name);
        let mut open = true;
        let custom_target = &mut state.custom_target;

        egui::Window::new(format!("Pet: {pet_name}"))
            .open(&mut open)
            .collapsible(true)
            .resizable(true)
            .default_size([400.0, 350.0])
            .show(ui.ctx(), |ui| {
                if let Some(pet) = pet {
                    show_pet_details(ui, pet, rates, custom_target, camp_ctx);
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

fn show_pet_details(
    ui: &mut Ui,
    pet: &MergedPet,
    rates: &GrowthRates,
    custom_target: &mut String,
    camp_ctx: &CampaignContext,
) {
    // Wiki data section
    if let Some(wiki) = &pet.wiki {
        // Wiki link
        if !wiki.wiki_url.is_empty()
            && ui
                .link(RichText::new("View on Wiki \u{2192}").color(style::ACCENT).size(12.0))
                .clicked()
        {
            ui.ctx().open_url(egui::OpenUrl::new_tab(&wiki.wiki_url));
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

    // Evolution requirements + readiness
    show_evolution_section(ui, pet, rates);

    // Campaign bonus (raw prose + effective per-campaign chips)
    show_campaign_section(ui, pet, camp_ctx);

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

                // Elemental pets only: the evolved form ("GnomeV2") from the
                // export "Other" column.
                if let Some(form) = pet.elemental_form() {
                    ui.label(RichText::new("Form:").color(style::TEXT_MUTED).size(12.0));
                    ui.label(
                        RichText::new(format!("{}V{}", form.name, form.version))
                            .color(style::TEXT_NORMAL)
                            .size(12.0)
                            .family(egui::FontFamily::Monospace),
                    );
                    ui.end_row();
                }

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

    // Growth-time calculator — works for any pet with export data, evolved or
    // not (e.g. "how long to grow this pet before slotting it into rotation").
    if let Some(export) = &pet.export {
        show_custom_target(ui, export.growth, rates, custom_target);
    }
}

/// A growth-time calculator: enter an arbitrary target base growth and see the
/// estimated time to reach it, with and without a Magic Egg. Like the evolution
/// estimate, "with egg" reaches the target at base = target / 1.3.
fn show_custom_target(ui: &mut Ui, base: u64, rates: &GrowthRates, input: &mut String) {
    ui.add_space(8.0);
    ui.separator();
    ui.label(
        RichText::new("Growth time calculator")
            .color(style::TEXT_BRIGHT)
            .size(13.0)
            .strong(),
    );
    ui.add_space(2.0);

    // Like the Earth Eater field, accept the forms the game itself displays —
    // plain, comma-grouped, or scientific notation (5e6, 1.5e9) — and flag
    // anything unparseable instead of silently mangling it.
    let invalid = {
        let t = input.trim();
        !t.is_empty() && parse_flexible_number(t).is_none()
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new("Target base growth:").color(style::TEXT_MUTED).size(12.0));
        let mut edit = egui::TextEdit::singleline(input)
            .desired_width(110.0)
            .hint_text("e.g. 50000 or 5e6");
        if invalid {
            edit = edit.text_color(style::WARNING);
        }
        ui.add(edit);
        if invalid {
            ui.label(RichText::new("✗ can't parse").color(style::WARNING).size(10.0));
        }
    });

    let Some(target) = parse_growth_target(input) else {
        return;
    };

    let target_egg = (target as f64 / MAGIC_EGG_GROWTH_MULT).ceil() as u64;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Time — no egg:").color(style::TEXT_MUTED).size(11.0));
        eta_label(ui, rates.hours_to_target(base, target));
        ui.label(RichText::new("· with egg:").color(style::TEXT_MUTED).size(11.0));
        eta_label(ui, rates.hours_to_target(base, target_egg));
    });
    ui.label(
        RichText::new("assumes a dedicated pendant + your Moai")
            .color(style::TEXT_MUTED)
            .italics()
            .size(10.0),
    );
    if base < target {
        show_cap_note(ui, rates, base, target);
    }
}

/// Parse the growth-target input: anything `parse_flexible_number` accepts,
/// floored to a whole growth value. `None` for blank, unparseable, or
/// non-positive input; finite values beyond `u64::MAX` saturate (the `as`
/// cast) rather than disappearing.
fn parse_growth_target(input: &str) -> Option<u64> {
    let v = parse_flexible_number(input)?;
    if v < 1.0 {
        return None;
    }
    Some(v as u64)
}

/// The threshold value shown on the "Growth:" row. For total-growth
/// thresholds the egg-assisted target — the base growth at which equipping a
/// Magic Egg clears the bar — is shown alongside, as the number to actually
/// aim for. Base-growth thresholds (Baby Carno) show the bare figure.
fn growth_threshold_text(req: &GrowthRequirement) -> String {
    let threshold = req.value().max(0) as u64;
    if !req.magic_egg_counts() {
        return format_number(threshold);
    }
    let egg_target = (threshold as f64 / MAGIC_EGG_GROWTH_MULT).ceil() as u64;
    format!("{} ({} with Magic Egg)", format_number(threshold), format_number(egg_target))
}

/// The "more growth to threshold" line for a pet that can't evolve yet, given
/// its *base* growth (export growth is always stored as true base). For
/// total-growth thresholds the Magic Egg's +30% lowers the bar, so the smaller
/// egg-assisted remainder is shown alongside; base-growth thresholds (Baby
/// Carno) ignore the egg, so only the base figure appears — labelled as such.
fn growth_needed_text(req: &GrowthRequirement, base_growth: u64) -> String {
    let needed = (req.value() - base_growth as i64).max(0) as u64;
    if !req.magic_egg_counts() {
        return format!("{} more base growth to threshold", format_number(needed));
    }
    // With an egg the threshold is met at base = ceil(threshold / 1.3) — same
    // arithmetic as the ETA estimate below.
    let egg_target = (req.value().max(0) as f64 / MAGIC_EGG_GROWTH_MULT).ceil() as u64;
    let needed_egg = egg_target.saturating_sub(base_growth);
    format!(
        "{} more growth to threshold ({} with Magic Egg)",
        format_number(needed),
        format_number(needed_egg)
    )
}

/// Evolution requirements (growth threshold, material, other) plus a
/// readiness badge and time-to-grow estimate for unevolved pets. No-op for pets
/// without scraped evo data.
fn show_evolution_section(ui: &mut Ui, pet: &MergedPet, rates: &GrowthRates) {
    let Some(req) = pet.wiki.as_ref().and_then(|w| w.evo_requirements.as_ref()) else {
        return;
    };

    ui.add_space(8.0);
    ui.separator();
    ui.label(
        RichText::new("Evolution Requirements")
            .color(style::TEXT_BRIGHT)
            .size(13.0)
            .strong(),
    );
    ui.add_space(2.0);

    egui::Grid::new("pet_evo_grid")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            // Growth threshold. Base-growth thresholds (Baby Carno) are flagged
            // because the Magic Egg can't help reach them.
            let label = if req.growth.requires_base_growth() {
                "Growth (base):"
            } else {
                "Growth:"
            };
            ui.label(RichText::new(label).color(style::TEXT_MUTED).size(12.0));
            ui.label(
                RichText::new(growth_threshold_text(&req.growth))
                    .color(style::TEXT_NORMAL)
                    .size(12.0)
                    .family(egui::FontFamily::Monospace),
            );
            ui.end_row();

            if let Some(material) = &req.material {
                ui.label(RichText::new("Material:").color(style::TEXT_MUTED).size(12.0));
                ui.label(RichText::new(material).color(style::TEXT_NORMAL).size(12.0));
                ui.end_row();
            }
            if let Some(other) = &req.other {
                ui.label(RichText::new("Other:").color(style::TEXT_MUTED).size(12.0));
                ui.label(RichText::new(other).color(style::TEXT_NORMAL).size(12.0));
                ui.end_row();
            }
        });

    // Readiness badge — only present for unlocked, still-unevolved pets.
    if let Some(readiness) = pet.evo_readiness() {
        ui.add_space(2.0);
        match readiness {
            EvoReadiness::Ready => {
                ui.label(
                    RichText::new("✓ Ready to evolve")
                        .color(style::SUCCESS)
                        .size(12.0)
                        .strong(),
                );
            }
            EvoReadiness::ReadyWithEgg => {
                ui.label(
                    RichText::new("Ready to evolve with Magic Egg (+30%)")
                        .color(style::WARNING)
                        .size(12.0)
                        .strong(),
                );
            }
            EvoReadiness::NotYet => {
                if let Some(export) = &pet.export {
                    ui.label(
                        RichText::new(growth_needed_text(&req.growth, export.growth))
                            .color(style::TEXT_MUTED)
                            .size(12.0),
                    );
                }
            }
        }

        // Time-to-grow estimate (pendant + Moai) for pets that can't evolve yet.
        if readiness != EvoReadiness::Ready
            && let Some(export) = &pet.export
        {
            let threshold = req.growth.value().max(0) as u64;
            ui.add_space(2.0);
            if req.growth.requires_base_growth() {
                // Base-growth threshold: the egg never helps, so one estimate.
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Est. time to grow:").color(style::TEXT_MUTED).size(11.0));
                    eta_label(ui, rates.hours_to_target(export.growth, threshold));
                    ui.label(RichText::new("(egg doesn't help)").color(style::TEXT_MUTED).size(10.0));
                });
            } else {
                // Total-growth threshold: with the egg you only need
                // threshold / 1.3 of base growth.
                let target_egg = (threshold as f64 / MAGIC_EGG_GROWTH_MULT).ceil() as u64;
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Est. grow time — no egg:").color(style::TEXT_MUTED).size(11.0));
                    eta_label(ui, rates.hours_to_target(export.growth, threshold));
                    ui.label(RichText::new("· with egg:").color(style::TEXT_MUTED).size(11.0));
                    // Stay consistent with the readiness badge: if the egg
                    // already clears the threshold, it's ready now (avoids a
                    // ~1-unit rounding disagreement with `evo_readiness`).
                    if readiness == EvoReadiness::ReadyWithEgg {
                        eta_label(ui, Some(0.0));
                    } else {
                        eta_label(ui, rates.hours_to_target(export.growth, target_egg));
                    }
                });
            }
            ui.label(
                RichText::new("assumes a dedicated pendant + your Moai")
                    .color(style::TEXT_MUTED)
                    .italics()
                    .size(10.0),
            );
            // Explain a slow estimate when the threshold is past the cap.
            show_cap_note(ui, rates, export.growth, threshold);
        }
    }
}

/// Campaign-bonus card section: the raw prose (if any) plus effective
/// per-campaign chips (highest first). No-op only when the pet has neither.
fn show_campaign_section(ui: &mut Ui, pet: &MergedPet, camp_ctx: &CampaignContext) {
    let raw = pet.wiki.as_ref().and_then(|w| w.campaign_bonus.as_ref()).map(|cb| &cb.raw);
    // Effective per-campaign values (baseline + overrides), split by source.
    let breakdown = pet.campaign_bonus_breakdown(camp_ctx);
    let map = breakdown.total();
    // An override-only pet may have a map without wiki prose, so show the
    // section whenever either side has something.
    if raw.is_none() && map.is_empty() {
        return;
    }
    ui.add_space(8.0);
    ui.separator();
    ui.label(
        RichText::new("Campaign Bonus")
            .color(style::TEXT_BRIGHT)
            .size(13.0)
            .strong(),
    );
    ui.add_space(2.0);
    // The cleaned prose, when available (the display fallback).
    if let Some(raw) = raw {
        ui.label(RichText::new(raw).color(style::TEXT_NORMAL).size(11.0));
    }

    // Per-campaign total chips, highest first.
    if !map.is_empty() {
        let mut entries: Vec<(CampaignType, f32)> = map.into_iter().collect();
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ui.horizontal_wrapped(|ui| {
            for (c, v) in entries {
                let color = if v >= 0.0 { style::SUCCESS } else { style::ERROR };
                ui.label(
                    RichText::new(format!("{}: {v:+}%", campaign_label(c)))
                        .color(color)
                        .size(11.0),
                );
            }
        });
    }

    // Source split — only when a flat layer (class/equipment) is folded into the
    // totals above; with neither, the totals *are* the innate values.
    if breakdown.class.is_some() || breakdown.equipment.is_some() {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("innate:").color(style::TEXT_MUTED).size(10.0));
            if breakdown.innate.is_empty() {
                ui.label(RichText::new("none").color(style::TEXT_MUTED).size(10.0));
            } else {
                let mut entries: Vec<(CampaignType, f32)> =
                    breakdown.innate.iter().map(|(&c, &v)| (c, v)).collect();
                entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                for (c, v) in entries {
                    ui.label(
                        RichText::new(format!("{}: {v:+}%", campaign_label(c)))
                            .color(style::TEXT_MUTED)
                            .size(10.0),
                    );
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            if let Some(v) = breakdown.class {
                ui.label(
                    RichText::new(format!("class: {v:+}%"))
                        .color(style::TEXT_MUTED)
                        .size(10.0),
                );
            }
            if let Some(v) = breakdown.equipment {
                ui.label(
                    RichText::new(format!("equipment: {v:+}%"))
                        .color(style::TEXT_MUTED)
                        .size(10.0),
                );
            }
            ui.label(
                RichText::new("(to every campaign)")
                    .color(style::TEXT_MUTED)
                    .italics()
                    .size(10.0),
            );
        });
    }
}

/// Render an estimated time-to-target as a short label: "ready now" at zero,
/// "~3.4 days" for a finite estimate, or "—" when unreachable with the tools.
fn eta_label(ui: &mut Ui, hours: Option<f64>) {
    match hours {
        Some(h) if h <= 0.0 => {
            ui.label(RichText::new("ready now").color(style::SUCCESS).size(11.0));
        }
        Some(h) => {
            ui.label(
                RichText::new(format!("~{}", format_duration(h)))
                    .color(style::TEXT_NORMAL)
                    .size(11.0),
            );
        }
        None => {
            ui.label(RichText::new("—").color(style::TEXT_MUTED).size(11.0))
                .on_hover_text(
                    "Unreachable with a pendant alone (target above its cap) — add Moai statues or grow another way",
                );
        }
    }
}

/// Explain the pendant cap when a growth target sits above it — that's why an
/// estimate can read in months (growth past the cap is Moai-only since the
/// pendant auto-unequips there). Renders nothing when the climb stays below the
/// cap. Intended for `current < target`.
fn show_cap_note(ui: &mut Ui, rates: &GrowthRates, current: u64, target: u64) {
    let cap = format_number(rates.pendant_cap);
    match rates.cap_relation(current, target) {
        CapRelation::BelowCap => {}
        CapRelation::CrossesCap { hours_to_cap } => {
            let when = match hours_to_cap {
                Some(h) => format!("reached in ~{}", format_duration(h)),
                None => "unreachable".to_string(),
            };
            ui.label(
                RichText::new(format!(
                    "Pendant cap {cap} {when}; growth beyond it is Moai-only (slow)"
                ))
                .color(style::WARNING)
                .size(10.0),
            );
        }
        CapRelation::AboveCap => {
            ui.label(
                RichText::new(format!(
                    "Above the pendant cap ({cap}) — growth is Moai-only (slow)"
                ))
                .color(style::WARNING)
                .size(10.0),
            );
        }
    }
}

/// Growth estimates & sorting: a read-only pendant/cap readout, the Moai statue
/// editor, a persisted global growth target, and the time-based table sorts.
fn show_growth_settings(ui: &mut Ui, state: &mut AnalyzerState, rates: &GrowthRates) {
    egui::CollapsingHeader::new(
        RichText::new("Growth estimates & sorting")
            .color(style::TEXT_BRIGHT)
            .size(13.0),
    )
    .default_open(false)
    .show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Growing Love Pendant:").color(style::TEXT_MUTED).size(12.0));
            ui.label(
                RichText::new(format!("{} base growth/hr", rates.evolved_pets))
                    .color(style::TEXT_NORMAL)
                    .size(12.0),
            )
            .on_hover_text("One per evolved pet, per hour — grows faster as you evolve more pets");
            ui.separator();
            ui.label(RichText::new("Cap:").color(style::TEXT_MUTED).size(12.0));
            let cap_text = if rates.pendant_cap == u64::MAX {
                "—".to_string() // fewer than 10 pets: effectively no cap
            } else {
                format_number(rates.pendant_cap)
            };
            ui.label(
                RichText::new(cap_text)
                    .color(style::TEXT_NORMAL)
                    .size(12.0),
            )
            .on_hover_text("The pendant stops working once a pet's base growth reaches your 10th-highest pet's growth");
        });

        ui.label(
            RichText::new(format!("Moai statues — {:.2} base growth/hr total", rates.moai_per_hour))
                .color(style::TEXT_MUTED)
                .size(12.0),
        );

        // Exactly two statues exist in-game: show both, tick the ones you own
        // and set each level independently.
        for (i, m) in state.moai.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut m.owned,
                    RichText::new(format!("Moai #{}", i + 1)).size(12.0),
                );
                // Level only matters (and is editable) once owned.
                ui.add_enabled(
                    m.owned,
                    egui::DragValue::new(&mut m.level).range(1..=20).prefix("Lv ").clearable(),
                );
                let per_hr = if m.owned { (m.level as f64 * 0.05).min(1.0) } else { 0.0 };
                ui.label(
                    RichText::new(format!("({per_hr:.2}/hr)")).color(style::TEXT_MUTED).size(11.0),
                );
            });
        }

        ui.separator();

        // Persisted global target + the time-based table sorts. These override
        // any active column sort; clicking a column header switches back.
        ui.horizontal(|ui| {
            ui.label(RichText::new("Custom target:").color(style::TEXT_MUTED).size(12.0));
            ui.add(
                egui::DragValue::new(&mut state.global_growth_target)
                    .speed(100.0)
                    .range(0..=1_000_000_000)
                    .clearable(),
            )
            .on_hover_text("Base growth target for the 'time to target' sort (0 = unset)");
        });

        ui.horizontal(|ui| {
            ui.label(RichText::new("Sort table by:").color(style::TEXT_MUTED).size(12.0));
            sort_toggle_button(ui, state, SortColumn::TimeToEvolve, "Time to evolve", true);
            let has_target = state.global_growth_target > 0;
            sort_toggle_button(ui, state, SortColumn::TimeToTarget, "Time to target", has_target);
        });
        ui.horizontal(|ui| {
            ui.checkbox(
                &mut state.evolve_sort_use_egg,
                RichText::new("'Time to evolve' uses egg growth (+30%)").size(11.0),
            )
            .on_hover_text(
                "On: total-growth thresholds are reached at base = threshold / 1.3. \
                 Base-growth pets (e.g. Baby Carno) are unaffected — the egg can't help them.",
            );
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Tiebreaker:").color(style::TEXT_MUTED).size(12.0))
                .on_hover_text("Secondary sort when several pets have the same estimated time");
            ui.selectable_value(
                &mut state.time_sort_tiebreak,
                TimeSortTiebreak::Growth,
                "Growth",
            );
            ui.selectable_value(
                &mut state.time_sort_tiebreak,
                TimeSortTiebreak::EvoDifficulty,
                "Evo difficulty",
            );
        });
        ui.label(
            RichText::new("Time sorts assume a dedicated pendant + your Moai; they respect the filters above")
                .color(style::TEXT_MUTED)
                .italics()
                .size(10.0),
        );
    });
}

/// Editor for the player-entered values that drive formula-based campaign
/// bonuses (Beachball, Unicorn, Bear, Ant Queen, Aether, Earth Eater, …).
fn show_campaign_inputs(ui: &mut Ui, state: &mut AnalyzerState) {
    egui::CollapsingHeader::new(
        RichText::new("Campaign bonus inputs").color(style::TEXT_BRIGHT).size(13.0),
    )
    .default_open(false)
    .show(ui, |ui| {
        ui.label(
            RichText::new("Values the export can't provide, for formula-based bonuses:")
                .color(style::TEXT_MUTED)
                .italics()
                .size(10.0),
        );
        let ci = &mut state.campaign_inputs;
        macro_rules! row {
            ($label:expr, $field:expr, $pet:expr) => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new($label).color(style::TEXT_MUTED).size(12.0));
                    ui.add(egui::DragValue::new($field).speed(1.0).clearable());
                    ui.label(RichText::new($pet).color(style::TEXT_MUTED).size(10.0));
                });
            };
        }
        row!("Pet stones (held):", &mut ci.pet_stones, "⭢ Beachball");
        row!("Stones given to Beachball:", &mut ci.beachball_given_stones, "⭢ Beachball");
        row!("Challenge points:", &mut ci.challenge_points, "⭢ Unicorn");
        row!("Honey given:", &mut ci.honey, "⭢ Bear");
        row!("Ants:", &mut ci.ants, "⭢ Ant Queen");
        row!("Meteor campaign hours:", &mut ci.meteor_campaign_hours, "⭢ Meteor (25 + hrs^0.42)");
        row!("Delirious Essence fights:", &mut ci.delirious_essence_fights, "⭢ Aether");
        row!("UCCs completed:", &mut ci.goblin_ucc, "⭢ Goblin (cap 75)");
        row!("Overflow Challenges:", &mut ci.goblin_oc, "⭢ Goblin evo (cap 470)");
        ui.horizontal(|ui| {
            ui.checkbox(&mut ci.stone_campaign_upgrade, "");
            ui.label(RichText::new("Stone +100% campaign upgrade bought").color(style::TEXT_MUTED).size(12.0));
            ui.label(RichText::new("⭢ Stone/Golem").color(style::TEXT_MUTED).size(10.0));
        });
        // Earth Eater's total accepts engineering/scientific notation (32.4e6),
        // so it's a parsed text field rather than a DragValue. Placed after the
        // `ci` rows so its borrow of `state` doesn't overlap theirs (the lock flag
        // is read/written via `state.campaign_inputs` directly, not the `ci`
        // alias, which is dead by here).
        let ee_invalid = {
            let t = state.earth_eater_planets_text.trim();
            !t.is_empty() && parse_flexible_number(t).is_none()
        };
        let ee_total = parse_flexible_number(&state.earth_eater_planets_text)
            .unwrap_or(0.0)
            .max(0.0);
        // Inverted: stored `show_lifetime` false ⇒ "Lock at +82%" checked.
        let mut ee_lock = !state.campaign_inputs.earth_eater_show_lifetime;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Earth Eater planets (total):").color(style::TEXT_MUTED).size(12.0));
            let mut edit = egui::TextEdit::singleline(&mut state.earth_eater_planets_text)
                .desired_width(80.0)
                .hint_text("e.g. 32.4e6");
            if ee_invalid {
                edit = edit.text_color(style::WARNING);
            }
            ui.add(edit);
            ui.checkbox(&mut ee_lock, RichText::new("Lock at +82%").color(style::TEXT_MUTED).size(12.0));
            if ee_invalid {
                ui.label(RichText::new("✗ can't parse").color(style::WARNING).size(10.0));
            } else if !ee_lock && ee_total > 0.0 && ee_total < 32_400_000.0 {
                // At 1 planet/sec, time left to reach the 32.4M permanent-lock cap.
                let hours = (32_400_000.0 - ee_total) / 3600.0;
                ui.label(
                    RichText::new(format!("~{} to lock @1/s", format_duration(hours)))
                        .color(style::TEXT_MUTED)
                        .size(10.0),
                );
            } else {
                ui.label(RichText::new("⭢ Earth Eater (token-improved)").color(style::TEXT_MUTED).size(10.0));
            }
        });
        state.campaign_inputs.earth_eater_show_lifetime = !ee_lock;
    });
}

/// A selectable button that activates a non-column sort mode, or toggles its
/// direction if it's already active. `enabled` gates the target sort until a
/// target is set. These share `sort_column`, so they replace any column sort.
fn sort_toggle_button(
    ui: &mut Ui,
    state: &mut AnalyzerState,
    col: SortColumn,
    label: &str,
    enabled: bool,
) {
    let active = state.sort_column == col;
    let text = if active {
        format!("{label} {}", if state.sort_ascending { "▲" } else { "▼" })
    } else {
        label.to_string()
    };
    if ui
        .add_enabled(enabled, egui::SelectableLabel::new(active, text))
        .clicked()
    {
        if active {
            state.sort_ascending = !state.sort_ascending;
        } else {
            state.sort_column = col;
            state.sort_ascending = col.default_ascending();
        }
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

        // Unevolved pets that meet (or can reach with the Magic Egg) their
        // evolution growth threshold.
        // Scoped to unlocked pets: this is a roster summary ("N of your pets
        // can evolve now"), unlike the per-pet checks which also flag unowned
        // pets for planning.
        let ready = data
            .merged
            .iter()
            .filter(|p| p.is_unlocked() && p.evo_readiness() == Some(EvoReadiness::Ready))
            .count();
        let ready_egg = data
            .merged
            .iter()
            .filter(|p| p.is_unlocked() && p.evo_readiness() == Some(EvoReadiness::ReadyWithEgg))
            .count();
        if ready + ready_egg > 0 {
            ui.separator();
            let text = if ready_egg > 0 {
                format!("Ready to evolve: {ready} (+{ready_egg} w/ egg)")
            } else {
                format!("Ready to evolve: {ready}")
            };
            ui.label(RichText::new(text).color(style::SUCCESS).size(13.0))
                .on_hover_text("Unevolved pets whose growth meets the evolution threshold (the '+N w/ egg' reach it only with a Magic Egg equipped)");
        }

        // Total growth of unlocked pets
        let total_growth: u64 = data
            .merged
            .iter()
            .filter_map(|p| p.export.as_ref())
            .filter(|e| e.unlocked)
            .map(|e| e.effective_growth())
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

    // Row 3: campaign-bonus filter + sort
    ui.horizontal(|ui| {
        ui.label(RichText::new("Campaign boost:").color(style::TEXT_MUTED));
        egui::ComboBox::from_id_salt("campaign_filter")
            .selected_text(match state.filter_campaign {
                None => "Any",
                Some(c) => campaign_label(c),
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.filter_campaign, None, "Any");
                for c in ALL_CAMPAIGNS {
                    ui.selectable_value(&mut state.filter_campaign, Some(c), campaign_label(c));
                }
            });
        // Rank the (filtered) pets by that campaign's bonus, once one is chosen.
        let has_campaign = state.filter_campaign.is_some();
        sort_toggle_button(ui, state, SortColumn::CampaignBonus, "Sort by bonus", has_campaign);
        ui.separator();
        ui.checkbox(
            &mut state.include_equipment_bonus,
            RichText::new("+ equipment").size(12.0),
        )
        .on_hover_text(
            "Add each pet's campaign-boost gear (sticks; the event items at SSS+20) \
             to its effective bonus. Off by default — innate bonuses are more \
             durable to plan around.",
        );
        ui.checkbox(
            &mut state.include_class_bonus,
            RichText::new("+ class").size(12.0),
        )
        .on_hover_text(
            "Add an Adventurer pet's class campaign bonus (2% × class level) to \
             its effective bonus. Per-pet Adventurer evo bonuses come later.",
        );
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
        .column(Column::initial(130.0).at_least(85.0))   // Growth
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

                // Growth (+ evolution-readiness marker)
                row.col(|ui| {
                    if let Some(export) = &pet.export {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            if export.has_magic_egg() {
                                ui.label(
                                    RichText::new(format_number(export.growth))
                                        .color(style::TEXT_MUTED)
                                        .size(12.0)
                                        .family(egui::FontFamily::Monospace),
                                );
                                ui.label(
                                    RichText::new(format!("({})", format_number(export.effective_growth())))
                                        .color(style::SUCCESS)
                                        .size(12.0)
                                        .family(egui::FontFamily::Monospace),
                                );
                            } else {
                                ui.label(
                                    RichText::new(format_number(export.growth))
                                        .color(text_color)
                                        .size(12.0)
                                        .family(egui::FontFamily::Monospace),
                                );
                            }
                            // Readiness check-mark: green = ready now, amber =
                            // only with a Magic Egg. Nothing when not ready.
                            match pet.evo_readiness() {
                                Some(EvoReadiness::Ready) => {
                                    ui.label(RichText::new("✓").color(style::SUCCESS).size(12.0))
                                        .on_hover_text("Meets the evolution growth threshold — ready to evolve");
                                }
                                Some(EvoReadiness::ReadyWithEgg) => {
                                    ui.label(RichText::new("✓").color(style::WARNING).size(12.0))
                                        .on_hover_text("Reaches the evolution threshold with a Magic Egg equipped (+30% growth)");
                                }
                                _ => {}
                            }
                        });
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
                    if let Some(export) = &pet.export
                        && export.class.is_some() {
                            ui.label(
                                RichText::new(export.class_level.to_string())
                                    .color(text_color)
                                    .size(12.0)
                                    .family(egui::FontFamily::Monospace),
                            );
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
                    } else if let Some(wiki) = &pet.wiki
                        && let Some(special) = &wiki.special_ability {
                            ui.label(
                                RichText::new(special)
                                    .color(style::TEXT_MUTED)
                                    .italics()
                                    .size(11.0),
                            );
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

fn filter_and_sort<'a>(
    pets: &'a [MergedPet],
    state: &AnalyzerState,
    rates: &GrowthRates,
    camp_ctx: &CampaignContext,
) -> Vec<&'a MergedPet> {
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
            if let Some(ref filter_el) = state.filter_element
                && pet.element().as_ref() != Some(filter_el) {
                    return false;
                }

            // Unlocked
            if let Some(filter_unlock) = state.filter_unlocked
                && pet.is_unlocked() != filter_unlock {
                    return false;
                }

            // Evolved
            if let Some(filter_evo) = state.filter_evolved
                && pet.is_evolved() != filter_evo {
                    return false;
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
            if state.filter_my_class != MyClassFilter::All
                && !state.filter_my_class.matches(pet.evolved_class()) {
                    return false;
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

            // Campaign boost: keep only pets with a positive parsed bonus to the
            // selected campaign. (Raw-only/unparsed pets have no entry, so they
            // sit out this filter until later phases structure them.)
            if let Some(c) = state.filter_campaign
                && !pet.campaign_bonus_for(c, camp_ctx).is_some_and(|v| v > 0.0)
            {
                return false;
            }

            true
        })
        .collect();

    // Sort — growth descending is the universal tiebreaker (strongest first in ties)
    let asc = state.sort_ascending;
    filtered.sort_by(|a, b| {
        let ga = a.export.as_ref().map(|e| e.effective_growth()).unwrap_or(0);
        let gb = b.export.as_ref().map(|e| e.effective_growth()).unwrap_or(0);

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
            // Time sorts: soonest first; not-applicable/unreachable pets (∞)
            // fall to the end. Ties (e.g. several already-met pets) break by the
            // user-chosen secondary key, then name for stability.
            SortColumn::TimeToEvolve => {
                let egg = state.evolve_sort_use_egg;
                let ta = a.hours_to_evolve(rates, egg).unwrap_or(f64::INFINITY);
                let tb = b.hours_to_evolve(rates, egg).unwrap_or(f64::INFINITY);
                ta.partial_cmp(&tb)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| time_tiebreak(a, b, ga, gb, state.time_sort_tiebreak))
            }
            SortColumn::TimeToTarget => {
                let target = state.global_growth_target;
                let ta = a.hours_to_growth(target, rates).unwrap_or(f64::INFINITY);
                let tb = b.hours_to_growth(target, rates).unwrap_or(f64::INFINITY);
                ta.partial_cmp(&tb)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| time_tiebreak(a, b, ga, gb, state.time_sort_tiebreak))
            }
            SortColumn::CampaignBonus => {
                // Ascending here (the `asc` flag below reverses to biggest-first
                // by default). Pets without a known bonus to the campaign sink.
                let c = state.filter_campaign;
                let va = c.and_then(|c| a.campaign_bonus_for(c, camp_ctx)).unwrap_or(f32::NEG_INFINITY);
                let vb = c.and_then(|c| b.campaign_bonus_for(c, camp_ctx)).unwrap_or(f32::NEG_INFINITY);
                va.partial_cmp(&vb)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| ga.cmp(&gb))
                    .then_with(|| a.name.cmp(&b.name))
            }
        };
        if asc { ord } else { ord.reverse() }
    });

    filtered
}

/// Secondary ordering for the time sorts when estimates tie (e.g. several pets
/// already meet the target). `ga`/`gb` are the pets' effective growths.
fn time_tiebreak(
    a: &MergedPet,
    b: &MergedPet,
    ga: u64,
    gb: u64,
    tiebreak: TimeSortTiebreak,
) -> std::cmp::Ordering {
    match tiebreak {
        // Higher effective growth first (matches the other columns' tiebreaker).
        TimeSortTiebreak::Growth => gb.cmp(&ga).then_with(|| a.name.cmp(&b.name)),
        // Easiest evo difficulty first, then higher growth, then name.
        TimeSortTiebreak::EvoDifficulty => {
            let key = |p: &MergedPet| {
                p.wiki
                    .as_ref()
                    .map(|w| (w.evo_difficulty.base, w.evo_difficulty.with_conditions))
                    .unwrap_or((99, 99))
            };
            key(a)
                .cmp(&key(b))
                .then_with(|| gb.cmp(&ga))
                .then_with(|| a.name.cmp(&b.name))
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_main_stats_fills_present_fields_only() {
        let mut st = AnalyzerState::default();
        st.campaign_inputs.pet_stones = 999; // should be overwritten
        let ms = MainStats {
            pet_stones: Some(250_882),
            ants: Some(187_331),
            honey_consumed_by_bear: Some(5),
            challenge_points: Some(721),
            goblin_ucc: Some(3),
            goblin_oc: Some(4),
            stone_campaign_upgrade: Some(true),
            earth_eater_planets_text: Some("7.142 E+6".to_string()),
            base_growth_per_hour: Some(2),
            ..Default::default()
        };
        let applied = st.apply_main_stats(&ms);
        assert_eq!(applied.len(), 9);
        assert_eq!(st.campaign_inputs.pet_stones, 250_882);
        assert_eq!(st.campaign_inputs.ants, 187_331);
        assert_eq!(st.campaign_inputs.honey, 5);
        assert_eq!(st.campaign_inputs.challenge_points, 721);
        assert_eq!(st.campaign_inputs.goblin_ucc, 3);
        assert_eq!(st.campaign_inputs.goblin_oc, 4);
        assert!(st.campaign_inputs.stone_campaign_upgrade);
        assert_eq!(st.earth_eater_planets_text, "7.142 E+6");
        assert!(st.moai.iter().all(|m| m.owned && m.level == 20));
    }

    #[test]
    fn apply_main_stats_leaves_absent_fields_untouched() {
        let mut st = AnalyzerState::default();
        st.campaign_inputs.ants = 42;
        let applied = st.apply_main_stats(&MainStats::default());
        assert!(applied.is_empty());
        assert_eq!(st.campaign_inputs.ants, 42); // not clobbered
        assert!(st.moai.iter().all(|m| !m.owned)); // default Moai untouched
    }

    #[test]
    fn growth_target_accepts_the_forms_the_game_shows() {
        assert_eq!(parse_growth_target("50000"), Some(50_000));
        assert_eq!(parse_growth_target("50,000"), Some(50_000));
        assert_eq!(parse_growth_target("5e6"), Some(5_000_000));
        assert_eq!(parse_growth_target("1.5e9"), Some(1_500_000_000));
        assert_eq!(parse_growth_target("3.664 E+9"), Some(3_664_000_000));
        // Blank / junk / non-positive → no target, calculator hidden.
        assert_eq!(parse_growth_target(""), None);
        assert_eq!(parse_growth_target("abc"), None);
        assert_eq!(parse_growth_target("0"), None);
        assert_eq!(parse_growth_target("-5"), None);
        // Finite but beyond u64::MAX saturates instead of vanishing.
        assert_eq!(parse_growth_target("1e30"), Some(u64::MAX));
    }

    #[test]
    fn moai_only_inferred_when_base_growth_is_exactly_two() {
        let mut st = AnalyzerState::default();
        st.apply_main_stats(&MainStats { base_growth_per_hour: Some(1), ..Default::default() });
        assert!(st.moai.iter().all(|m| !m.owned)); // 1 ≠ 2 → leave Moai alone
        st.apply_main_stats(&MainStats { base_growth_per_hour: Some(2), ..Default::default() });
        assert!(st.moai.iter().all(|m| m.owned && m.level == 20));
    }

    #[test]
    fn growth_needed_shows_egg_assisted_remainder_for_total_thresholds() {
        // Egg target is ceil(13_000 / 1.3) = 10_000.
        assert_eq!(
            growth_needed_text(&GrowthRequirement::Total(13_000), 2_889),
            "10,111 more growth to threshold (7,111 with Magic Egg)"
        );
        // Already past the egg-assisted bar (rounding edge): clamps to 0
        // instead of going negative.
        assert_eq!(
            growth_needed_text(&GrowthRequirement::Total(13_000), 11_000),
            "2,000 more growth to threshold (0 with Magic Egg)"
        );
    }

    #[test]
    fn growth_threshold_shows_egg_target_only_when_the_egg_counts() {
        // ceil(13_000 / 1.3) = 10_000 — the base growth to aim for.
        assert_eq!(
            growth_threshold_text(&GrowthRequirement::Total(13_000)),
            "13,000 (10,000 with Magic Egg)"
        );
        // Base-growth thresholds (Baby Carno): no egg target.
        assert_eq!(growth_threshold_text(&GrowthRequirement::Base(300_000)), "300,000");
    }

    #[test]
    fn growth_needed_omits_egg_for_base_thresholds() {
        // Baby Carno's threshold is checked against base growth, so the egg
        // figure must not appear.
        assert_eq!(
            growth_needed_text(&GrowthRequirement::Base(300_000), 100_000),
            "200,000 more base growth to threshold"
        );
    }
}
