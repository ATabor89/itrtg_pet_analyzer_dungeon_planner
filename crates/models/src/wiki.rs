use serde::{Deserialize, Serialize};

use crate::{Class, Element};

/// How the wiki recommends evolving a pet's class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecommendedClass {
    /// A single best class.
    Single(Class),

    /// Two viable classes (e.g. Assassin/Adventurer). First is typically primary.
    Dual(Class, Class),

    /// No meaningful class bonus — evolve as whatever you need.
    Wildcard,

    /// Any dungeon class is viable (Chameleon).
    DungeonWildcard,

    /// Village pet — class doesn't matter much. The role string describes the
    /// village building (e.g. "Fisher", "Fish Seller", "Dojo", "Tavern", "Alchemy Hut").
    Village(String),

    /// Can freely switch between all classes at no cost (Holy ITRTG Book, Nothing, Nugget).
    AllClasses,

    /// Gray — unique mechanic with children.
    Special,

    /// Feather Pile/Owl — alternates between forms.
    Alternates,
}

impl RecommendedClass {
    /// Extract the primary (first) concrete class, if any.
    pub fn primary_class(&self) -> Option<Class> {
        match self {
            RecommendedClass::Single(c) => Some(*c),
            RecommendedClass::Dual(a, _) => Some(*a),
            _ => None,
        }
    }
}

/// How a pet is unlocked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnlockCondition {
    /// Defeat the base gods (earliest pets).
    DefeatGods,

    /// Defeat a specific Planet Baal level (e.g. 5, 10, 15...).
    DefeatPBaal(u32),

    /// Defeat a specific Planet Baal *version* level (e.g. v125, v150...).
    DefeatPBaalVersion(u32),

    /// Complete a special in-game task.
    SpecialTask,

    /// Purchase with a Pet Token.
    PetToken,

    /// Obtainable via Milestones only.
    Milestones,

    /// Secret unlock condition.
    Secret,

    /// Special unlock (Four Sacred Beasts).
    Special,

    /// Tavern quest of a specific rank (e.g. "SSS").
    TavernQuest(String),

    /// Strategy Room at a certain level.
    StrategyRoom(u32),

    /// Collect a certain number of ancient mimic points.
    AncientMimicPoints(u32),

    /// Have a certain number of pets unlocked.
    PetCount(u32),

    /// Defeat a specific dungeon boss (e.g. "D3-0").
    DungeonBoss(String),

    /// Give the pet a specific item/resource (e.g. "1000 Honey").
    ItemGift(String),

    /// Redeem an Old Event Token (event pets from past events).
    OldEventToken,

    /// Any one of several methods unlocks the pet, e.g. "Old Event Token or
    /// Pet Token" or "Milestones or Pet Token". Never nested and always has
    /// at least two entries; see [`UnlockCondition::methods`].
    AnyOf(Vec<UnlockCondition>),
}

impl UnlockCondition {
    /// Every way this pet can be unlocked: the alternatives of an
    /// [`AnyOf`](Self::AnyOf), or just this condition.
    pub fn methods(&self) -> Vec<&UnlockCondition> {
        match self {
            Self::AnyOf(all) => all.iter().flat_map(|c| c.methods()).collect(),
            other => vec![other],
        }
    }

    /// Whether any unlock method satisfies `pred`.
    pub fn any_method(&self, pred: impl Fn(&UnlockCondition) -> bool) -> bool {
        self.methods().into_iter().any(pred)
    }

    /// The pet can be unlocked with a Pet Token.
    pub fn allows_pet_token(&self) -> bool {
        self.any_method(|c| matches!(c, Self::PetToken))
    }

    /// A Pet Token is the *only* way to unlock the pet (worth prioritizing a
    /// token on, since nothing else will get it).
    pub fn pet_token_only(&self) -> bool {
        self.methods().iter().all(|c| matches!(c, Self::PetToken))
    }
}

/// Evolution difficulty rating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvoDifficulty {
    /// Base difficulty from growth requirement alone (1-8).
    pub base: u8,
    /// Difficulty accounting for additional conditions (base + extras).
    pub with_conditions: u8,
}

impl EvoDifficulty {
    /// Sort key for "easiest first": an unrated pet (`None`) sorts last, the
    /// same as a pet with no wiki entry at all.
    pub fn sort_key(d: Option<&EvoDifficulty>) -> (u8, u8) {
        d.map_or((99, 99), |d| (d.base, d.with_conditions))
    }

