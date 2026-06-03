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
    /// Equipment pulled from a neighboring depth via a look-around hint, used
    /// to fill a slot whose own recommendation is absent or generic. Consumed
    /// by `equipment::enrich_equipment`, which turns it into a `Propagated`
    /// suggestion.
    pub equipment_hint: Option<PropagatedEquipment>,
}

/// Equipment borrowed from another depth's matching slot to fill a slot whose
/// own recommendation is absent or generic. Carried on the assignment so the
/// equipment enrichment pass can surface it (tagged with its origin depth).
#[derive(Debug, Clone)]
pub struct PropagatedEquipment {
    pub equipment: PartyEquipment,
    pub from_depth: u8,
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
    /// A dungeon-viable but *deprioritized* class (Blacksmith) filling an
    /// "any" class slot. Ranked below every proper dungeon-class candidate so
    /// a Blacksmith only lands in an "any" slot when nothing better is left —
    /// Blacksmiths do earn dungeon class XP, but you rarely want to grow one
    /// in a dungeon unless the recommendation specifically calls for it.
    LowPriority,
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
    /// Explicit enable/disable choices for dungeon events, keyed by
    /// (dungeon, depth, event name). An absent entry means "use the event's
    /// default": optional events are disabled, normal events enabled. A
    /// disabled event is skipped in coverage checks (no warning).
    pub event_overrides: HashMap<(Dungeon, u8, String), bool>,
}

// =============================================================================
// Dungeon class viability
// =============================================================================

/// How a pet's class fares when filling an "any" class slot in a dungeon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DungeonViability {
    /// A normal dungeon class — full priority.
    Normal,
    /// Blacksmith. Allowed (Blacksmiths *do* earn dungeon class XP), but
    /// ranked below every proper dungeon-class candidate so it's only used as
    /// a last resort or when the recommendation explicitly wants one.
    LowPriority,
    /// Adventurer / Alchemist — these never earn dungeon class XP, so they're
    /// excluded from "any" slots entirely (unless whitelisted).
    Excluded,
}

/// Classify a concrete class for "any" dungeon slot purposes.
fn classify_class(class: Class) -> DungeonViability {
    match class {
        Class::Blacksmith => DungeonViability::LowPriority,
        Class::Adventurer | Class::Alchemist => DungeonViability::Excluded,
        _ => DungeonViability::Normal,
    }
}

/// The more favorable of two viabilities (Normal > LowPriority > Excluded),
/// used to combine the two halves of a `Dual` recommendation.
fn better_viability(a: DungeonViability, b: DungeonViability) -> DungeonViability {
    let rank = |v: DungeonViability| match v {
        DungeonViability::Normal => 0u8,
        DungeonViability::LowPriority => 1,
        DungeonViability::Excluded => 2,
    };
    if rank(a) <= rank(b) { a } else { b }
}

/// Determine a pet's viability for dungeon "any" class slots.
///
/// Evolved class is ground truth; an unevolved pet is judged by its
/// recommended class. When a slot *specifically* requires a class (e.g.
/// Volcano's "Burning Weapons" event wanting a Blacksmith), this check is
/// bypassed by the caller.
fn dungeon_viability(pet: &MergedPet) -> DungeonViability {
    if let Some(class) = pet.evolved_class() {
        return classify_class(class);
    }
    // Unevolved: check recommended class
    match pet.recommended_class() {
        Some(RecommendedClass::Single(c)) => classify_class(*c),
        Some(RecommendedClass::Dual(a, b)) => {
            better_viability(classify_class(*a), classify_class(*b))
        }
        Some(
            RecommendedClass::Wildcard
            | RecommendedClass::DungeonWildcard
            | RecommendedClass::AllClasses,
        ) => DungeonViability::Normal,
        Some(
            RecommendedClass::Village(_)
            | RecommendedClass::Special
            | RecommendedClass::Alternates,
        ) => DungeonViability::Excluded,
        None => DungeonViability::Normal, // No wiki data — assume viable
    }
}

