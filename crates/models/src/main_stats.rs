//! Parser for the game's **"Main stats"** clipboard export — a second export
//! (distinct from the semicolon-delimited *pet* export) of newline-separated
//! `Label: value` lines under blank-line-separated section headers. See
//! `reference/main_stats_export.md` for the full field catalogue.
//!
//! We extract only the values that currently have a home in the app (the
//! campaign inputs we auto-fill + the Moai-from-Museum inference). Everything
//! else is ignored; adding a field later is just another lookup. Parsing is
//! deliberately lenient: unknown/missing/garbled lines are skipped, leaving the
//! corresponding `Option` as `None`, so a partial export still fills what it can.

use std::collections::HashMap;

/// The signature on the export's first line, used to reject the wrong paste.
const HEADER: &str = "Idling to Rule the Gods";

/// Values lifted from a Main-stats export. Each is `None` if its line was absent
/// or unparseable. Only the subset the app can act on today is modelled.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MainStats {
    /// `Pet Stones` → Beachball's held-stones input.
    pub pet_stones: Option<u64>,
    /// `Ants` (the "found" count) → Ant Queen / Anteater.
    pub ants: Option<u64>,
    /// `Honey consumed by Bear` → Bear's honey input (NOT the bottom-of-list
    /// `Honey`, which is the held count for Bee).
    pub honey_consumed_by_bear: Option<u64>,
    /// `Challenge Points` → Unicorn.
    pub challenge_points: Option<u64>,
    /// `Ultimate Challenge Challenges` completed → Goblin's UCC campaign term.
    pub goblin_ucc: Option<u32>,
    /// `Overflow Challenges` completed → Goblin's OC evo-bonus term.
    pub goblin_oc: Option<u32>,
    /// `Ultimate Pet Challenges` completed → the chamber's UPC bonus (`5 ·` this).
    pub ultimate_pet_challenges: Option<u32>,
    /// `Patreon Gods Challenges` as `(done, max)` → the chamber's PGC toggle
    /// (all done = a global ×1.5 growth multiplier).
    pub patreon_god_challenges: Option<(u32, u32)>,
    /// `Day Pet Challenge highest multi` (a percent, e.g. `3.664 E+9`) → the food
    /// DPC boost `log2(multi)` (capped 100%).
    pub day_pet_challenge_multi: Option<f64>,
    /// `Fish Power` → the fishing food boost `FishPower^0.25 · milestone`.
    pub fish_power: Option<f64>,
    /// `Fishing Level` → the fishing milestones (+10% at 15, +10% at 27).
    pub fishing_level: Option<u32>,
    /// `Chp Stone Pet improvement` → Stone/Golem's +100% campaign upgrade.
    pub stone_campaign_upgrade: Option<bool>,
    /// `Earth Eater Earthlike planets eaten`, kept as the **raw value string**
    /// (e.g. `"7.142 E+6"`) so the UI's flexible-notation text field round-trips
    /// it; `parse_flexible_number` reads it back.
    pub earth_eater_planets_text: Option<String>,
    /// Museum `Base Growth per hour`. A value of exactly **2** uniquely means
    /// both Moai statues owned at level 20 (each maxed = +1/hr); other values
    /// can't be decomposed, so the caller should only act on `Some(2)`.
    pub base_growth_per_hour: Option<u64>,
}

