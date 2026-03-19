use std::collections::HashSet;

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
}

/// What was assigned to a slot.
#[derive(Debug, Clone)]
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
// Solver
// =============================================================================

/// Solve the dungeon party assignment problem.
///
/// Given the recommendations for a dungeon+depth, the player's merged pet roster,
/// and the full dungeon data (needed for sub-depth coverage checks), produce a plan.
pub fn solve(
    dungeon: Dungeon,
    target_depth: u8,
    dungeon_data: &DungeonData,
    roster: &[MergedPet],
) -> DungeonPlan {
    let depth_data = match dungeon_data.depths.get(&target_depth) {
        Some(d) => d,
        None => {
            return DungeonPlan {
                dungeon,
                depth: target_depth,
                assignments: Vec::new(),
                warnings: vec![CoverageWarning {
                    source_depth: target_depth,
                    kind: CoverageKind::Event,
                    name: "Unknown depth".to_string(),
                    detail: format!("No data for depth {target_depth}"),
                }],
            };
        }
    };

    // Only consider unlocked pets that aren't village pets
    let available: Vec<&MergedPet> = roster
        .iter()
        .filter(|p| p.is_unlocked() && !p.is_village_pet())
        .collect();

    let assignments = assign_party(&depth_data.party, &available);

    // Check event/trap coverage across all depths up to and including target
    let assigned_pets: Vec<&MergedPet> = assignments
        .iter()
        .filter_map(|a| match &a.assignment {
            Assignment::Filled { pet, .. } => Some(pet),
            Assignment::Empty { .. } => None,
        })
        .collect();

    let warnings = check_coverage(dungeon_data, target_depth, &assigned_pets);

    DungeonPlan {
        dungeon,
        depth: target_depth,
        assignments,
        warnings,
    }
}

/// Score and assign pets to party slots using greedy constraint-first ordering.
fn assign_party(party_slots: &[PartySlot], available: &[&MergedPet]) -> Vec<SlotAssignment> {
    let n = party_slots.len();

    // Build scored candidates for each slot
    let mut slot_candidates: Vec<(usize, &PartySlot, Vec<(usize, MatchQuality)>)> = party_slots
        .iter()
        .enumerate()
        .map(|(i, slot)| {
            let candidates: Vec<(usize, MatchQuality)> = available
                .iter()
                .enumerate()
                .filter_map(|(pi, pet)| score_pet(pet, slot).map(|q| (pi, q)))
                .collect();
            (i, slot, candidates)
        })
        .collect();

    // Sort by constraint strictness: most constrained first (fewest candidates).
    // But maintain stable order for ties — we want the original party ordering to
    // act as a tiebreaker so front-row slots are filled first among equals.
    slot_candidates.sort_by_key(|(orig_idx, _, cands)| (cands.len(), *orig_idx));

    let mut used: HashSet<usize> = HashSet::new();
    let mut results: Vec<(usize, SlotAssignment)> = Vec::with_capacity(n);

    for (orig_idx, slot, candidates) in &slot_candidates {
        // Among unused candidates, pick the best quality, then highest growth as tiebreak
        let best = candidates
            .iter()
            .filter(|(pi, _)| !used.contains(pi))
            .min_by(|(pi_a, qa), (pi_b, qb)| {
                qa.cmp(qb).then_with(|| {
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
                    gb.cmp(&ga) // reverse: higher growth first
                })
            });

        let assignment = if let Some(&(pet_idx, quality)) = best {
            used.insert(pet_idx);
            Assignment::Filled {
                pet: available[pet_idx].clone(),
                quality,
            }
        } else {
            // No candidate found — generate unlock suggestions
            let suggestions = generate_unlock_suggestions(slot, available, &used);
            Assignment::Empty { suggestions }
        };

        results.push((
            *orig_idx,
            SlotAssignment {
                slot: (*slot).clone(),
                position: *orig_idx,
                assignment,
            },
        ));
    }

    // Restore original party order
    results.sort_by_key(|(orig_idx, _)| *orig_idx);
    results.into_iter().map(|(_, sa)| sa).collect()
}

