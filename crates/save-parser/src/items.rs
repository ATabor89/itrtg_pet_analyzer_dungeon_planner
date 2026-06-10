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
        19 => "Nothing",
        20 => "Flying Boots",
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
        // Unidentified ids seen in real inventories (kept here as a worklist):
        // nonzero in the reference save: 16, 17, 21, 130, 160, 164, 167, 168;
        // present at count 0: 128, 129, 139, 140, 142–145, 148, 150.
        // 131–135 are the five T4 materials — the set is known (Mythril,
        // Ocean Stone, Sun Stone, Sky Stone, Jungle Stone) but all five sit
        // at count 32 in the reference save, so which id is which is still
        // ambiguous; left unnamed until a save disambiguates the counts.
        // 126–149 look like per-dungeon boss material families (Gnome/earth,
        // fire, wind) — the matching water family is presumably nearby.
        _ => return None,
    })
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
    }

    #[test]
    fn unknown_ids_return_none() {
        assert_eq!(material_name(0), None);
        assert_eq!(material_name(21), None); // prior project: "Unknown #21"
        assert_eq!(material_name(130), None);
        assert_eq!(material_name(134), None); // T4 material, id↔name ambiguous
        assert_eq!(material_name(9999), None);
    }
}
