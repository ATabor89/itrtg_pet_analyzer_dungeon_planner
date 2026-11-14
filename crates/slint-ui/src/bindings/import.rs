//! Every way data gets into the app — the import dialog, the one-click
//! clipboard import, a picked file and a dropped file — ends up in
//! [`import_text`], which recognizes the content and routes it.
use super::*;
use crate::app::ImportKind;

pub(super) const UNRECOGNIZED: &str = "Unrecognized data. Expected a Pet Stats or Main Stats export, a dungeon log, the wiki pet table, or a full save.";

fn close_dialog(ui: &MainWindow) {
    ui.set_import_open(false);
    ui.set_import_text("".into());
}

/// Import `text` as `kind`, or work out what it is when `kind` is `None`.
pub(super) fn import_text(controller: &Rc<RefCell<Controller>>, ui: &MainWindow, name: String, text: String, kind: Option<ImportKind>) {
    if ui.get_import_busy() { return; }
    if text.trim().is_empty() { status(ui, "Nothing to import — no text found.", true); return; }
    let auto = kind.is_none();
    match kind.unwrap_or_else(|| app::detect_import_kind(&text)) {
        ImportKind::Pets => {
            let label = if text == app::EXAMPLE_EXPORT { "Example · June 2026" } else { "Imported Pet Stats" };
            let mut state = controller.borrow_mut();
            match state.app.import(&text, label) {
                Ok(count) => finish(&mut state, ui, &format!("Imported {count} pets from a Pet Stats export.")),
                Err(error) => status(ui, &error, true),
            }
        }
        ImportKind::MainStats => {
            let mut state = controller.borrow_mut();
            match state.app.import_main_stats(&text) {
                Ok(message) => finish(&mut state, ui, &message),
                Err(error) => status(ui, &error, true),
            }
        }
        ImportKind::Wiki => {
            let mut state = controller.borrow_mut();
            match wiki_extractor::parser::parse_pets(&text).map_err(|e| format!("Could not read the wiki source: {e}")).and_then(|pets| state.app.replace_wiki(pets)) {
                Ok((count, _)) => finish(&mut state, ui, &format!("Loaded {count} pets from the wiki source. Your roster is kept.")),
                Err(error) => status(ui, &error, true),
            }
        }
        ImportKind::Log => {
            ui.set_import_busy(true);
            status(ui, "Reading dungeon log…", false);
            let (weak, state) = (ui.as_weak(), controller.clone());
            platform::run_background(move || itrtg_planner::log_parser::parse_dungeon_log(&text), move |result| {
                let Some(ui) = weak.upgrade() else { return };
                ui.set_import_busy(false);
                match result {
                    Ok(log) => {
                        let mut state = state.borrow_mut();
                        state.app.log.replace(log);
                        state.render_log(&ui);
                        ui.set_log_tab(0);
                        ui.set_active_view(1);
                        close_dialog(&ui);
                        status(&ui, "Dungeon log loaded. Pet roster kept. Logs stay in memory for this session.", false);
                    }
                    Err(error) => status(&ui, &format!("Could not load dungeon log: {error}. Previous log kept."), true),
                }
            });
        }
        // A save that was recognized (not explicitly chosen) opens in the Save
        // Editor, which fills the planner too when its "on load" box is ticked.
        ImportKind::Save if auto => forms::load_save(controller, ui, name, text, true),
        ImportKind::Save => {
            ui.set_import_busy(true);
            status(ui, "Decoding full save…", false);
            let (weak, state) = (ui.as_weak(), controller.clone());
            platform::decode_save(text, move |result| {
                let Some(ui) = weak.upgrade() else { return };
                ui.set_import_busy(false);
                let mut state = state.borrow_mut();
                match result.and_then(|data| state.app.apply_save(data)) {
                    Ok(count) => finish(&mut state, &ui, &format!("Imported {count} pets, account stats and exact Moai levels. Live actions and combat stats are unavailable in saves.")),
                    Err(error) => status(&ui, &error, true),
                }
            });
        }
    }
}

