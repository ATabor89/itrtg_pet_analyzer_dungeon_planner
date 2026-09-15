//! Thin Slint adapter. Rust owns session state; callbacks issue commands and
//! refresh projections. A single VecModel is retained across updates.
use std::{cell::RefCell, rc::Rc};
use itrtg_models::Element;
use slint::{ComponentHandle, SharedString, VecModel};

use crate::{MainWindow, PetDetails, PetRow, app::{self, AppModel, Ownership, Session, Sort}, platform};

const ELEMENTS: [Option<Element>; 7] = [None, Some(Element::Fire), Some(Element::Water), Some(Element::Wind), Some(Element::Earth), Some(Element::Neutral), Some(Element::All)];
const SORTS: [Sort; 4] = [Sort::Name, Sort::Growth, Sort::DungeonLevel, Sort::ClassLevel];
const OWNERSHIP: [Ownership; 3] = [Ownership::All, Ownership::Owned, Ownership::Locked];

struct Controller {
    app: AppModel,
    rows: Rc<VecModel<PetRow>>,
    can_save: bool,
    save_failed: bool,
}

fn element_color(element: Option<Element>) -> slint::Color {
    let (r, g, b) = match element {
        Some(Element::Fire) => (255, 155, 119),
        Some(Element::Water) => (121, 190, 255),
        Some(Element::Wind) => (121, 215, 187),
        Some(Element::Earth) => (226, 196, 124),
        _ => (184, 168, 220),
    };
    slint::Color::from_rgb_u8(r, g, b)
}

fn unknown() -> SharedString { "—".into() }

impl Controller {
    fn render(&self, ui: &MainWindow, rows_changed: bool) {
        let app = &self.app;
        let visible = app.visible();
        if rows_changed {
            self.rows.set_vec(visible.iter().map(|pet| PetRow {
                name: pet.name.as_str().into(),
                element: pet.element().map(|e| format!("{e:?}").into()).unwrap_or_else(unknown),
                element_color: element_color(pet.element()),
                class_name: pet.export.as_ref().map(|e| e.class.map(|c| format!("{c:?}")).unwrap_or_else(|| "Unevolved".into()).into()).unwrap_or_else(unknown),
                growth: pet.export.as_ref().map(|e| app::number(e.growth).into()).unwrap_or_else(unknown),
                dungeon_level: pet.export.as_ref().map(|e| e.dungeon_level.to_string().into()).unwrap_or_else(unknown),
                class_level: pet.export.as_ref().map(|e| e.class_level.to_string().into()).unwrap_or_else(unknown),
            }).collect::<Vec<_>>());
        }
        ui.set_visible_summary(format!("{} of {} pets", visible.len(), app.pets.len()).into());
        ui.set_owned_count(app.pets.iter().filter(|p| p.is_unlocked()).count().to_string().into());
        ui.set_evolved_count(app.pets.iter().filter(|p| p.is_unlocked() && p.is_evolved()).count().to_string().into());
        ui.set_ready_count(app.pets.iter().filter(|p| p.is_unlocked() && app.readiness(p) == "Growth ready").count().to_string().into());
        ui.set_source_label(app.session.source.as_str().into());
        ui.set_selected_name(app.session.selected.as_deref().unwrap_or("").into());
        ui.set_query(app.session.query.as_str().into());
        ui.set_ownership_index(OWNERSHIP.iter().position(|v| *v == app.session.ownership).unwrap_or(0) as i32);
        ui.set_element_index(ELEMENTS.iter().position(|v| *v == app.session.element).unwrap_or(0) as i32);
        ui.set_sort_index(SORTS.iter().position(|v| *v == app.session.sort).unwrap_or(0) as i32);
        ui.set_pgc_done(app.session.pgc_done as i32);
        ui.set_pgc_max(app.session.pgc_max as i32);
        ui.set_multiplier_label(format!("×{:.2}", app.multiplier()).into());

        let details = app.selected().map(|pet| {
            let requirements = pet.wiki.as_ref().and_then(|w| w.evo_requirements.as_ref()).map(|req| {
                let basis = if req.growth.requires_base_growth() { "base" } else { "total" };
                let mut lines = vec![format!("{} {basis} growth", req.growth.value())];
                if let Some(material) = &req.material { lines.push(material.clone()); }
                if let Some(other) = &req.other { lines.push(other.clone()); }
                lines.join("\n")
            }).unwrap_or_else(|| "No evolution requirements in the wiki snapshot.".into());
            let notes = pet.wiki.as_ref().map(|wiki| {
                let mut notes = vec![wiki.class_bonus.clone()];
                if let Some(ability) = &wiki.special_ability { notes.push(ability.clone()); }
                if let Some(bonus) = &wiki.campaign_bonus { notes.push(format!("Campaign: {}", bonus.raw)); }
                notes.retain(|n| !n.is_empty());
                notes.join("\n\n")
            }).unwrap_or_else(|| "This pet has no matching wiki entry.".into());
            PetDetails {
                name: pet.name.as_str().into(),
                element: pet.element().map(|e| format!("{e:?}").into()).unwrap_or_else(unknown),
                element_color: element_color(pet.element()),
                ownership: match &pet.export { Some(e) if e.unlocked => "Owned", Some(_) => "Locked", None => "No export data" }.into(),
                base_growth: pet.export.as_ref().map(|e| app::number(e.growth).into()).unwrap_or_else(unknown),
                effective_growth: pet.export.as_ref().map(|e| app::number(e.effective_growth_with_global_mult(app.multiplier())).into()).unwrap_or_else(unknown),
                recommended_class: app::recommended_class(pet).into(),
                readiness: app.readiness(pet).into(), requirements: requirements.into(), notes: notes.into(),
            }
        }).unwrap_or_else(|| PetDetails { name: "No pet selected".into(), ..Default::default() });
        ui.set_details(details);
    }