    /// Display text, e.g. "3 (5)", or "? (?)" when unrated.
    pub fn display(d: Option<&EvoDifficulty>) -> String {
        d.map_or_else(|| "? (?)".to_string(), |d| format!("{} ({})", d.base, d.with_conditions))
    }
}

/// A pet's growth threshold to evolve, tagged with *which* growth value the
/// game checks it against.
///
/// Most pets check **total** growth, so a Magic Egg's +30% (which boosts total
/// but not base growth) counts toward reaching the threshold. A few — currently
/// just Baby Carno — require **base** growth, where the Magic Egg does not help.
/// Keeping the basis in the type forces every consumer to decide how the egg
/// applies instead of silently assuming total growth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrowthRequirement {
    /// Checked against total/effective growth — a Magic Egg's boost counts.
    Total(i64),
    /// Checked against base growth only — a Magic Egg's boost does *not* count.
    Base(i64),
}

impl GrowthRequirement {
    /// The numeric threshold, regardless of basis.
    pub fn value(&self) -> i64 {
        match self {
            Self::Total(v) | Self::Base(v) => *v,
        }
    }

    /// Whether the threshold is checked against base growth only (Baby Carno).
    pub fn requires_base_growth(&self) -> bool {
        matches!(self, Self::Base(_))
    }

    /// Whether a Magic Egg's growth boost counts toward reaching this
    /// threshold. True for total-growth requirements, false for base-growth.
    pub fn magic_egg_counts(&self) -> bool {
        matches!(self, Self::Total(_))
    }
}

/// The three evolution requirements shown in a pet page's infobox, under the
/// "Evolution Requirements" heading. Scraped per-pet (the main Pets table does
/// not carry these). Optional because the crawl can miss pets or a pet may have
/// no evolution.
///
/// Note: the infobox labels the growth threshold "Total Growth", which is a
/// different value from the pet's own `total_growth` starting stat. Here
/// [`Self::growth`] is the *threshold* (raw infobox param `evo_growth`), and its
/// [`GrowthRequirement`] variant records whether it is a base- or total-growth
/// requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvoRequirements {
    /// Growth threshold the pet must reach to evolve (e.g. Mouse `Total(100)`,
    /// Sylph `Total(55555)`, Baby Carno `Base(300000)`). The load-bearing field
    /// for evolution planning.
    pub growth: GrowthRequirement,

    /// Material(s) needed, as displayed (e.g. "5 Wood", "2778 Bound Feather").
    /// Template-computed in the infobox, so only available from the rendered
    /// page. Free text for display only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<String>,

    /// The variable third condition (infobox "Other" row, raw param
    /// `evo_special`), e.g. "100 Puny Food" or "Finish the Questline...".
    /// Free text for display only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other: Option<String>,
}

/// A pet's campaign-reward bonus prose, from the infobox "Campaign Bonus" row.
///
/// `raw` is the cleaned prose, kept as the display text for pets whose bonus
/// has no structured value. The structured percentages are *not* parsed from
/// it — they're hand-curated in `data/campaign_bonuses.yaml` (see
/// [`crate::CampaignBonusRules`]) or computed by per-pet formulas in the
/// planner. Pets with no bonus at all (`None`/empty on the wiki) carry no
/// `CampaignBonus`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampaignBonus {
    pub raw: String,
}

/// A pet entry as described by the wiki. Static reference data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPet {
    pub name: String,
    pub wiki_url: String,
    pub element: Element,
    pub recommended_class: RecommendedClass,
    pub class_bonus: String,
    pub unlock_condition: UnlockCondition,
    /// `None` when the wiki doesn't rate it yet (a brand-new pet shows `?(?)`).
    #[serde(default)]
    pub evo_difficulty: Option<EvoDifficulty>,
    pub token_improvable: bool,
    pub special_ability: Option<String>,

    /// Per-pet evolution requirements (growth threshold, material, other),
    /// scraped from the pet's wiki page infobox. `None` until the crawl
    /// populates it, so existing data without this field still deserializes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evo_requirements: Option<EvoRequirements>,

    /// Per-pet campaign-reward bonus prose, scraped from the infobox. `None`
    /// means no bonus (or not yet crawled); `Some` carries the raw display
    /// text. Structured values live in `data/campaign_bonuses.yaml`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign_bonus: Option<CampaignBonus>,
}

