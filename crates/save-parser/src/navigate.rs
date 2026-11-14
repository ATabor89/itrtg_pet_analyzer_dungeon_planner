//! Pure helpers for navigating the raw save tree, shared by the front ends'
//! tree views: resolving id fields to names (and equipment references to their
//! item), and the search pre-pass that finds every node matching a query.
use std::collections::HashSet;

use itrtg_models::{Class, Element};

use crate::labels::Resolve;
use crate::raw::{Field, Raw};
use crate::registry::FieldRegistry;
use crate::{items, model};

/// Turn an id field's raw value into a human name, per its [`Resolve`] hint.
/// `root` is the whole tree, needed to chase an equipment-instance id into `X.R`.
pub fn resolve_name(resolve: Resolve, value: &str, root: &Raw) -> Option<String> {
    let v = value.trim();
    match resolve {
        Resolve::Literal => (!v.is_empty()).then(|| v.to_string()),
        Resolve::Material => items::material_name(v.parse().ok()?).map(str::to_string),
        Resolve::Dungeon => items::dungeon_name(v.parse().ok()?).map(str::to_string),
        Resolve::Pond => items::pond_name(v.parse().ok()?).map(str::to_string),
        Resolve::Statue => items::statue_name(v.parse().ok()?).map(str::to_string),
        Resolve::VillageBuilding => items::village_building_name(v.parse().ok()?).map(str::to_string),
        Resolve::UltimateBeing => items::ultimate_being_name(v.parse().ok()?).map(str::to_string),
        Resolve::UltimateBeingV2 => {
            items::ultimate_being_name(v.parse().ok()?).map(|n| format!("{n} V2"))
        }
        Resolve::Equipment => items::equipment_type_name(v.parse().ok()?).map(str::to_string),
        Resolve::Monument => items::monument_name(v.parse().ok()?).map(str::to_string),
        Resolve::Might => items::might_name(v.parse().ok()?).map(str::to_string),
        Resolve::Creation => items::creation_name(v.parse().ok()?).map(str::to_string),
        Resolve::SpaceDim => items::spacedim_name(v.parse().ok()?).map(str::to_string),
        Resolve::PhysicalTraining => {
            items::physical_training_name(v.parse().ok()?).map(str::to_string)
        }
        Resolve::Skill => items::skill_name(v.parse().ok()?).map(str::to_string),
        Resolve::Monster => items::monster_name(v.parse().ok()?).map(str::to_string),
        Resolve::DivinityUpgrade => {
            items::divinity_upgrade_name(v.parse().ok()?).map(str::to_string)
        }
        Resolve::CrystalModule => items::crystal_module_name(v.parse().ok()?).map(str::to_string),
        Resolve::AdventureProfession => {
            items::adventure_profession_name(v.parse().ok()?).map(str::to_string)
        }
        Resolve::TavernQuest => items::tavern_quest_name(v.parse().ok()?).map(str::to_string),
        Resolve::QuestRank => items::quest_rank_name(v.parse().ok()?).map(str::to_string),
        Resolve::AdventureItem => items::adventure_item_name(v.parse().ok()?).map(str::to_string),
        Resolve::AdventureEnemy => items::adventure_enemy_name(v.parse().ok()?).map(str::to_string),
        Resolve::AdventureClass => items::adventure_class_name(v.parse().ok()?).map(str::to_string),
        Resolve::Research => model::research_name(v.parse().ok()?).map(str::to_string),
        Resolve::Element => model::element_from_id(v.parse().ok()?).map(element_name),
        Resolve::Class => model::class_from_id(v.parse().ok()?).map(class_name),
        Resolve::PetType => items::pet_type_name(v.parse().ok()?).map(str::to_string),
        Resolve::ElementalForm => {
            items::elemental_form_name(v.parse().ok()?).map(str::to_string)
        }
        Resolve::CampaignType => items::campaign_type_name(v.parse().ok()?).map(str::to_string),
        Resolve::CampaignPref => {
            // Stored offset by 1: 0 = unset, else campaign_type_name(id - 1).
            let id: u32 = v.parse().ok()?;
            match id.checked_sub(1) {
                None => Some("Unset".to_string()),
                Some(c) => items::campaign_type_name(c).map(str::to_string),
            }
        }
        Resolve::FeedingSetting => {
            items::feeding_setting_name(v.parse().ok()?).map(str::to_string)
        }
        Resolve::GemElement => items::gem_element_name(v.parse().ok()?).map(str::to_string),
        Resolve::Challenge => items::challenge_name(v.parse().ok()?).map(str::to_string),
        Resolve::ChallengeDifficulty => {
            items::challenge_difficulty_name(v.parse().ok()?).map(str::to_string)
        }
        Resolve::UltimateOverflowUpgrade => {
            items::ultimate_overflow_upgrade_name(v.parse().ok()?).map(str::to_string)
        }
        Resolve::RtiBonus => items::rti_bonus_name(v.parse().ok()?).map(str::to_string),
        Resolve::EquipmentInstance => resolve_equipment_instance(v, root),
        // Node-based: handled directly in `element_label`, never as a scalar.
        Resolve::EquipmentNode | Resolve::CoreNode => None,
    }
}