    fn save(&mut self, ui: &MainWindow) {
        if self.can_save {
            if let Err(error) = platform::save(&self.app.session) {
                self.save_failed = true;
                status(ui, &format!("Changes are in memory; could not save prototype settings: {error}"), true);
            } else if self.save_failed {
                self.save_failed = false;
                status(ui, "Prototype settings saved. Storage is working again.", false);
            }
        } else {
            status(ui, "Settings could not be restored. Saving is disabled this run to preserve the existing file.", true);
        }
    }

    fn import(&mut self, ui: &MainWindow, text: &str, label: &str) {
        match self.app.import(text, label) {
            Ok(count) => {
                self.render(ui, true);
                ui.set_import_open(false);
                ui.set_import_text("".into());
                status(ui, &format!("Imported {count} pets. Prototype settings are saved separately from the existing planner."), false);
                self.save(ui);
            }
            Err(error) => status(ui, &error, true),
        }
    }
}

fn status(ui: &MainWindow, message: &str, error: bool) {
    ui.set_status(message.into());
    ui.set_status_error(error);
}

pub fn wire(ui: &MainWindow) -> Result<(), String> {
    let (session, can_save, message) = match platform::load() {
        Ok(Some(session)) => (session, true, "Restored prototype session. Other planner screens remain in the existing app.".to_string()),
        Ok(None) => (Session::default(), true, "Browse the wiki reference, load an example, or import your Pet Stats export.".to_string()),
        Err(error) => (Session::default(), false, format!("Could not restore prototype settings: {error}. Saving disabled to preserve them.")),
    };
    let controller = Rc::new(RefCell::new(Controller { app: AppModel::new(session)?, rows: Rc::new(VecModel::default()), can_save, save_failed: false }));
    ui.set_pets(controller.borrow().rows.clone().into());
    controller.borrow().render(ui, true);
    status(ui, &message, !can_save);

    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_filters_changed(move |query, ownership, element, sort| {
        if let Some(ui) = weak.upgrade() {
            let mut state = state.borrow_mut();
            state.app.session.query = query.to_string();
            state.app.session.ownership = OWNERSHIP.get(ownership as usize).copied().unwrap_or_default();
            state.app.session.element = ELEMENTS.get(element as usize).copied().flatten();
            state.app.session.sort = SORTS.get(sort as usize).copied().unwrap_or_default();
            state.app.reconcile_selection();
            state.render(&ui, true);
            state.save(&ui);
        }
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_select_pet(move |name| {
        if let Some(ui) = weak.upgrade() {
            let mut state = state.borrow_mut();
            state.app.select(&name);
            state.render(&ui, false);
            state.save(&ui);
        }
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_pgc_changed(move |done, max| {
        if let Some(ui) = weak.upgrade() {
            let mut state = state.borrow_mut();
            state.app.session.pgc_max = max.clamp(0, 1000) as u32;
            state.app.session.pgc_done = done.clamp(0, max.max(0)) as u32;
            state.render(&ui, false);
            state.save(&ui);
        }
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_reset_filters(move || {
        if let Some(ui) = weak.upgrade() {
            let mut state = state.borrow_mut();
            state.app.reset_filters();
            state.render(&ui, true);
            state.save(&ui);
        }
    });
    let weak = ui.as_weak();
    ui.on_load_example(move || {
        if let Some(ui) = weak.upgrade() {
            ui.set_import_text(app::EXAMPLE_EXPORT.into());
            ui.set_import_open(true);
            status(&ui, "Example ready to review. Importing replaces the prototype roster; Cancel keeps it.", false);
        }
    });
    let weak = ui.as_weak();
    ui.on_import_paste(move |text| {
        if let Some(ui) = weak.upgrade() {
            let label = if text.as_str() == app::EXAMPLE_EXPORT { "Example · June 2026" } else { "Imported Pet Stats" };
            controller.borrow_mut().import(&ui, &text, label);
        }
    });
    let weak = ui.as_weak();
    ui.on_open_file(move || {
        if let Some(ui) = weak.upgrade() {
            if ui.get_import_busy() { return; }
            ui.set_import_busy(true);
            platform::pick_file(ui.as_weak());
        }
    });
    Ok(())
}

pub fn finish_file_pick(ui: &MainWindow, result: Option<Result<String, String>>) {
    ui.set_import_busy(false);
    match result {
        Some(Ok(text)) => ui.invoke_import_paste(text.into()),
        Some(Err(error)) => status(ui, &format!("Could not open export: {error}"), true),
        None => {},
    }
}
