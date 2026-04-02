use std::collections::{HashMap, HashSet};

use itrtg_models::dungeon::*;
use itrtg_models::*;

use crate::merge::MergedPet;

// =============================================================================
// Result types
// =============================================================================

/// The full solution for a dungeon + depth assignment.
#[derive(Debug, Clone)]
pub struct DungeonPlan {
    pub dungeon: Dungeon,
    pub depth: u8,
    /// Slot assignments in party order (indices 0-2 = front row, 3-5 = back row).
    pub assignments: Vec<SlotAssignment>,
    /// Counter coverage warnings — events/traps that won't be fully countered.
    pub warnings: Vec<CoverageWarning>,
}

/// Assignment of a pet to a party slot.
#[derive(Debug, Clone)]
pub struct SlotAssignment {
    /// The constraint for this slot (from recommendations).
    pub slot: PartySlot,
    /// The slot index in the party (0-5).
    pub position: usize,
    /// The assigned pet and how well it matches.
    pub assignment: Assignment,
    /// Equipment suggestion (populated by equipment::enrich_equipment after solving).
    pub equipment_suggestion: Option<crate::equipment::EquipmentSuggestion>,
}

/// What was assigned to a slot.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Assignment {
    /// A pet from the player's roster was assigned.
    Filled {
        pet: MergedPet,
        quality: MatchQuality,
    },
    /// No suitable pet was found. Includes suggestions for pets to unlock.
    Empty {
        suggestions: Vec<UnlockSuggestion>,
    },
}

/// How well a pet matches a slot's requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchQuality {
    /// Matches element AND class exactly.
    Exact,
    /// Matches element, not yet evolved but recommended class fits. Could evolve.
    Evolvable,
    /// Matches element, evolved into a different class, but the pet is a wildcard
    /// or was evolved differently from its recommendation and the needed class is
    /// among its recommended options.
    Reclassable,
    /// Partial match used as last resort for "any" slots.
    Fallback,
}

/// Suggestion for a pet to unlock that would fill a gap.
#[derive(Debug, Clone)]
pub struct UnlockSuggestion {
    pub pet: MergedPet,
    pub unlock_difficulty: UnlockDifficulty,
}

/// Rough difficulty of unlocking a pet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnlockDifficulty {
    /// Pet Token — easiest.
    Token,
    /// Defeat a specific P.Baal level.
    PBaal,
    /// Special task, milestone, secret, or other involved condition.
    Complex,
}

/// A warning about missing event/trap coverage.
#[derive(Debug, Clone)]
pub struct CoverageWarning {
    pub source_depth: u8,
    pub kind: CoverageKind,
    pub name: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub enum CoverageKind {
    Trap,
    Event,
}

// =============================================================================
// Custom constraints
// =============================================================================

/// User-defined constraints for the dungeon solver.
#[derive(Debug, Clone, Default)]
pub struct SolverConstraints {
    /// Pet names forbidden from all dungeon teams.
    pub forbidden: HashSet<String>,
    /// Pets forced into specific dungeon teams: dungeon → list of pet names.
    pub forced: HashMap<Dungeon, Vec<String>>,
    /// Pets forced into any available team — solver picks the best dungeon/slot.
    pub forced_any: Vec<String>,
    /// Whitelisted pets: bypass the non-dungeon class filter.
    /// These pets won't be automatically excluded from "any" class slots even
    /// if they have a non-dungeon class (Adventurer/Blacksmith/Alchemist).
    pub whitelisted: HashSet<String>,
}

// =============================================================================
// Dungeon class viability
// =============================================================================

/// Non-dungeon classes that should not fill "any" class slots in dungeons.
/// These classes generally don't gain class experience from dungeon runs.
const NON_DUNGEON_CLASSES: &[Class] = &[Class::Adventurer, Class::Blacksmith, Class::Alchemist];

/// Whether a pet is viable for dungeon "any" class slots.
///
/// Returns false for pets evolved as or recommended for non-dungeon classes
/// (Adventurer, Blacksmith, Alchemist), since those classes don't benefit
/// from dungeon runs. When a slot *specifically* requires one of these classes
/// (e.g. Volcano wanting Blacksmiths), the viability check is bypassed.
fn is_dungeon_viable(pet: &MergedPet) -> bool {
    if let Some(class) = pet.evolved_class() {
        return !NON_DUNGEON_CLASSES.contains(&class);
    }
    // Unevolved: check recommended class
    match pet.recommended_class() {
        Some(RecommendedClass::Single(c)) => !NON_DUNGEON_CLASSES.contains(c),
        Some(RecommendedClass::Dual(a, b)) => {
            // Viable if at least one recommended class is a dungeon class
            !NON_DUNGEON_CLASSES.contains(a) || !NON_DUNGEON_CLASSES.contains(b)
        }
        Some(
            RecommendedClass::Wildcard
            | RecommendedClass::DungeonWildcard
            | RecommendedClass::AllClasses,
        ) => true,
        Some(
            RecommendedClass::Village(_)
            | RecommendedClass::Special
            | RecommendedClass::Alternates,
        ) => false,
        None => true, // No wiki data — assume viable
    }
}

// =============================================================================
// Single-dungeon solver (backward-compatible)
// =============================================================================

/// Solve the dungeon party assignment problem for a single dungeon.
pub fn solve(
    dungeon: Dungeon,
    target_depth: u8,
    dungeon_data: &DungeonData,
    roster: &[MergedPet],
) -> DungeonPlan {
    let requests = [DungeonRequest {
        dungeon,
        depth: target_depth,
        data: dungeon_data,
    }];
    let mut plans = solve_multi(&requests, roster, &SolverConstraints::default());
    plans.pop().unwrap_or(DungeonPlan {
        dungeon,
        depth: target_depth,
        assignments: Vec::new(),
        warnings: Vec::new(),
    })
}

// =============================================================================
// Multi-dungeon solver
// =============================================================================

/// A request to solve a single dungeon at a specific depth.
pub struct DungeonRequest<'a> {
    pub dungeon: Dungeon,
    pub depth: u8,
    pub data: &'a DungeonData,
}

