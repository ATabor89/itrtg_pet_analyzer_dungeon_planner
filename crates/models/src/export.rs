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
    /// Class experience accumulated toward the next class level. The normal pet
    /// stats export does **not** carry this — only a full save-file import can
    /// supply it — so it defaults to 0 for export-sourced and pre-existing
    /// persisted pets. The Growth Chamber sim uses it as the starting point for
    /// per-cycle Adventurer class-XP accrual; see `reference/growth_chamber_status.md`.
    #[serde(default)]
    pub class_exp: f64,
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
    let done = if max > 0 { done.min(max) } else { 0 };
    let pct = if max > 0 && done >= max { 2.0 * done as f64 } else { done as f64 };
    1.0 + pct / 100.0
}

/// Apply a positive growth multiplier using the game's displayed whole-growth
/// rounding convention.
pub fn displayed_growth(base: u64, multiplier: f64) -> u64 {
    (base as f64 * multiplier).round() as u64
}

/// Smallest base growth whose rounded displayed value reaches `target`.
///
/// The direct `ceil(target / multiplier)` inversion is one point too high near
/// rounding boundaries. Start from the inverse of `round(x) >= target`, then
/// correct around floating-point boundaries against [`displayed_growth`].
pub fn base_growth_for_displayed_target(target: u64, multiplier: f64) -> u64 {
    if target == 0 {
        return 0;
    }
    if !multiplier.is_finite() || multiplier <= 0.0 {
        return u64::MAX;
    }

    let mut base = ((target as f64 - 0.5) / multiplier).ceil() as u64;
    while base > 0 && displayed_growth(base - 1, multiplier) >= target {
        base -= 1;
    }
    while displayed_growth(base, multiplier) < target {
        let next = base.saturating_add(1);
        if next == base {
            break;
        }
        base = next;
    }
    base
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
        self.growth_with_magic_egg_and_global_mult(1.0)
    }

    /// Growth with a Magic Egg plus an account-wide growth multiplier such as
    /// PGC. The multipliers stack before rounding, matching the chamber model.
    pub fn growth_with_magic_egg_and_global_mult(&self, global_mult: f64) -> u64 {
        displayed_growth(self.growth, MAGIC_EGG_GROWTH_MULT * global_mult)
    }

    /// Growth value the game uses *now* — includes the Magic Egg bonus only if
    /// one is actually equipped.
    pub fn effective_growth(&self) -> u64 {
        self.effective_growth_with_global_mult(1.0)
    }

    /// Current effective growth including equipped Magic Egg and an
    /// account-wide multiplier such as PGC.
    pub fn effective_growth_with_global_mult(&self, global_mult: f64) -> u64 {
        if self.has_magic_egg() {
            self.growth_with_magic_egg_and_global_mult(global_mult)
        } else {
            displayed_growth(self.growth, global_mult)
        }
    }

    /// The elemental pet's evolved **form**, parsed from the export "Other"
    /// column (`GnomeV2`, `SylphV1`, …). `None` for non-elemental pets (whose
    /// "Other" is empty or holds unrelated data) and for any "Other" value that
    /// isn't this pet's `<name>V<number>` form string.
    ///
    /// Each form upgrade (built via the pet's quest) bumps the version and the
    /// pet's base growth. The save-side counterpart is `SavePet.elemental_form_id`
    /// (`y`), an offset-encoded counter — the export carries the human label.
    pub fn elemental_form(&self) -> Option<ElementalForm> {
        let s = self.other.as_deref()?.trim();
        let bytes = s.as_bytes();
        // Strip the trailing run of digits (the version number).
        let mut i = bytes.len();
        while i > 0 && bytes[i - 1].is_ascii_digit() {
            i -= 1;
        }
        if i == bytes.len() || i == 0 || !bytes[i - 1].eq_ignore_ascii_case(&b'V') {
            return None;
        }
        let name = s[..i - 1].trim();
        // Guard against unrelated "Other" content: the prefix must be this pet.
        if name.is_empty() || !name.eq_ignore_ascii_case(&self.export_name) {
            return None;
        }
        // Use the canonical export-name casing (the guard confirmed they match).
        Some(ElementalForm { name: self.export_name.clone(), version: s[i..].parse().ok()? })
    }
}

