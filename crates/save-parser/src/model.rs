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
    /// Available god power (root `p.j`).
    pub gp_available: Option<u64>,
    /// Total god power spent (root `p.v`).
    pub gp_spent: Option<u64>,
    /// Total might (root `p.F`).
    pub total_might: Option<u64>,
    /// Crystal power (root `p.002` — the numeric keys are siblings of the
    /// letter keys inside `p`).
    pub crystal_power: Option<u64>,
    /// Rebirth count (root `x.k`).
    pub rebirths: Option<u64>,
    /// Light clones (root `O.030`).
    pub light_clones: Option<u64>,
    /// Anni Cake's current stat bonus in percent (root `033`), stored
    /// directly as a fractional float (948.969… displays as 949%). Grows by
    /// 10% (+0.1%×CL when evolved) per hour in food campaigns, fractional
    /// from early-cancelled campaigns; resets on rebirth, capped at 3653%.
    pub anni_cake_bonus_percent: Option<f64>,
    /// Adventure-mode researches (root `032.H.a`), in id order.
    pub researches: Vec<Research>,
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
    /// Current normal Health (`j`), recomputed live by the game. Health is
    /// exactly 10 × Physical ("each physical increases 10 Hp"), so
    /// [`SavePet::physical_stat`] derives from this. Mystic and Battle are
    /// not stored; in-game they differ from Physical only by the Strategy
    /// Room multiplier ratio. See
    /// `reference/save_file_deserialization/normal_stats_investigation.md`.
    pub normal_health: f64,
    /// Training-clone stats — a *snapshot* taken when training was last
    /// configured (bit-identical across saves a day apart, while
    /// `normal_health` moved): the clones this pet fights have these stats.
    /// `o` = clone Physical (Physical‰ × pet Battle / 1000), `p` = clone
    /// Mystic (Mystic‰ setting), `q` = clone Battle (Battle‰ setting),
    /// `r` = clone HP (= 10 × clone Physical, per the Health rule).
    pub clone_physical: f64,
    pub clone_mystic: f64,
    pub clone_battle: f64,
    pub clone_hp: f64,
    /// Dungeon team slot 1–6 (`v`), `None` when not on a team.
    pub team_slot: Option<u8>,
    /// Element (`w.a`).
    pub element: Option<Element>,
    /// Dungeon level (`w.b`).
    pub dungeon_level: u32,
    /// Dungeon exp (`w.c`): **current exp toward the next dungeon level**,
    /// resetting on level-up — matches the in-game "current / needed"
    /// display exactly. The requirement side is
    /// [`crate::formulas::dungeon_exp_to_next`].
    pub dungeon_exp: f64,
    /// Class (`w.d.a`), `None` for classless pets (id 0).
    pub class: Option<Class>,
    /// Class level (`w.d.b`).
    pub class_level: u32,
    /// Class exp (`w.d.c`): current exp toward the next class level, same
    /// semantics as `dungeon_exp` (verified across saves: Salamander hit
    /// CL 25 between the two reference saves and the counter reset).
    /// Requirement: [`crate::formulas::class_exp_to_next`].
    pub class_exp: f64,
    /// Equipment instance ids (`w.e`/`w.f`/`w.g`), `None` when empty (0).
    pub weapon_id: Option<u32>,
    pub armor_id: Option<u32>,
    pub accessory_id: Option<u32>,
    /// Partner pet type id (`F`), `None` when 999. Pairs are mutual
    /// (Cat↔Dog, Vampire↔Succubus, ...). Note id 0 (Mouse) is valid.
    pub partner_type_id: Option<u32>,
    /// Days partnered (`G`) — incremented by exactly 1 per day for every
    /// partnered pet (verified across the two reference saves).
    pub partner_days: u64,
    /// Current exp toward the next normal level (`h`). Matches the in-game
    /// "Current exp" display; only moves while the pet trains.
    pub current_exp: f64,
    /// Village working experience (`H`), in **milliseconds** of total time
    /// worked. Matches the in-game working-time display to the second
    /// (Lamb 9,375,772,300 ms ↔ ~108d 12h).
    pub working_experience_ms: u64,
    /// The pet's raw node, for the still-unidentified fields
    /// (`d,e,f,n,s,t,u,x,y,z,A–D`).
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

/// One adventure-mode research (root `032.H.a[i]`).
///
/// Verified 43/43 against the Main Stats export's "Researches" section
/// (same order as the export, 1-based: id 1 = God HP … id 43 = Core
/// Removal Cost). Exactly two entries had `in_progress` set, matching
/// "Research Slots Level: 2".
#[derive(Debug, Clone, Copy)]
pub struct Research {
    /// Research id (`a`). See [`research_name`] / [`researches`].
    pub id: u32,
    /// Current level (`b`).
    pub level: u32,
    /// Maximum level (`f`).
    pub max_level: u32,
    /// Currently being researched (`c` = 1).
    pub in_progress: bool,
    /// Accumulated research progress toward the next level (`d`).
    pub progress: f64,
}

