//! Typed extraction of the identified parts of a save tree.
//!
//! Field meanings come from cross-referencing a save against same-session
//! in-game exports — see `reference/save_file_deserialization/FINDINGS.md`.
//! Unknown fields stay accessible through the raw [`Node`]s kept on
//! [`SaveFile`] and [`SavePet`], which is the main tool for closing the
//! remaining gaps.

use anyhow::Context;
use itrtg_models::{Class, Element};

use crate::tree::Node;

/// Sentinel in the pet `F` field meaning "no partner".
const PARTNER_NONE: u32 = 999;

/// A fully parsed save file: the typed parts we understand, plus the raw
/// tree for everything else.
#[derive(Debug, Clone)]
pub struct SaveFile {
    /// Unix timestamp (seconds) the save was written (root `c`).
    pub saved_at_unix: Option<i64>,
    /// God name (root `s`).
    pub god_name: Option<String>,
    /// Player/account name (root `W`).
    pub player_name: Option<String>,
    /// Pet stones (root `X.y`).
    pub pet_stones: Option<u64>,
    /// Pet food counts (root `X.c`/`X.d`/`X.e`) and chocolate (root `X.v`).
    /// These are dedicated fields, not material-inventory entries. An absent
    /// field reads as 0.
    pub puny_food: u64,
    pub strong_food: u64,
    pub mighty_food: u64,
    pub chocolate: u64,
    /// Gem inventory (root `X.002`).
    pub gems: Vec<GemStack>,
    /// All pets (root `X.b`), in save order.
    pub pets: Vec<SavePet>,
    /// Owned pet equipment instances (root `X.R`).
    pub equipment: Vec<EquipmentItem>,
    /// Material/item stacks (root `X.Q`).
    pub materials: Vec<MaterialStack>,
    /// The three persistent dungeon teams (root `X.S`).
    pub dungeon_teams: Vec<DungeonTeam>,
    /// Campaign slots (root `X.x`).
    pub campaigns: Vec<CampaignSlot>,
    /// The full raw tree, for exploring not-yet-identified fields.
    pub root: Node,
}

/// One pet from the save (`X.b[i]`).
///
/// The save stores the pet's *display* name (e.g. "Rudolph", "Chicken",
/// "Pandora's box"), which differs from the export names for ~24 pets and
/// strips no spaces. Combat stats (HP/Attack/...) are not stored in the
/// save at all — they are derived at runtime by the game.
#[derive(Debug, Clone)]
pub struct SavePet {
    /// Display name (`a`).
    pub name: String,
    /// Internal pet type id (`k`) — the id used by team/campaign lists.
    pub type_id: u32,
    /// Unlocked flag (`l`).
    pub unlocked: bool,
    /// Growth (`E`). Fractional; in-game exports show it rounded.
    pub growth: f64,
    /// Normal level (`g`) — resets at rebirth; drives normal stats.
    pub normal_level: u32,
    /// Current Physical stat (`j` ÷ 10 — the save stores it ×10). Mystic and
    /// Battle are not stored; in-game they differ from Physical only by the
    /// Strategy Room multiplier ratio. See
    /// `reference/save_file_deserialization/normal_stats_investigation.md`.
    pub physical_stat: f64,
    /// Dungeon team slot 1–6 (`v`), `None` when not on a team.
    pub team_slot: Option<u8>,
    /// Element (`w.a`).
    pub element: Option<Element>,
    /// Dungeon level (`w.b`).
    pub dungeon_level: u32,
    /// Dungeon exp (`w.c`). Stored as float — exp accumulators elsewhere in
    /// the save are fractional, so don't assume this stays integral.
    pub dungeon_exp: f64,
    /// Class (`w.d.a`), `None` for classless pets (id 0).
    pub class: Option<Class>,
    /// Class level (`w.d.b`).
    pub class_level: u32,
    /// Class exp (`w.d.c`), float for the same reason as `dungeon_exp`.
    pub class_exp: f64,
    /// Equipment instance ids (`w.e`/`w.f`/`w.g`), `None` when empty (0).
    pub weapon_id: Option<u32>,
    pub armor_id: Option<u32>,
    pub accessory_id: Option<u32>,
    /// Partner pet type id (`F`), `None` when 999. Pairs are mutual
    /// (Cat↔Dog, Vampire↔Succubus, ...). Note id 0 (Mouse) is valid.
    pub partner_type_id: Option<u32>,
    /// Partner-related counter (`G`) — bond level? Only nonzero with partner.
    pub partner_bond: u64,
    /// The pet's raw node, for the still-unidentified fields
    /// (`d,e,f,h,n,o,p,q,r,s,t,u,x,y,z,A–D,H`). Known but derived: `p`/`q`/`r`
    /// are exactly 556×/550×/10× the accumulator `o` (meaning TBD), and `h`
    /// is level/exp-state related.
    pub raw: Node,
}