/// Whether a pet is a full-priority candidate for dungeon "any" class slots.
/// Used for unlock suggestions, which should never propose a deprioritized
/// (Blacksmith) or excluded (Adventurer/Alchemist) class for an "any" slot.
fn is_dungeon_viable(pet: &MergedPet) -> bool {
    dungeon_viability(pet) == DungeonViability::Normal
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
                    equipment_hint: None,
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
                    equipment_hint: None,
                },
            );
        }
    }

    // Look-around: derive per-slot hints from each dungeon's other depths
    // (deeper first, then shallower) so the team we grow now stays compatible
    // with what we'll face deeper and what we re-clear on the way there.
    // Indexed by request, aligned with that depth's party slots.
    let slot_hints: Vec<Vec<SlotHint>> = requests
        .iter()
        .map(|req| compute_slot_hints(req.data, req.depth))
        .collect();

    // Phase 2: Flatten remaining slots across all dungeons with their candidates
    struct GlobalSlot<'a> {
        req_idx: usize,
        slot_idx: usize,
        slot: &'a PartySlot,
        hint: SlotHint,
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
                let hint = slot_hints
                    .get(ri)
                    .and_then(|hs| hs.get(si))
                    .cloned()
                    .unwrap_or_default();
                all_slots.push(GlobalSlot {
                    req_idx: ri,
                    slot_idx: si,
                    slot,
                    hint,
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
                    // Look-around tiebreaker: among equally-good candidates,
                    // prefer the one that also fits this slot's hint derived
                    // from neighboring depths, so the team carries forward (and
                    // back). Purely a tiebreaker — it never changes which pets
                    // are eligible.
                    if !gs.hint.is_empty() {
                        let fa = forward_hint_score(available[*pi_a], &gs.hint);
                        let fb = forward_hint_score(available[*pi_b], &gs.hint);
                        let fwd_cmp = fb.cmp(&fa);
                        if fwd_cmp != std::cmp::Ordering::Equal {
                            return fwd_cmp;
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
                equipment_hint: None,
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

            let mut assignments: Vec<SlotAssignment> = (0..dd.party.len())
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
                            equipment_hint: None,
                        })
                })
                .collect();

            // Propagate neighboring-depth equipment into slots whose own
            // recommendation is absent or generic (look-around equipment).
            for (si, sa) in assignments.iter_mut().enumerate() {
                if let Some(hint) = slot_hints.get(ri).and_then(|h| h.get(si))
                    && let Some(eq) = &hint.equipment
                    && sa
                        .slot
                        .equipment
                        .as_ref()
                        .is_none_or(crate::equipment::has_generic_keys)
                {
                    sa.equipment_hint = Some(PropagatedEquipment {
                        equipment: eq.clone(),
                        from_depth: hint.equipment_from_depth.unwrap_or(req.depth),
                    });
                }
            }

            let assigned_pets: Vec<&MergedPet> = assignments
                .iter()
                .filter_map(|a| match &a.assignment {
                    Assignment::Filled { pet, .. } => Some(pet),
                    Assignment::Empty { .. } => None,
                })
                .collect();

            let mut warnings = check_coverage(
                req.dungeon,
                req.data,
                req.depth,
                &assigned_pets,
                config,
                &constraints.event_overrides,
            );
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
        // "Any" class slot.
        let base = if pet.is_evolved() {
            MatchQuality::Exact
        } else {
            MatchQuality::Evolvable
        };
        // Whitelisted pets bypass the viability filter entirely.
        if whitelisted {
            return Some(base);
        }
        return match dungeon_viability(pet) {
            DungeonViability::Normal => Some(base),
            // Blacksmiths are allowed but always sink to the bottom.
            DungeonViability::LowPriority => Some(MatchQuality::LowPriority),
            DungeonViability::Excluded => None,
        };
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
// Look-around hints (look-ahead + look-behind)
// =============================================================================

/// A soft preference for a slot, derived by looking at the *other* depths of
/// the same dungeon. Only ever fills fields the planned depth leaves as
/// wildcards (`any`), so it can refine but never contradict the planned
/// depth's explicit requirements (those are ground truth).
#[derive(Debug, Clone, Default)]
struct SlotHint {
    class: Option<Class>,
    element: Option<Element>,
    /// Real (non-generic) equipment from the matched neighboring slot, to be
    /// propagated into this slot if its own recommendation is absent/generic.
    equipment: Option<PartyEquipment>,
    /// Which depth `equipment` was borrowed from (for provenance display).
    equipment_from_depth: Option<u8>,
}

impl SlotHint {
    /// Whether this hint carries a class/element preference. Equipment-only
    /// hints count as empty here because equipment doesn't drive the pet
    /// selection tiebreaker — it's applied separately after solving.
    fn is_empty(&self) -> bool {
        self.class.is_none() && self.element.is_none()
    }
}

