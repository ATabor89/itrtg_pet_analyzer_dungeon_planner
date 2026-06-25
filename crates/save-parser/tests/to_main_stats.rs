//! Tests for the `SaveFile` → `MainStats` converter, cross-checked against the
//! same-session Main Stats export paired with the `2026-06-09` reference save.
//! Skips silently if the fixtures aren't present.
//!
//! Integer counters are asserted equal to the export. The scientific-notation
//! fields (Fish Power, Day-Pet multi, Earth-Eater planets) are display-rounded
//! in the export but exact in the save, so those are checked against the precise
//! save value rather than the rounded export.

use itrtg_models::{parse_flexible_number, parse_main_stats};
use save_parser::{moai_levels, save_to_main_stats};

const SAVE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../reference/save_file_deserialization/ManualSave_2026-06-09.txt"
);
const EXPORT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../reference/save_file_deserialization/Main Stats Export.txt"
);

fn load() -> Option<(save_parser::SaveFile, itrtg_models::MainStats)> {
    let raw = std::fs::read_to_string(SAVE_PATH).ok()?;
    let export = std::fs::read_to_string(EXPORT_PATH).ok()?;
    let save = save_parser::parse_save(&raw).expect("reference save parses");
    let ms = parse_main_stats(&export).expect("paired export parses");
    Some((save, ms))
}

macro_rules! require_pair {
    () => {
        match load() {
            Some(pair) => pair,
            None => {
                eprintln!("reference save/export pair not present; skipping");
                return;
            }
        }
    };
}

/// The integer counters the converter fills must match the paired export exactly.
#[test]
fn integer_fields_match_the_paired_export() {
    let (save, export) = require_pair!();
    let got = save_to_main_stats(&save);

    assert_eq!(got.pet_stones, export.pet_stones, "pet_stones"); // 267,028
    assert_eq!(got.ants, export.ants, "ants"); // 192,164
    assert_eq!(
        got.honey_consumed_by_bear, export.honey_consumed_by_bear,
        "honey_consumed_by_bear" // 0
    );
    assert_eq!(got.goblin_ucc, export.goblin_ucc, "goblin_ucc"); // 0 / 67
    assert_eq!(got.goblin_oc, export.goblin_oc, "goblin_oc"); // 0 / 9,999
    assert_eq!(
        got.ultimate_pet_challenges, export.ultimate_pet_challenges,
        "ultimate_pet_challenges" // 8 / 20
    );
    assert_eq!(
        got.patreon_god_challenges, export.patreon_god_challenges,
        "patreon_god_challenges" // 0 / 25
    );
    assert_eq!(got.fishing_level, export.fishing_level, "fishing_level"); // 14
    assert_eq!(
        got.stone_campaign_upgrade, export.stone_campaign_upgrade,
        "stone_campaign_upgrade" // False
    );
}

/// Scientific-notation fields are exact in the save (more precise than the
/// display-rounded export), so check the precise save value.
#[test]
fn precise_fields_use_the_exact_save_value() {
    let (save, _export) = require_pair!();
    let got = save_to_main_stats(&save);

    // Fish Power: export "1.227 E+6"; save exact 1,227,264.25.
    let fish = got.fish_power.expect("fish_power present");
    assert!((fish - 1_227_264.25).abs() < 1.0, "fish_power = {fish}");

    // Day Pet multi: export "3.664 E+9"; save exact 3,664,035,884.
    let dpc = got.day_pet_challenge_multi.expect("day_pet_challenge_multi present");
    assert!((dpc - 3_664_035_884.0).abs() < 1.0, "day_pet_multi = {dpc}");

    // Earth Eater planets: export "7.308 E+6"; save exact 7,308,846. Kept as a
    // string that the flexible parser round-trips.
    let text = got.earth_eater_planets_text.expect("earth_eater present");
    assert_eq!(parse_flexible_number(&text), Some(7_308_846.0), "earth_eater = {text:?}");
}

/// Moai are read as exact Museum-statue levels: this account owns two at L20
/// (matching the export's "Base Growth per hour: 2").
#[test]
fn moai_levels_are_exact() {
    let (save, _export) = require_pair!();
    assert_eq!(moai_levels(&save), vec![20, 20]);
}

/// Total Challenge Points is derived (non-Day completions + capped Day-score
/// formulas) and must match the export's "Challenge Points" total earned.
#[test]
fn challenge_points_match_the_export_total() {
    let (save, export) = require_pair!();
    let got = save_to_main_stats(&save);
    // Export: "Challenge Points: 751" (539 flat-rate + ~212 from the Day Pet
    // score). Confirms the export reports total *earned*, not the spendable
    // balance, and that our Day-challenge formulas reproduce it.
    assert_eq!(got.challenge_points, export.challenge_points);
    assert_eq!(got.challenge_points, Some(751));
}

/// `base_growth_per_hour` stays `None` (handled via exact `moai_levels`) so
/// selective-fill never clobbers a prior/manual value.
#[test]
fn base_growth_per_hour_is_none() {
    let (save, _export) = require_pair!();
    assert_eq!(save_to_main_stats(&save).base_growth_per_hour, None);
}