/// An elemental pet's evolved form, from the export "Other" column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementalForm {
    /// Name prefix (equals the pet's export name, e.g. "Gnome").
    pub name: String,
    /// The in-game "V" number (`GnomeV2` → 2). This is the game's own per-pet
    /// label, **not** a uniform 0-based index — Gnome's base form reads as V1
    /// while Sylph/Salamander start at V0.
    pub version: u32,
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
        assert!(close(pgc_growth_mult(30, 25), 1.50));
        assert!(close(pgc_growth_mult(30, 0), 1.0));
    }

    #[test]
    fn displayed_target_inverse_matches_rounding_boundaries() {
        assert_eq!(displayed_growth(909, 1.1), 1000);
        assert_eq!(base_growth_for_displayed_target(1000, 1.1), 909);
        assert_eq!(displayed_growth(699, 1.1 * 1.3), 1000);
        assert_eq!(base_growth_for_displayed_target(1000, 1.1 * 1.3), 699);
        assert_eq!(base_growth_for_displayed_target(0, 1.5), 0);
    }

    #[test]
    fn effective_growth_stacks_egg_and_global_multiplier_before_rounding() {
        let mut pet: ExportPet = serde_yaml::from_str(
            "export_name: Test\nelement: Earth\ngrowth: 101\ndungeon_level: 0\nclass: null\nclass_level: 0\ncombat_stats: { hp: 0, attack: 0, defense: 0, speed: 0 }\nelemental_affinities: { water: 0, fire: 0, wind: 0, earth: 0, dark: 0, light: 0 }\nloadout: { weapon: null, armor: null, accessory: null }\naction: Idle\nunlocked: true\nimproved: false\nother: null\nhas_partner: false\n",
        )
        .unwrap();
        assert_eq!(pet.effective_growth_with_global_mult(1.1), 111);

        pet.loadout.weapon = Some(crate::Equipment {
            name: "Magic Egg".to_string(),
            upgrade_level: None,
            quality: crate::Quality::SSS,
            enchant_level: None,
            gem: None,
            gem_level: None,
        });
        assert_eq!(pet.effective_growth_with_global_mult(1.1), 144);
    }

    /// A persisted pet from before `class_exp` existed (e.g. an older
    /// `app_state.yaml`) must still load, defaulting the field to 0.
    #[test]
    fn class_exp_defaults_to_zero_when_absent() {
        let yaml = r#"
export_name: Cupid
element: Fire
growth: 57018
dungeon_level: 20
class: Adventurer
class_level: 10
combat_stats: { hp: 1, attack: 1, defense: 1, speed: 1 }
elemental_affinities: { water: 0, fire: 0, wind: 0, earth: 0, dark: 0, light: 0 }
loadout: { weapon: null, armor: null, accessory: null }
action: Idle
unlocked: true
improved: false
other: null
has_partner: false
"#;
        let pet: ExportPet = serde_yaml::from_str(yaml).expect("deserializes without class_exp");
        assert_eq!(pet.class_exp, 0.0);
        assert_eq!(pet.class_level, 10);
    }

    /// Build a minimal pet with a given export name + "Other" value.
    fn pet_with_other(export_name: &str, other: Option<&str>) -> ExportPet {
        let other_yaml = other.map_or("null".to_string(), |s| format!("\"{s}\""));
        let yaml = format!(
            r#"
export_name: {export_name}
element: Wind
growth: 50331
dungeon_level: 1
class: null
class_level: 0
combat_stats: {{ hp: 1, attack: 1, defense: 1, speed: 1 }}
elemental_affinities: {{ water: 0, fire: 0, wind: 0, earth: 0, dark: 0, light: 0 }}
loadout: {{ weapon: null, armor: null, accessory: null }}
action: Idle
unlocked: true
improved: false
other: {other_yaml}
has_partner: false
"#
        );
        serde_yaml::from_str(&yaml).expect("pet deserializes")
    }

    #[test]
    fn elemental_form_parses_other_column() {
        // Real form strings: "<name>V<number>".
        assert_eq!(
            pet_with_other("Sylph", Some("SylphV2")).elemental_form(),
            Some(ElementalForm { name: "Sylph".into(), version: 2 })
        );
        assert_eq!(
            pet_with_other("Gnome", Some("GnomeV1")).elemental_form(),
            Some(ElementalForm { name: "Gnome".into(), version: 1 })
        );
        // V0 (base form) parses too.
        assert_eq!(
            pet_with_other("Salamander", Some("SalamanderV0")).elemental_form().unwrap().version,
            0
        );
        // Non-elemental / empty / non-matching "Other" → None.
        assert_eq!(pet_with_other("Cat", None).elemental_form(), None);
        assert_eq!(pet_with_other("Cat", Some("")).elemental_form(), None);
        // Prefix must be THIS pet (guards against unrelated "Other" content that
        // happens to end in V<digits>).
        assert_eq!(pet_with_other("Gnome", Some("SylphV2")).elemental_form(), None);
        assert_eq!(pet_with_other("Cat", Some("Lives9")).elemental_form(), None);
    }
}
