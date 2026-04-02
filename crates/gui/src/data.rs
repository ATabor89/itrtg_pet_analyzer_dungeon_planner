use std::sync::mpsc;

use itrtg_models::dungeon::{
    DungeonRecommendations, DungeonRecommendationsFile, EquipmentCatalog,
};
use itrtg_models::{ExportPet, WikiPet};
use itrtg_planner::merge::{self, MergedPet};

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
            import_status: None,
            data_version: 0,
        }
    }

    /// Re-merge wiki + export data after either side changes.
    pub fn rebuild_merged(&mut self) {
        self.merged = merge::merge_pets(&self.wiki_pets, &self.export_pets);
        self.data_version += 1;
    }

    /// Start an async wiki fetch on a background thread.
    pub fn fetch_wiki(&mut self) {
        if self.wiki_loading {
            return;
        }
        self.wiki_loading = true;
        self.wiki_error = None;

        let (tx, rx) = mpsc::channel();
        self.wiki_rx = Some(rx);

        std::thread::spawn(move || {
            let result = fetch_wiki_blocking();
            let _ = tx.send(result);
        });
    }

    /// Poll for async wiki fetch completion. Call this every frame.
    pub fn poll_wiki(&mut self) {
        if let Some(rx) = &self.wiki_rx
            && let Ok(result) = rx.try_recv() {
                self.wiki_loading = false;
                match result {
                    Ok(pets) => {
                        let count = pets.len();
                        self.wiki_pets = pets;
                        self.rebuild_merged();
                        self.import_status = Some((
                            format!("Wiki refreshed: {count} pets loaded"),
                            false,
                        ));
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
                self.import_status = Some((
                    format!("Imported {count} pets from export"),
                    false,
                ));
            }
            Err(e) => {
                self.import_status = Some((format!("Import error: {e}"), true));
            }
        }
    }

    /// Import from clipboard.
    pub fn import_from_clipboard(&mut self) {
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
                    self.import_status = Some((format!("Clipboard read error: {e}"), true));
                }
            },
            Err(e) => {
                self.import_status = Some((format!("Clipboard access error: {e}"), true));
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
