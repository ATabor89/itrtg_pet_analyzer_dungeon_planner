//! Campaign reward simulation — the shared shape behind every pet campaign.
//!
//! All campaigns look the same: slot **1–10 pets** for an integer **1–12 hour**
//! run, multiply by the global **UPC** bonus and each pet's **per-campaign**
//! bonus, and sum a per-pet term. Only three things differ between campaigns —
//! the **per-pet formula**, **how pets interact** with it (e.g. Growth's weakest
//! pet is the recipient, not a contributor), and the **output**. This module
//! captures that common pipeline; [`simulate`] dispatches the per-kind formula.
//!
//! Several campaigns key off **pet stats**, which depend on a pet's *normal*
//! level — not in the export yet (see `reference/campaign_simulation.md` §1–2).
//! Those formulas are implemented but **gated**: they return
//! [`CampaignOutcome::NeedsStats`] until every pet has a `stats` value, so they
//! light up automatically once a stats source lands. Stat-independent campaigns
//! (Growth, the rebirth half of Multiplier) compute today.
//!
//! Not yet modelled (the documented next layers): the **special-pet** effects
//! (Pandora's Box flat bonus, Bag's steal/gift, Nightmare/Ant Queen GP uncap),
//! the per-pet **food tiers** Food needs, and the UI / data wiring. The
//! growth-**chamber** loop ([`simulate_growth_chamber`]) is included as the first
//! consumer of the abstraction.

use itrtg_models::CampaignType;

/// One pet's inputs to a campaign formula, extracted from the roster and the
/// per-campaign multiplier we already compute (`MergedPet::campaign_bonus_for`).
#[derive(Debug, Clone)]
pub struct CampaignPet {
    pub name: String,
    pub growth: u64,
    /// Total pet stats (Physical + Mystic + Battle). `None` until normal level /
    /// stats can be read — stat-dependent campaigns return `NeedsStats` then.
    pub stats: Option<f64>,
    /// This pet's bonus to *this* campaign, as a percent (e.g. `+82.0`); applied
    /// as the multiplier `(1 + pct/100)`.
    pub campaign_bonus_pct: f32,
    /// Pendant + Moai growth **per hour** for this pet (0 if none). It accrues
    /// *during* the run, and the game computes the campaign from growth **at
    /// completion** — so for the Growth campaign this can flip which pet is the
    /// (lowest-growth) recipient. Moai are equal for everyone and don't change
    /// the order; a pendant on the lowest pet can.
    pub passive_per_hour: f64,
}

/// Campaign-agnostic parameters shared by every formula.
#[derive(Debug, Clone)]
pub struct CampaignParams {
    /// Ultimate Pet Challenge bonus as a percent (`5 · UPC`, capped 100).
    pub upc_pct: f64,
    /// Campaign length in hours (clamped to 1..=12).
    pub hours: u32,
    /// Total pets the player has unlocked — Food scales with this.
    pub unlocked_pets: usize,
    /// Divinity-gained-per-second (a user input); `None` blocks the Divinity
    /// campaign on missing data rather than stats.
    pub div_per_sec: Option<f64>,
}

