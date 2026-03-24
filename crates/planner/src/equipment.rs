//! Equipment recommendation engine.
//!
//! Computes concrete equipment suggestions for dungeon party slots that have
//! generic or missing gear, based on the pet's class, element, and the dungeon
//! context. Recommendations are derived from observed in-game patterns.

use itrtg_models::dungeon::*;
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
pub fn enrich_equipment(plan: &mut DungeonPlan, catalog: &EquipmentCatalog) {
    let dungeon = plan.dungeon;
    let depth = plan.depth;

    for sa in &mut plan.assignments {
        let is_generic = sa.slot.equipment.as_ref().is_some_and(|eq| has_generic_keys(eq));
        let is_missing = sa.slot.equipment.is_none();

        if let Some(equip) = &sa.slot.equipment {
            if !is_generic {
                // Static equipment from YAML — tag and preserve
                sa.equipment_suggestion = Some(EquipmentSuggestion {
                    equipment: equip.clone(),
                    source: EquipmentSource::Static,
                });
                continue;
            }
        }

        // Need to compute: get the pet's effective class
        let Assignment::Filled { pet, .. } = &sa.assignment else {
            continue; // No pet assigned — can't recommend equipment
        };

        let class = resolve_effective_class(pet, &sa.slot);
        let pet_element = pet.element().unwrap_or(Element::Neutral);

        if let Some(class) = class {
            let suggestion = recommend_equipment(class, pet_element, dungeon, depth, catalog);
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

/// Recommend equipment for a pet in a specific dungeon context.
pub fn recommend_equipment(
    class: Class,
    pet_element: Element,
    dungeon: Dungeon,
    depth: u8,
    catalog: &EquipmentCatalog,
) -> EquipmentSuggestion {
    let tier = depth.min(3).max(1);
    let dungeon_element = dungeon.element();

    let weapon_key = recommend_weapon(class, tier, catalog);
    let armor_key = recommend_armor(class, pet_element, dungeon_element, tier, catalog);
    let accessory_key = recommend_accessory(class, pet_element, tier, catalog);
    let gems = recommend_gems(class, depth);

    EquipmentSuggestion {
        equipment: PartyEquipment {
            weapon: weapon_key.map(|s| s.to_string()),
            armor: armor_key.map(|s| s.to_string()),
            accessory: accessory_key.map(|s| s.to_string()),
            gems,
        },
        source: EquipmentSource::Computed,
    }
}

// =============================================================================
// Weapon rules
// =============================================================================

/// Recommend a weapon catalog key.
///
/// Sources: Wiki "Dungeon Equipment Recommendations", "Introduction to Dungeons"
///
/// - Mage/Supporter: Fire sword (attack → damage / healing power)
/// - Assassin: Knives (defense reduction is important for assassins)
/// - Defender: Neutral sword (balanced stats, pure tank)
/// - Rogue: Knives (always — speed + defense reduction)
/// - Blacksmith: Knives at D2+ (useful for v4 fights), hammer at D1
fn recommend_weapon<'a>(
    class: Class,
    tier: u8,
    catalog: &'a EquipmentCatalog,
) -> Option<&'a str> {
    match class {
        Class::Mage | Class::Supporter => {
            // Fire sword for attack (healing scales with attack for supporters)
            catalog
                .find_by_kind(EquipmentSlot::Weapon, Element::Fire, tier, "sword")
                .map(|(k, _)| k)
        }
        Class::Assassin => {
            // Knives — "defense reduction is important, especially as knives get higher tier"
            catalog
                .find_by_kind(EquipmentSlot::Weapon, Element::Wind, tier, "knives")
                .map(|(k, _)| k)
        }
        Class::Defender => {
            // Neutral sword for balanced defense
            catalog
                .find_by_kind(EquipmentSlot::Weapon, Element::Neutral, tier, "sword")
                .map(|(k, _)| k)
        }
        Class::Rogue => {
            // Knives — always for rogues
            catalog
                .find_by_kind(EquipmentSlot::Weapon, Element::Wind, tier, "knives")
                .map(|(k, _)| k)
        }
        Class::Blacksmith => {
            if tier >= 2 {
                // Knives at D2+ ("very useful for v4 fights")
                catalog
                    .find_by_kind(EquipmentSlot::Weapon, Element::Wind, tier, "knives")
                    .map(|(k, _)| k)
            } else {
                // Hammer at D1
                catalog
                    .find_by_kind(EquipmentSlot::Weapon, Element::Fire, tier, "hammer")
                    .map(|(k, _)| k)
            }
        }
        _ => {
            // Adventurer/Alchemist/Wildcard — neutral sword fallback
            catalog
                .find_by_kind(EquipmentSlot::Weapon, Element::Neutral, tier, "sword")
                .map(|(k, _)| k)
        }
    }
}

// =============================================================================
// Armor rules
// =============================================================================

/// Recommend an armor catalog key.
///
/// Sources: Wiki "Dungeon Equipment Recommendations"
///
/// - Defender: Always neutral (pure tank stats)
/// - Assassin: Neutral (defensive, covering elemental weakness)
/// - Mage: Pet's own element ("Match the pets element" — D3 wiki)
/// - Supporter: Dungeon-element (defensive) or neutral for Scrapyard
/// - Rogue: Dungeon-element (defense) or neutral for Scrapyard
fn recommend_armor<'a>(
    class: Class,
    pet_element: Element,
    dungeon_element: Element,
    tier: u8,
    catalog: &'a EquipmentCatalog,
) -> Option<&'a str> {
    let element = match class {
        Class::Defender | Class::Assassin => Element::Neutral,
        Class::Mage => {
            // Mages: match pet's own element for offensive scaling
            // Fallback to neutral for Neutral/All element pets
            match pet_element {
                Element::Fire | Element::Water | Element::Wind | Element::Earth => pet_element,
                _ => Element::Neutral,
            }
        }
        Class::Supporter | Class::Rogue | Class::Blacksmith => {
            // Dungeon element for defense; neutral for Scrapyard/Newbie
            if dungeon_element == Element::Neutral {
                Element::Neutral
            } else {
                dungeon_element
            }
        }
        _ => Element::Neutral,
    };

    catalog
        .find(EquipmentSlot::Armor, element, tier)
        .map(|(k, _)| k)
}

