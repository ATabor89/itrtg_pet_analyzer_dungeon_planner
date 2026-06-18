//! Material/item id → name table for the `X.Q` inventory namespace.
//!
//! Provenance, in decreasing order of confidence:
//! - **export-confirmed**: the id's count in the reference save matches a
//!   uniquely-named line in the same-session Main Stats export.
//! - **prior-project**: carried over from the user's earlier save-decoding
//!   work (cross-referenced against their in-game inventory at the time).
//! - **inferred**: the prior project marked these as partially guessed
//!   (names ending in `?` there); kept as given, treat with suspicion.
//!
//! Ids the prior project explicitly listed as "Unknown #N" are *not* in the
//! table — `material_name` returns `None` so callers can't mistake a
//! placeholder for a name. Note this namespace is distinct from the
//! equipment *type* ids used in `X.R` (equipment type 21 is the Inferno
//! Sword; material 21 is something stackable and still unidentified).

/// Look up the display name for a material/item id (the `X.Q` namespace).
pub fn material_name(id: u32) -> Option<&'static str> {
    Some(match id {
        // -- prior-project, base materials --
        1 => "Herb",
        2 => "Iron Ore",
        3 => "Iron Bar",
        4 => "Ice Block",
        5 => "Nevermelting Ice",
        6 => "Wood",
        7 => "Special Wood",
        8 => "Feather",
        9 => "Bound Feathers",
        10 => "Hot Stone",
        11 => "Fire Stone",
        12 => "Whetstone",
        13 => "Sacred Stone",
        14 => "Phoenix Feather",
        15 => "Health Potion",
        // 16/17/19/21 confirmed against the 2026-06-10 full inventory
        // transcription (counts matched the second save). Note 19 was
        // "Nothing" in the prior project's table — that was wrong; the
        // Antidote count (128) matched id 19 exactly, and "Nothing" is 119.
        16 => "Health Potion X",
        17 => "Health Potion S",
        19 => "Antidote",
        20 => "Flying Boots",
        21 => "Torch",
        22 => "Ginger",
        23 => "Holy Water",
        // -- prior-project, T3 materials (the "Magic" tier) --
        24 => "Magic Fire Stone",
        25 => "Magic Wood",
        26 => "Magic Feather",
        27 => "Magic Ore",
        28 => "Magic Ice",
        29 => "Magic Herb",
        // -- user-confirmed against live inventory (2026-06): talismans --
        31 => "Lucky Talisman", // count 587 matched exactly
        32 => "Wise Talisman",  // adjacent id, count 212 = the "200-something"
        // -- elemental bars (crafted from the element's T1–T3 materials plus
        //    Whetstones and Sacred Stones). Counts in the reference save
        //    (Inferno 5, Hurricane 4, others 10) uniquely pin 33 and 35 and
        //    thereby confirm the prior project's element ordering for the
        //    three 10-count bars. --
        33 => "Inferno Bar",   // fire — count 5 ✓
        34 => "Tsunami Bar",   // water
        35 => "Hurricane Bar", // wind — count 4 ✓
        36 => "Forest Bar",    // earth
        37 => "Titanium Bar",  // neutral/crystal
        // -- export-confirmed in the 2026-06-09 reference save --
        117 => "Ant",            // count 192,164 = Main Stats "Ants"
        159 => "Strategy Book",  // count 2,840 = Main Stats "Strategy Books"
        166 => "Honey",          // count 787 = Main Stats "Honey"
        174 => "Acorn",          // count 24,727 = Main Stats "Acorns"
        // -- prior-project, special/dungeon items --
        118 => "Rebirth Bacon",
        119 => "Nothing", // a second "Nothing" id; both appeared in-game
        126 => "Core Shard of Gnome",
        127 => "Magic Soil",
        // T4 materials — resolved 2026-06-16 by a save-edit probe: the five
        // count-32 stacks were set to distinct counts (41–45) and read off
        // in-game by name.
        131 => "Sun Stone",
        132 => "Jungle Stone",
        133 => "Sky Stone",
        134 => "Mythril",
        135 => "Ocean Stone",
        138 => "Glowing Embers",
        141 => "Living Flame",
        146 => "Whispers of the Wind",
        147 => "Secrets of the Wind",
        149 => "Soul of Sylph",
        153 => "Ale",
        // Aether Ring (player-confirmed 2026-06-18 on a fresh/edited save: the
        // base, no-boss-fights ring is id 130). The in-game "+N" suffix tracks
        // boss kills and is almost certainly the SAME id 130 with a dynamic name
        // (not consecutive ids — 131 is Sun Stone), so the old save's "Aether
        // Ring +28" was also id 130. Resolves 130 from the singleton worklist.
        130 => "Aether Ring",
        162 => "Monster Blood", // player-confirmed 2026-06-18
        // Unidentified ids (worklist):
        // - {160, 164, 167, 168} sit at count 1 and pair with the four singleton
        //   items {Not Nothing, Absolutely Nothing, Food Journal One, Food
        //   Journal Two} — set known, per-id assignment unknown.
        // - Present at count 0: 128, 129, 139, 140, 142–145, 148, 150.
        // 126–149 look like per-dungeon boss material families (Gnome/earth,
        // fire, wind) — the matching water family is presumably nearby.
        // Foods and gems are NOT in this namespace: Puny/Strong/Mighty Food
        // and Chocolate are dedicated save fields (X.c/d/e/v), gems live in
        // X.002 keyed by element id.
        _ => return None,
    })
}

