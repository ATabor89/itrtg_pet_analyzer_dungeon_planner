use std::collections::{BTreeMap, HashMap};

use itrtg_models::{
    CampaignBonusRules, CampaignInputs, CampaignType, Class, ElementalForm, Element, Equipment,
    ExportPet, MAGIC_EGG_GROWTH_MULT, Quality, RecommendedClass, WikiPet, resolve_wiki_name,
};

use crate::growth::GrowthRates;

/// Whether an unevolved pet meets its evolution *growth* threshold (the other
/// requirements — material, special condition — are not considered here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvoReadiness {
    /// Base growth already meets the threshold — can evolve now.
    Ready,
    /// Below the threshold on base growth, but a Magic Egg's +30% would reach it
    /// (only possible for total-growth thresholds, not base-growth ones).
    ReadyWithEgg,
    /// Below the threshold even with a Magic Egg.
    NotYet,
}

/// The growth all elemental pets must reach (at their final form) to evolve.
pub const ELEMENTAL_EVO_GROWTH: i64 = 55_555;

/// Per-form **minimum growth** for an elemental pet. `min_growth[i]` is the
/// growth you need while at form `base_form + i` so that the remaining (fixed)
/// form-upgrade growth gains land you exactly on [`ELEMENTAL_EVO_GROWTH`] at the
/// final form; the last entry is therefore always 55,555. Wiki data, player-
/// provided 2026-06. Aether has no forms (his growth comes from challenge points
/// + Delirious-Essence fights — see [`MergedPet::aether_evo_plan`]).
struct ElementalEvo {
    base_form: u32,
    min_growth: &'static [i64],
}

fn elemental_evo(name: &str) -> Option<ElementalEvo> {
    Some(match name {
        // Gnome's base form is V1 (not V0).
        "Gnome" => ElementalEvo { base_form: 1, min_growth: &[-4_439, 12_226, 32_224, 55_555] },
        "Salamander" => {
            ElementalEvo { base_form: 0, min_growth: &[-440, 18_559, 31_558, 44_056, 55_555] }
        }
        "Sylph" => {
            ElementalEvo { base_form: 0, min_growth: &[-440, 6_559, 16_558, 30_556, 55_555] }
        }
        "Undine" => ElementalEvo {
            base_form: 0,
            min_growth: &[-11_111, -1_112, 8_890, 22_222, 38_887, 55_555],
        },
        _ => return None,
    })
}

/// Form-evolution plan for an unevolved elemental pet (Gnome/Salamander/Sylph/
/// Undine) at its current form. Tells the player whether they're on track to be
/// evolve-ready (≥55,555 growth) by the time they reach their final form, given
/// the fixed growth each remaining form upgrade grants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementalEvoPlan {
    /// Current form version (the "V" number).
    pub form: u32,
    /// Current base growth.
    pub current_growth: i64,
    /// Minimum growth to be at *this* form to stay on track.
    pub min_growth_for_form: i64,
    /// Total growth the remaining form upgrades will still grant (0 at final).
    pub remaining_form_gain: i64,
    /// Growth at the final form if you upgraded now without growing more.
    pub projected_final_growth: i64,
    /// `current_growth >= min_growth_for_form` (at the final form this means
    /// already ≥55,555 = evolve-ready).
    pub on_track: bool,
    /// Growth still needed to reach this form's minimum (0 when on track).
    pub shortfall: i64,
    /// Whether this is the final form (where the 55,555 evolve bar applies).
    pub is_final_form: bool,
}

/// Aether's evolution outlook. He has no forms: his growth derives from Total
/// Challenge Points (start = −10·CHP) and grows by `0.2·CHP` per Delirious-
/// Essence fight (up to 50). Evolution needs 55,555 growth, 2,778 Iron Bars, and
/// 25 fights. The CHP estimate is a rough indicator (it assumes growth is
/// otherwise static and ignores that CHP also lowers his starting growth).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AetherEvoPlan {
    pub current_growth: i64,
    pub challenge_points: u64,
    pub fights_done: u32,
    /// Growth each further fight grants: `0.2 · CHP`.
    pub growth_per_fight: f64,
    /// Fights left before the 50-fight growth cap.
    pub fights_remaining: u32,
    /// Rough CHP needed for the remaining fights to bridge current growth →
    /// 55,555: `(55555 − growth) / (fights_remaining · 0.2)`. `None` once the
    /// growth bar is already met or no growth-granting fights remain.
    pub chp_to_evolve_estimate: Option<f64>,
}

/// Runtime context for computing effective campaign bonuses.
///
/// Carries the curated bonus rules and the full roster (for export-derived
/// formulas like Bag's lowest-growth and Lizard's pet counts). A future phase
/// will add persisted user-input values here (pet stones, challenge points,
/// Delirious-Essence fights, …) — extending this rather than the seam signature.
pub struct CampaignContext<'a> {
    pub bonuses: &'a CampaignBonusRules,
    pub roster: &'a [MergedPet],
    pub inputs: &'a CampaignInputs,
    /// Add the pet's campaign-boost *equipment* (sticks) on top of its innate
    /// bonus. Off by default — planning around innate bonuses is more durable.
    pub include_equipment: bool,
    /// Add the pet's *class* campaign bonus (Adventurer's 2% · CL, plus any
    /// Adventurer evo bonus) on top of its innate bonus. Off by default.
    pub include_class: bool,
}

/// A pet's effective campaign bonus split by source, so the UI can show where
/// each total comes from. Produced by [`MergedPet::campaign_bonus_breakdown`];
/// [`Self::total`] reassembles the per-campaign totals the rest of the app uses.
#[derive(Debug, Clone, Default)]
pub struct CampaignBonusBreakdown {
    /// The pet's own per-campaign values: the curated rules in
    /// `campaign_bonuses.yaml` plus its dynamic formulas (Bag, Mermaid, …).
    pub innate: BTreeMap<CampaignType, f32>,
    /// Flat all-campaign boost from equipped campaign gear (sticks, event
    /// items). `None` when the context's equipment layer is off or the loadout
    /// has no contributing gear.
    pub equipment: Option<f32>,
    /// Flat all-campaign Adventurer class bonus. `None` when the context's
    /// class layer is off or the pet isn't currently an Adventurer.
    pub class: Option<f32>,
}

impl CampaignBonusBreakdown {
    /// Per-campaign totals: innate plus the flat layers, which apply to every
    /// campaign (so an active layer creates entries even where innate has none).
    pub fn total(&self) -> BTreeMap<CampaignType, f32> {
        let mut map = self.innate.clone();
        if self.equipment.is_some() || self.class.is_some() {
            let flat = self.equipment.unwrap_or(0.0) + self.class.unwrap_or(0.0);
            for c in CampaignType::ALL {
                *map.entry(c).or_insert(0.0) += flat;
            }
        }
        map
    }
}

/// Round a percentage to 2 decimal places. Dynamic campaign formulas produce
/// long fractions (e.g. `growth^0.4`); the game displays two decimals.
fn round2(x: f64) -> f32 {
    ((x * 100.0).round() / 100.0) as f32
}

/// The elemental pets that count toward Aether's "elementals unlocked" term.
/// Per the wiki source, this *includes* Aether itself.
fn is_elemental(name: &str) -> bool {
    matches!(name, "Undine" | "Gnome" | "Salamander" | "Sylph" | "Elemental" | "Aether")
}

/// Pets whose campaign bonus is (at least partly) computed in code — the match
/// arms of `apply_campaign_formulas`. Keep in sync with that match; the
/// campaign-bonus coverage test uses this list to know a pet is handled even
/// without a static entry in `data/campaign_bonuses.yaml`.
pub const CAMPAIGN_FORMULA_PETS: &[&str] = &[
    "Bag",
    "Mermaid",
    "Lizard/Zookeeper",
    "Beachball",
    "Unicorn",
    "Bear",
    "Ant Queen",
    "Aether",
    "Earth Eater",
    "Meteor",
    "Goblin",
    "Stone/Golem",
    "Cupid",
];

/// Extra Adventurer campaign bonus per class level, for pets with an Adventurer
/// evo bonus — added to the base 2%/CL. Keyed by canonical pet name. (The
/// in-game "Adventurer pet" entry has no match in the data and is omitted.)
const ADVENTURER_EVO_BONUS: &[(&str, f32)] = &[
    ("Pandora's Box", 0.9),
    ("Sphinx", 0.68),
    ("Earth Eater", 1.32),
    ("Meteor", 0.85),
    ("Thunder Ball/Raiju", 1.3),
    ("Hedgehog", 0.58),
    ("Bag", 1.0),
    ("Cupid", 0.5),
    ("Otter", 0.8),
    ("Anni Cake", 1.38),
    ("Ant Queen", 2.0),
    ("Aether", 1.5),
    ("Decorator Crab", 1.75),
    ("Nightmare", 0.9),
    ("Unicorn", 1.2),
    ("FSM", 0.85),
    ("Skeleton", 0.83),
    ("Seed/Yggdrasil", 1.7),
    ("Chocobear", 0.56),
    ("Llysnafedda", 0.65),
    ("Hydra", 0.7),
    ("UFO", 0.7),
    ("Serow", 0.65),
    ("Bug", 0.5),
    ("Camel", 0.51),
    ("God Power (Pet)", 0.53),
    ("Eagle", 0.52),
    ("Mole", 0.51),
    ("Lizard/Zookeeper", 0.9),
    ("Beachball", 0.68),
    ("Portal", 1.0),
    ("Afky Clone", 0.6),
    ("Wolf", 1.0),
    ("Oni", 2.0),
    ("Big Burger", 2.0),
    ("Flying Eyeball", 0.85),
    ("Holy ITRTG Book", 0.4),
    ("Tenko", 1.1),
    ("Bear", 0.75),
    ("Living Draw", 0.8),
    ("Goblin", 0.1),
    ("Sloth", 1.25),
    ("Nugget", 0.3),
    ("Dorgegebelle", 1.7),
];

/// The Adventurer evo bonus (% per CL) for a pet, or 0 if it has none.
fn adventurer_evo_bonus(name: &str) -> f32 {
    ADVENTURER_EVO_BONUS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, b)| *b)
        .unwrap_or(0.0)
}

/// The all-campaign boost from one equipped item (a stick or a known event
/// item), if it provides one.
fn item_campaign_bonus(item: &Equipment) -> Option<f32> {
    stick_bonus(item).or_else(|| event_equip_bonus(item))
}

/// A campaign stick's boost. The four sticks share
/// `value = cap · (rank/9) · ((1+upgrade)/21)` (so an SSS+20 hits the cap
/// exactly; the `.min(cap)` is a guard for malformed levels above +20).
fn stick_bonus(item: &Equipment) -> Option<f32> {
    let cap = match item.name.as_str() {
        "Walking Stick" => 50.0 / 3.0,     // 16.67%
        "Journeying Stick" => 100.0 / 3.0, // 33.33%
        "Magic Stick" => 50.0,
        "Legendary Stick" => 100.0,
        _ => return None,
    };
    let rank = item.quality.campaign_rank() as f64;
    let upgrade = item.upgrade_level.unwrap_or(0) as f64;
    Some(round2((cap * (rank / 9.0) * ((1.0 + upgrade) / 21.0)).min(cap)))
}