// =============================================================================
// Accessory rules
// =============================================================================

/// Recommend an accessory catalog key.
///
/// Sources: Wiki "Dungeon Equipment Recommendations"
///
/// - Assassin: Fire gloves ("Almost always Inferno")
/// - Mage: Alchemist Cape at T3 ("Usually Alchemist Cape"), same-element at lower tiers
/// - Defender: Neutral ring (always)
/// - Supporter: Fire gloves ("Inferno Gloves are a good choice — speed + healing power");
///              Alchemist Cape at T3
/// - Rogue: Wind ring for speed; Alchemist Cape at T3
fn recommend_accessory<'a>(
    class: Class,
    pet_element: Element,
    tier: u8,
    catalog: &'a EquipmentCatalog,
) -> Option<&'a str> {
    match class {
        Class::Assassin => {
            // "Almost always Inferno [Gloves]"
            catalog
                .find_by_kind(EquipmentSlot::Accessory, Element::Fire, tier, "gloves")
                .map(|(k, _)| k)
        }
        Class::Mage => {
            if tier >= 3 {
                // "Usually Alchemist Cape" at T3
                catalog
                    .find_by_name(EquipmentSlot::Accessory, tier, "alchemist")
                    .or_else(|| {
                        // Fallback: same-element accessory
                        let el = match pet_element {
                            Element::Fire | Element::Water | Element::Wind | Element::Earth => pet_element,
                            _ => Element::Neutral,
                        };
                        catalog.find(EquipmentSlot::Accessory, el, tier)
                    })
                    .map(|(k, _)| k)
            } else {
                // Same-element accessory at T1-T2 for elemental scaling
                let el = match pet_element {
                    Element::Fire | Element::Water | Element::Wind | Element::Earth => pet_element,
                    _ => Element::Neutral,
                };
                catalog
                    .find(EquipmentSlot::Accessory, el, tier)
                    .map(|(k, _)| k)
            }
        }
        Class::Defender => {
            // Neutral ring always
            catalog
                .find_by_kind(EquipmentSlot::Accessory, Element::Neutral, tier, "ring")
                .map(|(k, _)| k)
        }
        Class::Supporter => {
            if tier >= 3 {
                // Alchemist Cape at T3
                catalog
                    .find_by_name(EquipmentSlot::Accessory, tier, "alchemist")
                    .or_else(|| {
                        catalog.find_by_kind(
                            EquipmentSlot::Accessory, Element::Fire, tier, "gloves",
                        )
                    })
                    .map(|(k, _)| k)
            } else {
                // "Inferno Gloves are a good choice — speed + healing power"
                catalog
                    .find_by_kind(EquipmentSlot::Accessory, Element::Fire, tier, "gloves")
                    .map(|(k, _)| k)
            }
        }
        Class::Rogue => {
            if tier >= 3 {
                // Alchemist Cape at T3
                catalog
                    .find_by_name(EquipmentSlot::Accessory, tier, "alchemist")
                    .or_else(|| {
                        catalog.find_by_kind(
                            EquipmentSlot::Accessory, Element::Wind, tier, "ring",
                        )
                    })
                    .map(|(k, _)| k)
            } else {
                // Wind ring for speed
                catalog
                    .find_by_kind(EquipmentSlot::Accessory, Element::Wind, tier, "ring")
                    .map(|(k, _)| k)
            }
        }
        _ => {
            // Neutral ring fallback
            catalog
                .find_by_kind(EquipmentSlot::Accessory, Element::Neutral, tier, "ring")
                .map(|(k, _)| k)
        }
    }
}

