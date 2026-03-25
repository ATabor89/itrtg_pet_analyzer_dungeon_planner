use std::collections::BTreeMap;

use serde::de::IntoDeserializer;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{Class, Dungeon, Element, EquipmentSlot};

// =============================================================================
// Top-level structure
// =============================================================================

/// All dungeon data: equipment catalog + dungeon configurations + items.
///
/// Equipment lives in `equipment_catalog.yaml` and dungeon/item data lives in
/// `dungeon_recommendations.yaml`.  Construct this by deserializing each file
/// separately and combining with [`DungeonRecommendations::new`].
#[derive(Debug, Clone, Serialize)]
pub struct DungeonRecommendations {
    pub equipment: EquipmentCatalog,
    pub items: ItemCatalog,
    pub dungeons: BTreeMap<Dungeon, DungeonData>,
}

/// The dungeon recommendations YAML file (items + dungeons, no equipment).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DungeonRecommendationsFile {
    pub items: ItemCatalog,
    pub dungeons: BTreeMap<Dungeon, DungeonData>,
}

impl DungeonRecommendations {
    /// Assemble from a separately-loaded equipment catalog and dungeon file.
    pub fn new(equipment: EquipmentCatalog, file: DungeonRecommendationsFile) -> Self {
        Self {
            equipment,
            items: file.items,
            dungeons: file.dungeons,
        }
    }
}

// =============================================================================
// Equipment Catalog
// =============================================================================

/// All equipment organized by slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentCatalog {
    pub weapons: BTreeMap<String, CatalogEquipment>,
    pub armor: BTreeMap<String, CatalogEquipment>,
    pub accessories: BTreeMap<String, CatalogEquipment>,
}

impl EquipmentCatalog {
    /// Look up an equipment entry by its catalog key, searching all slots.
    pub fn lookup(&self, key: &str) -> Option<&CatalogEquipment> {
        self.weapons
            .get(key)
            .or_else(|| self.armor.get(key))
            .or_else(|| self.accessories.get(key))
    }

    /// Find equipment by slot type, element, and tier.
    pub fn find(
        &self,
        slot: EquipmentSlot,
        element: Element,
        tier: u8,
    ) -> Option<(&str, &CatalogEquipment)> {
        self.slot_map(slot)
            .iter()
            .find(|(_, eq)| eq.tier == tier && eq.element == Some(element))
            .map(|(k, v)| (k.as_str(), v))
    }

    /// Find equipment by slot type, element, tier, and a name substring.
    /// Useful for distinguishing weapon types like "knives", "sword", "pot".
    pub fn find_by_kind(
        &self,
        slot: EquipmentSlot,
        element: Element,
        tier: u8,
        name_contains: &str,
    ) -> Option<(&str, &CatalogEquipment)> {
        let lower = name_contains.to_lowercase();
        self.slot_map(slot)
            .iter()
            .find(|(_, eq)| {
                eq.tier == tier
                    && eq.element == Some(element)
                    && eq.name.to_lowercase().contains(&lower)
            })
            .map(|(k, v)| (k.as_str(), v))
    }

    /// Find equipment by slot type, tier, and name substring (ignoring element).
    /// Used for unique items like "Alchemist Cape".
    pub fn find_by_name(
        &self,
        slot: EquipmentSlot,
        tier: u8,
        name_contains: &str,
    ) -> Option<(&str, &CatalogEquipment)> {
        let lower = name_contains.to_lowercase();
        self.slot_map(slot)
            .iter()
            .find(|(_, eq)| eq.tier == tier && eq.name.to_lowercase().contains(&lower))
            .map(|(k, v)| (k.as_str(), v))
    }

    /// Find a catalog key by exact equipment name (case-insensitive), searching all slots.
    pub fn find_key_by_name_exact(&self, name: &str) -> Option<&str> {
        let lower = name.to_lowercase();
        for map in [&self.weapons, &self.armor, &self.accessories] {
            if let Some((k, _)) = map.iter().find(|(_, eq)| eq.name.to_lowercase() == lower) {
                return Some(k.as_str());
            }
        }
        None
    }

    /// Check if `candidate_key` is the same as `base_key` or is an upgraded version
    /// of it (walks the `upgraded_from` chain from the candidate back toward the base).
    pub fn is_upgrade_of(&self, candidate_key: &str, base_key: &str) -> bool {
        if candidate_key == base_key {
            return true;
        }
        // Walk the candidate's upgrade chain backward
        let mut current = candidate_key;
        // Safety limit to prevent infinite loops from bad data
        for _ in 0..10 {
            let Some(entry) = self.lookup(current) else {
                return false;
            };
            let Some(parent) = &entry.upgraded_from else {
                return false;
            };
            if parent == base_key {
                return true;
            }
            current = parent;
        }
        false
    }

