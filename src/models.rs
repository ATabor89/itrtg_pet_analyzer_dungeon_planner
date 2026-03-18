use serde::Serialize;

/// The primary element of a pet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Element {
    Neutral,
    Fire,
    Water,
    Wind,
    Earth,
    /// Chameleon can freely change its element.
    All,
}

/// A single pet class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Class {
    Adventurer,
    Blacksmith,
    Alchemist,
    Defender,
    Supporter,
    Rogue,
    Assassin,
    Mage,
    /// Not a real class — used in Dual recommendations to indicate the pet can
    /// be evolved as whatever the player needs.
    Wildcard,
}

/// How the wiki recommends evolving a pet's class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

/// How a pet is unlocked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvoDifficulty {
    /// Base difficulty from growth requirement alone (1-8).
    pub base: u8,
    /// Difficulty accounting for additional conditions (base + extras).
    pub with_conditions: u8,
}

/// A fully parsed pet entry.
#[derive(Debug, Clone, Serialize)]
pub struct Pet {
    pub name: String,
    pub wiki_url: String,
    pub element: Element,
    pub recommended_class: RecommendedClass,
    pub class_bonus: String,
    pub unlock_condition: UnlockCondition,
    pub evo_difficulty: EvoDifficulty,
    pub token_improvable: bool,
    pub special_ability: Option<String>,
}