// =============================================================================
// Gem rules
// =============================================================================

/// Recommend gems based on class and depth.
///
/// - D1: No gems.
/// - D2: Weapon gem only.
/// - D3: All slots.
///
/// Gem effects:
/// - Fire: attack    - Water: HP
/// - Earth: defense  - Wind: speed
/// - Neutral: all element stats
///
/// D3 gem guidelines by class (from wiki community consensus):
/// - Most classes: Fire/Water/Neutral as a solid baseline
/// - Mage:       Fire/Water/Neutral ("Fire/Water/Neutral is a good mix")
/// - Assassin:   Fire/Neutral/Water ("Fire/Neutral/Water for gems is a good mix")
/// - Supporter:  Fire/Water/Neutral ("One of each Attack/HP/Neutral")
/// - Defender:   All Water ("just use HP" for dungeons)
/// - Rogue:      Fire/Water/Neutral ("focus on HP, Neutral, or Attack")
/// - Blacksmith: Water/Water/Earth ("mix of HP and Defense" for dungeons)
fn recommend_gems(class: Class, depth: u8) -> Option<GemSlots> {
    if depth < 2 {
        return None;
    }

    // D2: weapon gem only
    let weapon_gem = match class {
        Class::Assassin | Class::Mage | Class::Supporter => Some(Element::Fire),
        Class::Rogue => Some(Element::Wind),
        Class::Defender => Some(Element::Water),
        _ => Some(Element::Fire),
    };

    if depth < 3 {
        return Some(GemSlots {
            weapon: weapon_gem,
            armor: None,
            accessory: None,
        });
    }

    // D3: all slots — wiki community consensus
    match class {
        Class::Mage => Some(GemSlots {
            weapon: Some(Element::Fire),    // attack
            armor: Some(Element::Water),    // HP
            accessory: Some(Element::Neutral), // elements (F/W/N mix)
        }),
        Class::Assassin => Some(GemSlots {
            weapon: Some(Element::Fire),    // attack
            armor: Some(Element::Neutral),  // elements (F/N/W mix)
            accessory: Some(Element::Water), // HP
        }),
        Class::Supporter => Some(GemSlots {
            weapon: Some(Element::Fire),    // attack (healing scales with attack)
            armor: Some(Element::Water),    // HP
            accessory: Some(Element::Neutral), // elements (F/W/N)
        }),
        Class::Defender => Some(GemSlots {
            weapon: Some(Element::Water),   // HP
            armor: Some(Element::Water),    // HP
            accessory: Some(Element::Water), // HP (all Water for dungeons)
        }),
        Class::Rogue => Some(GemSlots {
            weapon: Some(Element::Fire),    // attack
            armor: Some(Element::Water),    // HP
            accessory: Some(Element::Neutral), // elements (F/W/N)
        }),
        Class::Blacksmith => Some(GemSlots {
            weapon: Some(Element::Water),   // HP
            armor: Some(Element::Water),    // HP
            accessory: Some(Element::Earth), // defense (W/W/E for dungeons)
        }),
        _ => Some(GemSlots {
            weapon: Some(Element::Fire),
            armor: Some(Element::Water),
            accessory: Some(Element::Neutral),
        }),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
  flame_armor:
    name: "Flame Armor"
    type: Armor
    tier: 2
    element: Fire
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

    #[test]
    fn test_assassin_d2_knives() {
        let cat = test_catalog();
        let s = recommend_equipment(Class::Assassin, Element::Fire, Dungeon::Scrapyard, 2, &cat);
        assert_eq!(s.source, EquipmentSource::Computed);
        // Assassins use knives (defense reduction), not fire swords
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
        let s = recommend_equipment(Class::Rogue, Element::Wind, Dungeon::Scrapyard, 2, &cat);
        assert_eq!(s.equipment.weapon.as_deref(), Some("thundering_knives"));
        assert_eq!(s.equipment.accessory.as_deref(), Some("storm_ring"));
        let gems = s.equipment.gems.unwrap();
        assert_eq!(gems.weapon, Some(Element::Wind));
    }

    #[test]
    fn test_defender_always_neutral() {
        let cat = test_catalog();
        let s = recommend_equipment(Class::Defender, Element::Earth, Dungeon::WaterTemple, 2, &cat);
        assert_eq!(s.equipment.weapon.as_deref(), Some("steel_sword"));
        assert_eq!(s.equipment.armor.as_deref(), Some("steel_armor"));
        assert_eq!(s.equipment.accessory.as_deref(), Some("steel_ring"));
        let gems = s.equipment.gems.unwrap();
        assert_eq!(gems.weapon, Some(Element::Water));
    }

    #[test]
    fn test_mage_own_element_armor() {
        let cat = test_catalog();
        // Earth mage in Water Temple should get earth armor (own element), not water (dungeon)
        let s = recommend_equipment(Class::Mage, Element::Earth, Dungeon::WaterTemple, 2, &cat);
        assert_eq!(s.equipment.armor.as_deref(), Some("tree_armor"));
        // Earth mage gets earth accessory at T2
        assert_eq!(s.equipment.accessory.as_deref(), Some("tree_bracelet"));
    }

    #[test]
    fn test_mage_d3_alchemist_cape() {
        let cat = test_catalog();
        // At T3, mages get Alchemist Cape
        let s = recommend_equipment(Class::Mage, Element::Fire, Dungeon::Scrapyard, 3, &cat);
        assert_eq!(s.equipment.weapon.as_deref(), Some("inferno_sword"));
        assert_eq!(s.equipment.armor.as_deref(), Some("inferno_armor"));
        assert_eq!(s.equipment.accessory.as_deref(), Some("alchemist_cape"));
        // D3 gems: Fire/Water/Neutral
        let gems = s.equipment.gems.unwrap();
        assert_eq!(gems.weapon, Some(Element::Fire));
        assert_eq!(gems.armor, Some(Element::Water));
        assert_eq!(gems.accessory, Some(Element::Neutral));
    }

    #[test]
    fn test_supporter_d2_fire_gloves() {
        let cat = test_catalog();
        // Supporters get fire gloves (attack → healing power + speed)
        let s = recommend_equipment(Class::Supporter, Element::Water, Dungeon::Forest, 2, &cat);
        assert_eq!(s.equipment.weapon.as_deref(), Some("flame_sword"));
        assert_eq!(s.equipment.accessory.as_deref(), Some("flame_gloves"));
    }

    #[test]
    fn test_supporter_d3_alchemist_cape() {
        let cat = test_catalog();
        let s = recommend_equipment(Class::Supporter, Element::Water, Dungeon::Forest, 3, &cat);
        assert_eq!(s.equipment.weapon.as_deref(), Some("inferno_sword"));
        assert_eq!(s.equipment.accessory.as_deref(), Some("alchemist_cape"));
        // D3 gems: Fire/Water/Neutral
        let gems = s.equipment.gems.unwrap();
        assert_eq!(gems.weapon, Some(Element::Fire));
        assert_eq!(gems.armor, Some(Element::Water));
        assert_eq!(gems.accessory, Some(Element::Neutral));
    }

    #[test]
    fn test_d1_no_gems() {
        let cat = test_catalog();
        let s = recommend_equipment(Class::Assassin, Element::Fire, Dungeon::Scrapyard, 1, &cat);
        assert!(s.equipment.gems.is_none());
    }

    #[test]
    fn test_d3_all_gems() {
        let cat = test_catalog();
        let s = recommend_equipment(Class::Assassin, Element::Fire, Dungeon::Scrapyard, 3, &cat);
        let gems = s.equipment.gems.unwrap();
        assert!(gems.weapon.is_some());
        assert!(gems.armor.is_some());
        assert!(gems.accessory.is_some());
    }

    #[test]
    fn test_blacksmith_d1_hammer_d2_knives() {
        let cat = test_catalog();
        // D1: hammer
        let s1 = recommend_equipment(Class::Blacksmith, Element::Fire, Dungeon::Volcano, 1, &cat);
        assert_eq!(s1.equipment.weapon.as_deref(), Some("forging_hammer"));
        // D2+: knives ("very useful for v4 fights")
        let s2 = recommend_equipment(Class::Blacksmith, Element::Fire, Dungeon::Volcano, 2, &cat);
        assert_eq!(s2.equipment.weapon.as_deref(), Some("thundering_knives"));
    }

    #[test]
    fn test_assassin_d3_gems_fire_neutral_water() {
        let cat = test_catalog();
        let s = recommend_equipment(Class::Assassin, Element::Fire, Dungeon::Scrapyard, 3, &cat);
        let gems = s.equipment.gems.unwrap();
        // Wiki: "Fire/Neutral/Water for gems is a good mix"
        assert_eq!(gems.weapon, Some(Element::Fire));
        assert_eq!(gems.armor, Some(Element::Neutral));
        assert_eq!(gems.accessory, Some(Element::Water));
    }
}
