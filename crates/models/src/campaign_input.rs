//! Player-entered values that the pet export doesn't expose, used by the
//! formula-based campaign bonuses (Beachball, Unicorn, Bear, Ant Queen, Cupid's
//! couples, Aether, Earth Eater, Goblin, and Stone/Golem). Persisted in the app
//! state and passed to the planner via `CampaignContext`.

use serde::{Deserialize, Serialize};

/// Per-player inputs for campaign-bonus formulas. All default to 0 (so the
/// dependent pets contribute nothing until the user fills them in).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CampaignInputs {
    /// Pet Stones currently held — counts toward Beachball's bonus.
    pub pet_stones: u64,
    /// Pet Stones *given* to Beachball — locked in permanently (given in 100k
    /// chunks, no total cap; shown on its mouseover). Beachball's bonus uses
    /// held + given combined.
    pub beachball_given_stones: u64,
    /// Challenge Points — Unicorn.
    pub challenge_points: u64,
    /// Total Honey given to Bear.
    pub honey: u64,
    /// Ants held — Ant Queen.
    pub ants: u64,
    /// Cupid's "current couples" count (token-improved Cupid only).
    pub couples: u32,
    /// Delirious Essence of the Forgotten fights completed — Aether.
    pub delirious_essence_fights: u32,
    /// Earthlike Planets fed to Earth Eater across *all* rebirths — drives its
    /// permanent (token-improved) campaign bonus. Can reach tens of millions, so
    /// the UI accepts engineering/scientific notation (e.g. `32.4e6`).
    pub earth_eater_total_planets: u64,
    /// When `false` (the default), Earth Eater is locked at its realistic +82%
    /// (each rebirth is fed to the cap, and the token upgrade only *reduces* the
    /// per-rebirth penalty — it never makes him worse than the fed value). Set
    /// `true` to instead view the token-improved permanent value derived from
    /// `earth_eater_total_planets`. The UI presents this inverted, as a checked-
    /// by-default "Lock at +82%" box.
    pub earth_eater_show_lifetime: bool,
    /// Ultimate Challenge Challenges completed — Goblin (+1%/all campaigns each,
    /// capped at 75).
    pub goblin_ucc: u32,
    /// Overflow Challenges completed — raises Goblin's Adventurer evo bonus
    /// (capped at 470).
    pub goblin_oc: u32,
    /// Whether the 1500-Challenge-Point "+100% all campaigns" upgrade has been
    /// bought for Stone/Golem.
    pub stone_campaign_upgrade: bool,
}

/// Parse a user-entered number that may be plain (`7136000`), comma-grouped
/// (`7,136,000`), or in scientific/engineering notation (`7.136e6`, `17.13e6`,
/// `7.13691e6`) — the forms the game itself displays. Returns `None` for blank
/// or unparseable input. `f64::from_str` already handles `<mantissa>e<exp>`, so
/// engineering notation (an exponent that's a multiple of 3) needs no special
/// casing; we just strip grouping separators first.
pub fn parse_flexible_number(s: &str) -> Option<f64> {
    let cleaned: String = s.chars().filter(|c| *c != ',' && *c != '_' && !c.is_whitespace()).collect();
    if cleaned.is_empty() {
        return None;
    }
    cleaned.parse::<f64>().ok().filter(|v| v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::parse_flexible_number;

    #[test]
    fn parses_the_forms_the_game_shows() {
        assert_eq!(parse_flexible_number("7136000"), Some(7_136_000.0));
        assert_eq!(parse_flexible_number("7,136,000"), Some(7_136_000.0));
        assert_eq!(parse_flexible_number("7_136_000"), Some(7_136_000.0));
        // Scientific and engineering notation both reduce to mantissa·10^exp.
        assert_eq!(parse_flexible_number("7.136e6"), Some(7_136_000.0));
        assert_eq!(parse_flexible_number("7.13691e6"), Some(7_136_910.0));
        assert_eq!(parse_flexible_number("17.13e6"), Some(17_130_000.0));
        assert_eq!(parse_flexible_number(" 32.4e6 "), Some(32_400_000.0));
        // Blank / junk → None (treated as "unset" by callers).
        assert_eq!(parse_flexible_number(""), None);
        assert_eq!(parse_flexible_number("   "), None);
        assert_eq!(parse_flexible_number("abc"), None);
        assert_eq!(parse_flexible_number("1e999"), None); // overflows to inf
    }
}