/// Parse a Main-stats export. Errs only if the text isn't a Main-stats export at
/// all (missing header); a well-formed-but-sparse export parses to mostly-`None`.
pub fn parse_main_stats(source: &str) -> Result<MainStats, String> {
    if !source.trim_start().starts_with(HEADER) {
        return Err(format!("not a Main-stats export (missing the \"{HEADER}\" header)"));
    }

    // First `Label: value` per line; keep the first occurrence of each label.
    let mut map: HashMap<&str, &str> = HashMap::new();
    for line in source.lines() {
        if let Some((label, value)) = line.split_once(':') {
            map.entry(label.trim()).or_insert(value.trim());
        }
    }

    let count = |label: &str| map.get(label).copied().and_then(parse_count);
    // Challenge lines read "<done> / <max>"; we want the completed count.
    let challenge = |label: &str| {
        map.get(label)
            .copied()
            .and_then(|v| parse_count(v.split('/').next()?.trim()))
    };
    // Both sides of a "<done> / <max>" challenge line, for done-vs-max checks.
    let challenge_pair = |label: &str| -> Option<(u32, u32)> {
        let (done, max) = map.get(label)?.split_once('/')?;
        Some((parse_count(done.trim())? as u32, parse_count(max.trim())? as u32))
    };

    Ok(MainStats {
        pet_stones: count("Pet Stones"),
        ants: count("Ants"),
        honey_consumed_by_bear: count("Honey consumed by Bear"),
        challenge_points: count("Challenge Points"),
        goblin_ucc: challenge("Ultimate Challenge Challenges").map(|v| v as u32),
        goblin_oc: challenge("Overflow Challenges").map(|v| v as u32),
        ultimate_pet_challenges: challenge("Ultimate Pet Challenges").map(|v| v as u32),
        patreon_god_challenges: challenge_pair("Patreon Gods Challenges"),
        day_pet_challenge_multi: map
            .get("Day Pet Challenge highest multi")
            .copied()
            .and_then(crate::parse_flexible_number),
        fish_power: map.get("Fish Power").copied().and_then(crate::parse_flexible_number),
        fishing_level: count("Fishing Level").map(|v| v as u32),
        stone_campaign_upgrade: map
            .get("Chp Stone Pet improvement")
            .map(|v| v.eq_ignore_ascii_case("true")),
        earth_eater_planets_text: map
            .get("Earth Eater Earthlike planets eaten")
            .map(|v| v.to_string()),
        base_growth_per_hour: count("Base Growth per hour"),
    })
}

/// Parse an unsigned integer that may be comma-grouped / space-padded. Returns
/// `None` for non-integers (e.g. `"2.5"`), so callers never misread a decimal.
fn parse_count(s: &str) -> Option<u64> {
    let cleaned: String = s.chars().filter(|c| *c != ',' && !c.is_whitespace()).collect();
    cleaned.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_flexible_number;

    const SAMPLE: &str = include_str!("../../../reference/Main Stats Export.txt");

    #[test]
    fn parses_the_real_export() {
        let ms = parse_main_stats(SAMPLE).expect("should parse the sample export");
        assert_eq!(ms.pet_stones, Some(250_882));
        assert_eq!(ms.ants, Some(187_331));
        assert_eq!(ms.honey_consumed_by_bear, Some(0));
        assert_eq!(ms.challenge_points, Some(721));
        assert_eq!(ms.goblin_ucc, Some(0)); // "0 / 67"
        assert_eq!(ms.goblin_oc, Some(0)); // "0 / 9,999"
        assert_eq!(ms.ultimate_pet_challenges, Some(8)); // "8 / 20" → UPC 40%
        assert_eq!(ms.patreon_god_challenges, Some((0, 25))); // "0 / 25" — not complete
        assert_eq!(ms.day_pet_challenge_multi, Some(3.664e9)); // "3.664 E+9"
        assert_eq!(ms.fish_power, Some(1.050e6)); // "1.050 E+6"
        assert_eq!(ms.fishing_level, Some(14));
        assert_eq!(ms.stone_campaign_upgrade, Some(false)); // "False"
        assert_eq!(ms.base_growth_per_hour, Some(2)); // both Moai, L20
        // Earth Eater is kept raw and must round-trip through the flexible parser.
        assert_eq!(ms.earth_eater_planets_text.as_deref(), Some("7.142 E+6"));
        let parsed = parse_flexible_number(ms.earth_eater_planets_text.as_ref().unwrap());
        assert_eq!(parsed, Some(7_142_000.0));
    }

    #[test]
    fn rejects_a_non_main_stats_paste() {
        assert!(parse_main_stats("Name;Element;Growth;...").is_err());
        assert!(parse_main_stats("").is_err());
    }

    #[test]
    fn sparse_export_parses_to_none() {
        // Header present but no recognised lines → all None, no error.
        let ms = parse_main_stats("Idling to Rule the Gods - statistics export\n\nNothing: here\n")
            .unwrap();
        assert_eq!(ms, MainStats::default());
    }

    #[test]
    fn count_helper_rejects_decimals_and_handles_grouping() {
        assert_eq!(parse_count("250,882"), Some(250_882));
        assert_eq!(parse_count(" 2 "), Some(2));
        assert_eq!(parse_count("2.5"), None);
        assert_eq!(parse_count("abc"), None);
    }
}
