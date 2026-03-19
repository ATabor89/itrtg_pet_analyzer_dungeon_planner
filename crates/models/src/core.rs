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
