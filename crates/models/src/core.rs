use serde::{Deserialize, Serialize};

/// The primary element of a pet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Element {
    Neutral,
    Fire,
    Water,
    Wind,
    Earth,
    /// Chameleon can freely change its element.
    All,
}

/// A pet class. These are the actual in-game classes.
///
/// `Wildcard` is not a real in-game class — it is used in [`RecommendedClass::Dual`]
/// to indicate the pet can viably be evolved as whatever the player needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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

impl Element {
    /// What element this one is strong against offensively.
    /// Fire→Wind, Water→Fire, Wind→Earth, Earth→Water. Neutral→Neutral, All→Neutral.
    pub fn counters(self) -> Element {
        match self {
            Element::Fire => Element::Wind,
            Element::Water => Element::Fire,
            Element::Wind => Element::Earth,
            Element::Earth => Element::Water,
            Element::Neutral | Element::All => Element::Neutral,
        }
    }

    /// What element beats this one (the reverse of counters).
    pub fn countered_by(self) -> Element {
        match self {
            Element::Fire => Element::Water,
            Element::Water => Element::Earth,
            Element::Wind => Element::Fire,
            Element::Earth => Element::Wind,
            Element::Neutral | Element::All => Element::Neutral,
        }
    }
}

/// Dungeon names in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Dungeon {
    NewbieGround,
    Scrapyard,
    WaterTemple,
    Volcano,
    Mountain,
    Forest,
}

impl Dungeon {
    /// The primary element of this dungeon.
    pub fn element(self) -> Element {
        match self {
            Dungeon::NewbieGround => Element::Neutral,
            Dungeon::Scrapyard => Element::Neutral,
            Dungeon::WaterTemple => Element::Water,
            Dungeon::Volcano => Element::Fire,
            Dungeon::Mountain => Element::Wind,
            Dungeon::Forest => Element::Earth,
        }
    }
}
