//! Known game enums and the matcher that locates them in a decompiled dump.
//!
//! Each [`KnownEnum`] pairs a human label with the live `save-parser` lookup
//! table that already encodes that enum's ids. We never hard-code the enum's
//! obfuscated type name (it rotates per game build): instead we build the
//! `{id → name}` map from the Rust table and find the dump enum whose
//! `{value → member}` pairs overlap it most. The Rust table *is* the
//! fingerprint, so adding coverage is a single registry line.

use crate::parse::ParsedEnum;
use save_parser::items;
use std::collections::BTreeMap;

/// Whether the Rust table intends to mirror the *whole* enum.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Coverage {
    /// Every real id should be present — a non-sentinel miss is actionable.
    Complete,
    /// A deliberately curated subset (e.g. only gameplay-relevant entries);
    /// misses are summarized, not treated as work to do.
    Partial,
}

/// Inclusive id ceiling scanned when building a fingerprint from a Rust table.
/// The lookups return `None` past their real range, so a generous fixed ceiling
/// is essentially free and avoids per-table bounds (which, set too low, would
/// drop real high-id entries from the fingerprint and then falsely report them
/// as missing). The highest id in any table today is 1002; this leaves headroom.
pub const SCAN_MAX: u32 = 4096;

/// A game enum we mirror in Rust.
pub struct KnownEnum {
    /// Stable label used on the CLI (`--only <key>`) and in reports.
    pub key: &'static str,
    /// The `save-parser` lookup that encodes this enum's ids today.
    pub lookup: fn(u32) -> Option<&'static str>,
    /// Whether the table aims to be exhaustive (drives against-Rust severity).
    pub coverage: Coverage,
}

use Coverage::{Complete, Partial};

/// The enums we track. Ordered roughly by how often a game update touches them.
/// `coverage` reflects each table's intent as observed today: the `Partial`
/// ones (skills, craftable gear, materials, decorative village pieces) only
/// curate a subset, so against-Rust mode summarizes their gaps instead of
/// flagging every entry.
pub const REGISTRY: &[KnownEnum] = &[
    KnownEnum { key: "pets", lookup: items::pet_type_name, coverage: Complete },
    KnownEnum { key: "adventure_class", lookup: items::adventure_class_name, coverage: Complete },
    KnownEnum { key: "adventure_skill", lookup: items::adventure_skill_name, coverage: Partial },
    KnownEnum { key: "adventure_craft_gear", lookup: items::adventure_craft_gear_name, coverage: Partial },
    KnownEnum { key: "adventure_recipe", lookup: items::adventure_recipe_name, coverage: Complete },
    KnownEnum { key: "adventure_item", lookup: items::adventure_item_name, coverage: Complete },
    KnownEnum { key: "adventure_enemy", lookup: items::adventure_enemy_name, coverage: Complete },
    KnownEnum { key: "material", lookup: items::material_name, coverage: Partial },
    KnownEnum { key: "equipment_type", lookup: items::equipment_type_name, coverage: Complete },
    KnownEnum { key: "elemental_form", lookup: items::elemental_form_name, coverage: Complete },
    KnownEnum { key: "gem_element", lookup: items::gem_element_name, coverage: Complete },
    KnownEnum { key: "campaign_type", lookup: items::campaign_type_name, coverage: Complete },
    KnownEnum { key: "dungeon", lookup: items::dungeon_name, coverage: Complete },
    KnownEnum { key: "spacedim", lookup: items::spacedim_name, coverage: Complete },
    KnownEnum { key: "might", lookup: items::might_name, coverage: Complete },
    KnownEnum { key: "monument", lookup: items::monument_name, coverage: Complete },
    KnownEnum { key: "creation", lookup: items::creation_name, coverage: Complete },
    KnownEnum { key: "village_building", lookup: items::village_building_name, coverage: Partial },
    KnownEnum { key: "statue", lookup: items::statue_name, coverage: Complete },
    KnownEnum { key: "pond", lookup: items::pond_name, coverage: Complete },
    KnownEnum { key: "challenge", lookup: items::challenge_name, coverage: Complete },
];

