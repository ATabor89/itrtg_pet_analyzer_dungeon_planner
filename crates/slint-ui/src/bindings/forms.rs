//! Glue for the form-rendered screens (Growth Chamber, Save Editor): push the
//! rows Rust builds into Slint models and route `action(id, value)` back.
use super::*;
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

fn bump_revision(ui: &MainWindow) { ui.set_form_revision(ui.get_form_revision().wrapping_add(1)); }

impl Controller {
    /// Refresh whichever form screen is showing (they are built on demand).
    pub(super) fn render_forms(&mut self, ui: &MainWindow) {
        match ui.get_active_view() {
            3 => self.render_chamber(ui),
            _ => {},
        }
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
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_view_changed(move |_| {
        if let Some(ui) = weak.upgrade() { state.borrow_mut().render_forms(&ui); }
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
