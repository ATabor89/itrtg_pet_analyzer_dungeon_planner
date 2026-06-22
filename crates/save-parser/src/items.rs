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

/// Look up the display name for a material/item id — the **complete
/// `NCPJFPLCPPK` enum** (`Assembly-CSharp`), so every item names wherever its id
/// appears (the `X.Q` material inventory and elsewhere). Names confirmed against
/// in-game inventory / exports are kept verbatim; the rest are transcribed from
/// the enum (PascalCase split, "of" lowercased to match the confirmed style).
///
/// Notes: foods 101–105 (Pet/Puny/Strong/Mighty Food, Chocolate) and gems are
/// the same enum but live in dedicated save fields (`X.c/d/e/v`, `X.002`), not
/// `X.Q`; they're named here for completeness. The elemental-pet evolution-quest
/// material families are 106–116 (water/`Undine`), 126–129 (`Gnome`/earth),
/// 138–145 (`Salamander`/fire), 146–150 (`Sylph`/wind). 33–37 bars: see the
/// 2026-06-19 element-order bug fix (34 Forest / 36 Titanium / 37 Tsunami).
pub fn material_name(id: u32) -> Option<&'static str> {
    Some(match id {
        // -- base materials (T1) --
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
        // 16/17/19/21 confirmed against the 2026-06-10 full inventory
        // transcription; 19 = Antidote (the prior project wrongly had "Nothing";
        // real Nothing is 119).
        15 => "Health Potion",
        16 => "Health Potion X",
        17 => "Health Potion S",
        18 => "Elixir",
        19 => "Antidote",
        20 => "Flying Boots",
        21 => "Torch",
        22 => "Ginger",
        23 => "Holy Water",
        // -- T3 "Magic" tier --
        24 => "Magic Fire Stone",
        25 => "Magic Wood",
        26 => "Magic Feather",
        27 => "Magic Ore",
        28 => "Magic Ice",
        29 => "Magic Herb",
        30 => "Melting Bomb", // dungeon consumable (player-confirmed 2026-06-18)
        31 => "Lucky Talisman",
        32 => "Wise Talisman",
        // Elemental bars. 34/36/37 element order corrected 2026-06-19 from the
        // enum (were Tsunami/Forest/Titanium); 33/35 were already right.
        33 => "Inferno Bar",   // fire
        34 => "Forest Bar",    // earth
        35 => "Hurricane Bar", // wind
        36 => "Titanium Bar",  // neutral
        37 => "Tsunami Bar",   // water
        // -- keys / alloys / runes / super talismans --
        38 => "Golden Key",
        39 => "Frost Key",
        40 => "Magma Key",
        41 => "Rainbow Key",
        42 => "Mystic Key",
        43 => "Golden Stone",
        44 => "Frozen Alloy",
        45 => "Magma Stone",
        46 => "Rainbow Ore",
        47 => "Transparent Stone",
        48 => "Nanotrap",      // dungeon trap (player-confirmed 2026-06-18)
        49 => "Freezing Bomb", // dungeon consumable (player-confirmed 2026-06-18)
        50 => "Demonic Rune",
        51 => "Holy Rune",
        52 => "Super Lucky Talisman",
        53 => "Very Wise Talisman",
        // -- foods (stored in X.c/d/e/v, not X.Q — named for completeness) --
        101 => "Pet Food",
        102 => "Puny Food",
        103 => "Strong Food",
        104 => "Mighty Food",
        105 => "Chocolate",
        // -- water / Undine evolution-quest family --
        106 => "Undine",
        107 => "Body",
        108 => "Mecha Arm",
        109 => "Water Soul",
        110 => "Purified Water",
        111 => "Improved Water Soul",
        112 => "Soulless Soul",
        113 => "Soul of Undine",
        114 => "Magic Soul of Undine",
        115 => "Shadow Essence",
        116 => "Magic Shadow Essence",
        // -- export-confirmed / special items --
        117 => "Ant",          // count 192,164 = Main Stats "Ants"
        118 => "Rebirth Bacon",
        119 => "Nothing",
        120 => "Cure", // player-confirmed 2026-06-18
        121 => "Vaccine",
        122 => "Shiny Stone",
        123 => "Horn of Balrog",
        124 => "AF Coin",
        125 => "Runestone",
        // -- Gnome / earth evolution-quest family --
        126 => "Core Shard of Gnome",
        127 => "Magic Soil",
        128 => "Soul of Gnome",
        129 => "Magic Soul of Gnome",
        130 => "Aether Ring", // base ring; the in-game "+N" suffix is dynamic
        // -- T4 stones (save-edit probe 2026-06-16) --
        131 => "Sun Stone",
        132 => "Jungle Stone",
        133 => "Sky Stone",
        134 => "Mythril",
        135 => "Ocean Stone",
        136 => "Dark Matter",
        137 => "Angel Wing",
        // -- Salamander / fire evolution-quest family --
        138 => "Glowing Embers",
        139 => "Igneous Bones",
        140 => "Pliable Magma",
        141 => "Living Flame",
        142 => "Salamander Soul",
        143 => "Magic Soul of Salamander",
        144 => "Salamander Skin",
        145 => "Prosthetic Tail",
        // -- Sylph / wind evolution-quest family --
        146 => "Whispers of the Wind",
        147 => "Secrets of the Wind",
        148 => "Mysteries of the Wind",
        149 => "Soul of Sylph",
        150 => "Magic Soul of Sylph",
        // -- misc --
        151 => "Fools Coin",
        152 => "Shifting Scroll",
        153 => "Ale",
        154 => "Weird Glowing Rock",
        155 => "Scroll of Beginnings",
        156 => "Scroll of Trials Infinite",
        157 => "Scroll of Trials Help",
        158 => "Scroll Key",
        159 => "Strategy Book", // count 2,840 = Main Stats "Strategy Books"
        160 => "Not Nothing",
        161 => "Shinier Stone",
        162 => "Monster Blood",
        163 => "Blood Potion",
        164 => "Absolutely Nothing",
        165 => "Craziness",
        166 => "Honey", // count 787 = Main Stats "Honey"
        167 => "Food Journal One",
        168 => "Food Journal Two",
        169 => "Shiny Metal Stone",
        170 => "Shiny Water Stone",
        171 => "Shiny Fire Stone",
        172 => "Shiny Wind Stone",
        173 => "Shiny Earth Stone",
        174 => "Acorn", // count 24,727 = Main Stats "Acorns"
        // -- adventure-research "spark" tier --
        350 => "Spark of Genius",
        351 => "Spark of Empathy",
        352 => "Spark of Passion",
        // -- fishing: rods / baits / catches (stored in the fishing block, not
        //    X.Q — named for completeness) --
        500 => "Stick Rod",
        501 => "Wooden Rod",
        502 => "Bamboo Rod",
        503 => "Voodoo Rod",
        504 => "Titanium Rod",
        520 => "Feather Ball",
        521 => "Simple Worm",
        522 => "Big Worm",
        523 => "Caterpillar",
        524 => "Super Worm",
        525 => "Boots",
        526 => "Shrimp",
        527 => "Clam",
        528 => "Poison Gill",
        529 => "Snout",
        530 => "Crab",
        531 => "Anglerfish",
        532 => "Bigmofi",
        533 => "Bluecarp",
        534 => "Bottlefish",
        535 => "Crappy",
        536 => "Eyefish",
        537 => "Eye of Jelly",
        538 => "Pond Eye",
        539 => "Fishbone",
        540 => "Fugu",
        541 => "Golden Rinny",
        542 => "Goldfish",
        543 => "Green Carp",
        544 => "Green Jelly",
        545 => "Gremifi",
        546 => "Kinky Fish",
        547 => "Kraken",
        548 => "Lazy Rinny",
        549 => "Mackerel",
        550 => "Midlife Fish",
        551 => "Mighty Carp",
        552 => "Moonfish",
        553 => "Perch",
        554 => "Poisonperch",
        555 => "Queen Satchy",
        556 => "Rainbow Fish",
        557 => "Red Betta",
        558 => "Red Carp",
        559 => "Sadfish",
        560 => "Sad Shark",
        561 => "Seastar",
        562 => "Smilefish",
        563 => "Speedy Spear Fish",
        564 => "Spiketuna",
        565 => "Squeeny",
        566 => "Squid",
        567 => "Trout",
        800 => "Scales",
        801 => "Ultimate Reward",
        _ => return None,
    })
}

/// Every known material id and name, for the inventory editor's "add item"
/// picker. Built by scanning the [`material_name`] table (ids are sparse).
pub fn known_materials() -> Vec<(u32, &'static str)> {
    (0..=400).filter_map(|id| material_name(id).map(|n| (id, n))).collect()
}

/// All identified adventure-inventory item ids with names (for the editor's
/// "add item" picker). Same enum as the `032.d` inventory. The range spans the
/// full id space (incl. the 1000+ Event/Pet-Stone/Growth items); `filter_map`
/// drops the gaps.
pub fn known_adventure_items() -> Vec<(u32, &'static str)> {
    (0..=1002).filter_map(|id| adventure_item_name(id).map(|n| (id, n))).collect()
}