    /// Check if `candidate_key` is in the same upgrade chain as `base_key`
    /// (either an upgrade or a downgrade — checks both directions).
    pub fn is_same_line(&self, candidate_key: &str, base_key: &str) -> bool {
        self.is_upgrade_of(candidate_key, base_key)
            || self.is_upgrade_of(base_key, candidate_key)
    }

    fn slot_map(&self, slot: EquipmentSlot) -> &BTreeMap<String, CatalogEquipment> {
        match slot {
            EquipmentSlot::Weapon => &self.weapons,
            EquipmentSlot::Armor => &self.armor,
            EquipmentSlot::Accessory => &self.accessories,
        }
    }
}

/// A piece of equipment as defined in the catalog (not a player's actual gear).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEquipment {
    pub name: String,
    #[serde(rename = "type")]
    pub slot: EquipmentSlot,
    pub tier: u8,
    /// Equipment element. Use `null`/`~`/omit for elementless items, or `All` for omni-element.
    /// Also accepts `None` as a YAML string (treated as no element).
    #[serde(default, deserialize_with = "deserialize_element_or_none")]
    pub element: Option<Element>,
    pub notes: Option<String>,
    /// Catalog key of the lower-tier equipment this is upgraded from.
    #[serde(default)]
    pub upgraded_from: Option<String>,

    // -- Stats at Quality A, +0 --
    /// Base stat percentages at Quality A, +0 (hp, atk, def, spd).
    #[serde(default)]
    pub stats_a0: Option<EquipmentStats>,

    /// Element resistances at Quality A, +0.
    #[serde(default)]
    pub resistances: Option<EquipmentResistances>,

    // -- Special / premium metadata --
    /// Description of any special effect this equipment provides.
    #[serde(default)]
    pub special_effect: Option<String>,

    /// How this equipment is obtained (e.g. "premium_event", "tavern_quest",
    /// "crafted_by_hamster", "patreon_boss_reward", "ultimate_stats_challenge").
    #[serde(default)]
    pub source: Option<String>,

    /// Maximum number obtainable. Omitted for unlimited/craftable equipment.
    #[serde(default)]
    pub max_quantity: Option<u8>,

    /// Whether this equipment should be considered for dungeon team planning.
    /// Defaults to `true` when absent; set to `false` for pure crafting/utility items.
    #[serde(default)]
    pub dungeon_relevant: Option<bool>,
}

impl CatalogEquipment {
    /// Returns `true` if this equipment should be considered by the dungeon planner.
    /// Items with `dungeon_relevant: false` are excluded; everything else is included.
    pub fn is_dungeon_relevant(&self) -> bool {
        self.dungeon_relevant.unwrap_or(true)
    }
}

/// Base stat percentages for a piece of equipment at Quality A, +0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentStats {
    pub hp: i16,
    pub atk: i16,
    pub def: i16,
    pub spd: i16,
}

/// Elemental resistances for a piece of equipment at Quality A, +0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentResistances {
    #[serde(default)]
    pub fire: i16,
    #[serde(default)]
    pub water: i16,
    #[serde(default)]
    pub wind: i16,
    #[serde(default)]
    pub earth: i16,
}

// =============================================================================
// Item Catalog
// =============================================================================

/// All dungeon items, keyed by their catalog ID.
pub type ItemCatalog = BTreeMap<String, CatalogItem>;

/// An item that can be brought into a dungeon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogItem {
    pub name: String,
    pub description: String,
}

// =============================================================================
// Dungeon
// =============================================================================

/// A dungeon with all of its depth configurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DungeonData {
    pub name: String,
    pub depths: BTreeMap<u8, DepthData>,
}

/// Configuration for a single depth level within a dungeon.
///
/// Note: The YAML may use either `boss` (single entry) or `bosses` (list).
/// Both are normalized into `bosses: Vec<MonsterEntry>` during deserialization.
#[derive(Debug, Clone, Serialize)]
pub struct DepthData {
    pub rooms: u16,
    pub monsters_per_room: u8,
    pub gem_level: Option<u16>,

    pub requirements: DepthRequirements,
    pub monsters: Vec<MonsterEntry>,

    /// Bosses for this depth (always a Vec, even for single-boss depths).
    pub bosses: Vec<MonsterEntry>,