/// Research id constants (the ones the planner is likely to care about).
pub mod researches {
    /// "Multiplies the stats your pets gain from growth (not dungeon
    /// stats)" — +1% per level. This is the ×1.05 factor in the
    /// normal-stats global multiplier at level 5.
    pub const PET_STATS: u32 = 28;
    pub const CRAFTING_EXP: u32 = 16;
    pub const CRAFTING_SPEED: u32 = 17;
    pub const SMITHING_EXP: u32 = 18;
    pub const SMITHING_SPEED: u32 = 19;
    pub const ALCHEMY_EXP: u32 = 20;
    pub const ALCHEMY_SPEED: u32 = 21;
}

/// Display name for a research id, in Main Stats export order.
pub fn research_name(id: u32) -> Option<&'static str> {
    const NAMES: [&str; 43] = [
        "God HP",
        "God Attack",
        "God Mystic",
        "Building Speed",
        "Creating Speed",
        "Core Drop Rate",
        "Core Quality",
        "Drop Rate",
        "Exp Gain",
        "Equip Attack",
        "Equip Def",
        "Equip Int",
        "Equip Res",
        "Equip Hit",
        "Equip Speed",
        "Crafting Exp",
        "Crafting Speed",
        "Smithing Exp",
        "Smithing Speed",
        "Alchemy Exp",
        "Alchemy Speed",
        "Equip Quality Min",
        "Equip Quality Multi",
        "Active Skill Slot",
        "Passive Skill Slot",
        "Research Speed",
        "Research Slots",
        "Pet Stats",
        "Equip Hp",
        "Might Speed",
        "Spacedim Speed",
        "Multiverse Speed",
        "Max Class Lv",
        "Side Crafting Speed",
        "Min Pow",
        "Enemy Min Lv",
        "Reduce Class Lv Req",
        "Crit Chance",
        "Crit Dam",
        "No Overkill Crit",
        "Max Skill Lv",
        "Skill Experience",
        "Core Removal Cost",
    ];
    // id 0 is an unused placeholder entry in the save.
    if id >= 1 && (id as usize) <= NAMES.len() {
        Some(NAMES[id as usize - 1])
    } else {
        None
    }
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

/// Identified keys in the global tracker block (root `x`), for use with
/// [`SaveFile::global_tracker`]. All confirmed 2026-06-11 by diffing the two
/// reference saves against in-game tooltip readings and the Main Stats
/// exports (each per-pet value matched the user's predicted day-over-day
/// delta).
pub mod trackers {
    // -- per-pet special trackers --
    /// Chocobear: hours banked for the non-food-campaign bonus.
    pub const CHOCOBEAR_BANKED_HOURS: &str = "089";
    /// Pandora's box: feedings counter this rebirth. Observed negative
    /// (-28) right after a rebirth, so it is not a plain count.
    pub const PANDORA_FEEDINGS: &str = "169";
    /// Earth Eater: Earthlike planets eaten, lifetime total.
    pub const EARTH_EATER_PLANETS_TOTAL: &str = "185";
    /// Aether: Delirious Essence of the Forgotten kills (the Aether Ring's
    /// "+N" suffix shows the same number).
    pub const AETHER_BOSS_KILLS: &str = "186";
    /// Pignata: times bashed open.
    pub const PIGNATA_BASHES: &str = "216";
    /// God Power (pet): hours spent in God Power campaigns.
    pub const GOD_POWER_CAMPAIGN_HOURS: &str = "218";
    /// Meteor: total hours spent in campaigns (drives its campaign bonus).
    pub const METEOR_CAMPAIGN_HOURS: &str = "234";
    /// Caterpillar: total materials upgraded.
    pub const CATERPILLAR_MATERIALS_UPGRADED: &str = "259";
    /// Pack Mule: quests participated in.
    pub const MULE_QUESTS: &str = "310";
    /// Gold Dragon: bonus growth granted since July 2024.
    pub const GOLD_DRAGON_BONUS_GROWTH: &str = "311";
    /// Serow: total items saved in dungeons.
    pub const SEROW_ITEMS_SAVED: &str = "324";
    /// Bag: total bonus growth granted.
    pub const BAG_BONUS_GROWTH: &str = "336";
    // NOTE: x.138 was briefly misidentified as Anni Cake's bonus because
    // floor(x.138/3600) happened to equal the displayed 949% in save 2 — a
    // genuine coincidence (save 1 disagrees: 911 vs the actual 709). The
    // real bonus is stored directly at root `033` (see
    // [`crate::SaveFile::anni_cake_bonus_percent`]). x.138 remains an
    // unidentified food/campaign-time-shaped counter.

