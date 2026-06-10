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
    assert_eq!(save.player_name.as_deref(), Some("ShoggothUnknown"));
    assert_eq!(save.god_name.as_deref(), Some("Shoggoth269"));
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
fn current_exp_and_working_experience() {
    let save = require_second_save!();
    // User readings: Gnome current exp 1.115e12; Fire Fox & Swan 4.949e9.
    let gnome = save.pet_by_name("Gnome").unwrap();
    assert!((gnome.current_exp - 1.115e12).abs() / 1.115e12 < 1e-3);
    let ff = save.pet_by_name("Fire Fox").unwrap();
    let swan = save.pet_by_name("Swan").unwrap();
    assert_eq!(ff.current_exp, swan.current_exp); // same level, same exp state
    assert!((ff.current_exp - 4.949e9).abs() / 4.949e9 < 1e-3);

    // Working experience in ms: Lamb ≈ 108d12h, Santa ≈ 64d18h at save time
    // (user readings minus the elapsed 4h50m matched to within seconds).
    assert_eq!(
        save.pet_by_name("Lamb").unwrap().working_experience_ms,
        9_375_772_300
    );
    assert_eq!(
        save.pet_by_name("Santa").unwrap().working_experience_ms,
        5_597_822_340
    );
}

#[test]
fn partner_days_increment_daily() {
    let (Some(save1), Some(save2)) = (load_reference_save(), load_second_save()) else {
        eprintln!("reference saves not present; skipping");
        return;
    };
    // Every partnered pet gained exactly +1 partner day between the saves.
    for p1 in save1.pets.iter().filter(|p| p.partner_type_id.is_some()) {
        let p2 = save2.pet_by_name(&p1.name).unwrap();
        assert_eq!(
            p2.partner_days,
            p1.partner_days + 1,
            "{} partner days",
            p1.name
        );
    }
}

#[test]
fn global_trackers_match_exports_and_tooltips() {
    use save_parser::model::trackers;
    let (Some(save1), Some(save2)) = (load_reference_save(), load_second_save()) else {
        eprintln!("reference saves not present; skipping");
        return;
    };
    let t1 = |k| save1.global_tracker(k).unwrap();
    let t2 = |k| save2.global_tracker(k).unwrap();

    // Main Stats export (save 1): "Chocobear hours: 4,826", "Caterpillar
    // materials upgraded: 2,865", "Growth from Golden Dragon: 184,999",
    // "Growth from Bag: 5,483", "Earth Eater Earthlike planets eaten:
    // 7.308 E+6", "Dungeon Bosses defeated: 2,244", "Crystal Power (4,183)".
    assert_eq!(t1(trackers::CHOCOBEAR_BANKED_HOURS).floor(), 4826.0);
    assert_eq!(t1(trackers::CATERPILLAR_MATERIALS_UPGRADED), 2865.0);
    assert_eq!(t1(trackers::GOLD_DRAGON_BONUS_GROWTH).floor(), 184999.0);
    assert_eq!(t1(trackers::BAG_BONUS_GROWTH).floor(), 5482.0);
    assert_eq!(t1(trackers::EARTH_EATER_PLANETS_TOTAL), 7_308_846.0);
    assert_eq!(t1(trackers::DUNGEON_BOSSES), 2244.0);
    assert_eq!(t1(trackers::CRYSTAL_POWER), 4183.0);

    // User tooltip readings (after save 2): Meteor 4,572.11 campaign hours,
    // Serow 7,552 items saved, Mule 124 quests (123 in save 1), Aether 28
    // boss kills, God Power 863 hours.
    assert!((t2(trackers::METEOR_CAMPAIGN_HOURS) - 4572.11).abs() < 0.01);
    assert_eq!(t2(trackers::SEROW_ITEMS_SAVED), 7552.0);
    assert_eq!(t1(trackers::MULE_QUESTS), 123.0);
    assert_eq!(t2(trackers::MULE_QUESTS), 124.0);
    assert_eq!(t2(trackers::AETHER_BOSS_KILLS), 28.0);
    assert_eq!(t2(trackers::GOD_POWER_CAMPAIGN_HOURS), 863.0);

    // Pandora's feedings counter can be negative (observed after rebirth).
    assert_eq!(t1(trackers::PANDORA_FEEDINGS), -28.0);
    assert_eq!(t2(trackers::PANDORA_FEEDINGS), 27.0);
}

