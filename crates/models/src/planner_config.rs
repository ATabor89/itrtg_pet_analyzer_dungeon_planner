//! Planner configuration: equipment rules + per-pet special info.
//!
//! This module holds the types for `planner_config.yaml` (equipment selection
//! rules per class and depth) and `pet_special_info.yaml` (per-pet quirks).
//!
//! The idea is to let us tweak recommendations without recompiling. The
//! planner crate consumes `PlannerConfig` instead of hardcoding decisions
//! like "mages get fire swords" or "rogues get wind knives".

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::dungeon::GemSlots;
use crate::{Class, Element};

// =============================================================================
// Top-level config
// =============================================================================

/// The full planner configuration: equipment rules + pet special info.
///
/// Built from two YAML sources:
/// - `planner_config.yaml` → [`EquipmentRules`]
/// - `pet_special_info.yaml` → map of pet name → [`PetSpecialInfo`]
#[derive(Debug, Clone, Serialize)]
pub struct PlannerConfig {
    pub equipment_rules: EquipmentRules,
    pub pet_special_info: BTreeMap<String, PetSpecialInfo>,
}

impl PlannerConfig {
    /// Assemble a config from its two source documents.
    pub fn new(
        rules_file: PlannerConfigFile,
        special_info: BTreeMap<String, PetSpecialInfo>,
    ) -> Self {
        Self {
            equipment_rules: rules_file.equipment_rules,
            pet_special_info: special_info,
        }
    }

    /// Look up special info for a pet by name.
    pub fn special_info(&self, pet_name: &str) -> Option<&PetSpecialInfo> {
        self.pet_special_info.get(pet_name)
    }
}

/// On-disk schema for `planner_config.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerConfigFile {
    #[serde(default = "default_version")]
    pub version: u32,
    pub equipment_rules: EquipmentRules,
}

fn default_version() -> u32 {
    1
}

// =============================================================================
// Equipment rules
// =============================================================================

/// Equipment selection rules: a default rule per class, with optional
/// per-depth overrides, plus a catch-all fallback rule for pets whose class
/// isn't represented in the table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentRules {
    pub by_class: BTreeMap<Class, ClassEquipmentRule>,
    /// Fallback rule applied when no class-specific rule matches.
    pub fallback: ClassEquipmentRule,
}

impl EquipmentRules {
    /// Resolve the effective rule for a class at a specific depth.
    ///
    /// Returns a `ResolvedRule` where each slot is either the base selector
    /// or a depth-override selector from the matching `depth_overrides` entry.
    pub fn resolve(&self, class: Class, depth: u8) -> ResolvedRule<'_> {
        let base = self.by_class.get(&class).unwrap_or(&self.fallback);
        let ovr = base.depth_overrides.get(&depth);

        let weapon = ovr
            .and_then(|o| o.weapon.as_ref())
            .unwrap_or(&base.weapon);
        let armor = ovr.and_then(|o| o.armor.as_ref()).unwrap_or(&base.armor);
        let accessory = ovr
            .and_then(|o| o.accessory.as_ref())
            .unwrap_or(&base.accessory);
        let gems = ovr.and_then(|o| o.gems.as_ref()).or(base.gems.as_ref());

        ResolvedRule {
            weapon,
            armor,
            accessory,
            gems,
        }
    }
}

/// The resolved (base + per-depth override) rule for one class at one depth.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedRule<'a> {
    pub weapon: &'a EquipmentSelector,
    pub armor: &'a EquipmentSelector,
    pub accessory: &'a EquipmentSelector,
    pub gems: Option<&'a GemRules>,
}

/// The per-class base rule, plus optional depth-specific overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassEquipmentRule {
    pub weapon: EquipmentSelector,
    pub armor: EquipmentSelector,
    pub accessory: EquipmentSelector,
    /// Gem recommendations per depth. `None` means "no gems for any depth".
    #[serde(default)]
    pub gems: Option<GemRules>,
    /// Optional per-depth overrides for any of the slots or gems.
    #[serde(default)]
    pub depth_overrides: BTreeMap<u8, DepthOverride>,
}

