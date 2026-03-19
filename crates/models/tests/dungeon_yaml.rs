use itrtg_models::dungeon::DungeonRecommendations;
use itrtg_models::{Class, Dungeon, Element, EquipmentSlot};

const YAML_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/dungeon_recommendations.yaml"
);

#[test]
fn load_full_recommendations() {
    let recs = load();

    // We should have 5 dungeons
    assert_eq!(recs.dungeons.len(), 5, "expected 5 dungeons");

    // Keyed by Dungeon enum
    assert!(recs.dungeons.contains_key(&Dungeon::Scrapyard));
    assert!(recs.dungeons.contains_key(&Dungeon::WaterTemple));
    assert!(recs.dungeons.contains_key(&Dungeon::Volcano));
    assert!(recs.dungeons.contains_key(&Dungeon::Mountain));
    assert!(recs.dungeons.contains_key(&Dungeon::Forest));

    // Each dungeon should have 3 depths
    for (key, dungeon) in &recs.dungeons {
        assert_eq!(
            dungeon.depths.len(),
            3,
            "dungeon '{key:?}' should have 3 depths"
        );
    }
}

#[test]
fn scrapyard_d1_basics() {
    let recs = load();

    let scrapyard = &recs.dungeons[&Dungeon::Scrapyard];
    assert_eq!(scrapyard.name, "Scrapyard");

    let d1 = &scrapyard.depths[&1];
    assert_eq!(d1.rooms, 6);
    assert_eq!(d1.monsters_per_room, 6);
    assert_eq!(d1.gem_level, None);
    assert_eq!(d1.requirements.dungeon_level_avg, 10);
    assert_eq!(d1.requirements.class_level, 0);

    // D1 has a single boss
    assert_eq!(d1.bosses.len(), 1);
    assert_eq!(d1.bosses[0].name, "Oozing Inventor");

    // Party has 6 slots, first should be Rogue
    assert_eq!(d1.party.len(), 6);
    assert_eq!(d1.party[0].class, Some(Class::Rogue));
    // D1 equipment is null
    assert!(d1.party[0].equipment.is_none());
}

#[test]
fn scrapyard_d3_has_multiple_bosses() {
    let recs = load();

    let d3 = &recs.dungeons[&Dungeon::Scrapyard].depths[&3];
    assert_eq!(d3.bosses.len(), 2);
    assert_eq!(d3.bosses[0].name, "Alien Wreckage");
    assert_eq!(d3.bosses[1].name, "Metal Mind");
}

#[test]
fn water_temple_d2_party_equipment() {
    let recs = load();

    let d2 = &recs.dungeons[&Dungeon::WaterTemple].depths[&2];
    // First slot: Assassin with flame_sword
    assert_eq!(d2.party[0].class, Some(Class::Assassin));
    let equip = d2.party[0].equipment.as_ref().unwrap();
    assert_eq!(equip.weapon.as_deref(), Some("flame_sword"));
    assert_eq!(equip.armor.as_deref(), Some("steel_armor"));
}

#[test]
fn mountain_d3_portal_from_beyond_multi_counter() {
    let recs = load();

    let d3 = &recs.dungeons[&Dungeon::Mountain].depths[&3];
    let portal = d3
        .events
        .iter()
        .find(|e| e.name == "Portal From Beyond")
        .expect("should have Portal From Beyond event");

    // This event has a list of counter conditions
    assert_eq!(portal.countered_by.len(), 2);
    assert_eq!(portal.countered_by[0].class, Some(Class::Mage));
    assert_eq!(portal.countered_by[1].element, Some(Element::Neutral));
    assert_eq!(portal.countered_by[1].count, Some(2));
}

#[test]
fn volcano_d2_cursed_chest_class_and_item() {
    let recs = load();

    let d2 = &recs.dungeons[&Dungeon::Volcano].depths[&2];
    let cursed = d2
        .events
        .iter()
        .find(|e| e.name == "Cursed Chest")
        .expect("should have Cursed Chest event");

    // Cursed Chest needs Rogue AND holy_water (single condition, both fields)
    assert_eq!(cursed.countered_by.len(), 1);
    assert_eq!(cursed.countered_by[0].class, Some(Class::Rogue));
    assert_eq!(
        cursed.countered_by[0].item,
        Some("holy_water".to_string())
    );
}

#[test]
fn equipment_catalog_lookup() {
    let recs = load();

    let flame_sword = recs.equipment.lookup("flame_sword").unwrap();
    assert_eq!(flame_sword.name, "Flame Sword");
    assert_eq!(flame_sword.tier, 2);
    assert_eq!(flame_sword.slot, EquipmentSlot::Weapon);
    assert_eq!(flame_sword.element, Some(Element::Fire));

    let alchemist_cape = recs.equipment.lookup("alchemist_cape").unwrap();
    assert_eq!(alchemist_cape.name, "Alchemist Cape");
    assert_eq!(alchemist_cape.slot, EquipmentSlot::Accessory);
    assert!(alchemist_cape.notes.is_some());

    // Generic keys won't be in the catalog
    assert!(recs.equipment.lookup("generic_t2_s10").is_none());
}

#[test]
fn forest_d1_wild_animals_event() {
    let recs = load();

    let d1 = &recs.dungeons[&Dungeon::Forest].depths[&1];
    let wild = d1
        .events
        .iter()
        .find(|e| e.name == "Wild Animals")
        .expect("should have Wild Animals event");

    assert_eq!(wild.countered_by[0].item, Some("pet_food".to_string()));
    assert_eq!(wild.countered_by[0].quantity_per_clear, Some(30));
    assert!(wild.countered_by[0].notes.is_some());
}

#[test]
fn monsters_use_core_element() {
    let recs = load();

    // Water Temple monsters should have Element::Water
    let d1 = &recs.dungeons[&Dungeon::WaterTemple].depths[&1];
    assert_eq!(d1.monsters[0].element, Some(Element::Water));

    // Scrapyard D1 monsters are Neutral element
    let sd1 = &recs.dungeons[&Dungeon::Scrapyard].depths[&1];
    assert_eq!(sd1.monsters[0].element, Some(Element::Neutral));
}

#[test]
fn gem_slots_use_core_element() {
    let recs = load();

    let d2 = &recs.dungeons[&Dungeon::Scrapyard].depths[&2];
    let gems = d2.party[0].equipment.as_ref().unwrap().gems.as_ref().unwrap();
    assert_eq!(gems.weapon, Some(Element::Fire));
    assert_eq!(gems.armor, None);
    assert_eq!(gems.accessory, None);
}

fn load() -> DungeonRecommendations {
    let contents = std::fs::read_to_string(YAML_PATH).unwrap();
    serde_yaml::from_str(&contents).unwrap()
}
