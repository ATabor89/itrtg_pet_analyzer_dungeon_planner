//! Glue for the form-rendered screens (Growth Chamber, Save Editor): push the
//! rows Rust builds into Slint models and route `action(id, value)` back.
use super::*;
use crate::editor::{Editor, Effect};
use crate::form::{Dispatcher, FormModel};
use slint::Model;

/// Slint models and actions of the Growth Chamber screen.
#[derive(Default)]
pub(super) struct ChamberUi {
    run: FormModel,
    settings: FormModel,
    report: FormModel,
    picker: FormModel,
    cards: Rc<VecModel<crate::ChamberCard>>,
    card_forms: Vec<FormModel>,
    card_warn: Vec<bool>,
    dispatcher: Dispatcher<AppModel>,
}

/// Slint models, actions and state of the Save Editor screen.
#[derive(Default)]
pub(super) struct EditorUi {
    pub(super) editor: Editor,
    header: FormModel,
    pending: FormModel,
    top: FormModel,
    rows: FormModel,
    dispatcher: Dispatcher<Editor>,
}

fn bump_revision(ui: &MainWindow) { ui.set_form_revision(ui.get_form_revision().wrapping_add(1)); }

impl Controller {
    /// Commit a value still being typed in a form, ahead of a render that does
    /// not come from that form's own actions (which would otherwise reseed the
    /// field and lose it).
    pub(super) fn flush_forms(&mut self, ui: &MainWindow) {
        let Controller { app, chamber, editor, .. } = self;
        if editor.dispatcher.flush(&mut editor.editor) { editor.editor.settle(); }
        if chamber.dispatcher.flush(app) { self.save(ui); }
    }

    /// Refresh whichever form screen is showing (they are built on demand).
    pub(super) fn render_forms(&mut self, ui: &MainWindow) {
        match ui.get_active_view() {
            3 => self.render_chamber(ui),
            4 => self.render_editor(ui),
            _ => {},
        }
    }

    pub(super) fn render_editor(&mut self, ui: &MainWindow) {
        let view = &mut self.editor;
        let forms = crate::editor::build(&view.editor);
        view.header.set(forms.header);
        view.pending.set(forms.pending);
        view.top.set(forms.top);
        view.rows.set(forms.rows);
        ui.set_editor_loaded(view.editor.session.is_some());
        ui.set_editor_section(crate::editor::section_index(view.editor.section) as i32);
        if let Some(row) = forms.jump { ui.set_editor_jump_row(row as i32); }
        ui.set_editor_jump_token(view.editor.jump_token);
        view.dispatcher.set(forms.acts);
        bump_revision(ui);
    }

    pub(super) fn render_chamber(&mut self, ui: &MainWindow) {
        let forms = crate::chamber::build(&self.app);
        let view = &mut self.chamber;
        view.run.set(forms.run);
        view.settings.set(forms.settings);
        view.report.set(forms.report);
        view.picker.set(forms.picker);
        let count = forms.cards.len();
        for (index, card) in forms.cards.into_iter().enumerate() {
            if index >= view.card_forms.len() {
                let mut model = FormModel::default();
                model.set(card.rows);
                view.cards.push(crate::ChamberCard { rows: model.model(), warn: card.warn });
                view.card_forms.push(model);
                view.card_warn.push(card.warn);
            } else {
                view.card_forms[index].set(card.rows);
                if view.card_warn[index] != card.warn {
                    view.card_warn[index] = card.warn;
                    view.cards.set_row_data(index, crate::ChamberCard { rows: view.card_forms[index].model(), warn: card.warn });
                }
            }
        }
        while view.card_forms.len() > count {
            view.card_forms.pop();
            view.card_warn.pop();
            view.cards.remove(count);
        }
        ui.set_chamber_card_height(forms.card_height);
        view.dispatcher.set(forms.acts);
        bump_revision(ui);
    }
}