/// Known SSS+20 campaign boosts for event equipment. These have no published
/// formula, so only the as-purchased SSS+20 level is handled; other levels
/// return `None` rather than guess.
fn event_equip_bonus(item: &Equipment) -> Option<f32> {
    if item.quality != Quality::SSS || item.upgrade_level != Some(20) {
        return None;
    }
    match item.name.as_str() {
        "Candy Cane" => Some(101.0),
        "Merry Mantle" => Some(150.0),
        "Christmas Boots" => Some(150.0),
        _ => None,
    }
}

/// A pet with wiki reference data merged with the player's actual game data.
///
/// Either side can be missing:
/// - `wiki` is None if the pet exists in-game but isn't on the wiki yet
/// - `export` is None if the pet is on the wiki but not in the player's export
#[derive(Debug, Clone)]
pub struct MergedPet {
    /// Canonical display name (from wiki if available, otherwise resolved from export).
    pub name: String,
    pub wiki: Option<WikiPet>,
    pub export: Option<ExportPet>,
}

impl MergedPet {
    /// The pet's element (prefers export as ground truth, falls back to wiki).
    pub fn element(&self) -> Option<Element> {
        self.export
            .as_ref()
            .map(|e| e.element)
            .or_else(|| self.wiki.as_ref().map(|w| w.element))
    }

    /// The pet's actual evolved class (from export). None if unevolved or no export data.
    pub fn evolved_class(&self) -> Option<Class> {
        self.export.as_ref().and_then(|e| e.class)
    }

    /// The elemental pet's evolved **form** (from the export "Other" column,
    /// e.g. `GnomeV2`). `None` for non-elemental pets or without export data.
    pub fn elemental_form(&self) -> Option<ElementalForm> {
        self.export.as_ref().and_then(|e| e.elemental_form())
    }

    /// Whether the pet is evolved (has a class assigned).
    pub fn is_evolved(&self) -> bool {
        self.evolved_class().is_some()
    }

    /// Whether the pet is unlocked in-game.
    pub fn is_unlocked(&self) -> bool {
        self.export.as_ref().is_some_and(|e| e.unlocked)
    }

    /// The wiki's recommended class, if we have wiki data.
    pub fn recommended_class(&self) -> Option<&RecommendedClass> {
        self.wiki.as_ref().map(|w| &w.recommended_class)
    }

    /// Whether this pet is a village pet (class doesn't matter for dungeons).
    pub fn is_village_pet(&self) -> bool {
        matches!(
            self.recommended_class(),
            Some(RecommendedClass::Village(_))
        )
    }

    /// Check if a given class is among this pet's recommended classes.
    /// Returns true for Single match, either side of a Dual, AllClasses, DungeonWildcard
    /// (for dungeon classes), and Wildcard. Returns false for Village and Special/Alternates.
    pub fn recommends_class(&self, target: &Class) -> bool {
        match self.recommended_class() {
            Some(RecommendedClass::Single(c)) => c == target,
            Some(RecommendedClass::Dual(a, b)) => {
                a == target || b == target
                    || *a == Class::Wildcard
                    || *b == Class::Wildcard
            }
            Some(RecommendedClass::AllClasses) => true,
            Some(RecommendedClass::DungeonWildcard) => matches!(
                target,
                Class::Defender
                    | Class::Supporter
                    | Class::Rogue
                    | Class::Assassin
                    | Class::Mage
            ),
            Some(RecommendedClass::Wildcard) => true,
            Some(RecommendedClass::Village(_)) => false,
            Some(RecommendedClass::Special) => false,
            Some(RecommendedClass::Alternates) => false,
            None => false,
        }
    }

    /// How close an unevolved pet is to meeting its evolution *growth*
    /// threshold. Returns `None` when readiness doesn't apply: the pet is
    /// already evolved or has no scraped evolution data.
    ///
    /// Works for *unowned* pets too — the export carries their growth, and
    /// knowing a locked pet is already evolve-ready (or how close it is) helps
    /// decide what to unlock next. Filtering controls whether they're shown.
    ///
    /// For total-growth thresholds the Magic Egg's +30% can count toward
    /// reaching the bar; for base-growth thresholds (Baby Carno) it cannot, so
    /// `ReadyWithEgg` is never produced for those.
    pub fn evo_readiness(&self) -> Option<EvoReadiness> {
        let export = self.export.as_ref()?;
        // Already-evolved pets have no readiness; locked pets still do.
        if export.class.is_some() {
            return None;
        }
        let req = self.wiki.as_ref()?.evo_requirements.as_ref()?;
        let threshold = req.growth.value();

        // Base-growth thresholds (Baby Carno) ignore the Magic Egg entirely:
        // only true base growth counts.
        if !req.growth.magic_egg_counts() {
            return Some(if export.growth as i64 >= threshold {
                EvoReadiness::Ready
            } else {
                EvoReadiness::NotYet
            });
        }

        // Total-growth thresholds: "ready now" uses the pet's *current* growth,
        // which already includes the egg's boost if one is equipped (export
        // growth is stored as true base). Otherwise, see whether equipping an
        // egg would clear the bar.
        if export.effective_growth() as i64 >= threshold {
            Some(EvoReadiness::Ready)
        } else if export.growth_with_magic_egg() as i64 >= threshold {
            Some(EvoReadiness::ReadyWithEgg)
        } else {
            Some(EvoReadiness::NotYet)
        }
    }

    /// Estimated hours to grow this pet's base growth to its evolution
    /// threshold, via a dedicated pendant + Moai. `None` when not applicable
    /// (already evolved or no evo data) or the threshold is unreachable with
    /// these tools. `Some(0.0)` if already met. Computed for unowned pets too
    /// (a planning aid for what to unlock next).
    ///
    /// With `use_egg`, a *total*-growth threshold is reached at base =
    /// threshold / 1.3 (the egg covers the rest). A *base*-growth threshold
    /// (Baby Carno) ignores the egg entirely, so `use_egg` makes no difference
    /// for it — this is what keeps its estimate honest rather than falsely short.
    ///
    /// With `use_egg = false`, this is the honest base-growth grind time and
    /// orders pets Ready (0) → ReadyWithEgg (small) → NotYet (large).
    pub fn hours_to_evolve(&self, rates: &GrowthRates, use_egg: bool) -> Option<f64> {
        let export = self.export.as_ref()?;
        if export.class.is_some() {
            return None;
        }
        let req = self.wiki.as_ref()?.evo_requirements.as_ref()?;
        let threshold = req.growth.value().max(0) as u64;
        // The egg only discounts total-growth thresholds; base-growth pets gain
        // nothing, so never discount them (or we'd report a false, too-short time).
        let target = if use_egg && req.growth.magic_egg_counts() {
            (threshold as f64 / MAGIC_EGG_GROWTH_MULT).ceil() as u64
        } else {
            threshold
        };
        rates.hours_to_target(export.growth, target)
    }

    /// Estimated hours to grow this pet's base growth to an arbitrary `target`,
    /// via a dedicated pendant + Moai. `None` when there's no export data or the
    /// target is unreachable. Applies to any pet — evolved or not, owned or not.
    pub fn hours_to_growth(&self, target: u64, rates: &GrowthRates) -> Option<f64> {
        let export = self.export.as_ref()?;
        rates.hours_to_target(export.growth, target)
    }

    /// Form-evolution plan for an unevolved elemental pet at its current form.
    /// `None` for non-elemental pets, already-evolved pets, pets without export/
    /// form data, or an unrecognized form version.
    pub fn elemental_evo_plan(&self) -> Option<ElementalEvoPlan> {
        let export = self.export.as_ref()?;
        if export.class.is_some() {
            return None; // evolved → past the form progression
        }
        let form = self.elemental_form()?.version;
        let evo = elemental_evo(&self.name)?;
        let idx = form.checked_sub(evo.base_form)? as usize;
        let &min_growth_for_form = evo.min_growth.get(idx)?;
        let current_growth = export.growth as i64;
        let remaining_form_gain = ELEMENTAL_EVO_GROWTH - min_growth_for_form;
        Some(ElementalEvoPlan {
            form,
            current_growth,
            min_growth_for_form,
            remaining_form_gain,
            projected_final_growth: current_growth + remaining_form_gain,
            on_track: current_growth >= min_growth_for_form,
            shortfall: (min_growth_for_form - current_growth).max(0),
            is_final_form: idx + 1 == evo.min_growth.len(),
        })
    }

    /// Aether's evolution outlook from the player's Total Challenge Points and
    /// Delirious-Essence fight count. `None` unless this pet is Aether, unevolved,
    /// and has export data. See [`AetherEvoPlan`] for the caveats on the estimate.
    pub fn aether_evo_plan(&self, challenge_points: u64, fights_done: u32) -> Option<AetherEvoPlan> {
        if self.name != "Aether" {
            return None;
        }
        let export = self.export.as_ref()?;
        if export.class.is_some() {
            return None;
        }
        let current_growth = export.growth as i64;
        let growth_per_fight = 0.2 * challenge_points as f64;
        let fights_remaining = 50u32.saturating_sub(fights_done);
        let chp_to_evolve_estimate = (current_growth < ELEMENTAL_EVO_GROWTH
            && fights_remaining > 0)
            .then(|| {
                (ELEMENTAL_EVO_GROWTH - current_growth) as f64 / (fights_remaining as f64 * 0.2)
            });
        Some(AetherEvoPlan {
            current_growth,
            challenge_points,
            fights_done,
            growth_per_fight,
            fights_remaining,
            chp_to_evolve_estimate,
        })
    }

    /// This pet's effective per-campaign bonus percentages — **the single entry
    /// point** the UI uses for display, filtering, and sorting campaign bonuses.
    ///
    /// Static values come entirely from the curated rules in
    /// `data/campaign_bonuses.yaml` (via `ctx.bonuses`), conditioned on the
    /// pet's export state (token boosts like Hedgehog, evolution flips like
    /// Nothing). Export/user-input formulas (Bag, Mermaid, Beachball) are
    /// layered on top in code. Callers go through this method, so they won't
    /// change when the numbers get richer.
    pub fn campaign_bonuses(&self, ctx: &CampaignContext) -> BTreeMap<CampaignType, f32> {
        self.campaign_bonus_breakdown(ctx).total()
    }

