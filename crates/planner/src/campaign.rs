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
    let (total, recipient, _) = growth_campaign_detailed(team, p);
    CampaignOutcome::Growth { total, recipient }
}

/// The Growth campaign with each pet's contribution exposed (0 for the recipient),
/// for the chamber's per-cycle breakdown. `contributions.iter().sum()` is the
/// base total. Team must be non-empty.
pub fn growth_campaign_detailed(team: &[CampaignPet], p: &CampaignParams) -> (f64, usize, Vec<f64>) {
    let hours = p.hours.clamp(1, 12) as f64;
    let end_growth = |pet: &CampaignPet| pet.growth as f64 + pet.passive_per_hour * hours;
    let recipient = (0..team.len())
        .min_by(|&a, &b| {
            end_growth(&team[a])
                .partial_cmp(&end_growth(&team[b]))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("non-empty team");
    let contributions: Vec<f64> = team
        .iter()
        .enumerate()
        .map(|(i, pet)| {
            if i == recipient {
                0.0
            } else {
                (log_base(end_growth(pet), 15.0) - 1.75).max(0.0) * pet_factor(pet, p)
            }
        })
        .collect();
    let total = contributions.iter().sum();
    (total, recipient, contributions)
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
// Growth special-pet layer (Pandora's Box + Bag)
// =============================================================================

/// The growth a Growth-campaign run actually deposits, once the two special pets
/// are applied. Validated against a real chamber run (see
/// `reference/chamber_validation.md`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrowthSpecials {
    /// What the campaign **recipient** gains: the base total boosted by Pandora.
    pub recipient_gain: f64,
    /// What **Bag** gifts to the *global* lowest-growth pet (0 if Bag absent).
    pub bag_gift: f64,
}

/// Apply the Growth special-pet layer to a base campaign total (`Σ`
/// contributions, recipient excluded):
///
/// 1. **Pandora's Box** scales the base and the result goes to the recipient:
///    `recipient_gain = base · (1 + pandora_pct/100)`.
/// 2. **Bag** gifts a fraction of that *Pandora-boosted* recipient gain to the
///    **global** lowest-growth pet: `bag_gift = bag_fraction · recipient_gain`
///    (0.05 token-improved / 0.10 pre-token; 0 if Bag absent).
///
/// The caller decides `pandora_pct` (from Pandora's growth + feedings),
/// `bag_fraction` (from Bag's presence + token state), and *which* pet receives
/// each (recipient = chamber min; Bag's target = global min). For **pre-token**
/// Bag the gift is *stolen* — subtract `bag_gift` from `recipient_gain` too;
/// token-improved it's free (this fn returns the gross figures either way).
///
/// Verified: base 1,062.29, Pandora +43.42% → recipient_gain 1,523.5 (game
/// 1,523.6); Bag 5% → 76.18 (exact).
pub fn apply_growth_specials(base_total: f64, pandora_pct: f64, bag_fraction: f64) -> GrowthSpecials {
    let recipient_gain = base_total * (1.0 + pandora_pct / 100.0);
    GrowthSpecials { recipient_gain, bag_gift: bag_fraction * recipient_gain }
}

/// A chamber pet's special-pet behaviour in the Growth campaign.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialPet {
    /// Pandora's Box — scales the campaign total by its growth/feeding bonus,
    /// boosting what the recipient gains.
    Pandora { feedings: u32 },
    /// Bag — gifts to the **global** lowest-growth pet. Token-improved gives a
    /// free 5%; pre-token steals 10% from the campaign.
    Bag { token_improved: bool },
    /// Nightmare — subtracts `(20 − 0.25·class_level)` points (min 1) from every
    /// **other** chamber pet's campaign bonus. (Its own `+200%` self-boost is part
    /// of its campaign bonus already, via the curated overrides.)
    Nightmare { class_level: u32 },
}

/// The campaign-bonus points Nightmare docks from each *other* pet at a given
/// class level: `(20 − 0.25·CL)`, floored at 1 (subtractive, per the wiki).
pub fn nightmare_malus(class_level: u32) -> f64 {
    (20.0 - 0.25 * class_level as f64).max(1.0)
}

/// Fish Power's food boost **at a rebirth's start**, as a percent:
/// `FishPower^0.25 · milestoneMult`, capped at 200%. Milestones are additive:
/// +0.1× at fishing level 15, another +0.1× at 27 (so 1.0 / 1.1 / 1.2). The boost
/// decays linearly to 0 over the first 30 h of a rebirth (applied by the sim).
/// Returns 0 with no Fish Power. (e.g. 1e6 FP, level 14 → 31.62%.)
pub fn fishing_boost_pct(fish_power: f64, fishing_level: u32) -> f64 {
    if fish_power <= 0.0 {
        return 0.0;
    }
    let milestone =
        1.0 + 0.1 * (fishing_level >= 15) as u8 as f64 + 0.1 * (fishing_level >= 27) as u8 as f64;
    (fish_power.powf(0.25) * milestone).min(200.0)
}

/// Fraction of the fishing boost still active at `hours` into a rebirth — linear
/// from 1.0 at the start to 0.0 at 30 h, then 0.
fn fishing_decay(hours_into_rebirth: f64) -> f64 {
    (1.0 - hours_into_rebirth / 30.0).max(0.0)
}

/// The per-cycle hours over **one rebirth**: `floor(rebirth/cycle)` full cycles
/// of `cycle_hours`, then a shorter cycle for the leftover (a campaign can't span
/// a rebirth). If the rebirth is shorter than a cycle, the cycle is clamped to
/// the rebirth. The chamber repeats this schedule across rebirths.
///
/// `cycle_hours` is clamped to the campaign range 1..=12 and to the rebirth, so
/// every entry is a valid campaign length and the result is never empty.
///
/// Examples: `(12, 20) ⭢ [12, 8]`, `(12, 24) ⭢ [12, 12]`, `(12, 8) ⭢ [8]`,
/// `(12, 13) ⭢ [12, 1]`.
pub fn rebirth_schedule(cycle_hours: u32, rebirth_hours: u32) -> Vec<u32> {
    let rebirth = rebirth_hours.max(1);
    let full = cycle_hours.clamp(1, 12).min(rebirth); // ≤ 12 and ≤ rebirth
    let mut sched = vec![full; (rebirth / full) as usize];
    let rem = rebirth % full;
    if rem > 0 {
        sched.push(rem);
    }
    sched
}

/// Pandora's Box campaign bonus %: `min(growth, 100k)/5000 · rate`, where the
/// per-5k rate is 3 below 100k growth (4 at/above) plus 0.1 per feeding (cap +2).
/// (e.g. growth 57,138, 8 feedings → 43.42.)
fn pandora_pct(growth: f64, feedings: u32) -> f64 {
    let rate = (if growth < 100_000.0 { 3.0 } else { 4.0 }) + (feedings as f64 * 0.1).min(2.0);
    growth.min(100_000.0) / 5_000.0 * rate
}

// =============================================================================
// Growth chamber
// =============================================================================

/// A pet in the simulation's roster. The roster is the player's **full** set of
/// relevant pets — both the campaign participants and the bench — because some
/// effects (Bag's gift) target the global lowest pet, which may be benched.
#[derive(Debug, Clone)]
pub struct ChamberPet {
    pub name: String,
    /// **Base** growth — the accumulator. All gains (campaign reward, passive,
    /// feeding) are added here; the Growth campaign reads **total** growth
    /// (`growth · growth_multiplier`).
    pub growth: f64,
    /// Total-growth multiplier (Magic Egg ×1.3, Patreon-God-Challenge, …). The
    /// campaign formula, recipient, Bag, Pandora and targets all use total growth,
    /// while rewards land on base — so an egg-carrying recipient's total jumps by
    /// `reward · multiplier`.
    pub growth_multiplier: f64,
    pub campaign_bonus_pct: f32,
    /// Pendant + Moai growth per hour for this pet (0 if none). One tick per
    /// campaign-hour (see `campaign_simulation.md` §7).
    pub passive_per_hour: f64,
    /// Effective growth per **normal** feeding for this pet (food type ×
    /// multipliers; 0 to skip). Every pet is fed `floor(hours/3)` times per cycle
    /// — see `food_and_feedings.md`. Excludes the Gold Dragon broadcast (tracked
    /// separately for the report).
    pub food_per_feeding: f64,
    /// Gold Dragon's per-feeding broadcast to this pet (25% of his food's growth;
    /// 0 if he isn't fed). Added every feeding alongside `food_per_feeding`, but
    /// kept separate so the report can attribute it.
    pub gold_dragon_per_feeding: f64,
    /// Stop tracking this pet once its growth reaches this target (e.g. an evolve
    /// threshold). `None` = an untracked resident.
    pub target: Option<f64>,
    /// Whether this pet is in the campaign (a contributor / the recipient). Bench
    /// pets (`false`) still accrue passive growth and can receive Bag's gift, but
    /// don't run the campaign.
    pub in_chamber: bool,
    /// Special-pet behaviour, if any (Pandora's Box / Bag).
    pub special: Option<SpecialPet>,
}

/// Per-cycle record for the chamber trace.
#[derive(Debug, Clone)]
pub struct ChamberCycle {
    /// The campaign recipient (the lowest-growth *in-chamber* pet).
    pub recipient: usize,
    /// Growth deposited into the recipient (base total, Pandora-boosted, minus any
    /// pre-token Bag steal).
    pub recipient_gain: f64,
    /// Bag's gift `(global-lowest index, amount)`, if a Bag is in the chamber.
    pub bag_gift: Option<(usize, f64)>,
    /// Each in-chamber pet's growth contribution this cycle `(roster index,
    /// contribution)` — 0 for the recipient. Their sum is the base campaign total.
    pub contributions: Vec<(usize, f64)>,
    /// This cycle's campaign length in hours (a rebirth's last cycle is shorter).
    pub hours: u32,
}

/// A pet's growth over a run, split by source (all in **total** terms — already
/// scaled by the pet's `growth_multiplier`, so the four sum to its growth gain).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GrowthBreakdown {
    /// Campaign growth received: recipient deposits + any Bag gifts.
    pub campaign: f64,
    /// Passive growth: Moai (everyone) + the pendant (if equipped).
    pub passive: f64,
    /// Normal feeding growth (the chosen food).
    pub feeding: f64,
    /// Gold Dragon's broadcast feeding growth.
    pub gold_dragon: f64,
}