    // -- global counters (cross-checked against the Main Stats exports) --
    /// AFK clones killed (lifetime).
    pub const AFK_CLONES_KILLED: &str = "013";
    /// Lucky Draws opened.
    pub const LUCKY_DRAWS_OPENED: &str = "071";
    /// Crystal power.
    pub const CRYSTAL_POWER: &str = "074";
    /// Dungeon bosses defeated.
    pub const DUNGEON_BOSSES: &str = "078";
    /// Dungeon enemies defeated.
    pub const DUNGEON_ENEMIES: &str = "079";
    /// Dungeon rooms beaten.
    pub const DUNGEON_ROOMS: &str = "080";
    /// Total might.
    pub const TOTAL_MIGHT: &str = "129";
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

        let researches = root
            .get_path(&["032", "H", "a"])
            .map(|r| {
                r.list_or_single()
                    .iter()
                    .map(|n| Research {
                        id: get_u32(n, "a"),
                        level: get_u32(n, "b"),
                        max_level: get_u32(n, "f"),
                        in_progress: get_u32(n, "c") == 1,
                        progress: get_f64(n, "d"),
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
            gp_available: root.get_path(&["p", "j"]).and_then(Node::as_u64),
            gp_spent: root.get_path(&["p", "v"]).and_then(Node::as_u64),
            total_might: root.get_path(&["p", "F"]).and_then(Node::as_u64),
            crystal_power: root.get_path(&["p", "002"]).and_then(Node::as_u64),
            rebirths: root.get_path(&["x", "k"]).and_then(Node::as_u64),
            light_clones: root.get_path(&["O", "030"]).and_then(Node::as_u64),
            anni_cake_bonus_percent: root.get("033").and_then(Node::as_f64),
            researches,
            pets,
            equipment,
            materials,
            dungeon_teams,
            campaigns,
            root,
        })
    }

    /// Read a numeric value from the global tracker block (root `x`) — a
    /// flat struct of numeric-keyed game counters, including the per-pet
    /// special trackers. See [`trackers`] for the identified keys.
    pub fn global_tracker(&self, key: &str) -> Option<f64> {
        self.root.get_path(&["x", key]).and_then(Node::as_f64)
    }

    /// Level of a research by id (0 if absent). See [`researches`] for ids.
    pub fn research_level(&self, id: u32) -> u32 {
        self.researches
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.level)
            .unwrap_or(0)
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
    /// Current normal Physical stat (= Health / 10).
    pub fn physical_stat(&self) -> f64 {
        self.normal_health / 10.0
    }

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
            normal_health: get_f64(node, "j"),
            clone_physical: get_f64(node, "o"),
            clone_mystic: get_f64(node, "p"),
            clone_battle: get_f64(node, "q"),
            clone_hp: get_f64(node, "r"),
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
            partner_days: get_u64(node, "G"),
            current_exp: get_f64(node, "h"),
            working_experience_ms: get_u64(node, "H"),
            raw: node.clone(),
        }
    }
}

/// Save quality id for quality "A" — the wiki's stat baseline. Each step
/// above/below shifts the multiplier ±10% (additively).
const QUALITY_A: u32 = 5;

impl EquipmentItem {
    /// Display name of the item type, if identified.
    pub fn type_name(&self) -> Option<&'static str> {
        crate::items::equipment_type_name(self.type_id)
    }

    /// Quality letter. Verified against exports: 8=SSS, 7=SS, 6=S, 5=A, 4=B;
    /// 3=C and 2=D are inferred from the wiki's quality ladder.
    pub fn quality_name(&self) -> Option<&'static str> {
        Some(match self.quality {
            2 => "D",
            3 => "C",
            4 => "B",
            5 => "A",
            6 => "S",
            7 => "SS",
            8 => "SSS",
            _ => return None,
        })
    }

    /// Quality multiplier on the catalogued A+0 stat percentages:
    /// ±10% per quality step around A (wiki: C ⇒ 0.8, SSS ⇒ 1.3).
    pub fn quality_multiplier(&self) -> f64 {
        1.0 + (self.quality as f64 - QUALITY_A as f64) * 0.1
    }

    /// Upgrade multiplier: +5% per "+" level (wiki: +8 ⇒ 1.4, +20 ⇒ 2.0).
    pub fn upgrade_multiplier(&self) -> f64 {
        1.0 + 0.05 * self.plus as f64
    }

    /// Combined multiplier applied to the item's A+0 stat percentages
    /// (quality × upgrade; wiki: SSS +20 ⇒ 1.3 × 2.0 = 2.6).
    pub fn stat_multiplier(&self) -> f64 {
        self.quality_multiplier() * self.upgrade_multiplier()
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
