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
//!
//! Ancient Mimics are the Depth 4 version (lured by a Not Nothing). The wiki
//! does give their stats, so on top of the same offset rule the estimate is
//! capped by their defense-ignoring frozen aura against the team's weakest
//! pet ([`aura_check`]); a token-improved Cat negates the aura.

use itrtg_models::dungeon::DepthRequirements;
use itrtg_models::{Dungeon, ExportPet};

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

/// Which mimic a depth spawns: plain mimics in D3 (lured by a Nothing),
/// Ancient Mimics in D4 (lured by a Not Nothing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MimicKind {
    Mimic,
    Ancient,
}

impl MimicKind {
    pub fn for_depth(depth: u8) -> Option<Self> {
        match depth {
            MIMIC_DEPTH => Some(Self::Mimic),
            ANCIENT_MIMIC_DEPTH => Some(Self::Ancient),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Mimic => "Mimics",
            Self::Ancient => "Ancient Mimics",
        }
    }
    pub fn lure(self) -> &'static str {
        match self {
            Self::Mimic => "Nothing",
            Self::Ancient => "Not Nothing",
        }
    }
    /// Whether the enemy is Neutral, so negative elements hurt. Plain mimics
    /// always are; Ancient Mimics only in the Scrapyard (elsewhere they take a
    /// dungeon element and, per the wiki, have no elemental weakness).
    pub fn is_neutral_in(self, dungeon: Dungeon) -> bool {
        match self {
            Self::Mimic => true,
            Self::Ancient => dungeon == Dungeon::Scrapyard,
        }
    }
}

/// The only depth where Ancient Mimics spawn.
pub const ANCIENT_MIMIC_DEPTH: u8 = 4;
/// Ancient Mimic Attack at difficulty 0 (wiki: Dungeons#Ancient_Mimics).
pub const ANCIENT_MIMIC_BASE_ATTACK: f64 = 150_000.0;
/// Ancient Mimic stats grow 40% per difficulty level, compounding.
pub const ANCIENT_MIMIC_GROWTH: f64 = 1.4;
/// The frozen aura hits every pet at the start of each turn for this share of
/// the Ancient Mimic's Attack, ignoring defense.
pub const ANCIENT_MIMIC_AURA_SHARE: f64 = 0.2;
/// Planner heuristic, not a game rule: run Ancient Mimics no higher than the
/// difficulty where one aura tick is at most this share of the weakest pet's
/// max HP (about four turns of aura before healing is needed).
pub const AURA_HP_SHARE_LIMIT: f64 = 0.25;

/// Frozen-aura damage per turn at a difficulty, without event boosts (some D4
/// events raise it by half). Matches the wiki's table: 30,000 at 0, 115,248
/// at 4, 867,764 at 10.
pub fn ancient_mimic_aura(difficulty: u8) -> u64 {
    (ANCIENT_MIMIC_BASE_ATTACK * ANCIENT_MIMIC_AURA_SHARE * ANCIENT_MIMIC_GROWTH.powi(i32::from(difficulty)))
        .round() as u64
}

/// How the Ancient Mimic frozen aura limits a team.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuraCheck {
    /// Plain mimics have no aura.
    NotApplicable,
    /// A token-improved Cat in the team negates the aura.
    NegatedByCat,
    /// The highest difficulty whose aura stays within [`AURA_HP_SHARE_LIMIT`]
    /// of the weakest pet's max HP (`None` when not even difficulty 0 does).
    Limited { max_difficulty: Option<u8>, weakest: String, weakest_hp: i64 },
}

pub fn aura_check(kind: MimicKind, exports: &[&ExportPet]) -> AuraCheck {
    if kind == MimicKind::Mimic {
        return AuraCheck::NotApplicable;
    }
    if exports.iter().any(|e| e.improved && e.export_name.eq_ignore_ascii_case("Cat")) {
        return AuraCheck::NegatedByCat;
    }
    let Some(weakest) = exports.iter().min_by_key(|e| e.combat_stats.hp) else {
        return AuraCheck::Limited { max_difficulty: None, weakest: String::new(), weakest_hp: 0 };
    };
    let budget = weakest.combat_stats.hp as f64 * AURA_HP_SHARE_LIMIT;
    let max_difficulty = (0..=MAX_DIFFICULTY).rev().find(|d| ancient_mimic_aura(*d) as f64 <= budget);
    AuraCheck::Limited {
        max_difficulty,
        weakest: weakest.export_name.clone(),
        weakest_hp: weakest.combat_stats.hp,
    }
}