/// All identified adventure enemy/entity ids with names (for the editor's "add
/// core" picker). Same enum as the `032.G` cores; spans the event bosses (480,
/// 500-507). Excludes the "Player" entity (id 0/1 in the entity enum) — you
/// don't get a core from yourself.
pub fn known_adventure_enemies() -> Vec<(u32, &'static str)> {
    (0..=600)
        .filter_map(|id| adventure_enemy_name(id).map(|n| (id, n)))
        .filter(|(_, n)| *n != "Player")
        .collect()
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
/// 44 = {Magic Hammer | Storm Ring} tie). 2026-06-19, same method, resolved the
/// crafting-weapon families: 48 = Magic Hammer (the real one — the 44 tie went
/// to Storm Ring), 80 = Legendary Stick, 81 = Legendary Pot (with 79 Legendary
/// Hammer, the 79/80/81 Legendary family).
///
/// **The complete `MBBDNNAMMHO` equipment-type enum** (`Assembly-CSharp`),
/// transcribed verbatim so every owned item names in the save editor — not just
/// the curated subset. The base grid is element×slot (Iron/Steel/Titanium +
/// Fire/Flame/Inferno + Water/Flood/Tsunami + Wood/Tree/Forest + Feather/Storm/
/// Hurricane), then crafting/legendary/special/event families. Slot **categories**
/// stay in [`EQUIPMENT_TYPES`] (curated/verified, incl. catalog-vs-equip-slot
/// quirks like Ear Muffs); a unit test guards that the two never disagree on a name.
pub fn equipment_type_name(type_id: u32) -> Option<&'static str> {
    Some(match type_id {
        0 => "None",
        1 => "Iron Vest",
        2 => "Steel Armor",
        3 => "Titanium Armor",
        4 => "Fire Vest",
        5 => "Flame Armor",
        6 => "Inferno Armor",
        7 => "Water Vest",
        8 => "Flood Armor",
        9 => "Tsunami Armor",
        10 => "Wooden Vest",
        11 => "Tree Armor",
        12 => "Forest Armor",
        13 => "Feather Vest",
        14 => "Storm Armor",
        15 => "Hurricane Armor",
        16 => "Iron Sword",
        17 => "Steel Sword",
        18 => "Titanium Sword",
        19 => "Fire Sword",
        20 => "Flame Sword",
        21 => "Inferno Sword",
        22 => "Water Spear",
        23 => "Flood Spear",
        24 => "Tsunami Spear",
        25 => "Wood Axe",
        26 => "Tree Axe",
        27 => "Forest Axe",
        28 => "Feather Bow",
        29 => "Storm Bow",
        30 => "Hurricane Bow",
        31 => "Iron Ring",
        32 => "Steel Ring",
        33 => "Titanium Ring",
        34 => "Fire Gloves",
        35 => "Flame Gloves",
        36 => "Inferno Gloves",
        37 => "Water Necklace",
        38 => "Flood Necklace",
        39 => "Tsunami Necklace",
        40 => "Wood Bracelet",
        41 => "Tree Bracelet",
        42 => "Forest Bracelet",
        43 => "Feather Ring",
        44 => "Storm Ring",
        45 => "Hurricane Ring",
        46 => "Forging Hammer",
        47 => "Shaping Hammer",
        48 => "Magic Hammer",
        49 => "Walking Stick",
        50 => "Journeying Stick",
        51 => "Magic Stick",
        52 => "Iron Pot",
        53 => "Steel Pot",
        54 => "Magic Pot",
        55 => "Training Sword",
        56 => "Leeching Sword",
        57 => "Ego Sword",
        58 => "Howling Knives",
        59 => "Thundering Knives",
        60 => "Bursting Knives",
        61 => "Alchemist Cape",
        62 => "Celestial Bow",
        63 => "Gram",
        64 => "Mythril Armor",
        65 => "Sun Armor",
        66 => "Ocean Armor",
        67 => "Jungle Armor",
        68 => "Sky Armor",
        69 => "Mythril Shield",
        70 => "Sun Sword",
        71 => "Ocean Spear",
        72 => "Jungle Axe",
        73 => "Sky Bow",
        74 => "Mythril Ring",
        75 => "Sun Gloves",
        76 => "Ocean Necklace",
        77 => "Jungle Bracelet",
        78 => "Sky Ring",
        79 => "Legendary Hammer",
        80 => "Legendary Stick",
        81 => "Legendary Pot",
        82 => "Soul Sword",
        83 => "Exploding Knives",
        84 => "Mana Cape",
        85 => "Robe of Economy",
        86 => "Ear Muffs",
        140 => "Rune Patch",
        141 => "Haposti",
        142 => "Wonder Axe",
        143 => "Enlightment Vest", // sic — the game enum itself misspells "Enlighten"
        144 => "Ele Twin Dagger",
        150 => "Demonic Armor",
        151 => "Demonic Sword",
        152 => "Demonic Ring",
        200 => "Divine Armor",
        201 => "Divine Bow",
        202 => "Divine Ring",
        250 => "Neutral Crafting Sword",
        251 => "Water Crafting Sword",
        252 => "Fire Crafting Sword",
        253 => "Wind Crafting Sword",
        254 => "Earth Crafting Sword",
        300 => "Candy Cane",
        301 => "Spectrometers",
        302 => "Master Gloves",
        303 => "Learning Coat",
        304 => "Magic Egg",
        305 => "Creators Vest",
        306 => "Godly Hammer",
        307 => "Merry Mantle",
        308 => "Shroud of Enlightenment",
        309 => "Growing Love Pendant",
        310 => "Hungering Talon",
        311 => "Christmas Boots",
        _ => return None,
    })
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
        (23, "Flood Spear", Weapon), // MBBDNNAMMHO enum (2026-06-19)
        (26, "Tree Axe", Weapon), // MBBDNNAMMHO enum (2026-06-19)
        (29, "Storm Bow", Weapon),
        (30, "Hurricane Bow", Weapon), // MBBDNNAMMHO enum (2026-06-19)
        (47, "Shaping Hammer", Weapon),
        (48, "Magic Hammer", Weapon), // player-confirmed 2026-06-19 (Anteater)
        (50, "Journeying Stick", Weapon),
        (51, "Magic Stick", Weapon),
        (52, "Iron Pot", Weapon), // MBBDNNAMMHO enum (2026-06-19)
        (54, "Magic Pot", Weapon),
        (56, "Leeching Sword", Weapon), // MBBDNNAMMHO enum (2026-06-19)
        (57, "Ego Sword", Weapon),
        (60, "Bursting Knives", Weapon),
        // The Legendary crafting-weapon family (79/80/81) — player-confirmed
        // 2026-06-19 (Salamander / Caterpillar) by joining the Pet Stats export
        // gear ↔ the save's pet weapon slot ↔ the X.R instance→type map.
        (79, "Legendary Hammer", Weapon), // "Legend Hammer" in-game
        (80, "Legendary Stick", Weapon),
        (81, "Legendary Pot", Weapon),
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

/// Pet type name by internal type id (pet field `k`; partner id is field `F`,
/// 999 = none). **Authoritative** — transcribed verbatim from the game's
/// `HFNFDKEMAIK` enum in `Assembly-CSharp` (verified against the save's pet
/// roster; every FINDINGS anchor — 2=Cat, 25=Reindeer, 32=Pandora, 89=Salamander,
/// 123=Vampire, 803=Serow, 999=None — matches). Names are the export-normalized
/// spellings (e.g. `Reindeer` not `Rudolph`, `BHC`, `Firefox`), matching the
/// "save name → export name" table in `FINDINGS.md`. Ids are sparse past 151
/// (special pets at 750–803, 900–902, 999).
pub fn pet_type_name(type_id: u32) -> Option<&'static str> {
    Some(match type_id {
        0 => "Mouse",
        1 => "Rabbit",
        2 => "Cat",
        3 => "Dog",
        4 => "Fairy",
        5 => "Dragon",
        6 => "Doughnut",
        7 => "Eagle",
        8 => "Phoenix",
        9 => "Squirrel",
        10 => "Turtle",
        11 => "Penguin",
        12 => "Cupid",
        13 => "Camel",
        14 => "Goat",
        15 => "Mole",
        16 => "Octopus",
        17 => "Pegasus",
        18 => "Robot",
        19 => "Shark",
        20 => "Slime",
        21 => "Snake",
        22 => "Ufo",
        23 => "Wizard",
        24 => "Pumpkin",
        25 => "Reindeer",
        26 => "Stone",
        27 => "AfkyClone",
        28 => "GoldDragon",
        29 => "Egg",
        30 => "Whale",
        31 => "Hydra",
        32 => "Pandora",
        33 => "FSM",
        34 => "Hedgehog",
        35 => "Crab",
        36 => "Panda",
        37 => "Ape",
        38 => "Cloud",
        39 => "Book",
        40 => "Ghost",
        41 => "Question",
        42 => "Chameleon",
        43 => "Chocobear",
        44 => "Undine",
        45 => "Anteater",
        46 => "Bee",
        47 => "Frog",
        48 => "Elephant",
        49 => "LuckyCoin",
        50 => "Otter",
        51 => "Nightmare",
        52 => "Valkyrie",
        53 => "Santa",
        54 => "BHC",
        55 => "Corona",
        56 => "Vaccina",
        57 => "Firefox",
        58 => "BeachBall",
        59 => "Yggdrasil",
        60 => "Tanuki",
        61 => "Raiju",
        62 => "Armadillo",
        63 => "Raven",
        64 => "Balrog",
        65 => "Seal",
        66 => "Mimic",
        67 => "LivingDraw",
        68 => "GodPower",
        69 => "Elf",
        70 => "Hourglass",
        71 => "Bug",
        72 => "Archer",
        73 => "Rose",
        74 => "MysteriousEgg",
        75 => "Bottle",
        76 => "Bag",
        77 => "Succubus",
        78 => "Gray",
        79 => "Clam",
        80 => "Gnome",
        81 => "Aether",
        82 => "EarthEater",
        83 => "Unicorn",
        84 => "Portal",
        85 => "CardboardBox",
        86 => "StaleTortilla",
        87 => "Witch",
        88 => "Sloth",
        89 => "Salamander",
        90 => "Cocoa",
        91 => "Wolf",
        92 => "Volcano",
        93 => "Swan",
        94 => "Sylph",
        95 => "Void",
        96 => "Pignata",
        97 => "Carno",
        98 => "Lizard",
        99 => "Meteor",
        100 => "Sphinx",
        101 => "Alien",
        102 => "Bat",
        103 => "FlyingEyeball",
        104 => "Basilisk",
        105 => "Leviathan",
        106 => "Cherub",
        107 => "Spoon",
        108 => "Goblin",
        109 => "Dwarf",
        110 => "Koi",
        111 => "Caterpillar",
        112 => "DecoratorCrab",
        113 => "Elemental",
        114 => "BunnyGirl",
        115 => "MistSphere",
        116 => "ShadowClone",
        117 => "Sniper",
        118 => "Tenko",
        119 => "WhiteTiger",
        120 => "BlackTortoise",
        121 => "VermilionPheasant",
        122 => "AzureDragon",
        123 => "Vampire",
        124 => "Strategist",
        125 => "Mermaid",
        126 => "HoneyBadger",
        127 => "Monk",
        128 => "PackMule",
        129 => "Hamster",
        130 => "AnniCake",
        131 => "Llysnafedda",
        132 => "Aurelius",
        133 => "AncientMimic",
        134 => "HwangeumPig",
        135 => "Student",
        136 => "AntQueen",
        137 => "Crocodile",
        138 => "Lamb",
        139 => "Duragizer",
        140 => "Simulacrum",
        141 => "Nugget",
        142 => "Arachne",
        143 => "PixieGoat",
        144 => "FaintingCapra",
        145 => "Wolpertinger",
        146 => "Skeleton",
        147 => "BigBurger",
        148 => "Oni",
        149 => "Baphomate",
        150 => "Dorgegebelle",
        151 => "Bear",
        750 => "GrayChild1",
        751 => "GrayChild2",
        752 => "Dummy",
        800 => "Nothing",
        801 => "Owl",
        802 => "Unknown",
        803 => "Serow",
        900 => "Fawn",
        901 => "Herakles",
        902 => "Fool",
        999 => "None",
        _ => return None,
    })
}

