// Platform abstraction layer.
//
// Provides a unified interface for operations that differ between native
// desktop and WASM/web builds: data file loading, user data persistence,
// and async task spawning.

// =============================================================================
// Game data — baked into the binary on WASM, read from disk on native
// =============================================================================

/// Load the equipment catalog YAML.
pub fn load_equipment_catalog() -> Option<String> {
    load_game_data("data/equipment_catalog.yaml", include_str!("../../../data/equipment_catalog.yaml"))
}

/// Load the dungeon recommendations YAML.
pub fn load_dungeon_recommendations() -> Option<String> {
    load_game_data(
        "data/dungeon_recommendations.yaml",
        include_str!("../../../data/dungeon_recommendations.yaml"),
    )
}

/// Load the wiki pets YAML (pre-parsed pet data).
pub fn load_wiki_pets() -> Option<String> {
    load_game_data(
        "data/wiki_pets.yaml",
        include_str!("../../../data/wiki_pets.yaml"),
    )
}

/// Save updated wiki pets YAML to disk (native only, no-op on WASM).
#[cfg(not(target_arch = "wasm32"))]
pub fn save_wiki_pets(yaml: &str) -> Result<(), String> {
    std::fs::write("data/wiki_pets.yaml", yaml).map_err(|e| format!("Write error: {e}"))
}

#[cfg(target_arch = "wasm32")]
pub fn save_wiki_pets(_yaml: &str) -> Result<(), String> {
    Ok(()) // No persistent storage for wiki data on web
}

/// On native, try to read `path` from disk (falling back to the baked-in data
/// if the file is missing). On WASM, always return the baked-in data.
#[cfg(not(target_arch = "wasm32"))]
fn load_game_data(path: &str, baked: &str) -> Option<String> {
    let p = std::path::Path::new(path);
    if p.exists() {
        std::fs::read_to_string(p).ok()
    } else {
        Some(baked.to_string())
    }
}

#[cfg(target_arch = "wasm32")]
fn load_game_data(_path: &str, baked: &str) -> Option<String> {
    Some(baked.to_string())
}

// =============================================================================
// User data — localStorage on WASM, filesystem on native
// =============================================================================

const PLANNER_CONFIG_KEY: &str = "planner_config";
const PET_CONSTRAINTS_KEY: &str = "pet_constraints";

/// Load the planner configuration YAML (per-user).
pub fn load_planner_config() -> Option<String> {
    load_user_data("data/planner_config.yaml", PLANNER_CONFIG_KEY)
}

/// Load the pet constraints YAML (per-user).
pub fn load_pet_constraints() -> Option<String> {
    load_user_data("data/pet_constraints.yaml", PET_CONSTRAINTS_KEY)
}

/// Save the pet constraints YAML (per-user).
pub fn save_pet_constraints(yaml: &str) -> Result<(), String> {
    save_user_data("data/pet_constraints.yaml", PET_CONSTRAINTS_KEY, yaml)
}

#[cfg(not(target_arch = "wasm32"))]
fn load_user_data(path: &str, _key: &str) -> Option<String> {
    let p = std::path::Path::new(path);
    if p.exists() {
        std::fs::read_to_string(p).ok()
    } else {
        None
    }
}

#[cfg(target_arch = "wasm32")]
fn load_user_data(_path: &str, key: &str) -> Option<String> {
    local_storage_get(key)
}

#[cfg(not(target_arch = "wasm32"))]
fn save_user_data(path: &str, _key: &str, data: &str) -> Result<(), String> {
    std::fs::write(path, data).map_err(|e| format!("Write error: {e}"))
}

#[cfg(target_arch = "wasm32")]
fn save_user_data(_path: &str, key: &str, data: &str) -> Result<(), String> {
    local_storage_set(key, data)
}

// =============================================================================
// localStorage helpers (WASM only)
// =============================================================================

#[cfg(target_arch = "wasm32")]
fn local_storage() -> Result<web_sys::Storage, String> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .ok_or_else(|| "localStorage not available".to_string())
}

#[cfg(target_arch = "wasm32")]
fn local_storage_get(key: &str) -> Option<String> {
    local_storage().ok()?.get_item(key).ok().flatten()
}

#[cfg(target_arch = "wasm32")]
fn local_storage_set(key: &str, value: &str) -> Result<(), String> {
    local_storage()?
        .set_item(key, value)
        .map_err(|_| "Failed to write to localStorage".to_string())
}

// =============================================================================
// Platform queries
// =============================================================================

/// Returns true when running in a browser (WASM target).
#[allow(dead_code)]
pub fn is_web() -> bool {
    cfg!(target_arch = "wasm32")
}