/// Compute per-slot look-around hints for a dungeon at `depth`.
///
/// The planned depth's *explicit* class/element fields are ground truth and
/// are never overwritten — hints only ever fill the fields it leaves as `any`.
///
/// Dungeon progression is cumulative: a D2 team first clears every D1 room, a
/// D3 team clears D1+D2 first, and so on. So the team grown for the depth
/// being planned is both the team brought *deeper* and the team walked back
/// *through the shallower rooms* on the way to a deeper boss. We therefore
/// pull hints from every other depth, in priority order:
///
/// - **Look-ahead** (deeper depths, nearest first) is the primary source: a
///   deeper recommendation is — with rare exceptions — a strict refinement of
///   this one, so we steer the team toward what it will eventually need.
/// - **Look-behind** (shallower depths, nearest first) is the fallback: when
///   the planned depth leaves something open that a shallower depth pinned
///   down (e.g. D4 Scrapyard drops the elements that D3 needed to satisfy its
///   2-Wind Floating Shrine event), we still try to satisfy it on the way
///   back through those rooms.
///
/// Passes run nearest-first, ahead before behind, and each pass only fills
/// fields still open after the higher-priority passes — so a nearer depth
/// always wins a conflict with a farther one. When no other depth exists every
/// hint is empty and the solver behaves exactly as before.
fn compute_slot_hints(data: &DungeonData, depth: u8) -> Vec<SlotHint> {
    let Some(current) = data.depths.get(&depth) else {
        return Vec::new();
    };
    let current = &current.party;

    // Source depths in priority order: deeper nearest-first, then shallower
    // nearest-first. `BTreeMap::keys` is ascending, so deeper depths are
    // already nearest-first; shallower depths need reversing.
    let mut sources: Vec<u8> = data.depths.keys().copied().filter(|&d| d > depth).collect();
    let mut behind: Vec<u8> = data.depths.keys().copied().filter(|&d| d < depth).collect();
    behind.reverse();
    sources.extend(behind);

    let mut hints = vec![SlotHint::default(); current.len()];
    for src in sources {
        let pass = match_against(current, &data.depths[&src].party);
        for (acc, candidate) in hints.iter_mut().zip(pass) {
            // Only fill fields not already pinned by a higher-priority pass.
            if acc.class.is_none() {
                acc.class = candidate.class;
            }
            if acc.element.is_none() {
                acc.element = candidate.element;
            }
            if acc.equipment.is_none()
                && let Some(eq) = candidate.equipment
            {
                acc.equipment = Some(eq);
                acc.equipment_from_depth = Some(src);
            }
        }
    }

    hints
}

/// One greedy matching pass: pair each slot in `current` with a compatible
/// slot in `target`, returning the fields `target` pins down that `current`
/// leaves as `any`.
///
/// Matching is a greedy bipartite assignment: the planned depth's most
/// specific slots claim their counterparts first, so concrete slots take their
/// exact matches before wildcards grab the leftovers. A `target` slot is
/// *compatible* with a `current` slot when it agrees on every field `current`
/// specifies (an `any` field matches anything).
fn match_against(current: &[PartySlot], target: &[PartySlot]) -> Vec<SlotHint> {
    let mut hints = vec![SlotHint::default(); current.len()];
    let mut target_used = vec![false; target.len()];

    // Specificity = how many of {class, element} a slot pins down. Process the
    // most specific current slots first so concrete slots claim their exact
    // counterparts before full wildcards consume them.
    let mut order: Vec<usize> = (0..current.len()).collect();
    order.sort_by_key(|&ci| {
        let s = current[ci].class.is_some() as u8 + current[ci].element.is_some() as u8;
        std::cmp::Reverse(s)
    });

    for ci in order {
        let cur = &current[ci];

        // Find the best unused, compatible target slot.
        let best = target
            .iter()
            .enumerate()
            .filter(|(tj, _)| !target_used[*tj])
            .filter(|(_, t)| slot_refines(cur, t))
            .max_by_key(|(tj, t)| {
                // Prefer the target that adds the most specificity to the
                // fields this slot leaves open, then an exact class match for
                // stability, then the lowest index (deterministic).
                let gain = (cur.class.is_none() && t.class.is_some()) as u8
                    + (cur.element.is_none() && t.element.is_some()) as u8;
                let exact_class = (cur.class.is_some() && cur.class == t.class) as u8;
                (gain, exact_class, std::cmp::Reverse(*tj))
            });

        if let Some((tj, t)) = best {
            target_used[tj] = true;
            if cur.class.is_none() {
                hints[ci].class = t.class;
            }
            if cur.element.is_none() {
                hints[ci].element = t.element;
            }
            // Capture the matched slot's equipment to propagate later, but
            // only if it's a real (non-generic) recommendation worth reusing.
            if t.equipment
                .as_ref()
                .is_some_and(|e| !crate::equipment::has_generic_keys(e))
            {
                hints[ci].equipment = t.equipment.clone();
            }
        }
    }

    hints
}

/// Whether `target` is a valid refinement of slot `cur` — i.e. it agrees on
/// every field `cur` explicitly specifies. Wildcard (`None`) fields on `cur`
/// match anything.
fn slot_refines(cur: &PartySlot, target: &PartySlot) -> bool {
    cur.class.is_none_or(|c| target.class == Some(c))
        && cur.element.is_none_or(|e| target.element == Some(e))
}