/// Elemental-pet form name by id (pet field `y`; 0 for non-elemental pets).
/// **Authoritative** — verbatim from the game's `ANHOKMNPAKI` enum. Reveals the
/// full water/`Undine` family (3–8), the `Gnome`/`Salamander`/`Sylph` ladders,
/// and the `LostArm`/`LostBody`/`GrayChild` special forms. Note Salamander/Sylph
/// start at `V0` while Gnome starts at `V1`, and each line ends in `…Final`
/// (the in-game "V4"); this matches the per-pet `y` offsets in `FINDINGS.md`
/// (Gnome `y`=14=GnomeFinal, Salamander `y`=19=Final, Sylph `y`=24=Final).
/// Ultimate Being id → name (the 5 planet UBs; C# `CEFAAPALBMD.BIALOOCFKFI`).
/// Used for the planet system's UB list (`T.f`, keyed by `c`).
pub fn ultimate_being_name(id: u32) -> Option<&'static str> {
    Some(match id {
        1 => "Planet Eater",
        2 => "Godly Tribunal",
        3 => "Living Sun",
        4 => "God Above All",
        5 => "ITRTG",
        _ => return None,
    })
}

/// Village building/feature id → name (C# enum `IMBOLMEHKCG`; functional
/// buildings 0–14 — ids 100+ are cosmetic layout tiles/fences/walls). Used for
/// the `024.a` building-state list (`g` = building type).
pub fn village_building_name(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "None",
        1 => "Fishing",
        2 => "Tavern",
        3 => "Village Center",
        4 => "Dojo",
        5 => "Material Factory",
        6 => "Snack Bar",
        7 => "Forge",
        8 => "Alchemy Hut",
        9 => "Divine Hut",
        10 => "Hunters Guild",
        11 => "Crystal Tower",
        12 => "Strategy Room",
        13 => "Battle Tent",
        14 => "Museum",
        _ => return None,
    })
}

/// Museum statue id → name (C# enum `JBGNCMHGOFI`). Event commemorative statues;
/// the Museum list is at `024.f.a` (`a`=level, `b`=statue id).
pub fn statue_name(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "None",
        1 => "Easter 2024",
        2 => "Summer 2024",
        3 => "Anniversary 2024",
        4 => "Halloween 2024",
        5 => "Valentine 2025",
        6 => "Easter 2025",
        7 => "Summer 2025",
        8 => "Halloween 2025",
        9 => "Christmas 2025",
        10 => "Valentine 2026",
        11 => "Easter 2026",
        _ => return None,
    })
}

/// Dungeon id → name (C# enum `GFEKIABOPIH`). Used for the dungeon-team and
/// active-dungeon-run blocks (`X.S[i].b`, `X.P[i].a`).
pub fn dungeon_name(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "None",
        1 => "Newbie Ground",
        2 => "Scrapyard",
        3 => "Water Temple",
        4 => "Volcano",
        5 => "Forest",
        6 => "Mountain",
        7 => "Elemental Challenge",
        8 => "Neutral Challenge",
        9 => "Water Challenge",
        10 => "Fire Challenge",
        11 => "Wind Challenge",
        12 => "Earth Challenge",
        13 => "Elemental Tower",
        14 => "Neutral Tower",
        15 => "Water Tower",
        16 => "Fire Tower",
        17 => "Wind Tower",
        18 => "Earth Tower",
        19 => "Test Dummy",
        20 => "Dark Left",
        21 => "Dark Middle",
        22 => "Dark Right",
        23 => "Dark Final",
        24 => "Light Left",
        25 => "Light Middle",
        26 => "Light Right",
        27 => "Light Final",
        _ => return None,
    })
}

/// Fishing pond id → name (C# enum `BAMKFONNEMP`). The fishing block's `025.f`
/// is the current pond.
pub fn pond_name(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "New Pond",
        1 => "Lazy Pond",
        2 => "Kind Pond",
        3 => "Stupid Pond",
        4 => "Sad Pond",
        5 => "Midlife Pond",
        6 => "Kinky Pond",
        7 => "Great Pond",
        8 => "Sacred Pond",
        9 => "Final Pond",
        _ => return None,
    })
}

pub fn elemental_form_name(form_id: u32) -> Option<&'static str> {
    const NAMES: [&str; 25] = [
        "None",
        "LostArm",
        "LostBody",
        "FailedUndine",
        "UndineV1",
        "UndineV2",
        "UndineV3",
        "UndineV4",
        "UndineFinal",
        "GrayChild",
        "GrayChildDeleted",
        "GnomeV1",
        "GnomeV2",
        "GnomeV3",
        "GnomeFinal",
        "SalamanderV0",
        "SalamanderV1",
        "SalamanderV2",
        "SalamanderV3",
        "SalamanderFinal",
        "SylphV0",
        "SylphV1",
        "SylphV2",
        "SylphV3",
        "SylphFinal",
    ];
    NAMES.get(form_id as usize).copied()
}

/// Gem element name by id — the game's `EMGELCMNFOL` enum (`Assembly-CSharp`),
/// the element axis shared by gems and the dungeon/pet element. Pets and
/// dungeons only ever use 0–4 (see [`crate::model::Element`]), but **gems can
/// also be `Dark`(5), `Light`(6), `Elemental`(50) or `All`(99)** — ids the base
/// 5-element model can't name. Use this for the gem element (`X.002[i].a`,
/// equipment `X.R[i].g`); for pet/dungeon elements prefer `Element`.
pub fn gem_element_name(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "Neutral",
        1 => "Fire",
        2 => "Water",
        3 => "Earth",
        4 => "Wind",
        5 => "Dark",
        6 => "Light",
        50 => "Elemental",
        99 => "All",
        _ => return None,
    })
}

/// Campaign type name by id — the game's `AGGDKICFOAI` enum (`Assembly-CSharp`).
/// These are the campaign categories a pet can specialize in (see the pet's
/// `t`/`u` preferred-campaign fields, which store this id **+ 1**, reserving 0
/// for "no preference"). Also the category axis for the Growth Chamber sim.
pub fn campaign_type_name(id: u32) -> Option<&'static str> {
    const NAMES: [&str; 9] = [
        "Growth",
        "Divinity",
        "Food",
        "Item",
        "Level",
        "Multiplier",
        "GodPower",
        "All",
        "Event",
    ];
    NAMES.get(id as usize).copied()
}

/// Pet feeding-setting name by id (pet field `x`) — the per-pet auto-feed mode.
/// Transcribed from the pet class's `CJMBBFKNFNF()` accessor in
/// `Assembly-CSharp`: 0 None, 1 Puny, 2 Strong, 3 Mighty, 4 Chocolate, 5 Free,
/// 6 Starve. (The game treats any out-of-range value as "Chocolate"; we return
/// `None` for those since real saves only use 0–6.)
pub fn feeding_setting_name(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "None",
        1 => "Puny",
        2 => "Strong",
        3 => "Mighty",
        4 => "Chocolate",
        5 => "Free",
        6 => "Starve",
        _ => return None,
    })
}

/// Divinity Generator upgrade name by id (root `K.l` list order, 0-based).
/// Player-confirmed 2026-06-18.
pub fn divinity_upgrade_name(id: u32) -> Option<&'static str> {
    const NAMES: [&str; 3] = ["Capacity", "Divinity Gain", "Converting Speed"];
    NAMES.get(id as usize).copied()
}

/// Crystal Factory module grade by id — the `KEMALIHPLCG` enum (one module per
/// crystal grade; `X.w.b[i].a`). Confirmed against the save (grades 0–5).
pub fn crystal_module_name(id: u32) -> Option<&'static str> {
    const NAMES: [&str; 6] = ["Physical", "Mystic", "Battle", "Creation", "Ultimate", "God"];
    NAMES.get(id as usize).copied()
}

