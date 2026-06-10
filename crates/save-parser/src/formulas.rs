//! Experience-requirement formulas (wiki "Experience level tables",
//! verified to the digit against in-game displays — see
//! `reference/pet_class_and_combat_mechanics.md`).
//!
//! The save stores *current exp toward the next level* (pet `w.c` for
//! dungeon level, `w.d.c` for class level), resetting on level-up; the
//! requirement side is computed from these formulas.

/// Dungeon exp needed to go from `level` to `level + 1`.
///
/// Wiki: exp for DL n−1 → n is `10·(n−1)²` for n < 10 and `10·(n−1)^2.25`
/// for n ≥ 10. Verified exactly: DL 101 → 323,387; DL 80 → 191,405;
/// DL 20 → 8,458.97 (game displays it rounded to 8,459).
pub fn dungeon_exp_to_next(level: u32) -> f64 {
    let l = level as f64;
    if level + 1 < 10 {
        10.0 * l * l
    } else {
        10.0 * l.powf(2.25)
    }
}

/// Class exp needed to go from `level` to `level + 1`.
///
/// Wiki: exp for CL n−1 → n is `1000 + 2000·(n−1)²` (n > 1), with 1,000 to
/// recover CL 1 from a drained CL 0. Verified exactly: CL 25 → 1,251,000;
/// CL 22 → 969,000; CL 19 → 723,000.
pub fn class_exp_to_next(level: u32) -> f64 {
    if level == 0 {
        1000.0
    } else {
        let l = level as f64;
        1000.0 + 2000.0 * l * l
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dungeon_requirements_match_ingame_displays() {
        // User readings 2026-06-11: "DL 101 (147,749 / 323,387)" etc.
        assert_eq!(dungeon_exp_to_next(101).round(), 323_387.0);
        assert_eq!(dungeon_exp_to_next(80).round(), 191_405.0);
        assert_eq!(dungeon_exp_to_next(20).round(), 8_459.0);
        // Below the exponent switch: DL 1 → 2 needs 10.
        assert_eq!(dungeon_exp_to_next(1), 10.0);
        assert_eq!(dungeon_exp_to_next(8), 640.0);
    }

    #[test]
    fn class_requirements_match_ingame_displays() {
        assert_eq!(class_exp_to_next(25), 1_251_000.0);
        assert_eq!(class_exp_to_next(22), 969_000.0);
        assert_eq!(class_exp_to_next(19), 723_000.0);
        // Newly evolved: CL 1 → 2 needs 3,000; drained CL 0 → 1 needs 1,000.
        assert_eq!(class_exp_to_next(1), 3_000.0);
        assert_eq!(class_exp_to_next(0), 1_000.0);
    }
}