/// Format a core element struct as "Enemy Quality" (e.g. "Slime SSS"); falls
/// back to "Core" / omits the quality when an id is unknown.
pub fn core_label(node: &Raw) -> Option<String> {
    let enemy_id = scalar_u32(node, "a")?;
    let name = items::adventure_enemy_name(enemy_id).unwrap_or("Core");
    let mut s = name.to_string();
    if let Some(q) = scalar_u32(node, "d").and_then(items::quality_name) {
        s.push(' ');
        s.push_str(q);
    }
    Some(s)
}

/// Format an equipment element struct as "Name Quality+Plus" (e.g.
/// "Magic Stick SSS+20"); the +plus is omitted at +0.
pub fn equip_label(node: &Raw) -> Option<String> {
    let type_id = scalar_u32(node, "a")?;
    let name = items::equipment_type_name(type_id).unwrap_or("Equipment");
    let mut s = name.to_string();
    if let Some(q) = scalar_u32(node, "c").and_then(items::quality_name) {
        s.push(' ');
        s.push_str(q);
    }
    match scalar_u32(node, "b") {
        Some(plus) if plus > 0 => s.push_str(&format!("+{plus}")),
        _ => {}
    }
    Some(s)
}

fn element_name(e: Element) -> String {
    match e {
        Element::Fire => "Fire",
        Element::Water => "Water",
        Element::Wind => "Wind",
        Element::Earth => "Earth",
        Element::Neutral => "Neutral",
        Element::All => "All",
    }
    .to_string()
}

fn class_name(c: Class) -> String {
    match c {
        Class::Adventurer => "Adventurer",
        Class::Blacksmith => "Blacksmith",
        Class::Alchemist => "Alchemist",
        Class::Defender => "Defender",
        Class::Supporter => "Supporter",
        Class::Rogue => "Rogue",
        Class::Assassin => "Assassin",
        Class::Mage => "Mage",
        Class::Wildcard => "Wildcard",
    }
    .to_string()
}

/// Resolve a pet's equipment-slot id (`w.e`/`w.f`/`w.g`) to the item, e.g.
/// "Magic Stick SSS+20". A slot value of 0 means **empty** (no annotation).
///
/// Matches on the **equip id** (`d`) first — that's what slots reference — then
/// the catalog id `h` as a fallback. See the inline note for the proof and the
/// collision the old "`h` first" rule got wrong.
fn resolve_equipment_instance(value: &str, root: &Raw) -> Option<String> {
    let instance: u32 = value.parse().ok()?;
    if instance == 0 {
        return None; // 0 = empty slot
    }
    let Raw::List(list) = root.get_path(&["X", "R"])? else {
        return None;
    };
    // Pet slots (`w.e`/`w.f`/`w.g`) reference the **equip id `d`** (0 = empty /
    // in inventory) — proven 2026-06-19 on real Steam saves: event gear with
    // `d≠h` resolves by `d` (e.g. Vampire's armor slot 23 = `d`23 = Merry
    // Mantle, while that instance's `h` is 136). Match `d` first; fall back to
    // the always-unique catalog id `h` only when no `d` matches (defensive —
    // and what prevents the `d=20`/`h=20` cross-field collision seen on edited
    // saves from picking the wrong item).
    let item = list
        .iter()
        .find(|it| scalar_u32(it, "d") == Some(instance))
        .or_else(|| list.iter().find(|it| scalar_u32(it, "h") == Some(instance)))?;
    equip_label(item)
}