/// The result of one campaign run.
#[derive(Debug, Clone, PartialEq)]
pub enum CampaignOutcome {
    /// A per-run numeric reward (food chance, divinity, levels, stones, GP, or
    /// the rebirth-multiplier).
    Reward(f64),
    /// Growth campaign: the total growth produced and the index of the recipient
    /// (the lowest-growth pet, which receives it and does not contribute).
    Growth { total: f64, recipient: usize },
    /// Blocked until pet stats are readable (the normal-level export gap).
    NeedsStats,
    /// Blocked until some curated data exists (the reason names it).
    NeedsData(&'static str),
}

/// Run a single campaign for a team. The team should already be capped at 10 and
/// `params.hours` clamped to 1..=12 by the caller; this is robust to either.
pub fn simulate(kind: CampaignType, team: &[CampaignPet], params: &CampaignParams) -> CampaignOutcome {
    if team.is_empty() {
        return CampaignOutcome::Reward(0.0);
    }
    match kind {
        CampaignType::Growth => growth_campaign(team, params),
        // Food needs a per-pet mighty/strong/puny classification we don't curate
        // yet; everything else it needs (unlocked count, multipliers) is here.
        CampaignType::Food => CampaignOutcome::NeedsData("per-pet food tiers (mighty/strong/puny)"),
        // Only the rebirth-multiplier half is stat-independent; the pet-multiplier
        // half awaits stats (documented in `simulate_multiplier`).
        CampaignType::Multiplier => CampaignOutcome::Reward(rebirth_multiplier(team, params)),
        CampaignType::Divinity => divinity_campaign(team, params),
        CampaignType::Item => stat_term_sum(team, params, |_g, s| item_tier(s)),
        CampaignType::Level => stat_term_sum(team, params, |g, s| {
            (log_base(g, 10.0).powi(3) * 1.9 + s.powf(0.22) * 3.0).max(0.0)
        }),
        CampaignType::GodPower => god_power_campaign(team, params),
    }
}

/// The shared multiplier every per-pet term is scaled by: the global UPC bonus,
/// this pet's per-campaign bonus, and the run length.
fn pet_factor(pet: &CampaignPet, p: &CampaignParams) -> f64 {
    let hours = p.hours.clamp(1, 12) as f64;
    (1.0 + p.upc_pct / 100.0) * (1.0 + pet.campaign_bonus_pct as f64 / 100.0) * hours
}

fn log_base(x: f64, base: f64) -> f64 {
    x.ln() / base.ln()
}

fn all_have_stats(team: &[CampaignPet]) -> bool {
    team.iter().all(|p| p.stats.is_some())
}

/// Growth: each pet *except the weakest* contributes `(log15(growth) − 1.75)`,
/// and the sum raises the weakest pet's growth.
///
/// Growth is measured **at completion**: pendant + Moai (`passive_per_hour`)
/// accrue over the run first, then the lowest-growth pet is the recipient and
/// the rest contribute from their end-of-run growth. A pendant on the
/// start-of-run lowest pet can lift it past a neighbour, making *it* a
/// contributor and the neighbour the recipient — a real edge case when rushing a
/// new pet up toward the chamber. This returns `{total, recipient}` only; the
/// caller is responsible for depositing `total` into the recipient and applying
/// each pet's passive growth (the chamber does both).
fn growth_campaign(team: &[CampaignPet], p: &CampaignParams) -> CampaignOutcome {
    let hours = p.hours.clamp(1, 12) as f64;
    let end_growth = |pet: &CampaignPet| pet.growth as f64 + pet.passive_per_hour * hours;
    let recipient = (0..team.len())
        .min_by(|&a, &b| {
            end_growth(&team[a])
                .partial_cmp(&end_growth(&team[b]))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("non-empty team");
    let total: f64 = team
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != recipient)
        .map(|(_, pet)| (log_base(end_growth(pet), 15.0) - 1.75).max(0.0) * pet_factor(pet, p))
        .sum();
    CampaignOutcome::Growth { total, recipient }
}

/// Divinity: `f(stats) · div_per_sec`, summed. Blocked on stats and on the
/// `div_per_sec` user input.
fn divinity_campaign(team: &[CampaignPet], p: &CampaignParams) -> CampaignOutcome {
    let Some(dps) = p.div_per_sec else {
        return CampaignOutcome::NeedsData("divinity gained per second");
    };
    if !all_have_stats(team) {
        return CampaignOutcome::NeedsStats;
    }
    let total = team
        .iter()
        .map(|pet| {
            let s = pet.stats.unwrap();
            // The game leaves <10k stats unspecified; the low branch covers it.
            let f = if s < 1e7 { (s.powf(0.4) + 300.0) * 2.0 } else { s.powf(0.35) + 1580.0 };
            f * dps * pet_factor(pet, p)
        })
        .sum();
    CampaignOutcome::Reward(total)
}

/// Item / Level share the shape `Σ term(growth, stats) · factor`, blocked on stats.
fn stat_term_sum(
    team: &[CampaignPet],
    p: &CampaignParams,
    term: impl Fn(f64, f64) -> f64,
) -> CampaignOutcome {
    if !all_have_stats(team) {
        return CampaignOutcome::NeedsStats;
    }
    let total = team
        .iter()
        .map(|pet| term(pet.growth as f64, pet.stats.unwrap()) * pet_factor(pet, p))
        .sum();
    CampaignOutcome::Reward(total)
}

/// God Power: `Σ tier(stats) · factor`, then **FSM doubles the total**. Blocked
/// on stats. (Nightmare / evolved-Ant-Queen uncap the per-pet 1-GP cap; that cap
/// isn't modelled in this expected-value sum, so it's a no-op here — a TODO for
/// the special-pet layer.)
fn god_power_campaign(team: &[CampaignPet], p: &CampaignParams) -> CampaignOutcome {
    if !all_have_stats(team) {
        return CampaignOutcome::NeedsStats;
    }
    let mut total: f64 = team
        .iter()
        .map(|pet| gp_tier(pet.stats.unwrap()) * pet_factor(pet, p))
        .sum();
    if team.iter().any(|pet| pet.name == "FSM") {
        total *= 2.0;
    }
    CampaignOutcome::Reward(total)
}

/// The **rebirth-multiplier** half of the Multiplier campaign — `Σ (log2(growth)
/// − 7) · factor / 2`. Stat-independent, so it computes today. The headline
/// *pet-multiplier* half (`stats^0.3 · 0.016 + 1`, etc.) needs stats and is a
/// TODO; Multiplier's outcome is just this half for now.
fn rebirth_multiplier(team: &[CampaignPet], p: &CampaignParams) -> f64 {
    team.iter()
        .map(|pet| (log_base(pet.growth as f64, 2.0) - 7.0).max(0.0) * pet_factor(pet, p) / 2.0)
        .sum()
}

/// Item-campaign pet-stone tier: base 1, +1 past each of 10k/100k/500k/1m/5m/10m/
/// 50m stats, +2 past 100m, capped at 10.
fn item_tier(stats: f64) -> f64 {
    let mut tier: f64 = 1.0;
    for thresh in [1e4, 1e5, 5e5, 1e6, 5e6, 1e7, 5e7] {
        if stats >= thresh {
            tier += 1.0;
        }
    }
    if stats >= 1e8 {
        tier += 2.0;
    }
    tier.min(10.0)
}

/// God-Power-campaign find-chance tier: 3% base rising to 12% at 100m stats. The
/// exact "tiered linearity" curve is unconfirmed (`campaign_simulation.md` §8);
/// this is a linear-in-log placeholder between 0 and 100m.
fn gp_tier(stats: f64) -> f64 {
    if stats <= 1.0 {
        return 3.0;
    }
    let frac = (stats.log10() / 1e8_f64.log10()).clamp(0.0, 1.0);
    3.0 + 9.0 * frac
}

// =============================================================================
// Growth chamber
// =============================================================================

/// A pet locked into a growth chamber.
#[derive(Debug, Clone)]
pub struct ChamberPet {
    pub name: String,
    pub growth: f64,
    pub campaign_bonus_pct: f32,
    /// Pendant + Moai growth per hour for this pet (0 if none). One tick per
    /// campaign-hour (see `campaign_simulation.md` §7).
    pub passive_per_hour: f64,
    /// Stop tracking this pet once its growth reaches this target (e.g. an evolve
    /// threshold). `None` = an untracked resident.
    pub target: Option<f64>,
}

/// Per-cycle record for the chamber trace.
#[derive(Debug, Clone)]
pub struct ChamberCycle {
    pub recipient: usize,
    pub campaign_growth: f64,
}

/// Outcome of a chamber simulation.
#[derive(Debug, Clone)]
pub struct ChamberResult {
    pub cycles: u32,
    /// `(pet name, cycle index)` for each *targeted* pet that reached its target.
    pub reached: Vec<(String, u32)>,
    /// One entry per simulated cycle.
    pub trace: Vec<ChamberCycle>,
    /// Final growth per pet, in the input order.
    pub final_growth: Vec<(String, f64)>,
}

/// Simulate a growth chamber: repeatedly run the Growth campaign on the same
/// pets, depositing each run's total into the weakest pet (the recipient) and
/// ticking passive pendant/Moai growth into every pet, until all *targeted* pets
/// reach their target or `max_cycles` is hit.
///
/// This is the "pack the campaign into a loop" idea — it reuses [`simulate`] with
/// [`CampaignType::Growth`] so the chamber and the one-off campaign share a single
/// formula. (The Pandora's Box / Bag special-pet layer is a documented follow-up.)
pub fn simulate_growth_chamber(
    pets: &mut [ChamberPet],
    hours: u32,
    upc_pct: f64,
    max_cycles: u32,
) -> ChamberResult {
    let hours = hours.clamp(1, 12);
    let targeted = pets.iter().filter(|p| p.target.is_some()).count();
    let mut reached: Vec<(String, u32)> = Vec::new();
    // Per-pet "already reached" flags, keyed by index (not name — pets can share
    // a name, which would otherwise break the stop condition).
    let mut done = vec![false; pets.len()];
    let mut trace: Vec<ChamberCycle> = Vec::new();

    for cycle in 0..max_cycles {
        // Reuse the Growth campaign on the chamber's current state. Growth is an
        // integer in-game, so the f64 chamber value truncates to u64 here for the
        // log term — lossy by design, negligible at real growth magnitudes.
        let team: Vec<CampaignPet> = pets
            .iter()
            .map(|p| CampaignPet {
                name: p.name.clone(),
                growth: p.growth as u64,
                stats: None,
                campaign_bonus_pct: p.campaign_bonus_pct,
                passive_per_hour: p.passive_per_hour,
            })
            .collect();
        let params = CampaignParams { upc_pct, hours, unlocked_pets: pets.len(), div_per_sec: None };

        // `simulate` picks the recipient + contributions from end-of-run growth
        // (it factors in `passive_per_hour`); we then realise those growth
        // changes: passive into every pet, plus the campaign total into the
        // recipient. Recipient ends at start + passive·hours + total — consistent
        // with the end-of-run growth `simulate` used to choose it.
        let (total, recipient) = match simulate(CampaignType::Growth, &team, &params) {
            CampaignOutcome::Growth { total, recipient } => (total, recipient),
            _ => unreachable!("Growth campaign always yields a Growth outcome"),
        };

        for pet in pets.iter_mut() {
            pet.growth += pet.passive_per_hour * hours as f64;
        }
        pets[recipient].growth += total;
        trace.push(ChamberCycle { recipient, campaign_growth: total });

        // Record any targeted pet that crossed its target this cycle (by index).
        for i in 0..pets.len() {
            if !done[i]
                && let Some(t) = pets[i].target
                && pets[i].growth >= t
            {
                done[i] = true;
                reached.push((pets[i].name.clone(), cycle + 1));
            }
        }
        if targeted > 0 && reached.len() == targeted {
            return ChamberResult {
                cycles: cycle + 1,
                reached,
                trace,
                final_growth: pets.iter().map(|p| (p.name.clone(), p.growth)).collect(),
            };
        }
    }

    ChamberResult {
        cycles: max_cycles,
        reached,
        trace,
        final_growth: pets.iter().map(|p| (p.name.clone(), p.growth)).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pet(name: &str, growth: u64, bonus: f32) -> CampaignPet {
        CampaignPet { name: name.into(), growth, stats: None, campaign_bonus_pct: bonus, passive_per_hour: 0.0 }
    }

    fn params(hours: u32) -> CampaignParams {
        CampaignParams { upc_pct: 0.0, hours, unlocked_pets: 5, div_per_sec: None }
    }

    #[test]
    fn growth_excludes_the_weakest_and_names_it_recipient() {
        let team = vec![pet("A", 100_000, 0.0), pet("B", 50_000, 0.0), pet("C", 200_000, 0.0)];
        let p = params(1);
        let CampaignOutcome::Growth { total, recipient } = simulate(CampaignType::Growth, &team, &p)
        else {
            panic!("expected Growth outcome");
        };
        assert_eq!(recipient, 1); // B has the lowest growth
        // Only A and C contribute: (log15(g) - 1.75) each, hours=1, no bonuses.
        let expected = (log_base(100_000.0, 15.0) - 1.75) + (log_base(200_000.0, 15.0) - 1.75);
        assert!((total - expected).abs() < 1e-9, "got {total}, want {expected}");
    }

    #[test]
    fn growth_recipient_uses_end_of_run_growth_so_a_pendant_can_flip_it() {
        // A starts lowest (1000) but carries a pendant (+200/hr); over 12h it
        // reaches 1000 + 2400 = 3400, overtaking B (1100). So B becomes the
        // recipient and A contributes — the edge case from rushing a new pet.
        let mut a = pet("A", 1_000, 0.0);
        a.passive_per_hour = 200.0;
        let b = pet("B", 1_100, 0.0); // no pendant
        let p = params(12);
        let CampaignOutcome::Growth { total, recipient } = simulate(CampaignType::Growth, &[a, b], &p)
        else {
            panic!("expected Growth outcome");
        };
        assert_eq!(recipient, 1, "B should be the recipient at completion");
        // A contributes from its end-of-run growth (3400), not its start (1000).
        let expected = (log_base(3_400.0, 15.0) - 1.75) * 12.0;
        assert!((total - expected).abs() < 1e-6, "got {total}, want {expected}");
    }

    #[test]
    fn growth_scales_with_hours_upc_and_pet_bonus() {
        let team = vec![pet("A", 100_000, 100.0), pet("B", 50_000, 0.0)];
        // hours 12, UPC +50% → factor on A = 1.5 * 2.0 * 12 = 36.
        let p = CampaignParams { upc_pct: 50.0, hours: 12, unlocked_pets: 2, div_per_sec: None };
        let CampaignOutcome::Growth { total, recipient } = simulate(CampaignType::Growth, &team, &p)
        else {
            panic!();
        };
        assert_eq!(recipient, 1);
        let expected = (log_base(100_000.0, 15.0) - 1.75) * 1.5 * 2.0 * 12.0;
        assert!((total - expected).abs() < 1e-6, "got {total}, want {expected}");
    }

    #[test]
    fn stat_dependent_campaigns_block_without_stats() {
        let team = vec![pet("A", 100_000, 0.0)];
        let p = CampaignParams { upc_pct: 0.0, hours: 1, unlocked_pets: 1, div_per_sec: Some(10.0) };
        assert_eq!(simulate(CampaignType::Divinity, &team, &p), CampaignOutcome::NeedsStats);
        assert_eq!(simulate(CampaignType::Item, &team, &p), CampaignOutcome::NeedsStats);
        assert_eq!(simulate(CampaignType::Level, &team, &p), CampaignOutcome::NeedsStats);
        assert_eq!(simulate(CampaignType::GodPower, &team, &p), CampaignOutcome::NeedsStats);
    }

    #[test]
    fn divinity_blocks_on_missing_div_per_sec_then_computes() {
        let mut a = pet("A", 100_000, 0.0);
        a.stats = Some(50_000.0);
        let team = vec![a];
        // No div_per_sec → blocked on data even with stats.
        let p0 = params(1);
        assert_eq!(
            simulate(CampaignType::Divinity, &team, &p0),
            CampaignOutcome::NeedsData("divinity gained per second")
        );
        // With stats + div_per_sec it computes (50k stats < 10m → low branch).
        let p = CampaignParams { div_per_sec: Some(2.0), ..params(1) };
        let CampaignOutcome::Reward(v) = simulate(CampaignType::Divinity, &team, &p) else {
            panic!("expected Reward");
        };
        let expected = (50_000_f64.powf(0.4) + 300.0) * 2.0 * 2.0; // f(stats)*dps, factor=1
        assert!((v - expected).abs() < 1e-6, "got {v}");
    }

    #[test]
    fn food_blocks_on_missing_tier_data() {
        let team = vec![pet("A", 100_000, 0.0)];
        assert_eq!(
            simulate(CampaignType::Food, &team, &params(1)),
            CampaignOutcome::NeedsData("per-pet food tiers (mighty/strong/puny)")
        );
    }

    #[test]
    fn multiplier_returns_the_stat_independent_rebirth_half() {
        let team = vec![pet("A", 100_000, 0.0)];
        let CampaignOutcome::Reward(v) = simulate(CampaignType::Multiplier, &team, &params(1)) else {
            panic!("expected Reward");
        };
        let expected = (log_base(100_000.0, 2.0) - 7.0) / 2.0;
        assert!((v - expected).abs() < 1e-9, "got {v}");
    }

    #[test]
    fn item_and_gp_tiers_have_the_documented_shape() {
        assert_eq!(item_tier(5_000.0), 1.0); // below first threshold
        assert_eq!(item_tier(10_000.0), 2.0); // +1 at 10k
        assert_eq!(item_tier(6e7), 8.0); // past all seven +1 steps
        assert_eq!(item_tier(2e8), 10.0); // +2 at 100m, capped
        assert!((gp_tier(1.0) - 3.0).abs() < 1e-9); // base
        assert!((gp_tier(1e8) - 12.0).abs() < 1e-9); // cap at 100m
    }

    #[test]
    fn god_power_doubles_with_fsm() {
        let mut a = pet("A", 100_000, 0.0);
        a.stats = Some(1e6);
        let mut fsm = pet("FSM", 100_000, 0.0);
        fsm.stats = Some(1e6);
        let p = params(1);
        let CampaignOutcome::Reward(without) = simulate(CampaignType::GodPower, &[a.clone()], &p) else {
            panic!();
        };
        let CampaignOutcome::Reward(with) = simulate(CampaignType::GodPower, &[a, fsm], &p) else {
            panic!();
        };
        // Two identical-stat pets doubled by FSM = 4× one pet.
        assert!((with - without * 4.0).abs() < 1e-6, "with={with}, without={without}");
    }

    #[test]
    fn chamber_rushes_a_new_pet_to_its_target() {
        // A new low-growth pet stays the recipient while two residents feed it.
        let mut pets = vec![
            ChamberPet { name: "Resident1".into(), growth: 200_000.0, campaign_bonus_pct: 0.0, passive_per_hour: 0.0, target: None },
            ChamberPet { name: "Resident2".into(), growth: 210_000.0, campaign_bonus_pct: 0.0, passive_per_hour: 0.0, target: None },
            ChamberPet { name: "NewPet".into(), growth: 1_000.0, campaign_bonus_pct: 0.0, passive_per_hour: 0.0, target: Some(2_000.0) },
        ];
        let result = simulate_growth_chamber(&mut pets, 12, 0.0, 1000);
        // Reached its target in a finite number of cycles; the loop stopped there.
        assert!(result.reached.iter().any(|(n, _)| n == "NewPet"));
        assert!(result.cycles >= 1 && result.cycles < 1000);
        // The new pet is the recipient throughout (it never catches the residents).
        assert!(result.trace.iter().all(|c| c.recipient == 2));
        let new_final = result.final_growth.iter().find(|(n, _)| n == "NewPet").unwrap().1;
        assert!(new_final >= 2_000.0, "new pet only reached {new_final}");
    }

    #[test]
    fn chamber_recipient_rotates_between_close_pets() {
        // Two near-equal pets: whoever is fed leaps ahead, so the recipient flips
        // each cycle — the rotation that cycles a settled chamber.
        let mut pets = vec![
            ChamberPet { name: "A".into(), growth: 1_000.0, campaign_bonus_pct: 0.0, passive_per_hour: 0.0, target: None },
            ChamberPet { name: "B".into(), growth: 1_001.0, campaign_bonus_pct: 0.0, passive_per_hour: 0.0, target: None },
        ];
        let result = simulate_growth_chamber(&mut pets, 12, 0.0, 4);
        let recipients: Vec<usize> = result.trace.iter().map(|c| c.recipient).collect();
        assert_eq!(recipients, vec![0, 1, 0, 1]); // alternating
    }

    #[test]
    fn chamber_handles_duplicate_targeted_names() {
        // Two targeted pets that share a name must both be recorded (dedup is by
        // index, not name) so the stop condition can fire.
        let mut pets = vec![
            ChamberPet { name: "Feeder".into(), growth: 500_000.0, campaign_bonus_pct: 0.0, passive_per_hour: 0.0, target: None },
            ChamberPet { name: "Dup".into(), growth: 1_000.0, campaign_bonus_pct: 0.0, passive_per_hour: 100.0, target: Some(2_000.0) },
            ChamberPet { name: "Dup".into(), growth: 1_500.0, campaign_bonus_pct: 0.0, passive_per_hour: 100.0, target: Some(2_000.0) },
        ];
        let result = simulate_growth_chamber(&mut pets, 12, 0.0, 1000);
        assert_eq!(result.reached.len(), 2); // both, despite the shared name
        assert!(result.cycles < 1000); // stopped early, didn't spin to the cap
    }

    #[test]
    fn chamber_passive_growth_ticks_per_hour() {
        // One pet, no campaign contribution (it's always the recipient and the
        // only pet, so total is 0), only passive growth: +100/hr × 12h × 2 cycles.
        let mut pets = vec![ChamberPet {
            name: "Solo".into(),
            growth: 0.0,
            campaign_bonus_pct: 0.0,
            passive_per_hour: 100.0,
            target: None,
        }];
        let result = simulate_growth_chamber(&mut pets, 12, 0.0, 2);
        assert_eq!(result.cycles, 2);
        assert!((result.final_growth[0].1 - 2.0 * 100.0 * 12.0).abs() < 1e-9);
    }
}