    /// Recommended party composition. Ordering matters:
    /// positions 1-3 are front row, 4-6 are back row.
    pub party: Vec<PartySlot>,

    /// Items to bring into the dungeon.
    pub party_items: Vec<PartyItemEntry>,

    pub traps: Vec<TrapEntry>,
    pub events: Vec<EventEntry>,
}

/// Intermediate type that handles `boss` vs `bosses` in YAML.
#[derive(Deserialize)]
struct DepthDataRaw {
    rooms: u16,
    monsters_per_room: u8,
    gem_level: Option<u16>,
    requirements: DepthRequirements,
    monsters: Vec<MonsterEntry>,
    boss: Option<MonsterEntry>,
    bosses: Option<Vec<MonsterEntry>>,
    party: Vec<PartySlot>,
    party_items: Vec<PartyItemEntry>,
    traps: Vec<TrapEntry>,
    events: Vec<EventEntry>,
}

impl<'de> Deserialize<'de> for DepthData {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = DepthDataRaw::deserialize(deserializer)?;

        let bosses = match (raw.boss, raw.bosses) {
            (Some(b), None) => vec![b],
            (None, Some(v)) => v,
            (Some(_), Some(_)) => {
                return Err(serde::de::Error::custom(
                    "depth has both 'boss' and 'bosses' — use one or the other",
                ))
            }
            (None, None) => {
                return Err(serde::de::Error::custom(
                    "depth must have either 'boss' or 'bosses'",
                ))
            }
        };

        Ok(DepthData {
            rooms: raw.rooms,
            monsters_per_room: raw.monsters_per_room,
            gem_level: raw.gem_level,
            requirements: raw.requirements,
            monsters: raw.monsters,
            bosses,
            party: raw.party,
            party_items: raw.party_items,
            traps: raw.traps,
            events: raw.events,
        })
    }
}

/// Stat/level requirements to enter a depth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthRequirements {
    pub dungeon_level_avg: u32,
    /// [easy, hard] — how many dungeon levels each difficulty point translates to.
    pub levels_per_difficulty: Vec<u32>,
    pub class_level: u32,
    pub total_growth: Option<u64>,
}

// =============================================================================
// Monsters
// =============================================================================

/// A monster or boss entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterEntry {
    pub name: String,
    pub element: Option<Element>,
    pub hp: u64,
    pub att: u64,
    pub def: u64,
    pub spd: u64,
}

// =============================================================================
// Party
// =============================================================================

/// A single slot in the recommended party composition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartySlot {
    /// Required class. None means "any".
    #[serde(deserialize_with = "deserialize_class_or_any")]
    pub class: Option<Class>,

    /// Required element. None means "any".
    #[serde(deserialize_with = "deserialize_element_or_any")]
    pub element: Option<Element>,

    /// Equipment recommendation. None for D1 (or "any/none").
    #[serde(default)]
    pub equipment: Option<PartyEquipment>,
}

/// Equipment loadout for a party slot, referencing catalog keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyEquipment {
    /// Catalog key for the weapon (e.g. "flame_sword", "generic_t2_s10").
    pub weapon: Option<String>,
    /// Catalog key for the armor.
    pub armor: Option<String>,
    /// Catalog key for the accessory.
    pub accessory: Option<String>,
    /// Gem recommendations per slot.
    #[serde(default)]
    pub gems: Option<GemSlots>,
}

/// Gem element recommendations for each equipment slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GemSlots {
    pub weapon: Option<Element>,
    pub armor: Option<Element>,
    pub accessory: Option<Element>,
}

// =============================================================================
// Items
// =============================================================================

/// An item + quantity recommendation for a dungeon run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyItemEntry {
    /// Catalog key (e.g. "torch", "holy_water").
    pub item: String,
    pub quantity: u32,
}

// =============================================================================
// Traps & Events
// =============================================================================

/// A trap that can appear in a dungeon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrapEntry {
    pub name: String,
    pub chance_pct: u8,
    pub countered_by: CounterCondition,
}

/// An event that can trigger in a dungeon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEntry {
    pub name: String,
    pub chance_pct: u8,
    /// Counter conditions. The YAML can express this as either a single object
    /// (all fields are AND'd) or a list of objects (each entry is a separate
    /// requirement, all of which must be met).
    #[serde(deserialize_with = "deserialize_counter")]
    pub countered_by: Vec<CounterCondition>,
}