impl GrowthBreakdown {
    /// Total growth gained across all sources.
    pub fn total(&self) -> f64 {
        self.campaign + self.passive + self.feeding + self.gold_dragon
    }
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
    /// Growth gained per pet split by source, in the input order (parallel to
    /// `final_growth`).
    pub breakdown: Vec<(String, GrowthBreakdown)>,
}

/// Tick one round of between-campaign growth into every pet — passive
/// (Moai + pendant, over `passive_hours`) and feeding (normal + Gold Dragon) —
/// recording each source into `breakdown` (in total terms, scaled by the pet's
/// multiplier). `passive_hours` is 0 for a cycle whose passive is suppressed
/// (e.g. the first cycle when the export already includes it).
fn tick_passive_and_feeding(
    pets: &mut [ChamberPet],
    breakdown: &mut [GrowthBreakdown],
    passive_hours: f64,
    feedings: f64,
    // Multiplier on *food* growth this cycle (the Fish Power boost; 1.0 = none).
    // Passive isn't food, so it's unaffected.
    fishing_mult: f64,
) {
    for (i, pet) in pets.iter_mut().enumerate() {
        let mult = pet.growth_multiplier;
        let passive = pet.passive_per_hour * passive_hours;
        let feed = feedings * pet.food_per_feeding * fishing_mult;
        let gd = feedings * pet.gold_dragon_per_feeding * fishing_mult;
        pet.growth += passive + feed + gd;
        breakdown[i].passive += passive * mult;
        breakdown[i].feeding += feed * mult;
        breakdown[i].gold_dragon += gd * mult;
    }
}

/// Run parameters for [`simulate_growth_chamber`] — everything about the run
/// itself, as opposed to the roster. `Default` is a single uniform 12 h cycle
/// with no bonuses, so tests and callers only name what they change.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChamberRun {
    /// Campaign length per cycle in hours (clamped to 1..=12).
    pub hours: u32,
    /// Universal Pet Campaign bonus, in percent.
    pub upc_pct: f64,
    /// Cycles to simulate (the safety cap when `stop_at_targets` is set).
    pub max_cycles: u32,
    /// Return early once every *targeted* pet has reached its target.
    pub stop_at_targets: bool,
    /// When true, the **first** cycle adds no passive growth — for an export
    /// captured at a campaign's end, which already holds that campaign's passive
    /// (Moai). Avoids double-counting it; see `reference/real_growth_campaign`.
    pub skip_first_cycle_passive: bool,
    /// Average rebirth length in hours. `Some` makes the last cycle of each
    /// rebirth shorter (the remainder) so a campaign never spans a rebirth —
    /// e.g. a 20 h rebirth with 12 h cycles runs `12, 8, 12, 8, …`. `None` runs
    /// uniform cycles.
    pub rebirth_hours: Option<u32>,
    /// Fish Power food boost % at a rebirth's start; decays linearly to 0 over
    /// the first 30 h of each rebirth. 0 = none. Only applies when rebirths are
    /// modelled (it's a rebirth-relative effect).
    pub fishing_boost_pct: f64,
}

impl Default for ChamberRun {
    fn default() -> Self {
        Self {
            hours: 12,
            upc_pct: 0.0,
            max_cycles: 1,
            stop_at_targets: false,
            skip_first_cycle_passive: false,
            rebirth_hours: None,
            fishing_boost_pct: 0.0,
        }
    }
}