/// The `X.R` list **index** of the equipment instance a slot value references,
/// using the same `d`-first / `h`-fallback matching as
/// [`resolve_equipment_instance`]. Drives the cross-reference jump (a pet's
/// weapon/armor/accessory slot → the item's node). `None` for 0/empty or no match.
pub fn equipment_instance_index(value: &str, root: &Raw) -> Option<usize> {
    let instance: u32 = value.parse().ok()?;
    if instance == 0 {
        return None;
    }
    let Raw::List(list) = root.get_path(&["X", "R"])? else {
        return None;
    };
    list.iter()
        .position(|it| scalar_u32(it, "d") == Some(instance))
        .or_else(|| list.iter().position(|it| scalar_u32(it, "h") == Some(instance)))
}

fn scalar_u32(node: &Raw, key: &str) -> Option<u32> {
    match node.get(key) {
        Some(Raw::Scalar(s)) => s.parse().ok(),
        _ => None,
    }
}
/// The registry display-name at `path`, lowercased-contains the query?
fn name_matches(registry: &FieldRegistry, path: &[String], query: &str) -> bool {
    let p: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
    registry
        .lookup(&p)
        .is_some_and(|d| d.name.to_lowercase().contains(query))
}

/// The set of a dotted path and all its ancestor prefixes, e.g. `X.Q.1.b` →
/// {`X`, `X.Q`, `X.Q.1`, `X.Q.1.b`}. Used to drive a navigation jump: the walk
/// force-opens every container on the path and scrolls to the leaf, exactly as
/// Reveal mode does for a search match.
pub fn prefix_set(dotted: &str) -> HashSet<String> {
    let parts: Vec<&str> = dotted.split('.').collect();
    (1..=parts.len()).map(|i| parts[..i].join(".")).collect()
}

/// Build the set of node paths (dotted) that match the query or are an ancestor
/// of a match — a single pass over the whole tree.
pub fn build_matches(root: &Raw, query: &str, registry: &FieldRegistry) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut path: Vec<String> = Vec::new();
    if let Raw::Struct(fields) = root.peel() {
        for (k, f) in fields {
            if let Field::Value(v) = f {
                path.push(k.clone());
                collect_matches(v, &mut path, query, registry, &mut out);
                path.pop();
            }
        }
    }
    out
}

