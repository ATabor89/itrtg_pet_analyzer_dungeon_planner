//! Ballpark base-growth-over-time estimates.
//!
//! Two in-game tools grow a pet's *base* growth passively:
//!
//! - **Growing Love Pendant** — gives `1 × (evolved pets)` base growth per hour
//!   to the pet it's equipped on, but only while that pet's base growth is below
//!   your 10th-highest pet's base growth (the cap); it auto-unequips above that.
//!   Only two exist, so dedicating one is a real choice.
//! - **Moai Statues** (Easter 2026) — each gives `0.05 × level` base growth per
//!   hour (capped at `1.0` at level 20) to *every* pet, with no cap.
//!
//! These estimates deliberately ignore feedings, Golden Dragon, and growth
//! campaigns — they're a "which pet should I focus a pendant on" ballpark, not a
//! precise projection. Rates are a snapshot: as pets evolve and grow, the
//! pendant rate rises and the cap shifts, so real time will differ.

use crate::merge::MergedPet;

/// Per-hour base-growth rates and the pendant cap, derived from the player's
/// roster and their Moai statues.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrowthRates {
    /// Number of evolved pets — the Growing Love Pendant's base growth/hour.
    pub evolved_pets: u32,
    /// Combined Moai statue bonus in base growth/hour (applies to all pets).
    pub moai_per_hour: f64,
    /// Pendant cap: the 10th-highest base growth across all pets (including
    /// locked ones). The pendant stops working once a pet reaches this.
    pub pendant_cap: u64,
}

/// Combined Moai base-growth/hour from a list of statue levels. Each statue
/// contributes `0.05 × level`, capped at `1.0` (reached at level 20).
pub fn moai_per_hour(levels: &[u8]) -> f64 {
    levels.iter().map(|&l| (l as f64 * 0.05).min(1.0)).sum()
}

impl GrowthRates {
    /// Derive rates from the merged roster and the player's Moai statue levels.
    pub fn compute(pets: &[MergedPet], moai_levels: &[u8]) -> Self {
        let evolved_pets = pets.iter().filter(|p| p.is_evolved()).count() as u32;

        // Cap = the 10th-highest base growth among all pets that have export
        // data (the export includes locked pets). With fewer than 10 pets the
        // cap is effectively unbounded.
        let mut growths: Vec<u64> = pets
            .iter()
            .filter_map(|p| p.export.as_ref())
            .map(|e| e.growth)
            .collect();
        growths.sort_unstable_by(|a, b| b.cmp(a));
        let pendant_cap = growths.get(9).copied().unwrap_or(u64::MAX);

        Self {
            evolved_pets,
            moai_per_hour: moai_per_hour(moai_levels),
            pendant_cap,
        }
    }

    /// The Growing Love Pendant's base growth/hour.
    pub fn pendant_per_hour(&self) -> f64 {
        self.evolved_pets as f64
    }

    /// Base growth/hour for a pet with a pendant dedicated to it *and* Moai,
    /// while it is still below the pendant cap.
    pub fn combined_per_hour(&self) -> f64 {
        self.pendant_per_hour() + self.moai_per_hour
    }

    /// Estimated hours to grow a pet's base growth from `current` to `target`,
    /// assuming a dedicated pendant plus Moai.
    ///
    /// Returns `Some(0.0)` if already at/above the target, and `None` if the
    /// target is unreachable with these tools — i.e. it's above the pendant cap
    /// and there are no Moai statues to carry growth past the cap.
    pub fn hours_to_target(&self, current: u64, target: u64) -> Option<f64> {
        if current >= target {
            return Some(0.0);
        }
        let mut g = current as f64;
        let target = target as f64;
        let cap = self.pendant_cap as f64;
        let mut hours = 0.0;

        // Below the cap, the pendant contributes alongside Moai.
        if g < cap {
            let combined = self.combined_per_hour();
            if combined <= 0.0 {
                return None;
            }
            let seg_end = target.min(cap);
            hours += (seg_end - g) / combined;
            g = seg_end;
        }

        // At or above the cap, only Moai keeps growing the pet.
        if g < target {
            if self.moai_per_hour <= 0.0 {
                return None;
            }
            hours += (target - g) / self.moai_per_hour;
        }

        Some(hours)
    }
}

/// How a growth climb from `current` toward `target` relates to the pendant
/// cap — used to explain why an estimate is slow (growth past the cap is
/// Moai-only, since the pendant auto-unequips there).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CapRelation {
    /// The whole climb stays below the cap — the pendant works the entire way.
    BelowCap,
    /// The climb crosses the cap: the pendant works until `hours_to_cap`
    /// (`None` if even reaching the cap is impossible), then it's Moai-only.
    CrossesCap { hours_to_cap: Option<f64> },
    /// The pet already exceeds the cap — the pendant never engages (Moai-only).
    AboveCap,
}

impl GrowthRates {
    /// Classify a climb from `current` to `target` against the pendant cap.
    pub fn cap_relation(&self, current: u64, target: u64) -> CapRelation {
        if current >= self.pendant_cap {
            CapRelation::AboveCap
        } else if target > self.pendant_cap {
            CapRelation::CrossesCap {
                hours_to_cap: self.hours_to_target(current, self.pendant_cap),
            }
        } else {
            CapRelation::BelowCap
        }
    }
}

