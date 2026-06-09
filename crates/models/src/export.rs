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

/// Multiplier the Magic Egg applies to a pet's growth while equipped (+30%).
pub const MAGIC_EGG_GROWTH_MULT: f64 = 1.3;

/// Global growth multiplier once **all** Patreon God Challenges are completed
/// (+50%). Applies to every pet and stacks multiplicatively with the Magic Egg
/// (1.5 × 1.3 = 1.95×).
pub const PGC_GROWTH_MULT: f64 = 1.5;

impl ExportPet {
    /// Whether this pet currently has a Magic Egg equipped.
    pub fn has_magic_egg(&self) -> bool {
        self.loadout
            .weapon
            .as_ref()
            .is_some_and(|w| w.name == "Magic Egg")
    }

    /// Growth this pet *would* have with a Magic Egg equipped, regardless of its
    /// current loadout. Used for "could evolve if the egg were equipped" checks.
    pub fn growth_with_magic_egg(&self) -> u64 {
        (self.growth as f64 * MAGIC_EGG_GROWTH_MULT).round() as u64
    }

    /// Growth value the game uses *now* — includes the Magic Egg bonus only if
    /// one is actually equipped.
    pub fn effective_growth(&self) -> u64 {
        if self.has_magic_egg() {
            self.growth_with_magic_egg()
        } else {
            self.growth
        }
    }
}