/// Every known material id and name, for the inventory editor's "add item"
/// picker. Built by scanning the [`material_name`] table (ids are sparse).
pub fn known_materials() -> Vec<(u32, &'static str)> {
    (0..=400).filter_map(|id| material_name(id).map(|n| (id, n))).collect()
}

/// Look up the display name for an equipment *type* id (the `X.R[i].a`
/// namespace — distinct from material ids).
///
/// Derived 2026-06-10 by joining, for every equipped item: the gear name in
/// the Pet Stats export ↔ the instance id in the Pet Equips export ↔ the
/// instance→type mapping in the save's `X.R` list. 31 types resolved with
/// zero conflicting votes; Storm Bow (29) was pinned separately as the only
/// type whose instance count (3) uniquely matched the inventory
/// transcription.
///
/// 2026-06-13 the user equipped five formerly-ambiguous types in-game and
/// read them off save 2's instance→type map: 5 = Flame Armor, 8 = Flood Armor,
/// 22 = Water Spear, 41 = Tree Bracelet, 44 = Storm Ring (resolving the old
/// 44 = {Magic Hammer | Storm Ring} tie).
///
/// Still unidentified (all unequipped 1-count types):
/// {23, 26, 30, 52, 56} pair with {Iron Pot, Flood Spear, Leeching Sword,
/// Tree Axe, Hurricane Bow} — equip one in-game to resolve.
pub fn equipment_type_name(type_id: u32) -> Option<&'static str> {
    EQUIPMENT_TYPES
        .iter()
        .find(|(id, _, _)| *id == type_id)
        .map(|(_, name, _)| *name)
}