/// Something about a pet's wiki data that the extractor couldn't take at face
/// value: a placeholder (`?`, "TBD", ...), a value it didn't recognize, or a
/// page it couldn't read. The extractor never fails on these; it falls back to
/// a default and records one of these so the refresh job can raise an alert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiDataWarning {
    pub pet: String,
    /// Human-readable field name, e.g. "evo difficulty" or "campaign bonus".
    pub field: String,
    /// What was wrong and what was used instead.
    pub detail: String,
}

/// Words that, on their own or inside a value, mean "nobody knows yet".
const PLACEHOLDER_WORDS: &[&str] = &[
    "unknown",
    "tbd",
    "tba",
    "to be determined",
    "to be announced",
    "not yet known",
    "not known yet",
];

/// Whether a scraped wiki value is (or contains) an "unknown yet" placeholder.
///
/// The wiki is user-edited, so a brand-new pet's unknowns show up in whatever
/// form the editor picked: `?`, `???`, `?(?)`, `+?% growth`, "TBD", "Unknown",
/// "(exactly how much is to be determined)". This is deliberately broad: any
/// question mark, or any of a few placeholder words as a whole word. Empty
/// values and "None"/"-" are *not* placeholders; they mean "no value".
pub fn is_placeholder(value: &str) -> bool {
    if value.contains('?') {
        return true;
    }
    let lower = value.to_ascii_lowercase();
    PLACEHOLDER_WORDS.iter().any(|w| contains_word(&lower, w))
}

/// `needle` occurs in `hay` with no letter/digit directly on either side.
fn contains_word(hay: &str, needle: &str) -> bool {
    let mut start = 0;
    while let Some(pos) = hay[start..].find(needle) {
        let begin = start + pos;
        let end = begin + needle.len();
        let before_ok = hay[..begin].chars().next_back().is_none_or(|c| !c.is_alphanumeric());
        let after_ok = hay[end..].chars().next().is_none_or(|c| !c.is_alphanumeric());
        if before_ok && after_ok {
            return true;
        }
        start = begin + 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bundled `wiki_pets.yaml` must deserialize into `WikiPet`s — this
    /// guards the `evo_requirements` / `GrowthRequirement` serde format the GUI
    /// relies on at runtime, and confirms the base/total tagging round-trips.
    #[test]
    fn real_wiki_pets_yaml_deserializes_with_evo() {
        let yaml = include_str!("../../../data/wiki_pets.yaml");
        let pets: Vec<WikiPet> =
            serde_yaml::from_str(yaml).expect("wiki_pets.yaml should deserialize");
        assert!(!pets.is_empty());

        let find = |name: &str| {
            pets.iter()
                .find(|p| p.name == name)
                .unwrap_or_else(|| panic!("{name} missing from wiki_pets.yaml"))
        };

        // A normal total-growth pet: the Magic Egg counts toward its threshold.
        let mouse = find("Mouse").evo_requirements.as_ref().expect("Mouse evo");
        assert_eq!(mouse.growth, GrowthRequirement::Total(100));
        assert!(mouse.growth.magic_egg_counts());

        // Baby Carno requires base growth: the Magic Egg does not help.
        let carno = find("Baby Carno").evo_requirements.as_ref().expect("Carno evo");
        assert_eq!(carno.growth, GrowthRequirement::Base(300000));
        assert!(carno.growth.requires_base_growth());
        assert!(!carno.growth.magic_egg_counts());

        // Campaign-bonus prose round-trips for display.
        let cat = find("Cat").campaign_bonus.as_ref().expect("Cat campaign");
        assert!(!cat.raw.is_empty());
    }

    #[test]
    fn unlock_methods() {
        let either = UnlockCondition::AnyOf(vec![UnlockCondition::OldEventToken, UnlockCondition::PetToken]);
        assert_eq!(either.methods(), [&UnlockCondition::OldEventToken, &UnlockCondition::PetToken]);
        assert!(either.allows_pet_token());
        assert!(!either.pet_token_only());
        assert!(UnlockCondition::PetToken.pet_token_only());
        assert!(!UnlockCondition::SpecialTask.allows_pet_token());
        assert!(!UnlockCondition::SpecialTask.pet_token_only());
    }

    #[test]
    fn placeholder_detection() {
        for v in ["?", "???", "?(?)", "+?% to Growth", "TBD", "Unknown", "tba",
                  "costs more (exactly how much is to be determined)"] {
            assert!(is_placeholder(v), "{v:?} should be a placeholder");
        }
        for v in ["", "None", "-", "+77% to All Campaigns", "2.1% x CL",
                  "35600 Magic Ore", "Unknownable", "stbd"] {
            assert!(!is_placeholder(v), "{v:?} should not be a placeholder");
        }
    }
}
