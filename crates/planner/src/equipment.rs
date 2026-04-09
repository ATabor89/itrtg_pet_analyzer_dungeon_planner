//! Equipment recommendation engine.
//!
//! Computes concrete equipment suggestions for dungeon party slots that have
//! generic or missing gear, based on the pet's class, element, and the dungeon
//! context. Recommendations are driven by the [`PlannerConfig`] loaded from
//! `planner_config.yaml` — there are no hardcoded per-class rules in this
//! module any more. The only logic here is:
//!
//! 1. Resolve the effective rule for (class, depth) from the config.
//! 2. Apply per-pet overrides from `pet_special_info.yaml` (e.g. element
//!    priority for Sylph, required weapon kind for Archer).
//! 3. Turn each [`EquipmentSelector`] into a concrete catalog key.
//!
//! That lets us tweak the "mages die too often at D2" problem (and similar)
//! without touching Rust code — just edit the YAML.

use itrtg_models::dungeon::*;
use itrtg_models::planner_config::{
    EquipmentSelector, PlannerConfig, ResolvedRule,
};
use itrtg_models::*;

use crate::merge::MergedPet;
use crate::solver::{Assignment, DungeonPlan};

// =============================================================================
// Types
// =============================================================================

/// Where the equipment recommendation came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentSource {
    /// From the YAML dungeon_recommendations file (static, hand-curated).
    Static,
    /// Computed by the equipment recommendation engine.
    Computed,
}

/// Equipment suggestion with provenance tracking.
#[derive(Debug, Clone)]
pub struct EquipmentSuggestion {
    pub equipment: PartyEquipment,
    pub source: EquipmentSource,
}

// =============================================================================
// Post-processing enrichment
// =============================================================================

/// Enrich a solved plan with equipment suggestions.
///
/// - Static (YAML-defined, non-generic) equipment is preserved and tagged as `Static`.
/// - Generic or missing equipment is replaced with computed suggestions tagged as `Computed`.
pub fn enrich_equipment(
    plan: &mut DungeonPlan,
    catalog: &EquipmentCatalog,
    config: &PlannerConfig,
) {
    let dungeon = plan.dungeon;
    let depth = plan.depth;

    for sa in &mut plan.assignments {
        let is_generic = sa.slot.equipment.as_ref().is_some_and(has_generic_keys);
        let is_missing = sa.slot.equipment.is_none();

        if let Some(equip) = &sa.slot.equipment
            && !is_generic
        {
            // Static equipment from YAML — tag and preserve
            sa.equipment_suggestion = Some(EquipmentSuggestion {
                equipment: equip.clone(),
                source: EquipmentSource::Static,
            });
            continue;
        }

        // Need to compute: get the pet's effective class
        let Assignment::Filled { pet, .. } = &sa.assignment else {
            continue; // No pet assigned — can't recommend equipment
        };

        let class = resolve_effective_class(pet, &sa.slot);
        let pet_element = pet.element().unwrap_or(Element::Neutral);

        if let Some(class) = class {
            let suggestion = recommend_for_pet(
                pet, class, pet_element, dungeon, depth, catalog, config,
            );
            sa.equipment_suggestion = Some(suggestion);
        } else if is_missing || is_generic {
            // No class info at all — skip, leave as None
        }
    }
}

/// Check if a PartyEquipment contains any generic placeholder keys.
fn has_generic_keys(eq: &PartyEquipment) -> bool {
    eq.weapon.as_deref().is_some_and(|k| k.starts_with("generic_"))
        || eq.armor.as_deref().is_some_and(|k| k.starts_with("generic_"))
        || eq.accessory.as_deref().is_some_and(|k| k.starts_with("generic_"))
}

/// Determine the effective class for equipment selection.
/// Priority: evolved class > slot's required class > recommended class.
fn resolve_effective_class(pet: &MergedPet, slot: &PartySlot) -> Option<Class> {
    pet.evolved_class()
        .or(slot.class)
        .or_else(|| {
            pet.recommended_class()
                .and_then(|rc| rc.primary_class())
        })
}