/// Solve multiple dungeons simultaneously, ensuring no pet appears in more than
/// one team. Uses a greedy constraint-first approach across all dungeons: the
/// most constrained slots (fewest viable candidates) are filled first.
///
/// Respects custom constraints:
/// - **Forbidden** pets are excluded entirely from the available pool.
/// - **Forced** pets are pre-assigned to the best-fitting slot in their target
///   dungeon before the greedy solve runs for remaining slots.
pub fn solve_multi(
    requests: &[DungeonRequest],
    roster: &[MergedPet],
    constraints: &SolverConstraints,
) -> Vec<DungeonPlan> {
    // Only consider unlocked pets that aren't village pets or forbidden
    let available: Vec<&MergedPet> = roster
        .iter()
        .filter(|p| {
            p.is_unlocked()
                && !p.is_village_pet()
                && !constraints.forbidden.contains(&p.name)
        })
        .collect();

    // Resolve depth data for each request
    let depth_datas: Vec<Option<&DepthData>> = requests
        .iter()
        .map(|req| req.data.depths.get(&req.depth))
        .collect();

    // Track used pets and pre-assigned slots
    let mut used: HashSet<usize> = HashSet::new();
    let mut assignment_map: HashMap<(usize, usize), SlotAssignment> = HashMap::new();

    // Phase 1: Pre-assign forced pets to their target dungeons
    for (ri, req) in requests.iter().enumerate() {
        let Some(dd) = depth_datas[ri] else { continue };
        let Some(forced_names) = constraints.forced.get(&req.dungeon) else {
            continue;
        };

        for forced_name in forced_names {
            // Find this pet in the available pool
            let Some((pi, pet)) = available
                .iter()
                .enumerate()
                .find(|(pi, p)| !used.contains(pi) && p.name == *forced_name)
            else {
                continue;
            };

            // Find the best slot for this pet in the dungeon
            let best_slot = dd
                .party
                .iter()
                .enumerate()
                .filter(|(si, _)| !assignment_map.contains_key(&(ri, *si)))
                .filter_map(|(si, slot)| score_pet(pet, slot, true).map(|q| (si, q)))
                .min_by_key(|(_, q)| *q);

            // If no slot matches constraints, force into the first open slot anyway
            // (user explicitly forced this pet — respect the intent)
            let (slot_idx, quality) = best_slot.unwrap_or_else(|| {
                let first_open = dd
                    .party
                    .iter()
                    .enumerate()
                    .find(|(si, _)| !assignment_map.contains_key(&(ri, *si)))
                    .map(|(si, _)| si)
                    .unwrap_or(0);
                (first_open, MatchQuality::Fallback)
            });

            used.insert(pi);
            assignment_map.insert(
                (ri, slot_idx),
                SlotAssignment {
                    slot: dd.party[slot_idx].clone(),
                    position: slot_idx,
                    assignment: Assignment::Filled {
                        pet: (*pet).clone(),
                        quality,
                    },
                    equipment_suggestion: None,
                },
            );
        }
    }

    // Phase 1b: Pre-assign forced_any pets (solver picks best dungeon/slot)
    for forced_name in &constraints.forced_any {
        // Find this pet in the available pool
        let Some((pi, pet)) = available
            .iter()
            .enumerate()
            .find(|(pi, p)| !used.contains(pi) && p.name == *forced_name)
        else {
            continue;
        };

        // Find the best (request, slot) pair across all dungeons
        let mut best: Option<(usize, usize, MatchQuality)> = None;
        let mut first_open: Option<(usize, usize)> = None;

        for (ri, dd_opt) in depth_datas.iter().enumerate() {
            let Some(dd) = dd_opt else { continue };
            for (si, slot) in dd.party.iter().enumerate() {
                if assignment_map.contains_key(&(ri, si)) {
                    continue;
                }
                if first_open.is_none() {
                    first_open = Some((ri, si));
                }
                if let Some(q) = score_pet(pet, slot, true)
                    && best.as_ref().is_none_or(|(_, _, bq)| q < *bq)
                {
                    best = Some((ri, si, q));
                }
            }
        }

        let target = best
            .or_else(|| first_open.map(|(ri, si)| (ri, si, MatchQuality::Fallback)));

        if let Some((ri, si, quality)) = target {
            let dd = depth_datas[ri].unwrap();
            used.insert(pi);
            assignment_map.insert(
                (ri, si),
                SlotAssignment {
                    slot: dd.party[si].clone(),
                    position: si,
                    assignment: Assignment::Filled {
                        pet: (*pet).clone(),
                        quality,
                    },
                    equipment_suggestion: None,
                },
            );
        }
    }

    // Phase 2: Flatten remaining slots across all dungeons with their candidates
    struct GlobalSlot<'a> {
        req_idx: usize,
        slot_idx: usize,
        slot: &'a PartySlot,
        candidates: Vec<(usize, MatchQuality)>,
    }

    let mut all_slots: Vec<GlobalSlot> = Vec::new();

    for (ri, dd_opt) in depth_datas.iter().enumerate() {
        if let Some(dd) = dd_opt {
            for (si, slot) in dd.party.iter().enumerate() {
                // Skip slots already filled by forced pets
                if assignment_map.contains_key(&(ri, si)) {
                    continue;
                }
                let candidates: Vec<(usize, MatchQuality)> = available
                    .iter()
                    .enumerate()
                    .filter(|(pi, _)| !used.contains(pi))
                    .filter_map(|(pi, pet)| {
                        let wl = constraints.whitelisted.contains(&pet.name);
                        score_pet(pet, slot, wl).map(|q| (pi, q))
                    })
                    .collect();
                all_slots.push(GlobalSlot {
                    req_idx: ri,
                    slot_idx: si,
                    slot,
                    candidates,
                });
            }
        }
    }

    // Sort by constraint strictness: fewest candidates first.
    // Ties broken by request index then slot index for stability.
    all_slots.sort_by_key(|s| (s.candidates.len(), s.req_idx, s.slot_idx));

    // Phase 3: Greedy assignment across all dungeons
    for gs in &all_slots {
        let best = gs
            .candidates
            .iter()
            .filter(|(pi, _)| !used.contains(pi))
            .min_by(|(pi_a, qa), (pi_b, qb)| {
                qa.cmp(qb).then_with(|| {
                    // For Evolvable matches, prefer easier evo difficulty
                    if *qa == MatchQuality::Evolvable {
                        let evo_a = available[*pi_a]
                            .wiki
                            .as_ref()
                            .map(|w| (w.evo_difficulty.base, w.evo_difficulty.with_conditions))
                            .unwrap_or((99, 99));
                        let evo_b = available[*pi_b]
                            .wiki
                            .as_ref()
                            .map(|w| (w.evo_difficulty.base, w.evo_difficulty.with_conditions))
                            .unwrap_or((99, 99));
                        let evo_cmp = evo_a.cmp(&evo_b);
                        if evo_cmp != std::cmp::Ordering::Equal {
                            return evo_cmp;
                        }
                    }
                    // Higher growth = better tiebreak (prefer stronger pets)
                    let ga = available[*pi_a]
                        .export
                        .as_ref()
                        .map(|e| e.growth)
                        .unwrap_or(0);
                    let gb = available[*pi_b]
                        .export
                        .as_ref()
                        .map(|e| e.growth)
                        .unwrap_or(0);
                    gb.cmp(&ga)
                })
            });

        let assignment = if let Some(&(pet_idx, quality)) = best {
            used.insert(pet_idx);
            Assignment::Filled {
                pet: available[pet_idx].clone(),
                quality,
            }
        } else {
            // Generate unlock suggestions from the full roster
            let suggestions = suggest_unlocks_for_slot(gs.slot, roster);
            Assignment::Empty { suggestions }
        };

        assignment_map.insert(
            (gs.req_idx, gs.slot_idx),
            SlotAssignment {
                slot: gs.slot.clone(),
                position: gs.slot_idx,
                assignment,
                equipment_suggestion: None,
            },
        );
    }

    // Build DungeonPlans for each request
    requests
        .iter()
        .enumerate()
        .map(|(ri, req)| {
            let Some(dd) = depth_datas[ri] else {
                return DungeonPlan {
                    dungeon: req.dungeon,
                    depth: req.depth,
                    assignments: Vec::new(),
                    warnings: vec![CoverageWarning {
                        source_depth: req.depth,
                        kind: CoverageKind::Event,
                        name: "Unknown depth".to_string(),
                        detail: format!("No data for depth {}", req.depth),
                    }],
                };
            };

            let assignments: Vec<SlotAssignment> = (0..dd.party.len())
                .map(|si| {
                    assignment_map
                        .remove(&(ri, si))
                        .unwrap_or_else(|| SlotAssignment {
                            slot: dd.party[si].clone(),
                            position: si,
                            assignment: Assignment::Empty {
                                suggestions: Vec::new(),
                            },
                            equipment_suggestion: None,
                        })
                })
                .collect();

            let assigned_pets: Vec<&MergedPet> = assignments
                .iter()
                .filter_map(|a| match &a.assignment {
                    Assignment::Filled { pet, .. } => Some(pet),
                    Assignment::Empty { .. } => None,
                })
                .collect();

            let warnings = check_coverage(req.data, req.depth, &assigned_pets);

            DungeonPlan {
                dungeon: req.dungeon,
                depth: req.depth,
                assignments,
                warnings,
            }
        })
        .collect()
}