/// One owned equipment instance (`X.R[i]`).
#[derive(Debug, Clone)]
pub struct EquipmentItem {
    /// Item type id (`a`) — resolve with [`EquipmentItem::type_name`].
    pub type_id: u32,
    /// Upgrade ("+") level (`b`).
    pub plus: u32,
    /// Quality (`c`): observed 8=SSS, 6=S, 4=B (likely 4=B..8=SSS).
    pub quality: u32,
    /// Instance id (`d`, mirrored in `h`) — referenced by pets' equip slots.
    pub instance_id: u32,
    /// `e`: 20 on items whose export shows a "(20)" suffix, else 0.
    pub plus_cap: u32,
    /// Gem level (`f`), 0 = no gem.
    pub gem_level: u32,
    /// Gem element (`g`) when a gem is socketed.
    pub gem_element: Option<Element>,
}

/// A material/item stack (`X.Q[i]`), e.g. id 159 = Strategy Books.
#[derive(Debug, Clone, Copy)]
pub struct MaterialStack {
    pub item_id: u32,
    pub count: u64,
}

impl MaterialStack {
    /// Display name from the known id table, if this id has been identified.
    pub fn name(&self) -> Option<&'static str> {
        crate::items::material_name(self.item_id)
    }
}

/// One gem stack (`X.002[i]`): element id + gem level + count.
/// Uses the same element ids as pets (0=Neutral, 1=Fire, 2=Water, 3=Earth,
/// 4=Wind).
#[derive(Debug, Clone, Copy)]
pub struct GemStack {
    /// Raw element id (`a`), kept so an unrecognized id stays diagnosable.
    pub element_id: u32,
    /// Decoded element, `None` if the id is unknown.
    pub element: Option<Element>,
    pub level: u32,
    pub count: u64,
}

/// One persistent dungeon team (`X.S[i]`).
#[derive(Debug, Clone)]
pub struct DungeonTeam {
    /// Dungeon id (`b`).
    pub dungeon_id: u32,
    /// Depth (`d`).
    pub depth: u32,
    /// Dungeon display name (`i`), possibly truncated ("Water Temp").
    pub dungeon_name: String,
    /// Member pet type ids (`a`); slot order lives on each pet (`team_slot`).
    pub pet_type_ids: Vec<u32>,
    /// Pending loot as `(item id, count)` pairs (`c`).
    pub loot: Vec<(u32, u64)>,
}

/// One campaign slot (`X.x[i]`).
#[derive(Debug, Clone)]
pub struct CampaignSlot {
    /// Slot index (`a`).
    pub index: u32,
    /// Pet type ids assigned to the campaign (`d`).
    pub pet_type_ids: Vec<u32>,
    /// Campaign duration in ms (`e`), 43,200,000 = 12 h.
    pub duration_ms: u64,
    /// Bonus value (`f`) — semantics not yet pinned down.
    pub bonus: u64,
}

pub fn element_from_id(id: u32) -> Option<Element> {
    match id {
        0 => Some(Element::Neutral),
        1 => Some(Element::Fire),
        2 => Some(Element::Water),
        3 => Some(Element::Earth),
        4 => Some(Element::Wind),
        _ => None,
    }
}

pub fn class_from_id(id: u32) -> Option<Class> {
    match id {
        1 => Some(Class::Blacksmith),
        2 => Some(Class::Alchemist),
        3 => Some(Class::Adventurer),
        4 => Some(Class::Defender),
        5 => Some(Class::Supporter),
        6 => Some(Class::Rogue),
        7 => Some(Class::Assassin),
        8 => Some(Class::Mage),
        _ => None, // 0 = no class
    }
}

