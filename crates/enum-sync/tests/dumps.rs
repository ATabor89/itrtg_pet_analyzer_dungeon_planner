//! Validation against the real decompile dumps.
//!
//! These dumps are game-derived and gitignored (see CLAUDE.md), so they exist
//! only on a machine that has run the decompile. Each test skips cleanly when
//! its dump directory is absent — CI and fresh clones still pass. On a dev box
//! with the dumps present they are the tool's oracle: the 2026-06 update added
//! exactly the Boar pet (and Monk's skills), and Monk the *class* was already
//! present, so the diff must report precisely that.

use enum_sync::parse;
use enum_sync::registry::{self, REGISTRY};
use std::path::{Path, PathBuf};

fn dump(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference/save_file_deserialization")
        .join(name)
}

fn matched_values(dir: &Path, key: &str) -> Option<Vec<(i64, String)>> {
    let enums = parse::parse_dir(dir).expect("dump parses");
    let known = REGISTRY.iter().find(|k| k.key == key).unwrap();
    let (e, _) = registry::match_enum(known, &enums)?;
    Some(e.by_value().iter().map(|(v, n)| (*v, n.to_string())).collect())
}

#[test]
fn new_dump_has_boar_at_152() {
    let new = dump("_cs_decomp_new");
    if !new.exists() {
        eprintln!("skipping: {} not present", new.display());
        return;
    }
    let pets = matched_values(&new, "pets").expect("pets enum located in new dump");
    assert!(
        pets.contains(&(152, "Boar".to_string())),
        "expected Boar = 152 in the refreshed pet enum"
    );
}

#[test]
fn update_added_only_boar_to_pets() {
    let (old, new) = (dump("_cs_decomp"), dump("_cs_decomp_new"));
    if !old.exists() || !new.exists() {
        eprintln!("skipping: need both _cs_decomp and _cs_decomp_new");
        return;
    }
    let old_pets = matched_values(&old, "pets").expect("pets in old");
    let new_pets = matched_values(&new, "pets").expect("pets in new");

    let added: Vec<_> = new_pets.iter().filter(|m| !old_pets.contains(m)).collect();
    let removed: Vec<_> = old_pets.iter().filter(|m| !new_pets.contains(m)).collect();

    assert_eq!(added, vec![&(152, "Boar".to_string())], "only Boar should be added");
    assert!(removed.is_empty(), "no pets should be removed: {removed:?}");
}

/// Non-sentinel members of a registered enum that have no Rust entry — the
/// "MISSING from Rust" set the audit computes. Mirrors the binary's logic.
fn missing_against_rust(dir: &Path, key: &str) -> Vec<(i64, String)> {
    let enums = parse::parse_dir(dir).expect("dump parses");
    let known = REGISTRY.iter().find(|k| k.key == key).unwrap();
    let fp = registry::rust_fingerprint(known);
    let (e, _) = registry::match_enum(known, &enums).expect("enum located");
    let mut missing: Vec<(i64, String)> = e
        .by_value()
        .iter()
        .filter(|(v, n)| !fp.contains_key(v) && !registry::is_sentinel(n))
        .map(|(v, n)| (*v, n.to_string()))
        .collect();
    missing.sort();
    missing
}

#[test]
fn complete_tables_with_high_ids_report_no_false_missing() {
    // Regression: these tables have real entries above the old per-enum scan
    // ceilings (equipment_type→311, gem_element→99, adventure_item→1000+), which
    // used to be excluded from the fingerprint and falsely flagged as missing.
    let new = dump("_cs_decomp_new");
    if !new.exists() {
        eprintln!("skipping: {} not present", new.display());
        return;
    }
    for key in ["equipment_type", "gem_element", "adventure_item"] {
        let missing = missing_against_rust(&new, key);
        assert!(
            missing.is_empty(),
            "{key} should be in sync, but tool reports missing: {missing:?}"
        );
    }
}

#[test]
fn monk_class_was_already_present_before_the_update() {
    // The class enum slot was reserved ahead of release, so an old↔new diff of
    // adventure_class must show no change — the tool must NOT flag Monk as new.
    let (old, new) = (dump("_cs_decomp"), dump("_cs_decomp_new"));
    if !old.exists() || !new.exists() {
        eprintln!("skipping: need both dumps");
        return;
    }
    let old_cls = matched_values(&old, "adventure_class").expect("classes in old");
    let new_cls = matched_values(&new, "adventure_class").expect("classes in new");

    assert!(
        old_cls.contains(&(30, "Monk".to_string())),
        "Monk = 30 should already exist in the pre-update dump"
    );
    let added: Vec<_> = new_cls.iter().filter(|m| !old_cls.contains(m)).collect();
    assert!(added.is_empty(), "no classes should be added this update: {added:?}");
}
