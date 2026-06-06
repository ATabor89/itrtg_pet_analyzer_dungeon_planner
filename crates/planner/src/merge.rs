use std::collections::{BTreeMap, HashMap};

use itrtg_models::{
    CampaignOverrides, CampaignType, Class, Element, ExportPet, MAGIC_EGG_GROWTH_MULT,
    RecommendedClass, WikiPet, resolve_wiki_name,
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

/// Runtime context for computing effective campaign bonuses.
///
/// Carries the curated overrides and the full roster (for export-derived
/// formulas like Bag's lowest-growth and Lizard's pet counts). A future phase
/// will add persisted user-input values here (pet stones, challenge points,
/// Delirious-Essence fights, …) — extending this rather than the seam signature.
pub struct CampaignContext<'a> {
    pub overrides: &'a CampaignOverrides,
    pub roster: &'a [MergedPet],
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

    /// This pet's effective per-campaign bonus percentages — **the single entry
    /// point** the UI uses for display, filtering, and sorting campaign bonuses.
    ///
    /// Starts from the *static* parsed baseline scraped from the wiki, then
    /// applies curated overrides conditioned on the pet's export state (token
    /// boosts like Hedgehog, evolution flips like Nothing, prose corrections
    /// like Cat). Export/user-input formulas (Bag, Mermaid, Beachball) will be
    /// layered in here in later phases via `ctx`. Callers go through this method
    /// rather than reading `wiki.campaign_bonus.per_campaign` directly, so they
    /// won't change when the numbers get richer.
    pub fn campaign_bonuses(&self, ctx: &CampaignContext) -> BTreeMap<CampaignType, f32> {
        let mut map = self
            .wiki
            .as_ref()
            .and_then(|w| w.campaign_bonus.as_ref())
            .map(|cb| cb.per_campaign.clone())
            .unwrap_or_default();
        let improved = self.export.as_ref().is_some_and(|e| e.improved);
        ctx.overrides.apply(&self.name, &mut map, self.is_evolved(), improved);
        self.apply_campaign_formulas(&mut map, ctx);
        map
    }

    /// Apply per-pet export-derived campaign formulas — bespoke math the parser
    /// can't express, computed from the roster and the pet's own export. No-op
    /// for pets without a formula. (User-input-driven formulas like Aether and
    /// Cupid's couples come in a later phase once the inputs are persisted.)
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
                    let v = (lowest as f64).powf(0.4).min(100.0) as f32;
                    map.insert(CampaignType::Growth, v);
                }
            }
            // Mermaid: -(own growth / 1000)% to all campaigns, capped at -333%.
            "Mermaid" => {
                if let Some(g) = self.export.as_ref().map(|e| e.growth) {
                    let v = (-(g as f64 / 1000.0)).max(-333.0) as f32;
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
                let v = (((unlocked + evolved) as f64).sqrt() * 10.0).min(100.0) as f32;
                let target = if self.is_evolved() {
                    CampaignType::Food
                } else {
                    CampaignType::Growth
                };
                map.insert(target, v);
            }
            _ => {}
        }
    }

    /// This pet's effective bonus to a single campaign, if known. `None` when the
    /// pet has no bonus or its bonus wasn't structured (raw-only) — distinct from
    /// `Some(0.0)`. Routes through [`Self::campaign_bonuses`] so it tracks
    /// overrides and future dynamic adjustments.
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
        let empty = CampaignOverrides::default();
        let ctx = CampaignContext { overrides: &empty, roster: &[] };

        let mut wiki = make_wiki_pet("Dwarf", Element::Fire, RecommendedClass::Wildcard);
        wiki.campaign_bonus = Some(CampaignBonus {
            raw: "+151% food camp, +75% godpower camp.".to_string(),
            per_campaign: [(CampaignType::Food, 151.0), (CampaignType::GodPower, 75.0)]
                .into_iter()
                .collect(),
        });
        let pet = MergedPet { name: "Dwarf".into(), wiki: Some(wiki), export: None };
        assert_eq!(pet.campaign_bonus_for(CampaignType::Food, &ctx), Some(151.0));
        assert_eq!(pet.campaign_bonus_for(CampaignType::GodPower, &ctx), Some(75.0));
        // No entry for a campaign it doesn't affect → None (not Some(0.0)).
        assert_eq!(pet.campaign_bonus_for(CampaignType::Growth, &ctx), None);

        // A pet with no wiki bonus yields an empty map.
        let bare = MergedPet { name: "X".into(), wiki: None, export: None };
        assert!(bare.campaign_bonuses(&ctx).is_empty());
        assert_eq!(bare.campaign_bonus_for(CampaignType::Food, &ctx), None);
    }

    #[test]
    fn test_campaign_bonuses_applies_overrides() {
        // Hedgehog: +25 base, +141 each when token-improved.
        let ov: CampaignOverrides = serde_yaml::from_str(
            "Hedgehog:\n  - when: TokenImproved\n    add: { Growth: 141 }\n",
        )
        .unwrap();
        let ctx = CampaignContext { overrides: &ov, roster: &[] };

        let mut wiki = make_wiki_pet("Hedgehog", Element::Earth, RecommendedClass::Wildcard);
        wiki.campaign_bonus = Some(CampaignBonus {
            raw: "+25% growth".into(),
            per_campaign: [(CampaignType::Growth, 25.0)].into_iter().collect(),
        });
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
        let empty = CampaignOverrides::default();

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
        let ctx = CampaignContext { overrides: &empty, roster: &roster };
        let v = bag.campaign_bonus_for(CampaignType::Growth, &ctx).unwrap();
        assert!((v - 39.8).abs() < 0.2, "got {v}");

        // Mermaid: -(own growth / 1000) to all, capped -333. growth 50000 → -50.
        let mut mer_export = make_export_pet("Mermaid", Element::Water, None);
        mer_export.growth = 50_000;
        let mermaid = MergedPet { name: "Mermaid".into(), wiki: None, export: Some(mer_export) };
        let roster = pets(mermaid.clone());
        let ctx = CampaignContext { overrides: &empty, roster: &roster };
        assert_eq!(mermaid.campaign_bonus_for(CampaignType::GodPower, &ctx), Some(-50.0));

        // Lizard: (unlocked + evolved)^0.5 * 10, capped 100, to Growth (unevolved).
        // Roster has 3 unlocked (Frog, Bee, Lizard) + 1 evolved (Bee) = 4 → 20.
        let lizard = MergedPet { name: "Lizard/Zookeeper".into(), wiki: None, export: Some({
            let mut e = make_export_pet("Lizard", Element::Earth, None);
            e.unlocked = true;
            e
        }) };
        let roster = pets(lizard.clone());
        let ctx = CampaignContext { overrides: &empty, roster: &roster };
        // unlocked: Frog, Bee, Lizard = 3; evolved: Bee = 1 → 4^0.5*10 = 20.
        assert_eq!(lizard.campaign_bonus_for(CampaignType::Growth, &ctx), Some(20.0));
        assert_eq!(lizard.campaign_bonus_for(CampaignType::Food, &ctx), None);
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