// =============================================================================
// Scoring
// =============================================================================

/// Score how well a pet matches a slot. Returns None if completely unsuitable.
fn score_pet(pet: &MergedPet, slot: &PartySlot, whitelisted: bool) -> Option<MatchQuality> {
    let element_ok = pet.matches_element(slot.element);
    if !element_ok {
        return None;
    }

    // If slot has no class requirement, any dungeon-viable pet works
    let Some(required_class) = &slot.class else {
        // "Any" class slot — exclude non-dungeon classes (unless whitelisted)
        if !whitelisted && !is_dungeon_viable(pet) {
            return None;
        }
        if pet.is_evolved() {
            return Some(MatchQuality::Exact);
        }
        return Some(MatchQuality::Evolvable);
    };

    // Exact: evolved into the required class + element matches
    if let Some(actual_class) = pet.evolved_class() {
        if actual_class == *required_class {
            return Some(MatchQuality::Exact);
        }

        // Reclassable: evolved into something else, but the pet is a wildcard type
        // or was evolved differently from recommendation and the needed class is
        // among its recommended options. Village pets are excluded by the caller.
        if pet.recommends_class(required_class) {
            return Some(MatchQuality::Reclassable);
        }

        // Evolved into the wrong class and doesn't recommend the target — skip
        return None;
    }

    // Unevolved: check if the pet's recommended class includes what we need
    if pet.recommends_class(required_class) {
        return Some(MatchQuality::Evolvable);
    }

    None
}