/// Every known equipment type: `(type id, name, slot category)`. The single
/// source for both [`equipment_type_name`] and [`equipment_category`], and the
/// type list the editor's equipment builder offers. The 300-series event gear
/// categories are cross-checked against `data/equipment_catalog.yaml`.
pub const EQUIPMENT_TYPES: &[(u32, &str, EquipCategory)] = {
    use EquipCategory::{Accessory, Armor, Weapon};
    &[
        // -- Armor --
        (3, "Titanium Armor", Armor),
        (5, "Flame Armor", Armor),
        (8, "Flood Armor", Armor),
        (12, "Forest Armor", Armor),
        (13, "Feather Vest", Armor),
        (15, "Hurricane Armor", Armor),
        (303, "Learning Coat", Armor),
        (305, "Creators Vest", Armor),
        (307, "Merry Mantle", Armor),
        // -- Weapons --
        (18, "Titanium Sword", Weapon),
        (21, "Inferno Sword", Weapon),
        (22, "Water Spear", Weapon),
        (29, "Storm Bow", Weapon),
        (47, "Shaping Hammer", Weapon),
        (50, "Journeying Stick", Weapon),
        (51, "Magic Stick", Weapon),
        (54, "Magic Pot", Weapon),
        (57, "Ego Sword", Weapon),
        (60, "Bursting Knives", Weapon),
        (79, "Legendary Hammer", Weapon), // "Legend Hammer" in-game
        (83, "Exploding Knives", Weapon),
        (300, "Candy Cane", Weapon),
        (304, "Magic Egg", Weapon),
        (306, "Godly Hammer", Weapon),
        // -- Accessories --
        (33, "Titanium Ring", Accessory),
        (36, "Inferno Gloves", Accessory),
        (39, "Tsunami Necklace", Accessory),
        (40, "Wood Bracelet", Accessory),
        (41, "Tree Bracelet", Accessory),
        (44, "Storm Ring", Accessory),
        (45, "Hurricane Ring", Accessory),
        (61, "Alchemist Cape", Accessory),
        (86, "Ear Muffs", Accessory),
        (301, "Spectrometers", Accessory),
        (302, "Master Gloves", Accessory),
        (309, "Growing Love Pendant", Accessory),
        (311, "Christmas Boots", Accessory),
    ]
};

/// Creation name by id (root `i` list order; user-transcribed 2026-06-11,
/// anchored by Next Ats values: Light 12,000, Village 90, Town 60, Moon 10,
/// Solar System 5, Galaxy 25, Universe 84,500).
pub fn creation_name(id: u32) -> Option<&'static str> {
    const NAMES: [&str; 29] = [
        "Shadow Clone",
        "Light",
        "Stone",
        "Soil",
        "Air",
        "Water",
        "Plant",
        "Tree",
        "Fish",
        "Animal",
        "Human",
        "River",
        "Mountain",
        "Forest",
        "Village",
        "Town",
        "Ocean",
        "Nation",
        "Continent",
        "Weather",
        "Sky",
        "Night",
        "Moon",
        "Planet",
        "Earthlike Planet",
        "Sun",
        "Solar System",
        "Galaxy",
        "Universe",
    ];
    NAMES.get(id as usize).copied()
}

/// Monument name by id (root `D` list order; anchored by Next Ats values).
pub fn monument_name(id: u32) -> Option<&'static str> {
    const NAMES: [&str; 9] = [
        "Mighty Statue",
        "Mystic Garden",
        "Tomb of Gods",
        "Everlasting Lighthouse",
        "Godly Statue",
        "Pyramids of Power",
        "Temple of God",
        "Black Hole",
        "White Hole",
    ];
    NAMES.get(id as usize).copied()
}

/// Might name by id (root `V` list order). Ids 0–7 are the normal mights;
/// 8–13 are the special "Unleash Might" abilities whose level adds +1 s to
/// the base duration.
pub fn might_name(id: u32) -> Option<&'static str> {
    const NAMES: [&str; 14] = [
        "Physical HP +",
        "Physical Attack +",
        "Mystic Defense +",
        "Mystic Regen +",
        "Battle Might +",
        "Clones on Divinity +",
        "Clones on Planet +",
        "Powersurge +",
        "Focused Breathing +",
        "Defensive Aura +",
        "Offensive Aura +",
        "Elemental Manipulation",
        "Mystic Mode +",
        "Transformation Aura +",
    ];
    NAMES.get(id as usize).copied()
}

