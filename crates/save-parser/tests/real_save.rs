//! Integration test against the real reference save in
//! `reference/save_file_deserialization/`, cross-checked with the in-game
//! exports captured in the same session (see FINDINGS.md there).
//!
//! Skips silently if the reference save is not present, so the suite stays
//! green if the reference data is ever pruned.

use itrtg_models::{Class, Element};

fn load_reference_save() -> Option<save_parser::SaveFile> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../reference/save_file_deserialization/ManualSave_2026-06-09.txt"
    );
    let raw = std::fs::read_to_string(path).ok()?;
    Some(save_parser::parse_save(&raw).expect("reference save should parse"))
}

/// The second reference save (2026-06-10), captured together with a full
/// manual inventory transcription — see `second_save/` in the reference dir.
fn load_second_save() -> Option<save_parser::SaveFile> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../reference/save_file_deserialization/second_save/ManualSave_2026-06-10.txt"
    );
    let raw = std::fs::read_to_string(path).ok()?;
    Some(save_parser::parse_save(&raw).expect("second save should parse"))
}

macro_rules! require_second_save {
    () => {
        match load_second_save() {
            Some(save) => save,
            None => {
                eprintln!("second reference save not present; skipping");
                return;
            }
        }
    };
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

#[test]
fn parses_metadata() {
    let save = require_save!();
    assert_eq!(save.player_name.as_deref(), Some("RedactedGod"));
    assert_eq!(save.god_name.as_deref(), Some("RedactedAccount"));
    assert_eq!(save.saved_at_unix, Some(1781053129));
    // Main Stats export: "Pet Stones: 267,028"
    assert_eq!(save.pet_stones, Some(267028));
}

#[test]
fn parses_all_pets() {
    let save = require_save!();
    // Pet Stats export has 158 rows.
    assert_eq!(save.pets.len(), 158);
    // Main Stats export: "Unlocked Pets: 104"
    assert_eq!(save.pets.iter().filter(|p| p.unlocked).count(), 104);
}

#[test]
fn salamander_matches_exports() {
    let save = require_save!();
    let pet = save.pet_by_name("Salamander").expect("Salamander exists");
    // Pet Stats export: Salamander;Fire;66,841;101;Supporter;24;...
    assert_eq!(pet.type_id, 89);
    assert!(pet.unlocked);
    assert_eq!(pet.growth.round() as u64, 66841);
    assert_eq!(pet.element, Some(Element::Fire));
    assert_eq!(pet.dungeon_level, 101);
    assert_eq!(pet.class, Some(Class::Supporter));
    assert_eq!(pet.class_level, 24);
    // Pet Equips export: Salamander=704,766,787
    assert_eq!(pet.weapon_id, Some(704));
    assert_eq!(pet.armor_id, Some(766));
    assert_eq!(pet.accessory_id, Some(787));
    // Dungeon Teams export: team 0 slot 4
    assert_eq!(pet.team_slot, Some(4));
}

#[test]
fn growth_is_stored_without_magic_egg_bonus() {
    let save = require_save!();
    // Pandora's box has a Magic Egg equipped. The Pet Stats export shows
    // 57,635 (base × 1.3, rounded); the save stores the true base —
    // matching the value documented in pet-importer's Magic Egg inversion
    // ("base 44334.321…, total 57634.617…") to the digit.
    let pandora = save.pet_by_name("Pandora's box").expect("Pandora's box");
    assert!((pandora.growth - 44334.321043064).abs() < 1e-6);
    assert_eq!((pandora.growth * 1.3).round() as u64, 57635);
}

#[test]
fn locked_pet_has_no_class_or_gear() {
    let save = require_save!();
    let pet = save.pet_by_name("Ancient Mimic").expect("Ancient Mimic");
    assert!(!pet.unlocked);
    assert_eq!(pet.growth.round() as u64, 223882);
    assert_eq!(pet.class, None);
    assert_eq!(pet.weapon_id, None);
    assert_eq!(pet.team_slot, None);
    assert_eq!(pet.partner_type_id, None); // F = 999
}

#[test]
fn display_names_are_preserved() {
    let save = require_save!();
    // Save stores display names, not export names.
    assert!(save.pet_by_name("Rudolph").is_some()); // export "Reindeer"
    assert!(save.pet_by_name("Chicken").is_some()); // export "Egg"
    assert!(save.pet_by_name("Pigñata").is_some()); // export "Pignata"
    assert!(save.pet_by_name("Tödlicher Löffel").is_some()); // export "Spoon"
    assert!(save.pet_by_name("Reindeer").is_none());
}

#[test]
fn partners_are_mutual() {
    let save = require_save!();
    // Vampire ↔ Succubus per the Partner export column.
    let vampire = save.pet_by_name("Vampire").unwrap();
    let succubus = save.pet_by_name("Succubus").unwrap();
    assert_eq!(vampire.partner_type_id, Some(succubus.type_id));
    assert_eq!(succubus.partner_type_id, Some(vampire.type_id));
    // Fairy's partner is Mouse, whose type id is 0 — the one case where a
    // naive zero-check would lose a real partner.
    let fairy = save.pet_by_name("Fairy").unwrap();
    let mouse = save.pet_by_name("Mouse").unwrap();
    assert_eq!(mouse.type_id, 0);
    assert_eq!(fairy.partner_type_id, Some(0));

    // Every pet with a partner is its partner's partner.
    for pet in save.pets.iter().filter(|p| p.partner_type_id.is_some()) {
        let partner = save
            .pet_by_type_id(pet.partner_type_id.unwrap())
            .unwrap_or_else(|| panic!("{}'s partner id should resolve", pet.name));
        assert_eq!(
            partner.partner_type_id,
            Some(pet.type_id),
            "{} ↔ {} should be mutual",
            pet.name,
            partner.name
        );
    }
}

#[test]
fn dungeon_teams_match_export() {
    let save = require_save!();
    assert_eq!(save.dungeon_teams.len(), 3);

    // Dungeon Teams export team 0:
    // Reindeer=5,Dog=3,Dragon=6,Egg=2,Salamander=4,Succubus=1
    // (export names; save names are Rudolph/Chicken)
    let scrapyard = &save.dungeon_teams[0];
    assert_eq!(scrapyard.dungeon_name, "Scrapyard");
    assert_eq!(scrapyard.pet_type_ids.len(), 6);
    let names: Vec<&str> = scrapyard
        .pet_type_ids
        .iter()
        .map(|id| save.pet_by_type_id(*id).unwrap().name.as_str())
        .collect();
    for expected in ["Rudolph", "Dog", "Dragon", "Chicken", "Salamander", "Succubus"] {
        assert!(names.contains(&expected), "{expected} should be in team 0");
    }
    // Slot order comes from the pets' team_slot field.
    let rudolph = save.pet_by_name("Rudolph").unwrap();
    assert_eq!(rudolph.team_slot, Some(5));

    let names: Vec<&str> = save
        .dungeon_teams
        .iter()
        .map(|t| t.dungeon_name.as_str())
        .collect();
    assert_eq!(names, ["Scrapyard", "Water Temp", "Forest"]);
}

#[test]
fn equipment_inventory_resolves_pet_gear() {
    let save = require_save!();
    assert_eq!(save.equipment.len(), 209);

    // Salamander's weapon, instance 704: "Inferno Sword + 10, SSS, Wind gem
    // lv 10" per the Pet Stats export.
    let weapon = save.equipment_by_instance_id(704).expect("instance 704");
    assert_eq!(weapon.plus, 10);
    assert_eq!(weapon.quality, 8); // SSS
    assert_eq!(weapon.gem_level, 10);
    assert_eq!(weapon.gem_element, Some(Element::Wind));

    // Every equipped item id on every pet resolves to an inventory instance.
    for pet in &save.pets {
        for id in [pet.weapon_id, pet.armor_id, pet.accessory_id].into_iter().flatten() {
            assert!(
                save.equipment_by_instance_id(id).is_some(),
                "{}'s equip instance {id} should exist in X.R",
                pet.name
            );
        }
    }
}

#[test]
fn materials_match_main_stats_export() {
    let save = require_save!();
    let by_name = |name: &str| {
        save.materials
            .iter()
            .find(|m| m.name() == Some(name))
            .map(|m| m.count)
    };
    // All four export-confirmed ids, resolved through the name table:
    assert_eq!(by_name("Strategy Book"), Some(2840)); // "Strategy Books: 2,840"
    assert_eq!(by_name("Ant"), Some(192164)); // "Ants: 192,164"
    assert_eq!(by_name("Honey"), Some(787)); // "Honey: 787"
    assert_eq!(by_name("Acorn"), Some(24727)); // "Acorns: 24,727"
}

#[test]
fn campaigns_have_twelve_hour_durations() {
    let save = require_save!();
    assert_eq!(save.campaigns.len(), 8);
    for slot in save.campaigns.iter().filter(|c| !c.pet_type_ids.is_empty()) {
        assert_eq!(slot.duration_ms, 43_200_000, "slot {}", slot.index);
        assert!(slot.pet_type_ids.len() <= 10);
        for id in &slot.pet_type_ids {
            assert!(
                save.pet_by_type_id(*id).is_some(),
                "campaign pet id {id} should resolve"
            );
        }
    }
}

// ---- Second save (2026-06-10) + inventory transcription cross-checks ----

#[test]
fn second_save_materials_match_transcription() {
    let save = require_second_save!();
    let by_name = |name: &str| {
        save.materials
            .iter()
            .find(|m| m.name() == Some(name))
            .map(|m| m.count)
    };
    // Exact matches with the manual inventory transcription:
    assert_eq!(by_name("Antidote"), Some(128)); // was wrongly "Nothing" before
    assert_eq!(by_name("Torch"), Some(2332));
    assert_eq!(by_name("Health Potion X"), Some(794));
    assert_eq!(by_name("Health Potion S"), Some(798));
    assert_eq!(by_name("Nothing"), Some(678)); // id 119
    assert_eq!(by_name("Glowing Embers"), Some(225));
    assert_eq!(by_name("Rebirth Bacon"), Some(1935));
    assert_eq!(by_name("Ale"), Some(2162));
    assert_eq!(by_name("Flying Boots"), Some(1512));
    assert_eq!(by_name("Lucky Talisman"), Some(587));
}

#[test]
fn second_save_foods_and_chocolate_match_transcription() {
    let save = require_second_save!();
    assert_eq!(save.puny_food, 123_548);
    assert_eq!(save.strong_food, 16_276);
    assert_eq!(save.mighty_food, 7_239);
    assert_eq!(save.chocolate, 9_989);
}

#[test]
fn second_save_gems_match_transcription() {
    let save = require_second_save!();
    let count = |el: Element, lvl: u32| {
        save.gems
            .iter()
            .find(|g| g.element == Some(el) && g.level == lvl)
            .map(|g| g.count)
    };
    assert_eq!(count(Element::Neutral, 1), Some(3796));
    assert_eq!(count(Element::Fire, 1), Some(2353));
    assert_eq!(count(Element::Water, 1), Some(2225));
    assert_eq!(count(Element::Earth, 1), Some(7882));
    assert_eq!(count(Element::Wind, 1), Some(5220));
    assert_eq!(count(Element::Water, 10), Some(1));
    assert_eq!(count(Element::Wind, 10), Some(1));
}

#[test]
fn second_save_normal_level_and_stats() {
    let save = require_second_save!();
    // User-confirmed displayed values, same day as the save:
    let gnome = save.pet_by_name("Gnome").unwrap();
    assert_eq!(gnome.normal_level, 13_724);
    // Displayed: Health 36.885e9, Physical 3.688e9 (Health = 10 × Physical).
    assert!((gnome.normal_health - 36.885e9).abs() / 36.885e9 < 1e-3);
    assert!((gnome.physical_stat() - 3.688e9).abs() / 3.688e9 < 1e-3);

    let anni = save.pet_by_name("Anni Cake").unwrap();
    assert_eq!(anni.normal_level, 10_861);

    // Fire Fox and Swan were both level 2,052.
    assert_eq!(save.pet_by_name("Fire Fox").unwrap().normal_level, 2052);
    assert_eq!(save.pet_by_name("Swan").unwrap().normal_level, 2052);
}

#[test]
fn training_clone_stats_match_per_mille_settings() {
    let save = require_second_save!();
    // The user's global training settings: Physical‰ 1, Mystic‰ 556,
    // Battle‰ 550. Stored clone stats keep exactly those ratios, and
    // clone HP = 10 × clone Physical (the Health rule).
    for pet in save.pets.iter().filter(|p| p.clone_physical > 1.0) {
        let o = pet.clone_physical;
        assert!((pet.clone_mystic / o - 556.0).abs() < 1e-6, "{}", pet.name);
        assert!((pet.clone_battle / o - 550.0).abs() < 1e-6, "{}", pet.name);
        assert!((pet.clone_hp / o - 10.0).abs() < 1e-9, "{}", pet.name);
    }
}

#[test]
fn clone_stats_are_a_snapshot_health_is_live() {
    // Across the two saves (one day apart): the training-clone stats are
    // bit-identical (configuration snapshot), while normal Health moved by
    // ~30% (Anni Cake bonus accumulation).
    let (Some(save1), Some(save2)) = (load_reference_save(), load_second_save()) else {
        eprintln!("reference saves not present; skipping");
        return;
    };
    let g1 = save1.pet_by_name("Gnome").unwrap();
    let g2 = save2.pet_by_name("Gnome").unwrap();
    assert_eq!(g1.clone_physical, g2.clone_physical);
    assert_eq!(g1.clone_mystic, g2.clone_mystic);
    assert!(g2.normal_health > g1.normal_health * 1.2);
}

#[test]
fn equipment_multipliers_follow_wiki_rules() {
    let save = require_second_save!();
    // Salamander's Inferno Sword is SSS +10: quality 1.3, upgrade 1.5.
    let salamander = save.pet_by_name("Salamander").unwrap();
    let weapon = save
        .equipment_by_instance_id(salamander.weapon_id.unwrap())
        .unwrap();
    assert_eq!(weapon.quality_name(), Some("SSS"));
    assert!((weapon.quality_multiplier() - 1.3).abs() < 1e-9);
    assert!((weapon.upgrade_multiplier() - 1.5).abs() < 1e-9);
    assert!((weapon.stat_multiplier() - 1.95).abs() < 1e-9);
}

#[test]
fn second_save_equipment_type_names_resolve() {
    let save = require_second_save!();
    // Salamander's weapon is an Inferno Sword (type 21, confirmed earlier).
    let salamander = save.pet_by_name("Salamander").unwrap();
    let weapon = save
        .equipment_by_instance_id(salamander.weapon_id.unwrap())
        .unwrap();
    assert_eq!(weapon.type_name(), Some("Inferno Sword"));

    // Pandora's box wears the Magic Egg (type 304).
    let pandora = save.pet_by_name("Pandora's box").unwrap();
    let egg = save
        .equipment_by_instance_id(pandora.weapon_id.unwrap())
        .unwrap();
    assert_eq!(egg.type_name(), Some("Magic Egg"));

    // Per-type instance counts, asserting only types where the save and the
    // transcription agree exactly (a few others drifted by items the
    // blacksmiths crafted between save and transcription):
    let count_of = |name: &str| {
        save.equipment
            .iter()
            .filter(|e| e.type_name() == Some(name))
            .count()
    };
    assert_eq!(count_of("Inferno Sword"), 10);
    assert_eq!(count_of("Titanium Armor"), 11);
    assert_eq!(count_of("Storm Bow"), 3);
}

#[test]
fn raw_tree_keeps_unidentified_fields_reachable() {
    let save = require_save!();
    // X.v = 10062 — meaning still unknown; the raw tree must keep it visible.
    let v = save.root.get_path(&["X", "v"]).and_then(|n| n.as_u64());
    assert_eq!(v, Some(10062));
    // Unknown per-pet fields stay on the pet's raw node.
    let cat = save.pet_by_name("Cat").unwrap();
    assert_eq!(cat.raw.get("H").and_then(|n| n.as_u64()), Some(10920));
}