/// Returns whether `node` (at `path`) matches or contains a match, inserting
/// every such node's dotted path into `out`.
fn collect_matches(
    node: &Raw,
    path: &mut Vec<String>,
    query: &str,
    registry: &FieldRegistry,
    out: &mut HashSet<String>,
) -> bool {
    // This node matches if its key (last path segment) or registry name matches.
    let mut found = name_matches(registry, path, query)
        || path
            .last()
            .is_some_and(|seg| seg.to_lowercase().contains(query));

    match node.peel() {
        Raw::Scalar(s) => {
            if s.to_lowercase().contains(query) {
                found = true;
            }
        }
        Raw::Struct(fields) => {
            for (k, f) in fields {
                if let Field::Value(v) = f {
                    path.push(k.clone());
                    found |= collect_matches(v, path, query, registry, out);
                    path.pop();
                }
            }
        }
        Raw::List(items) => {
            for (i, it) in items.iter().enumerate() {
                path.push(i.to_string());
                found |= collect_matches(it, path, query, registry, out);
                path.pop();
            }
        }
        Raw::Base64(_) => {}
    }

    if found {
        out.insert(path.join("."));
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scalar(s: &str) -> Field {
        Field::Value(Raw::Scalar(s.into()))
    }

    /// Searching a registry display name finds the labeled node (and its
    /// ancestors), not just literal occurrences of the text elsewhere.
    #[test]
    fn search_matches_registry_labels() {
        // `e` is labeled "Shadow Clones"; `e.a`/`e.b` "Shadow Clones (current/max)".
        let root = Raw::Struct(vec![(
            "e".into(),
            Field::Value(Raw::Base64(Box::new(Raw::Struct(vec![
                ("a".into(), scalar("5")),
                ("b".into(), scalar("9")),
            ])))),
        )]);
        let reg = FieldRegistry::new();
        let m = build_matches(&root, "shadow", &reg);
        assert!(m.contains("e"), "ancestor block marked");
        assert!(m.contains("e.a"), "leaf matched by its label");
        assert!(m.contains("e.b"));
    }

    #[test]
    fn resolves_ids_to_names() {
        let empty = Raw::Struct(vec![]);
        // Class / element ids via the model tables.
        assert_eq!(resolve_name(Resolve::Class, "8", &empty).as_deref(), Some("Mage"));
        assert_eq!(resolve_name(Resolve::Element, "1", &empty).as_deref(), Some("Fire"));
        // Dungeon ids (dungeon-team / active-run blocks).
        assert_eq!(resolve_name(Resolve::Dungeon, "2", &empty).as_deref(), Some("Scrapyard"));
        assert_eq!(resolve_name(Resolve::Dungeon, "5", &empty).as_deref(), Some("Forest"));
        // Fishing pond.
        assert_eq!(resolve_name(Resolve::Pond, "4", &empty).as_deref(), Some("Sad Pond"));
        // Monument id matches the items table (don't hardcode the name).
        assert_eq!(
            resolve_name(Resolve::Monument, "0", &empty),
            crate::items::monument_name(0).map(str::to_string)
        );
        // Physical-training / skill / monster ids via the items tables.
        assert_eq!(
            resolve_name(Resolve::PhysicalTraining, "0", &empty),
            crate::items::physical_training_name(0).map(str::to_string)
        );
        assert_eq!(
            resolve_name(Resolve::Skill, "0", &empty),
            crate::items::skill_name(0).map(str::to_string)
        );
        // Adventure item / core-enemy ids (separate namespaces).
        assert_eq!(
            resolve_name(Resolve::AdventureItem, "3", &empty),
            crate::items::adventure_item_name(3).map(str::to_string)
        );
        assert_eq!(
            resolve_name(Resolve::AdventureEnemy, "50", &empty),
            crate::items::adventure_enemy_name(50).map(str::to_string)
        );
        // A core element struct titles as "Enemy Quality".
        let core = Raw::Struct(vec![
            ("a".into(), scalar("50")), // Slime
            ("d".into(), scalar("8")),  // SSS
        ]);
        assert_eq!(core_label(&core).as_deref(), Some("Slime SSS"));
        assert_eq!(
            resolve_name(Resolve::Monster, "33", &empty),
            crate::items::monster_name(33).map(str::to_string)
        );
        // A literal is returned as-is; an unknown id resolves to nothing.
        assert_eq!(resolve_name(Resolve::Literal, "Robot", &empty).as_deref(), Some("Robot"));
        assert!(resolve_name(Resolve::Material, "not-a-number", &empty).is_none());
    }

    #[test]
    fn resolves_equipment_instance_across_the_tree() {
        // Pet slots reference the equip id `d`; `h` is a separate always-unique
        // catalog id. An unequipped inventory item carries `d=0` and is never
        // referenced by a slot — but resolution falls back to `h` for it.
        let root = Raw::Struct(vec![(
            "X".into(),
            Field::Value(Raw::Base64(Box::new(Raw::Struct(vec![(
                "R".into(),
                Field::Value(Raw::List(vec![Raw::Struct(vec![
                    ("a".into(), scalar("51")),  // type id
                    ("b".into(), scalar("20")),  // plus
                    ("c".into(), scalar("8")),   // quality (SSS)
                    ("d".into(), scalar("0")),   // not equipped → equip id 0
                    ("h".into(), scalar("858")), // unique catalog id
                ])])),
            )])))),
        )]);
        let got = resolve_name(Resolve::EquipmentInstance, "858", &root).expect("via h fallback");
        assert!(got.contains("SSS"), "quality letter: {got}");
        assert!(got.contains("+20"), "plus level: {got}");
        assert!(!got.contains("q8"), "no raw quality id: {got}");
        // 0 = empty slot → no annotation (even though an item has d==0).
        assert!(resolve_name(Resolve::EquipmentInstance, "0", &root).is_none());
        assert!(resolve_name(Resolve::EquipmentInstance, "999", &root).is_none());
    }

    /// `d`/`h` cross-field collision: a slot value matches one item's equip id
    /// `d` and a *different* item's catalog id `h`. The slot must resolve by `d`
    /// (proven on real Steam saves; this is the case the old "h first" rule got
    /// wrong on edited saves — Legendary `d=20` vs Magic Stick `h=20`).
    #[test]
    fn equipment_slot_resolves_by_d_on_collision() {
        let item = |a: &str, d: &str, h: &str| {
            Raw::Struct(vec![
                ("a".into(), scalar(a)),
                ("b".into(), scalar("10")),
                ("c".into(), scalar("6")),
                ("d".into(), scalar(d)),
                ("h".into(), scalar(h)),
            ])
        };
        let root = Raw::Struct(vec![(
            "X".into(),
            Field::Value(Raw::Base64(Box::new(Raw::Struct(vec![(
                "R".into(),
                Field::Value(Raw::List(vec![
                    item("51", "0", "20"), // Magic Stick, unequipped, catalog id 20
                    item("80", "20", "0"), // Legendary Stick, equipped, equip id 20
                ])),
            )])))),
        )]);
        // Slot 20 → the equipped Legendary Stick (type 80), not the Magic Stick.
        let got = resolve_name(Resolve::EquipmentInstance, "20", &root).expect("resolves");
        assert!(got.contains("Legendary Stick"), "got {got}");
    }

    /// The cross-ref jump finds the item's `X.R` index by the same `d`-first /
    /// `h`-fallback rule as resolution (here index 1 holds `d=20`).
    #[test]
    fn equipment_instance_index_finds_the_item() {
        let item = |a: &str, d: &str, h: &str| {
            Raw::Struct(vec![
                ("a".into(), scalar(a)),
                ("d".into(), scalar(d)),
                ("h".into(), scalar(h)),
            ])
        };
        let root = Raw::Struct(vec![(
            "X".into(),
            Field::Value(Raw::Base64(Box::new(Raw::Struct(vec![(
                "R".into(),
                Field::Value(Raw::List(vec![
                    item("51", "0", "858"), // [0] unequipped
                    item("80", "20", "0"),  // [1] equipped, equip id 20
                ])),
            )])))),
        )]);
        assert_eq!(equipment_instance_index("20", &root), Some(1)); // by d
        assert_eq!(equipment_instance_index("858", &root), Some(0)); // by h fallback
        assert_eq!(equipment_instance_index("0", &root), None); // empty slot
        assert_eq!(equipment_instance_index("999", &root), None); // no match
    }

    /// "Copy node (raw)" copies `value.peel().serialize()`: a base64-wrapped
    /// struct copies as its readable `k:v;` text (not a base64 blob), and the
    /// result re-parses to the same tree (so it can be pasted back later, #17).
    #[test]
    fn copy_node_raw_is_readable_and_round_trips() {
        let wrapped = Raw::Base64(Box::new(Raw::Struct(vec![
            ("a".into(), scalar("5")),
            ("b".into(), scalar("9")),
        ])));
        let copied = wrapped.peel().serialize();
        assert_eq!(copied, "a:5;b:9;", "readable struct text, not a base64 blob");
        // The copied text re-parses back to the same (peeled) structure.
        assert_eq!(crate::raw::parse(&copied), *wrapped.peel());
    }

    /// A jump's prefix set force-opens every ancestor container and the leaf.
    #[test]
    fn prefix_set_includes_path_and_all_ancestors() {
        let s = prefix_set("X.Q.1.b");
        assert!(s.contains("X"));
        assert!(s.contains("X.Q"));
        assert!(s.contains("X.Q.1"));
        assert!(s.contains("X.Q.1.b"));
        assert_eq!(s.len(), 4);
        // A single segment is its own only prefix.
        assert_eq!(prefix_set("W"), HashSet::from(["W".to_string()]));
    }

    /// A value/key match is included; unrelated siblings are not.
    #[test]
    fn search_matches_value_and_excludes_unrelated() {
        let root = Raw::Struct(vec![("W".into(), scalar("Adam")), ("c".into(), scalar("123"))]);
        let reg = FieldRegistry::new();
        let m = build_matches(&root, "adam", &reg);
        assert!(m.contains("W"), "matched by scalar value");
        assert!(!m.contains("c"), "unrelated field excluded");
    }
}
