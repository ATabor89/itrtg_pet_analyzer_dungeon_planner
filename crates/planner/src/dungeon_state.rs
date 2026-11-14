//! Persisted dungeon settings shared by frontends.
use itrtg_models::{Dungeon, Quality};
use serde::{Deserialize, Serialize};

/// A user's explicit enable/disable choice for one dungeon event, identified
/// by dungeon + depth + event name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventOverride {
    pub dungeon: Dungeon,
    pub depth: u8,
    pub event: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DungeonSelection {
    pub dungeon: Dungeon,
    pub depth: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ConstraintsState {
    /// When false the solver ignores all constraints, producing a fresh
    /// unconstrained recommendation. The constraints themselves are still
    /// preserved in the UI so the user can re-enable without re-entering.
    pub enabled: bool,
    pub forbidden: Vec<String>,
    pub forced: Vec<ForcedEntry>,
    pub whitelisted: Vec<String>,
}

/// Custom Default so that `enabled` starts as `true` both when Rust code
/// calls `ConstraintsState::default()` (fresh install via `AppState::default`)
/// AND when serde fills in a missing field from YAML. The `#[serde(default)]`
/// on the struct delegates to this impl for any field not present in the input.
impl Default for ConstraintsState {
    fn default() -> Self {
        Self {
            enabled: true,
            forbidden: Vec::new(),
            forced: Vec::new(),
            whitelisted: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForcedEntry {
    pub pet: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dungeon: Option<Dungeon>,
    /// Exact party slot to pin the pet to, as the in-game slot number 1–6
    /// (1–3 front row, 4–6 back row). Only meaningful when `dungeon` is set;
    /// `None` lets the solver pick the best-fitting open slot. Older saved
    /// states without this field load as `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EquipmentStandardOverride {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_tier: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_quality: Option<Quality>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_upgrade: Option<u8>,
}
