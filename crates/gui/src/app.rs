use eframe::egui::{self, RichText};

use crate::data::DataStore;
use crate::platform;
use crate::style;
use crate::views::{analyzer, dungeon, log_viewer};

// =============================================================================
// App
// =============================================================================

#[derive(PartialEq, Eq)]
enum Tab {
    Analyzer,
    DungeonPlanner,
    DungeonLog,
}

pub struct App {
    tab: Tab,
    data: DataStore,
    analyzer_state: analyzer::AnalyzerState,
    dungeon_state: dungeon::DungeonState,
    log_viewer_state: log_viewer::LogViewerState,
    show_import_dialog: bool,
    import_text: String,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        style::configure_style(&cc.egui_ctx);

        let mut data = DataStore::new();

        // Load game data (baked on WASM, from disk on native)
        if let (Some(equip_yaml), Some(recs_yaml)) = (
            platform::load_equipment_catalog(),
            platform::load_dungeon_recommendations(),
        ) {
            data.load_dungeon_recs(&equip_yaml, &recs_yaml);
        }

        // Load wiki pet data from YAML (baked on WASM, from disk on native)
        data.load_wiki_pets_from_yaml();

        // Load per-user configuration (localStorage on WASM, filesystem on native)
        let mut dungeon_state = dungeon::DungeonState::default();

        if let Some(yaml) = platform::load_planner_config()
            && let Err(e) = dungeon_state.load_planner_config(&yaml)
        {
            data.import_status = Some((e, true));
        }

        if let Some(yaml) = platform::load_pet_constraints()
            && let Err(e) = dungeon_state.load_constraints_yaml(&yaml)
        {
            data.import_status = Some((e, true));
        }

        Self {
            tab: Tab::Analyzer,
            data,
            analyzer_state: analyzer::AnalyzerState::default(),
            dungeon_state,
            log_viewer_state: log_viewer::LogViewerState::default(),
            show_import_dialog: false,
            import_text: String::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll async operations
        self.data.poll_wiki();
        self.data.poll_clipboard();

        // Handle dropped files
        ctx.input(|i| {
            for file in &i.raw.dropped_files {
                let is_html = file
                    .path
                    .as_ref()
                    .map(|p| {
                        p.extension()
                            .map(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"))
                            .unwrap_or(false)
                    })
                    .unwrap_or(false)
                    || file.name.ends_with(".html")
                    || file.name.ends_with(".htm");

                // On WASM, eframe always provides file contents as bytes.
                // On native, bytes may be None so we fall back to reading the path.
                #[allow(clippy::unnecessary_literal_unwrap)]
                let text = if let Some(bytes) = &file.bytes {
                    Some(String::from_utf8_lossy(bytes).into_owned())
                } else {
                    #[cfg(not(target_arch = "wasm32"))]
                    { file.path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()) }
                    #[cfg(target_arch = "wasm32")]
                    { None }
                };

                let fname = file.path.as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .or_else(|| {
                        let n = &file.name;
                        if n.is_empty() { None } else { Some(n.clone()) }
                    });

                if let Some(text) = text {
                    if is_html || text.contains("<br>") || text.contains("<BR>") {
                        self.log_viewer_state.load_html(&text, fname);
                        self.tab = Tab::DungeonLog;
                    } else if text.starts_with("Name;") {
                        self.data.import_export(&text);
                    } else if text.contains("{| class=\"wikitable") || text.contains("[[Mouse]]") {
                        match wiki_extractor::parser::parse_pets(&text) {
                            Ok(pets) => {
                                let count = pets.len();
                                self.data.wiki_pets = pets;
                                self.data.rebuild_merged();
                                self.data.import_status = Some((
                                    format!("Loaded {count} pets from dropped file"),
                                    false,
                                ));
                            }
                            Err(e) => {
                                self.data.import_status =
                                    Some((format!("Wiki parse error: {e}"), true));
                            }
                        }
                    } else {
                        self.data.import_status = Some((
                            "Unrecognized file format. Expected pet export, wiki source, or dungeon log HTML."
                                .to_string(),
                            true,
                        ));
                    }
                }
            }
        });

        // Top panel: title, tabs, action buttons
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("ITRTG Pet Planner")
                        .color(style::ACCENT)
                        .size(18.0)
                        .strong(),
                );

