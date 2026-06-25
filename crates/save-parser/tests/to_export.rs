//! Tests for the `SaveFile` → `ExportPet` converter, run against the redacted
//! reference save (skips silently if it isn't present, like `real_save.rs`).
//!
//! The reference save's per-pet values are already cross-checked against the
//! same-session in-game pet export in `real_save.rs` / FINDINGS.md, so asserting
//! the converter reproduces them validates the mapping end-to-end.

use itrtg_models::{Quality, resolve_wiki_name};
use save_parser::save_to_export_pets;

fn load_reference_save() -> Option<save_parser::SaveFile> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../reference/save_file_deserialization/ManualSave_2026-06-09.txt"
    );
    let raw = std::fs::read_to_string(path).ok()?;
    Some(save_parser::parse_save(&raw).expect("reference save should parse"))
}

macro_rules! require_save {
    () => {
        match load_reference_save() {
            Some(save) => save,
            None => {
                eprintln!("reference save not present; skipping");
                return;
            }
        }
    };
}

/// Every converted pet should faithfully carry the save's typed per-pet fields,
/// in the same order — a comprehensive check of the field mapping.
#[test]
fn converted_pets_match_the_save_fields() {
    let save = require_save!();
    let exports = save_to_export_pets(&save);

    assert_eq!(exports.len(), save.pets.len(), "one export per save pet");

    for (pet, export) in save.pets.iter().zip(&exports) {
        // Growth is the base `E`, rounded — never pre-multiplied by the egg.
        assert_eq!(
            export.growth,
            pet.growth.round() as u64,
            "{} growth mismatch",
            pet.name
        );
        assert_eq!(export.dungeon_level, pet.dungeon_level, "{}", pet.name);
        assert_eq!(export.class, pet.class, "{} class", pet.name);
        assert_eq!(export.class_level, pet.class_level, "{} class level", pet.name);
        assert_eq!(export.unlocked, pet.unlocked, "{} unlocked", pet.name);
        assert_eq!(export.improved, pet.token_improved, "{} improved", pet.name);
        assert_eq!(
            export.has_partner,
            pet.partner_type_id.is_some(),
            "{} partner",
            pet.name
        );
        // Identity is the export-normalized name, so the planner's merge keys it
        // like a text export would.
        let expected_name = pet.type_name().unwrap_or(pet.name.as_str());
        assert_eq!(export.export_name, expected_name, "{} name", pet.name);
    }
}

/// A pet whose save *display* name differs from its *export* name (Rudolph →
/// "Reindeer") must convert to the export name and resolve back to the wiki name.
#[test]
fn display_name_converts_to_export_name() {
    let save = require_save!();
    let exports = save_to_export_pets(&save);

    let Some(idx) = save.pets.iter().position(|p| p.name == "Rudolph") else {
        eprintln!("Rudolph not in this save; skipping name check");
        return;
    };
    assert_eq!(exports[idx].export_name, "Reindeer");
    // And the planner's resolver turns it back into the wiki display name.
    assert_eq!(resolve_wiki_name(&exports[idx].export_name), "Rudolph");
}

/// Slash-named pets must still round-trip through the merge resolver.
#[test]
fn slash_named_pet_resolves_for_merge() {
    let save = require_save!();
    let exports = save_to_export_pets(&save);

    // "Chicken" (display) → "Egg" (export) → "Egg/Chicken" (wiki).
    if let Some(idx) = save.pets.iter().position(|p| p.type_name() == Some("Egg")) {
        assert_eq!(exports[idx].export_name, "Egg");
        assert_eq!(resolve_wiki_name(&exports[idx].export_name), "Egg/Chicken");
    }
}

/// Pandora's Box wears a Magic Egg; the converter keeps the true base growth and
/// flags the egg via the loadout (so `effective_growth` reapplies the ×1.3).
#[test]
fn magic_egg_pet_keeps_base_growth_and_flags_the_egg() {
    let save = require_save!();
    let exports = save_to_export_pets(&save);

    let idx = save
        .pets
        .iter()
        .position(|p| p.name == "Pandora's box")
        .expect("Pandora's box in the reference save");
    let pandora = &exports[idx];

    // True fractional base 44334.321… → 44334 (the save is exact; a text export
    // would recover 44335 via round(57635 / 1.3)).
    assert_eq!(pandora.growth, 44_334);
    assert!(pandora.has_magic_egg(), "weapon should be the Magic Egg");
    assert_eq!(pandora.loadout.weapon.as_ref().unwrap().name, "Magic Egg");
}

/// Equipped gear converts to named [`Equipment`] matching the save's resolved
/// instance (name, quality, upgrade level).
#[test]
fn equipment_resolves_to_named_gear() {
    let save = require_save!();
    let exports = save_to_export_pets(&save);

    // Find any pet with an equipped weapon and cross-check the converted piece
    // against the save's own instance resolution.
    let Some((idx, pet)) = save
        .pets
        .iter()
        .enumerate()
        .find(|(_, p)| p.weapon_id.is_some())
    else {
        eprintln!("no equipped weapon in this save; skipping");
        return;
    };
    let item = save
        .equipment_by_instance_id(pet.weapon_id.unwrap())
        .expect("equipped weapon resolves");
    let weapon = exports[idx]
        .loadout
        .weapon
        .as_ref()
        .expect("converted weapon present");

    if let Some(name) = item.type_name() {
        assert_eq!(weapon.name, name, "{} weapon name", pet.name);
    }
    // Quality grade round-trips (numeric id → letter grade).
    if let Some(letter) = item.quality_name() {
        let expected = match weapon.quality {
            Quality::F => "F",
            Quality::E => "E",
            Quality::D => "D",
            Quality::C => "C",
            Quality::B => "B",
            Quality::A => "A",
            Quality::S => "S",
            Quality::SS => "SS",
            Quality::SSS => "SSS",
        };
        assert_eq!(expected, letter, "{} weapon quality", pet.name);
    }
}