    /// The same effective bonuses as [`Self::campaign_bonuses`], but split by
    /// source (innate / equipment / class) so the UI can show the full picture.
    /// `campaign_bonuses` is this breakdown's `total()`, so the two can never
    /// disagree.
    pub fn campaign_bonus_breakdown(&self, ctx: &CampaignContext) -> CampaignBonusBreakdown {
        let mut innate = BTreeMap::new();
        let improved = self.export.as_ref().is_some_and(|e| e.improved);
        let form = self.elemental_form().map(|f| f.version);
        ctx.bonuses
            .apply_with_form(&self.name, &mut innate, self.is_evolved(), improved, form);
        self.apply_campaign_formulas(&mut innate, ctx);

        // Optional equipment layer — campaign-boost gear across all three slots,
        // additive to all campaigns.
        let equipment = if ctx.include_equipment {
            self.export.as_ref().and_then(|export| {
                let total: f32 = [
                    &export.loadout.weapon,
                    &export.loadout.armor,
                    &export.loadout.accessory,
                ]
                .into_iter()
                .flatten()
                .filter_map(item_campaign_bonus)
                .sum();
                (total != 0.0).then_some(total)
            })
        } else {
            None
        };

        // Optional class layer — Adventurer's campaign bonus, additive to all.
        let class = if ctx.include_class { self.class_campaign_bonus(ctx) } else { None };

        CampaignBonusBreakdown { innate, equipment, class }
    }

    /// The all-campaign bonus from an Adventurer's class, or `None` if the pet
    /// isn't currently an Adventurer. The rate is `(2 + evo)% · CL`, where `evo`
    /// is the pet's Adventurer evo bonus from `ADVENTURER_EVO_BONUS` (0 for most
    /// pets; e.g. Hedgehog +0.58 → 56.76% at CL22).
    fn class_campaign_bonus(&self, ctx: &CampaignContext) -> Option<f32> {
        let export = self.export.as_ref()?;
        let per_level = self.adventurer_per_level_bonus(ctx)? as f64;
        Some(round2(per_level * export.class_level as f64))
    }

    /// The Adventurer campaign-bonus rate **per class level** (%), or `None` if
    /// the pet isn't currently an Adventurer. `2 + evo`, where `evo` is the
    /// pet's Adventurer evo bonus from `ADVENTURER_EVO_BONUS` (0 for most pets;
    /// Goblin's is dynamic with Overflow Challenges). The full class bonus is
    /// this × class level — see [`Self::class_campaign_bonus`]. The Growth
    /// Chamber sim uses this as the per-level slope when a pet gains class levels
    /// mid-run.
    pub fn adventurer_per_level_bonus(&self, ctx: &CampaignContext) -> Option<f32> {
        let export = self.export.as_ref()?;
        if export.class != Some(Class::Adventurer) {
            return None;
        }
        // Base 2%/CL for any Adventurer, plus the pet's Adventurer evo bonus.
        let mut evo = adventurer_evo_bonus(&self.name) as f64;
        // Goblin's evo bonus is dynamic: its 0.1 base climbs with Overflow
        // Challenges — the first 100 add 0.008 each, 101..=470 add 0.001622 each
        // (an empirical fit; it slightly overshoots, landing at ~1.50014),
        // reaching the documented full 1.5 at the 470 cap.
        if self.name == "Goblin" {
            let oc = ctx.inputs.goblin_oc;
            evo += (oc.min(100) as f64) * 0.008
                + (oc.saturating_sub(100).min(370) as f64) * 0.001622;
        }
        Some((2.0 + evo) as f32)
    }