/// SpaceDim / Light-Dimension element name by its 1-based display id
/// (root `009.b[i].a`), in the in-game list order. Transcribed from the
/// player's 2026-06-13 notes (the Next-At/Spread columns anchor the order).
pub fn spacedim_name(id: u32) -> Option<&'static str> {
    const NAMES: [&str; 20] = [
        "Controlled Entropy",
        "Quantum Genesis",
        "Fusion Torch",
        "Fusion Retrofitting",
        "Dyson Harvester",
        "Hive Mind",
        "Timeline Manipulation",
        "Assembly Matrix",
        "Wormhole Network",
        "Dimension Beamer",
        "Focusing Amplifier",
        "Expanded Awareness",
        "Recursive Memory",
        "Gene Splicing",
        "Hyperlane Engine",
        "Substrate Analysis",
        "Sentient Lattice",
        "Matter Compiler",
        "Symbiotic Link",
        "Self Replicating AI",
    ];
    // ids are 1-based in the save (the list has no id-0 element).
    if id >= 1 && (id as usize) <= NAMES.len() {
        Some(NAMES[id as usize - 1])
    } else {
        None
    }
}

/// Physical-training name by id (root `h` list order, 0-based).
///
/// These are the Physical conditioning exercises (they raise the Physical stat),
/// distinct from the Skills (`j`) — the in-game menu doesn't call them skills.
/// The list order is the in-game Physical screen, transcribed by the player
/// 2026-06-18. A challenge eventually unlocks one extra entry the player doesn't
/// have yet, so a future save may carry one more element than there are names
/// here — unknown ids return `None` and the editor falls back to the index.
pub fn physical_training_name(id: u32) -> Option<&'static str> {
    const NAMES: [&str; 28] = [
        "Running",
        "Sit Ups",
        "Push Ups",
        "Swimming",
        "Long Jumps",
        "Shadow Boxing",
        "Jump Rope",
        "Climb Mountains",
        "Run in Water",
        "Meditate",
        "Throw Spears",
        "Smash Rocks",
        "Run with Weights",
        "Walk on Tightropes",
        "Swim with Weights",
        "Dive with Sharks",
        "Jump on Trees",
        "Walk on Water",
        "Walk with 10x Gravity",
        "Run with 50x Gravity",
        "Move Mountains",
        "Learn to Fly",
        "Fly Around the World",
        "Carry Mountains",
        "Fly to the Moon",
        "Fly Around the Universe",
        "Smash Meteorites",
        "Train on Dimension X",
    ];
    NAMES.get(id as usize).copied()
}

/// Skill name by id (root `j` list order, 0-based) — the actual Skills (they
/// raise the Mystic stat and are the only ones with a "Special"-menu usage
/// count). Same caveat as [`physical_training_name`] about a future
/// challenge-unlocked skill.
pub fn skill_name(id: u32) -> Option<&'static str> {
    const NAMES: [&str; 28] = [
        "Double Punch",
        "High Kick",
        "Dodge",
        "Shadow Fist",
        "Focused Breathing",
        "Raging Fist",
        "Defensive Aura",
        "Misdirection",
        "Whirling Foot",
        "Invisible Hand",
        "Dragon Fist",
        "Offense Aura",
        "Elemental Manipulation",
        "Earth Armor",
        "Ice Wall",
        "Clairvoyance",
        "Aura Ball",
        "Mystic Mode",
        "108 Fists of Destiny",
        "Big Bang",
        "God Speed",
        "Teleport",
        "Transformation Aura",
        "Gear Eyes",
        "Reflection Barrier",
        "Ionioi Hero Summon",
        "Unlimited Creation Works",
        "Time Manipulation",
    ];
    NAMES.get(id as usize).copied()
}

