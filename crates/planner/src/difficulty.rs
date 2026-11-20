//! Dungeon difficulty estimates shared by the front ends, including the
//! Depth 3 mimic adjustment.
//!
//! A depth's difficulty is the team's average dungeon level above the depth's
//! requirement, divided by the depth's (conservative) levels-per-difficulty.
//! The in-game setting stops at 10, but the uncapped value ("headroom") is kept
//! because it is what makes mimic planning dynamic: a team that could
//! nominally run difficulty 12 still has room for mimics at 10.
//!
//! Mimics (wiki: Dungeons#Mimics) are a Neutral elite that only spawns in
//! Depth 3 rooms (10% per room) when the party carries a Nothing item; each
//! encounter consumes one Nothing. A kill gives one Mimic Point per difficulty
//! level, so difficulty 0 earns no points. The wiki gives no stat table for
//! them, only the rule of thumb to drop "a couple" of difficulty levels, so
//! the mimic difficulty is the headroom minus a user-tunable offset
//! ([`DEFAULT_MIMIC_OFFSET`]). Because Neutral enemies punish negative element
//! values, [`neutral_weak_pets`] lists the team members the wiki would flag.

use itrtg_models::dungeon::DepthRequirements;
use itrtg_models::ExportPet;

use crate::solver::{Assignment, DungeonPlan};

/// Highest in-game difficulty setting.
pub const MAX_DIFFICULTY: u8 = 10;
/// The only depth where (non-ancient) mimics spawn.
pub const MIMIC_DEPTH: u8 = 3;
/// Difficulty levels to drop when mimics are enabled, unless the user
/// changes it.
pub const DEFAULT_MIMIC_OFFSET: u8 = 2;
/// Largest offset the settings accept.
pub const MAX_MIMIC_OFFSET: u8 = 6;
/// Mimics are worth farming from this difficulty up (difficulty 0 gives no
/// Mimic Points).
pub const MIN_MIMIC_DIFFICULTY: u8 = 1;

/// Team numbers the difficulty estimate uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamLevels {
    pub filled: usize,
    /// Average dungeon level over all six slots (empty slots count as 0).
    pub avg_dungeon_level: u64,
    pub min_class_level: u32,
    pub min_growth: u64,
}

impl TeamLevels {
    pub fn from_exports(exports: &[&ExportPet]) -> Self {
        Self {
            filled: exports.len(),
            avg_dungeon_level: exports.iter().map(|e| u64::from(e.dungeon_level)).sum::<u64>() / 6,
            min_class_level: exports.iter().map(|e| e.class_level).min().unwrap_or(0),
            min_growth: exports.iter().map(|e| e.growth).min().unwrap_or(0),
        }
    }
}

/// The export data of every filled slot in a plan.
pub fn team_exports(plan: &DungeonPlan) -> Vec<&ExportPet> {
    plan.assignments
        .iter()
        .filter_map(|sa| match &sa.assignment {
            Assignment::Filled { pet, .. } => pet.export.as_ref(),
            _ => None,
        })
        .collect()
}

/// Difficulty estimate for one depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DepthDifficulty {
    pub depth: u8,
    /// Uncapped difficulty levels the team's dungeon level affords. Negative
    /// when the team is below the depth's level requirement.
    pub headroom: i64,
    /// The recommended setting without mimics: headroom clamped to 0–10.
    pub difficulty: u8,
    pub class_level_ok: bool,
    pub growth_ok: bool,
}

pub fn depth_difficulty(team: &TeamLevels, depth: u8, reqs: &DepthRequirements) -> DepthDifficulty {
    // The higher (conservative) levels-per-difficulty value, as before.
    let per = i64::from(reqs.levels_per_difficulty.last().copied().unwrap_or(5).max(1));
    let over = team.avg_dungeon_level as i64 - i64::from(reqs.dungeon_level_avg);
    let headroom = over.div_euclid(per);
    DepthDifficulty {
        depth,
        headroom,
        difficulty: clamp_difficulty(headroom),
        class_level_ok: team.min_class_level >= reqs.class_level,
        growth_ok: reqs.total_growth.is_none_or(|g| team.min_growth >= g),
    }
}