// =============================================================================
// Unlock suggestions
// =============================================================================

/// Generate unlock suggestions from the full merged pet list (including locked pets).
pub fn suggest_unlocks_for_slot(
    slot: &PartySlot,
    all_pets: &[MergedPet],
) -> Vec<UnlockSuggestion> {
    let mut suggestions: Vec<UnlockSuggestion> = all_pets
        .iter()
        .filter(|pet| {
            // Must not be unlocked
            !pet.is_unlocked()
            // Must not be a village pet
            && !pet.is_village_pet()
            // Must match element
            && pet.matches_element(slot.element)
            // Must recommend the required class (if any)
            && slot.class.as_ref().is_none_or(|c| pet.recommends_class(c))
            // For "any" class slots, must be dungeon-viable
            && (slot.class.is_some() || is_dungeon_viable(pet))
        })
        .map(|pet| UnlockSuggestion {
            pet: pet.clone(),
            unlock_difficulty: classify_unlock(&pet.wiki),
        })
        .collect();

    // Sort by unlock difficulty (easiest first), then by evo difficulty
    suggestions.sort_by(|a, b| {
        a.unlock_difficulty.cmp(&b.unlock_difficulty).then_with(|| {
            let evo_a = a.pet.wiki.as_ref().map(|w| w.evo_difficulty.base).unwrap_or(99);
            let evo_b = b.pet.wiki.as_ref().map(|w| w.evo_difficulty.base).unwrap_or(99);
            evo_a.cmp(&evo_b)
        })
    });

    suggestions
}