/// Monster name by id (root `k` list order, 0-based) — the creatures fought to
/// generate Battle and Divinity. Transcribed by the player 2026-06-18.
pub fn monster_name(id: u32) -> Option<&'static str> {
    const NAMES: [&str; 34] = [
        "Slimy",
        "Frog",
        "Bunny",
        "Goblin",
        "Wolf",
        "Kobold",
        "Big Burger",
        "Skeleton",
        "Zombie",
        "Harpy",
        "Orc",
        "Mummy",
        "Fighting Turtle",
        "Ape",
        "Salamander",
        "Golem",
        "Dullahan",
        "Succubus",
        "Minotaurus",
        "Devil",
        "Gargoyle",
        "Demon",
        "Vampire",
        "Lamia",
        "Dragon",
        "Behemoth",
        "Valkyrie",
        "Nine Tailed Fox",
        "Genbu",
        "Byakko",
        "Suzaku",
        "Seiryuu",
        "Godzilla",
        "Monster Queen",
    ];
    NAMES.get(id as usize).copied()
}

/// Divinity Generator upgrade name by id (root `K.l` list order, 0-based).
/// Player-confirmed 2026-06-18.
pub fn divinity_upgrade_name(id: u32) -> Option<&'static str> {
    const NAMES: [&str; 3] = ["Capacity", "Divinity Gain", "Converting Speed"];
    NAMES.get(id as usize).copied()
}

/// Adventure-mode inventory item name by id (the `032.d` namespace — distinct
/// from the main `X.Q` materials and from the core/enemy ids). Player-identified
/// 2026-06-18 by matching the save's `032.d` list (id `a`, count `b`) against the
/// in-game Adventure inventory (a full 32-item Steam export plus Flask from a
/// Kongregate save). Ids cluster by material family (raw / refined / enhanced /
/// jewel). Ids not held in those saves return `None`.
pub fn adventure_item_name(id: u32) -> Option<&'static str> {
    Some(match id {
        1 => "Sticky Fluid",
        2 => "Rough Hide",
        3 => "Bag of Sand",
        4 => "Heat",
        50 => "Cloth",
        51 => "Leather",
        52 => "Paper",
        53 => "Common Herb",
        54 => "Uncommon Herb",
        57 => "Godly Herb",
        58 => "Common Mana Herb",
        59 => "Uncommon Mana Herb",
        62 => "Godly Mana Herb",
        63 => "Flask", // Kongregate save only (no Steam save held it)
        64 => "Small Bottle",
        68 => "Fire Flower",
        100 => "Scrap Metal",
        101 => "Iron Ore",
        120 => "Metal Bar",
        121 => "Iron Bar",
        150 => "Pine Plank",
        151 => "Beech Plank",
        199 => "Golden Chestnut",
        200 => "Refined Cloth",
        201 => "Refined Leather",
        220 => "Refined Metal",
        221 => "Refined Iron",
        240 => "Enhanced Pine",
        241 => "Enhanced Beech",
        260 => "Enhanced Paper",
        261 => "Fire Jewel",
        265 => "Dark Jewel",
        266 => "Light Jewel",
        _ => return None,
    })
}

/// Adventure-mode **enemy** name by id (the `032.G` core namespace — a core is
/// "<enemy> <quality>", e.g. "Slime SSS"). Distinct from both the adventure-item
/// ids and the training Monster list. Player-identified 2026-06-18 from a Steam
/// save holding cores from all seven of these enemies.
pub fn adventure_enemy_name(id: u32) -> Option<&'static str> {
    Some(match id {
        50 => "Slime",
        53 => "Astaroth", // core namespace; id 53 is "Common Herb" as an item
        63 => "Goblin",
        64 => "Ghoul",
        69 => "Imp",
        72 => "Wraith",
        87 => "Shinigami",
        _ => return None,
    })
}

/// Equipment slot category.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EquipCategory {
    Weapon,
    Armor,
    Accessory,
}

impl EquipCategory {
    pub fn name(self) -> &'static str {
        match self {
            EquipCategory::Weapon => "Weapon",
            EquipCategory::Armor => "Armor",
            EquipCategory::Accessory => "Accessory",
        }
    }
}