/// The difficulty a team should run its depth's mimics at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MimicEstimate {
    pub kind: MimicKind,
    /// Headroom minus the offset, before the aura limit.
    pub by_level: Option<u8>,
    pub aura: AuraCheck,
    /// The recommendation: `by_level`, lowered to the aura limit if any.
    /// `None` when the team can't take them even at difficulty 0.
    pub difficulty: Option<u8>,
}

pub fn mimic_estimate(kind: MimicKind, headroom: i64, offset: u8, exports: &[&ExportPet]) -> MimicEstimate {
    let by_level = mimic_difficulty(headroom, offset);
    let aura = aura_check(kind, exports);
    let difficulty = match &aura {
        AuraCheck::Limited { max_difficulty, .. } => by_level.zip(*max_difficulty).map(|(a, b)| a.min(b)),
        _ => by_level,
    };
    MimicEstimate { kind, by_level, aura, difficulty }
}

/// A team's mimic outlook, for display.
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
    pub fn new(estimate: &MimicEstimate, enabled: bool) -> Self {
        match (enabled, estimate.difficulty) {
            (true, Some(difficulty)) => Self::Enabled { difficulty },
            (true, None) => Self::TooWeak,
            (false, Some(difficulty)) if difficulty >= MIN_MIMIC_DIFFICULTY => Self::Ready { difficulty },
            (false, _) => Self::NotYet,
        }
    }
}

/// Everything the UIs show about a plan's mimics at its own depth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MimicOutlook {
    pub kind: MimicKind,
    /// The depth's recommended difficulty without mimics.
    pub without: u8,
    pub estimate: MimicEstimate,
    pub status: MimicStatus,
}

/// The mimic outlook for a D3 or D4 plan (`reqs` = that depth's
/// requirements); `None` for shallower plans or an empty team. `enabled`
/// is the dungeon's toggle and `offset` the kind's offset.
pub fn mimic_outlook(plan: &DungeonPlan, reqs: &DepthRequirements, enabled: bool, offset: u8) -> Option<MimicOutlook> {
    let kind = MimicKind::for_depth(plan.depth)?;
    let exports = team_exports(plan);
    if exports.is_empty() {
        return None;
    }
    let depth = depth_difficulty(&TeamLevels::from_exports(&exports), plan.depth, reqs);
    let estimate = mimic_estimate(kind, depth.headroom, offset, &exports);
    let status = MimicStatus::new(&estimate, enabled);
    Some(MimicOutlook { kind, without: depth.difficulty, estimate, status })
}

/// One-line explanation of how a kind's difficulty is derived.
pub fn mimic_rule_text(kind: MimicKind, offset: u8) -> String {
    let levels = format!("{offset} difficulty level{}", if offset == 1 { "" } else { "s" });
    match kind {
        MimicKind::Mimic => format!(
            "Mimics (D3 only, one Nothing used per encounter) are Neutral elites; the planner runs them {levels} \
             below what your team's dungeon level affords, counting levels beyond 10."
        ),
        MimicKind::Ancient => format!(
            "Ancient Mimics (D4 only, one Not Nothing used per encounter) have huge HP and a frozen aura that hits \
             every pet each turn for 20% of their Attack, ignoring defense: {} at difficulty 0, +40% per level, \
             and half again during some D4 events. The planner runs them {levels} below what your team's dungeon \
             level affords (counting levels beyond 10), and no higher than where one aura tick is a quarter of \
             your weakest pet's max HP. A token-improved Cat negates the aura.",
            ancient_mimic_aura(0)
        ),
    }
}

