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
        138 => "Glowing Embers",
        141 => "Living Flame",
        146 => "Whispers of the Wind",
        147 => "Secrets of the Wind",
        149 => "Soul of Sylph",
        153 => "Ale",
        // Unidentified ids (worklist):
        // - {130, 160, 164, 167, 168} all sit at count 1 and pair with the
        //   five singleton inventory items {Not Nothing, Absolutely Nothing,
        //   Aether Ring +28, Food Journal One, Food Journal Two} — set known,
        //   per-id assignment unknown (all five predate the first save).
        // - 131–135 are the five T4 materials {Mythril, Ocean Stone,
        //   Sun Stone, Sky Stone, Jungle Stone}; still all at count 32 in
        //   both saves, so per-id assignment remains ambiguous.
        // - Present at count 0: 128, 129, 139, 140, 142–145, 148, 150.
        // 126–149 look like per-dungeon boss material families (Gnome/earth,
        // fire, wind) — the matching water family is presumably nearby.
        // Foods and gems are NOT in this namespace: Puny/Strong/Mighty Food
        // and Chocolate are dedicated save fields (X.c/d/e/v), gems live in
        // X.002 keyed by element id.
        _ => return None,
    })
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
/// Still unidentified (all unequipped): the nine 1-count types
/// {5, 8, 22, 23, 26, 30, 41, 52, 56} pair with {Iron Pot, Water Spear,
/// Flood Spear, Leeching Sword, Tree Axe, Hurricane Bow, Flame Armor,
/// Flood Armor, Tree Bracelet}, and type 44 is {Magic Hammer | Storm Ring}.
pub fn equipment_type_name(type_id: u32) -> Option<&'static str> {
    Some(match type_id {
        // armor
        3 => "Titanium Armor",
        5 => "Flame Armor",  // equipped on Bag 2026-06-13 (instance 7)
        8 => "Flood Armor",  // equipped on Cupid 2026-06-13 (instance 10)
        12 => "Forest Armor",
        13 => "Feather Vest",
        15 => "Hurricane Armor",
        // weapons
        18 => "Titanium Sword",
        21 => "Inferno Sword",
        22 => "Water Spear",  // equipped on Nugget 2026-06-13 (instance 122)
        29 => "Storm Bow",
        47 => "Shaping Hammer",
        50 => "Journeying Stick",
        51 => "Magic Stick",
        54 => "Magic Pot",
        57 => "Ego Sword",
        60 => "Bursting Knives",
        79 => "Legendary Hammer", // "Legend Hammer" in the in-game inventory
        83 => "Exploding Knives",
        // accessories
        33 => "Titanium Ring",
        36 => "Inferno Gloves",
        39 => "Tsunami Necklace",
        40 => "Wood Bracelet",
        41 => "Tree Bracelet",  // equipped on Meteor 2026-06-13 (instance 3)
        44 => "Storm Ring",     // equipped on Bag 2026-06-13 (instance 173); resolves the Magic Hammer|Storm Ring tie
        45 => "Hurricane Ring",
        61 => "Alchemist Cape",
        86 => "Ear Muffs",
        // 300-series: event/special gear
        300 => "Candy Cane",
        301 => "Spectrometers",
        302 => "Master Gloves",
        303 => "Learning Coat",
        304 => "Magic Egg",
        305 => "Creators Vest",
        306 => "Godly Hammer",
        307 => "Merry Mantle",
        309 => "Growing Love Pendant",
        311 => "Christmas Boots",
        _ => return None,
    })
}

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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn unknown_ids_return_none() {
        assert_eq!(material_name(0), None);
        assert_eq!(material_name(130), None); // singleton set, unassigned
        assert_eq!(material_name(134), None); // T4 material, id↔name ambiguous
        assert_eq!(material_name(9999), None);
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
}
