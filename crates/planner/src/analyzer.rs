//! Shared analyzer state, filtering, sorting and display helpers.
//! Extracted from the egui analyzer so native and WASM frontends use the same rules.
use itrtg_models::{base_growth_for_displayed_target, parse_flexible_number,
    pgc_growth_mult, CampaignInputs, CampaignType, Class, Dungeon, Element, GrowthRequirement,
    MainStats, MAGIC_EGG_GROWTH_MULT, PetAction, RecommendedClass, UnlockCondition, VillageJob};
use crate::growth::GrowthRates;
use crate::merge::{CampaignContext, MergedPet};
use serde::{Deserialize, Serialize};

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
    pub fn label(self) -> &'static str {
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

    pub fn matches(self, cond: &UnlockCondition) -> bool {
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
    pub fn label(self) -> &'static str {
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

    pub fn matches(self, rec: &RecommendedClass) -> bool {
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
    pub fn label(self) -> &'static str {
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

    pub fn matches(self, class: Option<Class>) -> bool {
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
    pub fn label(self) -> &'static str {
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
    /// PGC completion counts used by displayed growth, evolution readiness, and
    /// growth-target estimates. Auto-filled from Main stats and persisted so a
    /// pet-export-only refresh keeps the account-wide multiplier.
    pub pgc_done: u32,
    pub pgc_max: u32,
    /// Global displayed-growth target used for the "time to target" table
    /// sort. Persisted; `0` means unset. Before analyzer PGC support this was
    /// described as base growth; existing numeric values are intentionally
    /// retained and reinterpreted as displayed targets once PGC is non-1.
    /// Distinct from the pet card's ephemeral `custom_target` scratch input.
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
            pgc_done: 0,
            pgc_max: 25,
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
    /// Auto-fill campaign inputs, Moai statues, and PGC from a parsed Main-stats
    /// export. Returns short labels for the fields that were filled (for a
    /// status message). Only values present in the export are applied; the rest
    /// are left untouched, so importing never clears a field the export didn't
    /// carry.
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
        if let Some((done, max)) = ms.patreon_god_challenges {
            self.pgc_max = max;
            self.pgc_done = done.min(max);
            applied.push("PGC");
        }
        applied
    }

    pub fn pgc_mult(&self) -> f64 {
        pgc_growth_mult(self.pgc_done, self.pgc_max)
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
    pub fn default_ascending(self) -> bool {
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
pub fn campaign_label(c: CampaignType) -> &'static str {
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

pub const ALL_CAMPAIGNS: [CampaignType; 7] = [
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

pub fn parse_growth_target(input: &str) -> Option<u64> {
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
pub fn growth_threshold_text(req: &GrowthRequirement, growth_mult: f64) -> String {
    let threshold = req.value().max(0) as u64;
    if !req.magic_egg_counts() {
        return format_number(threshold);
    }
    let no_egg_target = base_growth_for_displayed_target(threshold, growth_mult);
    let egg_target = base_growth_for_displayed_target(
        threshold,
        MAGIC_EGG_GROWTH_MULT * growth_mult,
    );
    if growth_mult > 1.0 {
        format!(
            "{} ({} with PGC; {} with PGC + Magic Egg)",
            format_number(threshold),
            format_number(no_egg_target),
            format_number(egg_target)
        )
    } else {
        format!("{} ({} with Magic Egg)", format_number(threshold), format_number(egg_target))
    }
}

/// The "more growth to threshold" line for a pet that can't evolve yet, given
/// its *base* growth (export growth is always stored as true base). For
/// total-growth thresholds the Magic Egg's +30% lowers the bar, so the smaller
/// egg-assisted remainder is shown alongside; base-growth thresholds (Baby
/// Carno) ignore the egg, so only the base figure appears — labelled as such.
pub fn growth_needed_text(req: &GrowthRequirement, base_growth: u64, growth_mult: f64) -> String {
    let needed = (req.value() - base_growth as i64).max(0) as u64;
    if !req.magic_egg_counts() {
        return format!("{} more base growth to threshold", format_number(needed));
    }
    // Convert the displayed threshold back to the base-growth accumulator,
    // with PGC alone and with PGC + egg.
    let threshold = req.value().max(0) as u64;
    let no_egg_target = base_growth_for_displayed_target(threshold, growth_mult);
    let egg_target = base_growth_for_displayed_target(
        threshold,
        MAGIC_EGG_GROWTH_MULT * growth_mult,
    );
    let needed_pgc = no_egg_target.saturating_sub(base_growth);
    let needed_egg = egg_target.saturating_sub(base_growth);
    if growth_mult > 1.0 {
        format!(
            "{} more base growth ({} with PGC; {} with PGC + Magic Egg)",
            format_number(needed),
            format_number(needed_pgc),
            format_number(needed_egg)
        )
    } else {
        format!(
            "{} more growth to threshold ({} with Magic Egg)",
            format_number(needed),
            format_number(needed_egg)
        )
    }
}

/// Evolution requirements (growth threshold, material, other) plus a
/// readiness badge and time-to-grow estimate for unevolved pets. No-op for pets
/// without scraped evo data.
pub fn format_signed(n: i64) -> String {
    if n < 0 {
        format!("-{}", format_number(n.unsigned_abs()))
    } else {
        format_number(n as u64)
    }
}

/// Campaign-bonus card section: the raw prose (if any) plus effective
/// per-campaign chips (highest first). No-op only when the pet has neither.
pub fn format_action(action: &PetAction) -> String {
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

/// Sort key for actions: groups by type, then by sub-variant.
pub fn action_sort_key(action: &PetAction) -> u16 {
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

pub fn format_unlock_condition(cond: &UnlockCondition) -> String {
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

pub fn filter_and_sort<'a>(
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
                let name_match = pet.name.to_lowercase().contains(&search_lower)
                    || pet.export.as_ref().is_some_and(|e| e.export_name.to_lowercase().contains(&search_lower));
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
        let growth_mult = state.pgc_mult();
        let ga = a
            .export
            .as_ref()
            .map(|e| e.effective_growth_with_global_mult(growth_mult))
            .unwrap_or(0);
        let gb = b
            .export
            .as_ref()
            .map(|e| e.effective_growth_with_global_mult(growth_mult))
            .unwrap_or(0);

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
                let ta = a
                    .hours_to_evolve_with_growth_mult(rates, egg, growth_mult)
                    .unwrap_or(f64::INFINITY);
                let tb = b
                    .hours_to_evolve_with_growth_mult(rates, egg, growth_mult)
                    .unwrap_or(f64::INFINITY);
                ta.partial_cmp(&tb)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| time_tiebreak(a, b, ga, gb, state.time_sort_tiebreak))
            }
            SortColumn::TimeToTarget => {
                let target = state.global_growth_target;
                let ta = a
                    .hours_to_growth_with_mult(target, rates, growth_mult)
                    .unwrap_or(f64::INFINITY);
                let tb = b
                    .hours_to_growth_with_mult(target, rates, growth_mult)
                    .unwrap_or(f64::INFINITY);
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
pub fn time_tiebreak(
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

pub fn rec_class_sort_key(rec: Option<&RecommendedClass>) -> u8 {
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

pub fn class_order(c: &Class) -> u8 {
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

pub fn format_number(n: u64) -> String {
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

