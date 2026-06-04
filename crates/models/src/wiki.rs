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

    /// Obtainable via Milestones or a Pet Token.
    MilestonesOrPetToken,

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
}

/// Evolution difficulty rating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvoDifficulty {
    /// Base difficulty from growth requirement alone (1-8).
    pub base: u8,
    /// Difficulty accounting for additional conditions (base + extras).
    pub with_conditions: u8,
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

/// A pet entry as described by the wiki. Static reference data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPet {
    pub name: String,
    pub wiki_url: String,
    pub element: Element,
    pub recommended_class: RecommendedClass,
    pub class_bonus: String,
    pub unlock_condition: UnlockCondition,
    pub evo_difficulty: EvoDifficulty,
    pub token_improvable: bool,
    pub special_ability: Option<String>,

    /// Per-pet evolution requirements (growth threshold, material, other),
    /// scraped from the pet's wiki page infobox. `None` until the crawl
    /// populates it, so existing data without this field still deserializes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evo_requirements: Option<EvoRequirements>,
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
    }
}
