use serde::{Deserialize, Serialize};

use crate::{Class, Element, Loadout, PetAction};

/// Combat stats for a pet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatStats {
    pub hp: i64,
    pub attack: i64,
    pub defense: i64,
    pub speed: i64,
}

/// Elemental affinities for a pet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementalAffinities {
    pub water: i64,
    pub fire: i64,
    pub wind: i64,
    pub earth: i64,
    pub dark: i64,
    pub light: i64,
}

/// A pet as parsed from the in-game export. Represents the player's actual pet state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPet {
    /// The export name (may differ from wiki name — use name mappings to correlate).
    pub export_name: String,
    pub element: Element,
    pub growth: u64,
    pub dungeon_level: u32,
    /// None means unevolved.
    pub class: Option<Class>,
    pub class_level: u32,
    pub combat_stats: CombatStats,
    pub elemental_affinities: ElementalAffinities,
    pub loadout: Loadout,
    pub action: PetAction,
    pub unlocked: bool,
    pub improved: bool,
    /// Miscellaneous pet-specific data from the "Other" column.
    pub other: Option<String>,
    pub has_partner: bool,
}
