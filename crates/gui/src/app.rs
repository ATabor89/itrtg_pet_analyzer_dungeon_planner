use eframe::egui::{self, RichText};

use crate::data::DataStore;
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

        // Try to load dungeon data from data directory
        let equip_path = std::path::Path::new("data/equipment_catalog.yaml");
        let recs_path = std::path::Path::new("data/dungeon_recommendations.yaml");
        if equip_path.exists() && recs_path.exists() {
            if let (Ok(equip_yaml), Ok(recs_yaml)) = (
                std::fs::read_to_string(equip_path),
                std::fs::read_to_string(recs_path),
            ) {
                data.load_dungeon_recs(&equip_yaml, &recs_yaml);
            }
        }

        // Auto-fetch wiki on startup
        data.fetch_wiki();

        // Load planner configuration and pet constraints
        let mut dungeon_state = dungeon::DungeonState::default();

        let config_path = std::path::Path::new("data/planner_config.yaml");
        if config_path.exists() {
            if let Ok(yaml) = std::fs::read_to_string(config_path) {
                if let Err(e) = dungeon_state.load_planner_config(&yaml) {
                    data.import_status = Some((e, true));
                }
            }
        }

        let constraints_path = std::path::Path::new("data/pet_constraints.yaml");
        if constraints_path.exists() {
            if let Ok(yaml) = std::fs::read_to_string(constraints_path) {
                if let Err(e) = dungeon_state.load_constraints_yaml(&yaml) {
                    data.import_status = Some((e, true));
                }
            }
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
                    .unwrap_or(false);

                if let Some(bytes) = &file.bytes {
                    let text = String::from_utf8_lossy(bytes);
                    if is_html || text.contains("<br>") || text.contains("<BR>") {
                        // Dungeon log HTML
                        let fname = file
                            .path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy().to_string());
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
                                    format!("Loaded {count} pets from dropped wiki file"),
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
                } else if let Some(path) = &file.path {
                    if let Ok(text) = std::fs::read_to_string(path) {
                        if is_html || text.contains("<br>") || text.contains("<BR>") {
                            let fname = path.file_name().map(|n| n.to_string_lossy().to_string());
                            self.log_viewer_state.load_html(&text, fname);
                            self.tab = Tab::DungeonLog;
                        } else if text.starts_with("Name;") {
                            self.data.import_export(&text);
                        } else {
                            match wiki_extractor::parser::parse_pets(&text) {
                                Ok(pets) => {
                                    let count = pets.len();
                                    self.data.wiki_pets = pets;
                                    self.data.rebuild_merged();
                                    self.data.import_status = Some((
                                        format!("Loaded {count} pets from {}", path.display()),
                                        false,
                                    ));
                                }
                                Err(e) => {
                                    self.data.import_status =
                                        Some((format!("Parse error: {e}"), true));
                                }
                            }
                        }
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
                    // Open dungeon log file
                    if ui
                        .button(RichText::new("\u{1F4C2} Open Log File").size(12.0))
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Dungeon Log", &["html", "htm"])
                            .set_directory("data/dungeon_logs")
                            .pick_file()
                        {
                            if let Ok(text) = std::fs::read_to_string(&path) {
                                let fname =
                                    path.file_name().map(|n| n.to_string_lossy().to_string());
                                self.log_viewer_state.load_html(&text, fname);
                                self.tab = Tab::DungeonLog;
                            }
                        }
                    }

                    ui.separator();

                    // Wiki refresh
                    if self.data.wiki_loading {
                        ui.spinner();
                        ui.label(
                            RichText::new("Fetching wiki...")
                                .color(style::TEXT_MUTED)
                                .size(12.0),
                        );
                    } else {
                        if ui
                            .button(RichText::new("↻ Refresh Wiki").size(12.0))
                            .clicked()
                        {
                            self.data.fetch_wiki();
                        }
                    }

                    ui.separator();

                    // Import buttons
                    if ui
                        .button(RichText::new("📋 Import Clipboard").size(12.0))
                        .clicked()
                    {
                        self.data.import_from_clipboard();
                    }

                    if ui
                        .button(RichText::new("📝 Paste Export").size(12.0))
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