impl SaveFile {
    /// Build the typed model from a parsed tree (see [`crate::parse_save`]
    /// for the one-call entry point).
    pub fn from_tree(root: Node) -> anyhow::Result<Self> {
        let x = root.get("X").context("save has no pet block (root key X)")?;

        let pets: Vec<SavePet> = x
            .get("b")
            .map(|b| b.list_or_single().iter().map(SavePet::from_node).collect())
            .unwrap_or_default();

        let equipment = x
            .get("R")
            .map(|r| {
                r.list_or_single()
                    .iter()
                    .map(EquipmentItem::from_node)
                    .collect()
            })
            .unwrap_or_default();

        let materials = x
            .get("Q")
            .map(|q| {
                q.list_or_single()
                    .iter()
                    .map(|n| MaterialStack {
                        item_id: get_u32(n, "a"),
                        count: get_u64(n, "b"),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let dungeon_teams = x
            .get("S")
            .map(|s| {
                s.list_or_single()
                    .iter()
                    .map(DungeonTeam::from_node)
                    .collect()
            })
            .unwrap_or_default();

        let campaigns = x
            .get("x")
            .map(|c| {
                c.list_or_single()
                    .iter()
                    .map(CampaignSlot::from_node)
                    .collect()
            })
            .unwrap_or_default();

        let gems = x
            .get("002")
            .map(|g| {
                g.list_or_single()
                    .iter()
                    .map(|n| {
                        let element_id = get_u32(n, "a");
                        GemStack {
                            element_id,
                            element: element_from_id(element_id),
                            level: get_u32(n, "b"),
                            count: get_u64(n, "c"),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(SaveFile {
            saved_at_unix: root.get("c").and_then(Node::as_i64),
            god_name: root.get("s").and_then(Node::as_str).map(str::to_string),
            player_name: root.get("W").and_then(Node::as_str).map(str::to_string),
            pet_stones: x.get("y").and_then(Node::as_u64),
            puny_food: get_u64(x, "c"),
            strong_food: get_u64(x, "d"),
            mighty_food: get_u64(x, "e"),
            chocolate: get_u64(x, "v"),
            gems,
            pets,
            equipment,
            materials,
            dungeon_teams,
            campaigns,
            root,
        })
    }

    pub fn pet_by_type_id(&self, type_id: u32) -> Option<&SavePet> {
        self.pets.iter().find(|p| p.type_id == type_id)
    }

    pub fn pet_by_name(&self, name: &str) -> Option<&SavePet> {
        self.pets.iter().find(|p| p.name == name)
    }

    pub fn equipment_by_instance_id(&self, instance_id: u32) -> Option<&EquipmentItem> {
        self.equipment
            .iter()
            .find(|e| e.instance_id == instance_id)
    }
}

impl SavePet {
    fn from_node(node: &Node) -> Self {
        let w = node.get("w");
        let class_node = w.and_then(|w| w.get("d"));
        let partner_raw = node.get("F").and_then(Node::as_u32);
        SavePet {
            name: node
                .get("a")
                .and_then(Node::as_str)
                .unwrap_or_default()
                .to_string(),
            type_id: get_u32(node, "k"),
            unlocked: node.get("l").and_then(Node::as_bool).unwrap_or(false),
            growth: node.get("E").and_then(Node::as_f64).unwrap_or(0.0),
            normal_level: get_u32(node, "g"),
            physical_stat: get_f64(node, "j") / 10.0,
            team_slot: match get_u32(node, "v") {
                0 => None,
                slot => u8::try_from(slot).ok(),
            },
            element: w.map(|w| get_u32(w, "a")).and_then(element_from_id),
            dungeon_level: w.map(|w| get_u32(w, "b")).unwrap_or(0),
            dungeon_exp: w.map(|w| get_f64(w, "c")).unwrap_or(0.0),
            class: class_node.map(|d| get_u32(d, "a")).and_then(class_from_id),
            class_level: class_node.map(|d| get_u32(d, "b")).unwrap_or(0),
            class_exp: class_node.map(|d| get_f64(d, "c")).unwrap_or(0.0),
            weapon_id: w.and_then(|w| nonzero_u32(w, "e")),
            armor_id: w.and_then(|w| nonzero_u32(w, "f")),
            accessory_id: w.and_then(|w| nonzero_u32(w, "g")),
            partner_type_id: partner_raw.filter(|&id| id != PARTNER_NONE),
            partner_bond: get_u64(node, "G"),
            raw: node.clone(),
        }
    }
}

impl EquipmentItem {
    /// Display name of the item type, if identified.
    pub fn type_name(&self) -> Option<&'static str> {
        crate::items::equipment_type_name(self.type_id)
    }

    fn from_node(node: &Node) -> Self {
        let gem_level = get_u32(node, "f");
        EquipmentItem {
            type_id: get_u32(node, "a"),
            plus: get_u32(node, "b"),
            quality: get_u32(node, "c"),
            instance_id: get_u32(node, "d"),
            plus_cap: get_u32(node, "e"),
            gem_level,
            gem_element: if gem_level > 0 {
                element_from_id(get_u32(node, "g"))
            } else {
                None
            },
        }
    }
}

impl DungeonTeam {
    fn from_node(node: &Node) -> Self {
        DungeonTeam {
            dungeon_id: get_u32(node, "b"),
            depth: get_u32(node, "d"),
            dungeon_name: node
                .get("i")
                .and_then(Node::as_str)
                .unwrap_or_default()
                .to_string(),
            pet_type_ids: node
                .get("a")
                .and_then(Node::as_int_list)
                .unwrap_or_default(),
            loot: node
                .get("c")
                .map(|c| {
                    c.list_or_single()
                        .iter()
                        .map(|n| (get_u32(n, "a"), get_u64(n, "b")))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

impl CampaignSlot {
    fn from_node(node: &Node) -> Self {
        CampaignSlot {
            index: get_u32(node, "a"),
            pet_type_ids: node
                .get("d")
                .and_then(Node::as_int_list)
                .unwrap_or_default(),
            duration_ms: get_u64(node, "e"),
            bonus: get_u64(node, "f"),
        }
    }
}

fn get_u32(node: &Node, key: &str) -> u32 {
    node.get(key).and_then(Node::as_u32).unwrap_or(0)
}

fn get_u64(node: &Node, key: &str) -> u64 {
    node.get(key).and_then(Node::as_u64).unwrap_or(0)
}

fn get_f64(node: &Node, key: &str) -> f64 {
    node.get(key).and_then(Node::as_f64).unwrap_or(0.0)
}

fn nonzero_u32(node: &Node, key: &str) -> Option<u32> {
    match get_u32(node, key) {
        0 => None,
        v => Some(v),
    }
}