fn classify_unlock(wiki: &Option<WikiPet>) -> UnlockDifficulty {
    match wiki.as_ref().map(|w| &w.unlock_condition) {
        Some(UnlockCondition::PetToken) => UnlockDifficulty::Token,
        Some(UnlockCondition::MilestonesOrPetToken) => UnlockDifficulty::Token,
        Some(UnlockCondition::DefeatPBaal(_)) => UnlockDifficulty::PBaal,
        Some(UnlockCondition::DefeatPBaalVersion(_)) => UnlockDifficulty::PBaal,
        _ => UnlockDifficulty::Complex,
    }
}

// =============================================================================
// Coverage checking
// =============================================================================

/// Check event/trap coverage across all sub-depths up to the target.
///
/// When running D2, you first clear all D1 rooms. When running D3, you clear
/// D1 and D2 rooms first. So events/traps from lower depths can still occur.
fn check_coverage(
    dungeon_data: &DungeonData,
    target_depth: u8,
    team: &[&MergedPet],
) -> Vec<CoverageWarning> {
    let mut warnings = Vec::new();

    for depth in 1..=target_depth {
        let Some(dd) = dungeon_data.depths.get(&depth) else {
            continue;
        };

        // Check traps
        for trap in &dd.traps {
            if !counter_satisfied(&trap.countered_by, team) {
                warnings.push(CoverageWarning {
                    source_depth: depth,
                    kind: CoverageKind::Trap,
                    name: trap.name.clone(),
                    detail: describe_counter(&trap.countered_by),
                });
            }
        }

        // Check events
        for event in &dd.events {
            for condition in &event.countered_by {
                if !counter_satisfied(condition, team) {
                    warnings.push(CoverageWarning {
                        source_depth: depth,
                        kind: CoverageKind::Event,
                        name: event.name.clone(),
                        detail: describe_counter(condition),
                    });
                }
            }
        }
    }

    warnings
}

/// Check if a counter condition is satisfied by the team.
fn counter_satisfied(condition: &CounterCondition, team: &[&MergedPet]) -> bool {
    // Item-only conditions are satisfied by bringing items, not by team composition
    if condition.item.is_some() && condition.class.is_none() && condition.element.is_none() {
        return true; // Items are handled separately
    }

    let required_count = condition.count.unwrap_or(1) as usize;

    let matching = team.iter().filter(|pet| {
        let class_ok = condition.class.as_ref().is_none_or(|c| {
            pet.evolved_class().as_ref() == Some(c)
        });
        let element_ok = condition.element.as_ref().is_none_or(|el| {
            pet.matches_element(Some(*el))
        });
        class_ok && element_ok
    }).count();

    matching >= required_count
}