                ui.add_space(20.0);

                // Tabs
                if ui
                    .selectable_label(
                        self.tab == Tab::Analyzer,
                        RichText::new("Pet Analyzer").size(14.0),
                    )
                    .clicked()
                {
                    self.tab = Tab::Analyzer;
                }
                if ui
                    .selectable_label(
                        self.tab == Tab::DungeonPlanner,
                        RichText::new("Dungeon Planner").size(14.0),
                    )
                    .clicked()
                {
                    self.tab = Tab::DungeonPlanner;
                }
                if ui
                    .selectable_label(
                        self.tab == Tab::DungeonLog,
                        RichText::new("Dungeon Log").size(14.0),
                    )
                    .clicked()
                {
                    self.tab = Tab::DungeonLog;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Open dungeon log file (native only — WASM uses drag-and-drop)
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if ui
                            .button(RichText::new("\u{1F4C2} Open Log File").size(12.0))
                            .clicked()
                            && let Some(path) = rfd::FileDialog::new()
                                .add_filter("Dungeon Log", &["html", "htm"])
                                .set_directory("data/dungeon_logs")
                                .pick_file()
                                && let Ok(text) = std::fs::read_to_string(&path)
                        {
                            let fname =
                                path.file_name().map(|n| n.to_string_lossy().to_string());
                            self.log_viewer_state.load_html(&text, fname);
                            self.tab = Tab::DungeonLog;
                        }

                        ui.separator();
                    }

                    // Wiki refresh
                    // Wiki refresh (native only — CORS blocks this on WASM)
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if self.data.wiki_loading {
                            ui.spinner();
                            ui.label(
                                RichText::new("Fetching wiki...")
                                    .color(style::TEXT_MUTED)
                                    .size(12.0),
                            );
                        } else if ui
                            .button(RichText::new("\u{21BB} Refresh Wiki").size(12.0))
                            .clicked()
                        {
                            self.data.fetch_wiki();
                        }

                        ui.separator();
                    }

                    if ui
                        .button(RichText::new("\u{1F4CB} Import Clipboard").size(12.0))
                        .clicked()
                    {
                        self.data.import_from_clipboard();
                    }

                    if ui
                        .button(RichText::new("\u{1F4DD} Paste Export").size(12.0))
                        .clicked()
                    {
                        self.show_import_dialog = !self.show_import_dialog;
                    }
                });
            });

            // Status bar
            if let Some((msg, is_err)) = &self.data.import_status {
                ui.horizontal(|ui| {
                    let color = if *is_err { style::ERROR } else { style::SUCCESS };
                    ui.label(RichText::new(msg).color(color).size(11.0));
                });
            }

            ui.add_space(2.0);
        });

        // Import dialog window
        if self.show_import_dialog {
            egui::Window::new("Import Pet Export")
                .collapsible(false)
                .resizable(true)
                .default_size([500.0, 300.0])
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new(
                            "Paste your pet stats export below (semicolon-delimited):",
                        )
                        .color(style::TEXT_MUTED),
                    );
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.import_text)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(10)
                                    .font(egui::TextStyle::Monospace),
                            );
                        });
                    ui.horizontal(|ui| {
                        if ui.button("Import").clicked() && !self.import_text.is_empty() {
                            self.data.import_export(&self.import_text);
                            self.import_text.clear();
                            self.show_import_dialog = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_import_dialog = false;
                        }
                    });
                });
        }

        // Central panel: active view
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.tab {
                Tab::Analyzer => {
                    analyzer::show(ui, &mut self.analyzer_state, &self.data);
                }
                Tab::DungeonPlanner => {
                    dungeon::show(ui, &mut self.dungeon_state, &self.data);
                }
                Tab::DungeonLog => {
                    log_viewer::show(ui, &mut self.log_viewer_state);
                }
            }
        });
    }
}
