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

/// Load the planner config YAML (equipment selection rules).
pub fn load_planner_config() -> Option<String> {
    load_game_data(
        "data/planner_config.yaml",
        include_str!("../../../data/planner_config.yaml"),
    )
}

/// Load the pet special info YAML (per-pet quirks).
pub fn load_pet_special_info() -> Option<String> {
    load_game_data(
        "data/pet_special_info.yaml",
        include_str!("../../../data/pet_special_info.yaml"),
    )
}

/// Load the curated campaign-bonus rules YAML.
pub fn load_campaign_bonuses() -> Option<String> {
    load_game_data(
        "data/campaign_bonuses.yaml",
        include_str!("../../../data/campaign_bonuses.yaml"),
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

const APP_STATE_KEY: &str = "app_state";
const APP_STATE_PATH: &str = "data/app_state.yaml";

/// Load the unified app state YAML (per-user).
pub fn load_app_state() -> Option<String> {
    load_user_data(APP_STATE_PATH, APP_STATE_KEY)
}

/// Save the unified app state YAML (per-user).
pub fn save_app_state(yaml: &str) -> Result<(), String> {
    save_user_data(APP_STATE_PATH, APP_STATE_KEY, yaml)
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
// File download (WASM) — trigger a browser "Save As" for generated text
// =============================================================================

/// Trigger a browser download of `contents` as `filename`. WASM only; on native
/// the save editor writes via the `rfd` file dialog instead.
#[cfg(target_arch = "wasm32")]
pub fn download_text(filename: &str, contents: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;

    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?;

    // Blob([contents]) → object URL → a temporary <a download> we click.
    let parts = js_sys::Array::new();
    parts.push(&wasm_bindgen::JsValue::from_str(contents));
    let blob = web_sys::Blob::new_with_str_sequence(&parts).map_err(|_| "blob creation failed")?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(|_| "object URL failed")?;

    let anchor = document
        .create_element("a")
        .map_err(|_| "anchor creation failed")?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|_| "anchor cast failed")?;
    anchor.set_href(&url);
    anchor.set_download(filename);
    anchor.click();

    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(())
}

// =============================================================================
// Platform queries
// =============================================================================

/// Returns true when running in a browser (WASM target).
#[allow(dead_code)]
pub fn is_web() -> bool {
    cfg!(target_arch = "wasm32")
}