/// Produce a human-readable description of what a counter condition requires.
fn describe_counter(condition: &CounterCondition) -> String {
    let mut parts = Vec::new();

    if let Some(class) = &condition.class {
        parts.push(format!("{class:?}"));
    }
    if let Some(element) = &condition.element {
        if let Some(count) = condition.count {
            parts.push(format!("{count}x {element:?}"));
        } else {
            parts.push(format!("{element:?}"));
        }
    }
    if let Some(item) = &condition.item {
        parts.push(format!("item: {item}"));
    }

    if parts.is_empty() {
        "unknown requirement".to_string()
    } else {
        format!("Needs {}", parts.join(" + "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merge::MergedPet;
    #[allow(unused_imports)]
    use itrtg_models::*;

    fn mock_pet(
        name: &str,
        element: Element,
        class: Option<Class>,
        rec: RecommendedClass,
        unlocked: bool,
    ) -> MergedPet {
        mock_pet_with_evo(name, element, class, rec, unlocked, 1, 1, 10000)
    }

    fn mock_pet_with_evo(
        name: &str,
        element: Element,
        class: Option<Class>,
        rec: RecommendedClass,
        unlocked: bool,
        evo_base: u8,
        evo_cond: u8,
        growth: u64,
    ) -> MergedPet {
        let wiki = WikiPet {
            name: name.to_string(),
            wiki_url: String::new(),
            element,
            recommended_class: rec,
            class_bonus: String::new(),
            unlock_condition: UnlockCondition::PetToken,
            evo_difficulty: EvoDifficulty { base: evo_base, with_conditions: evo_cond },
            token_improvable: false,
            special_ability: None,
        };
        let export = ExportPet {
            export_name: name.to_string(),
            element,
            growth,
            dungeon_level: 20,
            class,
            class_level: 10,
            combat_stats: CombatStats { hp: 500, attack: 200, defense: 100, speed: 150 },
            elemental_affinities: ElementalAffinities {
                water: 0, fire: 0, wind: 0, earth: 0, dark: 0, light: 0,
            },
            loadout: Loadout { weapon: None, armor: None, accessory: None },
            action: PetAction::Idle,
            unlocked,
            improved: false,
            other: None,
            has_partner: false,
        };
        MergedPet {
            name: name.to_string(),
            wiki: Some(wiki),
            export: Some(export),
        }
    }

    fn make_slot(class: Option<Class>, element: Option<Element>) -> PartySlot {
        PartySlot {
            class,
            element,
            equipment: None,
        }
    }

    #[test]
    fn test_exact_match_preferred() {
        let pets = vec![
            mock_pet("Frog", Element::Water, Some(Class::Supporter), RecommendedClass::Single(Class::Supporter), true),
            mock_pet("Mouse", Element::Earth, None, RecommendedClass::Wildcard, true),
        ];
        let available: Vec<&MergedPet> = pets.iter().collect();

        let slot = make_slot(Some(Class::Supporter), Some(Element::Water));
        let score = score_pet(&available[0], &slot, false);
        assert_eq!(score, Some(MatchQuality::Exact));
    }

    #[test]
    fn test_evolvable_match() {
        let pet = mock_pet("Rabbit", Element::Earth, None, RecommendedClass::Single(Class::Mage), true);
        let slot = make_slot(Some(Class::Mage), Some(Element::Earth));
        assert_eq!(score_pet(&pet, &slot, false), Some(MatchQuality::Evolvable));
    }

    #[test]
    fn test_element_mismatch_rejected() {
        let pet = mock_pet("Frog", Element::Water, Some(Class::Supporter), RecommendedClass::Single(Class::Supporter), true);
        let slot = make_slot(Some(Class::Supporter), Some(Element::Fire));
        assert_eq!(score_pet(&pet, &slot, false), None);
    }

    #[test]
    fn test_village_pet_excluded() {
        let pet = mock_pet("Swan", Element::Water, None, RecommendedClass::Village("Fisher".to_string()), true);
        assert!(!pet.recommends_class(&Class::Supporter));
        assert!(pet.is_village_pet());
    }

    #[test]
    fn test_wildcard_reclassable() {
        let pet = mock_pet("Mouse", Element::Earth, Some(Class::Adventurer), RecommendedClass::Wildcard, true);
        let slot = make_slot(Some(Class::Defender), Some(Element::Earth));
        assert_eq!(score_pet(&pet, &slot, false), Some(MatchQuality::Reclassable));
    }

    #[test]
    fn test_any_slot_accepts_dungeon_class() {
        let pet = mock_pet("Dog", Element::Neutral, Some(Class::Defender), RecommendedClass::Single(Class::Defender), true);
        let slot = make_slot(None, None);
        assert_eq!(score_pet(&pet, &slot, false), Some(MatchQuality::Exact));
    }

    #[test]
    fn test_non_dungeon_class_excluded_from_any_slot() {
        let pet = mock_pet("Mouse", Element::Earth, Some(Class::Adventurer), RecommendedClass::Wildcard, true);
        let slot = make_slot(None, None);
        assert_eq!(score_pet(&pet, &slot, false), None);
    }

    #[test]
    fn test_whitelisted_non_dungeon_class_allowed() {
        // Whitelisted pets bypass the non-dungeon class filter for "any" slots
        let pet = mock_pet("Bee", Element::Wind, Some(Class::Alchemist), RecommendedClass::Single(Class::Alchemist), true);
        let slot = make_slot(None, None);
        assert_eq!(score_pet(&pet, &slot, false), None); // Normally excluded
        assert_eq!(score_pet(&pet, &slot, true), Some(MatchQuality::Exact)); // Whitelisted: allowed
    }

    #[test]
    fn test_non_dungeon_class_allowed_when_required() {
        let pet = mock_pet("Smith", Element::Fire, Some(Class::Blacksmith), RecommendedClass::Single(Class::Blacksmith), true);
        let slot = make_slot(Some(Class::Blacksmith), Some(Element::Fire));
        assert_eq!(score_pet(&pet, &slot, false), Some(MatchQuality::Exact));
    }

    #[test]
    fn test_chameleon_matches_any_element() {
        let pet = mock_pet("Chameleon", Element::All, Some(Class::Mage), RecommendedClass::DungeonWildcard, true);
        let slot = make_slot(Some(Class::Mage), Some(Element::Fire));
        assert_eq!(score_pet(&pet, &slot, false), Some(MatchQuality::Exact));
    }

    #[test]
    fn test_party_assignment_preserves_order() {
        let pets = vec![
            mock_pet("Frog", Element::Water, Some(Class::Supporter), RecommendedClass::Single(Class::Supporter), true),
            mock_pet("Cat", Element::Neutral, Some(Class::Assassin), RecommendedClass::Single(Class::Assassin), true),
            mock_pet("Rabbit", Element::Earth, Some(Class::Mage), RecommendedClass::Single(Class::Mage), true),
            mock_pet("Squirrel", Element::Fire, Some(Class::Rogue), RecommendedClass::Single(Class::Rogue), true),
            mock_pet("Dog", Element::Neutral, Some(Class::Defender), RecommendedClass::Single(Class::Defender), true),
            mock_pet("Mouse", Element::Earth, None, RecommendedClass::Wildcard, true),
        ];

        let dungeon_data = DungeonData {
            name: "Test".to_string(),
            depths: {
                let mut m = std::collections::BTreeMap::new();
                m.insert(1, DepthData {
                    rooms: 10,
                    monsters_per_room: 3,
                    gem_level: None,
                    requirements: DepthRequirements {
                        dungeon_level_avg: 10,
                        levels_per_difficulty: vec![1, 2],
                        class_level: 5,
                        total_growth: None,
                    },
                    monsters: Vec::new(),
                    bosses: vec![MonsterEntry {
                        name: "Boss".to_string(),
                        element: None,
                        hp: 100, att: 50, def: 30, spd: 20,
                    }],
                    party: vec![
                        make_slot(Some(Class::Rogue), None),
                        make_slot(Some(Class::Assassin), None),
                        make_slot(Some(Class::Defender), None),
                        make_slot(Some(Class::Supporter), None),
                        make_slot(Some(Class::Mage), None),
                        make_slot(None, None),
                    ],
                    party_items: Vec::new(),
                    traps: Vec::new(),
                    events: Vec::new(),
                });
                m
            },
        };

        let plan = solve(Dungeon::Scrapyard, 1, &dungeon_data, &pets);

        for (i, a) in plan.assignments.iter().enumerate() {
            assert_eq!(a.position, i);
        }

        assert!(matches!(&plan.assignments[0].assignment,
            Assignment::Filled { pet, quality: MatchQuality::Exact } if pet.evolved_class() == Some(Class::Rogue)));
        assert!(matches!(&plan.assignments[1].assignment,
            Assignment::Filled { pet, quality: MatchQuality::Exact } if pet.evolved_class() == Some(Class::Assassin)));
        assert!(matches!(&plan.assignments[2].assignment,
            Assignment::Filled { pet, quality: MatchQuality::Exact } if pet.evolved_class() == Some(Class::Defender)));
    }

    #[test]
    fn test_multi_solve_no_pet_reuse() {
        let pets = vec![
            mock_pet("Frog", Element::Water, Some(Class::Supporter), RecommendedClass::Single(Class::Supporter), true),
            mock_pet("Cat", Element::Neutral, Some(Class::Assassin), RecommendedClass::Single(Class::Assassin), true),
            mock_pet("Dog", Element::Neutral, Some(Class::Defender), RecommendedClass::Single(Class::Defender), true),
        ];

        let dd1 = DungeonData {
            name: "Dungeon1".to_string(),
            depths: {
                let mut m = std::collections::BTreeMap::new();
                m.insert(1, DepthData {
                    rooms: 5, monsters_per_room: 2, gem_level: None,
                    requirements: DepthRequirements {
                        dungeon_level_avg: 5, levels_per_difficulty: vec![1, 2],
                        class_level: 3, total_growth: None,
                    },
                    monsters: Vec::new(),
                    bosses: vec![MonsterEntry { name: "B".into(), element: None, hp: 50, att: 20, def: 10, spd: 10 }],
                    party: vec![make_slot(Some(Class::Supporter), None)],
                    party_items: Vec::new(), traps: Vec::new(), events: Vec::new(),
                });
                m
            },
        };
        let dd2 = dd1.clone();

        let requests = [
            DungeonRequest { dungeon: Dungeon::Scrapyard, depth: 1, data: &dd1 },
            DungeonRequest { dungeon: Dungeon::WaterTemple, depth: 1, data: &dd2 },
        ];

        let plans = solve_multi(&requests, &pets, &SolverConstraints::default());
        assert_eq!(plans.len(), 2);

        assert!(matches!(&plans[0].assignments[0].assignment,
            Assignment::Filled { pet, .. } if pet.name == "Frog"));
        assert!(matches!(&plans[1].assignments[0].assignment,
            Assignment::Empty { .. }));
    }

    #[test]
    fn test_evolvable_prefers_easier_evo() {
        // Two unevolved pets, both recommend Mage, but different evo difficulties
        let pets = vec![
            mock_pet_with_evo("HardEvo", Element::Fire, None, RecommendedClass::Single(Class::Mage), true, 7, 9, 50000),
            mock_pet_with_evo("EasyEvo", Element::Fire, None, RecommendedClass::Single(Class::Mage), true, 2, 3, 10000),
        ];

        let dd = DungeonData {
            name: "Test".to_string(),
            depths: {
                let mut m = std::collections::BTreeMap::new();
                m.insert(1, DepthData {
                    rooms: 5, monsters_per_room: 2, gem_level: None,
                    requirements: DepthRequirements {
                        dungeon_level_avg: 5, levels_per_difficulty: vec![1, 2],
                        class_level: 3, total_growth: None,
                    },
                    monsters: Vec::new(),
                    bosses: vec![MonsterEntry { name: "B".into(), element: None, hp: 50, att: 20, def: 10, spd: 10 }],
                    party: vec![make_slot(Some(Class::Mage), Some(Element::Fire))],
                    party_items: Vec::new(), traps: Vec::new(), events: Vec::new(),
                });
                m
            },
        };

        let plan = solve(Dungeon::Scrapyard, 1, &dd, &pets);
        // EasyEvo should be preferred despite lower growth, because evo difficulty is lower
        assert!(matches!(&plan.assignments[0].assignment,
            Assignment::Filled { pet, quality: MatchQuality::Evolvable } if pet.name == "EasyEvo"));
    }

    #[test]
    fn test_forbidden_pets_excluded() {
        let pets = vec![
            mock_pet("Frog", Element::Water, Some(Class::Supporter), RecommendedClass::Single(Class::Supporter), true),
            mock_pet("Cat", Element::Water, Some(Class::Supporter), RecommendedClass::Single(Class::Supporter), true),
        ];

        let dd = DungeonData {
            name: "Test".to_string(),
            depths: {
                let mut m = std::collections::BTreeMap::new();
                m.insert(1, DepthData {
                    rooms: 5, monsters_per_room: 2, gem_level: None,
                    requirements: DepthRequirements {
                        dungeon_level_avg: 5, levels_per_difficulty: vec![1, 2],
                        class_level: 3, total_growth: None,
                    },
                    monsters: Vec::new(),
                    bosses: vec![MonsterEntry { name: "B".into(), element: None, hp: 50, att: 20, def: 10, spd: 10 }],
                    party: vec![make_slot(Some(Class::Supporter), None)],
                    party_items: Vec::new(), traps: Vec::new(), events: Vec::new(),
                });
                m
            },
        };

        let mut constraints = SolverConstraints::default();
        constraints.forbidden.insert("Frog".to_string());

        let requests = [DungeonRequest { dungeon: Dungeon::Scrapyard, depth: 1, data: &dd }];
        let plans = solve_multi(&requests, &pets, &constraints);

        // Frog is forbidden, so Cat should be assigned
        assert!(matches!(&plans[0].assignments[0].assignment,
            Assignment::Filled { pet, .. } if pet.name == "Cat"));
    }

    #[test]
    fn test_forced_pets_assigned() {
        let pets = vec![
            mock_pet("Frog", Element::Water, Some(Class::Supporter), RecommendedClass::Single(Class::Supporter), true),
            mock_pet("Cat", Element::Water, Some(Class::Supporter), RecommendedClass::Single(Class::Supporter), true),
        ];

        let dd = DungeonData {
            name: "Test".to_string(),
            depths: {
                let mut m = std::collections::BTreeMap::new();
                m.insert(1, DepthData {
                    rooms: 5, monsters_per_room: 2, gem_level: None,
                    requirements: DepthRequirements {
                        dungeon_level_avg: 5, levels_per_difficulty: vec![1, 2],
                        class_level: 3, total_growth: None,
                    },
                    monsters: Vec::new(),
                    bosses: vec![MonsterEntry { name: "B".into(), element: None, hp: 50, att: 20, def: 10, spd: 10 }],
                    party: vec![make_slot(Some(Class::Supporter), None)],
                    party_items: Vec::new(), traps: Vec::new(), events: Vec::new(),
                });
                m
            },
        };

        let mut constraints = SolverConstraints::default();
        constraints.forced.insert(Dungeon::Scrapyard, vec!["Cat".to_string()]);

        let requests = [DungeonRequest { dungeon: Dungeon::Scrapyard, depth: 1, data: &dd }];
        let plans = solve_multi(&requests, &pets, &constraints);

        // Cat is forced, so Cat should be assigned even though Frog might be "better"
        assert!(matches!(&plans[0].assignments[0].assignment,
            Assignment::Filled { pet, .. } if pet.name == "Cat"));
    }
}