// =============================================================================
// Core recommendation engine
// =============================================================================

/// Recommend equipment for a pet in a specific dungeon context, honoring
/// per-pet overrides from `pet_special_info.yaml`.
///
/// This is the primary entry point. `recommend_equipment` (below) remains as
/// a thin wrapper for callers that don't have a specific pet in hand — e.g.
/// unit tests and the dungeon preview UI.
fn recommend_for_pet(
    pet: &MergedPet,
    class: Class,
    pet_element: Element,
    dungeon: Dungeon,
    depth: u8,
    catalog: &EquipmentCatalog,
    config: &PlannerConfig,
) -> EquipmentSuggestion {
    let rule = config.equipment_rules.resolve(class, depth);
    let special = config.special_info(&pet.name);
    let ctx = SelectCtx {
        pet_element,
        dungeon_element: dungeon.element(),
        tier: depth.clamp(1, 3),
    };

    // Weapon: apply pet-level overrides first (required/forbidden kinds),
    // then resolve via the normal selector.
    let weapon_key = select_weapon(&rule, special, &ctx, catalog);
    let armor_key = resolve_selector(rule.armor, &ctx, catalog, EquipmentSlot::Armor);
    let accessory_key = select_accessory(&rule, special, &ctx, catalog);
    let gems = rule.gems.and_then(|g| g.for_depth(depth).cloned());

    EquipmentSuggestion {
        equipment: PartyEquipment {
            weapon: weapon_key,
            armor: armor_key,
            accessory: accessory_key,
            gems,
        },
        source: EquipmentSource::Computed,
    }
}

/// Recommend equipment ignoring per-pet special info. Used by callers that
/// don't have a specific pet in hand, such as the dungeon preview view.
pub fn recommend_equipment(
    class: Class,
    pet_element: Element,
    dungeon: Dungeon,
    depth: u8,
    catalog: &EquipmentCatalog,
    config: &PlannerConfig,
) -> EquipmentSuggestion {
    let rule = config.equipment_rules.resolve(class, depth);
    let ctx = SelectCtx {
        pet_element,
        dungeon_element: dungeon.element(),
        tier: depth.clamp(1, 3),
    };

    let weapon_key = resolve_selector(rule.weapon, &ctx, catalog, EquipmentSlot::Weapon);
    let armor_key = resolve_selector(rule.armor, &ctx, catalog, EquipmentSlot::Armor);
    let accessory_key =
        resolve_selector(rule.accessory, &ctx, catalog, EquipmentSlot::Accessory);
    let gems = rule.gems.and_then(|g| g.for_depth(depth).cloned());

    EquipmentSuggestion {
        equipment: PartyEquipment {
            weapon: weapon_key,
            armor: armor_key,
            accessory: accessory_key,
            gems,
        },
        source: EquipmentSource::Computed,
    }
}

// =============================================================================
// Selector resolution
// =============================================================================

/// Lookup context passed down through selector resolution.
#[derive(Debug, Clone, Copy)]
struct SelectCtx {
    pet_element: Element,
    dungeon_element: Element,
    tier: u8,
}