/// Simulate a growth chamber: repeatedly run the Growth campaign over the
/// **in-chamber** pets, then realise growth across the **whole roster** —
/// depositing the (Pandora-boosted) total into the recipient, ticking passive
/// pendant/Moai growth into every pet, and applying Bag's gift to the global
/// lowest pet — for `max_cycles` rounds. If `stop_at_targets` is set, it returns
/// early once every *targeted* pet has reached its target (with `max_cycles` as
/// the safety cap); otherwise it always runs the full `max_cycles` (still
/// recording the cycle each target was first hit).
///
/// `pets` is the full roster (chamber + bench, flagged by `in_chamber`). It reuses
/// [`simulate`] with [`CampaignType::Growth`] for the base total so the chamber and
/// the one-off campaign share one formula, then layers Pandora/Bag on top
/// ([`apply_growth_specials`]).
pub fn simulate_growth_chamber(pets: &mut [ChamberPet], run: &ChamberRun) -> ChamberResult {
    let ChamberRun {
        hours,
        upc_pct,
        max_cycles,
        stop_at_targets,
        skip_first_cycle_passive,
        rebirth_hours,
        fishing_boost_pct,
    } = *run;
    let base_hours = hours.clamp(1, 12);
    // Per-cycle hours, repeating over a rebirth (`None` ⭢ uniform). Each entry is
    // already ≤ base_hours ≤ 12, so it needs no further clamping.
    let schedule: Vec<u32> = match rebirth_hours {
        Some(rb) => rebirth_schedule(base_hours, rb.max(1)),
        None => vec![base_hours],
    };
    let targeted = pets.iter().filter(|p| p.target.is_some()).count();
    let mut reached: Vec<(String, u32)> = Vec::new();
    // Per-pet "already reached" flags, keyed by index (not name — pets can share
    // a name, which would otherwise break the stop condition).
    let mut done = vec![false; pets.len()];
    let mut trace: Vec<ChamberCycle> = Vec::new();
    // Growth gained per pet, split by source (total terms). Parallel to `pets`.
    let mut breakdown = vec![GrowthBreakdown::default(); pets.len()];

    // Pandora's running feeding count: seeds from her entered value, climbs as she
    // is fed each cycle (the bonus caps at 20 feedings), and resets each rebirth.
    let mut pandora_feedings: u32 = pets
        .iter()
        .find_map(|p| match p.special {
            Some(SpecialPet::Pandora { feedings }) => Some(feedings),
            _ => None,
        })
        .unwrap_or(0);
    // Hours elapsed into the current rebirth (drives the fishing-boost decay);
    // reset at each rebirth boundary alongside Pandora's feedings.
    let mut rebirth_elapsed = 0.0_f64;

    for cycle in 0..max_cycles {
        // This cycle's length — the rebirth schedule may shorten the last cycle.
        let cycle_hours = schedule[cycle as usize % schedule.len()];
        // Feedings per pet this round — one every 3 hours (`food_and_feedings.md`).
        let cycle_feedings = cycle_hours / 3;
        let feedings = cycle_feedings as f64;

        // Reset rebirth-relative state (Pandora's feedings, the fishing clock) at
        // the start of each rebirth — only when rebirths are modelled (otherwise
        // `schedule.len() == 1` and every cycle would falsely look like a boundary).
        if rebirth_hours.is_some() && cycle > 0 && (cycle as usize).is_multiple_of(schedule.len()) {
            pandora_feedings = 0;
            rebirth_elapsed = 0.0;
        }
        // Fish Power boosts food, decaying over the rebirth's first 30 h. Evaluate
        // at the cycle's midpoint; only applies when rebirths are modelled.
        let fishing_mult = if rebirth_hours.is_some() && fishing_boost_pct > 0.0 {
            let midpoint = rebirth_elapsed + cycle_hours as f64 / 2.0;
            1.0 + fishing_boost_pct / 100.0 * fishing_decay(midpoint)
        } else {
            1.0
        };
        // End-of-run **total** growth — the basis the campaign picks the recipient
        // / global lowest from, and what targets compare against. Total = base ·
        // multiplier (Magic Egg etc.). Only **passive** (pendant + Moai) accrues
        // *during* the campaign; feeding happens between rounds (applied after the
        // deposits, not here), compounding into the next round.
        let end_total =
            |p: &ChamberPet| (p.growth + p.passive_per_hour * cycle_hours as f64) * p.growth_multiplier;
        // Passive hours realised this cycle — suppressed on the first cycle when
        // the export already includes the finishing campaign's passive. (The
        // formula's end-of-run growth still uses the full hours; the extra passive
        // there shifts contributions by a negligible fraction.)
        let passive_hours =
            if cycle == 0 && skip_first_cycle_passive { 0.0 } else { cycle_hours as f64 };
        // Indices of the campaign participants, in roster order.
        let chamber_idx: Vec<usize> = (0..pets.len()).filter(|&i| pets[i].in_chamber).collect();

        let mut cycle_record =
            ChamberCycle { recipient: 0, recipient_gain: 0.0, bag_gift: None, contributions: Vec::new(), hours: cycle_hours };

        if !chamber_idx.is_empty() {
            // Nightmare's team malus: every *other* chamber pet's campaign bonus is
            // reduced by `(20 − 0.25·CL)` points (subtractive). Find the Nightmare,
            // if any, so its own bonus is left intact.
            let nightmare: Option<(usize, f64)> = chamber_idx.iter().find_map(|&i| {
                match pets[i].special {
                    Some(SpecialPet::Nightmare { class_level }) => {
                        Some((i, nightmare_malus(class_level)))
                    }
                    _ => None,
                }
            });

            // Base total + recipient from the Growth campaign over the chamber. The
            // f64 growth truncates to u64 for the log term — lossy by design,
            // negligible at real magnitudes.
            let team: Vec<CampaignPet> = chamber_idx
                .iter()
                .map(|&i| CampaignPet {
                    name: pets[i].name.clone(),
                    // Total growth drives the formula/recipient; passive is scaled
                    // to total too, so `simulate`'s end-of-run growth matches
                    // `end_total`.
                    growth: (pets[i].growth * pets[i].growth_multiplier) as u64,
                    stats: None,
                    campaign_bonus_pct: match nightmare {
                        Some((nm, malus)) if i != nm => {
                            pets[i].campaign_bonus_pct - malus as f32
                        }
                        _ => pets[i].campaign_bonus_pct,
                    },
                    passive_per_hour: pets[i].passive_per_hour * pets[i].growth_multiplier,
                })
                .collect();
            let params =
                CampaignParams { upc_pct, hours: cycle_hours, unlocked_pets: chamber_idx.len(), div_per_sec: None };
            let (base, recipient_sub, contribs) = growth_campaign_detailed(&team, &params);
            // Map the sub-team contributions back to roster indices.
            cycle_record.contributions =
                chamber_idx.iter().zip(contribs).map(|(&i, c)| (i, c)).collect();
            let recipient = chamber_idx[recipient_sub];

            // Special-pet parameters, read from the in-chamber pets. Assumes at
            // most one Pandora and one Bag (the game has one of each); with
            // duplicates the last in roster order wins.
            let mut pandora = 0.0;
            let mut bag_fraction = 0.0;
            let mut bag_steals = false;
            for &i in &chamber_idx {
                match pets[i].special {
                    Some(SpecialPet::Pandora { .. }) => {
                        // Use the running feeding count, not the static seed.
                        pandora = pandora_pct(end_total(&pets[i]), pandora_feedings);
                    }
                    Some(SpecialPet::Bag { token_improved }) => {
                        bag_fraction = if token_improved { 0.05 } else { 0.10 };
                        bag_steals = !token_improved;
                    }
                    // Nightmare's malus is already folded into the team bonuses above.
                    Some(SpecialPet::Nightmare { .. }) | None => {}
                }
            }
            let specials = apply_growth_specials(base, pandora, bag_fraction);
            // Token-improved Bag's gift is free; pre-token it's stolen from the
            // recipient's deposit.
            let recipient_deposit = if bag_steals {
                specials.recipient_gain - specials.bag_gift
            } else {
                specials.recipient_gain
            };

            // Bag's gift goes to the global lowest pet (end-of-run growth), across
            // the whole roster — usually a benched pet, possibly the recipient.
            let global_lowest = (0..pets.len())
                .min_by(|&a, &b| {
                    end_total(&pets[a])
                        .partial_cmp(&end_total(&pets[b]))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .expect("non-empty roster");

            // Realise growth: passive + feeding into all, then the deposits.
            tick_passive_and_feeding(pets, &mut breakdown, passive_hours, feedings, fishing_mult);
            pets[recipient].growth += recipient_deposit;
            breakdown[recipient].campaign += recipient_deposit * pets[recipient].growth_multiplier;
            if bag_fraction > 0.0 {
                pets[global_lowest].growth += specials.bag_gift;
                breakdown[global_lowest].campaign +=
                    specials.bag_gift * pets[global_lowest].growth_multiplier;
                cycle_record.bag_gift = Some((global_lowest, specials.bag_gift));
            }
            cycle_record.recipient = recipient;
            cycle_record.recipient_gain = recipient_deposit;
        } else {
            // No campaign this cycle — only passive + feeding growth ticks.
            tick_passive_and_feeding(pets, &mut breakdown, passive_hours, feedings, fishing_mult);
        }
        // This cycle's feedings raise Pandora's count toward the 20-feeding cap.
        pandora_feedings = (pandora_feedings + cycle_feedings).min(20);
        // Advance the rebirth clock (drives the fishing-boost decay).
        rebirth_elapsed += cycle_hours as f64;
        trace.push(cycle_record);

        // Record any targeted pet that crossed its target this cycle (by index).
        for i in 0..pets.len() {
            if !done[i]
                && let Some(t) = pets[i].target
                && pets[i].growth * pets[i].growth_multiplier >= t
            {
                done[i] = true;
                reached.push((pets[i].name.clone(), cycle + 1));
            }
        }
        if stop_at_targets && targeted > 0 && reached.len() == targeted {
            return ChamberResult {
                cycles: cycle + 1,
                reached,
                trace,
                final_growth: pets.iter().map(|p| (p.name.clone(), p.growth * p.growth_multiplier)).collect(),
                breakdown: pets.iter().zip(&breakdown).map(|(p, b)| (p.name.clone(), b.clone())).collect(),
            };
        }
    }

    ChamberResult {
        cycles: max_cycles,
        reached,
        trace,
        final_growth: pets.iter().map(|p| (p.name.clone(), p.growth * p.growth_multiplier)).collect(),
        breakdown: pets.iter().zip(&breakdown).map(|(p, b)| (p.name.clone(), b.clone())).collect(),
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
            ChamberPet { name: "Resident1".into(), growth: 200_000.0, growth_multiplier: 1.0,
            campaign_bonus_pct: 0.0, passive_per_hour: 0.0, food_per_feeding: 0.0, gold_dragon_per_feeding: 0.0, target: None, in_chamber: true, special: None },
            ChamberPet { name: "Resident2".into(), growth: 210_000.0, growth_multiplier: 1.0,
            campaign_bonus_pct: 0.0, passive_per_hour: 0.0, food_per_feeding: 0.0, gold_dragon_per_feeding: 0.0, target: None, in_chamber: true, special: None },
            ChamberPet { name: "NewPet".into(), growth: 1_000.0, growth_multiplier: 1.0,
            campaign_bonus_pct: 0.0, passive_per_hour: 0.0, food_per_feeding: 0.0, gold_dragon_per_feeding: 0.0, target: Some(2_000.0), in_chamber: true, special: None },
        ];
        let result = simulate_growth_chamber(
            &mut pets,
            &ChamberRun { max_cycles: 1000, stop_at_targets: true, ..Default::default() },
        );
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
            ChamberPet { name: "A".into(), growth: 1_000.0, growth_multiplier: 1.0,
            campaign_bonus_pct: 0.0, passive_per_hour: 0.0, food_per_feeding: 0.0, gold_dragon_per_feeding: 0.0, target: None, in_chamber: true, special: None },
            ChamberPet { name: "B".into(), growth: 1_001.0, growth_multiplier: 1.0,
            campaign_bonus_pct: 0.0, passive_per_hour: 0.0, food_per_feeding: 0.0, gold_dragon_per_feeding: 0.0, target: None, in_chamber: true, special: None },
        ];
        let result = simulate_growth_chamber(
            &mut pets,
            &ChamberRun { max_cycles: 4, stop_at_targets: true, ..Default::default() },
        );
        let recipients: Vec<usize> = result.trace.iter().map(|c| c.recipient).collect();
        assert_eq!(recipients, vec![0, 1, 0, 1]); // alternating
    }

    #[test]
    fn chamber_handles_duplicate_targeted_names() {
        // Two targeted pets that share a name must both be recorded (dedup is by
        // index, not name) so the stop condition can fire.
        let mut pets = vec![
            ChamberPet { name: "Feeder".into(), growth: 500_000.0, growth_multiplier: 1.0,
            campaign_bonus_pct: 0.0, passive_per_hour: 0.0, food_per_feeding: 0.0, gold_dragon_per_feeding: 0.0, target: None, in_chamber: true, special: None },
            ChamberPet { name: "Dup".into(), growth: 1_000.0, growth_multiplier: 1.0,
            campaign_bonus_pct: 0.0, passive_per_hour: 100.0, food_per_feeding: 0.0, gold_dragon_per_feeding: 0.0, target: Some(2_000.0), in_chamber: true, special: None },
            ChamberPet { name: "Dup".into(), growth: 1_500.0, growth_multiplier: 1.0,
            campaign_bonus_pct: 0.0, passive_per_hour: 100.0, food_per_feeding: 0.0, gold_dragon_per_feeding: 0.0, target: Some(2_000.0), in_chamber: true, special: None },
        ];
        let result = simulate_growth_chamber(
            &mut pets,
            &ChamberRun { max_cycles: 1000, stop_at_targets: true, ..Default::default() },
        );
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
            growth_multiplier: 1.0,
            campaign_bonus_pct: 0.0,
            passive_per_hour: 100.0,
            food_per_feeding: 0.0,
            gold_dragon_per_feeding: 0.0,
            target: None,
            in_chamber: true,
            special: None,
        }];
        let result = simulate_growth_chamber(
            &mut pets,
            &ChamberRun { max_cycles: 2, stop_at_targets: true, ..Default::default() },
        );
        assert_eq!(result.cycles, 2);
        assert!((result.final_growth[0].1 - 2.0 * 100.0 * 12.0).abs() < 1e-9);
    }

    #[test]
    fn chamber_feeding_growth_ticks_per_cycle() {
        let solo = |food: f64| ChamberPet {
            name: "Solo".into(),
            growth: 0.0,
            growth_multiplier: 1.0,
            campaign_bonus_pct: 0.0,
            passive_per_hour: 0.0,
            food_per_feeding: food,
            gold_dragon_per_feeding: 0.0,
            target: None,
            in_chamber: true,
            special: None,
        };
        // 12 h → floor(12/3) = 4 feedings × 10/feeding = +40/cycle × 2 cycles = 80.
        let mut pets = vec![solo(10.0)];
        let r = simulate_growth_chamber(
            &mut pets,
            &ChamberRun { max_cycles: 2, stop_at_targets: true, ..Default::default() },
        );
        assert!((r.final_growth[0].1 - 80.0).abs() < 1e-9, "got {}", r.final_growth[0].1);
        // A sub-3h campaign yields no feedings (floor(2/3) = 0).
        let mut pets2 = vec![solo(10.0)];
        let r2 = simulate_growth_chamber(
            &mut pets2,
            &ChamberRun { hours: 2, stop_at_targets: true, ..Default::default() },
        );
        assert_eq!(r2.final_growth[0].1, 0.0);
    }

    #[test]
    fn chamber_egg_recipient_total_jumps_by_reward_times_multiplier() {
        let pet = |name: &str, base: f64, mult: f64| ChamberPet {
            name: name.into(),
            growth: base,
            growth_multiplier: mult,
            campaign_bonus_pct: 0.0,
            passive_per_hour: 0.0,
            food_per_feeding: 0.0,
            gold_dragon_per_feeding: 0.0,
            target: None,
            in_chamber: true,
            special: None,
        };
        // A is a high-total contributor; B is the low-total recipient, egg ×1.3.
        let mut pets = vec![pet("A", 100_000.0, 1.0), pet("B", 1_000.0, 1.3)];
        let r = simulate_growth_chamber(&mut pets, &ChamberRun::default());
        // Reward = A's contribution = (log15(100000) − 1.75) · 12. The reward lands
        // on B's *base*, so its *total* rises by reward · 1.3.
        let reward = (100_000_f64.ln() / 15_f64.ln() - 1.75) * 12.0;
        let b_total = r.final_growth.iter().find(|(n, _)| n == "B").unwrap().1;
        let expected = (1_000.0 + reward) * 1.3;
        assert!((b_total - expected).abs() < 0.01, "B total {b_total}, want {expected}");
        assert!(b_total > 1_000.0 + reward, "egg should amplify the total");
    }

    #[test]
    fn chamber_records_per_cycle_contributions() {
        let pet = |name: &str, g: f64| ChamberPet {
            name: name.into(),
            growth: g,
            growth_multiplier: 1.0,
            campaign_bonus_pct: 0.0,
            passive_per_hour: 0.0,
            food_per_feeding: 0.0,
            gold_dragon_per_feeding: 0.0,
            target: None,
            in_chamber: true,
            special: None,
        };
        let mut pets = vec![pet("Low", 1_000.0), pet("High", 100_000.0)];
        pets[1].campaign_bonus_pct = 50.0; // exercise the bonus factor
        let r = simulate_growth_chamber(&mut pets, &ChamberRun::default());
        let c = &r.trace[0].contributions;
        assert_eq!(c.len(), 2);
        // Low is the recipient (contributes 0); High contributes the base total,
        // scaled by its bonus: factor = (1+0) · (1+0.5) · 12 = 18.
        assert_eq!(c.iter().find(|(i, _)| *i == 0).unwrap().1, 0.0);
        let high = c.iter().find(|(i, _)| *i == 1).unwrap().1;
        let expected = (100_000_f64.ln() / 15_f64.ln() - 1.75) * 1.5 * 12.0;
        assert!((high - expected).abs() < 0.01, "got {high}");
    }

    /// End-to-end against a real finished 12 h chamber run (UPC +40%). See
    /// `reference/chamber_validation.md`. (growth, bonus%) per pet; Otter is the
    /// recipient and contributes nothing.
    #[test]
    fn real_chamber_run_matches_the_finish_screen() {
        // (name, growth, growth-campaign bonus %). Bonuses = in-game "total
        // reward" − 100. Otter (the recipient) is first.
        let chamber = [
            ("Otter", 55_266, 154.0),       // recipient — excluded from the sum
            ("Cupid", 55_338, 184.0),
            ("Bag", 55_468, 115.51),
            ("Hedgehog", 55_565, 222.76),
            ("Thunder Ball", 55_661, 481.0),
            ("Meteor", 55_856, 139.61),
            ("Earth Eater", 55_943, 132.0),
            ("Sphinx", 56_177, 119.97),
            ("Pandora's Box", 57_138, 0.0),
            ("Vampire", 57_310, 470.0),
        ];
        let team: Vec<CampaignPet> = chamber
            .iter()
            .map(|&(name, growth, bonus)| CampaignPet {
                name: name.into(),
                growth,
                stats: None,
                campaign_bonus_pct: bonus,
                passive_per_hour: 0.0, // Moai already baked into these growths
            })
            .collect();
        let params = CampaignParams { upc_pct: 40.0, hours: 12, unlocked_pets: 10, div_per_sec: None };

        let CampaignOutcome::Growth { total, recipient } = simulate(CampaignType::Growth, &team, &params)
        else {
            panic!("expected Growth outcome");
        };
        assert_eq!(recipient, 0, "Otter is the lowest-growth chamber pet");
        // Base total of the nine contributors — finish screen summed to 1,062.29.
        assert!((total - 1_062.29).abs() < 6.0, "base total {total}, want ≈1062.29");

        // Special-pet layer against the in-game base: Pandora +43.42% to Otter,
        // Bag 5% (token-improved) to Wolf (the global lowest, benched).
        let s = apply_growth_specials(1_062.29, 43.42, 0.05);
        assert!((s.recipient_gain - 1_523.6).abs() < 0.5, "Otter gain {}", s.recipient_gain);
        assert!((s.bag_gift - 76.18).abs() < 0.1, "Bag gift {}", s.bag_gift);
    }

    /// The same real run through the *chamber* — one cycle over the 10 chamber
    /// pets plus benched Wolf — exercising the full integration: Pandora boosts
    /// the recipient's deposit, and Bag's gift lands on the global lowest (Wolf).
    #[test]
    fn chamber_cycle_applies_pandora_to_recipient_and_bag_to_global_lowest() {
        // (name, growth, bonus%, in_chamber, special)
        let data: [(&str, f64, f32, bool, Option<SpecialPet>); 11] = [
            ("Otter", 55_266.0, 154.0, true, None), // recipient (chamber min)
            ("Cupid", 55_338.0, 184.0, true, None),
            ("Bag", 55_468.0, 115.51, true, Some(SpecialPet::Bag { token_improved: true })),
            ("Hedgehog", 55_565.0, 222.76, true, None),
            ("Thunder Ball", 55_661.0, 481.0, true, None),
            ("Meteor", 55_856.0, 139.61, true, None),
            ("Earth Eater", 55_943.0, 132.0, true, None),
            ("Sphinx", 56_177.0, 119.97, true, None),
            ("Pandora's Box", 57_138.0, 0.0, true, Some(SpecialPet::Pandora { feedings: 8 })),
            ("Vampire", 57_310.0, 470.0, true, None),
            ("Wolf", 10_956.0, 0.0, false, None), // benched — the global lowest
        ];
        let mut pets: Vec<ChamberPet> = data
            .iter()
            .map(|&(name, growth, bonus, in_chamber, special)| ChamberPet {
                name: name.into(),
                growth,
                growth_multiplier: 1.0,
                campaign_bonus_pct: bonus,
                passive_per_hour: 0.0,
                food_per_feeding: 0.0,
                gold_dragon_per_feeding: 0.0,
                target: None,
                in_chamber,
                special,
            })
            .collect();

        let result = simulate_growth_chamber(
            &mut pets,
            &ChamberRun { upc_pct: 40.0, stop_at_targets: true, ..Default::default() },
        );
        assert_eq!(result.cycles, 1);

        // Otter (index 0) is the recipient and gains base × Pandora's 1.4342
        // ≈ 1,523.6 → final ≈ 56,790 (the in-game after-value).
        let otter = result.final_growth[0].1;
        assert!((otter - 56_790.0).abs() < 2.0, "Otter final {otter}, want ≈56,790");
        // Wolf (index 10, benched) is the global lowest and gets Bag's gift
        // (≈76.18) → 10,956 + 76.18 ≈ 11,032.
        let wolf = result.final_growth[10].1;
        assert!((wolf - 11_032.0).abs() < 1.0, "Wolf final {wolf}, want ≈11,032");

        // The trace names the right recipient and Bag target.
        assert_eq!(result.trace[0].recipient, 0);
        let (bag_target, bag_amount) = result.trace[0].bag_gift.expect("Bag gift recorded");
        assert_eq!(bag_target, 10, "Bag's gift goes to Wolf, not the recipient");
        assert!((bag_amount - 76.18).abs() < 0.2, "Bag gift {bag_amount}");
    }

    /// SECOND real run (`reference/real_growth_campaign/`): a 12 h chamber where
    /// **Pandora carries a Magic Egg**. In-game, Bag (recipient, token-improved)
    /// gained **+1678.4**, Pandora's own contribution was 38.57, and Pandora's
    /// special boost was +580 (≈ +52.8% on the 1098.3 base).
    ///
    /// Egg handling end-to-end: the pet importer divides the egg's ×1.3 back out
    /// of the export "Growth" column and stores the **true base** (Pandora shows
    /// 57,410 → base 44,162); the chamber sets `growth_multiplier = 1.3` so the
    /// campaign reads `base · multiplier = 57,410` — the value the game uses. This
    /// seeds exactly that and confirms the egg is **not** double-applied.
    #[test]
    fn pandora_egg_real_run_reconciles() {
        // The file-1 growths were captured ~1 min before the campaign finished, so
        // they already include this run's 12 h of Moai — passive must be 0 here or
        // the recipient deposit double-counts it (same note as the run above).
        let moai = 0.0_f64;
        // Per feeding: chocolate 10.38 + Gold Dragon's broadcast 2.60 (already his
        // 25%). Irrelevant to the deposit (feeding lands after), set for realism.
        let food = 10.38;
        let gd = 2.60;
        // `total` is the export "Growth" column; `growth` is the true base the
        // importer stores (total / mult for an egg pet). The campaign reads
        // `growth · mult`, recovering the in-game total — no double egg.
        let mk = |name: &str, total: f64, bonus: f32, mult: f64, special: Option<SpecialPet>| {
            ChamberPet {
                name: name.into(),
                growth: total / mult,
                growth_multiplier: mult,
                campaign_bonus_pct: bonus,
                passive_per_hour: moai,
                food_per_feeding: food,
                gold_dragon_per_feeding: gd,
                target: None,
                in_chamber: true,
                special,
            }
        };
        // Bonuses backed out of the in-game contribution log (file 2) via the
        // contribution formula `(log15(g) − 1.75) · 1.4 · 12 · (1 + bonus/100)`,
        // so this exercises the egg fix + special-pet layer, not the bonus calc.
        let mut pets = vec![
            mk("Bag", 55_678.0, 116.0, 1.0, Some(SpecialPet::Bag { token_improved: true })),
            mk("Hedgehog", 55_775.0, 222.57, 1.0, None),
            mk("Thunder Ball/Raiju", 55_871.0, 480.86, 1.0, None),
            mk("Meteor", 56_066.0, 139.58, 1.0, None),
            mk("Earth Eater", 56_153.0, 131.96, 1.0, None),
            mk("Sphinx", 56_386.0, 119.80, 1.0, None),
            mk("Otter", 57_000.0, 157.14, 1.0, None),
            mk("Cupid", 57_018.0, 231.96, 1.0, None),
            // Pandora carries a Magic Egg → the chamber sets multiplier 1.3.
            mk("Pandora's Box", 57_410.0, 0.0, 1.3, Some(SpecialPet::Pandora { feedings: 16 })),
            mk("Vampire", 57_499.0, 469.86, 1.0, None),
        ];
        let result = simulate_growth_chamber(&mut pets, &ChamberRun { upc_pct: 40.0, ..Default::default() });
        let cyc = &result.trace[0];
        assert_eq!(cyc.recipient, 0, "Bag is the lowest-growth chamber pet");

        let pandora_contrib = cyc.contributions.iter().find(|(i, _)| *i == 8).unwrap().1;
        let base_total: f64 = cyc.contributions.iter().map(|(_, c)| c).sum();
        eprintln!("--- real_growth_campaign reconciliation ---");
        eprintln!("Pandora contribution: {pandora_contrib:.2}  (game 38.57)");
        eprintln!("base total: {base_total:.2}  (game 1098.3)");
        eprintln!("Bag recipient deposit: {:.2}  (game 1678.4)", cyc.recipient_gain);

        // Pandora's own contribution must use its export total (57,410), not
        // 57,410 · 1.3 — i.e. the egg is not double-applied.
        assert!(
            (pandora_contrib - 38.57).abs() < 0.3,
            "Pandora contribution {pandora_contrib:.2}, game 38.57"
        );
        // The full deposit (base · Pandora-boost) lands on the in-game +1678.4.
        assert!(
            (cyc.recipient_gain - 1678.4).abs() < 3.0,
            "Bag deposit {:.2}, game 1678.4",
            cyc.recipient_gain
        );
    }

    #[test]
    fn breakdown_splits_growth_by_source_and_sums_to_gain() {
        let mk = |name: &str, growth: f64| ChamberPet {
            name: name.into(),
            growth,
            growth_multiplier: 1.0,
            campaign_bonus_pct: 0.0,
            passive_per_hour: 2.0,
            food_per_feeding: 5.0,
            gold_dragon_per_feeding: 1.0,
            target: None,
            in_chamber: true,
            special: None,
        };
        let mut pets = vec![mk("Low", 1_000.0), mk("High", 5_000.0)];
        let result = simulate_growth_chamber(&mut pets, &ChamberRun { max_cycles: 3, ..Default::default() });

        let get = |name: &str| result.breakdown.iter().find(|(n, _)| n == name).unwrap().1.clone();
        let (low, high) = (get("Low"), get("High"));

        // Uniform sources over 3 cycles, 4 feedings/cycle (12 h): passive 2·12·3=72,
        // feeding 5·4·3=60, Gold Dragon 1·4·3=12 — for both pets.
        for b in [&low, &high] {
            assert!((b.passive - 72.0).abs() < 1e-6, "passive {}", b.passive);
            assert!((b.feeding - 60.0).abs() < 1e-6, "feeding {}", b.feeding);
            assert!((b.gold_dragon - 12.0).abs() < 1e-6, "gold dragon {}", b.gold_dragon);
        }
        // Only the recipient (Low, lowest growth) gains campaign growth.
        assert!(low.campaign > 0.0, "recipient gains campaign growth");
        assert_eq!(high.campaign, 0.0, "a pure contributor gains none");

        // The four sources sum to each pet's actual growth gain.
        for (name, &start) in [("Low", &1_000.0), ("High", &5_000.0)] {
            let final_g = result.final_growth.iter().find(|(n, _)| n == name).unwrap().1;
            let b = get(name);
            assert!(
                (b.total() - (final_g - start)).abs() < 1e-6,
                "{name}: breakdown {} vs gain {}",
                b.total(),
                final_g - start
            );
        }
    }

    #[test]
    fn skip_first_cycle_passive_drops_exactly_one_round_of_passive() {
        // A lone pet is the recipient and contributes nothing, so its growth comes
        // purely from passive — isolating the toggle's effect.
        let mk = || ChamberPet {
            name: "P".into(),
            growth: 1_000.0,
            growth_multiplier: 1.0,
            campaign_bonus_pct: 0.0,
            passive_per_hour: 3.0,
            food_per_feeding: 0.0,
            gold_dragon_per_feeding: 0.0,
            target: None,
            in_chamber: true,
            special: None,
        };
        // 2 cycles, 12 h, passive 3/h → 36/cycle.
        let mut normal = vec![mk()];
        let mut skipped = vec![mk()];
        let r_normal = simulate_growth_chamber(&mut normal, &ChamberRun { max_cycles: 2, ..Default::default() });
        let r_skip = simulate_growth_chamber(
            &mut skipped,
            &ChamberRun { max_cycles: 2, skip_first_cycle_passive: true, ..Default::default() },
        );

        assert!((r_normal.breakdown[0].1.passive - 72.0).abs() < 1e-6);
        // First cycle's 36 is suppressed; only the second cycle's passive remains.
        assert!((r_skip.breakdown[0].1.passive - 36.0).abs() < 1e-6);
        assert!(
            (r_normal.final_growth[0].1 - r_skip.final_growth[0].1 - 36.0).abs() < 1e-6,
            "skipping drops exactly one cycle of passive from the total"
        );
    }

    #[test]
    fn nightmare_subtracts_its_malus_from_other_pets_only() {
        let mk = |name: &str, growth: f64, bonus: f32, special: Option<SpecialPet>| ChamberPet {
            name: name.into(),
            growth,
            growth_multiplier: 1.0,
            campaign_bonus_pct: bonus,
            passive_per_hour: 0.0,
            food_per_feeding: 0.0,
            gold_dragon_per_feeding: 0.0,
            target: None,
            in_chamber: true,
            special,
        };
        let mut pets = vec![
            mk("R", 1_000.0, 0.0, None),  // recipient (lowest) — contributes 0
            mk("C", 5_000.0, 100.0, None), // contributor — bonus docked by the malus
            mk("N", 6_000.0, 299.0, Some(SpecialPet::Nightmare { class_level: 17 })),
        ];
        let r = simulate_growth_chamber(&mut pets, &ChamberRun::default());
        let cyc = &r.trace[0];
        assert_eq!(cyc.recipient, 0);
        let contrib = |idx: usize| cyc.contributions.iter().find(|(i, _)| *i == idx).unwrap().1;

        // CL 17 → malus 20 − 0.25·17 = 15.75 points.
        let base = |g: f64| (log_base(g, 15.0) - 1.75) * 12.0; // UPC 0
        // C's +100% is reduced to +84.25%; Nightmare's own +299% is untouched.
        assert!((contrib(1) - base(5_000.0) * 1.8425).abs() < 1e-4, "C {}", contrib(1));
        assert!((contrib(2) - base(6_000.0) * 3.99).abs() < 1e-4, "N {}", contrib(2));
    }

    #[test]
    fn nightmare_malus_floors_at_one_point() {
        // 20 − 0.25·CL would go negative past CL 80; it floors at 1 point.
        assert_eq!(nightmare_malus(17), 15.75);
        assert_eq!(nightmare_malus(80), 1.0); // 20 − 20 = 0 → floored
        assert_eq!(nightmare_malus(200), 1.0);
    }

    #[test]
    fn rebirth_schedule_fits_cycles_into_a_rebirth() {
        assert_eq!(rebirth_schedule(12, 20), vec![12, 8]);
        assert_eq!(rebirth_schedule(12, 24), vec![12, 12]);
        assert_eq!(rebirth_schedule(12, 36), vec![12, 12, 12]);
        assert_eq!(rebirth_schedule(12, 13), vec![12, 1]);
        // Rebirth shorter than a cycle clamps the cycle to the rebirth.
        assert_eq!(rebirth_schedule(12, 8), vec![8]);
        assert_eq!(rebirth_schedule(12, 1), vec![1]);
        // A cycle over the 12 h campaign cap is clamped to 12.
        assert_eq!(rebirth_schedule(15, 40), vec![12, 12, 12, 4]);
    }

    #[test]
    fn rebirth_shortens_the_last_cycle_of_each_rebirth() {
        // A lone pet (recipient, contributes nothing) so growth = passive only,
        // making the per-cycle hours directly observable.
        let mk = || ChamberPet {
            name: "P".into(),
            growth: 1_000.0,
            growth_multiplier: 1.0,
            campaign_bonus_pct: 0.0,
            passive_per_hour: 1.0,
            food_per_feeding: 0.0,
            gold_dragon_per_feeding: 0.0,
            target: None,
            in_chamber: true,
            special: None,
        };
        // 20 h rebirth, 12 h cycles → schedule [12, 8] repeating. 4 cycles = two
        // rebirths: 12 + 8 + 12 + 8 = 40 h total.
        let mut pets = vec![mk()];
        let r = simulate_growth_chamber(
            &mut pets,
            &ChamberRun { max_cycles: 4, rebirth_hours: Some(20), ..Default::default() },
        );
        let cycle_hours: Vec<u32> = r.trace.iter().map(|c| c.hours).collect();
        assert_eq!(cycle_hours, vec![12, 8, 12, 8]);
        // Passive = 1/h over 40 h.
        assert!((r.breakdown[0].1.passive - 40.0).abs() < 1e-6, "{}", r.breakdown[0].1.passive);
    }

    // Recipient + Pandora, both with no passive/feeding *growth* so Pandora's
    // growth (and thus the per-5k term) stays fixed — isolating the feeding count.
    // The implied Pandora % each cycle is `recipient_gain / base − 1`.
    fn pandora_accumulation_pets(start: u32) -> Vec<ChamberPet> {
        let mk = |name: &str, growth: f64, special: Option<SpecialPet>| ChamberPet {
            name: name.into(),
            growth,
            growth_multiplier: 1.0,
            campaign_bonus_pct: 0.0,
            passive_per_hour: 0.0,
            food_per_feeding: 0.0,
            gold_dragon_per_feeding: 0.0,
            target: None,
            in_chamber: true,
            special,
        };
        vec![
            mk("R", 1_000.0, None), // recipient (lowest) — contributes 0
            mk("P", 50_000.0, Some(SpecialPet::Pandora { feedings: start })),
        ]
    }

    #[test]
    fn pandora_feedings_accumulate_toward_the_cap() {
        let mut pets = pandora_accumulation_pets(0);
        let r = simulate_growth_chamber(&mut pets, &ChamberRun { max_cycles: 3, ..Default::default() });
        // 12 h → 4 feedings/cycle; the count used each cycle is 0, then 4, then 8.
        for (cyc, fed) in [(0usize, 0u32), (1, 4), (2, 8)] {
            let c = &r.trace[cyc];
            let base: f64 = c.contributions.iter().map(|(_, a)| a).sum();
            let implied = (c.recipient_gain / base - 1.0) * 100.0;
            let expected = pandora_pct(50_000.0, fed);
            assert!((implied - expected).abs() < 0.01, "cycle {cyc}: {implied} vs {expected}");
        }
    }

    #[test]
    fn pandora_feedings_reset_each_rebirth() {
        // 20 h rebirth, 12 h cycles → schedule [12, 8] (len 2): cycle 2 starts a new
        // rebirth, so its feeding count resets to 0 (matching cycle 0).
        let mut pets = pandora_accumulation_pets(0);
        let r = simulate_growth_chamber(
            &mut pets,
            &ChamberRun { max_cycles: 3, rebirth_hours: Some(20), ..Default::default() },
        );
        let implied = |c: &ChamberCycle| {
            let base: f64 = c.contributions.iter().map(|(_, a)| a).sum();
            (c.recipient_gain / base - 1.0) * 100.0
        };
        assert!((implied(&r.trace[0]) - pandora_pct(50_000.0, 0)).abs() < 0.01);
        assert!((implied(&r.trace[1]) - pandora_pct(50_000.0, 4)).abs() < 0.01, "12 h cycle fed 4");
        // Rebirth boundary: back to 0, not 6.
        assert!((implied(&r.trace[2]) - pandora_pct(50_000.0, 0)).abs() < 0.01, "reset on rebirth");
    }

    #[test]
    fn fishing_boost_matches_the_tooltip() {
        // 1e6 FP → FP^0.25 = 31.62%; level 14 has no milestone (×1.0).
        assert!((fishing_boost_pct(1e6, 14) - 31.6228).abs() < 0.01);
        // Milestones at 15 and 27 add +0.1× each (1.1, then 1.2).
        assert!((fishing_boost_pct(1e6, 15) - 31.6228 * 1.1).abs() < 0.01);
        assert!((fishing_boost_pct(1e6, 27) - 31.6228 * 1.2).abs() < 0.01);
        // No fish power → nothing; capped at 200%.
        assert_eq!(fishing_boost_pct(0.0, 30), 0.0);
        assert_eq!(fishing_boost_pct(1e12, 27), 200.0); // 1000 × 1.2 → capped
        // The tooltip's worked example: 12 h into a rebirth, 31.6% → 18.96%.
        assert!((31.6228 * fishing_decay(12.0) - 18.9737).abs() < 0.01);
    }

    #[test]
    fn fishing_boost_decays_over_the_rebirth() {
        // Lone pet (recipient, no campaign), so growth is feeding only and the
        // boost is directly observable. 6 h cycles, 30 h rebirth → [6×5]; midpoints
        // 3/9/15/21/27 h → decay 0.9/0.7/0.5/0.3/0.1. Feeding/cycle = 2 × food 10.
        let mk = || ChamberPet {
            name: "P".into(),
            growth: 1_000.0,
            growth_multiplier: 1.0,
            campaign_bonus_pct: 0.0,
            passive_per_hour: 0.0,
            food_per_feeding: 10.0,
            gold_dragon_per_feeding: 0.0,
            target: None,
            in_chamber: true,
            special: None,
        };
        // With a 100% boost, feeding/cycle = 20 × (1 + decay); summed = 20 × 7.5 = 150.
        let mut on = vec![mk()];
        let r_on = simulate_growth_chamber(
            &mut on,
            &ChamberRun {
                hours: 6,
                max_cycles: 5,
                rebirth_hours: Some(30),
                fishing_boost_pct: 100.0,
                ..Default::default()
            },
        );
        assert!((r_on.breakdown[0].1.feeding - 150.0).abs() < 1e-6, "{}", r_on.breakdown[0].1.feeding);
        // No boost → flat 20 × 5 = 100.
        let mut off = vec![mk()];
        let r_off = simulate_growth_chamber(
            &mut off,
            &ChamberRun { hours: 6, max_cycles: 5, rebirth_hours: Some(30), ..Default::default() },
        );
        assert!((r_off.breakdown[0].1.feeding - 100.0).abs() < 1e-6);
        // Fishing only applies with rebirths modelled (None ⭢ no boost).
        let mut norb = vec![mk()];
        let r_norb = simulate_growth_chamber(
            &mut norb,
            &ChamberRun { hours: 6, max_cycles: 5, fishing_boost_pct: 100.0, ..Default::default() },
        );
        assert!((r_norb.breakdown[0].1.feeding - 100.0).abs() < 1e-6, "no rebirths ⭢ no fishing");
    }
}