#[test]
fn anni_cake_bonus_is_stored_directly() {
    let (Some(save1), Some(save2)) = (load_reference_save(), load_second_save()) else {
        eprintln!("reference saves not present; skipping");
        return;
    };
    // Root `033` holds the bonus as a fractional percent. The user
    // predicted "709% in the first save" from the 10%/hour accrual — and
    // save 1 stores exactly 709.02; save 2's 948.97 displays as 949%.
    assert_eq!(save1.anni_cake_bonus_percent, Some(709.0245829717));
    assert_eq!(save2.anni_cake_bonus_percent, Some(948.969027416145));
    let delta = save2.anni_cake_bonus_percent.unwrap() - save1.anni_cake_bonus_percent.unwrap();
    // ~24 hours of food campaigns at 10%/hour credited between the saves.
    assert!((delta - 239.94).abs() < 0.01);
}

#[test]
fn researches_match_main_stats_export() {
    use save_parser::model::{research_name, researches};
    let save = require_second_save!();
    assert_eq!(save.researches.len(), 44); // ids 0–43; id 0 is a placeholder

    // Main Stats export "Researches" section (save 1 era, unchanged):
    assert_eq!(save.research_level(researches::PET_STATS), 5);
    assert_eq!(save.research_level(6), 22); // Core Drop Rate
    assert_eq!(save.research_level(7), 40); // Core Quality
    assert_eq!(save.research_level(26), 20); // Research Speed
    assert_eq!(save.research_level(27), 2); // Research Slots
    assert_eq!(save.research_level(researches::ALCHEMY_SPEED), 10);
    assert_eq!(research_name(researches::PET_STATS), Some("Pet Stats"));
    assert_eq!(research_name(43), Some("Core Removal Cost"));
    assert_eq!(research_name(0), None);

    // "Research Slots Level: 2" — exactly two researches in progress.
    let active: Vec<u32> = save
        .researches
        .iter()
        .filter(|r| r.in_progress)
        .map(|r| r.id)
        .collect();
    assert_eq!(active.len(), 2);
    assert!(active.contains(&6)); // Core Drop Rate
    assert!(active.contains(&31)); // Spacedim Speed
}

#[test]
fn exp_counters_store_current_toward_next_level() {
    use save_parser::formulas::{class_exp_to_next, dungeon_exp_to_next};
    let save = require_second_save!();
    // User readings (taken after save 2, values unchanged since):
    // Salamander DL 101 (147,749 / 323,387), CL 25 (62,692 / 1.251e6);
    // Hedgehog DL 20 (0 / 8,459), CL 22 (476,666 / 969,000);
    // Succubus DL 80 (130,099 / 191,405), CL 19 (44,700 / 723,000).
    for (name, dl, dexp, cl, cexp) in [
        ("Salamander", 101, 147_749.0, 25, 62_692.0),
        ("Hedgehog", 20, 0.0, 22, 476_666.0),
        ("Succubus", 80, 130_099.0, 19, 44_700.0),
    ] {
        let pet = save.pet_by_name(name).unwrap();
        assert_eq!(pet.dungeon_level, dl, "{name} DL");
        assert_eq!(pet.dungeon_exp, dexp, "{name} dungeon exp");
        assert_eq!(pet.class_level, cl, "{name} CL");
        assert_eq!(pet.class_exp, cexp, "{name} class exp");
        // Stored exp is always below the requirement for the next level.
        assert!(pet.dungeon_exp < dungeon_exp_to_next(pet.dungeon_level));
        assert!(pet.class_exp < class_exp_to_next(pet.class_level));
    }
}

#[test]
fn class_exp_reset_on_level_up_across_saves() {
    let (Some(save1), Some(save2)) = (load_reference_save(), load_second_save()) else {
        eprintln!("reference saves not present; skipping");
        return;
    };
    // Salamander was CL 24 in save 1 with 1,144,938 class exp — just shy of
    // the 1,153,000 needed for CL 25. By save 2 he is CL 25 with a small,
    // freshly reset counter.
    let s1 = save1.pet_by_name("Salamander").unwrap();
    let s2 = save2.pet_by_name("Salamander").unwrap();
    assert_eq!(s1.class_level, 24);
    assert_eq!(s1.class_exp, 1_144_938.0);
    assert!(s1.class_exp < save_parser::formulas::class_exp_to_next(24));
    assert_eq!(s2.class_level, 25);
    assert!(s2.class_exp < s1.class_exp);
}

#[test]
fn raw_tree_keeps_unidentified_fields_reachable() {
    let save = require_save!();
    // X.z — meaning still unknown; the raw tree must keep it visible.
    let z = save.root.get_path(&["X", "z"]).and_then(|n| n.as_u64());
    assert_eq!(z, Some(13253888));
    // Unknown per-pet fields stay on the pet's raw node (Santa t = 4).
    let santa = save.pet_by_name("Santa").unwrap();
    assert_eq!(santa.raw.get("t").and_then(|n| n.as_u64()), Some(4));
}
