//! Player-entered values that the pet export doesn't expose, used by the
//! formula-based campaign bonuses (Beachball, Unicorn, Bear, Ant Queen, Cupid's
//! couples, and Aether). Persisted in the app state and passed to the planner
//! via `CampaignContext`.

use serde::{Deserialize, Serialize};

/// Per-player inputs for campaign-bonus formulas. All default to 0 (so the
/// dependent pets contribute nothing until the user fills them in).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CampaignInputs {
    /// Pet Stones currently held — counts toward Beachball's bonus.
    pub pet_stones: u64,
    /// Pet Stones *given* to Beachball (locked in, max 100k; shown on its
    /// mouseover). Beachball's bonus uses held + given combined.
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
}