/// Turn an [`EquipmentSelector`] into a catalog key for the given slot.
/// Returns `None` if nothing in the catalog matches.
fn resolve_selector(
    selector: &EquipmentSelector,
    ctx: &SelectCtx,
    catalog: &EquipmentCatalog,
    slot: EquipmentSlot,
) -> Option<String> {
    match selector {
        EquipmentSelector::Fixed { element, kind } => {
            lookup_element(catalog, slot, *element, ctx.tier, kind.as_deref())
        }
        EquipmentSelector::PetElement { fallback, kind } => {
            let el = effective_pet_element(ctx.pet_element, *fallback);
            lookup_element(catalog, slot, el, ctx.tier, kind.as_deref())
        }
        EquipmentSelector::PetWeakness { fallback, kind } => {
            let el = match ctx.pet_element {
                Element::Fire | Element::Water | Element::Wind | Element::Earth => {
                    ctx.pet_element.countered_by()
                }
                _ => *fallback,
            };
            lookup_element(catalog, slot, el, ctx.tier, kind.as_deref())
        }
        EquipmentSelector::DungeonElement { fallback, kind } => {
            let el = if ctx.dungeon_element == Element::Neutral {
                *fallback
            } else {
                ctx.dungeon_element
            };
            lookup_element(catalog, slot, el, ctx.tier, kind.as_deref())
        }
        EquipmentSelector::DungeonCounter { fallback, kind } => {
            let el = match ctx.dungeon_element {
                Element::Fire | Element::Water | Element::Wind | Element::Earth => {
                    ctx.dungeon_element.countered_by()
                }
                _ => *fallback,
            };
            lookup_element(catalog, slot, el, ctx.tier, kind.as_deref())
        }
        EquipmentSelector::ByName { name_contains } => catalog
            .find_by_name(slot, ctx.tier, name_contains)
            .map(|(k, _)| k.to_string()),
        EquipmentSelector::Chain { options } => options
            .iter()
            .find_map(|s| resolve_selector(s, ctx, catalog, slot)),
    }
}

/// Map Neutral/All pet elements to the configured fallback.
fn effective_pet_element(pet_element: Element, fallback: Element) -> Element {
    match pet_element {
        Element::Fire | Element::Water | Element::Wind | Element::Earth => pet_element,
        _ => fallback,
    }
}

/// Look up a catalog entry for (slot, element, tier), optionally filtered by
/// a name substring ("knives", "sword", etc.).
fn lookup_element(
    catalog: &EquipmentCatalog,
    slot: EquipmentSlot,
    element: Element,
    tier: u8,
    kind: Option<&str>,
) -> Option<String> {
    if let Some(kind) = kind {
        catalog
            .find_by_kind(slot, element, tier, kind)
            .map(|(k, _)| k.to_string())
    } else {
        catalog.find(slot, element, tier).map(|(k, _)| k.to_string())
    }
}

// =============================================================================
// Per-pet special overrides
// =============================================================================

/// Select the weapon, applying pet-level constraints from `pet_special_info`.
///
/// Order of precedence (most specific first):
///   1. Required weapon kind (e.g. Archer/Cherub need a bow) — force that
///      kind, preferring the priority element override if set, otherwise
///      keeping the class default element.
///   2. Forbidden weapon kind (e.g. Ghost cannot equip knives) — if the
///      class default would pick that kind, substitute a sword instead.
///      The element is the priority override if set, otherwise Neutral.
///   3. Priority element override without a kind change (e.g. Sylph
///      prioritizing Wind) — keep the class default kind, swap element.
///   4. Plain class-default selector.
fn select_weapon(
    rule: &ResolvedRule<'_>,
    special: Option<&itrtg_models::planner_config::PetSpecialInfo>,
    ctx: &SelectCtx,
    catalog: &EquipmentCatalog,
) -> Option<String> {
    // Start with the element/kind the class rule would pick.
    let (base_element, base_kind) = selector_summary(rule.weapon, ctx);

    if let Some(info) = special {
        // Required kind (e.g. bow) — preserve element, swap kind.
        if let Some(req) = info.required_weapon_kind() {
            let el = info.priority_element_override().unwrap_or(base_element);
            return lookup_element(catalog, EquipmentSlot::Weapon, el, ctx.tier, Some(req));
        }

        // Forbidden kind — if the default would use it, fall back to a
        // sword. Element comes from the priority override if set
        // (e.g. a hypothetical pet that forbids knives and scales with
        // fire would get a fire sword); otherwise Neutral for the safe
        // generic choice. In practice today Ghost is the only pet with a
        // forbidden kind and has no priority override, so this resolves
        // to a plain neutral sword.
        if let Some(forbidden) = info.forbidden_weapon_kind()
            && base_kind
                .as_deref()
                .is_some_and(|k| k.eq_ignore_ascii_case(forbidden))
        {
            let el = info.priority_element_override().unwrap_or(Element::Neutral);
            return lookup_element(
                catalog,
                EquipmentSlot::Weapon,
                el,
                ctx.tier,
                Some("sword"),
            );
        }

        // Element priority override without kind change.
        if let Some(el) = info.priority_element_override() {
            return lookup_element(
                catalog,
                EquipmentSlot::Weapon,
                el,
                ctx.tier,
                base_kind.as_deref(),
            );
        }
    }

    resolve_selector(rule.weapon, ctx, catalog, EquipmentSlot::Weapon)
}