/// Partial override of a class rule for a specific depth.
/// All fields are optional; unset fields inherit from the base rule.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DepthOverride {
    pub weapon: Option<EquipmentSelector>,
    pub armor: Option<EquipmentSelector>,
    pub accessory: Option<EquipmentSelector>,
    pub gems: Option<GemRules>,
}

// =============================================================================
// Equipment selector
// =============================================================================

/// A rule for picking one piece of equipment.
///
/// Selectors describe *how* to pick, not *what* to pick — the catalog lookup
/// happens inside the planner. Fields that are `None` are treated as "any".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "strategy", rename_all = "snake_case")]
pub enum EquipmentSelector {
    /// A specific element. Used when the choice is fixed regardless of pet
    /// or dungeon (e.g. "always fire sword for mages").
    Fixed {
        element: Element,
        /// Optional substring match on equipment name (e.g. "knives", "sword",
        /// "hammer"). Matched case-insensitively.
        #[serde(default)]
        kind: Option<String>,
    },

    /// Use the pet's own element. Falls back to `fallback` when the pet is
    /// Neutral or All-element.
    PetElement {
        fallback: Element,
        #[serde(default)]
        kind: Option<String>,
    },

    /// The element the pet is weak to (i.e. what counters the pet). Used for
    /// defensive armor — "CounterElement" in the wiki equipment recs.
    PetWeakness {
        fallback: Element,
        #[serde(default)]
        kind: Option<String>,
    },

    /// The dungeon's primary element. Falls back when the dungeon is Neutral
    /// (e.g. Scrapyard, Newbie).
    DungeonElement {
        fallback: Element,
        #[serde(default)]
        kind: Option<String>,
    },

    /// The element that counters the dungeon element — "DungeonCounter" in
    /// the wiki docs. Useful for offense against a mono-element dungeon.
    DungeonCounter {
        fallback: Element,
        #[serde(default)]
        kind: Option<String>,
    },

    /// Look up a specific named item (substring match on equipment name),
    /// e.g. "Alchemist Cape".
    ByName { name_contains: String },

    /// Try each selector in order; the first one that resolves to a catalog
    /// entry wins. Lets us express "alchemist cape at T3, otherwise pet
    /// element accessory".
    Chain { options: Vec<EquipmentSelector> },
}

// =============================================================================
// Gem rules
// =============================================================================

/// Gem recommendations per depth level.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GemRules {
    pub d1: Option<GemSlots>,
    pub d2: Option<GemSlots>,
    pub d3: Option<GemSlots>,
}

impl GemRules {
    /// Select the gem slot recommendation for a specific depth.
    ///
    /// Depth 1 → `d1`, 2 → `d2`, anything else → `d3`.
    pub fn for_depth(&self, depth: u8) -> Option<&GemSlots> {
        match depth {
            0 | 1 => self.d1.as_ref(),
            2 => self.d2.as_ref(),
            _ => self.d3.as_ref(),
        }
    }
}

// =============================================================================
// Pet special info
// =============================================================================

/// Per-pet quirks loaded from `pet_special_info.yaml`.
///
/// This mirrors the structure of that file. Many fields are loose/freeform —
/// the solver and equipment recommender only consume a subset. Fields the
/// algorithm doesn't act on are kept so that UI can surface them to the user.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PetSpecialInfo {
    #[serde(default)]
    pub stat_modifiers: Option<StatModifiers>,

    #[serde(default)]
    pub element_scaling: Vec<ElementScaling>,

    #[serde(default)]
    pub team_synergies: Vec<TeamSynergy>,

    #[serde(default)]
    pub team_anti_synergies: Vec<TeamAntiSynergy>,

    #[serde(default)]
    pub class_constraints: Vec<ClassConstraint>,

    #[serde(default)]
    pub element_constraints: Option<ElementConstraints>,

    #[serde(default)]
    pub equipment_constraints: Vec<EquipmentConstraint>,

    #[serde(default)]
    pub special_mechanics: Vec<SpecialMechanic>,

    #[serde(default)]
    pub token_improvement: Option<TokenImprovement>,

    #[serde(default)]
    pub notes: Option<String>,
}