/// Challenge name by id — the `OIDDHCOBPLG` enum (challenge struct field `a`,
/// `root.x.242` completion list). Ids are the enum's declaration order (None=0).
/// Display names come from the in-game name strings in `KPLPGPEOFNB` matched to
/// each abbreviation by initialism; the eleven the player has completed (UUC=2,
/// DRC=8, UPC=8, AAC=10, UBC=1, CPC=2, PMC=1, MMC=13, GPC=2, PLC=12, BHC=1) were
/// cross-checked against an in-game capture (2026-06-20). The five that had no
/// surviving UI string (OKC, PBC, OKBHC, RTI, TLC) were named from the player +
/// the wiki (2026-06-21); note RTI(30) = the "Road to Infinity" Day challenge,
/// distinct from TLC(37) = "RTI Temp Level Challenge" (an earlier pass wrongly
/// gave RTI's slot TLC's name). `UNUSED` is a retired slot.
pub fn challenge_name(id: u32) -> Option<&'static str> {
    Some(match id {
        1 => "Ultimate Universe Challenge",
        2 => "Black Hole Challenge",
        3 => "Double Rebirth Challenge",
        4 => "Ultimate Pet Challenge",
        5 => "God Skip Challenge",
        6 => "Clone Buildup Challenge",
        7 => "1000 Clone Challenge", // OKC — abbrev not shown in-game; wiki + player-confirmed
        8 => "No Divinity Challenge",
        9 => "Planet Multi Challenge",
        10 => "All Achievements Challenge",
        11 => "Ultimate Baal Challenge",
        12 => "No Rebirth Challenge",
        13 => "Ultimate Arty Challenge",
        14 => "Day Baal Challenge",
        15 => "Day Universe Challenge",
        16 => "Day Pet Challenge",
        17 => "Crystal Power Challenge",
        18 => "P. Baal Challenge", // PBC — abbrev not shown in-game; player-confirmed
        19 => "Ultimate Challenge Challenge",
        20 => "1K Clones Black Hole Challenge", // OKBHC — player-confirmed (abbrev shown in-game)
        21 => "Day Might Challenge",
        22 => "Day No Divinity Challenge",
        23 => "Total Might Challenge",
        24 => "No Rebirth Dungeon Challenge",
        25 => "Monument Multi Challenge",
        26 => "SpaceDim Challenge",
        27 => "Ultimate Beings V2 Challenge",
        28 => "No Div Monument Challenge",
        29 => "Overflow Challenge",
        30 => "Road to Infinity Challenge", // RTI — the Day challenge (distinct from TLC); was wrongly "RTI Temp Level Challenge"
        31 => "Patreon Gods Challenge",
        32 => "God Power Challenge",
        33 => "Ultimate Gods Challenge",
        34 => "One CC Challenge",
        35 => "Ultimate Beings V4 Challenge",
        36 => "Monster Queen Challenge",
        37 => "RTI Temp Level Challenge", // TLC — C# "TLCs" feed RTI level speed; player-confirmed
        38 => "Light Clone Challenge",
        39 => "Universes for Clones Challenge",
        40 => "Ultimate Stats Challenge",
        41 => "Day No Rebirth Challenge",
        42 => "Div Gen Challenge",
        43 => "Max Crystal Challenge",
        44 => "Ultimate Black Hole Challenge",
        45 => "No Training Challenge",
        46 => "Expensive Monument Challenge",
        47 => "True God Skip Challenge",
        48 => "Pet Level Challenge",
        49 => "Unused",
        50 => "Super Divinity Generator Challenge",
        51 => "Might Accumulation Challenge",
        52 => "Day God Power Challenge",
        53 => "Ultimate Multiverse Challenge",
        54 => "Day Multiverse Challenge",
        55 => "Greedy God Challenge",
        56 => "No Rebirth CP Challenge",
        57 => "Powerful Unleash Challenge",
        58 => "Ultimate Overflow Challenge",
        59 => "SpaceDim Accumulation Challenge",
        60 => "Day Extreme Building Challenge",
        61 => "Base Speed Challenge",
        62 => "Total Growth Challenge",
        63 => "SpaceDim Reset Challenge",
        64 => "Super Pet Level Challenge",
        65 => "Pet Crafting Challenge",
        66 => "Ultimate Being V1 Challenge",
        67 => "Limited Clone v4 Challenge",
        68 => "God Power Accumulation Challenge",
        69 => "Powersurge Challenge",
        70 => "No Might No Rebirth Challenge",
        71 => "Exhausted Training Challenge",
        72 => "Clone Creator Challenge",
        73 => "Limited Clone No Rebirth Challenge",
        74 => "Divinity Accumulation Challenge",
        75 => "Powerful Worker Challenge",
        76 => "Boosting Capacity Challenge",
        _ => return None,
    })
}

/// Challenge difficulty by id — the `HOLHIHDKBKA` enum (challenge struct field
/// `c`). The game folds `Mixed` to `Hard` on load, but the stored value keeps
/// the four declared variants.
pub fn challenge_difficulty_name(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "Normal",
        1 => "Hard",
        2 => "Root",
        3 => "Mixed",
        _ => return None,
    })
}

/// Whether a challenge (`OIDDHCOBPLG` id) is **score-based** — the Day challenges
/// plus Road to Infinity, whose ChP comes from a high-score statistic in `root.x`
/// rather than the `x.242` completion count. These are exactly the real-challenge
/// ids for which [`challenge_chp`] returns `None`. The per-challenge score-stat
/// `root.x` keys are tabulated in FINDINGS (e.g. Day Pet → x.049).
pub fn challenge_is_score_based(id: u32) -> bool {
    matches!(id, 14 | 15 | 16 | 21 | 22 | 30 | 41 | 52 | 54 | 60)
}

/// Challenge Points awarded **per completion** for a challenge id (`OIDDHCOBPLG`).
/// Returns `None` for the score-based **Day** challenges (Day*/Road to Infinity),
/// whose ChP comes from a high-score stat rather than the completion count, and
/// for `None`(0)/`UNUSED`(49). Values transcribed from each challenge's wiki page
/// (itrtg.wiki.gg, 2026-06-21); cross-checked exactly against an in-game capture
/// (20 AAC + 4 UBHC + 4 UBC + 12 UPC + 14 MMC + 4 PMC + 12 PLC + 8 GPC + 4 BHC =
/// 1040 ChP). So total ChP = Σ(completions × challenge_chp) over non-Day
/// challenges + the Day-challenge score contributions.
pub fn challenge_chp(id: u32) -> Option<u32> {
    Some(match id {
        1 => 1,    // Ultimate Universe
        2 => 2,    // Black Hole
        3 => 10,   // Double Rebirth
        4 => 8,    // Ultimate Pet
        5 => 8,    // God Skip
        6 => 20,   // Clone Buildup
        7 => 10,   // 1000 Clone
        8 => 20,   // No Divinity
        9 => 10,   // Planet Multi
        10 => 30,  // All Achievements
        11 => 25,  // Ultimate Baal
        12 => 40,  // No Rebirth
        13 => 30,  // Ultimate Arty
        // 14 Day Baal — score-based
        // 15 Day Universe — score-based
        // 16 Day Pet — score-based
        17 => 10,  // Crystal Power
        18 => 20,  // P. Baal
        19 => 45,  // Ultimate Challenge
        20 => 35,  // 1K Clones Black Hole
        // 21 Day Might — score-based
        // 22 Day No Divinity — score-based
        23 => 10,  // Total Might
        24 => 40,  // No Rebirth Dungeon
        25 => 4,   // Monument Multi
        26 => 15,  // SpaceDim
        27 => 10,  // Ultimate Beings V2
        28 => 20,  // No Div Monument
        29 => 1,   // Overflow
        // 30 Road to Infinity — score-based
        31 => 20,  // Patreon Gods
        32 => 1,   // God Power
        33 => 15,  // Ultimate Gods
        34 => 15,  // One CC
        35 => 20,  // Ultimate Beings V4
        36 => 15,  // Monster Queen
        37 => 25,  // RTI Temp Level
        38 => 20,  // Light Clone
        39 => 30,  // Universes for Clones
        40 => 20,  // Ultimate Stats
        // 41 Day No Rebirth — score-based
        42 => 25,  // Div Gen
        43 => 30,  // Max Crystal
        44 => 30,  // Ultimate Black Hole
        45 => 20,  // No Training
        46 => 30,  // Expensive Monument
        47 => 20,  // True God Skip
        48 => 1,   // Pet Level
        // 49 UNUSED
        50 => 35,  // Super Divinity Generator
        51 => 12,  // Might Accumulation
        // 52 Day God Power — score-based
        53 => 25,  // Ultimate Multiverse
        // 54 Day Multiverse — score-based
        55 => 20,  // Greedy God
        56 => 60,  // No Rebirth CP
        57 => 10,  // Powerful Unleash
        58 => 10,  // Ultimate Overflow
        59 => 20,  // SpaceDim Accumulation
        // 60 Day Extreme Building — score-based
        61 => 25,  // Base Speed
        62 => 10,  // Total Growth
        63 => 20,  // SpaceDim Reset
        64 => 20,  // Super Pet Level
        65 => 25,  // Pet Crafting
        66 => 25,  // Ultimate Being V1
        67 => 40,  // Limited Clone v4
        68 => 20,  // God Power Accumulation
        69 => 15,  // Powersurge
        70 => 50,  // No Might No Rebirth
        71 => 12,  // Exhausted Training
        72 => 15,  // Clone Creator
        73 => 50,  // Limited Clone No Rebirth
        74 => 10,  // Divinity Accumulation
        75 => 20,  // Powerful Worker
        76 => 15,  // Boosting Capacity
        _ => return None,
    })
}

/// Ultimate-Overflow upgrade type by id — the `IDFOIHJPCHP` enum (entry field
/// `a` of the `root.029` list, marker `UltimateOverflowBoost`). These are the
/// boosts bought with Ultimate Overflow Points; names transcribed from the enum.
pub fn ultimate_overflow_upgrade_name(id: u32) -> Option<&'static str> {
    Some(match id {
        1 => "Dungeon Slot",
        2 => "Multiverse Rebirth Multi",
        3 => "Multiverse GP Increase",
        4 => "Multiverse Growth %",
        5 => "Multiverse Growth Levels",
        6 => "Higher PBaal",
        _ => return None,
    })
}

/// RTI (Road to Infinity) bonus type by id — the `BDAFIPJBPFN` enum (entry
/// field `a` of the `root.014.a` list, marker `RtiElement`). One entry per stat
/// type; names transcribed from the enum (the game derives the display name from
/// the same enum via `ToString`).
pub fn rti_bonus_name(id: u32) -> Option<&'static str> {
    Some(match id {
        1 => "Physical",
        2 => "Mystic",
        3 => "Battle",
        4 => "Creating",
        5 => "TBS",
        6 => "GodCrystal",
        7 => "SpaceDim",
        8 => "Divinity",
        9 => "BuildingSpeed",
        10 => "CreatingSpeed",
        _ => return None,
    })
}