pub(super) fn wire(ui: &MainWindow, controller: &Rc<RefCell<Controller>>) {
    {
        let state = controller.borrow();
        ui.set_chamber_run(state.chamber.run.model());
        ui.set_chamber_settings(state.chamber.settings.model());
        ui.set_chamber_report(state.chamber.report.model());
        ui.set_chamber_picker(state.chamber.picker.model());
        ui.set_chamber_cards(state.chamber.cards.clone().into());
    }
    {
        let state = controller.borrow();
        ui.set_editor_header(state.editor.header.model());
        ui.set_editor_pending(state.editor.pending.model());
        ui.set_editor_top(state.editor.top.model());
        ui.set_editor_rows(state.editor.rows.model());
        ui.set_editor_sections(strings(crate::editor::section_titles()));
    }
    {
        let mut state = controller.borrow_mut();
        state.editor.editor.populate_on_load = state.app.session.populate_on_load;
    }
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_editor_select_section(move |index| {
        let Some(ui) = weak.upgrade() else { return };
        let mut state = state.borrow_mut();
        state.flush_forms(&ui);
        state.editor.editor.select_section(index.max(0) as usize);
        state.render_editor(&ui);
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_editor_action(move |id, value| {
        let Some(ui) = weak.upgrade() else { return };
        let effects = {
            let mut controller = state.borrow_mut();
            let view = &mut controller.editor;
            if !view.dispatcher.dispatch(&mut view.editor, &id, &value) { return; }
            view.editor.settle();
            let effects = std::mem::take(&mut view.editor.effects);
            // The "on load" preference is the one editor setting that persists.
            let preference = view.editor.populate_on_load;
            if controller.app.session.populate_on_load != preference {
                controller.app.session.populate_on_load = preference;
                controller.save(&ui);
            }
            controller.render_editor(&ui);
            effects
        };
        for effect in effects { run_effect(&state, &ui, effect); }
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_view_changed(move |_| {
        let Some(ui) = weak.upgrade() else { return };
        let mut state = state.borrow_mut();
        state.flush_forms(&ui);
        state.render_forms(&ui);
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_chamber_action(move |id, value| {
        let Some(ui) = weak.upgrade() else { return };
        let mut state = state.borrow_mut();
        let Controller { app, chamber, .. } = &mut *state;
        if chamber.dispatcher.dispatch(app, &id, &value) {
            state.render_chamber(&ui);
            state.save(&ui);
        }
    });
}

/// Carry out something the editor asked for that needs the platform or the planner.
fn run_effect(controller: &Rc<RefCell<Controller>>, ui: &MainWindow, effect: Effect) {
    let report = |controller: &Rc<RefCell<Controller>>, ui: &MainWindow, message: String, error: bool| {
        let mut state = controller.borrow_mut();
        state.flush_forms(ui);
        state.editor.editor.status = Some((message, error));
        state.render_editor(ui);
    };
    match effect {
        Effect::PickSave => {
            if ui.get_import_busy() { return; }
            ui.set_import_busy(true);
            let (weak, controller) = (ui.as_weak(), controller.clone());
            platform::pick_file(move |result| {
                let Some(ui) = weak.upgrade() else { return };
                ui.set_import_busy(false);
                match result {
                    Some(Ok(file)) => load_save(&controller, &ui, file.name, file.text, false),
                    Some(Err(error)) => report(&controller, &ui, format!("Read failed: {error}"), true),
                    None => {},
                }
            });
        }
        Effect::SaveFile { name, contents } => {
            let (weak, controller) = (ui.as_weak(), controller.clone());
            let shown = name.clone();
            platform::save_text_file(name, contents, move |result| {
                let Some(ui) = weak.upgrade() else { return };
                match result {
                    Ok(true) => report(&controller, &ui, format!("{} {shown}", if platform::NATIVE { "Wrote" } else { "Downloaded" }), false),
                    Ok(false) => {},
                    Err(error) => report(&controller, &ui, format!("Write failed: {error}"), true),
                }
            });
        }
        Effect::Copy(text) => {
            let weak = ui.as_weak();
            platform::copy_text(text, move |result| {
                if let (Some(ui), Err(error)) = (weak.upgrade(), result) { status(&ui, &format!("Could not copy: {error}"), true); }
            });
        }
        Effect::PasteClipboard => {
            let (weak, controller) = (ui.as_weak(), controller.clone());
            platform::read_clipboard(move |result| {
                let Some(ui) = weak.upgrade() else { return };
                match result {
                    Ok(text) => {
                        let mut state = controller.borrow_mut();
                        state.flush_forms(&ui);
                        state.editor.editor.pasted(text);
                        state.render_editor(&ui);
                    }
                    Err(error) => status(&ui, &error, true),
                }
            });
        }
        Effect::Populate => populate(controller, ui),
    }
}

/// Fill the analyzer and Growth Chamber from the save loaded in the editor.
pub(super) fn populate(controller: &Rc<RefCell<Controller>>, ui: &MainWindow) {
    let mut state = controller.borrow_mut();
    state.flush_forms(ui);
    let outcome = match state.editor.editor.extract_for_planner() {
        None => Err("No fully-parsed save to import from — load a save first.".to_string()),
        Some((pets, stats, moai)) => state.app.apply_save(crate::app::PreparedSave { pets, stats, moai }),
    };
    state.editor.editor.status = Some(match outcome {
        Ok(count) => (format!("Imported {count} pets + account stats from save"), false),
        Err(error) => (error, true),
    });
    state.render(ui, true);
    state.render_forms(ui);
    state.save(ui);
}

/// Decode `text` as a save and open it in the Save Editor. `guessed` means the
/// text was only assumed to be a save because nothing else matched.
pub(super) fn load_save(controller: &Rc<RefCell<Controller>>, ui: &MainWindow, name: String, text: String, guessed: bool) {
    if ui.get_import_busy() { return; }
    ui.set_import_busy(true);
    status(ui, "Decoding save…", false);
    let (weak, controller) = (ui.as_weak(), controller.clone());
    // Parser errors are not echoed: malformed decoded fields may include account data.
    platform::run_background(move || save_parser::session::EditSession::load(&text, Some(name).filter(|n| !n.is_empty())).map_err(|_| String::new()), move |result| {
        let Some(ui) = weak.upgrade() else { return };
        ui.set_import_busy(false);
        match result {
            Ok(session) => {
                let populate_now = {
                    let mut state = controller.borrow_mut();
                    let populate_now = state.editor.editor.set_session(session);
                    ui.set_import_open(false);
                    ui.set_import_text("".into());
                    ui.set_active_view(4);
                    state.render_editor(&ui);
                    populate_now
                };
                if populate_now { populate(&controller, &ui); }
                status(&ui, if populate_now { "Save loaded in the Save editor and the planner was filled from it." }
                    else { "Save loaded in the Save editor. “Populate planner” fills the analyzer and Growth Chamber from it." }, false);
            }
            Err(_) if guessed => status(&ui, super::import::UNRECOGNIZED, true),
            Err(_) => {
                status(&ui, "Not a recognized ITRTG save file.", true);
                let mut state = controller.borrow_mut();
                state.editor.editor.status = Some(("Not a recognized ITRTG save file".to_string(), true));
                state.render_editor(&ui);
            }
        }
    });
}