fn clamp_difficulty(v: i64) -> u8 {
    v.clamp(0, i64::from(MAX_DIFFICULTY)) as u8
}

/// Difficulty to run with mimics: the headroom minus `offset`, capped at 10.
/// `None` when the team can't take mimics even at difficulty 0.
pub fn mimic_difficulty(headroom: i64, offset: u8) -> Option<u8> {
    let v = headroom - i64::from(offset);
    (v >= 0).then(|| clamp_difficulty(v))
}

/// Whether mimics would be worth turning on: viable at difficulty 1 or more.
pub fn mimics_ready(headroom: i64, offset: u8) -> bool {
    mimic_difficulty(headroom, offset).is_some_and(|d| d >= MIN_MIMIC_DIFFICULTY)
}

/// Names (export names) of team members with a Water/Fire/Wind/Earth value of
/// 0 or below in their current gear. The wiki calls all elements above 0
/// "almost necessary" against Neutral enemies like mimics.
pub fn neutral_weak_pets<'a>(exports: &[&'a ExportPet]) -> Vec<&'a str> {
    exports
        .iter()
        .filter(|e| {
            let a = &e.elemental_affinities;
            [a.water, a.fire, a.wind, a.earth].iter().any(|v| *v <= 0)
        })
        .map(|e| e.export_name.as_str())
        .collect()
}

/// The D3 mimic outlook for a team, for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MimicStatus {
    /// Mimics are on and the team can run them at this difficulty.
    Enabled { difficulty: u8 },
    /// Mimics are on but the team is too weak even at difficulty 0.
    TooWeak,
    /// Mimics are off and the team could run them at difficulty 1 or more.
    Ready { difficulty: u8 },
    /// Mimics are off and not yet worth it.
    NotYet,
}

impl MimicStatus {
    pub fn new(d3: &DepthDifficulty, enabled: bool, offset: u8) -> Self {
        match (enabled, mimic_difficulty(d3.headroom, offset)) {
            (true, Some(difficulty)) => Self::Enabled { difficulty },
            (true, None) => Self::TooWeak,
            (false, Some(difficulty)) if difficulty >= MIN_MIMIC_DIFFICULTY => Self::Ready { difficulty },
            (false, _) => Self::NotYet,
        }
    }
}