/// Short note on the aura limit, for display next to an Ancient Mimic estimate.
pub fn aura_note(estimate: &MimicEstimate) -> Option<String> {
    match &estimate.aura {
        AuraCheck::NotApplicable => None,
        AuraCheck::NegatedByCat => Some("Frozen aura negated by improved Cat.".into()),
        AuraCheck::Limited { max_difficulty, weakest, weakest_hp } => Some(match max_difficulty {
            Some(d) => format!(
                "Frozen aura limits this team to difficulty {d} ({} per turn vs. {weakest}'s {weakest_hp} max HP).",
                ancient_mimic_aura(*d)
            ),
            None => format!(
                "Frozen aura ({} per turn at difficulty 0) is over a quarter of {weakest}'s {weakest_hp} max HP.",
                ancient_mimic_aura(0)
            ),
        }),
    }
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

    fn est(kind: MimicKind, headroom: i64, exports: &[&ExportPet]) -> MimicEstimate {
        mimic_estimate(kind, headroom, 2, exports)
    }

    #[test]
    fn status_reflects_toggle_and_strength() {
        let p = pet("A", 1, 1);
        let t = [&p];
        assert_eq!(MimicStatus::new(&est(MimicKind::Mimic, 9, &t), true), MimicStatus::Enabled { difficulty: 7 });
        assert_eq!(MimicStatus::new(&est(MimicKind::Mimic, 1, &t), true), MimicStatus::TooWeak);
        assert_eq!(MimicStatus::new(&est(MimicKind::Mimic, 4, &t), false), MimicStatus::Ready { difficulty: 2 });
        // Difficulty 0 gives no points, so it isn't "ready".
        assert_eq!(MimicStatus::new(&est(MimicKind::Mimic, 2, &t), false), MimicStatus::NotYet);
    }

    #[test]
    fn kinds_by_depth() {
        assert_eq!(MimicKind::for_depth(2), None);
        assert_eq!(MimicKind::for_depth(3), Some(MimicKind::Mimic));
        assert_eq!(MimicKind::for_depth(4), Some(MimicKind::Ancient));
        assert!(MimicKind::Ancient.is_neutral_in(Dungeon::Scrapyard));
        assert!(!MimicKind::Ancient.is_neutral_in(Dungeon::Forest));
    }

    #[test]
    fn aura_matches_the_wiki_table() {
        assert_eq!(ancient_mimic_aura(0), 30_000);
        assert_eq!(ancient_mimic_aura(1), 42_000);
        assert_eq!(ancient_mimic_aura(4), 115_248);
        assert_eq!(ancient_mimic_aura(7), 316_241);
        assert_eq!(ancient_mimic_aura(10), 867_764);
    }

    fn with_hp(name: &str, hp: i64) -> ExportPet {
        let mut p = pet(name, 1, 1);
        p.combat_stats.hp = hp;
        p
    }

    #[test]
    fn aura_caps_ancient_mimics_by_the_weakest_pet() {
        // Budget = 25% of 500K = 125K: difficulty 4 (115,248) fits, 5 doesn't.
        let (tank, weak) = (with_hp("Tank", 5_000_000), with_hp("Glass", 500_000));
        let e = est(MimicKind::Ancient, 12, &[&tank, &weak]);
        assert_eq!(e.by_level, Some(10));
        assert_eq!(e.difficulty, Some(4));
        assert_eq!(
            e.aura,
            AuraCheck::Limited { max_difficulty: Some(4), weakest: "Glass".into(), weakest_hp: 500_000 }
        );
        // The level rule still wins when it's lower.
        assert_eq!(est(MimicKind::Ancient, 5, &[&tank, &weak]).difficulty, Some(3));
        // Too fragile for even difficulty 0 (30K > 25% of 100K).
        let paper = with_hp("Paper", 100_000);
        assert_eq!(est(MimicKind::Ancient, 12, &[&paper]).difficulty, None);
        // Plain mimics have no aura.
        assert_eq!(est(MimicKind::Mimic, 12, &[&paper]).difficulty, Some(10));
    }

    #[test]
    fn improved_cat_negates_the_aura() {
        let mut cat = with_hp("Cat", 100_000);
        let paper = with_hp("Paper", 100_000);
        assert_eq!(est(MimicKind::Ancient, 12, &[&cat, &paper]).difficulty, None);
        cat.improved = true;
        let e = est(MimicKind::Ancient, 12, &[&cat, &paper]);
        assert_eq!((e.aura, e.difficulty), (AuraCheck::NegatedByCat, Some(10)));
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