/// Adventure-mode inventory item name by id (the `032.d` namespace — distinct
/// from the main `X.Q` materials and from the core/enemy ids). Player-identified
/// 2026-06-18 by matching the save's `032.d` list (id `a`, count `b`) against the
/// in-game Adventure inventory (a full 32-item Steam export plus Flask from a
/// Kongregate save). Ids cluster by material family (raw / refined / enhanced /
/// jewel). Ids not held in those saves return `None`.
pub fn adventure_item_name(id: u32) -> Option<&'static str> {
    // Complete `BFNFKADNAKD` enum (Assembly-CSharp) — the adventure crafting
    // material namespace (also the type for `032.d` inventory and the AdvCrafting
    // material fields). Raw/refined/enhanced families by tier.
    Some(match id {
        1 => "Sticky Fluid",
        2 => "Rough Hide",
        3 => "Bag of Sand",
        4 => "Heat",
        5 => "Smooth Hide",
        6 => "Feather",
        7 => "Smooth Leather",
        8 => "Silky Cloth",
        9 => "Fairy Wing",
        10 => "Fairy Leather",
        11 => "Lizard Leather",
        12 => "Lizard Skin",
        50 => "Cloth",
        51 => "Leather",
        52 => "Paper",
        53 => "Common Herb",
        54 => "Uncommon Herb",
        55 => "Rare Herb",
        56 => "Super Herb",
        57 => "Godly Herb",
        58 => "Common Mana Herb",
        59 => "Uncommon Mana Herb",
        60 => "Rare Mana Herb",
        61 => "Super Mana Herb",
        62 => "Godly Mana Herb",
        63 => "Flask",
        64 => "Small Bottle",
        65 => "Medium Bottle",
        66 => "Big Bottle",
        67 => "Godly Bottle",
        68 => "Fire Flower",
        69 => "Rainshroom",
        70 => "Misty Flower",
        71 => "Geo Root",
        72 => "Shadow Flower",
        73 => "Luminary Flower",
        74 => "Tough Root",
        75 => "Energy Flower",
        76 => "Berserk Shroom",
        77 => "Wise Herb",
        78 => "Dragon Flower",
        100 => "Scrap Metal",
        101 => "Iron Ore",
        102 => "Copper Ore",
        103 => "Tin Ore",
        104 => "Blood Stone",
        105 => "Cobalt Ore",
        106 => "Mythril Ore",
        107 => "Magic Ore",
        108 => "Godly Ore",
        120 => "Metal Bar",
        121 => "Iron Bar",
        122 => "Copper Bar",
        123 => "Bronze Bar",
        124 => "Blood Bar",
        125 => "Cobalt Bar",
        126 => "Mythril Bar",
        127 => "Magic Bar",
        128 => "Godly Bar",
        150 => "Pine Plank",
        151 => "Beech Plank",
        152 => "Oak Plank",
        153 => "Teak Plank",
        154 => "Maple Plank",
        155 => "Ebony Plank",
        156 => "Magic Plank",
        157 => "Godly Plank",
        199 => "Golden Chestnut",
        200 => "Refined Cloth",
        201 => "Refined Leather",
        202 => "Refined Silky",
        203 => "Refined Smoothy",
        204 => "Refined Wing",
        205 => "Refined Lizard",
        220 => "Refined Metal",
        221 => "Refined Iron",
        222 => "Refined Bronze",
        223 => "Refined Bloodstone",
        224 => "Refined Cobalt",
        225 => "Refined Mythril",
        226 => "Refined Magic Bar",
        227 => "Refined Godly Bar",
        240 => "Enhanced Pine",
        241 => "Enhanced Beech",
        242 => "Enhanced Oak",
        243 => "Enhanced Teak",
        244 => "Enhanced Maple",
        245 => "Enhanced Ebony",
        246 => "Enhanced Magic Plank",
        247 => "Enhanced Godly Plank",
        260 => "Enhanced Paper",
        261 => "Fire Jewel",
        262 => "Water Jewel",
        263 => "Earth Jewel",
        264 => "Wind Jewel",
        265 => "Dark Jewel",
        266 => "Light Jewel",
        267 => "Magic Powder",
        268 => "Toughness Pill",
        269 => "Speedy Pill",
        270 => "Berserk Pill",
        271 => "Wise Pill",
        272 => "Critical Pill",
        273 => "Ultimate Pill",
        1000 => "Event",
        1001 => "Pet Stone",
        1002 => "Growth",
        _ => return None,
    })
}

/// Adventure-mode **enemy** name by id (the `032.G` core namespace — a core is
/// "<enemy> <quality>", e.g. "Slime SSS"). Distinct from both the adventure-item
/// ids and the training Monster list. Player-identified 2026-06-18 from a Steam
/// save holding cores from all seven of these enemies.
pub fn adventure_enemy_name(id: u32) -> Option<&'static str> {
    // The full `NFKHCMANAKF` entity enum (Assembly-CSharp): Player + the complete
    // adventure enemy/boss roster. Used by both the monster cores (`032.G[i].a`)
    // and the adventurer/enemy battle-stats entity (`032.b.a`). Separate id
    // namespace from items (id 53 = Astaroth here, "Common Herb" as an item).
    Some(match id {
        1 => "Player",
        50 => "Slime",
        51 => "Akuma",
        52 => "Amon",
        53 => "Astaroth",
        54 => "Asura",
        55 => "Belial",
        56 => "Baphomet",
        57 => "Bifrons",
        58 => "Empress",
        59 => "Dagon",
        60 => "Devil",
        61 => "Fire Lizard",
        62 => "Blood Lizard",
        63 => "Goblin",
        64 => "Ghoul",
        65 => "Gorgon",
        66 => "Haagenti",
        67 => "Ifrit",
        68 => "Incubus",
        69 => "Imp",
        70 => "Jinn",
        71 => "Krampus",
        72 => "Wraith",
        73 => "Lamia",
        74 => "Lilim",
        75 => "Lilith",
        76 => "Lucifer",
        77 => "Mammon",
        78 => "Marchosias",
        79 => "Naberius",
        80 => "Oni",
        81 => "Phenex",
        82 => "Raum",
        83 => "Rakshasa",
        84 => "Satan",
        85 => "Succubus",
        86 => "Shax",
        87 => "Shinigami",
        88 => "Tengu",
        89 => "Ukobach",
        90 => "Vassago",
        91 => "Grim Reaper",
        92 => "Tyrant",
        93 => "Fairy",
        94 => "Tree Golem",
        95 => "Gloom Flower",
        300 => "Dark Reaper",
        480 => "Squirrel Mage",
        500 => "Weak Pumpkin",
        501 => "Normal Pumpkin",
        502 => "Strong Pumpkin",
        503 => "Valentine Newbie",
        504 => "Valentine Easy",
        505 => "Valentine Normal",
        506 => "Valentine Hard",
        507 => "Valentine Hardcore",
        _ => return None,
    })
}

/// Adventure-mode **class** name by id — the game's `APJDLMDFIGI` enum
/// (`Assembly-CSharp`), the adventurer's class (`root.032.b.e`).
pub fn adventure_class_name(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "None",
        1 => "Newbie",
        2 => "Adventurer",
        3 => "Squire",
        4 => "Student",
        5 => "Thief",
        6 => "Archer",
        7 => "Warrior",
        8 => "Fighter",
        9 => "Mage",
        10 => "Cleric",
        20 => "Rogue",
        21 => "Assassin",
        22 => "Sniper",
        23 => "Knight",
        24 => "Pyromancer",
        25 => "Aeromancer",
        26 => "Geomancer",
        27 => "Aquamancer",
        28 => "Priest",
        29 => "Hunter",
        30 => "Monk",
        31 => "Scholar",
        32 => "Alchemist",
        40 => "Paladin",
        41 => "Samurai",
        42 => "Ninja",
        43 => "Magic Shooter",
        44 => "Holy Archer",
        45 => "Sage",
        46 => "Chrono Mage",
        100 => "Onion Knight",
        101 => "Tea Rogue",
        102 => "Ice Cream Wizard",
        _ => return None,
    })
}

