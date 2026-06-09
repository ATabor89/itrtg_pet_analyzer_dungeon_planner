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

/// Global growth multiplier from Patreon God Challenge completions: **+1% per
/// completion**, doubled once **all** are complete — so 24/25 is ×1.24 but
/// 25/25 jumps to ×1.50. Applies to every pet and stacks multiplicatively with
/// the Magic Egg (at 25/25: 1.5 × 1.3 = 1.95×).
pub fn pgc_growth_mult(done: u32, max: u32) -> f64 {
    let pct = if max > 0 && done >= max { 2.0 * done as f64 } else { done as f64 };
    1.0 + pct / 100.0
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pgc_mult_is_gradual_and_doubles_at_completion() {
        let close = |a: f64, b: f64| (a - b).abs() < 1e-12;
        assert!(close(pgc_growth_mult(0, 25), 1.0));
        assert!(close(pgc_growth_mult(10, 25), 1.10));
        assert!(close(pgc_growth_mult(24, 25), 1.24));
        assert!(close(pgc_growth_mult(25, 25), 1.50));
        // No challenges known at all → no bonus (and no spurious doubling).
        assert!(close(pgc_growth_mult(0, 0), 1.0));
    }
}