/// Format an hours value as a short human-readable duration ("45 min",
/// "6.2 hours", "3.4 days", "2.1 months", "1.3 years").
pub fn format_duration(hours: f64) -> String {
    if !hours.is_finite() || hours < 0.0 {
        return "—".to_string();
    }
    if hours < 1.0 {
        return format!("{} min", (hours * 60.0).round().max(1.0) as u64);
    }
    if hours < 48.0 {
        return format!("{:.1} hours", hours);
    }
    let days = hours / 24.0;
    if days < 60.0 {
        return format!("{days:.1} days");
    }
    let months = days / 30.0;
    if months < 24.0 {
        return format!("{months:.1} months");
    }
    format!("{:.1} years", days / 365.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moai_per_hour() {
        assert_eq!(moai_per_hour(&[]), 0.0);
        // 0.05 per level.
        assert!((moai_per_hour(&[10]) - 0.5).abs() < 1e-9);
        // Capped at 1.0 per statue (level 20 = max), so level 25 still caps.
        assert!((moai_per_hour(&[20]) - 1.0).abs() < 1e-9);
        assert!((moai_per_hour(&[25]) - 1.0).abs() < 1e-9);
        // Sum across statues.
        assert!((moai_per_hour(&[20, 10, 4]) - (1.0 + 0.5 + 0.2)).abs() < 1e-9);
    }

    fn rates(evolved: u32, moai: f64, cap: u64) -> GrowthRates {
        GrowthRates { evolved_pets: evolved, moai_per_hour: moai, pendant_cap: cap }
    }

    #[test]
    fn test_hours_to_target_below_cap() {
        // 80 evolved + 0 moai = 80/hr. (1000 - 200) / 80 = 10 hours.
        let r = rates(80, 0.0, 54_664);
        assert_eq!(r.hours_to_target(200, 1000), Some(10.0));
        // Already there.
        assert_eq!(r.hours_to_target(1000, 1000), Some(0.0));
        assert_eq!(r.hours_to_target(2000, 1000), Some(0.0));
    }

    #[test]
    fn test_hours_to_target_combines_pendant_and_moai() {
        // 80 evolved + 2.0 moai = 82/hr. (820) / 82 = 10 hours.
        let r = rates(80, 2.0, 1_000_000);
        assert_eq!(r.hours_to_target(180, 1000), Some(10.0));
    }

    #[test]
    fn test_hours_to_target_above_cap_uses_moai_only() {
        // Target beyond the cap: pendant carries to the cap, Moai the rest.
        // cap 1000, current 200, target 1100, pendant 80 + moai 1.0.
        // Below cap: (1000-200)/81 = 9.876..h. Above cap: (1100-1000)/1.0 = 100h.
        let r = rates(80, 1.0, 1000);
        let h = r.hours_to_target(200, 1100).unwrap();
        assert!((h - (800.0 / 81.0 + 100.0)).abs() < 1e-6);
    }

    #[test]
    fn test_hours_to_target_unreachable_past_cap_without_moai() {
        // No Moai and target above cap → pendant auto-unequips, can't get there.
        let r = rates(80, 0.0, 1000);
        assert_eq!(r.hours_to_target(200, 1100), None);
        // But a target at/below the cap is fine.
        assert_eq!(r.hours_to_target(200, 1000), Some(10.0));
    }

    #[test]
    fn test_hours_to_target_no_rates_is_unreachable() {
        // No evolved pets, no Moai → nothing grows.
        let r = rates(0, 0.0, 1_000_000);
        assert_eq!(r.hours_to_target(200, 1000), None);
    }

    #[test]
    fn test_hours_to_target_current_at_cap_uses_moai_only() {
        // Pet already at the cap: pendant won't engage, Moai only.
        let r = rates(80, 1.0, 1000);
        assert_eq!(r.hours_to_target(1000, 1100), Some(100.0));
        // At the cap with no Moai → unreachable.
        let r = rates(80, 0.0, 1000);
        assert_eq!(r.hours_to_target(1000, 1100), None);
    }

    #[test]
    fn test_cap_relation() {
        let r = rates(80, 2.0, 1000); // cap = 1000, combined = 82/hr
        // Entirely below the cap.
        assert_eq!(r.cap_relation(200, 900), CapRelation::BelowCap);
        // Crosses the cap: pendant reaches it, then Moai only.
        match r.cap_relation(180, 5000) {
            CapRelation::CrossesCap { hours_to_cap } => {
                // (1000 - 180) / 82 = 10 hours to the cap.
                assert!((hours_to_cap.unwrap() - 10.0).abs() < 1e-9);
            }
            other => panic!("expected CrossesCap, got {other:?}"),
        }
        // Already above the cap.
        assert_eq!(r.cap_relation(1000, 5000), CapRelation::AboveCap);
        assert_eq!(r.cap_relation(1500, 5000), CapRelation::AboveCap);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0.5), "30 min");
        assert_eq!(format_duration(6.25), "6.2 hours");
        assert_eq!(format_duration(72.0), "3.0 days");
        assert_eq!(format_duration(24.0 * 90.0), "3.0 months");
        assert_eq!(format_duration(24.0 * 365.0 * 2.0), "2.0 years");
        assert_eq!(format_duration(f64::INFINITY), "—");
    }
}