/// A successful roster/settings import: refresh everything, close the dialog, persist.
fn finish(state: &mut Controller, ui: &MainWindow, message: &str) {
    state.render(ui, true);
    state.render_forms(ui);
    close_dialog(ui);
    status(ui, message, false);
    state.save(ui);
}

pub(super) fn wire(ui: &MainWindow, controller: &Rc<RefCell<Controller>>) {
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_import_paste(move |text| {
        let Some(ui) = weak.upgrade() else { return };
        let kind = app::IMPORT_KINDS.get(ui.get_import_kind() as usize).copied().flatten();
        import_text(&state, &ui, String::new(), text.to_string(), kind);
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_import_clipboard(move || {
        let Some(ui) = weak.upgrade() else { return };
        if ui.get_import_busy() { return; }
        let (weak, state) = (ui.as_weak(), state.clone());
        platform::read_clipboard(move |result| {
            let Some(ui) = weak.upgrade() else { return };
            match result {
                Ok(text) if text.trim().is_empty() => status(&ui, "The clipboard is empty. Copy an export in the game first.", true),
                Ok(text) => import_text(&state, &ui, String::new(), text, None),
                Err(error) => status(&ui, &error, true),
            }
        });
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_open_file(move || {
        let Some(ui) = weak.upgrade() else { return };
        if ui.get_import_busy() { return; }
        ui.set_import_busy(true);
        let (weak, state) = (ui.as_weak(), state.clone());
        platform::pick_file(move |result| {
            let Some(ui) = weak.upgrade() else { return };
            ui.set_import_busy(false);
            match result {
                Some(Ok(file)) => {
                    let kind = app::IMPORT_KINDS.get(ui.get_import_kind() as usize).copied().flatten();
                    import_text(&state, &ui, file.name, file.text, kind);
                }
                Some(Err(error)) => status(&ui, &format!("Could not open file: {error}"), true),
                None => {},
            }
        });
    });
    let weak = ui.as_weak();
    ui.on_load_example(move || {
        if let Some(ui) = weak.upgrade() {
            ui.set_import_kind(app::import_kind_index(Some(ImportKind::Pets)));
            ui.set_import_text(app::EXAMPLE_EXPORT.into());
            ui.set_import_open(true);
            status(&ui, "Example ready to review. Importing replaces the roster; Cancel keeps it.", false);
        }
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_refresh_wiki(move || {
        let Some(ui) = weak.upgrade() else { return };
        if ui.get_wiki_busy() { return; }
        ui.set_wiki_busy(true);
        status(&ui, "Fetching the pet table from the wiki…", false);
        let (weak, state) = (ui.as_weak(), state.clone());
        platform::run_background(platform::fetch_wiki, move |result| {
            let Some(ui) = weak.upgrade() else { return };
            ui.set_wiki_busy(false);
            let mut state = state.borrow_mut();
            let before = state.app.wiki_len();
            match result.and_then(|pets| state.app.replace_wiki(pets)) {
                Ok((count, changed)) => {
                    // Only a validated table that differs is written back, with
                    // the evolution requirements it kept.
                    if changed { platform::save_wiki(state.app.wiki()); }
                    state.render(&ui, true);
                    state.render_forms(&ui);
                    status(&ui, &if changed { format!("Wiki data refreshed: {count} pets ({before} before).") } else { format!("Wiki data is up to date: {count} pets.") }, false);
                }
                Err(error) => status(&ui, &format!("Wiki refresh failed: {error}. Existing wiki data kept."), true),
            }
        });
    });
}

/// A file dropped on the window is imported straight away, by content.
pub fn finish_drop(controller: &Rc<RefCell<Controller>>, ui: &MainWindow, result: Result<platform::PickedFile, String>) {
    ui.set_import_busy(false);
    match result {
        Ok(file) => import_text(controller, ui, file.name, file.text, None),
        Err(error) => status(ui, &format!("Could not read dropped file: {error}"), true),
    }
}