    /// Apply per-pet campaign formulas — bespoke math a static rule can't
    /// express, computed from the roster, the pet's own export, and the
    /// player-entered `ctx.inputs` (pet stones, fights, …). No-op for pets
    /// without a formula. Match arms are mirrored in [`CAMPAIGN_FORMULA_PETS`].
    ///
    /// The Bag/Lizard arms walk the roster; only those two pets pay it, so it's
    /// negligible at the current roster size, but worth precomputing the
    /// aggregates into the context if more roster-scanning formulas are added.
    fn apply_campaign_formulas(
        &self,
        map: &mut BTreeMap<CampaignType, f32>,
        ctx: &CampaignContext,
    ) {
        match self.name.as_str() {
            // Bag: lowest *unlocked* pet's growth ^ 0.4, capped at 100%, to Growth.
            "Bag" => {
                if let Some(lowest) = ctx
                    .roster
                    .iter()
                    .filter(|p| p.is_unlocked())
                    .filter_map(|p| p.export.as_ref())
                    .map(|e| e.growth)
                    .min()
                {
                    let v = round2((lowest as f64).powf(0.4).min(100.0));
                    map.insert(CampaignType::Growth, v);
                }
            }
            // Mermaid: -(own growth / 1000)% to all campaigns, capped at -333%.
            "Mermaid" => {
                if let Some(g) = self.export.as_ref().map(|e| e.growth) {
                    let v = round2((-(g as f64 / 1000.0)).max(-333.0));
                    for c in CampaignType::ALL {
                        map.insert(c, v);
                    }
                }
            }
            // Lizard: (unlocked + evolved pets) ^ 0.5 * 10, capped at 100%. The
            // bonus is to Growth before evolving, Food after.
            "Lizard/Zookeeper" => {
                let unlocked = ctx.roster.iter().filter(|p| p.is_unlocked()).count();
                let evolved = ctx.roster.iter().filter(|p| p.is_evolved()).count();
                let v = round2((((unlocked + evolved) as f64).sqrt() * 10.0).min(100.0));
                let target = if self.is_evolved() {
                    CampaignType::Food
                } else {
                    CampaignType::Growth
                };
                map.insert(target, v);
            }
            // Beachball: sqrt(stones^1.00001 - stones) * 2, capped at 200%, to
            // all campaigns. Stones = held + those given to (locked into) it.
            "Beachball" => {
                let s = (ctx.inputs.pet_stones + ctx.inputs.beachball_given_stones) as f64;
                let v = round2((((s.powf(1.00001) - s).max(0.0)).sqrt() * 2.0).min(200.0));
                for c in CampaignType::ALL {
                    map.insert(c, v);
                }
            }
            // Unicorn: sqrt(challenge points) / 2, cap 100, to growth/godpower/divinity.
            "Unicorn" => {
                let v = round2(((ctx.inputs.challenge_points as f64).sqrt() / 2.0).min(100.0));
                for c in [CampaignType::Growth, CampaignType::GodPower, CampaignType::Divinity] {
                    map.insert(c, v);
                }
            }
            // Bear: honey given / 500, cap 100, to all campaigns.
            "Bear" => {
                let v = round2(((ctx.inputs.honey as f64) / 500.0).min(100.0));
                for c in CampaignType::ALL {
                    map.insert(c, v);
                }
            }
            // Ant Queen: ants ^ 0.27, to divinity and god power.
            "Ant Queen" => {
                let v = round2((ctx.inputs.ants as f64).powf(0.27));
                for c in [CampaignType::Divinity, CampaignType::GodPower] {
                    map.insert(c, v);
                }
            }
            // Aether: an all-campaign penalty that shrinks as you beat Delirious
            // Essence (-99% reduced by 10% per fight, maxing at +1% after 10),
            // PLUS an added growth-campaign bonus that scales with fights, the
            // elementals you own (Aether included), and Aether's own growth.
            "Aether" => {
                let fights = ctx.inputs.delirious_essence_fights as f64;
                let penalty = round2((-99.0 + 10.0 * fights).min(1.0));
                for c in CampaignType::ALL {
                    map.insert(c, penalty);
                }
                let elementals = ctx
                    .roster
                    .iter()
                    .filter(|p| p.is_unlocked() && is_elemental(&p.name))
                    .count() as f64;
                // The game floors Aether's growth at 1 for the log (it can be
                // negative); export growth is u64, so 0 maps to 1.
                let growth = self.export.as_ref().map(|e| e.growth).unwrap_or(0).max(1) as f64;
                let growth_bonus = ((elementals + 5.0) / 10.0)
                    * fights
                    * (1.0 + 0.57 * growth.ln() / 1000_f64.ln());
                *map.entry(CampaignType::Growth).or_insert(0.0) += round2(growth_bonus);
            }
            // Earth Eater: a flat all-campaign bonus that ramps -80% -> +82%.
            // Each rebirth it's fed up to the +82% cap in ~1.35h, and the token
            // upgrade only *lowers* the per-rebirth starting penalty (eventually
            // removing it, locking him at +82%) — so his realistic in-play value
            // is +82%. We show that by default. Only when the player opts in
            // (`show_lifetime`) on a token-improved pet with a total entered do we
            // expose the lower permanent value: -80% + 1% per 200k, cap +82%.
            "Earth Eater" => {
                let total = ctx.inputs.earth_eater_total_planets;
                let v = if ctx.inputs.earth_eater_show_lifetime
                    && total > 0
                    && self.export.as_ref().is_some_and(|e| e.improved)
                {
                    round2((-80.0 + total as f64 / 200_000.0).clamp(-80.0, 82.0))
                } else {
                    82.0
                };
                for c in CampaignType::ALL {
                    map.insert(c, v);
                }
            }
            // Meteor: an all-campaign bonus that grows with time spent running
            // campaigns — `25 + hours^0.42`%. Replaces the curated static +25.
            // (e.g. 4501 hours → 25 + 34.23 ≈ 59.23.)
            "Meteor" => {
                let hours = ctx.inputs.meteor_campaign_hours as f64;
                let v = round2(25.0 + hours.powf(0.42));
                for c in CampaignType::ALL {
                    map.insert(c, v);
                }
            }
            // Goblin: +1% to every campaign per UCC completed (capped at 75),
            // stacked on her curated base (-100 growth/item, +150 divinity, +50
            // others). At the cap that base becomes -25 / +225 / +125. Her
            // OC-driven evo bonus is handled in `class_campaign_bonus`.
            "Goblin" => {
                let ucc = ctx.inputs.goblin_ucc.min(75) as f32;
                if ucc != 0.0 {
                    for c in CampaignType::ALL {
                        *map.entry(c).or_insert(0.0) += ucc;
                    }
                }
            }
            // Stone/Golem: evolved it's a flat +100% (from its curated rule).
            // Unevolved it ramps with growth: -100% + 20% per 5000 growth, capped
            // at 0% (25000 growth). The 1500-CP upgrade adds +100% to all
            // campaigns on top of either state.
            "Stone/Golem" => {
                if !self.is_evolved() {
                    let g = self.export.as_ref().map(|e| e.growth).unwrap_or(0) as f64;
                    let v = round2((-100.0 + g / 5000.0 * 20.0).clamp(-100.0, 0.0));
                    for c in CampaignType::ALL {
                        map.insert(c, v);
                    }
                }
                if ctx.inputs.stone_campaign_upgrade {
                    for c in CampaignType::ALL {
                        *map.entry(c).or_insert(0.0) += 100.0;
                    }
                }
            }
            // Cupid: token-improved adds +2% per couple to all campaigns, on top
            // of the curated flat token bonus. A couple is two pets, and a pet can
            // be coupled with itself, so counting "pets with a partner" (the
            // export's `has_partner`) sidesteps the self-couple ambiguity:
            // +1% per partnered pet = +2% per couple. Read straight off the
            // roster — no user input.
            "Cupid" => {
                if self.export.as_ref().is_some_and(|e| e.improved) {
                    let partnered = ctx
                        .roster
                        .iter()
                        .filter(|p| p.export.as_ref().is_some_and(|e| e.has_partner))
                        .count();
                    let bonus = round2(partnered as f64);
                    if bonus != 0.0 {
                        for c in CampaignType::ALL {
                            *map.entry(c).or_insert(0.0) += bonus;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// This pet's effective bonus to a single campaign, if known. `None` when the
    /// pet has no bonus or its bonus wasn't structured (raw-only) — distinct from
    /// `Some(0.0)`. Routes through [`Self::campaign_bonuses`] so it tracks the
    /// curated rules and dynamic adjustments.
    pub fn campaign_bonus_for(&self, campaign: CampaignType, ctx: &CampaignContext) -> Option<f32> {
        self.campaign_bonuses(ctx).get(&campaign).copied()
    }

    /// Whether this pet's element matches the target. `Element::All` (Chameleon) matches
    /// anything, and `None` target means "any element".
    pub fn matches_element(&self, target: Option<Element>) -> bool {
        let Some(target) = target else {
            return true; // "any" element
        };
        match self.element() {
            Some(Element::All) => true,
            Some(el) => el == target,
            None => false,
        }
    }
}

/// Merge wiki pet data with export pet data using name resolution.
///
/// Returns a list of `MergedPet` entries covering the union of both datasets.
pub fn merge_pets(wiki_pets: &[WikiPet], export_pets: &[ExportPet]) -> Vec<MergedPet> {
    // Build a lookup from wiki name → wiki pet
    let wiki_by_name: HashMap<String, &WikiPet> = wiki_pets
        .iter()
        .map(|w| (w.name.clone(), w))
        .collect();

    // Track which wiki pets got matched
    let mut matched_wiki: HashMap<String, bool> = wiki_pets
        .iter()
        .map(|w| (w.name.clone(), false))
        .collect();

    let mut merged = Vec::new();

    // First, process all export pets and try to match them to wiki entries
    for export in export_pets {
        let wiki_name = resolve_wiki_name(&export.export_name);
        let wiki = wiki_by_name.get(&wiki_name).copied().cloned();

        if wiki.is_some() {
            matched_wiki.insert(wiki_name.clone(), true);
        }

        merged.push(MergedPet {
            name: wiki_name,
            wiki,
            export: Some(export.clone()),
        });
    }

    // Then, add wiki-only pets (not present in the player's export)
    for wiki_pet in wiki_pets {
        if !matched_wiki.get(&wiki_pet.name).copied().unwrap_or(false) {
            merged.push(MergedPet {
                name: wiki_pet.name.clone(),
                wiki: Some(wiki_pet.clone()),
                export: None,
            });
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use itrtg_models::*;

    fn make_wiki_pet(name: &str, element: Element, rec_class: RecommendedClass) -> WikiPet {
        WikiPet {
            name: name.to_string(),
            wiki_url: format!("https://itrtg.wiki.gg/wiki/{}", name.replace(' ', "_")),
            element,
            recommended_class: rec_class,
            class_bonus: "0.5% x CL".to_string(),
            unlock_condition: UnlockCondition::PetToken,
            evo_difficulty: EvoDifficulty { base: 1, with_conditions: 1 },
            token_improvable: false,
            special_ability: None,
            evo_requirements: None,
            campaign_bonus: None,
        }
    }

    fn make_export_pet(name: &str, element: Element, class: Option<Class>) -> ExportPet {
        ExportPet {
            export_name: name.to_string(),
            element,
            growth: 1000,
            dungeon_level: 10,
            class,
            class_level: 5,
            class_exp: 0.0,
            combat_stats: CombatStats { hp: 100, attack: 50, defense: 30, speed: 40 },
            elemental_affinities: ElementalAffinities {
                water: 0, fire: 0, wind: 0, earth: 0, dark: 0, light: 0,
            },
            loadout: Loadout { weapon: None, armor: None, accessory: None },
            action: PetAction::Idle,
            unlocked: true,
            improved: false,
            other: None,
            has_partner: false,
        }
    }

    #[test]
    fn test_merge_basic() {
        let wiki = vec![
            make_wiki_pet("Mouse", Element::Earth, RecommendedClass::Wildcard),
            make_wiki_pet("Frog", Element::Water, RecommendedClass::Single(Class::Supporter)),
        ];
        let export = vec![
            make_export_pet("Mouse", Element::Earth, None),
        ];

        let merged = merge_pets(&wiki, &export);
        assert_eq!(merged.len(), 2);

        // Mouse: both wiki and export
        assert!(merged[0].wiki.is_some());
        assert!(merged[0].export.is_some());

        // Frog: wiki only
        assert!(merged[1].wiki.is_some());
        assert!(merged[1].export.is_none());
    }

    #[test]
    fn test_merge_name_resolution() {
        let wiki = vec![
            make_wiki_pet("Egg/Chicken", Element::Wind, RecommendedClass::Single(Class::Assassin)),
        ];
        let export = vec![
            make_export_pet("Egg", Element::Wind, Some(Class::Assassin)),
        ];

        let merged = merge_pets(&wiki, &export);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "Egg/Chicken");
        assert!(merged[0].wiki.is_some());
        assert!(merged[0].export.is_some());
    }

    /// Build a merged pet with a chosen growth, class, unlock state, and
    /// optional evolution growth requirement, for readiness tests.
    fn readiness_pet(
        growth: u64,
        class: Option<Class>,
        unlocked: bool,
        req: Option<GrowthRequirement>,
    ) -> MergedPet {
        let mut wiki = make_wiki_pet("Test", Element::Earth, RecommendedClass::Wildcard);
        wiki.evo_requirements = req.map(|growth| EvoRequirements {
            growth,
            material: None,
            other: None,
        });
        let mut export = make_export_pet("Test", Element::Earth, class);
        export.growth = growth;
        export.unlocked = unlocked;
        MergedPet { name: "Test".to_string(), wiki: Some(wiki), export: Some(export) }
    }

    /// Equip a Magic Egg on the pet's weapon slot (so `effective_growth`
    /// applies the +30%). `growth` is still stored as true base growth.
    fn equip_magic_egg(pet: &mut MergedPet) {
        pet.export.as_mut().unwrap().loadout.weapon = Some(Equipment {
            name: "Magic Egg".to_string(),
            upgrade_level: None,
            quality: Quality::SSS,
            enchant_level: None,
            gem: None,
            gem_level: None,
        });
    }

    #[test]
    fn test_evo_readiness_total_growth() {
        // Total-growth threshold of 1000.
        let req = || Some(GrowthRequirement::Total(1000));
        // base >= threshold → Ready
        assert_eq!(readiness_pet(1000, None, true, req()).evo_readiness(), Some(EvoReadiness::Ready));
        // base 800 → 800*1.3 = 1040 >= 1000 → ReadyWithEgg
        assert_eq!(readiness_pet(800, None, true, req()).evo_readiness(), Some(EvoReadiness::ReadyWithEgg));
        // base 700 → 910 < 1000 → NotYet
        assert_eq!(readiness_pet(700, None, true, req()).evo_readiness(), Some(EvoReadiness::NotYet));
    }

    #[test]
    fn test_evo_readiness_egg_already_equipped_is_ready_now() {
        // base 800 < threshold 1000, but with an egg equipped the in-game total
        // is 800*1.3 = 1040 >= 1000 — evolvable *now*, so Ready (not ReadyWithEgg).
        let mut pet = readiness_pet(800, None, true, Some(GrowthRequirement::Total(1000)));
        equip_magic_egg(&mut pet);
        assert_eq!(pet.evo_readiness(), Some(EvoReadiness::Ready));

        // With the egg equipped but still short even after the boost -> NotYet.
        let mut pet = readiness_pet(700, None, true, Some(GrowthRequirement::Total(1000)));
        equip_magic_egg(&mut pet);
        assert_eq!(pet.evo_readiness(), Some(EvoReadiness::NotYet));
    }

    #[test]
    fn test_evo_readiness_zero_threshold_is_ready() {
        // 0 is a real threshold (some questline/auto-evo pets), always met.
        assert_eq!(readiness_pet(0, None, true, Some(GrowthRequirement::Total(0))).evo_readiness(), Some(EvoReadiness::Ready));
        assert_eq!(readiness_pet(0, None, true, Some(GrowthRequirement::Base(0))).evo_readiness(), Some(EvoReadiness::Ready));
    }

    #[test]
    fn test_evo_readiness_base_growth_ignores_egg() {
        // Base-growth threshold (Baby Carno style): the egg never counts.
        let req = || Some(GrowthRequirement::Base(1000));
        assert_eq!(readiness_pet(1000, None, true, req()).evo_readiness(), Some(EvoReadiness::Ready));
        // 900*1.3 would clear 1000, but base-growth thresholds ignore the egg.
        assert_eq!(readiness_pet(900, None, true, req()).evo_readiness(), Some(EvoReadiness::NotYet));
    }

    #[test]
    fn test_evo_readiness_not_applicable() {
        let req = || Some(GrowthRequirement::Total(1000));
        // Already evolved → None
        assert_eq!(readiness_pet(5000, Some(Class::Mage), true, req()).evo_readiness(), None);
        // No evolution data → None
        assert_eq!(readiness_pet(5000, None, true, None).evo_readiness(), None);
    }

    #[test]
    fn test_evo_readiness_unowned_pets_count() {
        // Locked (unowned) but unevolved pets still get readiness — knowing an
        // unowned pet is already evolve-ready helps decide what to unlock next.
        let req = || Some(GrowthRequirement::Total(1000));
        assert_eq!(readiness_pet(5000, None, false, req()).evo_readiness(), Some(EvoReadiness::Ready));
        assert_eq!(readiness_pet(500, None, false, req()).evo_readiness(), Some(EvoReadiness::NotYet));
    }

    fn rates(evolved: u32, moai: f64, cap: u64) -> GrowthRates {
        GrowthRates { evolved_pets: evolved, moai_per_hour: moai, pendant_cap: cap }
    }

    #[test]
    fn test_hours_to_evolve() {
        let r = rates(80, 0.0, 1_000_000); // 80/hr, no cap concern
        // Unevolved, unlocked, Total(1000), growth 200 → (800)/80 = 10h (no egg).
        let pet = readiness_pet(200, None, true, Some(GrowthRequirement::Total(1000)));
        assert_eq!(pet.hours_to_evolve(&r, false), Some(10.0));
        // Already at threshold → 0.
        let pet = readiness_pet(1000, None, true, Some(GrowthRequirement::Total(1000)));
        assert_eq!(pet.hours_to_evolve(&r, false), Some(0.0));
        // Unowned (locked) but unevolved pets are still estimated (planning aid).
        assert_eq!(readiness_pet(200, None, false, Some(GrowthRequirement::Total(1000))).hours_to_evolve(&r, false), Some(10.0));
        // Not applicable: evolved / no evo data → None.
        assert_eq!(readiness_pet(200, Some(Class::Mage), true, Some(GrowthRequirement::Total(1000))).hours_to_evolve(&r, false), None);
        assert_eq!(readiness_pet(200, None, true, None).hours_to_evolve(&r, false), None);
    }

    #[test]
    fn test_hours_to_evolve_egg_time() {
        let r = rates(80, 0.0, 1_000_000);
        // Total(1300) with egg: target = ceil(1300/1.3) = 1000, growth 200 →
        // (800)/80 = 10h. Without egg it's the full 1300 → (1100)/80 = 13.75h.
        let pet = readiness_pet(200, None, true, Some(GrowthRequirement::Total(1300)));
        assert_eq!(pet.hours_to_evolve(&r, true), Some(10.0));
        assert_eq!(pet.hours_to_evolve(&r, false), Some(13.75));

        // Baby Carno case: a BASE-growth threshold ignores the egg, so use_egg
        // must NOT discount it (a false discount would give ~7.1h, not 10h).
        let carno = readiness_pet(200, None, true, Some(GrowthRequirement::Base(1000)));
        assert_eq!(carno.hours_to_evolve(&r, true), Some(10.0));
        assert_eq!(carno.hours_to_evolve(&r, false), Some(10.0));
    }

    #[test]
    fn test_hours_to_growth_arbitrary_target() {
        let r = rates(80, 0.0, 1_000_000);
        // Applies to any pet — evolved or not, owned or not.
        let pet = readiness_pet(200, Some(Class::Mage), true, None);
        assert_eq!(pet.hours_to_growth(1000, &r), Some(10.0));
        // Unowned (locked) pet is still estimated.
        assert_eq!(readiness_pet(200, None, false, None).hours_to_growth(1000, &r), Some(10.0));
        // No export data → None.
        let no_export = MergedPet {
            name: "X".to_string(),
            wiki: Some(make_wiki_pet("X", Element::Earth, RecommendedClass::Wildcard)),
            export: None,
        };
        assert_eq!(no_export.hours_to_growth(1000, &r), None);
    }

    #[test]
    fn test_campaign_bonuses_seam() {
        let rules: CampaignBonusRules = serde_yaml::from_str(
            "Dwarf:\n  - when: Always\n    set: { Food: 151, GodPower: 75 }\n",
        )
        .unwrap();
        let inputs = CampaignInputs::default();
        let ctx = CampaignContext { bonuses: &rules, roster: &[], inputs: &inputs, include_equipment: false, include_class: false };

        let wiki = make_wiki_pet("Dwarf", Element::Fire, RecommendedClass::Wildcard);
        let pet = MergedPet { name: "Dwarf".into(), wiki: Some(wiki), export: None };
        assert_eq!(pet.campaign_bonus_for(CampaignType::Food, &ctx), Some(151.0));
        assert_eq!(pet.campaign_bonus_for(CampaignType::GodPower, &ctx), Some(75.0));
        // No entry for a campaign it doesn't affect → None (not Some(0.0)).
        assert_eq!(pet.campaign_bonus_for(CampaignType::Growth, &ctx), None);

        // A pet with no curated rules yields an empty map.
        let bare = MergedPet { name: "X".into(), wiki: None, export: None };
        assert!(bare.campaign_bonuses(&ctx).is_empty());
        assert_eq!(bare.campaign_bonus_for(CampaignType::Food, &ctx), None);
    }

    #[test]
    fn test_campaign_bonuses_applies_conditional_rules() {
        // Hedgehog: +25 base, +141 more when token-improved.
        let rules: CampaignBonusRules = serde_yaml::from_str(
            "Hedgehog:\n  - when: Always\n    set: { Growth: 25 }\n  - when: TokenImproved\n    add: { Growth: 141 }\n",
        )
        .unwrap();
        let inputs = CampaignInputs::default();
        let ctx = CampaignContext { bonuses: &rules, roster: &[], inputs: &inputs, include_equipment: false, include_class: false };

        let wiki = make_wiki_pet("Hedgehog", Element::Earth, RecommendedClass::Wildcard);
        let mut export = make_export_pet("Hedgehog", Element::Earth, None);
        export.improved = true;
        let pet = MergedPet { name: "Hedgehog".into(), wiki: Some(wiki.clone()), export: Some(export) };
        assert_eq!(pet.campaign_bonus_for(CampaignType::Growth, &ctx), Some(166.0));

        // Not improved → baseline only.
        let export2 = make_export_pet("Hedgehog", Element::Earth, None); // improved = false
        let pet2 = MergedPet { name: "Hedgehog".into(), wiki: Some(wiki), export: Some(export2) };
        assert_eq!(pet2.campaign_bonus_for(CampaignType::Growth, &ctx), Some(25.0));
    }

    #[test]
    fn test_campaign_formulas() {
        let empty = CampaignBonusRules::default();
        let inputs = CampaignInputs::default();

        // Build a small roster: a low-growth unlocked pet, a higher one, and a
        // very-low locked pet that must be ignored by Bag.
        let mut low = make_export_pet("Frog", Element::Water, None);
        low.growth = 10_000; // 10000^0.4 ≈ 39.8
        low.unlocked = true;
        let mut high = make_export_pet("Bee", Element::Wind, Some(Class::Mage));
        high.growth = 1_000_000;
        high.unlocked = true;
        let mut locked = make_export_pet("Void", Element::Neutral, None);
        locked.growth = 1; // would dominate the min, but it's locked
        locked.unlocked = false;

        let pets = |extra: MergedPet| {
            vec![
                MergedPet { name: "Frog".into(), wiki: None, export: Some(low.clone()) },
                MergedPet { name: "Bee".into(), wiki: None, export: Some(high.clone()) },
                MergedPet { name: "Void".into(), wiki: None, export: Some(locked.clone()) },
                extra,
            ]
        };

        // Bag: lowest *unlocked* growth (10000) ^ 0.4 → ~39.8 to Growth.
        let bag = MergedPet { name: "Bag".into(), wiki: None, export: None };
        let roster = pets(bag.clone());
        let ctx = CampaignContext { bonuses: &empty, roster: &roster, inputs: &inputs, include_equipment: false, include_class: false };
        // 10000^0.4 = 39.8107… rounded to 2 decimals.
        assert_eq!(bag.campaign_bonus_for(CampaignType::Growth, &ctx), Some(39.81));

        // Mermaid: -(own growth / 1000) to all, capped -333. growth 50000 → -50.
        let mut mer_export = make_export_pet("Mermaid", Element::Water, None);
        mer_export.growth = 50_000;
        let mermaid = MergedPet { name: "Mermaid".into(), wiki: None, export: Some(mer_export) };
        let roster = pets(mermaid.clone());
        let ctx = CampaignContext { bonuses: &empty, roster: &roster, inputs: &inputs, include_equipment: false, include_class: false };
        assert_eq!(mermaid.campaign_bonus_for(CampaignType::GodPower, &ctx), Some(-50.0));

        // Lizard: (unlocked + evolved)^0.5 * 10, capped 100, to Growth (unevolved).
        // Roster has 3 unlocked (Frog, Bee, Lizard) + 1 evolved (Bee) = 4 → 20.
        let lizard = MergedPet { name: "Lizard/Zookeeper".into(), wiki: None, export: Some({
            let mut e = make_export_pet("Lizard", Element::Earth, None);
            e.unlocked = true;
            e
        }) };
        let roster = pets(lizard.clone());
        let ctx = CampaignContext { bonuses: &empty, roster: &roster, inputs: &inputs, include_equipment: false, include_class: false };
        // unlocked: Frog, Bee, Lizard = 3; evolved: Bee = 1 → 4^0.5*10 = 20.
        assert_eq!(lizard.campaign_bonus_for(CampaignType::Growth, &ctx), Some(20.0));
        assert_eq!(lizard.campaign_bonus_for(CampaignType::Food, &ctx), None);
    }

    #[test]
    fn test_campaign_formula_caps() {
        let empty = CampaignBonusRules::default();
        let inputs = CampaignInputs::default();

        // Bag clamps at +100 (1e6^0.4 ≈ 251).
        let mut p = make_export_pet("Frog", Element::Water, None);
        p.growth = 1_000_000;
        p.unlocked = true;
        let bag = MergedPet { name: "Bag".into(), wiki: None, export: None };
        let roster = vec![
            MergedPet { name: "Frog".into(), wiki: None, export: Some(p) },
            bag.clone(),
        ];
        let ctx = CampaignContext { bonuses: &empty, roster: &roster, inputs: &inputs, include_equipment: false, include_class: false };
        assert_eq!(bag.campaign_bonus_for(CampaignType::Growth, &ctx), Some(100.0));

        // Mermaid clamps at -333 (1e6/1000 = 1000).
        let mut e = make_export_pet("Mermaid", Element::Water, None);
        e.growth = 1_000_000;
        let mermaid = MergedPet { name: "Mermaid".into(), wiki: None, export: Some(e) };
        let roster = vec![mermaid.clone()];
        let ctx = CampaignContext { bonuses: &empty, roster: &roster, inputs: &inputs, include_equipment: false, include_class: false };
        assert_eq!(mermaid.campaign_bonus_for(CampaignType::Growth, &ctx), Some(-333.0));
    }

    #[test]
    fn test_campaign_input_formulas() {
        let empty = CampaignBonusRules::default();
        let inputs = CampaignInputs {
            challenge_points: 10_000, // sqrt(10000)/2 = 50
            honey: 25_000,            // 25000/500 = 50
            ants: 10_000,
            pet_stones: 10_000,
            ..Default::default()
        };
        let roster: Vec<MergedPet> = vec![];
        let ctx = CampaignContext { bonuses: &empty, roster: &roster, inputs: &inputs, include_equipment: false, include_class: false };

        let pet = |name: &str, improved: bool| {
            let mut e = make_export_pet(name, Element::Neutral, None);
            e.improved = improved;
            MergedPet { name: name.into(), wiki: None, export: Some(e) }
        };

        // Unicorn → 50 on growth/godpower/divinity only.
        let u = pet("Unicorn", false);
        assert_eq!(u.campaign_bonus_for(CampaignType::Growth, &ctx), Some(50.0));
        assert_eq!(u.campaign_bonus_for(CampaignType::Food, &ctx), None);

        // Bear → 50 on all campaigns.
        assert_eq!(pet("Bear", false).campaign_bonus_for(CampaignType::Item, &ctx), Some(50.0));

        // Ant Queen → equal, positive bonus on divinity & god power.
        let a = pet("Ant Queen", false);
        let d = a.campaign_bonus_for(CampaignType::Divinity, &ctx);
        assert_eq!(d, a.campaign_bonus_for(CampaignType::GodPower, &ctx));
        assert!(d.unwrap() > 0.0);

        // Cupid: +1% per partnered pet (from the roster's `has_partner`), only
        // when token-improved. Two partnered pets here → +2% to all campaigns.
        let partnered = |name: &str, has: bool| {
            let mut e = make_export_pet(name, Element::Neutral, None);
            e.has_partner = has;
            MergedPet { name: name.into(), wiki: None, export: Some(e) }
        };
        let cupid_roster = vec![partnered("X", true), partnered("Y", true), partnered("Z", false)];
        let cupid_ctx = CampaignContext { bonuses: &empty, roster: &cupid_roster, inputs: &inputs, include_equipment: false, include_class: false };
        assert_eq!(pet("Cupid", true).campaign_bonus_for(CampaignType::Growth, &cupid_ctx), Some(2.0));
        assert_eq!(pet("Cupid", false).campaign_bonus_for(CampaignType::Growth, &cupid_ctx), None);

        // Beachball → a positive all-campaign bonus from stones.
        assert!(pet("Beachball", false).campaign_bonus_for(CampaignType::Growth, &ctx).unwrap() > 0.0);

        // Beachball combines held + given stones: 5000 + 5000 == 10000 held.
        let combined = CampaignInputs { pet_stones: 5_000, beachball_given_stones: 5_000, ..Default::default() };
        let held_only = CampaignInputs { pet_stones: 10_000, ..Default::default() };
        let roster: Vec<MergedPet> = vec![];
        let c1 = CampaignContext { bonuses: &empty, roster: &roster, inputs: &combined, include_equipment: false, include_class: false };
        let c2 = CampaignContext { bonuses: &empty, roster: &roster, inputs: &held_only, include_equipment: false, include_class: false };
        let bb = pet("Beachball", false);
        assert_eq!(
            bb.campaign_bonus_for(CampaignType::Growth, &c1),
            bb.campaign_bonus_for(CampaignType::Growth, &c2),
        );

        // Beachball caps at 200% (1e9 stones would otherwise give ~900%).
        let huge = CampaignInputs { pet_stones: 1_000_000_000, ..Default::default() };
        let ctx = CampaignContext { bonuses: &empty, roster: &roster, inputs: &huge, include_equipment: false, include_class: false };
        assert_eq!(bb.campaign_bonus_for(CampaignType::Growth, &ctx), Some(200.0));
    }

    #[test]
    fn test_campaign_aether() {
        let empty = CampaignBonusRules::default();
        let aether = |growth: u64| {
            let mut e = make_export_pet("Aether", Element::Neutral, None);
            e.growth = growth;
            MergedPet { name: "Aether".into(), wiki: None, export: Some(e) }
        };
        let elemental = |name: &str| {
            let mut e = make_export_pet(name, Element::Neutral, None);
            e.unlocked = true;
            MergedPet { name: name.into(), wiki: None, export: Some(e) }
        };

        // fights = 0 → full -99% penalty to all, no growth bonus.
        let inputs0 = CampaignInputs::default();
        let roster: Vec<MergedPet> = vec![];
        let ctx = CampaignContext { bonuses: &empty, roster: &roster, inputs: &inputs0, include_equipment: false, include_class: false };
        let a = aether(1);
        assert_eq!(a.campaign_bonus_for(CampaignType::Food, &ctx), Some(-99.0));
        assert_eq!(a.campaign_bonus_for(CampaignType::Growth, &ctx), Some(-99.0));

        // fights = 10 (penalty maxes at +1), 5 elementals owned, growth 1e6
        // (log_1000 = 2). Growth bonus = (10/10)*10*(1+0.57*2) = 21.4 → 22.4.
        let inputs10 = CampaignInputs { delirious_essence_fights: 10, ..Default::default() };
        let roster = vec![
            elemental("Undine"),
            elemental("Gnome"),
            elemental("Salamander"),
            elemental("Sylph"),
            elemental("Elemental"),
        ];
        let ctx = CampaignContext { bonuses: &empty, roster: &roster, inputs: &inputs10, include_equipment: false, include_class: false };
        let a = aether(1_000_000);
        assert_eq!(a.campaign_bonus_for(CampaignType::Food, &ctx), Some(1.0)); // penalty only
        let g = a.campaign_bonus_for(CampaignType::Growth, &ctx).unwrap();
        assert!((g - 22.4).abs() < 0.01, "got {g}");

        // Real player values: Gnome/Salamander/Sylph/Aether unlocked, 28 fights,
        // Aether growth 48,013. Aether counts itself among the 4 elementals:
        // 1 + (9/10)*28*(1 + 0.57*log_1000(48013)) ≈ 48.61% (the game shows 49%).
        let aether_pet = aether(48_013);
        let roster = vec![
            elemental("Gnome"),
            elemental("Salamander"),
            elemental("Sylph"),
            aether_pet.clone(),
        ];
        let inputs28 = CampaignInputs { delirious_essence_fights: 28, ..Default::default() };
        let ctx = CampaignContext { bonuses: &empty, roster: &roster, inputs: &inputs28, include_equipment: false, include_class: false };
        let g = aether_pet.campaign_bonus_for(CampaignType::Growth, &ctx).unwrap();
        assert!((g - 48.61).abs() < 0.1, "got {g}");
    }

    #[test]
    fn test_stick_campaign_bonus() {
        let empty = CampaignBonusRules::default();
        let inputs = CampaignInputs::default();
        let roster: Vec<MergedPet> = vec![];

        let stick_pet = |name: &str, quality: Quality, upgrade: u8| {
            let mut e = make_export_pet("Otter", Element::Water, Some(Class::Mage));
            e.loadout.weapon = Some(Equipment {
                name: name.to_string(),
                upgrade_level: Some(upgrade),
                quality,
                enchant_level: None,
                gem: None,
                gem_level: None,
            });
            MergedPet { name: "Otter".into(), wiki: None, export: Some(e) }
        };
        let on = CampaignContext { bonuses: &empty, roster: &roster, inputs: &inputs, include_equipment: true, include_class: false };
        let off = CampaignContext { bonuses: &empty, roster: &roster, inputs: &inputs, include_equipment: false, include_class: false };

        // Magic Stick SSS+10 = 26.19% (the in-game value), added to all campaigns.
        let pet = stick_pet("Magic Stick", Quality::SSS, 10);
        assert_eq!(pet.campaign_bonus_for(CampaignType::Growth, &on), Some(26.19));
        // Toggle off → no equipment bonus (no innate here either → None).
        assert_eq!(pet.campaign_bonus_for(CampaignType::Growth, &off), None);

        // SSS+20 hits each stick's cap.
        assert_eq!(stick_pet("Magic Stick", Quality::SSS, 20).campaign_bonus_for(CampaignType::Food, &on), Some(50.0));
        assert_eq!(stick_pet("Legendary Stick", Quality::SSS, 20).campaign_bonus_for(CampaignType::Food, &on), Some(100.0));

        // A non-stick weapon contributes nothing.
        assert_eq!(stick_pet("Flame Sword", Quality::SSS, 10).campaign_bonus_for(CampaignType::Food, &on), None);

        // The stick STACKS on an innate bonus (Growth +10) rather than clobbering.
        let mut e = make_export_pet("Whale", Element::Water, Some(Class::Mage));
        e.loadout.weapon = Some(Equipment {
            name: "Magic Stick".to_string(),
            upgrade_level: Some(10),
            quality: Quality::SSS,
            enchant_level: None,
            gem: None,
            gem_level: None,
        });
        let rules: CampaignBonusRules =
            serde_yaml::from_str("Whale:\n  - when: Always\n    set: { Growth: 10 }\n").unwrap();
        let on_with_innate = CampaignContext { bonuses: &rules, roster: &roster, inputs: &inputs, include_equipment: true, include_class: false };
        let wiki = make_wiki_pet("Whale", Element::Water, RecommendedClass::Wildcard);
        let pet = MergedPet { name: "Whale".into(), wiki: Some(wiki), export: Some(e) };
        // 10 + 26.19 = 36.19 (tolerance for f32 summation jitter).
        let g = pet.campaign_bonus_for(CampaignType::Growth, &on_with_innate).unwrap();
        assert!((g - 36.19).abs() < 0.001, "got {g}");
        assert_eq!(pet.campaign_bonus_for(CampaignType::Food, &on_with_innate), Some(26.19)); // stick only
    }

    #[test]
    fn test_event_equip_campaign_bonus() {
        let empty = CampaignBonusRules::default();
        let inputs = CampaignInputs::default();
        let roster: Vec<MergedPet> = vec![];
        let on = CampaignContext { bonuses: &empty, roster: &roster, inputs: &inputs, include_equipment: true, include_class: false };

        let item = |name: &str, q: Quality, up: Option<u8>| Equipment {
            name: name.into(),
            upgrade_level: up,
            quality: q,
            enchant_level: None,
            gem: None,
            gem_level: None,
        };
        let pet_with = |weapon: Option<Equipment>, armor: Option<Equipment>| {
            let mut e = make_export_pet("X", Element::Neutral, Some(Class::Mage));
            e.loadout.weapon = weapon;
            e.loadout.armor = armor;
            MergedPet { name: "X".into(), wiki: None, export: Some(e) }
        };

        // Candy Cane SSS+20 (weapon) → +101 to all.
        let p = pet_with(Some(item("Candy Cane", Quality::SSS, Some(20))), None);
        assert_eq!(p.campaign_bonus_for(CampaignType::Growth, &on), Some(101.0));
        // Merry Mantle SSS+20 in the *armor* slot → +150 (non-weapon slot works).
        let p = pet_with(None, Some(item("Merry Mantle", Quality::SSS, Some(20))));
        assert_eq!(p.campaign_bonus_for(CampaignType::Food, &on), Some(150.0));
        // Unknown level (S+10) → no bonus (only SSS+20 is known).
        let p = pet_with(Some(item("Candy Cane", Quality::S, Some(10))), None);
        assert_eq!(p.campaign_bonus_for(CampaignType::Growth, &on), None);
        // A stick + an event item across slots stack: 50 + 150 = 200.
        let p = pet_with(
            Some(item("Magic Stick", Quality::SSS, Some(20))),
            Some(item("Merry Mantle", Quality::SSS, Some(20))),
        );
        assert_eq!(p.campaign_bonus_for(CampaignType::Item, &on), Some(200.0));
    }

    #[test]
    fn test_class_campaign_bonus() {
        let empty = CampaignBonusRules::default();
        let inputs = CampaignInputs::default();
        let roster: Vec<MergedPet> = vec![];
        let on = CampaignContext { bonuses: &empty, roster: &roster, inputs: &inputs, include_equipment: false, include_class: true };
        let off = CampaignContext { bonuses: &empty, roster: &roster, inputs: &inputs, include_equipment: false, include_class: false };

        let pet = |class: Option<Class>, cl: u32| {
            let mut e = make_export_pet("Robot", Element::Neutral, class);
            e.class_level = cl;
            MergedPet { name: "Robot".into(), wiki: None, export: Some(e) }
        };

        // Adventurer CL8 → 2% × 8 = 16% to all campaigns.
        let p = pet(Some(Class::Adventurer), 8);
        assert_eq!(p.campaign_bonus_for(CampaignType::Growth, &on), Some(16.0));
        assert_eq!(p.campaign_bonus_for(CampaignType::Food, &on), Some(16.0));
        // Toggle off → no class bonus.
        assert_eq!(p.campaign_bonus_for(CampaignType::Growth, &off), None);
        // Non-Adventurer class contributes nothing even with the toggle on.
        assert_eq!(pet(Some(Class::Mage), 8).campaign_bonus_for(CampaignType::Growth, &on), None);

        // A pet with an Adventurer evo bonus stacks it on the base:
        // Hedgehog (+0.58/CL) at CL22 → (2 + 0.58) × 22 = 56.76% (game shows 57).
        let mut e = make_export_pet("Hedgehog", Element::Neutral, Some(Class::Adventurer));
        e.class_level = 22;
        let hedgehog = MergedPet { name: "Hedgehog".into(), wiki: None, export: Some(e) };
        let g = hedgehog.campaign_bonus_for(CampaignType::Growth, &on).unwrap();
        assert!((g - 56.76).abs() < 0.001, "got {g}");
    }

    /// Every name in ADVENTURER_EVO_BONUS must match a real pet in the scraped
    /// data — otherwise the bonus silently never applies.
    #[test]
    fn test_adventurer_evo_bonus_names_exist() {
        let yaml = include_str!("../../../data/wiki_pets.yaml");
        let pets: Vec<WikiPet> = serde_yaml::from_str(yaml).expect("parse wiki_pets.yaml");
        let names: std::collections::HashSet<&str> = pets.iter().map(|p| p.name.as_str()).collect();
        let missing: Vec<&str> = ADVENTURER_EVO_BONUS
            .iter()
            .map(|(n, _)| *n)
            .filter(|n| !names.contains(n))
            .collect();
        assert!(missing.is_empty(), "unknown pet names: {missing:?}");
    }

    #[test]
    fn test_earth_eater_campaign_bonus() {
        let empty = CampaignBonusRules::default();
        let mk = |improved: bool, total: u64, show_lifetime: bool| {
            let inputs = CampaignInputs {
                earth_eater_total_planets: total,
                earth_eater_show_lifetime: show_lifetime,
                ..Default::default()
            };
            let mut e = make_export_pet("Earth Eater", Element::Earth, None);
            e.improved = improved;
            (inputs, MergedPet { name: "Earth Eater".into(), wiki: None, export: Some(e) })
        };
        let check = |inputs: &CampaignInputs, pet: &MergedPet, want: f32| {
            let ctx = CampaignContext { bonuses: &empty, roster: &[], inputs, include_equipment: false, include_class: false };
            // Flat bonus → all campaigns share the value.
            assert_eq!(pet.campaign_bonus_for(CampaignType::Growth, &ctx), Some(want));
            assert_eq!(pet.campaign_bonus_for(CampaignType::Item, &ctx), Some(want));
        };
        // Default (locked at +82) → always +82, regardless of token/total.
        let (i, p) = mk(false, 0, false);
        check(&i, &p, 82.0);
        let (i, p) = mk(true, 10_000_000, false);
        check(&i, &p, 82.0); // locked beats the lower permanent value
        // Opt into the lifetime view: only a token-improved pet with a total set
        // shows the lower permanent value (+1% per 200k from -80%).
        let (i, p) = mk(true, 0, true);
        check(&i, &p, 82.0); // no total entered → still 82
        let (i, p) = mk(false, 10_000_000, true);
        check(&i, &p, 82.0); // not token-improved → still 82
        let (i, p) = mk(true, 100_000, true);
        check(&i, &p, -79.5); // near the -80 floor (-80 + 0.5)
        let (i, p) = mk(true, 10_000_000, true);
        check(&i, &p, -30.0); // -80 + 50
        let (i, p) = mk(true, 32_400_000, true);
        check(&i, &p, 82.0); // cap
        let (i, p) = mk(true, 100_000_000, true);
        check(&i, &p, 82.0); // past cap, clamped
    }

    #[test]
    fn test_goblin_campaign_bonus() {
        let empty = CampaignBonusRules::default();
        // Goblin's static base, as curated in campaign_bonuses.yaml.
        let rules: CampaignBonusRules = serde_yaml::from_str(
            "Goblin:\n  - when: Always\n    set_all: 50\n    set: { Growth: -100, Item: -100, Divinity: 150 }\n",
        )
        .unwrap();
        let wiki = make_wiki_pet("Goblin", Element::Neutral, RecommendedClass::Wildcard);

        // UCC adds +1% to every campaign, capped at 75 → base shifts to the
        // documented -25 / +225 / +125.
        let inputs = CampaignInputs { goblin_ucc: 200, ..Default::default() };
        let pet = MergedPet {
            name: "Goblin".into(),
            wiki: Some(wiki.clone()),
            export: Some(make_export_pet("Goblin", Element::Neutral, None)),
        };
        let ctx = CampaignContext { bonuses: &rules, roster: &[], inputs: &inputs, include_equipment: false, include_class: false };
        assert_eq!(pet.campaign_bonus_for(CampaignType::Growth, &ctx), Some(-25.0));
        assert_eq!(pet.campaign_bonus_for(CampaignType::Divinity, &ctx), Some(225.0));
        assert_eq!(pet.campaign_bonus_for(CampaignType::Food, &ctx), Some(125.0));

        // OC-driven Adventurer evo bonus (class layer). Isolate it with no wiki
        // base and no UCC: Growth = (2 + evo) · CL only.
        let evo_pet = |cl: u32| {
            let mut e = make_export_pet("Goblin", Element::Neutral, Some(Class::Adventurer));
            e.class_level = cl;
            MergedPet { name: "Goblin".into(), wiki: None, export: Some(e) }
        };
        let class_bonus = |oc: u32, cl: u32| {
            let inp = CampaignInputs { goblin_oc: oc, ..Default::default() };
            let ctx = CampaignContext { bonuses: &empty, roster: &[], inputs: &inp, include_equipment: false, include_class: true };
            evo_pet(cl).campaign_bonus_for(CampaignType::Growth, &ctx).unwrap()
        };
        assert_eq!(class_bonus(0, 10), 21.0); // (2 + 0.1) · 10
        assert_eq!(class_bonus(100, 10), 29.0); // 0.1 + 0.8 → 2.9 · 10
        // 470+ → full evo bonus 1.5 → (3.5) · 10 = 35 (within rounding).
        let capped = class_bonus(470, 10);
        assert!((capped - 35.0).abs() < 0.02, "got {capped}");
    }

    #[test]
    fn test_stone_campaign_bonus() {
        let empty = CampaignBonusRules::default();
        let ov: CampaignBonusRules =
            serde_yaml::from_str("Stone/Golem:\n  - when: Evolved\n    set_all: 100\n").unwrap();
        let mk = |growth: u64, evolved: bool, upgrade: bool| {
            let inputs = CampaignInputs { stone_campaign_upgrade: upgrade, ..Default::default() };
            let class = if evolved { Some(Class::Defender) } else { None };
            let mut e = make_export_pet("Stone/Golem", Element::Earth, class);
            e.growth = growth;
            (inputs, MergedPet { name: "Stone/Golem".into(), wiki: None, export: Some(e) })
        };
        let g = |inputs: &CampaignInputs, pet: &MergedPet, ov: &CampaignBonusRules| {
            let ctx = CampaignContext { bonuses: ov, roster: &[], inputs, include_equipment: false, include_class: false };
            pet.campaign_bonus_for(CampaignType::Growth, &ctx).unwrap()
        };
        // Unevolved ramp: -100 + 20 per 5000 growth, capped at 0.
        let (i, p) = mk(10_000, false, false);
        assert_eq!(g(&i, &p, &empty), -60.0);
        let (i, p) = mk(25_000, false, false);
        assert_eq!(g(&i, &p, &empty), 0.0);
        let (i, p) = mk(50_000, false, false);
        assert_eq!(g(&i, &p, &empty), 0.0); // clamped
        // The +100% upgrade stacks on the ramp.
        let (i, p) = mk(10_000, false, true);
        assert_eq!(g(&i, &p, &empty), 40.0);
        // Evolved: +100 from the curated rule, +100 from the upgrade = 200.
        let (i, p) = mk(99_999, true, true);
        assert_eq!(g(&i, &p, &ov), 200.0);
        let (i, p) = mk(99_999, true, false);
        assert_eq!(g(&i, &p, &ov), 100.0);
    }

    #[test]
    fn test_elemental_form_campaign_bonus() {
        // End-to-end: the elemental form (from the export "Other" column) drives
        // the all-campaign bonus through the real curated rules.
        let ov: CampaignBonusRules =
            serde_yaml::from_str(include_str!("../../../data/campaign_bonuses.yaml")).unwrap();
        let mk = |other: Option<&str>, evolved: bool| {
            let class = if evolved { Some(Class::Mage) } else { None };
            let mut e = make_export_pet("Sylph", Element::Wind, class);
            e.other = other.map(str::to_string);
            MergedPet { name: "Sylph".into(), wiki: None, export: Some(e) }
        };
        let g = |pet: &MergedPet| {
            let inputs = CampaignInputs::default();
            let ctx = CampaignContext {
                bonuses: &ov, roster: &[], inputs: &inputs,
                include_equipment: false, include_class: false,
            };
            pet.campaign_bonus_for(CampaignType::Growth, &ctx).unwrap()
        };
        // Unevolved, progressing through forms: V0 base → V2 → V3.
        assert_eq!(g(&mk(Some("SylphV0"), false)), -150.0);
        assert_eq!(g(&mk(Some("SylphV2"), false)), -50.0);
        assert_eq!(g(&mk(Some("SylphV3"), false)), 25.0);
        // Evolved shows "…Final" (no numeric form) → the final value.
        assert_eq!(g(&mk(Some("SylphFinal"), true)), 75.0);
        // No form data (e.g. a different pet's "Other") → unevolved base fallback.
        assert_eq!(g(&mk(None, false)), -150.0);
    }

    #[test]
    fn test_elemental_evo_plan() {
        let mk = |name: &str, growth: u64, other: &str, evolved: bool| {
            let class = if evolved { Some(Class::Mage) } else { None };
            let mut e = make_export_pet(name, Element::Earth, class);
            e.growth = growth;
            e.other = Some(other.to_string());
            MergedPet { name: name.into(), wiki: None, export: Some(e) }
        };

        // Gnome at V2 (min 12,226), below it → behind by the shortfall.
        let plan = mk("Gnome", 5_000, "GnomeV2", false).elemental_evo_plan().unwrap();
        assert_eq!(plan.form, 2);
        assert_eq!(plan.min_growth_for_form, 12_226);
        assert!(!plan.on_track);
        assert_eq!(plan.shortfall, 12_226 - 5_000);
        assert_eq!(plan.remaining_form_gain, 55_555 - 12_226);
        assert_eq!(plan.projected_final_growth, 5_000 + (55_555 - 12_226)); // < 55,555
        assert!(!plan.is_final_form);

        // On track at V2 (≥ 12,226) → projected final clears the bar.
        let plan = mk("Gnome", 20_000, "GnomeV2", false).elemental_evo_plan().unwrap();
        assert!(plan.on_track);
        assert_eq!(plan.shortfall, 0);
        assert!(plan.projected_final_growth >= 55_555);

        // Final form V4: min = the 55,555 evolve bar; on_track == evolve-ready.
        let plan = mk("Gnome", 60_000, "GnomeV4", false).elemental_evo_plan().unwrap();
        assert!(plan.is_final_form);
        assert_eq!(plan.min_growth_for_form, 55_555);
        assert_eq!(plan.remaining_form_gain, 0);
        assert!(plan.on_track);

        // Undine's V0 base form is the lowest minimum.
        assert_eq!(
            mk("Undine", 0, "UndineV0", false).elemental_evo_plan().unwrap().min_growth_for_form,
            -11_111
        );

        // Non-elemental, evolved, or no form → None.
        assert!(mk("Cat", 1_000, "", false).elemental_evo_plan().is_none());
        assert!(mk("Gnome", 60_000, "GnomeFinal", true).elemental_evo_plan().is_none());
    }

    #[test]
    fn test_aether_evo_plan() {
        let mk = |growth: u64, evolved: bool| {
            let class = if evolved { Some(Class::Mage) } else { None };
            let mut e = make_export_pet("Aether", Element::Neutral, class);
            e.growth = growth;
            MergedPet { name: "Aether".into(), wiki: None, export: Some(e) }
        };
        // Player's example: 721 CHP → 144.2 growth per fight.
        let plan = mk(10_000, false).aether_evo_plan(721, 28).unwrap();
        assert!((plan.growth_per_fight - 144.2).abs() < 1e-9);
        assert_eq!(plan.fights_remaining, 50 - 28);
        // CHP estimate = (55555 − 10000) / ((50 − 28)·0.2).
        let est = plan.chp_to_evolve_estimate.unwrap();
        assert!((est - 45_555.0 / 4.4).abs() < 0.01, "got {est}");
        // Past the growth bar → no estimate.
        assert!(
            mk(60_000, false).aether_evo_plan(721, 28).unwrap().chp_to_evolve_estimate.is_none()
        );
        // Evolved → None.
        assert!(mk(10_000, true).aether_evo_plan(721, 28).is_none());
    }

    #[test]
    fn test_meteor_campaign_bonus() {
        let empty = CampaignBonusRules::default();
        let meteor = MergedPet {
            name: "Meteor".into(),
            wiki: None,
            export: Some(make_export_pet("Meteor", Element::Neutral, None)),
        };
        // 25 + 4501^0.42 ≈ 59.23, applied to every campaign.
        let inputs = CampaignInputs { meteor_campaign_hours: 4501, ..Default::default() };
        let ctx = CampaignContext { bonuses: &empty, roster: &[], inputs: &inputs, include_equipment: false, include_class: false };
        let g = meteor.campaign_bonus_for(CampaignType::Growth, &ctx).unwrap();
        assert!((g - 59.23).abs() < 0.01, "got {g}");
        assert_eq!(meteor.campaign_bonus_for(CampaignType::Food, &ctx), Some(g));
        // Zero hours → just the +25 base.
        let zero = CampaignInputs::default();
        let ctx0 = CampaignContext { bonuses: &empty, roster: &[], inputs: &zero, include_equipment: false, include_class: false };
        assert_eq!(meteor.campaign_bonus_for(CampaignType::Growth, &ctx0), Some(25.0));
    }

    /// Every wiki pet with campaign-bonus text must be accounted for: a curated
    /// entry in `data/campaign_bonuses.yaml`, a code formula
    /// ([`CAMPAIGN_FORMULA_PETS`]), or the explicit raw-only list below. When
    /// the weekly wiki refresh picks up a new pet, this fails — that's the
    /// prompt to curate its bonus (there is no prose parser anymore).
    /// Conversely, every curated/formula name must resolve to a real wiki pet,
    /// guarding against typos and wiki renames.
    #[test]
    fn test_campaign_bonus_coverage() {
        // Pets whose campaign text is deliberately left unstructured (display
        // raw prose only), with the reason.
        const RAW_ONLY: &[&str] = &[
            "Pandora's Box",    // multiplies the whole campaign, not its own bonus (simulator concern)
            "Bug",              // random, per its description
            "Turtle",           // per-campaign-duration bonus, not one of the 7 types
            "Afky Clone",       // input formula identified but not yet built (campaign_bonus_design.md)
            "Chocobear",        // banked-hours model identified but not yet built
            "Gold Dragon",      // growth-sharing on feed, not a campaign bonus
            "Living Draw",      // base 80% + per-lucky-draw increments (needs an input)
            "Black Hole Chan",  // UBV4-points formula (needs an input)
            "Treasure/Mimic",   // no bonus; only a Pandora interaction penalty
            "Seed/Yggdrasil",   // RTI-god-based growth trickle, not a % bonus
            "Pumpkin",          // finds chocolate; no inherent % bonus
            "Ghost",            // explicitly no bonuses or maluses
            "FSM",              // divinity-generator/log2 effect, not a % bonus
            "Hourglass",        // days-since-start ramp (needs an input)
            "Sloth",            // per-campaign-duration formula
            "Wolf",             // per-challenge increments (needs an input)
            "Basilisk",         // wiki says unknown
            "Feather Pile/Owl", // alternates with Owl's focus
        ];

        let wiki: Vec<WikiPet> =
            serde_yaml::from_str(include_str!("../../../data/wiki_pets.yaml")).unwrap();
        let rules: CampaignBonusRules =
            serde_yaml::from_str(include_str!("../../../data/campaign_bonuses.yaml")).unwrap();
        let names: std::collections::BTreeSet<&str> =
            wiki.iter().map(|p| p.name.as_str()).collect();

        // Forward: every pet with campaign text is curated, a formula, or raw-only.
        let mut uncovered = Vec::new();
        for pet in &wiki {
            if pet.campaign_bonus.is_some()
                && !rules.0.contains_key(&pet.name)
                && !CAMPAIGN_FORMULA_PETS.contains(&pet.name.as_str())
                && !RAW_ONLY.contains(&pet.name.as_str())
            {
                uncovered.push(pet.name.as_str());
            }
        }
        assert!(
            uncovered.is_empty(),
            "pets with campaign-bonus text but no curated entry in \
             data/campaign_bonuses.yaml (add one, or list them as formula/raw-only): {uncovered:?}"
        );

        // Reverse: every listed name must be a real wiki pet.
        for name in rules.0.keys() {
            assert!(names.contains(name.as_str()), "campaign_bonuses.yaml key '{name}' is not a wiki pet");
        }
        for name in CAMPAIGN_FORMULA_PETS {
            assert!(names.contains(name), "CAMPAIGN_FORMULA_PETS entry '{name}' is not a wiki pet");
        }
        for name in RAW_ONLY {
            assert!(names.contains(name), "RAW_ONLY entry '{name}' is not a wiki pet");
        }
    }

    #[test]
    fn test_campaign_bonus_breakdown_splits_sources() {
        // A curated +50 Growth innate, an Adventurer at CL10 (class +20 to all),
        // and an SSS+20 Legendary Stick (equipment +100 to all).
        let rules: CampaignBonusRules =
            serde_yaml::from_str("Robot:\n  - when: Always\n    set: { Growth: 50 }\n").unwrap();
        let inputs = CampaignInputs::default();
        let mut e = make_export_pet("Robot", Element::Neutral, Some(Class::Adventurer));
        e.class_level = 10;
        e.loadout.weapon = Some(Equipment {
            name: "Legendary Stick".to_string(),
            upgrade_level: Some(20),
            quality: Quality::SSS,
            enchant_level: None,
            gem: None,
            gem_level: None,
        });
        let pet = MergedPet { name: "Robot".into(), wiki: None, export: Some(e) };

        let on = CampaignContext { bonuses: &rules, roster: &[], inputs: &inputs, include_equipment: true, include_class: true };
        let bd = pet.campaign_bonus_breakdown(&on);
        assert_eq!(bd.innate.get(&CampaignType::Growth), Some(&50.0));
        assert_eq!(bd.innate.get(&CampaignType::Food), None);
        assert_eq!(bd.equipment, Some(100.0));
        assert_eq!(bd.class, Some(20.0));
        // The flat layers apply to every campaign; innate only where curated.
        let total = bd.total();
        assert_eq!(total.get(&CampaignType::Growth), Some(&170.0));
        assert_eq!(total.get(&CampaignType::Food), Some(&120.0));
        // The totals are exactly what `campaign_bonuses` reports.
        assert_eq!(total, pet.campaign_bonuses(&on));

        // Layers toggled off → no equipment/class parts, totals are innate only.
        let off = CampaignContext { bonuses: &rules, roster: &[], inputs: &inputs, include_equipment: false, include_class: false };
        let bd = pet.campaign_bonus_breakdown(&off);
        assert_eq!(bd.equipment, None);
        assert_eq!(bd.class, None);
        assert_eq!(bd.total(), bd.innate);

        // Layers on but nothing to add (no gear, not an Adventurer) → still None,
        // so the UI knows there's no split to show.
        let plain = MergedPet {
            name: "Robot".into(),
            wiki: None,
            export: Some(make_export_pet("Robot", Element::Neutral, Some(Class::Mage))),
        };
        let bd = plain.campaign_bonus_breakdown(&on);
        assert_eq!(bd.equipment, None);
        assert_eq!(bd.class, None);
    }

    #[test]
    fn test_recommends_class() {
        let wiki = vec![
            make_wiki_pet("Frog", Element::Water, RecommendedClass::Single(Class::Supporter)),
            make_wiki_pet("Chameleon", Element::All, RecommendedClass::DungeonWildcard),
            make_wiki_pet("Swan", Element::Water, RecommendedClass::Village("Fisher".to_string())),
        ];
        let merged = merge_pets(&wiki, &[]);

        // Frog recommends Supporter but not Mage
        assert!(merged[0].recommends_class(&Class::Supporter));
        assert!(!merged[0].recommends_class(&Class::Mage));

        // Chameleon recommends any dungeon class
        assert!(merged[1].recommends_class(&Class::Defender));
        assert!(merged[1].recommends_class(&Class::Mage));
        assert!(!merged[1].recommends_class(&Class::Adventurer));

        // Swan is a village pet, never recommends for dungeon
        assert!(!merged[2].recommends_class(&Class::Supporter));
        assert!(merged[2].is_village_pet());
    }
}