/// Sentinel members the Rust tables intentionally omit (the caller handles
/// "none" separately), so they should never count as missing.
pub fn is_sentinel(name: &str) -> bool {
    normalize(name) == "none"
}

/// Normalize a name for comparison: lowercase, alphanumerics only. This makes
/// the deliberate display spellings in the Rust tables ("Magic Shooter",
/// "Onion Knight") compare equal to the enum's PascalCase members
/// ("MagicShooter", "OnionKnight").
pub fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// The `{id → normalized name}` fingerprint for a known enum, from the live
/// Rust table.
pub fn rust_fingerprint(known: &KnownEnum) -> BTreeMap<i64, String> {
    let mut map = BTreeMap::new();
    for id in 0..=SCAN_MAX {
        if let Some(name) = (known.lookup)(id) {
            map.insert(id as i64, normalize(name));
        }
    }
    map
}

/// How well a dump enum matches a fingerprint: the count of values present in
/// both with an equal normalized name.
fn match_score(fingerprint: &BTreeMap<i64, String>, candidate: &ParsedEnum) -> usize {
    candidate
        .by_value()
        .iter()
        .filter(|(v, name)| fingerprint.get(v).map(|f| f == &normalize(name)).unwrap_or(false))
        .count()
}

/// Locate the dump enum that best matches a known enum's fingerprint.
///
/// Returns the best-matching enum and its score, or `None` if no candidate
/// clears the threshold (`max(3, fingerprint_len / 4)`) — which is the honest
/// answer when the enum isn't enum-backed in the dump, or the table is a
/// hand-curated name list with no decompiled counterpart.
pub fn match_enum<'a>(
    known: &KnownEnum,
    dump: &'a [ParsedEnum],
) -> Option<(&'a ParsedEnum, usize)> {
    let fingerprint = rust_fingerprint(known);
    if fingerprint.is_empty() {
        return None;
    }
    let threshold = std::cmp::max(3, fingerprint.len() / 4);

    let best = dump
        .iter()
        .map(|e| (e, match_score(&fingerprint, e)))
        .max_by_key(|(_, score)| *score)?;

    if best.1 >= threshold {
        Some(best)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_collapses_spelling_differences() {
        assert_eq!(normalize("Magic Shooter"), normalize("MagicShooter"));
        assert_eq!(normalize("Onion Knight"), "onionknight");
        assert_eq!(normalize("GoldDragon"), "golddragon");
    }

    #[test]
    fn fingerprint_covers_known_pets() {
        let pets = REGISTRY.iter().find(|k| k.key == "pets").unwrap();
        let fp = rust_fingerprint(pets);
        // Anchors that have held since the format was cracked.
        assert_eq!(fp.get(&2).map(String::as_str), Some("cat"));
        assert_eq!(fp.get(&32).map(String::as_str), Some("pandora"));
        assert_eq!(fp.get(&152).map(String::as_str), Some("boar"));
    }

    #[test]
    fn matches_enum_despite_rotated_type_name_and_spelling() {
        // A stand-in "adventure class" enum with an obfuscated type name and
        // PascalCase members; the matcher should still pick it.
        let dump = vec![ParsedEnum {
            type_name: "JAFCHHNMDAC".into(),
            members: vec![
                ("None".into(), 0),
                ("Newbie".into(), 1),
                ("Adventurer".into(), 2),
                ("Squire".into(), 3),
                ("Student".into(), 4),
                ("Thief".into(), 5),
                ("Archer".into(), 6),
                ("Warrior".into(), 7),
                ("Fighter".into(), 8),
                ("Mage".into(), 9),
                ("Cleric".into(), 10),
                ("MagicShooter".into(), 43), // Rust spells it "Magic Shooter"
            ],
        }];
        let class = REGISTRY.iter().find(|k| k.key == "adventure_class").unwrap();
        let (matched, score) = match_enum(class, &dump).expect("should match");
        assert_eq!(matched.type_name, "JAFCHHNMDAC");
        // All 12 members align (incl. the PascalCase/space rename) → high score.
        assert!(score >= 12, "score was {score}");
    }
}