/// Select the accessory, honoring the priority-element override from
/// `pet_special_info` (used e.g. by Sylph/Rabbit to stack their scaling
/// element).
fn select_accessory(
    rule: &ResolvedRule<'_>,
    special: Option<&itrtg_models::planner_config::PetSpecialInfo>,
    ctx: &SelectCtx,
    catalog: &EquipmentCatalog,
) -> Option<String> {
    if let Some(info) = special
        && let Some(el) = info.priority_element_override()
    {
        let (_, kind) = selector_summary(rule.accessory, ctx);
        return lookup_element(
            catalog,
            EquipmentSlot::Accessory,
            el,
            ctx.tier,
            kind.as_deref(),
        );
    }

    resolve_selector(rule.accessory, ctx, catalog, EquipmentSlot::Accessory)
}

/// What element and kind filter the given selector would ask for right now,
/// before the catalog lookup happens. Used by the per-pet override logic to
/// decide whether a "forbidden kind" rule applies.
///
/// For chains we inspect the first option, which is good enough — a chain
/// whose first entry doesn't hit the forbidden kind should be allowed to try
/// its later options naturally.
fn selector_summary(
    selector: &EquipmentSelector,
    ctx: &SelectCtx,
) -> (Element, Option<String>) {
    match selector {
        EquipmentSelector::Fixed { element, kind } => (*element, kind.clone()),
        EquipmentSelector::PetElement { fallback, kind } => {
            (effective_pet_element(ctx.pet_element, *fallback), kind.clone())
        }
        EquipmentSelector::PetWeakness { fallback, kind } => {
            let el = match ctx.pet_element {
                Element::Fire | Element::Water | Element::Wind | Element::Earth => {
                    ctx.pet_element.countered_by()
                }
                _ => *fallback,
            };
            (el, kind.clone())
        }
        EquipmentSelector::DungeonElement { fallback, kind } => {
            let el = if ctx.dungeon_element == Element::Neutral {
                *fallback
            } else {
                ctx.dungeon_element
            };
            (el, kind.clone())
        }
        EquipmentSelector::DungeonCounter { fallback, kind } => {
            let el = match ctx.dungeon_element {
                Element::Fire | Element::Water | Element::Wind | Element::Earth => {
                    ctx.dungeon_element.countered_by()
                }
                _ => *fallback,
            };
            (el, kind.clone())
        }
        EquipmentSelector::ByName { .. } => (Element::Neutral, None),
        EquipmentSelector::Chain { options } => options
            .first()
            .map(|s| selector_summary(s, ctx))
            .unwrap_or((Element::Neutral, None)),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use itrtg_models::planner_config::PlannerConfigFile;
    use std::collections::BTreeMap;

    /// Build a minimal catalog with enough entries to test all recommendations.
    fn test_catalog() -> EquipmentCatalog {
        let yaml = r#"
weapons:
  iron_sword:
    name: "Iron Sword"
    type: Weapon
    tier: 1
    element: Neutral
  fire_sword:
    name: "Fire Sword"
    type: Weapon
    tier: 1
    element: Fire
  howling_knives:
    name: "Howling Knives"
    type: Weapon
    tier: 1
    element: Wind
  forging_hammer:
    name: "Forging Hammer"
    type: Weapon
    tier: 1
    element: Fire
  steel_sword:
    name: "Steel Sword"
    type: Weapon
    tier: 2
    element: Neutral
  flame_sword:
    name: "Flame Sword"
    type: Weapon
    tier: 2
    element: Fire
  thundering_knives:
    name: "Thundering Knives"
    type: Weapon
    tier: 2
    element: Wind
  shaping_hammer:
    name: "Shaping Hammer"
    type: Weapon
    tier: 2
    element: Fire
  titanium_sword:
    name: "Titanium Sword"
    type: Weapon
    tier: 3
    element: Neutral
  inferno_sword:
    name: "Inferno Sword"
    type: Weapon
    tier: 3
    element: Fire
  bursting_knives:
    name: "Bursting Knives"
    type: Weapon
    tier: 3
    element: Wind
  magic_hammer:
    name: "Magic Hammer"
    type: Weapon
    tier: 3
    element: Fire
  feather_bow:
    name: "Feather Bow"
    type: Weapon
    tier: 2
    element: Wind
armor:
  iron_vest:
    name: "Iron Vest"
    type: Armor
    tier: 1
    element: Neutral
  fire_vest:
    name: "Fire Vest"
    type: Armor
    tier: 1
    element: Fire
  water_vest:
    name: "Water Vest"
    type: Armor
    tier: 1
    element: Water
  steel_armor:
    name: "Steel Armor"
    type: Armor
    tier: 2
    element: Neutral
  flame_armor:
    name: "Flame Armor"
    type: Armor
    tier: 2
    element: Fire
  flood_armor:
    name: "Flood Armor"
    type: Armor
    tier: 2
    element: Water
  tree_armor:
    name: "Tree Armor"
    type: Armor
    tier: 2
    element: Earth
  storm_armor:
    name: "Storm Armor"
    type: Armor
    tier: 2
    element: Wind
  titanium_armor:
    name: "Titanium Armor"
    type: Armor
    tier: 3
    element: Neutral
  inferno_armor:
    name: "Inferno Armor"
    type: Armor
    tier: 3
    element: Fire
  tsunami_armor:
    name: "Tsunami Armor"
    type: Armor
    tier: 3
    element: Water
  forest_armor:
    name: "Forest Armor"
    type: Armor
    tier: 3
    element: Earth
  hurricane_armor:
    name: "Hurricane Armor"
    type: Armor
    tier: 3
    element: Wind
accessories:
  iron_ring:
    name: "Iron Ring"
    type: Accessory
    tier: 1
    element: Neutral
  fire_gloves:
    name: "Fire Gloves"
    type: Accessory
    tier: 1
    element: Fire
  feather_ring:
    name: "Feather Ring"
    type: Accessory
    tier: 1
    element: Wind
  steel_ring:
    name: "Steel Ring"
    type: Accessory
    tier: 2
    element: Neutral
  flame_gloves:
    name: "Flame Gloves"
    type: Accessory
    tier: 2
    element: Fire
  storm_ring:
    name: "Storm Ring"
    type: Accessory
    tier: 2
    element: Wind
  titanium_ring:
    name: "Titanium Ring"
    type: Accessory
    tier: 3
    element: Neutral
  inferno_gloves:
    name: "Inferno Gloves"
    type: Accessory
    tier: 3
    element: Fire
  hurricane_ring:
    name: "Hurricane Ring"
    type: Accessory
    tier: 3
    element: Wind
  flood_necklace:
    name: "Flood Necklace"
    type: Accessory
    tier: 2
    element: Water
  tree_bracelet:
    name: "Tree Bracelet"
    type: Accessory
    tier: 2
    element: Earth
  tsunami_necklace:
    name: "Tsunami Necklace"
    type: Accessory
    tier: 3
    element: Water
  forest_bracelet:
    name: "Forest Bracelet"
    type: Accessory
    tier: 3
    element: Earth
  alchemist_cape:
    name: "Alchemist Cape"
    type: Accessory
    tier: 3
    element: Neutral
"#;

        #[derive(serde::Deserialize)]
        struct Wrapper {
            weapons: std::collections::BTreeMap<String, CatalogEquipment>,
            armor: std::collections::BTreeMap<String, CatalogEquipment>,
            accessories: std::collections::BTreeMap<String, CatalogEquipment>,
        }
        let w: Wrapper = serde_yaml::from_str(yaml).unwrap();
        EquipmentCatalog {
            weapons: w.weapons,
            armor: w.armor,
            accessories: w.accessories,
        }
    }

    /// Load the default `planner_config.yaml` checked into the repo. This is
    /// the config the planner runs with in production; tests that exercise
    /// it directly catch regressions when the YAML changes.
    fn default_config() -> PlannerConfig {
        let rules_yaml = include_str!("../../../data/planner_config.yaml");
        let file: PlannerConfigFile = serde_yaml::from_str(rules_yaml).unwrap();
        PlannerConfig::new(file, BTreeMap::new())
    }

    fn config_with_special(name: &str, yaml: &str) -> PlannerConfig {
        let rules_yaml = include_str!("../../../data/planner_config.yaml");
        let file: PlannerConfigFile = serde_yaml::from_str(rules_yaml).unwrap();
        let info: itrtg_models::planner_config::PetSpecialInfo =
            serde_yaml::from_str(yaml).unwrap();
        let mut map = BTreeMap::new();
        map.insert(name.to_string(), info);
        PlannerConfig::new(file, map)
    }

    #[test]
    fn test_assassin_d2_knives() {
        let cat = test_catalog();
        let cfg = default_config();
        let s = recommend_equipment(
            Class::Assassin, Element::Fire, Dungeon::Scrapyard, 2, &cat, &cfg,
        );
        assert_eq!(s.source, EquipmentSource::Computed);
        assert_eq!(s.equipment.weapon.as_deref(), Some("thundering_knives"));
        assert_eq!(s.equipment.armor.as_deref(), Some("steel_armor"));
        assert_eq!(s.equipment.accessory.as_deref(), Some("flame_gloves"));
        let gems = s.equipment.gems.unwrap();
        assert_eq!(gems.weapon, Some(Element::Fire));
        assert_eq!(gems.armor, None);
    }

    #[test]
    fn test_rogue_d2_knives() {
        let cat = test_catalog();
        let cfg = default_config();
        let s = recommend_equipment(
            Class::Rogue, Element::Wind, Dungeon::Scrapyard, 2, &cat, &cfg,
        );
        assert_eq!(s.equipment.weapon.as_deref(), Some("thundering_knives"));
        assert_eq!(s.equipment.accessory.as_deref(), Some("storm_ring"));
        let gems = s.equipment.gems.unwrap();
        assert_eq!(gems.weapon, Some(Element::Wind));
    }

    #[test]
    fn test_defender_always_neutral() {
        let cat = test_catalog();
        let cfg = default_config();
        let s = recommend_equipment(
            Class::Defender, Element::Earth, Dungeon::WaterTemple, 2, &cat, &cfg,
        );
        assert_eq!(s.equipment.weapon.as_deref(), Some("steel_sword"));
        assert_eq!(s.equipment.armor.as_deref(), Some("steel_armor"));
        assert_eq!(s.equipment.accessory.as_deref(), Some("steel_ring"));
        let gems = s.equipment.gems.unwrap();
        assert_eq!(gems.weapon, Some(Element::Water));
    }

    #[test]
    fn test_mage_d3_own_element_armor() {
        let cat = test_catalog();
        let cfg = default_config();
        // At D3 the wiki says "match the pet's element" — earth mage gets
        // earth armor and earth accessory (alchemist cape chain falls back
        // to pet element if cape isn't in catalog).
        let s = recommend_equipment(
            Class::Mage, Element::Earth, Dungeon::WaterTemple, 3, &cat, &cfg,
        );
        assert_eq!(s.equipment.armor.as_deref(), Some("forest_armor"));
        // Accessory uses alchemist cape chain (cape is in the test catalog).
        assert_eq!(s.equipment.accessory.as_deref(), Some("alchemist_cape"));
    }

    #[test]
    fn test_mage_d2_uses_dungeon_element_armor_for_survivability() {
        let cat = test_catalog();
        let cfg = default_config();
        // The D2 override swaps mage armor from pet_element to
        // dungeon_element. A fire mage in the Water Temple should therefore
        // get water armor (defensive) rather than fire armor (offensive).
        let s = recommend_equipment(
            Class::Mage, Element::Fire, Dungeon::WaterTemple, 2, &cat, &cfg,
        );
        assert_eq!(s.equipment.armor.as_deref(), Some("flood_armor"));
    }

    #[test]
    fn test_mage_d2_scrapyard_falls_back_to_neutral_armor() {
        let cat = test_catalog();
        let cfg = default_config();
        // Scrapyard is a Neutral dungeon, so DungeonElement falls back to
        // the rule's neutral fallback.
        let s = recommend_equipment(
            Class::Mage, Element::Fire, Dungeon::Scrapyard, 2, &cat, &cfg,
        );
        assert_eq!(s.equipment.armor.as_deref(), Some("steel_armor"));
    }

    #[test]
    fn test_mage_d3_alchemist_cape() {
        let cat = test_catalog();
        let cfg = default_config();
        let s = recommend_equipment(
            Class::Mage, Element::Fire, Dungeon::Scrapyard, 3, &cat, &cfg,
        );
        assert_eq!(s.equipment.weapon.as_deref(), Some("inferno_sword"));
        assert_eq!(s.equipment.armor.as_deref(), Some("inferno_armor"));
        assert_eq!(s.equipment.accessory.as_deref(), Some("alchemist_cape"));
        let gems = s.equipment.gems.unwrap();
        assert_eq!(gems.weapon, Some(Element::Fire));
        assert_eq!(gems.armor, Some(Element::Water));
        assert_eq!(gems.accessory, Some(Element::Neutral));
    }

    #[test]
    fn test_supporter_d2_fire_gloves() {
        let cat = test_catalog();
        let cfg = default_config();
        let s = recommend_equipment(
            Class::Supporter, Element::Water, Dungeon::Forest, 2, &cat, &cfg,
        );
        assert_eq!(s.equipment.weapon.as_deref(), Some("flame_sword"));
        assert_eq!(s.equipment.accessory.as_deref(), Some("flame_gloves"));
    }

    #[test]
    fn test_supporter_d3_alchemist_cape() {
        let cat = test_catalog();
        let cfg = default_config();
        let s = recommend_equipment(
            Class::Supporter, Element::Water, Dungeon::Forest, 3, &cat, &cfg,
        );
        assert_eq!(s.equipment.weapon.as_deref(), Some("inferno_sword"));
        assert_eq!(s.equipment.accessory.as_deref(), Some("alchemist_cape"));
        let gems = s.equipment.gems.unwrap();
        assert_eq!(gems.weapon, Some(Element::Fire));
        assert_eq!(gems.armor, Some(Element::Water));
        assert_eq!(gems.accessory, Some(Element::Neutral));
    }

    #[test]
    fn test_d1_no_gems() {
        let cat = test_catalog();
        let cfg = default_config();
        let s = recommend_equipment(
            Class::Assassin, Element::Fire, Dungeon::Scrapyard, 1, &cat, &cfg,
        );
        assert!(s.equipment.gems.is_none());
    }

    #[test]
    fn test_d3_all_gems() {
        let cat = test_catalog();
        let cfg = default_config();
        let s = recommend_equipment(
            Class::Assassin, Element::Fire, Dungeon::Scrapyard, 3, &cat, &cfg,
        );
        let gems = s.equipment.gems.unwrap();
        assert!(gems.weapon.is_some());
        assert!(gems.armor.is_some());
        assert!(gems.accessory.is_some());
    }

    #[test]
    fn test_blacksmith_d1_hammer_d2_knives() {
        let cat = test_catalog();
        let cfg = default_config();
        let s1 = recommend_equipment(
            Class::Blacksmith, Element::Fire, Dungeon::Volcano, 1, &cat, &cfg,
        );
        assert_eq!(s1.equipment.weapon.as_deref(), Some("forging_hammer"));
        let s2 = recommend_equipment(
            Class::Blacksmith, Element::Fire, Dungeon::Volcano, 2, &cat, &cfg,
        );
        assert_eq!(s2.equipment.weapon.as_deref(), Some("thundering_knives"));
    }

    #[test]
    fn test_assassin_d3_gems_fire_neutral_water() {
        let cat = test_catalog();
        let cfg = default_config();
        let s = recommend_equipment(
            Class::Assassin, Element::Fire, Dungeon::Scrapyard, 3, &cat, &cfg,
        );
        let gems = s.equipment.gems.unwrap();
        assert_eq!(gems.weapon, Some(Element::Fire));
        assert_eq!(gems.armor, Some(Element::Neutral));
        assert_eq!(gems.accessory, Some(Element::Water));
    }

    // -------- pet_special_info integration --------

    fn mock_pet(name: &str, element: Element, class: Option<Class>) -> MergedPet {
        let wiki = WikiPet {
            name: name.to_string(),
            wiki_url: String::new(),
            element,
            recommended_class: RecommendedClass::Wildcard,
            class_bonus: String::new(),
            unlock_condition: UnlockCondition::PetToken,
            evo_difficulty: EvoDifficulty { base: 1, with_conditions: 1 },
            token_improvable: false,
            special_ability: None,
        };
        let export = ExportPet {
            export_name: name.to_string(),
            element,
            growth: 10000,
            dungeon_level: 20,
            class,
            class_level: 10,
            combat_stats: CombatStats {
                hp: 500, attack: 200, defense: 100, speed: 150,
            },
            elemental_affinities: ElementalAffinities {
                water: 0, fire: 0, wind: 0, earth: 0, dark: 0, light: 0,
            },
            loadout: Loadout { weapon: None, armor: None, accessory: None },
            action: PetAction::Idle,
            unlocked: true,
            improved: false,
            other: None,
            has_partner: false,
        };
        MergedPet {
            name: name.to_string(),
            wiki: Some(wiki),
            export: Some(export),
        }
    }

    #[test]
    fn test_archer_requires_bow() {
        let cat = test_catalog();
        let cfg = config_with_special(
            "Archer",
            "equipment_constraints:\n  - required_weapon_type: bow\n",
        );
        let archer = mock_pet("Archer", Element::Wind, Some(Class::Assassin));
        let s = recommend_for_pet(
            &archer, Class::Assassin, Element::Wind, Dungeon::Forest, 2, &cat, &cfg,
        );
        // Should get a bow instead of the default assassin knives.
        assert_eq!(s.equipment.weapon.as_deref(), Some("feather_bow"));
    }

    #[test]
    fn test_ghost_forbidden_knives_fallback_to_sword() {
        let cat = test_catalog();
        let cfg = config_with_special(
            "Ghost",
            "equipment_constraints:\n  - forbidden_weapon_type: knives\n",
        );
        let ghost = mock_pet("Ghost", Element::Fire, Some(Class::Assassin));
        let s = recommend_for_pet(
            &ghost, Class::Assassin, Element::Fire, Dungeon::Scrapyard, 2, &cat, &cfg,
        );
        // Default assassin weapon is knives — Ghost should get a neutral
        // sword instead.
        assert_eq!(s.equipment.weapon.as_deref(), Some("steel_sword"));
    }

    #[test]
    fn test_sylph_priority_element_override() {
        let cat = test_catalog();
        let cfg = config_with_special(
            "Sylph",
            "element_scaling:\n  - element: wind\n    priority_override: true\n",
        );
        // Sylph is a wind pet; the override is redundant here but it still
        // proves the wiring — the accessory should come out as wind instead
        // of whatever the pet_element selector would normally pick.
        let sylph = mock_pet("Sylph", Element::Neutral, Some(Class::Mage));
        let s = recommend_for_pet(
            &sylph, Class::Mage, Element::Neutral, Dungeon::Mountain, 2, &cat, &cfg,
        );
        // Neutral sylph at D2 would normally hit the D2 mage accessory rule
        // (pet_element with Neutral fallback). With the override, the
        // priority element (wind) takes over.
        assert_eq!(s.equipment.accessory.as_deref(), Some("storm_ring"));
    }
}
