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
        //    Whetstones and Sacred Stones). Order is now from the game's
        //    `NCPJFPLCPPK` enum. **Bug fix 2026-06-19:** the three 10-count
        //    bars (34/36/37) were indistinguishable by count, so the prior
        //    project guessed their element order and got it wrong — 34 was
        //    "Tsunami", 36 "Forest", 37 "Titanium". The enum (corroborated by
        //    `dungeon_recommendations.yaml`: the neutral Scrapyard event gives
        //    a Titanium Bar, the Water Temple event a Tsunami Bar) sets them
        //    straight. (33 Inferno / 35 Hurricane were already correct, pinned
        //    by their distinct counts 5 / 4.) --
        33 => "Inferno Bar",   // fire — count 5 ✓
        34 => "Forest Bar",    // earth (was wrongly "Tsunami Bar")
        35 => "Hurricane Bar", // wind — count 4 ✓
        36 => "Titanium Bar",  // neutral (was wrongly "Forest Bar")
        37 => "Tsunami Bar",   // water (was wrongly "Titanium Bar")
        // -- dungeon consumables: bombs & traps (player-confirmed 2026-06-18) --
        30 => "Melting Bomb",
        48 => "Nanotrap",
        49 => "Freezing Bomb",
        // -- export-confirmed in the 2026-06-09 reference save --
        117 => "Ant",            // count 192,164 = Main Stats "Ants"
        159 => "Strategy Book",  // count 2,840 = Main Stats "Strategy Books"
        166 => "Honey",          // count 787 = Main Stats "Honey"
        174 => "Acorn",          // count 24,727 = Main Stats "Acorns"
        // -- prior-project, special/dungeon items --
        118 => "Rebirth Bacon",
        119 => "Nothing", // a second "Nothing" id; both appeared in-game
        120 => "Cure",    // player-confirmed 2026-06-18
        // -- elemental-pet evolution-quest materials. Each elemental pet
        //    (Gnome/earth, Salamander/fire, Sylph/wind, …) has a "quest" where
        //    you craft a family of items to give it. NOT dungeon-boss drops. --
        126 => "Core Shard of Gnome", // Gnome / earth
        127 => "Magic Soil",          // Gnome / earth
        // T4 materials — resolved 2026-06-16 by a save-edit probe: the five
        // count-32 stacks were set to distinct counts (41–45) and read off
        // in-game by name.
        131 => "Sun Stone",
        132 => "Jungle Stone",
        133 => "Sky Stone",
        134 => "Mythril",
        135 => "Ocean Stone",
        138 => "Glowing Embers",  // Salamander / fire
        139 => "Igneous Bones",   // Salamander / fire (player-confirmed 2026-06-18)
        140 => "Pliable Magma",   // Salamander / fire (player-confirmed 2026-06-18)
        141 => "Living Flame",    // Salamander / fire
        145 => "Prosthetic Tail", // Salamander upgrade item (player-confirmed 2026-06-18)
        146 => "Whispers of the Wind",  // Sylph / wind
        147 => "Secrets of the Wind",   // Sylph / wind
        148 => "Mysteries of the Wind", // Sylph / wind (player-confirmed 2026-06-18)
        149 => "Soul of Sylph",         // Sylph / wind
        153 => "Ale",
        // Aether Ring (player-confirmed 2026-06-18 on a fresh/edited save: the
        // base, no-boss-fights ring is id 130). The in-game "+N" suffix tracks
        // boss kills and is almost certainly the SAME id 130 with a dynamic name
        // (not consecutive ids — 131 is Sun Stone), so the old save's "Aether
        // Ring +28" was also id 130. Resolves 130 from the singleton worklist.
        130 => "Aether Ring",
        160 => "Not Nothing",        // player-confirmed 2026-06-18; enum NotNothing
        162 => "Monster Blood",      // player-confirmed 2026-06-18
        164 => "Absolutely Nothing", // player-confirmed 2026-06-18; enum AbsolutelyNothing
        // {167, 168} assignment resolved from the `NCPJFPLCPPK` enum (the prior
        // worklist had the set but not the per-id mapping).
        167 => "Food Journal One",   // enum FoodJournal1
        168 => "Food Journal Two",   // enum FoodJournal2
        // The formerly count-0 ids are also named by the enum: 128 Soul of
        // Gnome, 129 Magic Soul of Gnome, 142 Salamander Soul, 143 Magic Soul
        // of Salamander, 144 Salamander Skin, 150 Magic Soul of Sylph (the
        // per-element evolution-quest "soul" tier). Left out of this table only
        // because no reference save has held a nonzero count to double-check the
        // display spelling against; add them when one does, or trust the enum.
        // 126–149 are the elemental-pet evolution-quest / upgrade-item families
        // (you craft items to advance each pet through its quest; these aren't
        // strictly contiguous — e.g. Salamander's 145 Prosthetic Tail sits past
        // its 138–141 quest materials):
        // Gnome/earth (126–129), Salamander/fire (138–144 = Glowing Embers /
        // Igneous Bones / Pliable Magma / Living Flame / Salamander Soul / Magic
        // Soul of Salamander / Salamander Skin; 145 Prosthetic Tail),
        // Sylph/wind (146–150 = Whispers / Secrets / Mysteries of the Wind /
        // Soul of Sylph / Magic Soul of Sylph). The **water pet is `Undine`**
        // (enum); its quest family is the 106–116 cluster (Undine / Body /
        // Mecha Arm / Water Soul / Purified Water / … / Soul of Undine /
        // Magic Soul of Undine), confirmed by the `NCPJFPLCPPK` enum — no longer
        // a count-0 mystery.
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
/// 44 = {Magic Hammer | Storm Ring} tie). 2026-06-19, same method, resolved the
/// crafting-weapon families: 48 = Magic Hammer (the real one — the 44 tie went
/// to Storm Ring), 80 = Legendary Stick, 81 = Legendary Pot (with 79 Legendary
/// Hammer, the 79/80/81 Legendary family).
///
/// The previously-ambiguous {23, 26, 30, 52, 56} are now **resolved from the
/// game's `MBBDNNAMMHO` equipment-type enum** (`Assembly-CSharp`): 23 = Flood
/// Spear, 26 = Tree Axe, 30 = Hurricane Bow, 52 = Iron Pot, 56 = Leeching Sword.
/// (The full enum has ~110 types; this table stays the curated subset that
/// carries slot categories — the names below match the enum exactly.)
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
        // 169 (Shiny Metal Stone) is named by the enum but kept out of the
        // table until a save holds it — so it's still None here.
        assert_eq!(material_name(169), None);
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
}