/// A single counter condition. When multiple fields are present, they are all
/// required simultaneously (AND). For example, `class: Rogue` + `item: holy_water`
/// means you need both a Rogue AND holy water.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterCondition {
    /// Counter item (catalog key).
    pub item: Option<String>,
    /// Counter class.
    pub class: Option<Class>,
    /// Counter element.
    pub element: Option<Element>,
    /// How many pets must satisfy this condition (e.g. 2 Wind-element pets).
    pub count: Option<u32>,
    /// Per-clear quantity for consumable events (e.g. Wild Animals pet food).
    pub quantity_per_clear: Option<u32>,
    /// Additional notes.
    pub notes: Option<String>,
}

// =============================================================================
// Custom Deserialization Helpers
// =============================================================================

/// Deserialize "any" as None, otherwise parse as the target enum type using
/// serde's built-in PascalCase variant matching.
fn deserialize_or_any<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s == "any" {
        return Ok(None);
    }
    T::deserialize(s.into_deserializer()).map(Some)
}

/// Deserialize element for catalog equipment: handles null, "None" string, and normal variants.
/// "None" (the string) is treated as no element, same as null/omitted.
fn deserialize_element_or_none<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Element>, D::Error> {
    // Use an intermediate that can be null or a string
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(s) if s == "None" => Ok(None),
        Some(s) => Element::deserialize(s.into_deserializer()).map(Some),
    }
}

/// Deserialize "any" as None, otherwise parse as Class.
fn deserialize_class_or_any<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Class>, D::Error> {
    deserialize_or_any(deserializer)
}

/// Deserialize "any" as None, otherwise parse as Element.
fn deserialize_element_or_any<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Element>, D::Error> {
    deserialize_or_any(deserializer)
}

/// Deserialize the `countered_by` field for events, which can be either a single
/// object or a list of objects.
fn deserialize_counter<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<CounterCondition>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(CounterCondition),
        Many(Vec<CounterCondition>),
    }

    match OneOrMany::deserialize(deserializer)? {
        OneOrMany::One(c) => Ok(vec![c]),
        OneOrMany::Many(v) => Ok(v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_class_or_any() {
        #[derive(Deserialize)]
        struct Test {
            #[serde(deserialize_with = "deserialize_class_or_any")]
            class: Option<Class>,
        }

        let t: Test = serde_yaml::from_str("class: Assassin").unwrap();
        assert_eq!(t.class, Some(Class::Assassin));

        let t: Test = serde_yaml::from_str("class: any").unwrap();
        assert_eq!(t.class, None);
    }

    #[test]
    fn test_parse_element_or_any() {
        #[derive(Deserialize)]
        struct Test {
            #[serde(deserialize_with = "deserialize_element_or_any")]
            element: Option<Element>,
        }

        let t: Test = serde_yaml::from_str("element: Fire").unwrap();
        assert_eq!(t.element, Some(Element::Fire));

        let t: Test = serde_yaml::from_str("element: any").unwrap();
        assert_eq!(t.element, None);
    }

    #[test]
    fn test_parse_counter_single() {
        let yaml = r#"
            name: "Fog"
            chance_pct: 13
            countered_by:
              item: torch
        "#;
        let ev: EventEntry = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(ev.countered_by.len(), 1);
        assert_eq!(ev.countered_by[0].item, Some("torch".to_string()));
    }

    #[test]
    fn test_parse_counter_multi() {
        let yaml = r#"
            name: "Portal From Beyond"
            chance_pct: 17
            countered_by:
              - class: Mage
              - element: Neutral
                count: 2
        "#;
        let ev: EventEntry = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(ev.countered_by.len(), 2);
        assert_eq!(ev.countered_by[0].class, Some(Class::Mage));
        assert_eq!(ev.countered_by[1].element, Some(Element::Neutral));
        assert_eq!(ev.countered_by[1].count, Some(2));
    }

    #[test]
    fn test_parse_counter_class_and_item() {
        let yaml = r#"
            name: "Cursed Chest"
            chance_pct: 5
            countered_by:
              class: Rogue
              item: holy_water
        "#;
        let ev: EventEntry = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(ev.countered_by.len(), 1);
        assert_eq!(ev.countered_by[0].class, Some(Class::Rogue));
        assert_eq!(ev.countered_by[0].item, Some("holy_water".to_string()));
    }

    #[test]
    fn test_parse_counter_element_count() {
        let yaml = r#"
            name: "Deep Sea Treasure"
            chance_pct: 10
            countered_by:
              element: Water
              count: 3
        "#;
        let ev: EventEntry = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(ev.countered_by.len(), 1);
        assert_eq!(ev.countered_by[0].element, Some(Element::Water));
        assert_eq!(ev.countered_by[0].count, Some(3));
    }
}
