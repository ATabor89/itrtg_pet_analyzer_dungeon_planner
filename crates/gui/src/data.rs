use std::sync::mpsc;

use itrtg_models::dungeon::{
    DungeonRecommendations, DungeonRecommendationsFile, EquipmentCatalog,
};
use itrtg_models::{ExportPet, WikiPet};
use itrtg_planner::merge::{self, MergedPet};

use crate::platform;

/// All loaded data, centralized to keep the App struct clean.
pub struct DataStore {
    pub wiki_pets: Vec<WikiPet>,
    pub export_pets: Vec<ExportPet>,
    pub merged: Vec<MergedPet>,
    pub dungeon_recs: Option<DungeonRecommendations>,

    /// Channel for receiving async wiki fetch results.
    wiki_rx: Option<mpsc::Receiver<Result<Vec<WikiPet>, String>>>,
    pub wiki_loading: bool,
    pub wiki_error: Option<String>,

    /// Channel for receiving async clipboard read results (WASM).
    clipboard_rx: Option<mpsc::Receiver<Result<String, String>>>,

    /// Status message for imports.
    pub import_status: Option<(String, bool)>, // (message, is_error)

    /// Incremented every time merged data changes. Used by views to detect stale data.
    pub data_version: u64,
}

impl DataStore {
    pub fn new() -> Self {
        Self {
            wiki_pets: Vec::new(),
            export_pets: Vec::new(),
            merged: Vec::new(),
            dungeon_recs: None,
            wiki_rx: None,
            wiki_loading: false,
            wiki_error: None,
            clipboard_rx: None,
            import_status: None,
            data_version: 0,
        }
    }

    /// Load wiki pets from the baked-in / on-disk YAML file.
    /// This is the primary data source on both native and WASM.
    pub fn load_wiki_pets_from_yaml(&mut self) {
        if let Some(yaml) = platform::load_wiki_pets() {
            match serde_yaml::from_str::<Vec<WikiPet>>(&yaml) {
                Ok(pets) => {
                    let count = pets.len();
                    self.wiki_pets = pets;
                    self.rebuild_merged();
                    self.import_status =
                        Some((format!("Loaded {count} pets from data"), false));
                }
                Err(e) => {
                    self.import_status =
                        Some((format!("Failed to parse wiki pets YAML: {e}"), true));
                }
            }
        }
    }

    /// Re-merge wiki + export data after either side changes.
    pub fn rebuild_merged(&mut self) {
        self.merged = merge::merge_pets(&self.wiki_pets, &self.export_pets);
        self.data_version += 1;
    }