/// One-line explanation of how the mimic difficulty is derived.
pub fn mimic_rule_text(offset: u8) -> String {
    format!(
        "Mimics (D3 only, needs a Nothing item per encounter) are Neutral elites; the planner runs them {offset} \
         difficulty level{} below what your team's dungeon level affords, counting levels beyond 10.",
        if offset == 1 { "" } else { "s" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use itrtg_models::{CombatStats, ElementalAffinities};
    use itrtg_models::{Element, Loadout, PetAction};

    fn reqs(avg: u32, per: &[u32]) -> DepthRequirements {
        DepthRequirements {
            dungeon_level_avg: avg,
            levels_per_difficulty: per.to_vec(),
            class_level: 25,
            total_growth: None,
        }
    }

    fn team(avg: u64) -> TeamLevels {
        TeamLevels { filled: 6, avg_dungeon_level: avg, min_class_level: 30, min_growth: 0 }
    }

    fn pet(name: &str, dl: u32, water: i64) -> ExportPet {
        ExportPet {
            export_name: name.into(),
            element: Element::Fire,
            growth: 1000,
            dungeon_level: dl,
            class: None,
            class_level: 30,
            class_exp: 0.0,
            combat_stats: CombatStats { hp: 1, attack: 1, defense: 1, speed: 1 },
            elemental_affinities: ElementalAffinities { water, fire: 10, wind: 0, earth: 0, dark: -5, light: 0 },
            loadout: Loadout { weapon: None, armor: None, accessory: None },
            action: PetAction::Idle,
            unlocked: true,
            improved: false,
            other: None,
            has_partner: false,
        }
    }

    #[test]
    fn difficulty_matches_the_old_capped_estimate() {
        // 60 levels over at 8 per difficulty: 7 levels.
        let d = depth_difficulty(&team(160), 3, &reqs(100, &[6, 8]));
        assert_eq!((d.headroom, d.difficulty), (7, 7));
        // Below the requirement: negative headroom, difficulty 0.
        let d = depth_difficulty(&team(90), 3, &reqs(100, &[6, 8]));
        assert_eq!((d.headroom, d.difficulty), (-2, 0));
        // Well above: capped at 10, headroom kept.
        let d = depth_difficulty(&team(220), 3, &reqs(100, &[6, 8]));
        assert_eq!((d.headroom, d.difficulty), (15, 10));
    }

    #[test]
    fn class_level_and_growth_checks() {
        let mut r = reqs(100, &[6, 8]);
        r.total_growth = Some(5000);
        let t = TeamLevels { filled: 6, avg_dungeon_level: 120, min_class_level: 20, min_growth: 4000 };
        let d = depth_difficulty(&t, 3, &r);
        assert!(!d.class_level_ok && !d.growth_ok);
    }

    #[test]
    fn mimic_difficulty_drops_the_offset_from_uncapped_headroom() {
        // Aaron's usual case: recommended 8 → mimics at 6.
        assert_eq!(mimic_difficulty(8, 2), Some(6));
        // A stronger team whose headroom passes the cap only drops one level
        // below the capped recommendation (11 → 9 vs. shown 10).
        assert_eq!(mimic_difficulty(11, 2), Some(9));
        assert_eq!(mimic_difficulty(14, 2), Some(10));
        // Too weak for mimics at all.
        assert_eq!(mimic_difficulty(1, 2), None);
        assert_eq!(mimic_difficulty(2, 2), Some(0));
    }

    #[test]
    fn ready_means_difficulty_one_or_more() {
        assert!(!mimics_ready(2, 2));
        assert!(mimics_ready(3, 2));
        assert!(mimics_ready(1, 0));
        assert!(!mimics_ready(-4, 0));
    }

    #[test]
    fn status_reflects_toggle_and_strength() {
        let d3 = |headroom| DepthDifficulty { depth: 3, headroom, difficulty: clamp_difficulty(headroom), class_level_ok: true, growth_ok: true };
        assert_eq!(MimicStatus::new(&d3(9), true, 2), MimicStatus::Enabled { difficulty: 7 });
        assert_eq!(MimicStatus::new(&d3(1), true, 2), MimicStatus::TooWeak);
        assert_eq!(MimicStatus::new(&d3(4), false, 2), MimicStatus::Ready { difficulty: 2 });
        assert_eq!(MimicStatus::new(&d3(2), false, 2), MimicStatus::NotYet);
    }

    #[test]
    fn team_levels_count_empty_slots_as_zero() {
        let (a, b) = (pet("A", 60, 0), pet("B", 60, 0));
        let t = TeamLevels::from_exports(&[&a, &b]);
        assert_eq!((t.filled, t.avg_dungeon_level), (2, 20));
    }

    #[test]
    fn neutral_weakness_flags_zero_or_below_and_ignores_dark_and_light() {
        let mut ok = pet("Ok", 1, 4);
        ok.elemental_affinities.wind = 2;
        ok.elemental_affinities.earth = 3;
        let mut zero = ok.clone();
        zero.export_name = "Zero".into();
        zero.elemental_affinities.water = 0;
        let mut weak = ok.clone();
        weak.export_name = "Weak".into();
        weak.elemental_affinities.water = -3;
        // `ok` has dark -5, which doesn't count.
        assert_eq!(neutral_weak_pets(&[&ok, &zero, &weak]), vec!["Zero", "Weak"]);
    }
}
