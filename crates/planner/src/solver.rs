use std::collections::{HashMap, HashSet};

use itrtg_models::dungeon::*;
use itrtg_models::planner_config::{PetSpecialInfo, PlannerConfig};
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
    /// A teammate's [`PetSpecialInfo::forbidden_team_equipment`] is present
    /// on another pet in the same party — e.g. Bat loses its class XP bonus
    /// if any teammate has Ego Sword or Gram. The warning carries the
    /// equipment name + which pet is holding it; the planner itself does
    /// not auto-swap gear.
    Equipment,
    /// Two pets with a dungeon-context anti-synergy ended up on the same
    /// team. The greedy solver filters these out, but user-forced
    /// assignments bypass the filter — this variant reports the resulting
    /// conflict so the user can adjust.
    Synergy,
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
///
/// The planner config is optional; when `None`, per-pet special info
/// (anti-synergies, locked classes, class/element wildcards, etc.) is
/// ignored and the solver falls back to the generic wiki-driven behavior.
pub fn solve(
    dungeon: Dungeon,
    target_depth: u8,
    dungeon_data: &DungeonData,
    roster: &[MergedPet],
    config: Option<&PlannerConfig>,
) -> DungeonPlan {
    let requests = [DungeonRequest {
        dungeon,
        depth: target_depth,
        data: dungeon_data,
    }];
    let mut plans = solve_multi(&requests, roster, &SolverConstraints::default(), config);
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
///
/// Also applies per-pet special info when `config` is `Some`:
/// - **Locked class** pets (e.g. Basilisk) can only fill slots requiring that
///   class, and are never considered reclassable.
/// - **Avoid-class** hints downgrade match quality without hard-filtering,
///   so an Ape-as-Mage match still works as a last resort.
/// - **Flexible-class** pets (Holy ITRTG Book, Nothing, Nugget, Gray's
///   children) match any class slot without the usual reclass penalty.
/// - **Anti-synergies** (Hourglass + Undine) prevent co-assignment during
///   greedy selection, and trigger post-solve warnings if two forced pets
///   were pre-assigned into a conflict.
/// - **Positive synergies** act as a tiebreaker during greedy selection.
///
/// Note: `preferred_class` from `pet_special_info.yaml` is *not* consumed
/// here — the wiki's `recommended_class` already drives scoring and the
/// two fields are redundant for every pet currently in the YAML. The
/// typed accessor exists for display purposes.
pub fn solve_multi(
    requests: &[DungeonRequest],
    roster: &[MergedPet],
    constraints: &SolverConstraints,
    config: Option<&PlannerConfig>,
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
                .filter_map(|(si, slot)| score_pet(pet, slot, true, config).map(|q| (si, q)))
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
                if let Some(q) = score_pet(pet, slot, true, config)
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
                        score_pet(pet, slot, wl, config).map(|q| (pi, q))
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
        // Build the current team for this dungeon so we can filter out
        // anti-synergies and score positive synergies against already-placed
        // pets. This is rebuilt per slot because placement is greedy and
        // teammates may have been added by earlier iterations. Slot indices
        // are not necessarily contiguous (forced pets may have landed in
        // any slot), so scan all assignment_map entries for this request.
        let current_team: Vec<&MergedPet> = assignment_map
            .iter()
            .filter(|((ri, _), _)| *ri == gs.req_idx)
            .filter_map(|(_, sa)| match &sa.assignment {
                Assignment::Filled { pet, .. } => Some(pet),
                Assignment::Empty { .. } => None,
            })
            .collect();

        let best = gs
            .candidates
            .iter()
            .filter(|(pi, _)| !used.contains(pi))
            // Hard filter: anti-synergies with pets already on this team.
            .filter(|(pi, _)| {
                config.is_none_or(|cfg| {
                    !conflicts_with_team(available[*pi], &current_team, cfg)
                })
            })
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
                    // Positive-synergy tiebreaker: pets that synergize with
                    // already-placed teammates win over equally-good
                    // candidates that don't. Higher synergy score first.
                    if let Some(cfg) = config {
                        let sa = synergy_score(available[*pi_a], &current_team, cfg);
                        let sb = synergy_score(available[*pi_b], &current_team, cfg);
                        let syn_cmp = sb.cmp(&sa);
                        if syn_cmp != std::cmp::Ordering::Equal {
                            return syn_cmp;
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
            let suggestions = suggest_unlocks_for_slot(gs.slot, roster, config);
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

            let mut warnings =
                check_coverage(req.data, req.depth, &assigned_pets, config);
            // The greedy phase filters anti-synergies out of its candidate
            // pool, but Phase 1 / 1b (forced and forced_any pre-assignment)
            // honors explicit user intent without rejecting the choice.
            // If two forced pets ended up on the same team despite a
            // dungeon-context anti-synergy, surface that to the user as a
            // warning rather than silently overriding their selection.
            if let Some(cfg) = config {
                warnings.extend(check_team_anti_synergies(
                    &assigned_pets,
                    req.depth,
                    cfg,
                ));
            }

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
///
/// When `config` carries special info for the pet, scoring also honors:
/// - `locked_class`: the pet may only fill a slot whose required class
///   matches the lock, and slots without a class requirement still need
///   the locked class to be dungeon-viable (it is by construction).
/// - `avoid_class`: matching into an avoided class degrades the quality to
///   `Reclassable` or `Fallback` — we still consider the pet, but only
///   when nothing better exists.
/// - `flexible_class`: the pet can fill any class slot without the usual
///   `Reclassable` penalty (Holy ITRTG Book, Nothing, Nugget, Gray's
///   children).
fn score_pet(
    pet: &MergedPet,
    slot: &PartySlot,
    whitelisted: bool,
    config: Option<&PlannerConfig>,
) -> Option<MatchQuality> {
    let special = config.and_then(|c| c.special_info(&pet.name));
    let element_ok = matches_slot_element(pet, slot.element, special);
    if !element_ok {
        return None;
    }

    // Locked-class hard filter. If the pet is locked to class X and the
    // slot demands a different class, reject. Slots without a class
    // requirement still accept locked-class pets.
    if let Some(locked) = special.and_then(PetSpecialInfo::locked_class)
        && let Some(required) = slot.class
        && required != locked
    {
        return None;
    }

    // Avoid-class penalty modifier. Used to downgrade an otherwise-exact
    // match when the slot's class is on this pet's avoid list.
    let avoided = slot
        .class
        .zip(special)
        .is_some_and(|(c, info)| info.avoid_classes().any(|ac| ac == c));
    // Downgrades a positive match to `Reclassable` so any equally-qualified
    // non-avoiding candidate wins the head-to-head. Existing `Reclassable`
    // and `Fallback` qualities are passed through unchanged — we never make
    // an avoided match *worse* than the caller's starting quality.
    let apply_avoid = |q: MatchQuality| -> MatchQuality {
        if avoided {
            match q {
                MatchQuality::Exact | MatchQuality::Evolvable => MatchQuality::Reclassable,
                other => other,
            }
        } else {
            q
        }
    };

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

    // Flexible-class pets (Holy ITRTG Book etc.) count as a clean match for
    // any class without the reclass penalty.
    if special.is_some_and(PetSpecialInfo::is_flexible_class) {
        return Some(apply_avoid(if pet.is_evolved() {
            MatchQuality::Exact
        } else {
            MatchQuality::Evolvable
        }));
    }

    // Exact: evolved into the required class + element matches
    if let Some(actual_class) = pet.evolved_class() {
        if actual_class == *required_class {
            return Some(apply_avoid(MatchQuality::Exact));
        }

        // Reclassable: evolved into something else, but the pet is a wildcard type
        // or was evolved differently from recommendation and the needed class is
        // among its recommended options. Village pets are excluded by the caller.
        if pet.recommends_class(required_class) {
            return Some(apply_avoid(MatchQuality::Reclassable));
        }

        // Evolved into the wrong class and doesn't recommend the target — skip
        return None;
    }

    // Unevolved: check if the pet's recommended class includes what we need
    if pet.recommends_class(required_class) {
        return Some(apply_avoid(MatchQuality::Evolvable));
    }

    None
}

/// Element-matching check that also understands multi-element pets
/// (Tödlicher Löffel counts as every non-neutral element) and element
/// wildcards (Chameleon, already handled via `Element::All` but we
/// double-cover for pets whose YAML special info says so explicitly).
fn matches_slot_element(
    pet: &MergedPet,
    slot_element: Option<Element>,
    special: Option<&PetSpecialInfo>,
) -> bool {
    let Some(target) = slot_element else {
        return true; // "any" element slot
    };

    if pet.matches_element(Some(target)) {
        return true;
    }

    if let Some(info) = special {
        if info.is_element_wildcard() {
            return true;
        }
        if info.is_multi_element() && target != Element::Neutral {
            return true;
        }
    }

    false
}

// =============================================================================
// Team synergies / anti-synergies
// =============================================================================

/// Whether placing `candidate` on a team that already contains `team` would
/// violate any dungeon-context anti-synergy. Anti-synergies are checked in
/// both directions — Hourglass listing "don't pair with Undine" means a
/// team containing Undine cannot add Hourglass, and vice versa.
fn conflicts_with_team(
    candidate: &MergedPet,
    team: &[&MergedPet],
    config: &PlannerConfig,
) -> bool {
    let cand_info = config.special_info(&candidate.name);

    for teammate in team {
        // Candidate's anti-synergies mention this teammate.
        if let Some(info) = cand_info
            && info.has_dungeon_anti_synergy_with(&teammate.name)
        {
            return true;
        }
        // Teammate's anti-synergies mention the candidate (symmetric).
        if let Some(tm_info) = config.special_info(&teammate.name)
            && tm_info.has_dungeon_anti_synergy_with(&candidate.name)
        {
            return true;
        }
    }

    false
}

/// Score a candidate's positive synergy with the current team. Higher is
/// better. Used as a tiebreaker in the greedy solver — candidates with
/// equal match quality but stronger synergies win.
///
/// A synergy is counted either if:
/// - The candidate lists a teammate by name, or
/// - The candidate lists a class that a teammate is evolved into, or
/// - A teammate lists the candidate by name (symmetric).
fn synergy_score(
    candidate: &MergedPet,
    team: &[&MergedPet],
    config: &PlannerConfig,
) -> u32 {
    let mut score = 0u32;

    if let Some(cand_info) = config.special_info(&candidate.name) {
        for synergy in cand_info.team_synergies() {
            for teammate in team {
                if let Some(pet_name) = &synergy.pet
                    && pet_name.eq_ignore_ascii_case(&teammate.name)
                {
                    score += 2; // name-level synergy is stronger
                }
                if let Some(class_name) = &synergy.class
                    && let Some(teammate_class) = teammate.evolved_class()
                    && format!("{teammate_class:?}").eq_ignore_ascii_case(class_name)
                {
                    score += 1;
                }
            }
        }
    }

    // Symmetric: teammates that like the candidate.
    for teammate in team {
        if let Some(tm_info) = config.special_info(&teammate.name) {
            for synergy in tm_info.team_synergies() {
                if let Some(pet_name) = &synergy.pet
                    && pet_name.eq_ignore_ascii_case(&candidate.name)
                {
                    score += 2;
                }
            }
        }
    }

    score
}

/// Scan a finalized team for dungeon-context anti-synergies and emit one
/// warning per conflict. The greedy assignment step already filters these
/// out, but Phase 1 / 1b respects user-forced assignments without rejecting
/// them — so two forced pets with an anti-synergy can legitimately end up
/// together, and this check surfaces that to the user instead of silently
/// letting it slide.
///
/// Conflicts are reported only once per unordered `{pet_a, pet_b}` pair,
/// using lexicographic ordering on the names so the warning is stable
/// across runs.
fn check_team_anti_synergies(
    team: &[&MergedPet],
    depth: u8,
    config: &PlannerConfig,
) -> Vec<CoverageWarning> {
    let mut warnings = Vec::new();

    for (i, pet_a) in team.iter().enumerate() {
        let Some(info_a) = config.special_info(&pet_a.name) else { continue };
        for pet_b in &team[i + 1..] {
            if info_a.has_dungeon_anti_synergy_with(&pet_b.name) {
                let (first, second) = if pet_a.name <= pet_b.name {
                    (pet_a.name.as_str(), pet_b.name.as_str())
                } else {
                    (pet_b.name.as_str(), pet_a.name.as_str())
                };
                warnings.push(CoverageWarning {
                    source_depth: depth,
                    kind: CoverageKind::Synergy,
                    name: format!("{first} + {second}"),
                    detail: format!(
                        "{first} and {second} have a dungeon anti-synergy and \
                         were both forced onto the team"
                    ),
                });
            }
        }
    }

    warnings
}

// =============================================================================
// Unlock suggestions
// =============================================================================

/// Generate unlock suggestions from the full merged pet list (including locked pets).
pub fn suggest_unlocks_for_slot(
    slot: &PartySlot,
    all_pets: &[MergedPet],
    config: Option<&PlannerConfig>,
) -> Vec<UnlockSuggestion> {
    let mut suggestions: Vec<UnlockSuggestion> = all_pets
        .iter()
        .filter(|pet| {
            let special = config.and_then(|c| c.special_info(&pet.name));
            // Locked-class filter: never suggest a Basilisk for a Defender slot.
            if let Some(locked) = special.and_then(PetSpecialInfo::locked_class)
                && let Some(required) = slot.class
                && required != locked
            {
                return false;
            }
            // Must not be unlocked
            !pet.is_unlocked()
            // Must not be a village pet
            && !pet.is_village_pet()
            // Must match element (accounting for multi_element / element wildcards)
            && matches_slot_element(pet, slot.element, special)
            // Must recommend the required class, or be a flexible-class pet
            && slot.class.as_ref().is_none_or(|c| {
                pet.recommends_class(c)
                    || special.is_some_and(PetSpecialInfo::is_flexible_class)
            })
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
    config: Option<&PlannerConfig>,
) -> Vec<CoverageWarning> {
    let mut warnings = Vec::new();

    for depth in 1..=target_depth {
        let Some(dd) = dungeon_data.depths.get(&depth) else {
            continue;
        };

        // Check traps
        for trap in &dd.traps {
            if !counter_satisfied(&trap.countered_by, team, config) {
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
                if !counter_satisfied(condition, team, config) {
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
///
/// Honors per-pet special info when `config` is `Some`:
/// - Pets with `class_wildcard` (Holy ITRTG Book, Nothing, Nugget) satisfy
///   any class requirement.
/// - Pets with `multi_element` (Tödlicher Löffel) satisfy any non-Neutral
///   element requirement.
/// - Element wildcards (Chameleon) already work via `pet.matches_element`.
fn counter_satisfied(
    condition: &CounterCondition,
    team: &[&MergedPet],
    config: Option<&PlannerConfig>,
) -> bool {
    // Item-only conditions are satisfied by bringing items, not by team composition
    if condition.item.is_some() && condition.class.is_none() && condition.element.is_none() {
        return true; // Items are handled separately
    }

    let required_count = condition.count.unwrap_or(1) as usize;

    let matching = team
        .iter()
        .filter(|pet| {
            let special = config.and_then(|c| c.special_info(&pet.name));

            let class_ok = condition.class.as_ref().is_none_or(|c| {
                pet.evolved_class().as_ref() == Some(c)
                    || special.is_some_and(PetSpecialInfo::is_class_wildcard)
            });
            let element_ok = condition.element.as_ref().is_none_or(|el| {
                pet.matches_element(Some(*el))
                    || special.is_some_and(|info| {
                        info.is_element_wildcard()
                            || (info.is_multi_element() && *el != Element::Neutral)
                    })
            });
            class_ok && element_ok
        })
        .count();

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
        let score = score_pet(&available[0], &slot, false, None);
        assert_eq!(score, Some(MatchQuality::Exact));
    }

    #[test]
    fn test_evolvable_match() {
        let pet = mock_pet("Rabbit", Element::Earth, None, RecommendedClass::Single(Class::Mage), true);
        let slot = make_slot(Some(Class::Mage), Some(Element::Earth));
        assert_eq!(score_pet(&pet, &slot, false, None), Some(MatchQuality::Evolvable));
    }

    #[test]
    fn test_element_mismatch_rejected() {
        let pet = mock_pet("Frog", Element::Water, Some(Class::Supporter), RecommendedClass::Single(Class::Supporter), true);
        let slot = make_slot(Some(Class::Supporter), Some(Element::Fire));
        assert_eq!(score_pet(&pet, &slot, false, None), None);
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
        assert_eq!(score_pet(&pet, &slot, false, None), Some(MatchQuality::Reclassable));
    }

    #[test]
    fn test_any_slot_accepts_dungeon_class() {
        let pet = mock_pet("Dog", Element::Neutral, Some(Class::Defender), RecommendedClass::Single(Class::Defender), true);
        let slot = make_slot(None, None);
        assert_eq!(score_pet(&pet, &slot, false, None), Some(MatchQuality::Exact));
    }

    #[test]
    fn test_non_dungeon_class_excluded_from_any_slot() {
        let pet = mock_pet("Mouse", Element::Earth, Some(Class::Adventurer), RecommendedClass::Wildcard, true);
        let slot = make_slot(None, None);
        assert_eq!(score_pet(&pet, &slot, false, None), None);
    }

    #[test]
    fn test_whitelisted_non_dungeon_class_allowed() {
        // Whitelisted pets bypass the non-dungeon class filter for "any" slots
        let pet = mock_pet("Bee", Element::Wind, Some(Class::Alchemist), RecommendedClass::Single(Class::Alchemist), true);
        let slot = make_slot(None, None);
        assert_eq!(score_pet(&pet, &slot, false, None), None); // Normally excluded
        assert_eq!(score_pet(&pet, &slot, true, None), Some(MatchQuality::Exact)); // Whitelisted: allowed
    }

    #[test]
    fn test_non_dungeon_class_allowed_when_required() {
        let pet = mock_pet("Smith", Element::Fire, Some(Class::Blacksmith), RecommendedClass::Single(Class::Blacksmith), true);
        let slot = make_slot(Some(Class::Blacksmith), Some(Element::Fire));
        assert_eq!(score_pet(&pet, &slot, false, None), Some(MatchQuality::Exact));
    }

    #[test]
    fn test_chameleon_matches_any_element() {
        let pet = mock_pet("Chameleon", Element::All, Some(Class::Mage), RecommendedClass::DungeonWildcard, true);
        let slot = make_slot(Some(Class::Mage), Some(Element::Fire));
        assert_eq!(score_pet(&pet, &slot, false, None), Some(MatchQuality::Exact));
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

        let plan = solve(Dungeon::Scrapyard, 1, &dungeon_data, &pets, None);

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

        let plans = solve_multi(&requests, &pets, &SolverConstraints::default(), None);
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

        let plan = solve(Dungeon::Scrapyard, 1, &dd, &pets, None);
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
        let plans = solve_multi(&requests, &pets, &constraints, None);

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
        let plans = solve_multi(&requests, &pets, &constraints, None);

        // Cat is forced, so Cat should be assigned even though Frog might be "better"
        assert!(matches!(&plans[0].assignments[0].assignment,
            Assignment::Filled { pet, .. } if pet.name == "Cat"));
    }

    // ========================================================================
    // Pet special info — class constraints, synergies, wildcards
    // ========================================================================

    use itrtg_models::planner_config::{
        PetSpecialInfo, PlannerConfig, PlannerConfigFile,
    };

    /// Build a planner config from the checked-in `planner_config.yaml` plus
    /// an arbitrary map of pet special info. Tests can focus on the
    /// special-info behavior without having to hand-roll equipment rules.
    fn config_with(info: std::collections::BTreeMap<String, PetSpecialInfo>) -> PlannerConfig {
        let rules_yaml = include_str!("../../../data/planner_config.yaml");
        let file: PlannerConfigFile = serde_yaml::from_str(rules_yaml).unwrap();
        PlannerConfig::new(file, info)
    }

    fn special_info_from_yaml(yaml: &str) -> PetSpecialInfo {
        serde_yaml::from_str(yaml).unwrap()
    }

    // -- Locked class --------------------------------------------------------

    #[test]
    fn test_locked_class_rejects_other_class_slots() {
        // Basilisk is always Mage — it must not fill a Defender slot.
        let basilisk = mock_pet(
            "Basilisk",
            Element::Fire,
            Some(Class::Mage),
            RecommendedClass::Single(Class::Mage),
            true,
        );

        let mut info = std::collections::BTreeMap::new();
        info.insert(
            "Basilisk".to_string(),
            special_info_from_yaml(
                "class_constraints:\n  - locked_class: Mage\n",
            ),
        );
        let cfg = config_with(info);

        let mage_slot = make_slot(Some(Class::Mage), Some(Element::Fire));
        let defender_slot = make_slot(Some(Class::Defender), Some(Element::Fire));

        assert_eq!(
            score_pet(&basilisk, &mage_slot, false, Some(&cfg)),
            Some(MatchQuality::Exact)
        );
        assert_eq!(
            score_pet(&basilisk, &defender_slot, false, Some(&cfg)),
            None,
            "Basilisk should not be considered for a Defender slot"
        );
    }

    #[test]
    fn test_avoid_class_downgrades_match_quality() {
        // Ape should avoid being a Mage. If the slot demands Mage and the
        // pet recommends Mage, the match quality degrades from Evolvable
        // to Reclassable — so another Mage candidate would beat it, but
        // the solver can still fall back on Ape if nothing else fits.
        let ape = mock_pet(
            "Ape",
            Element::Fire,
            None,
            RecommendedClass::Single(Class::Mage),
            true,
        );

        let mut info = std::collections::BTreeMap::new();
        info.insert(
            "Ape".to_string(),
            special_info_from_yaml(
                "class_constraints:\n  - avoid_class: Mage\n",
            ),
        );
        let cfg = config_with(info);

        let slot = make_slot(Some(Class::Mage), Some(Element::Fire));
        assert_eq!(
            score_pet(&ape, &slot, false, Some(&cfg)),
            Some(MatchQuality::Reclassable)
        );
    }

    #[test]
    fn test_flexible_class_matches_any_slot() {
        // Holy ITRTG Book / Nothing / Nugget can freely switch classes —
        // they match an Assassin slot as Exact even if the pet is evolved
        // as something else.
        let nothing = mock_pet(
            "Nothing (Other)",
            Element::Neutral,
            Some(Class::Adventurer),
            RecommendedClass::AllClasses,
            true,
        );

        let mut info = std::collections::BTreeMap::new();
        info.insert(
            "Nothing (Other)".to_string(),
            special_info_from_yaml(
                "class_constraints:\n  - flexible_class: true\n",
            ),
        );
        let cfg = config_with(info);

        let slot = make_slot(Some(Class::Assassin), Some(Element::Neutral));
        assert_eq!(
            score_pet(&nothing, &slot, false, Some(&cfg)),
            Some(MatchQuality::Exact)
        );
    }

    // -- Anti-synergies ------------------------------------------------------

    #[test]
    fn test_anti_synergy_prevents_co_assignment() {
        // Hourglass + Undine must not end up on the same team. Both are
        // water pets here so they'd both be viable for a water Supporter
        // slot on equal quality. The solver should pick the one that
        // sorts first lexicographically for stability (Hourglass) in one
        // slot, then reject Undine from a second slot.
        let pets = vec![
            mock_pet(
                "Hourglass",
                Element::Water,
                Some(Class::Supporter),
                RecommendedClass::Single(Class::Supporter),
                true,
            ),
            mock_pet(
                "Undine",
                Element::Water,
                Some(Class::Supporter),
                RecommendedClass::Single(Class::Supporter),
                true,
            ),
            mock_pet(
                "Frog",
                Element::Water,
                Some(Class::Supporter),
                RecommendedClass::Single(Class::Supporter),
                true,
            ),
        ];

        let mut info = std::collections::BTreeMap::new();
        info.insert(
            "Hourglass".to_string(),
            special_info_from_yaml(
                "team_anti_synergies:\n  - pet: Undine\n    context: all\n",
            ),
        );
        let cfg = config_with(info);

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
                    party: vec![
                        make_slot(Some(Class::Supporter), Some(Element::Water)),
                        make_slot(Some(Class::Supporter), Some(Element::Water)),
                    ],
                    party_items: Vec::new(), traps: Vec::new(), events: Vec::new(),
                });
                m
            },
        };

        // Force Hourglass into the team so we know it will be placed.
        let mut constraints = SolverConstraints::default();
        constraints.forced.insert(Dungeon::WaterTemple, vec!["Hourglass".to_string()]);

        let requests = [DungeonRequest {
            dungeon: Dungeon::WaterTemple, depth: 1, data: &dd,
        }];
        let plans = solve_multi(&requests, &pets, &constraints, Some(&cfg));

        // Assignment 0: Hourglass (forced).
        assert!(matches!(&plans[0].assignments[0].assignment,
            Assignment::Filled { pet, .. } if pet.name == "Hourglass"));
        // Assignment 1: Frog should have been picked — Undine was filtered
        // out by the anti-synergy check.
        assert!(matches!(&plans[0].assignments[1].assignment,
            Assignment::Filled { pet, .. } if pet.name == "Frog"),
            "Undine should have been filtered out by anti-synergy, leaving Frog");
    }

    #[test]
    fn test_forced_anti_synergy_emits_warning_but_keeps_assignment() {
        // User explicitly forces both Hourglass AND Undine into the same
        // dungeon. The greedy filter would normally reject Undine, but
        // the forced phase respects user intent without rejecting the
        // choice. We should surface the conflict as a Synergy warning
        // rather than silently letting it pass.
        let pets = vec![
            mock_pet(
                "Hourglass",
                Element::Water,
                Some(Class::Supporter),
                RecommendedClass::Single(Class::Supporter),
                true,
            ),
            mock_pet(
                "Undine",
                Element::Water,
                Some(Class::Supporter),
                RecommendedClass::Single(Class::Supporter),
                true,
            ),
        ];

        let mut info = std::collections::BTreeMap::new();
        info.insert(
            "Hourglass".to_string(),
            special_info_from_yaml(
                "team_anti_synergies:\n  - pet: Undine\n    context: all\n",
            ),
        );
        let cfg = config_with(info);

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
                    party: vec![
                        make_slot(Some(Class::Supporter), Some(Element::Water)),
                        make_slot(Some(Class::Supporter), Some(Element::Water)),
                    ],
                    party_items: Vec::new(), traps: Vec::new(), events: Vec::new(),
                });
                m
            },
        };

        let mut constraints = SolverConstraints::default();
        constraints.forced.insert(
            Dungeon::WaterTemple,
            vec!["Hourglass".to_string(), "Undine".to_string()],
        );

        let requests = [DungeonRequest {
            dungeon: Dungeon::WaterTemple, depth: 1, data: &dd,
        }];
        let plans = solve_multi(&requests, &pets, &constraints, Some(&cfg));

        // Both forced pets are on the team — user intent is respected.
        let names: Vec<&str> = plans[0]
            .assignments
            .iter()
            .filter_map(|a| match &a.assignment {
                Assignment::Filled { pet, .. } => Some(pet.name.as_str()),
                Assignment::Empty { .. } => None,
            })
            .collect();
        assert!(names.contains(&"Hourglass"));
        assert!(names.contains(&"Undine"));

        // …but a Synergy warning should surface the conflict.
        let synergy_warnings: Vec<_> = plans[0]
            .warnings
            .iter()
            .filter(|w| matches!(w.kind, CoverageKind::Synergy))
            .collect();
        assert_eq!(
            synergy_warnings.len(),
            1,
            "expected exactly one synergy warning, got {:?}",
            plans[0].warnings,
        );
        // Names are reported in lexicographic order for stability.
        assert_eq!(synergy_warnings[0].name, "Hourglass + Undine");
        assert!(synergy_warnings[0].detail.contains("anti-synergy"));
    }

    // -- Positive synergy tiebreaker -----------------------------------------

    #[test]
    fn test_positive_synergy_breaks_ties() {
        // Two equally-ranked Mage candidates, one with a positive synergy
        // to an already-placed Student. The synergy candidate should win.
        let pets = vec![
            mock_pet_with_evo(
                "Student",
                Element::Earth,
                Some(Class::Mage),
                RecommendedClass::Single(Class::Mage),
                true, 1, 1, 10000,
            ),
            mock_pet_with_evo(
                "Basilisk",
                Element::Fire,
                Some(Class::Mage),
                RecommendedClass::Single(Class::Mage),
                true, 1, 1, 10000,
            ),
            mock_pet_with_evo(
                "RandoMage",
                Element::Fire,
                Some(Class::Mage),
                RecommendedClass::Single(Class::Mage),
                true, 1, 1, 10000,
            ),
        ];

        let mut info = std::collections::BTreeMap::new();
        info.insert(
            "Basilisk".to_string(),
            special_info_from_yaml(
                "team_synergies:\n  - pet: Student\n",
            ),
        );
        let cfg = config_with(info);

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
                    party: vec![
                        make_slot(Some(Class::Mage), Some(Element::Earth)),
                        make_slot(Some(Class::Mage), Some(Element::Fire)),
                    ],
                    party_items: Vec::new(), traps: Vec::new(), events: Vec::new(),
                });
                m
            },
        };

        let requests = [DungeonRequest {
            dungeon: Dungeon::Volcano, depth: 1, data: &dd,
        }];
        let plans = solve_multi(
            &requests, &pets, &SolverConstraints::default(), Some(&cfg),
        );

        // Slot 0 (Earth Mage) goes to Student — the only earth mage.
        assert!(matches!(&plans[0].assignments[0].assignment,
            Assignment::Filled { pet, .. } if pet.name == "Student"));
        // Slot 1 (Fire Mage) has two equally-qualified candidates; the
        // synergy tiebreaker should favor Basilisk.
        assert!(matches!(&plans[0].assignments[1].assignment,
            Assignment::Filled { pet, .. } if pet.name == "Basilisk"),
            "Basilisk should beat RandoMage on the positive synergy tiebreaker");
    }

    // -- Coverage: class wildcard --------------------------------------------

    #[test]
    fn test_class_wildcard_satisfies_event_class_requirement() {
        // Holy ITRTG Book (class_wildcard) counts as any class for event
        // coverage. A D1 event demanding a Rogue should be satisfied by a
        // Holy Book even when nobody else on the team is a Rogue.
        let book = mock_pet(
            "Holy ITRTG Book",
            Element::Neutral,
            Some(Class::Defender),
            RecommendedClass::AllClasses,
            true,
        );

        let mut info = std::collections::BTreeMap::new();
        info.insert(
            "Holy ITRTG Book".to_string(),
            special_info_from_yaml(
                "class_constraints:\n  - class_wildcard: true\n",
            ),
        );
        let cfg = config_with(info);

        let condition = CounterCondition {
            item: None,
            class: Some(Class::Rogue),
            element: None,
            count: None,
            quantity_per_clear: None,
            notes: None,
        };
        let team = vec![&book];

        assert!(
            counter_satisfied(&condition, &team, Some(&cfg)),
            "class_wildcard pet should satisfy any class event requirement"
        );
        assert!(
            !counter_satisfied(&condition, &team, None),
            "without config the event should remain uncovered"
        );
    }

    // -- Coverage: multi-element ---------------------------------------------

    #[test]
    fn test_multi_element_satisfies_any_non_neutral_event() {
        // Tödlicher Löffel counts as every non-neutral element for events.
        // A team of one Löffel should satisfy a 1x Fire event, 1x Water
        // event, etc. — but *not* a Neutral one.
        let loffel = mock_pet(
            "Tödlicher Löffel",
            Element::Neutral,
            Some(Class::Mage),
            RecommendedClass::Single(Class::Mage),
            true,
        );

        let mut info = std::collections::BTreeMap::new();
        info.insert(
            "Tödlicher Löffel".to_string(),
            special_info_from_yaml("element_constraints:\n  multi_element: true\n"),
        );
        let cfg = config_with(info);

        let team = vec![&loffel];

        for el in [Element::Fire, Element::Water, Element::Wind, Element::Earth] {
            let cond = CounterCondition {
                item: None, class: None, element: Some(el),
                count: None, quantity_per_clear: None, notes: None,
            };
            assert!(
                counter_satisfied(&cond, &team, Some(&cfg)),
                "multi-element pet should satisfy {el:?} event"
            );
        }

        // Neutral is NOT covered — multi_element only claims non-neutral.
        let neutral_cond = CounterCondition {
            item: None, class: None, element: Some(Element::Neutral),
            count: None, quantity_per_clear: None, notes: None,
        };
        // Löffel's export element is Neutral, so it does match a Neutral
        // condition via the normal path. This assertion is a cross-check
        // that the multi_element branch doesn't accidentally *exclude*
        // neutral-matching pets.
        assert!(counter_satisfied(&neutral_cond, &team, Some(&cfg)));
    }

    // -- Element matching with multi_element in slot filling -----------------

    #[test]
    fn test_multi_element_pet_fills_any_element_slot() {
        // A multi-element pet should be eligible for any non-neutral
        // element slot (Wind in this test).
        let loffel = mock_pet(
            "Tödlicher Löffel",
            Element::Neutral,
            Some(Class::Mage),
            RecommendedClass::Single(Class::Mage),
            true,
        );

        let mut info = std::collections::BTreeMap::new();
        info.insert(
            "Tödlicher Löffel".to_string(),
            special_info_from_yaml("element_constraints:\n  multi_element: true\n"),
        );
        let cfg = config_with(info);

        let slot = make_slot(Some(Class::Mage), Some(Element::Wind));
        assert_eq!(
            score_pet(&loffel, &slot, false, Some(&cfg)),
            Some(MatchQuality::Exact)
        );
        // Without config, the pet's Neutral export element makes it fail
        // the wind requirement.
        assert_eq!(score_pet(&loffel, &slot, false, None), None);
    }
}