/// Score how well a pet satisfies a look-around hint. Higher is better; 0
/// means the hint is empty or the pet matches none of it. Used only as a
/// tiebreaker among equal-quality candidates, so it can bias the choice toward
/// a forward/backward-compatible pet without ever changing which pets are
/// eligible.
fn forward_hint_score(pet: &MergedPet, hint: &SlotHint) -> u32 {
    let mut score = 0;
    if let Some(el) = hint.element
        && pet.matches_element(Some(el))
    {
        score += 1;
    }
    if let Some(cl) = hint.class {
        let class_ok = match pet.evolved_class() {
            Some(actual) => actual == cl,
            None => pet.recommends_class(&cl),
        };
        if class_ok {
            score += 1;
        }
    }
    score
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

/// Whether a dungeon event is enabled for coverage checks, given the user's
/// overrides. Absent override → default: optional events are disabled (they
/// carry no in-game penalty), normal events are enabled.
fn event_enabled(
    dungeon: Dungeon,
    depth: u8,
    event: &EventEntry,
    overrides: &HashMap<(Dungeon, u8, String), bool>,
) -> bool {
    overrides
        .get(&(dungeon, depth, event.name.clone()))
        .copied()
        .unwrap_or(!event.optional)
}

/// Check event/trap coverage across all sub-depths up to the target.
///
/// When running D2, you first clear all D1 rooms. When running D3, you clear
/// D1 and D2 rooms first. So events/traps from lower depths can still occur.
///
/// Events the user has disabled (or optional events left at their disabled
/// default) are skipped entirely — no warning is produced for them.
fn check_coverage(
    dungeon: Dungeon,
    dungeon_data: &DungeonData,
    target_depth: u8,
    team: &[&MergedPet],
    config: Option<&PlannerConfig>,
    event_overrides: &HashMap<(Dungeon, u8, String), bool>,
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

        // Check events (skipping any the user has disabled).
        for event in &dd.events {
            if !event_enabled(dungeon, depth, event, event_overrides) {
                continue;
            }
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

    // ========================================================================
    // Blacksmith: allowed in "any" slots but lowest priority
    // ========================================================================

    #[test]
    fn test_blacksmith_low_priority_in_any_slot() {
        // A Blacksmith is eligible for an "any" class slot, but only at
        // LowPriority — below every proper dungeon-class candidate.
        let smith = mock_pet(
            "Smith",
            Element::Fire,
            Some(Class::Blacksmith),
            RecommendedClass::Single(Class::Blacksmith),
            true,
        );
        let slot = make_slot(None, None);
        assert_eq!(
            score_pet(&smith, &slot, false, None),
            Some(MatchQuality::LowPriority),
        );
    }

    #[test]
    fn test_alchemist_still_excluded_from_any_slot() {
        // Alchemists never earn dungeon class XP — still excluded.
        let bee = mock_pet(
            "Bee",
            Element::Wind,
            Some(Class::Alchemist),
            RecommendedClass::Single(Class::Alchemist),
            true,
        );
        let slot = make_slot(None, None);
        assert_eq!(score_pet(&bee, &slot, false, None), None);
    }

    #[test]
    fn test_blacksmith_still_matches_required_blacksmith_slot() {
        // When a slot *requires* a Blacksmith (e.g. Volcano's Burning Weapons
        // event), the viability deprioritization is bypassed entirely.
        let smith = mock_pet(
            "Smith",
            Element::Fire,
            Some(Class::Blacksmith),
            RecommendedClass::Single(Class::Blacksmith),
            true,
        );
        let slot = make_slot(Some(Class::Blacksmith), Some(Element::Fire));
        assert_eq!(
            score_pet(&smith, &slot, false, None),
            Some(MatchQuality::Exact),
        );
    }

    #[test]
    fn test_dungeon_class_beats_blacksmith_for_any_slot() {
        // Given one "any" slot and both a Defender and a Blacksmith, the
        // Defender wins because LowPriority sorts below Exact.
        let pets = vec![
            mock_pet(
                "Smith",
                Element::Fire,
                Some(Class::Blacksmith),
                RecommendedClass::Single(Class::Blacksmith),
                true,
            ),
            mock_pet(
                "Guard",
                Element::Fire,
                Some(Class::Defender),
                RecommendedClass::Single(Class::Defender),
                true,
            ),
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
                    party: vec![make_slot(None, None)],
                    party_items: Vec::new(), traps: Vec::new(), events: Vec::new(),
                });
                m
            },
        };

        let plan = solve(Dungeon::Scrapyard, 1, &dd, &pets, None);
        assert!(matches!(&plan.assignments[0].assignment,
            Assignment::Filled { pet, .. } if pet.name == "Guard"),
            "Defender should beat Blacksmith for an 'any' slot");
    }

    // ========================================================================
    // Look-around: bias toward neighboring-depth recommendations
    // ========================================================================

    /// Build a dungeon from a list of per-depth party compositions (index 0 →
    /// depth 1, index 1 → depth 2, …). Requirements/monsters are filler — only
    /// the party shape matters for look-around tests.
    fn dungeon_with_depths(parties: Vec<Vec<PartySlot>>) -> DungeonData {
        let filler = |party: Vec<PartySlot>| DepthData {
            rooms: 5, monsters_per_room: 2, gem_level: None,
            requirements: DepthRequirements {
                dungeon_level_avg: 5, levels_per_difficulty: vec![1, 2],
                class_level: 3, total_growth: None,
            },
            monsters: Vec::new(),
            bosses: vec![MonsterEntry { name: "B".into(), element: None, hp: 50, att: 20, def: 10, spd: 10 }],
            party,
            party_items: Vec::new(), traps: Vec::new(), events: Vec::new(),
        };
        let mut depths = std::collections::BTreeMap::new();
        for (i, party) in parties.into_iter().enumerate() {
            depths.insert((i + 1) as u8, filler(party));
        }
        DungeonData { name: "Test".to_string(), depths }
    }

    /// Convenience wrapper for the common two-depth case.
    fn two_depth_dungeon(d1: Vec<PartySlot>, d2: Vec<PartySlot>) -> DungeonData {
        dungeon_with_depths(vec![d1, d2])
    }

    /// An empty equipment catalog — enough for `enrich_equipment` to run the
    /// static/propagated tagging paths (which don't touch the catalog).
    fn test_catalog() -> EquipmentCatalog {
        EquipmentCatalog {
            weapons: std::collections::BTreeMap::new(),
            armor: std::collections::BTreeMap::new(),
            accessories: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn test_forward_hint_refines_wildcard_element() {
        // D1 wants a Defender of any element; D2 wants a Fire Defender. When
        // planning D1, the Fire defender should be preferred over a
        // higher-growth Water defender purely on the look-ahead tiebreaker.
        let dd = two_depth_dungeon(
            vec![make_slot(Some(Class::Defender), None)],
            vec![make_slot(Some(Class::Defender), Some(Element::Fire))],
        );

        let pets = vec![
            // Water defender with much higher growth — would win without the hint.
            mock_pet_with_evo("WaterGuard", Element::Water, Some(Class::Defender),
                RecommendedClass::Single(Class::Defender), true, 1, 1, 99999),
            mock_pet_with_evo("FireGuard", Element::Fire, Some(Class::Defender),
                RecommendedClass::Single(Class::Defender), true, 1, 1, 10000),
        ];

        let hints = compute_slot_hints(&dd, 1);
        assert_eq!(hints[0].element, Some(Element::Fire));

        let plan = solve(Dungeon::Volcano, 1, &dd, &pets, None);
        assert!(matches!(&plan.assignments[0].assignment,
            Assignment::Filled { pet, .. } if pet.name == "FireGuard"),
            "look-ahead should prefer the Fire defender for the eventual D2 Fire slot");
    }

    #[test]
    fn test_forward_hint_fills_full_wildcard_class() {
        // D1: [Defender/any, any/any]. D2: [Defender/any, Assassin/Wind].
        // The leftover wildcard should lean toward a Wind Assassin.
        let dd = two_depth_dungeon(
            vec![
                make_slot(Some(Class::Defender), None),
                make_slot(None, None),
            ],
            vec![
                make_slot(Some(Class::Defender), None),
                make_slot(Some(Class::Assassin), Some(Element::Wind)),
            ],
        );

        let hints = compute_slot_hints(&dd, 1);
        // Slot 0 (Defender) matched the D2 Defender → no refinement.
        assert!(hints[0].is_empty());
        // Slot 1 (wildcard) inherited Assassin + Wind from the leftover D2 slot.
        assert_eq!(hints[1].class, Some(Class::Assassin));
        assert_eq!(hints[1].element, Some(Element::Wind));

        let pets = vec![
            mock_pet_with_evo("Guard", Element::Earth, Some(Class::Defender),
                RecommendedClass::Single(Class::Defender), true, 1, 1, 10000),
            // High-growth mage — would win the wildcard without the hint.
            mock_pet_with_evo("Sage", Element::Water, Some(Class::Mage),
                RecommendedClass::Single(Class::Mage), true, 1, 1, 99999),
            mock_pet_with_evo("WindBlade", Element::Wind, Some(Class::Assassin),
                RecommendedClass::Single(Class::Assassin), true, 1, 1, 10000),
        ];

        let plan = solve(Dungeon::Scrapyard, 1, &dd, &pets, None);
        // Slot 0 → Guard (only defender). Slot 1 (wildcard) → WindBlade via hint.
        assert!(matches!(&plan.assignments[0].assignment,
            Assignment::Filled { pet, .. } if pet.name == "Guard"));
        assert!(matches!(&plan.assignments[1].assignment,
            Assignment::Filled { pet, .. } if pet.name == "WindBlade"),
            "wildcard slot should lean toward the forward-compatible Wind Assassin");
    }

    #[test]
    fn test_no_hint_for_single_depth_dungeon() {
        // A dungeon with only the depth being planned has no neighbors to pull
        // hints from — the solver behaves exactly as before.
        let dd = dungeon_with_depths(vec![vec![make_slot(Some(Class::Defender), None)]]);
        let hints = compute_slot_hints(&dd, 1);
        assert!(hints.iter().all(|h| h.is_empty()));
    }

    #[test]
    fn test_lookbehind_recovers_dropped_element() {
        // D4-style case: the deepest depth drops the elements that a shallower
        // depth needed (e.g. Scrapyard D3's 2-Wind Floating Shrine event).
        // Planning the deepest depth, look-behind should recover them.
        let dd = dungeon_with_depths(vec![
            // D1: specific Wind elements.
            vec![
                make_slot(Some(Class::Assassin), Some(Element::Wind)),
                make_slot(Some(Class::Rogue), Some(Element::Wind)),
            ],
            // D2 (planned, deepest): same classes but elements wildcard.
            vec![
                make_slot(Some(Class::Assassin), None),
                make_slot(Some(Class::Rogue), None),
            ],
        ]);

        let hints = compute_slot_hints(&dd, 2);
        assert_eq!(hints[0].element, Some(Element::Wind));
        assert_eq!(hints[1].element, Some(Element::Wind));
    }

    #[test]
    fn test_nearer_depth_wins_hint_conflict() {
        // Planning D1 with two deeper depths that disagree on the Defender's
        // element. The nearer depth (D2) should win the conflict.
        let dd = dungeon_with_depths(vec![
            vec![make_slot(Some(Class::Defender), None)],          // D1 (planned)
            vec![make_slot(Some(Class::Defender), Some(Element::Water))], // D2
            vec![make_slot(Some(Class::Defender), Some(Element::Fire))],  // D3
        ]);
        let hints = compute_slot_hints(&dd, 1);
        assert_eq!(
            hints[0].element,
            Some(Element::Water),
            "the nearer depth (D2) should win the element conflict over D3",
        );
    }

    #[test]
    fn test_farther_depth_fills_what_nearer_left_open() {
        // Volcano-style: D2 and D3 both leave the Defender's element open, but
        // D4 pins it to Fire. Planning D2, the Fire hint should come from D4
        // since the nearer D3 contributes nothing for that field.
        let dd = dungeon_with_depths(vec![
            vec![make_slot(Some(Class::Defender), None)],          // D1
            vec![make_slot(Some(Class::Defender), None)],          // D2 (planned)
            vec![make_slot(Some(Class::Defender), None)],          // D3
            vec![make_slot(Some(Class::Defender), Some(Element::Fire))], // D4
        ]);
        let hints = compute_slot_hints(&dd, 2);
        assert_eq!(hints[0].element, Some(Element::Fire));
    }

    #[test]
    fn test_hints_ignore_party_order_reshuffle() {
        // Mirrors Scrapyard D2 -> D3, where the Rogue moves from the front
        // row to the back and a wildcard becomes a second (Wind) Assassin.
        // Matching is by class/element, not position, so planning D2 should
        // still turn a wildcard into an Assassin and bias both assassins/rogue
        // toward Wind — regardless of which slot index things sit in.
        let d2 = vec![
            make_slot(Some(Class::Assassin), None),
            make_slot(Some(Class::Rogue), None),
            make_slot(Some(Class::Defender), None),
            make_slot(Some(Class::Supporter), None),
            make_slot(None, None),
            make_slot(None, None),
        ];
        let d3 = vec![
            make_slot(Some(Class::Assassin), None),
            make_slot(Some(Class::Assassin), Some(Element::Wind)),
            make_slot(Some(Class::Defender), None),
            make_slot(Some(Class::Supporter), None),
            make_slot(Some(Class::Rogue), Some(Element::Wind)),
            make_slot(None, None),
        ];
        let dd = dungeon_with_depths(vec![d2, d3]);
        let hints = compute_slot_hints(&dd, 1);

        // The existing Assassin slot leans Wind (from D3's Wind Assassin).
        assert_eq!(hints[0].element, Some(Element::Wind));
        // The Rogue slot leans Wind even though it sits in a different D3 slot.
        assert_eq!(hints[1].element, Some(Element::Wind));
        // Exactly one wildcard becomes an Assassin (the extra one D3 adds);
        // the other stays a pure wildcard.
        let wildcard_classes: Vec<Option<Class>> = vec![hints[4].class, hints[5].class];
        assert!(
            wildcard_classes.contains(&Some(Class::Assassin)),
            "a leftover wildcard should be hinted toward the extra Assassin",
        );
        assert_eq!(
            wildcard_classes.iter().filter(|c| c.is_some()).count(),
            1,
            "only the one extra Assassin should be added, not both wildcards",
        );
    }

    #[test]
    fn test_equipment_propagates_from_deeper_depth() {
        // A D1 wildcard slot with no equipment should inherit the exact gear
        // from the deeper slot that the hint matched it to.
        let deeper_equip = PartyEquipment {
            weapon: Some("inferno_sword".to_string()),
            armor: Some("titanium_armor".to_string()),
            accessory: Some("inferno_gloves".to_string()),
            gems: None,
        };
        let mut d2_slot = make_slot(Some(Class::Assassin), Some(Element::Fire));
        d2_slot.equipment = Some(deeper_equip.clone());
        let dd = dungeon_with_depths(vec![
            vec![make_slot(None, None)], // D1 wildcard, no equipment
            vec![d2_slot],               // D2 Fire Assassin with real gear
        ]);

        let hints = compute_slot_hints(&dd, 1);
        assert_eq!(hints[0].class, Some(Class::Assassin));
        assert_eq!(hints[0].equipment_from_depth, Some(2));

        let pets = vec![mock_pet("Blade", Element::Fire, Some(Class::Assassin),
            RecommendedClass::Single(Class::Assassin), true)];
        let plan = solve(Dungeon::Scrapyard, 1, &dd, &pets, None);

        // The slot carries the propagated equipment from D2.
        let hint = plan.assignments[0]
            .equipment_hint
            .as_ref()
            .expect("wildcard slot should inherit D2 equipment");
        assert_eq!(hint.from_depth, 2);
        assert_eq!(hint.equipment.weapon.as_deref(), Some("inferno_sword"));

        // And enrichment turns it into a Propagated suggestion.
        let mut plan = plan;
        let catalog = test_catalog();
        crate::equipment::enrich_equipment(&mut plan, &catalog, None);
        let suggestion = plan.assignments[0]
            .equipment_suggestion
            .as_ref()
            .expect("propagated equipment should yield a suggestion even without config");
        assert!(matches!(
            suggestion.source,
            crate::equipment::EquipmentSource::Propagated { from_depth: 2 }
        ));
    }

    #[test]
    fn test_equipment_not_propagated_over_real_static_gear() {
        // A slot that already has its own (non-generic) gear keeps it — the
        // deeper depth's gear is not pulled in over a real recommendation.
        let own = PartyEquipment {
            weapon: Some("steel_sword".to_string()),
            armor: Some("steel_armor".to_string()),
            accessory: Some("steel_ring".to_string()),
            gems: None,
        };
        let deeper = PartyEquipment {
            weapon: Some("titanium_sword".to_string()),
            armor: Some("titanium_armor".to_string()),
            accessory: Some("titanium_ring".to_string()),
            gems: None,
        };
        let mut d1_slot = make_slot(Some(Class::Defender), None);
        d1_slot.equipment = Some(own);
        let mut d2_slot = make_slot(Some(Class::Defender), Some(Element::Fire));
        d2_slot.equipment = Some(deeper);
        let dd = dungeon_with_depths(vec![vec![d1_slot], vec![d2_slot]]);

        let pets = vec![mock_pet("Guard", Element::Fire, Some(Class::Defender),
            RecommendedClass::Single(Class::Defender), true)];
        let plan = solve(Dungeon::Scrapyard, 1, &dd, &pets, None);

        assert!(
            plan.assignments[0].equipment_hint.is_none(),
            "a slot with its own real gear must not be overwritten by propagation",
        );
    }

    #[test]
    fn test_lookahead_beats_lookbehind_on_conflict() {
        // When a deeper and a shallower depth disagree on an open field, the
        // deeper (look-ahead) value wins — we prioritize where the team is
        // headed over where it's been.
        let dd = dungeon_with_depths(vec![
            vec![make_slot(Some(Class::Defender), Some(Element::Earth))], // D1 (behind)
            vec![make_slot(Some(Class::Defender), None)],                 // D2 (planned)
            vec![make_slot(Some(Class::Defender), Some(Element::Fire))],  // D3 (ahead)
        ]);
        let hints = compute_slot_hints(&dd, 2);
        assert_eq!(
            hints[0].element,
            Some(Element::Fire),
            "look-ahead (D3 Fire) should win over look-behind (D1 Earth)",
        );
    }

    #[test]
    fn test_forward_hint_does_not_override_match_quality() {
        // The hint is only a tiebreaker: it must not promote a worse-quality
        // candidate. D1 wildcard hinted toward Assassin, but the only Assassin
        // is unevolved (Evolvable) while an evolved Defender is Exact for the
        // "any" slot — the evolved pet must still win.
        let dd = two_depth_dungeon(
            vec![make_slot(None, None)],
            vec![make_slot(Some(Class::Assassin), Some(Element::Wind))],
        );

        let pets = vec![
            // Evolved defender → Exact for an "any" slot.
            mock_pet_with_evo("Guard", Element::Earth, Some(Class::Defender),
                RecommendedClass::Single(Class::Defender), true, 1, 1, 10000),
            // Unevolved assassin → only Evolvable, even though it fits the hint.
            mock_pet_with_evo("Recruit", Element::Wind, None,
                RecommendedClass::Single(Class::Assassin), true, 1, 1, 99999),
        ];

        let plan = solve(Dungeon::Scrapyard, 1, &dd, &pets, None);
        assert!(matches!(&plan.assignments[0].assignment,
            Assignment::Filled { pet, quality: MatchQuality::Exact } if pet.name == "Guard"),
            "an Exact evolved pet must beat an Evolvable pet that merely fits the hint");
    }

    // ========================================================================
    // Event enable/disable in coverage
    // ========================================================================

    fn class_event(name: &str, class: Class, optional: bool) -> EventEntry {
        EventEntry {
            name: name.to_string(),
            chance_pct: 20,
            countered_by: vec![CounterCondition {
                item: None,
                class: Some(class),
                element: None,
                count: None,
                quantity_per_clear: None,
                notes: None,
            }],
            optional,
        }
    }

    /// Build a one-depth dungeon whose D1 has the given events and a single
    /// "any" slot, so we can drive the coverage check via `solve_multi`.
    fn dungeon_with_events(events: Vec<EventEntry>) -> DungeonData {
        let mut depths = std::collections::BTreeMap::new();
        depths.insert(1, DepthData {
            rooms: 5, monsters_per_room: 2, gem_level: None,
            requirements: DepthRequirements {
                dungeon_level_avg: 5, levels_per_difficulty: vec![1, 2],
                class_level: 3, total_growth: None,
            },
            monsters: Vec::new(),
            bosses: vec![MonsterEntry { name: "B".into(), element: None, hp: 50, att: 20, def: 10, spd: 10 }],
            party: vec![make_slot(None, None)],
            party_items: Vec::new(), traps: Vec::new(), events,
        });
        DungeonData { name: "Test".to_string(), depths }
    }

    fn event_warning_names(plan: &DungeonPlan) -> Vec<String> {
        plan.warnings
            .iter()
            .filter(|w| matches!(w.kind, CoverageKind::Event))
            .map(|w| w.name.clone())
            .collect()
    }

    #[test]
    fn test_optional_event_skipped_by_default() {
        // A Defender team fails both a normal Rogue event and an optional
        // Blacksmith event. By default only the normal event warns.
        let dd = dungeon_with_events(vec![
            class_event("Rogue Event", Class::Rogue, false),
            class_event("Blacksmith Event", Class::Blacksmith, true),
        ]);
        let pets = vec![mock_pet("Guard", Element::Neutral, Some(Class::Defender),
            RecommendedClass::Single(Class::Defender), true)];

        let requests = [DungeonRequest { dungeon: Dungeon::WaterTemple, depth: 1, data: &dd }];
        let plans = solve_multi(&requests, &pets, &SolverConstraints::default(), None);

        let warned = event_warning_names(&plans[0]);
        assert!(warned.contains(&"Rogue Event".to_string()));
        assert!(
            !warned.contains(&"Blacksmith Event".to_string()),
            "optional events should be skipped by default",
        );
    }

    #[test]
    fn test_event_overrides_flip_coverage() {
        // Overrides can disable a normal event and enable an optional one.
        let dd = dungeon_with_events(vec![
            class_event("Rogue Event", Class::Rogue, false),
            class_event("Blacksmith Event", Class::Blacksmith, true),
        ]);
        let pets = vec![mock_pet("Guard", Element::Neutral, Some(Class::Defender),
            RecommendedClass::Single(Class::Defender), true)];

        let mut constraints = SolverConstraints::default();
        // Disable the normal Rogue event…
        constraints.event_overrides.insert(
            (Dungeon::WaterTemple, 1, "Rogue Event".to_string()), false);
        // …and enable the optional Blacksmith event.
        constraints.event_overrides.insert(
            (Dungeon::WaterTemple, 1, "Blacksmith Event".to_string()), true);

        let requests = [DungeonRequest { dungeon: Dungeon::WaterTemple, depth: 1, data: &dd }];
        let plans = solve_multi(&requests, &pets, &constraints, None);

        let warned = event_warning_names(&plans[0]);
        assert!(
            !warned.contains(&"Rogue Event".to_string()),
            "an explicitly disabled normal event should not warn",
        );
        assert!(
            warned.contains(&"Blacksmith Event".to_string()),
            "an explicitly enabled optional event should warn when uncovered",
        );
    }
}