/// The slot category of an equipment *type* id (from [`EQUIPMENT_TYPES`]).
/// `None` for unknown ids.
pub fn equipment_category(type_id: u32) -> Option<EquipCategory> {
    EQUIPMENT_TYPES
        .iter()
        .find(|(id, _, _)| *id == type_id)
        .map(|(_, _, cat)| *cat)
}

/// Equipment quality letter for the raw quality id. Player-confirmed ladder
/// (2026-06-17): F E D C B A S SS SSS for ids 0…8.
pub fn quality_name(quality: u32) -> Option<&'static str> {
    Some(match quality {
        0 => "F",
        1 => "E",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_materials_lists_named_ids() {
        let mats = known_materials();
        assert!(mats.len() > 30);
        assert!(mats.contains(&(117, "Ant")));
        assert!(mats.contains(&(130, "Aether Ring")));
        // Only named ids appear.
        assert!(mats.iter().all(|(id, _)| material_name(*id).is_some()));
    }

    #[test]
    fn equipment_categories() {
        use EquipCategory::*;
        assert_eq!(equipment_category(51), Some(Weapon)); // Magic Stick
        assert_eq!(equipment_category(300), Some(Weapon)); // Candy Cane
        assert_eq!(equipment_category(5), Some(Armor)); // Flame Armor
        assert_eq!(equipment_category(303), Some(Armor)); // Learning Coat
        assert_eq!(equipment_category(44), Some(Accessory)); // Storm Ring
        assert_eq!(equipment_category(309), Some(Accessory)); // Growing Love Pendant
        assert_eq!(equipment_category(48), None); // unidentified type
    }

    #[test]
    fn export_confirmed_ids() {
        assert_eq!(material_name(117), Some("Ant"));
        assert_eq!(material_name(159), Some("Strategy Book"));
        assert_eq!(material_name(166), Some("Honey"));
        assert_eq!(material_name(174), Some("Acorn"));
    }

    #[test]
    fn user_confirmed_ids() {
        assert_eq!(material_name(31), Some("Lucky Talisman"));
        assert_eq!(material_name(32), Some("Wise Talisman"));
        assert_eq!(material_name(33), Some("Inferno Bar"));
        assert_eq!(material_name(35), Some("Hurricane Bar"));
        assert_eq!(material_name(37), Some("Titanium Bar"));
        assert_eq!(material_name(19), Some("Antidote")); // not "Nothing"
        assert_eq!(material_name(21), Some("Torch"));
        assert_eq!(material_name(16), Some("Health Potion X"));
        assert_eq!(material_name(17), Some("Health Potion S"));
    }

    #[test]
    fn t4_material_ids() {
        // Resolved 2026-06-16 via a save-edit probe (counts 41–45 → names).
        assert_eq!(material_name(131), Some("Sun Stone"));
        assert_eq!(material_name(132), Some("Jungle Stone"));
        assert_eq!(material_name(133), Some("Sky Stone"));
        assert_eq!(material_name(134), Some("Mythril"));
        assert_eq!(material_name(135), Some("Ocean Stone"));
    }

    #[test]
    fn unknown_ids_return_none() {
        assert_eq!(material_name(0), None);
        assert_eq!(material_name(160), None); // singleton set, still unassigned
        assert_eq!(material_name(9999), None);
        // 130 (Aether Ring) and 162 (Monster Blood) are now known.
        assert_eq!(material_name(130), Some("Aether Ring"));
        assert_eq!(material_name(162), Some("Monster Blood"));
    }

    #[test]
    fn equipment_type_names() {
        assert_eq!(equipment_type_name(21), Some("Inferno Sword"));
        assert_eq!(equipment_type_name(51), Some("Magic Stick"));
        assert_eq!(equipment_type_name(304), Some("Magic Egg"));
        // Resolved 2026-06-13 by equipping each in-game and reading the
        // instance→type map (Bag/Cupid/Meteor/Nugget).
        assert_eq!(equipment_type_name(44), Some("Storm Ring")); // was Magic Hammer | Storm Ring
        assert_eq!(equipment_type_name(5), Some("Flame Armor"));
        assert_eq!(equipment_type_name(8), Some("Flood Armor"));
        assert_eq!(equipment_type_name(22), Some("Water Spear"));
        assert_eq!(equipment_type_name(41), Some("Tree Bracelet"));
    }

    #[test]
    fn training_and_monster_names() {
        // Counts match the reference save's list lengths (h=28, j=28, k=34).
        assert_eq!((0..28).filter_map(physical_training_name).count(), 28);
        assert_eq!((0..28).filter_map(skill_name).count(), 28);
        assert_eq!((0..34).filter_map(monster_name).count(), 34);
        // Endpoints (0-based ids).
        assert_eq!(physical_training_name(0), Some("Running"));
        assert_eq!(physical_training_name(27), Some("Train on Dimension X"));
        assert_eq!(skill_name(0), Some("Double Punch"));
        assert_eq!(skill_name(27), Some("Time Manipulation"));
        assert_eq!(monster_name(0), Some("Slimy"));
        assert_eq!(monster_name(33), Some("Monster Queen"));
        // Out-of-range (e.g. a future challenge-unlocked skill) falls through.
        assert_eq!(physical_training_name(28), None);
        assert_eq!(skill_name(28), None);
        assert_eq!(monster_name(34), None);
    }

    #[test]
    fn divinity_upgrade_names() {
        assert_eq!(divinity_upgrade_name(0), Some("Capacity"));
        assert_eq!(divinity_upgrade_name(1), Some("Divinity Gain"));
        assert_eq!(divinity_upgrade_name(2), Some("Converting Speed"));
        assert_eq!(divinity_upgrade_name(3), None);
    }

    #[test]
    fn adventure_item_and_enemy_names() {
        // Adventure items (032.d namespace) — full 32-item Steam set + Flask.
        assert_eq!(adventure_item_name(1), Some("Sticky Fluid"));
        assert_eq!(adventure_item_name(4), Some("Heat"));
        assert_eq!(adventure_item_name(63), Some("Flask"));
        assert_eq!(adventure_item_name(261), Some("Fire Jewel"));
        assert_eq!(adventure_item_name(266), Some("Light Jewel"));
        assert_eq!(adventure_item_name(9999), None);
        // The two namespaces collide on ids and must stay separate: 50 is
        // "Cloth" (item) vs "Slime" (enemy); 53 is "Common Herb" vs "Astaroth";
        // 64 is "Small Bottle" vs "Ghoul".
        assert_eq!(adventure_item_name(50), Some("Cloth"));
        assert_eq!(adventure_enemy_name(50), Some("Slime"));
        assert_eq!(adventure_item_name(53), Some("Common Herb"));
        assert_eq!(adventure_enemy_name(53), Some("Astaroth"));
        assert_eq!(adventure_item_name(64), Some("Small Bottle"));
        assert_eq!(adventure_enemy_name(64), Some("Ghoul"));
        // All seven held enemies resolve.
        assert_eq!(adventure_enemy_name(69), Some("Imp"));
        assert_eq!(adventure_enemy_name(72), Some("Wraith"));
        assert_eq!(adventure_enemy_name(87), Some("Shinigami"));
        assert_eq!(adventure_enemy_name(9999), None);
        // Core quality reuses the equipment 0–8 F→SSS ladder.
        assert_eq!(quality_name(6), Some("S"));
        assert_eq!(quality_name(8), Some("SSS"));
    }

    #[test]
    fn spacedim_names() {
        // 1-based ids in display order.
        assert_eq!(spacedim_name(1), Some("Controlled Entropy"));
        assert_eq!(spacedim_name(3), Some("Fusion Torch"));
        assert_eq!(spacedim_name(20), Some("Self Replicating AI"));
        assert_eq!(spacedim_name(0), None); // no id-0 element
        assert_eq!(spacedim_name(21), None);
    }
}