/// Adventure-mode **skill** name by id — the game's `ADCGDPGPBOI` enum
/// (`Assembly-CSharp`); the adventurer's skill lists (`root.032.b.h`/`i`) are
/// lists of these ids.
pub fn adventure_skill_name(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "None",
        1 => "Basic Attack",
        2 => "First Aid",
        3 => "Bash",
        4 => "Prepare",
        5 => "Drops Boost",
        6 => "Speed Boost",
        7 => "Attack",
        8 => "Quick Hit",
        9 => "Attack Boost",
        10 => "Defense Boost",
        11 => "Magic Arrow",
        12 => "Random Blast",
        13 => "Int Boost",
        14 => "Res Boost",
        15 => "Double Attack",
        16 => "Steal",
        17 => "Hide",
        18 => "Dagger Mastery",
        19 => "Dodge",
        20 => "Double Shot",
        21 => "Trap",
        22 => "Arrow Rain",
        23 => "Bow Mastery",
        24 => "Ambush",
        25 => "Aura Blade",
        26 => "Defense Break",
        27 => "Sword Rush",
        28 => "Sword Mastery",
        29 => "HP Regen",
        30 => "Aura Shot",
        31 => "Dragon Punch",
        32 => "Whirling Foot",
        33 => "Fist Mastery",
        34 => "Counter Strike",
        35 => "Magic Missile",
        36 => "Elemental Blast",
        37 => "Push Blast",
        38 => "Wand Mastery",
        39 => "Casting Boost",
        40 => "Heal",
        41 => "Holy Light",
        42 => "Bless",
        43 => "Book Mastery",
        44 => "Ailment Res",
        45 => "Throw Sand",
        46 => "Back Stab",
        47 => "Binding Shot",
        48 => "Dual Wield",
        49 => "Extra Attack",
        50 => "Poison Attack",
        51 => "Killing Strike",
        52 => "Smoke Screen",
        53 => "Poison Boost",
        54 => "Stealth",
        55 => "Sharp Shooting",
        56 => "Mark Target",
        57 => "Charge Up",
        58 => "Concentration",
        59 => "Hit Boost",
        60 => "Pierce",
        61 => "Empower HP",
        62 => "Rapid Stabs",
        63 => "Spear Mastery",
        64 => "HP Boost",
        65 => "Fire Ball",
        66 => "Fire Pillar",
        67 => "Explosion",
        68 => "Fire Boost",
        69 => "Fire Resistance",
        70 => "Gust",
        71 => "Air Compression",
        72 => "Wind Splicer",
        73 => "Wind Boost",
        74 => "Wind Resistance",
        75 => "Rock Shot",
        76 => "Stone Barrier",
        77 => "Earth Quake",
        78 => "Earth Boost",
        79 => "Earth Resistance",
        80 => "Water Punch",
        81 => "Whirlpool",
        82 => "Tsunami",
        83 => "Water Boost",
        84 => "Water Resistance",
        85 => "Careful Shot",
        86 => "Weakening Shot",
        87 => "Aimed Shot",
        88 => "Core Boost",
        89 => "Multi Arrows",
        90 => "Holy Ray",
        91 => "Dispel",
        92 => "Prayer",
        93 => "Light Boost",
        94 => "Bless Mastery",
        95 => "Shield Charge",
        96 => "Holy Cross",
        97 => "Sacrifice",
        98 => "Shield Mastery",
        99 => "Defensive Aura",
        100 => "Onion Slash",
        101 => "Onion Wave",
        102 => "Weapon Mastery",
        103 => "Analyze",
        104 => "Safe Distance",
        105 => "Taking Notes",
        106 => "Speedy Analyze",
        107 => "Smart Analyze",
        108 => "Throw Potion",
        109 => "Alchemic Reaction",
        110 => "Potion Inventor",
        111 => "Potion Slots",
        112 => "Pill Inventor",
        113 => "Brew Tea",
        114 => "Drink Tea",
        115 => "Tea Shot",
        116 => "Tea Boost",
        117 => "Patience",
        118 => "Kyrie Eleyson",
        119 => "Celestial Arrow",
        120 => "Celestial Ray",
        121 => "Divine Boost",
        122 => "True Sight",
        300 => "Dark Boost",
        301 => "Light Resistance",
        302 => "Dark Resistance",
        400 => "Dark Slash",
        401 => "Counter Dodge",
        402 => "Heal Counter",
        403 => "Newbie Love Shot",
        404 => "Easy Love Shot",
        405 => "Love Shot",
        406 => "Hard Love Shot",
        407 => "Mighty Love Shot",
        408 => "Soul Slash",
        409 => "Charge Attack",
        410 => "Blind Enemy",
        411 => "Holy Slash",
        412 => "Holy Power Slash",
        413 => "Sense",
        414 => "Arrow of Light",
        415 => "Exp Boost",
        416 => "Flee",
        417 => "Earth Blast",
        418 => "Dark Blast",
        419 => "Fire Slash",
        420 => "Dark Revenge",
        421 => "Bloody Slash",
        422 => "Blood Drain",
        423 => "Survival Instinct",
        424 => "Electric Beam",
        425 => "Magic Blast",
        600 => "Push Up x100",
        601 => "Sit Up x100",
        602 => "Squat 100",
        603 => "Run 10k",
        1000 => "Weak Attack",
        1001 => "Mana Arrow",
        _ => return None,
    })
}

/// Adventure-mode **craftable gear** name by id — the game's `LEIFLPFLEHJ` enum
/// (`Assembly-CSharp`); the AdvCrafting target (`032.b.k → MANFDMLBOMG.a`).
/// Armor sets 1–51, wands/bows/books 200–212, accessories 400/499 & 700–712,
/// weapons 500–521, metal armor 600–619.
pub fn adventure_craft_gear_name(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "None",
        1 => "Cloth Mantle",
        2 => "Cloth Bracers",
        3 => "Cloth Boots",
        4 => "Cloth Hood",
        5 => "Cloth Pants",
        6 => "Leather Belt",
        7 => "Leather Shirt",
        8 => "Leather Bracers",
        9 => "Leather Boots",
        10 => "Leather Helmet",
        11 => "Leather Pants",
        12 => "Blazing Mantle",
        13 => "Blazing Bracers",
        14 => "Blazing Boots",
        15 => "Blazing Hood",
        16 => "Blazing Pants",
        17 => "Aim Chest Piece",
        18 => "Aim Bracers",
        19 => "Aim Boots",
        20 => "Aim Hat",
        21 => "Aim Pants",
        22 => "Dark Leather Mantle",
        23 => "Dark Leather Bracers",
        24 => "Dark Leather Boots",
        25 => "Dark Leather Hood",
        26 => "Dark Leather Pants",
        27 => "Holy Chest Piece",
        28 => "Holy Bracers",
        29 => "Holy Boots",
        30 => "Holy Hood",
        31 => "Holy Pants",
        32 => "Training Mail",
        33 => "Training Bracers",
        34 => "Training Boots",
        35 => "Training Helmet",
        36 => "Training Pants",
        37 => "Hunting Shirt",
        38 => "Hunting Bracers",
        39 => "Hunting Boots",
        40 => "Hunting Hood",
        41 => "Hunting Pants",
        42 => "Earthen Mantle",
        43 => "Earthen Bracers",
        44 => "Earthen Boots",
        45 => "Earthen Mask",
        46 => "Earthen Skirt",
        47 => "Aero Chest Piece",
        48 => "Aero Bracers",
        49 => "Aero Boots",
        50 => "Aero Hood",
        51 => "Aero Skirt",
        200 => "Wooden Wand",
        201 => "Magic Book",
        202 => "Short Bow",
        203 => "Hidden Book",
        204 => "Beech Bow",
        205 => "Oak Bow",
        206 => "Blazing Wand",
        207 => "Holy Book",
        208 => "Earthen Wand",
        209 => "Wise Book",
        210 => "Teak Bow",
        211 => "Teak Wand",
        212 => "Aero Wand",
        400 => "Acc 1",
        499 => "Tsury Finke",
        500 => "Rusty Knife",
        501 => "Metal Knuckles",
        502 => "Metal Dagger",
        503 => "Wooden Shield",
        504 => "Metal Sword",
        505 => "Metal Spear",
        506 => "Iron Knuckles",
        507 => "Iron Dagger",
        508 => "Iron Shield",
        509 => "Iron Sword",
        510 => "Iron Spear",
        511 => "Iron Wand",
        512 => "Bronze Knuckles",
        513 => "Bronze Dagger",
        514 => "Bronze Shield",
        515 => "Bronze Sword",
        516 => "Bronze Spear",
        517 => "Cobalt Knuckles",
        518 => "Cobalt Dagger",
        519 => "Cobalt Shield",
        520 => "Cobalt Sword",
        521 => "Cobalt Spear",
        600 => "Metal Helmet",
        601 => "Metal Boots",
        602 => "Metal Bracers",
        603 => "Metal Pants",
        604 => "Metal Chest",
        605 => "Iron Helmet",
        606 => "Iron Boots",
        607 => "Iron Bracers",
        608 => "Iron Pants",
        609 => "Iron Chest",
        610 => "Bronze Helmet",
        611 => "Bronze Boots",
        612 => "Bronze Bracers",
        613 => "Bronze Pants",
        614 => "Bronze Chest",
        615 => "Aquatic Helmet",
        616 => "Aquatic Boots",
        617 => "Aquatic Bracers",
        618 => "Aquatic Pants",
        619 => "Aquatic Chest",
        700 => "Metal Ring",
        701 => "Metal Necklace",
        702 => "Golden Belt",
        703 => "Glass Dagger",
        704 => "Bronze Necklace",
        705 => "Bronze Ring",
        706 => "Cobalt Necklace",
        707 => "Cobalt Ring",
        708 => "Golden Cape",
        709 => "Golden Necklace",
        710 => "Golden Ring",
        711 => "Spy Glass",
        712 => "Cauldron",
        _ => return None,
    })
}

/// Adventure-mode **alchemy recipe** name by id — the game's `DLCMNADKOJK` enum
/// (`Assembly-CSharp`); the AdvAlchemy product (`032.b.n → JADFDPJGJPA.a`).
/// HP/MP potion tiers (Basic→Godly), an `I`-prefixed second series, and the
/// effect potions. (Sequential enum, ids 0–26.)
pub fn adventure_recipe_name(id: u32) -> Option<&'static str> {
    const NAMES: [&str; 27] = [
        "None",
        "Basic HP",
        "HP",
        "Good HP",
        "Super HP",
        "Godly HP",
        "Basic MP",
        "MP",
        "Good MP",
        "Super MP",
        "Godly MP",
        "I Basic HP",
        "I HP",
        "I Good HP",
        "I Super HP",
        "I Godly HP",
        "I Basic MP",
        "I MP",
        "I Good MP",
        "I Super MP",
        "I Godly MP",
        "Burning",
        "Freezing",
        "Geo",
        "Acid",
        "Black",
        "Luminary",
    ];
    NAMES.get(id as usize).copied()
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
/// (2026-06-17): F E D C B A S SS SSS for ids 0…8, matching the game's
/// `GBFGHANMFII` enum. That enum also defines a 10th tier `Ult` (id 9), but the
/// save's equipment loader **clamps stored quality to 8 (SSS)** on read, so a 9
/// never persists in a real save; it is included here for completeness.
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
        9 => "Ult",
        _ => return None,
    })
}

/// Parse a quality from a letter grade (`F E D C B A S SS SSS`, `Ult`,
/// case-insensitive) or a numeric string (`0`–`9`). Inverse of [`quality_name`];
/// lets the equipment editor accept either form.
pub fn quality_from_str(s: &str) -> Option<u32> {
    let t = s.trim();
    if let Ok(n) = t.parse::<u32>() {
        return (n <= 9).then_some(n);
    }
    Some(match t.to_ascii_uppercase().as_str() {
        "F" => 0,
        "E" => 1,
        "D" => 2,
        "C" => 3,
        "B" => 4,
        "A" => 5,
        "S" => 6,
        "SS" => 7,
        "SSS" => 8,
        "ULT" => 9,
        _ => return None,
    })
}