impl PetSpecialInfo {
    /// The first `element_scaling` entry that sets `priority_override: true`.
    /// Used by the equipment recommender to override element selection.
    pub fn priority_element_override(&self) -> Option<Element> {
        self.element_scaling
            .iter()
            .find(|es| es.priority_override)
            .and_then(|es| parse_element_name(&es.element))
    }

    /// Required weapon type (e.g. "bow") if the pet's mechanic depends on it.
    pub fn required_weapon_kind(&self) -> Option<&str> {
        self.equipment_constraints
            .iter()
            .find_map(|ec| ec.required_weapon_type.as_deref())
    }

    /// Forbidden weapon type (e.g. "knives" for Ghost) the recommender must
    /// avoid suggesting.
    pub fn forbidden_weapon_kind(&self) -> Option<&str> {
        self.equipment_constraints
            .iter()
            .find_map(|ec| ec.forbidden_weapon_type.as_deref())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StatModifiers {
    pub hp: Option<i32>,
    pub attack: Option<i32>,
    pub defense: Option<i32>,
    pub speed: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ElementScaling {
    /// Element name as a lowercase string (e.g. "fire", "water"). Kept as a
    /// string to tolerate YAML inputs that don't match the `Element` enum
    /// exactly; see [`PetSpecialInfo::priority_element_override`].
    pub element: String,
    pub multiplier: Option<f32>,
    pub extra_hits: Option<ExtraHits>,
    pub party_buff: Option<PartyBuff>,
    /// When true, this pet should prefer the listed element over the default
    /// class-based element choice in the equipment recommender.
    pub priority_override: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ExtraHits {
    pub per: Option<u32>,
    pub max: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PartyBuff {
    pub stat: Option<String>,
    pub per_cl: Option<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TeamSynergy {
    pub pet: Option<String>,
    pub class: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TeamAntiSynergy {
    pub pet: Option<String>,
    /// "dungeon" | "campaign" | "all"
    pub context: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ClassConstraint {
    pub locked_class: Option<String>,
    pub preferred_class: Option<String>,
    pub avoid_class: Option<String>,
    pub flexible_class: Option<bool>,
    pub class_wildcard: Option<bool>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ElementConstraints {
    pub element_wildcard: Option<bool>,
    pub multi_element: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EquipmentConstraint {
    pub required_weapon_type: Option<String>,
    pub forbidden_weapon_type: Option<String>,
    /// Equipment (by display name) that must not be on ANY teammate.
    #[serde(default)]
    pub forbidden_team_equipment: Vec<String>,
    pub special_weapon: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SpecialMechanic {
    pub name: Option<String>,
    pub description: Option<String>,
    /// "self" | "team" | "enemies"
    pub affects: Option<String>,
    pub combat_relevant: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TokenImprovement {
    pub description: Option<String>,
    #[serde(default)]
    pub unlocks_mechanics: Vec<String>,
}

// =============================================================================
// Helpers
// =============================================================================

/// Parse a case-insensitive element name from a string.
/// Returns `None` for unknown names (so mismatched YAML is ignored rather
/// than crashing the planner).
fn parse_element_name(s: &str) -> Option<Element> {
    match s.trim().to_lowercase().as_str() {
        "fire" => Some(Element::Fire),
        "water" => Some(Element::Water),
        "wind" => Some(Element::Wind),
        "earth" => Some(Element::Earth),
        "neutral" => Some(Element::Neutral),
        "all" => Some(Element::All),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_element_name() {
        assert_eq!(parse_element_name("fire"), Some(Element::Fire));
        assert_eq!(parse_element_name("Water"), Some(Element::Water));
        assert_eq!(parse_element_name("  WIND  "), Some(Element::Wind));
        assert_eq!(parse_element_name("nope"), None);
    }

    #[test]
    fn test_gem_rules_for_depth() {
        let rules = GemRules {
            d1: None,
            d2: Some(GemSlots {
                weapon: Some(Element::Fire),
                armor: None,
                accessory: None,
            }),
            d3: Some(GemSlots {
                weapon: Some(Element::Fire),
                armor: Some(Element::Water),
                accessory: Some(Element::Neutral),
            }),
        };
        assert!(rules.for_depth(1).is_none());
        assert!(rules.for_depth(2).is_some());
        assert!(rules.for_depth(3).is_some());
        assert!(rules.for_depth(4).is_some()); // clamped to d3
    }

    #[test]
    fn test_resolve_rule_with_depth_override() {
        let rules: EquipmentRules = serde_yaml::from_str(
            r#"
fallback:
  weapon:
    strategy: fixed
    element: Neutral
  armor:
    strategy: fixed
    element: Neutral
  accessory:
    strategy: fixed
    element: Neutral
by_class:
  Mage:
    weapon:
      strategy: fixed
      element: Fire
      kind: sword
    armor:
      strategy: pet_element
      fallback: Neutral
    accessory:
      strategy: pet_element
      fallback: Neutral
    depth_overrides:
      2:
        armor:
          strategy: dungeon_element
          fallback: Neutral
"#,
        )
        .unwrap();

        // At depth 3 the mage uses its base armor rule (pet_element).
        let d3 = rules.resolve(Class::Mage, 3);
        assert!(matches!(d3.armor, EquipmentSelector::PetElement { .. }));

        // At depth 2 the override kicks in.
        let d2 = rules.resolve(Class::Mage, 2);
        assert!(matches!(d2.armor, EquipmentSelector::DungeonElement { .. }));

        // Weapon is inherited from the base rule on both.
        assert!(matches!(d2.weapon, EquipmentSelector::Fixed { .. }));
    }

    #[test]
    fn test_pet_special_info_weapon_overrides() {
        let yaml = r#"
required_weapon_type: bow
"#;
        let ec: EquipmentConstraint = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(ec.required_weapon_type.as_deref(), Some("bow"));

        let info = PetSpecialInfo {
            equipment_constraints: vec![ec],
            ..Default::default()
        };
        assert_eq!(info.required_weapon_kind(), Some("bow"));
    }

    #[test]
    fn test_priority_element_override() {
        let info = PetSpecialInfo {
            element_scaling: vec![ElementScaling {
                element: "wind".to_string(),
                priority_override: true,
                ..Default::default()
            }],
            ..Default::default()
        };
        assert_eq!(info.priority_element_override(), Some(Element::Wind));
    }

    /// The real `pet_special_info.yaml` bundled in `data/` must round-trip
    /// through our types without error. This guards against schema drift —
    /// if someone adds a new field in the YAML that we don't know about,
    /// this test would still succeed (serde drops unknown fields), but any
    /// syntax or type error in the existing data will trip it.
    #[test]
    fn test_real_pet_special_info_parses() {
        let yaml = include_str!("../../../data/pet_special_info.yaml");
        let map: BTreeMap<String, PetSpecialInfo> =
            serde_yaml::from_str(yaml).expect("pet_special_info.yaml should parse");
        // Sanity check: we expect at least a handful of known pets.
        assert!(map.contains_key("Hourglass"));
        assert!(map.contains_key("Sylph"));
        assert!(map.contains_key("Archer"));
    }

    /// Same for `planner_config.yaml` — make sure the checked-in equipment
    /// rules actually parse against the current schema.
    #[test]
    fn test_real_planner_config_parses() {
        let yaml = include_str!("../../../data/planner_config.yaml");
        let file: PlannerConfigFile =
            serde_yaml::from_str(yaml).expect("planner_config.yaml should parse");
        // Every dungeon class we care about must have a rule.
        for class in [
            Class::Defender,
            Class::Supporter,
            Class::Mage,
            Class::Assassin,
            Class::Rogue,
            Class::Blacksmith,
        ] {
            assert!(
                file.equipment_rules.by_class.contains_key(&class),
                "missing rule for {class:?}"
            );
        }
    }
}