    /// Start an async wiki fetch (native only — CORS blocks this on WASM).
    #[allow(dead_code)]
    ///
    /// On native this spawns a background thread with blocking reqwest.
    pub fn fetch_wiki(&mut self) {
        if self.wiki_loading {
            return;
        }
        self.wiki_loading = true;
        self.wiki_error = None;

        let (tx, rx) = mpsc::channel();
        self.wiki_rx = Some(rx);

        #[cfg(not(target_arch = "wasm32"))]
        {
            std::thread::spawn(move || {
                let result = fetch_wiki_blocking();
                let _ = tx.send(result);
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            // CORS prevents fetching from itrtg.wiki.gg in the browser.
            let _ = tx.send(Err(
                "Wiki refresh is not available in the web version (CORS). \
                 Pet data is loaded from the bundled snapshot."
                    .to_string(),
            ));
        }
    }

    /// Poll for async wiki fetch completion. Call this every frame.
    pub fn poll_wiki(&mut self) {
        if let Some(rx) = &self.wiki_rx
            && let Ok(result) = rx.try_recv()
        {
            self.wiki_loading = false;
            match result {
                Ok(pets) => {
                    let count = pets.len();
                    let old_count = self.wiki_pets.len();

                    // On native, persist the refreshed data to disk if it changed.
                    if count != old_count
                        && let Ok(yaml) = serde_yaml::to_string(&pets)
                        && let Err(e) = platform::save_wiki_pets(&yaml)
                    {
                        log::warn!("Failed to save wiki pets: {e}");
                    }

                    self.wiki_pets = pets;
                    self.rebuild_merged();

                    let msg = if count != old_count {
                        format!(
                            "Wiki refreshed: {count} pets loaded (was {old_count}, updated on disk)"
                        )
                    } else {
                        format!("Wiki refreshed: {count} pets loaded (no changes)")
                    };
                    self.import_status = Some((msg, false));
                    self.wiki_error = None;
                }
                Err(e) => {
                    self.wiki_error = Some(e.clone());
                    self.import_status = Some((format!("Wiki error: {e}"), true));
                }
            }
            self.wiki_rx = None;
        }
    }

    /// Import pet export data from a string (clipboard or file contents).
    pub fn import_export(&mut self, source: &str) {
        match pet_importer::parser::parse_export(source) {
            Ok(pets) => {
                let count = pets.len();
                self.export_pets = pets;
                self.rebuild_merged();
                self.import_status = Some((format!("Imported {count} pets from export"), false));
            }
            Err(e) => {
                self.import_status = Some((format!("Import error: {e}"), true));
            }
        }
    }

    /// Import from clipboard.
    ///
    /// On native: reads synchronously via arboard.
    /// On WASM: kicks off an async clipboard read; poll with `poll_clipboard`.
    pub fn import_from_clipboard(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            match arboard::Clipboard::new() {
                Ok(mut clipboard) => match clipboard.get_text() {
                    Ok(text) => {
                        if text.starts_with("Name;") {
                            self.import_export(&text);
                        } else {
                            self.import_status = Some((
                                "Clipboard doesn't contain a pet export (expected \"Name;\" header)"
                                    .to_string(),
                                true,
                            ));
                        }
                    }
                    Err(e) => {
                        self.import_status =
                            Some((format!("Clipboard read error: {e}"), true));
                    }
                },
                Err(e) => {
                    self.import_status =
                        Some((format!("Clipboard access error: {e}"), true));
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            use eframe::wasm_bindgen::JsCast;

            let (tx, rx) = mpsc::channel();
            self.clipboard_rx = Some(rx);

            wasm_bindgen_futures::spawn_local(async move {
                let result: Result<String, String> = async {
                    let window = web_sys::window()
                        .ok_or_else(|| "No window object".to_string())?;
                    let clipboard = window.navigator().clipboard();
                    let promise = clipboard.read_text();
                    let js_value = wasm_bindgen_futures::JsFuture::from(promise)
                        .await
                        .map_err(|e| format!("Clipboard read failed: {e:?}"))?;
                    js_value
                        .dyn_into::<js_sys::JsString>()
                        .map(String::from)
                        .map_err(|_| "Clipboard did not return text".to_string())
                }
                .await;
                let _ = tx.send(result);
            });
        }
    }

    /// Poll for async clipboard read completion (WASM). Call every frame.
    pub fn poll_clipboard(&mut self) {
        if let Some(rx) = &self.clipboard_rx
            && let Ok(result) = rx.try_recv()
        {
            self.clipboard_rx = None;
            match result {
                Ok(text) => {
                    if text.starts_with("Name;") {
                        self.import_export(&text);
                    } else {
                        self.import_status = Some((
                            "Clipboard doesn't contain a pet export (expected \"Name;\" header)"
                                .to_string(),
                            true,
                        ));
                    }
                }
                Err(e) => {
                    self.import_status = Some((format!("Clipboard error: {e}"), true));
                }
            }
        }
    }

    /// Load dungeon recommendations from the two YAML sources.
    pub fn load_dungeon_recs(&mut self, equipment_yaml: &str, dungeons_yaml: &str) {
        let equipment: EquipmentCatalog = match serde_yaml::from_str(equipment_yaml) {
            Ok(eq) => eq,
            Err(e) => {
                self.import_status = Some((format!("Equipment catalog error: {e}"), true));
                return;
            }
        };
        let file: DungeonRecommendationsFile = match serde_yaml::from_str(dungeons_yaml) {
            Ok(f) => f,
            Err(e) => {
                self.import_status = Some((format!("Dungeon recs error: {e}"), true));
                return;
            }
        };
        self.dungeon_recs = Some(DungeonRecommendations::new(equipment, file));
    }
}

// =============================================================================
// Wiki fetch — native (blocking)
// =============================================================================

#[cfg(not(target_arch = "wasm32"))]
fn fetch_wiki_blocking() -> Result<Vec<WikiPet>, String> {
    let url = "https://itrtg.wiki.gg/wiki/Pets?action=raw";
    let client = reqwest::blocking::Client::builder()
        .user_agent("pet_extractor/0.1.0 (ITRTG tool)")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(url).send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let source = resp.text().map_err(|e| e.to_string())?;
    wiki_extractor::parser::parse_pets(&source).map_err(|e| e.to_string())
}