/// Score how well a pet matches a slot. Returns None if completely unsuitable.
fn score_pet(pet: &MergedPet, slot: &PartySlot) -> Option<MatchQuality> {
    let element_ok = pet.matches_element(slot.element);
    if !element_ok {
        return None;
    }

    // If slot has no class requirement, any pet works
    let Some(required_class) = &slot.class else {
        // Element matches (or slot is "any" element). This is a valid assignment.
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

/// Generate suggestions for pets that could be unlocked to fill a slot.
fn generate_unlock_suggestions(
    slot: &PartySlot,
    all_available: &[&MergedPet],
    used: &HashSet<usize>,
) -> Vec<UnlockSuggestion> {
    // Look at ALL merged pets (including those not unlocked) from the roster
    // We can only work with what we have in the available list, but the caller
    // already filtered to unlocked. For unlock suggestions, we'd need the full
    // roster. For now, return empty — the GUI can populate this from the full
    // merged list.
    let _ = (slot, all_available, used);
    Vec::new()
}

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
            && slot.class.as_ref().map_or(true, |c| pet.recommends_class(c))
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
        let class_ok = condition.class.as_ref().map_or(true, |c| {
            pet.evolved_class().as_ref() == Some(c)
        });
        let element_ok = condition.element.as_ref().map_or(true, |el| {
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
        let wiki = WikiPet {
            name: name.to_string(),
            wiki_url: String::new(),
            element,
            recommended_class: rec,
            class_bonus: String::new(),
            unlock_condition: UnlockCondition::PetToken,
            evo_difficulty: EvoDifficulty { base: 1, with_conditions: 1 },
            token_improvable: false,
            special_ability: None,
        };
        let export = ExportPet {
            export_name: name.to_string(),
            element,
            growth: 10000,
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
        let score = score_pet(&available[0], &slot);
        assert_eq!(score, Some(MatchQuality::Exact));
    }

    #[test]
    fn test_evolvable_match() {
        let pet = mock_pet("Rabbit", Element::Earth, None, RecommendedClass::Single(Class::Mage), true);
        let slot = make_slot(Some(Class::Mage), Some(Element::Earth));
        assert_eq!(score_pet(&pet, &slot), Some(MatchQuality::Evolvable));
    }

    #[test]
    fn test_element_mismatch_rejected() {
        let pet = mock_pet("Frog", Element::Water, Some(Class::Supporter), RecommendedClass::Single(Class::Supporter), true);
        let slot = make_slot(Some(Class::Supporter), Some(Element::Fire));
        assert_eq!(score_pet(&pet, &slot), None);
    }

    #[test]
    fn test_village_pet_excluded() {
        let pet = mock_pet("Swan", Element::Water, None, RecommendedClass::Village("Fisher".to_string()), true);
        // Village pets should be filtered out before reaching the solver,
        // but verify recommends_class returns false
        assert!(!pet.recommends_class(&Class::Supporter));
        assert!(pet.is_village_pet());
    }

    #[test]
    fn test_wildcard_reclassable() {
        // A pet evolved as Adventurer but with Wildcard recommendation — could reclass
        let pet = mock_pet("Mouse", Element::Earth, Some(Class::Adventurer), RecommendedClass::Wildcard, true);
        let slot = make_slot(Some(Class::Defender), Some(Element::Earth));
        assert_eq!(score_pet(&pet, &slot), Some(MatchQuality::Reclassable));
    }

    #[test]
    fn test_any_slot_accepts_anyone() {
        let pet = mock_pet("Mouse", Element::Earth, Some(Class::Adventurer), RecommendedClass::Wildcard, true);
        let slot = make_slot(None, None);
        assert_eq!(score_pet(&pet, &slot), Some(MatchQuality::Exact));
    }

    #[test]
    fn test_chameleon_matches_any_element() {
        let pet = mock_pet("Chameleon", Element::All, Some(Class::Mage), RecommendedClass::DungeonWildcard, true);
        let slot = make_slot(Some(Class::Mage), Some(Element::Fire));
        assert_eq!(score_pet(&pet, &slot), Some(MatchQuality::Exact));
    }

    #[test]
    fn test_party_assignment_preserves_order() {
        let pets = vec![
            mock_pet("Frog", Element::Water, Some(Class::Supporter), RecommendedClass::Single(Class::Supporter), true),
            mock_pet("Cat", Element::Neutral, Some(Class::Assassin), RecommendedClass::Single(Class::Assassin), true),
            mock_pet("Rabbit", Element::Earth, Some(Class::Mage), RecommendedClass::Single(Class::Mage), true),
            mock_pet("Squirrel", Element::Fire, Some(Class::Rogue), RecommendedClass::Single(Class::Rogue), true),
            mock_pet("Dog", Element::Neutral, Some(Class::Defender), RecommendedClass::Single(Class::Defender), true),
            mock_pet("Mouse", Element::Earth, Some(Class::Adventurer), RecommendedClass::Wildcard, true),
        ];
        let available: Vec<&MergedPet> = pets.iter().collect();

        let slots = vec![
            make_slot(Some(Class::Rogue), None),
            make_slot(Some(Class::Assassin), None),
            make_slot(Some(Class::Defender), None),
            make_slot(Some(Class::Supporter), None),
            make_slot(Some(Class::Mage), None),
            make_slot(None, None),
        ];

        let assignments = assign_party(&slots, &available);

        // Verify order is preserved (positions 0-5)
        for (i, a) in assignments.iter().enumerate() {
            assert_eq!(a.position, i);
        }

        // Verify correct class assignments
        assert!(matches!(&assignments[0].assignment,
            Assignment::Filled { pet, quality: MatchQuality::Exact } if pet.evolved_class() == Some(Class::Rogue)));
        assert!(matches!(&assignments[1].assignment,
            Assignment::Filled { pet, quality: MatchQuality::Exact } if pet.evolved_class() == Some(Class::Assassin)));
        assert!(matches!(&assignments[2].assignment,
            Assignment::Filled { pet, quality: MatchQuality::Exact } if pet.evolved_class() == Some(Class::Defender)));
    }
}
