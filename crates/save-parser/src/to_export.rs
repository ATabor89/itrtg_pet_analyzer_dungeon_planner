//! Convert a parsed [`SaveFile`] into the planner's [`ExportPet`] roster — the
//! same shape the in-game *pet stats* export produces — so a full save can drive
//! the analyzer / growth chamber through the existing import pipeline
//! (`DataStore::import_export` → `merge_pets`) instead of a pasted text export.
//!
//! Parity with the text export (so the two import sources are interchangeable):
//!
//! - **Growth** is the save's base growth `E`, rounded — exactly the value the
//!   pet export carries *after* its Magic-Egg inversion. The ×1.3 egg bonus is
//!   reapplied downstream by [`ExportPet::effective_growth`] from the loadout, so
//!   we must **not** pre-multiply here. (See the `growth_is_stored_without_magic_egg_bonus`
//!   cross-check in `tests/real_save.rs`.) For a Magic-Egg pet the save is in fact
//!   *more* accurate than a text export: the text importer recovers the base as
//!   `round(displayed_total / 1.3)`, which can land ±1 off the true fractional
//!   base the save stores — so a save import and a text import of the same
//!   account may differ by 1 growth point on egg pets, by design.
//! - **Identity** uses the pet's *export* name — `pet_type_name(type_id)`, which
//!   is export-normalized (e.g. Rudolph → "Reindeer", Chicken → "Egg") — so
//!   `merge_pets`' `resolve_wiki_name` keys it the same way it keys a text export.
//! - **Loadout** is resolved from the equipped instance ids to named
//!   [`Equipment`]; the campaign-bonus / Magic-Egg / Growing-Love-Pendant logic
//!   all keys off the equipment *name* (plus `quality`/`upgrade_level`), so this
//!   is the load-bearing part of the conversion.
//!
//! Known gaps (display-only or unrecoverable; none feed the chamber/solver):
//!
//! - **Combat stats** (HP/Attack/Defense/Speed) are not stored in the save — the
//!   game derives them at runtime — so we emit zeroes. They show only in the
//!   analyzer's per-pet "Stats" row.
//! - **Elemental affinities** aren't in the save either; emitted as zeroes
//!   (loaded but unused by the analyzer).
//! - **Action** (the live Campaign/Dungeon/Village assignment) isn't recoverable
//!   from the save as a single field; emitted as [`PetAction::Idle`].

use itrtg_models::{
    CombatStats, Element, ElementalAffinities, Equipment, ExportPet, Loadout, PetAction, Quality,
};

use crate::model::{EquipmentItem, SaveFile, SavePet};

/// Convert every pet in the save into an [`ExportPet`], preserving save order.
///
/// The result is interchangeable with [`itrtg_models`]'s text-export parse: feed
/// it through the same `import_export` path to populate the analyzer and growth
/// chamber from a save file.
pub fn save_to_export_pets(save: &SaveFile) -> Vec<ExportPet> {
    save.pets.iter().map(|pet| convert_pet(save, pet)).collect()
}

fn convert_pet(save: &SaveFile, pet: &SavePet) -> ExportPet {
    ExportPet {
        // Export-normalized name (so the merge keys this pet like a text export);
        // fall back to the save's display name if the type id is unknown.
        export_name: pet.type_name().unwrap_or(pet.name.as_str()).to_string(),
        element: pet.element.unwrap_or(Element::Neutral),
        // Base growth `E`, rounded — the export's "Growth" column to the digit.
        // The Magic Egg ×1.3 is reapplied downstream from the loadout.
        growth: pet.growth.round() as u64,
        dungeon_level: pet.dungeon_level,
        class: pet.class,
        class_level: pet.class_level,
        class_exp: pet.class_exp,
        // Not stored in the save (derived at runtime by the game); analyzer
        // display-only, unused by the chamber/solver.
        combat_stats: CombatStats { hp: 0, attack: 0, defense: 0, speed: 0 },
        elemental_affinities: ElementalAffinities {
            water: 0,
            fire: 0,
            wind: 0,
            earth: 0,
            dark: 0,
            light: 0,
        },
        loadout: Loadout {
            weapon: resolve_equipment(save, pet.weapon_id),
            armor: resolve_equipment(save, pet.armor_id),
            accessory: resolve_equipment(save, pet.accessory_id),
        },
        // The live action isn't a single save field; the text export's "Other"
        // form label is, though (see below).
        action: PetAction::Idle,
        unlocked: pet.unlocked,
        improved: pet.token_improved,
        // The export "Other" column carries the elemental form label
        // ("GnomeV2", …) for elemental pets; non-elemental pets have form id 0.
        other: (pet.elemental_form_id != 0)
            .then(|| pet.elemental_form_name().map(str::to_string))
            .flatten(),
        has_partner: pet.partner_type_id.is_some(),
    }
}

/// Resolve an equipped instance id to a named [`Equipment`], mirroring the text
/// export's parsed fields. `None` when the slot is empty.
fn resolve_equipment(save: &SaveFile, instance_id: Option<u32>) -> Option<Equipment> {
    let item = save.equipment_by_instance_id(instance_id?)?;
    Some(equipment_from_item(item))
}

fn equipment_from_item(item: &EquipmentItem) -> Equipment {
    Equipment {
        name: item
            .type_name()
            .map(str::to_string)
            // Unknown type id: keep a stable placeholder rather than dropping the
            // item, so the slot still reads as "occupied".
            .unwrap_or_else(|| format!("Item #{}", item.type_id)),
        // The text export omits "+0"; match that (None when unupgraded).
        upgrade_level: (item.plus > 0).then_some(item.plus as u8),
        quality: quality_from_id(item.quality),
        enchant_level: (item.enchant_level > 0).then_some(item.enchant_level as u8),
        gem: item.gem_element,
        gem_level: (item.gem_level > 0).then_some(item.gem_level as u8),
    }
}

/// Map the save's numeric quality (`0..=8`) to the model's [`Quality`] grade.
/// Out-of-range values clamp to `SSS`. Pet *equipment* only ever stores `0..=8`;
/// the `9 => "Ult"` grade that `items::quality_name` recognizes is for adventure
/// cores, which the model's [`Quality`] enum can't represent and gear never uses.
fn quality_from_id(quality: u32) -> Quality {
    match quality {
        0 => Quality::F,
        1 => Quality::E,
        2 => Quality::D,
        3 => Quality::C,
        4 => Quality::B,
        5 => Quality::A,
        6 => Quality::S,
        7 => Quality::SS,
        _ => Quality::SSS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_ids_map_to_grades() {
        assert_eq!(quality_from_id(0), Quality::F);
        assert_eq!(quality_from_id(4), Quality::B);
        assert_eq!(quality_from_id(5), Quality::A);
        assert_eq!(quality_from_id(8), Quality::SSS);
        // Out-of-range clamps to SSS rather than panicking.
        assert_eq!(quality_from_id(99), Quality::SSS);
    }
}