/// Campaign-boost % a piece of campaign-boost gear gives the pet it's equipped
/// on, at the given quality (`quality_id`: F=0…SSS=8) and upgrade `plus`.
///
/// From the game's C# (`DOBKHNKLLLM`): `base × (1 + quality_id) × (1 + plus)`,
/// where the `CampaignBoost` effect's base is `0.088185 × factor` (the per-item
/// factor `NJDOCOGAJEM`). Only the two campaign-boost items are covered (other
/// gear gives different effects); returns `None` for anything else.
///
/// - **Magic Stick** (51): factor 3 → base `0.264555`. SSS+20 = 50.0%, matching
///   its tooltip "(0.2646% × (1 + upgrade level) × quality multiplier)", up to 50%.
/// - **Candy Cane** (300): factor 6 → base `0.52911` (2× Magic Stick). It's the
///   only item upgradable to +30, and at **SSS** the game hardcodes three
///   milestones — +20→101%, +25→125%, +30→150% — while every other level uses the
///   general formula (SSS+21 = 0.52911·9·22 = 104.76%, player-confirmed).
pub fn campaign_boost_pct(type_id: u32, quality_id: u32, plus: u32) -> Option<f64> {
    // base = 0.088185 (CampaignBoost effect) × per-item factor.
    let base = match type_id {
        51 => 0.088_185 * 3.0,  // Magic Stick
        300 => 0.088_185 * 6.0, // Candy Cane
        _ => return None,
    };
    // Candy Cane's hardcoded SSS milestones (it alone reaches +30).
    if type_id == 300 && quality_id == 8 {
        match plus {
            20 => return Some(101.0),
            25 => return Some(125.0),
            30 => return Some(150.0),
            _ => {}
        }
    }
    Some(base * (1.0 + quality_id as f64) * (1.0 + plus as f64))
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
        assert_eq!(equipment_category(48), Some(Weapon)); // Magic Hammer (resolved 2026-06-19)
        assert_eq!(equipment_category(49), None); // still-unidentified type
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
        // Elemental bars, ordering from the NCPJFPLCPPK enum. 34/36/37 were
        // mis-assigned before (all count 10, so guessed); the enum corrects them.
        assert_eq!(material_name(33), Some("Inferno Bar")); // fire
        assert_eq!(material_name(34), Some("Forest Bar")); // earth
        assert_eq!(material_name(35), Some("Hurricane Bar")); // wind
        assert_eq!(material_name(36), Some("Titanium Bar")); // neutral
        assert_eq!(material_name(37), Some("Tsunami Bar")); // water
        assert_eq!(material_name(19), Some("Antidote")); // not "Nothing"
        assert_eq!(material_name(21), Some("Torch"));
        assert_eq!(material_name(16), Some("Health Potion X"));
        assert_eq!(material_name(17), Some("Health Potion S"));
    }

    #[test]
    fn full_material_enum_added() {
        // Spot-checks of ids that the curated subset never covered, now from the
        // full NCPJFPLCPPK enum.
        assert_eq!(material_name(18), Some("Elixir"));
        assert_eq!(material_name(38), Some("Golden Key"));
        assert_eq!(material_name(52), Some("Super Lucky Talisman"));
        assert_eq!(material_name(106), Some("Undine")); // water-quest family
        assert_eq!(material_name(123), Some("Horn of Balrog"));
        assert_eq!(material_name(136), Some("Dark Matter"));
        assert_eq!(material_name(142), Some("Salamander Soul"));
        assert_eq!(material_name(169), Some("Shiny Metal Stone"));
        assert_eq!(material_name(350), Some("Spark of Genius"));
        assert_eq!(material_name(500), Some("Stick Rod")); // fishing rod (NCPJFPLCPPK.Rod1)
        assert_eq!(material_name(503), Some("Voodoo Rod"));
        assert_eq!(material_name(522), Some("Big Worm")); // bait (NCPJFPLCPPK.Bait3)
        assert_eq!(material_name(547), Some("Kraken"));
        assert_eq!(material_name(801), Some("Ultimate Reward"));
        // Foods are the same enum (stored elsewhere) but still name.
        assert_eq!(material_name(102), Some("Puny Food"));
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
    fn bombs_traps_and_elemental_quest_materials() {
        // Player-confirmed 2026-06-18.
        assert_eq!(material_name(30), Some("Melting Bomb"));
        assert_eq!(material_name(48), Some("Nanotrap"));
        assert_eq!(material_name(49), Some("Freezing Bomb"));
        assert_eq!(material_name(120), Some("Cure"));
        // Elemental-pet evolution-quest materials: Salamander/fire (138–141)
        // and Sylph/wind (146–149), with the formerly count-0 ids now named.
        assert_eq!(material_name(139), Some("Igneous Bones"));
        assert_eq!(material_name(140), Some("Pliable Magma"));
        assert_eq!(material_name(148), Some("Mysteries of the Wind"));
        // The two "Nothing"-adjacent count-1 singletons, assigned from the enum.
        assert_eq!(material_name(167), Some("Food Journal One"));
        assert_eq!(material_name(168), Some("Food Journal Two"));
        // X.Q id 120 = Cure is a different namespace from adventure-item 120
        // (Metal Bar) — the two tables stay separate.
        assert_eq!(adventure_item_name(120), Some("Metal Bar"));
    }

    #[test]
    fn unknown_ids_return_none() {
        assert_eq!(material_name(0), None);
        assert_eq!(material_name(9999), None);
        // 175-349 / 353-499 / 568-799 are gaps in the enum → None.
        assert_eq!(material_name(300), None);
        assert_eq!(material_name(600), None);
        // 130 (Aether Ring) and 162 (Monster Blood) are now known.
        assert_eq!(material_name(130), Some("Aether Ring"));
        assert_eq!(material_name(162), Some("Monster Blood"));
        // Two of the count-1 singletons resolved (160/164); a Salamander
        // upgrade item (145).
        assert_eq!(material_name(160), Some("Not Nothing"));
        assert_eq!(material_name(164), Some("Absolutely Nothing"));
        assert_eq!(material_name(145), Some("Prosthetic Tail"));
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
        // Resolved 2026-06-19 (Anteater / Salamander / Caterpillar). The real
        // Magic Hammer is type 48 (the old "44 = Magic Hammer | Storm Ring" tie
        // went to Storm Ring); 80/81 complete the Legendary family with 79.
        assert_eq!(equipment_type_name(48), Some("Magic Hammer"));
        assert_eq!(equipment_type_name(80), Some("Legendary Stick"));
        assert_eq!(equipment_type_name(81), Some("Legendary Pot"));
        assert_eq!(equipment_category(48), Some(EquipCategory::Weapon));
        // Full-enum coverage: ids outside the curated category list still name.
        assert_eq!(equipment_type_name(1), Some("Iron Vest"));
        assert_eq!(equipment_type_name(63), Some("Gram"));
        assert_eq!(equipment_type_name(250), Some("Neutral Crafting Sword"));
        assert_eq!(equipment_type_name(311), Some("Christmas Boots"));
        assert_eq!(equipment_type_name(999), None);
    }

    #[test]
    fn equipment_types_table_agrees_with_full_enum_names() {
        // EQUIPMENT_TYPES is the curated category/builder source; its names must
        // never disagree with the authoritative full `equipment_type_name` table.
        for (id, name, _cat) in EQUIPMENT_TYPES {
            assert_eq!(
                equipment_type_name(*id),
                Some(*name),
                "EQUIPMENT_TYPES name for id {id} disagrees with the full enum table"
            );
        }
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
        // Full NFKHCMANAKF roster now (entity enum, also used by 032.b.a).
        assert_eq!(adventure_enemy_name(1), Some("Player"));
        assert_eq!(adventure_enemy_name(91), Some("Grim Reaper"));
        assert_eq!(adventure_enemy_name(300), Some("Dark Reaper"));
        assert_eq!(adventure_enemy_name(507), Some("Valentine Hardcore"));
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

    #[test]
    fn equipment_type_ambiguities_resolved_from_enum() {
        // The five formerly-ambiguous ids, now pinned from the `MBBDNNAMMHO`
        // equipment-type enum (all weapons).
        assert_eq!(equipment_type_name(23), Some("Flood Spear"));
        assert_eq!(equipment_type_name(26), Some("Tree Axe"));
        assert_eq!(equipment_type_name(30), Some("Hurricane Bow"));
        assert_eq!(equipment_type_name(52), Some("Iron Pot"));
        assert_eq!(equipment_type_name(56), Some("Leeching Sword"));
        for id in [23, 26, 30, 52, 56] {
            assert_eq!(equipment_category(id), Some(EquipCategory::Weapon));
        }
    }

    #[test]
    fn feeding_setting_names() {
        assert_eq!(feeding_setting_name(0), Some("None"));
        assert_eq!(feeding_setting_name(1), Some("Puny"));
        assert_eq!(feeding_setting_name(2), Some("Strong"));
        assert_eq!(feeding_setting_name(3), Some("Mighty"));
        assert_eq!(feeding_setting_name(4), Some("Chocolate"));
        assert_eq!(feeding_setting_name(5), Some("Free"));
        assert_eq!(feeding_setting_name(6), Some("Starve"));
        assert_eq!(feeding_setting_name(7), None);
    }

    #[test]
    fn gem_element_names() {
        // Base 5 (shared with pets/dungeons).
        assert_eq!(gem_element_name(0), Some("Neutral"));
        assert_eq!(gem_element_name(4), Some("Wind"));
        // Gem-only elements the 5-element model can't name.
        assert_eq!(gem_element_name(5), Some("Dark"));
        assert_eq!(gem_element_name(6), Some("Light"));
        assert_eq!(gem_element_name(50), Some("Elemental"));
        assert_eq!(gem_element_name(99), Some("All"));
        assert_eq!(gem_element_name(7), None);
    }

    #[test]
    fn campaign_boost_matches_ingame_values() {
        let approx = |a: f64, b: f64| (a - b).abs() < 0.01;
        // Magic Stick (51) SSS+20 = 50.0% ("up to 50%" per tooltip).
        assert!(approx(campaign_boost_pct(51, 8, 20).unwrap(), 50.0));
        // Candy Cane (300) SSS: hardcoded milestones + general formula between.
        assert_eq!(campaign_boost_pct(300, 8, 20), Some(101.0));
        assert!(approx(campaign_boost_pct(300, 8, 21).unwrap(), 104.76)); // player-confirmed
        assert_eq!(campaign_boost_pct(300, 8, 25), Some(125.0));
        assert_eq!(campaign_boost_pct(300, 8, 30), Some(150.0));
        // Candy Cane is exactly 2× Magic Stick at the same quality/plus (away
        // from the overrides).
        assert!(approx(
            campaign_boost_pct(300, 8, 21).unwrap(),
            2.0 * campaign_boost_pct(51, 8, 21).unwrap()
        ));
        // Non-campaign-boost gear isn't covered.
        assert_eq!(campaign_boost_pct(304, 8, 20), None); // Magic Egg
    }

    #[test]
    fn adventure_class_and_skill_names() {
        // Classes (APJDLMDFIGI) — sparse id ranges.
        assert_eq!(adventure_class_name(7), Some("Warrior"));
        assert_eq!(adventure_class_name(20), Some("Rogue"));
        assert_eq!(adventure_class_name(46), Some("Chrono Mage"));
        assert_eq!(adventure_class_name(102), Some("Ice Cream Wizard"));
        assert_eq!(adventure_class_name(11), None); // gap (10→20)
        // Skills (ADCGDPGPBOI).
        assert_eq!(adventure_skill_name(1), Some("Basic Attack"));
        assert_eq!(adventure_skill_name(29), Some("HP Regen"));
        assert_eq!(adventure_skill_name(122), Some("True Sight"));
        assert_eq!(adventure_skill_name(405), Some("Love Shot"));
        assert_eq!(adventure_skill_name(603), Some("Run 10k"));
        assert_eq!(adventure_skill_name(1001), Some("Mana Arrow"));
        assert_eq!(adventure_skill_name(200), None); // gap
    }

    #[test]
    fn adventure_craft_and_recipe_names() {
        assert_eq!(adventure_craft_gear_name(1), Some("Cloth Mantle"));
        assert_eq!(adventure_craft_gear_name(200), Some("Wooden Wand"));
        assert_eq!(adventure_craft_gear_name(509), Some("Iron Sword"));
        assert_eq!(adventure_craft_gear_name(712), Some("Cauldron"));
        assert_eq!(adventure_craft_gear_name(52), None); // gap (51→200)
        assert_eq!(adventure_recipe_name(1), Some("Basic HP"));
        assert_eq!(adventure_recipe_name(10), Some("Godly MP"));
        assert_eq!(adventure_recipe_name(26), Some("Luminary"));
        assert_eq!(adventure_recipe_name(27), None);
    }

    #[test]
    fn campaign_type_names() {
        assert_eq!(campaign_type_name(0), Some("Growth"));
        assert_eq!(campaign_type_name(3), Some("Item"));
        assert_eq!(campaign_type_name(6), Some("GodPower"));
        assert_eq!(campaign_type_name(8), Some("Event"));
        assert_eq!(campaign_type_name(9), None);
    }

    #[test]
    fn quality_ladder_includes_ult_tier() {
        // The save clamps stored quality to 8 (SSS), but the GBFGHANMFII enum
        // names a 10th tier at id 9.
        assert_eq!(quality_name(8), Some("SSS"));
        assert_eq!(quality_name(9), Some("Ult"));
        assert_eq!(quality_name(10), None);
    }

    #[test]
    fn dungeon_ids_map_to_names() {
        // Player-confirmed: the three running dungeons (Scrapyard/Water Temple/
        // Forest) are ids 2/3/5.
        assert_eq!(dungeon_name(2), Some("Scrapyard"));
        assert_eq!(dungeon_name(3), Some("Water Temple"));
        assert_eq!(dungeon_name(5), Some("Forest"));
        assert_eq!(dungeon_name(1), Some("Newbie Ground"));
        assert_eq!(dungeon_name(27), Some("Light Final"));
        assert_eq!(dungeon_name(28), None);
    }

    #[test]
    fn ultimate_being_ids_map_to_names() {
        assert_eq!(ultimate_being_name(1), Some("Planet Eater"));
        assert_eq!(ultimate_being_name(3), Some("Living Sun"));
        assert_eq!(ultimate_being_name(5), Some("ITRTG"));
        assert_eq!(ultimate_being_name(0), None);
        assert_eq!(ultimate_being_name(6), None);
    }

    #[test]
    fn village_building_ids_map_to_names() {
        assert_eq!(village_building_name(1), Some("Fishing"));
        assert_eq!(village_building_name(2), Some("Tavern"));
        assert_eq!(village_building_name(9), Some("Divine Hut"));
        assert_eq!(village_building_name(14), Some("Museum"));
        assert_eq!(village_building_name(100), None); // cosmetic tile, not a building
    }

    #[test]
    fn statue_ids_map_to_names() {
        // Committed save Museum has ids 8/1/9/2/3.
        assert_eq!(statue_name(8), Some("Halloween 2025"));
        assert_eq!(statue_name(1), Some("Easter 2024"));
        assert_eq!(statue_name(9), Some("Christmas 2025"));
        assert_eq!(statue_name(11), Some("Easter 2026"));
        assert_eq!(statue_name(12), None);
    }

    #[test]
    fn pond_ids_map_to_names() {
        assert_eq!(pond_name(0), Some("New Pond"));
        assert_eq!(pond_name(4), Some("Sad Pond")); // committed save's 025.f
        assert_eq!(pond_name(9), Some("Final Pond"));
        assert_eq!(pond_name(10), None);
    }

    #[test]
    fn challenge_ids_map_to_names() {
        // The eleven cross-checked against an in-game capture (2026-06-20).
        assert_eq!(challenge_name(1), Some("Ultimate Universe Challenge"));
        assert_eq!(challenge_name(3), Some("Double Rebirth Challenge"));
        assert_eq!(challenge_name(10), Some("All Achievements Challenge"));
        assert_eq!(challenge_name(25), Some("Monument Multi Challenge"));
        assert_eq!(challenge_name(32), Some("God Power Challenge"));
        assert_eq!(challenge_name(48), Some("Pet Level Challenge"));
        assert_eq!(challenge_name(76), Some("Boosting Capacity Challenge")); // last id
        // The five named from player+wiki (2026-06-21); RTI(30) vs TLC(37) must not collide.
        assert_eq!(challenge_name(7), Some("1000 Clone Challenge"));
        assert_eq!(challenge_name(18), Some("P. Baal Challenge"));
        assert_eq!(challenge_name(20), Some("1K Clones Black Hole Challenge"));
        assert_eq!(challenge_name(30), Some("Road to Infinity Challenge"));
        assert_eq!(challenge_name(37), Some("RTI Temp Level Challenge"));
        assert_eq!(challenge_name(0), None); // None sentinel
        assert_eq!(challenge_name(77), None); // past the end
        assert_eq!(challenge_difficulty_name(0), Some("Normal"));
        assert_eq!(challenge_difficulty_name(2), Some("Root"));
        assert_eq!(challenge_difficulty_name(4), None);
    }

    #[test]
    fn challenge_chp_matches_in_game_total() {
        // (challenge id, completions) from the in-game capture that yielded 1040
        // ChP: 20 AAC, 4 Ultimate Black Hole, 4 Ultimate Baal, 12 Ultimate Pet,
        // 14 Monument Multi, 4 Planet Multi, 12 Pet Level, 8 God Power, 4 Black Hole.
        let set = [
            (10, 20),
            (44, 4),
            (11, 4),
            (4, 12),
            (25, 14),
            (9, 4),
            (48, 12),
            (32, 8),
            (2, 4),
        ];
        let total: u32 = set.iter().map(|&(id, n)| challenge_chp(id).unwrap() * n).sum();
        assert_eq!(total, 1040);
        // Day challenges are score-based → no flat per-completion value.
        assert_eq!(challenge_chp(16), None); // Day Pet
        assert_eq!(challenge_chp(30), None); // Road to Infinity
        assert_eq!(challenge_chp(49), None); // Unused
        assert_eq!(challenge_chp(0), None); // None sentinel
        assert_eq!(challenge_chp(77), None); // past the end
    }

    #[test]
    fn score_based_challenges_are_exactly_the_no_chp_ones() {
        // The 10 score-based (Day + Road to Infinity) ids.
        for id in [14, 15, 16, 21, 22, 30, 41, 52, 54, 60] {
            assert!(challenge_is_score_based(id), "id {id} should be score-based");
        }
        for id in [1, 10, 25, 48, 76] {
            assert!(!challenge_is_score_based(id), "id {id} should not be score-based");
        }
        // Invariant: among real challenges (1..=76 minus UNUSED 49), a challenge
        // is score-based iff it has no flat per-completion ChP value.
        for id in 1..=76 {
            if id == 49 {
                continue; // UNUSED
            }
            assert_eq!(
                challenge_chp(id).is_none(),
                challenge_is_score_based(id),
                "id {id}: challenge_chp None vs score_based mismatch"
            );
        }
    }

    #[test]
    fn ultimate_overflow_upgrade_ids_map_to_names() {
        assert_eq!(ultimate_overflow_upgrade_name(1), Some("Dungeon Slot"));
        assert_eq!(ultimate_overflow_upgrade_name(6), Some("Higher PBaal"));
        assert_eq!(ultimate_overflow_upgrade_name(0), None); // None sentinel
        assert_eq!(ultimate_overflow_upgrade_name(7), None); // past the end
    }

    #[test]
    fn rti_bonus_ids_map_to_names() {
        assert_eq!(rti_bonus_name(1), Some("Physical"));
        assert_eq!(rti_bonus_name(10), Some("CreatingSpeed"));
        assert_eq!(rti_bonus_name(0), None); // None sentinel
        assert_eq!(rti_bonus_name(11), None); // past the end
    }

    #[test]
    fn quality_from_str_accepts_letters_and_numbers() {
        assert_eq!(quality_from_str("F"), Some(0));
        assert_eq!(quality_from_str("sss"), Some(8)); // case-insensitive
        assert_eq!(quality_from_str("SS"), Some(7));
        assert_eq!(quality_from_str(" Ult "), Some(9)); // trims
        assert_eq!(quality_from_str("6"), Some(6));
        assert_eq!(quality_from_str("9"), Some(9));
        assert_eq!(quality_from_str("10"), None); // out of range
        assert_eq!(quality_from_str("Z"), None);
        // Round-trips against quality_name.
        for q in 0..=9 {
            assert_eq!(quality_from_str(quality_name(q).unwrap()), Some(q));
        }
    }
}
