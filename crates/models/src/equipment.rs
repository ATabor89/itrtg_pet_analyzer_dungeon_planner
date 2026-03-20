use serde::{Deserialize, Serialize};

use crate::Element;

/// Equipment quality grade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Quality {
    F,
    E,
    D,
    C,
    B,
    A,
    S,
    SS,
    SSS,
}

/// Which slot a piece of equipment occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EquipmentSlot {
    Weapon,
    Armor,
    Accessory,
}

/// A piece of pet equipment parsed from the export data.
///
/// Format examples from export:
///   "Journeying Stick + 5, S (20)"  → name, +5 upgrade, S quality, 20 enchant
///   "Flame Sword + 10, SSS (1)"     → name, +10 upgrade, SSS quality, 1 enchant
///   "Feather Vest, S"               → name, no upgrade, S quality, no enchant
///   "Alchemist Cape, SSS"           → name, no upgrade, SSS quality, no enchant
///   "none"                          → no equipment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Equipment {
    pub name: String,
    pub upgrade_level: Option<u8>,
    pub quality: Quality,
    pub enchant_level: Option<u8>,
    /// Embedded gem element, if any. Parsed from export data when available.
    #[serde(default)]
    pub gem: Option<Element>,
}

/// The full loadout for a pet (all three equipment slots).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Loadout {
    pub weapon: Option<Equipment>,
    pub armor: Option<Equipment>,
    pub accessory: Option<Equipment>,
}
