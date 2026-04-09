// Unified app state.
//
// Loaded on launch from the platform's user-data store (`data/app_state.yaml`
// on native, the `app_state` localStorage key on WASM). Auto-saved each frame
// when the current state diverges from the last-saved snapshot.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use itrtg_models::{Dungeon, Quality};

use crate::platform;

/// Bump when the on-disk schema changes in a non-backwards-compatible way.
pub const CURRENT_VERSION: u32 = 1;

/// Top-level persisted state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppState {
    pub version: u32,
    pub default_dungeons: Vec<DungeonSelection>,
    pub inventory: BTreeMap<String, u8>,
    pub equipment_standards: BTreeMap<Dungeon, EquipmentStandardOverride>,
    pub constraints: ConstraintsState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DungeonSelection {
    pub dungeon: Dungeon,
    pub depth: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ConstraintsState {
    pub forbidden: Vec<String>,
    pub forced: Vec<ForcedEntry>,
    pub whitelisted: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForcedEntry {
    pub pet: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dungeon: Option<Dungeon>,
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

/// Load result paired with the initial auto-save diff sentinel.
///
/// The sentinel is what `App` stores as `last_saved_yaml`: the first frame's
/// snapshot is compared against it, and a save only fires when they differ.
///
/// - On fresh start (no saved file): the sentinel is the canonical serialization
///   of the empty default state, so no save fires until the user changes something.
/// - On successful load: the sentinel is the canonical serialization of the
///   loaded state, matching what the next snapshot will produce.
/// - On parse failure: the sentinel is the *raw* malformed YAML. The file is left
///   untouched until the user actually changes something in the UI, at which
///   point the corrupt file gets overwritten with the new canonical form.
pub fn load() -> (AppState, String) {
    match platform::load_app_state() {
        None => {
            let state = AppState::default();
            let yaml = serialize(&state);
            (state, yaml)
        }
        Some(raw) => match serde_yaml::from_str::<AppState>(&raw) {
            Ok(state) => {
                let yaml = serialize(&state);
                (state, yaml)
            }
            Err(e) => {
                log::warn!(
                    "Failed to parse app_state.yaml ({e}); keeping file untouched until next UI change"
                );
                (AppState::default(), raw)
            }
        },
    }
}

/// Canonical serialization. Prepends a short header so users who open the file
/// understand it's auto-maintained.
pub fn serialize(state: &AppState) -> String {
    let body = serde_yaml::to_string(state).unwrap_or_default();
    let mut out = String::with_capacity(body.len() + 256);
    out.push_str("# Pet Planner App State (auto-maintained)\n");
    out.push_str("#\n");
    out.push_str("# This file is read on launch and rewritten whenever you change state in the UI.\n");
    out.push_str("# Hand-edits are fine while the tool is closed, but comments will not survive\n");
    out.push_str("# the next save.\n\n");
    out.push_str(&body);
    out
}
