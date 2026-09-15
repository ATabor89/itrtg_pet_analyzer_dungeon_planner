//! Thin Slint adapter. Rust owns session state; callbacks issue commands and
//! refresh projections. A single VecModel is retained across updates.
use std::{cell::RefCell, rc::Rc};
use itrtg_models::Element;
use slint::{ComponentHandle, SharedString, VecModel};

use crate::{controls, details, ChoiceSetting, NumberSetting, DetailSection, MainWindow, PetDetails, PetRow, app::{self, AppModel, Ownership, Session, Sort}, platform};

const ELEMENTS: [Option<Element>; 7] = [None, Some(Element::Fire), Some(Element::Water), Some(Element::Wind), Some(Element::Earth), Some(Element::Neutral), Some(Element::All)];
use crate::controls::SORTS;
use itrtg_planner::analyzer::{campaign_label, TimeSortTiebreak};
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
                recommended: app::recommended_class(pet).into(),
                difficulty: pet.wiki.as_ref().map(|w| format!("{} ({})", w.evo_difficulty.base, w.evo_difficulty.with_conditions).into()).unwrap_or_else(unknown),
                action: app.action_text(pet).into(),
                ranking: details::ranking_value(app, pet).into(),
                name: pet.name.as_str().into(),
                element: pet.element().map(|e| format!("{e:?}").into()).unwrap_or_else(unknown),
                element_color: element_color(pet.element()),
                class_name: pet.export.as_ref().map(|e| e.class.map(|c| format!("{c:?}")).unwrap_or_else(|| "Unevolved".into()).into()).unwrap_or_else(unknown),
                growth: pet.export.as_ref().map(|e| app::number(e.effective_growth_with_global_mult(app.multiplier())).into()).unwrap_or_else(unknown),
                dungeon_level: pet.export.as_ref().map(|e| e.dungeon_level.to_string().into()).unwrap_or_else(unknown),
                class_level: pet.export.as_ref().filter(|e| e.class.is_some()).map(|e| e.class_level.to_string().into()).unwrap_or_else(unknown),
            }).collect::<Vec<_>>());
        }
        ui.set_visible_summary(format!("{} of {} pets", visible.len(), app.pets.len()).into());
        ui.set_owned_count(app.pets.iter().filter(|p| p.is_unlocked()).count().to_string().into());
        ui.set_evolved_count(app.pets.iter().filter(|p| p.is_unlocked() && p.is_evolved()).count().to_string().into());
        ui.set_ready_count(app.pets.iter().filter(|p| p.is_unlocked() && app.readiness(p) == "Growth ready").count().to_string().into());
        ui.set_source_label(app.session.source.as_str().into());
        let mut levels: Vec<_> = app.session.pets.iter().filter(|e| e.unlocked && e.dungeon_level > 0).map(|e| u64::from(e.dungeon_level)).collect();
        levels.sort_unstable_by(|a, b| b.cmp(a));
        let growth: u64 = app.session.pets.iter().filter(|e| e.unlocked).map(|e| e.effective_growth_with_global_mult(app.multiplier())).fold(0, u64::saturating_add);
        let egg_ready = app.pets.iter().filter(|p| p.is_unlocked() && app.readiness(p) == "Ready with egg").count();
        ui.set_roster_summary(format!("Owned effective growth: {} · Top-{} dungeon levels: {} · {} more growth-ready with egg", app::number(growth), levels.len().min(50), app::number(levels.iter().take(50).sum()), egg_ready).into());
        ui.set_selected_name(app.session.selected.as_deref().unwrap_or("").into());
        ui.set_query(app.session.query.as_str().into());
        ui.set_ownership_index(OWNERSHIP.iter().position(|v| *v == app.session.ownership).unwrap_or(0) as i32);
        ui.set_element_index(ELEMENTS.iter().position(|v| *v == app.session.element).unwrap_or(0) as i32);
        ui.set_sort_index(SORTS.iter().position(|v| *v == app.session.sort).unwrap_or(0) as i32);
        ui.set_pgc_done(app.session.pgc_done as i32);
        ui.set_pgc_max(app.session.pgc_max as i32);
        ui.set_multiplier_label(format!("×{:.2}", app.multiplier()).into());

        let settings = app.settings();
        ui.set_ascending(settings.sort_ascending);
        ui.set_advanced_filters(Rc::new(VecModel::from(controls::choices(app).into_iter().map(|c| ChoiceSetting {
            key: c.key.into(), label: c.label.into(), current: c.current as i32,
            options: Rc::new(VecModel::from(c.options.into_iter().map(SharedString::from).collect::<Vec<_>>())).into(),
        }).collect::<Vec<_>>())).into());
        ui.set_number_settings(Rc::new(VecModel::from(controls::numbers(app).into_iter().map(|n| NumberSetting {
            key: n.key.into(), label: n.label.into(), value: n.value.into(), hint: n.hint.into(),
        }).collect::<Vec<_>>())).into());
        ui.set_use_egg(settings.evolve_sort_use_egg);
        ui.set_include_equipment(settings.include_equipment_bonus);
        ui.set_include_class(settings.include_class_bonus);
        ui.set_stone_upgrade(settings.campaign_inputs.stone_campaign_upgrade);
        ui.set_earth_lock(!settings.campaign_inputs.earth_eater_show_lifetime);
        ui.set_tiebreak(i32::from(settings.time_sort_tiebreak == TimeSortTiebreak::EvoDifficulty));
        ui.set_rates_summary(details::rates_summary(app).into());
        ui.set_custom_target(settings.custom_target.as_str().into());
        ui.set_show_ranking(matches!(app.session.sort, Sort::TimeToEvolve | Sort::TimeToTarget) || settings.filter_campaign.is_some());
        ui.set_ranking_title(match app.session.sort {
            Sort::TimeToEvolve => "EVOLVE ETA".into(), Sort::TimeToTarget => "TARGET ETA".into(),
            _ => settings.filter_campaign.map(|c| campaign_label(c).into()).unwrap_or_default(),
        });
        ui.set_has_wiki(app.selected().and_then(|p| p.wiki.as_ref()).is_some_and(|w| !w.wiki_url.is_empty()));
        ui.set_detail_sections(Rc::new(VecModel::from(app.selected().map(|p| details::sections(app, p)).unwrap_or_default().into_iter()
            .map(|(title, body)| DetailSection { title: title.into(), body: body.into() }).collect::<Vec<_>>())).into());
        ui.set_target_result(app.selected().map(|p| details::custom_target(app, p)).unwrap_or_default().into());

        let details = app.selected().map(|pet| {
            PetDetails {
                name: pet.name.as_str().into(),
                element: pet.element().map(|e| format!("{e:?}").into()).unwrap_or_else(unknown),
                element_color: element_color(pet.element()),
                ownership: match &pet.export { Some(e) if e.unlocked => "Owned", Some(_) => "Locked", None => "No export data" }.into(),
                base_growth: pet.export.as_ref().map(|e| app::number(e.growth).into()).unwrap_or_else(unknown),
                effective_growth: pet.export.as_ref().map(|e| app::number(e.effective_growth_with_global_mult(app.multiplier())).into()).unwrap_or_else(unknown),
                recommended_class: app::recommended_class(pet).into(),
                readiness: app.readiness(pet).into(),
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
        if ui.get_import_kind() == 1 {
            match self.app.import_main_stats(text) {
                Ok(message) => {
                    self.render(ui, true);
                    ui.set_import_open(false);
                    ui.set_import_text("".into());
                    status(ui, &message, false);
                    self.save(ui);
                }
                Err(error) => status(ui, &error, true),
            }
            return;
        }
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
            let new_sort = SORTS.get(sort as usize).copied().unwrap_or_default();
            if state.app.session.sort != new_sort { state.app.session.ascending = None; }
            state.app.session.sort = new_sort;
            controls::normalize_sort(&mut state.app);
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
            state.app.reconcile_selection();
            state.render(&ui, true);
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
            ui.set_import_kind(0);
            ui.set_import_text(app::EXAMPLE_EXPORT.into());
            ui.set_import_open(true);
            status(&ui, "Example ready to review. Importing replaces the prototype roster; Cancel keeps it.", false);
        }
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_choice_changed(move |key, index| {
        if let Some(ui) = weak.upgrade() {
            let mut state = state.borrow_mut();
            controls::change_choice(&mut state.app, &key, index.max(0) as usize);
            state.render(&ui, true);
            state.save(&ui);
        }
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_number_changed(move |key, text| {
        if let Some(ui) = weak.upgrade() {
            let mut state = state.borrow_mut();
            match controls::change_number(&mut state.app, &key, &text) {
                Ok(()) => { state.render(&ui, true); status(&ui, "Setting applied.", false); state.save(&ui); },
                Err(error) => status(&ui, &error, true),
            }
        }
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_flag_changed(move |key, value| {
        if let Some(ui) = weak.upgrade() {
            let mut state = state.borrow_mut();
            controls::change_flag(&mut state.app, &key, value);
            state.render(&ui, true);
            state.save(&ui);
        }
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_open_wiki(move || {
        let state = state.borrow();
        if let Some(wiki) = state.app.selected().and_then(|p| p.wiki.as_ref())
            && let Err(error) = platform::open_wiki(&wiki.wiki_url)
            && let Some(ui) = weak.upgrade() { status(&ui, &error, true); }
    });
    let weak = ui.as_weak();
    ui.on_import_paste(move |text| {
        if let Some(ui) = weak.upgrade() {
            if ui.get_import_busy() { return; }
            if ui.get_import_kind() == 2 {
                ui.set_import_busy(true);
                status(&ui, "Decoding full save…", false);
                let state = controller.clone();
                let weak = ui.as_weak();
                platform::decode_save(text.to_string(), move |result| {
                    if let Some(ui) = weak.upgrade() {
                        ui.set_import_busy(false);
                        let mut state = state.borrow_mut();
                        match result.and_then(|data| state.app.apply_save(data)) {
                            Ok(count) => {
                                state.render(&ui, true);
                                ui.set_import_open(false);
                                ui.set_import_text("".into());
                                status(&ui, &format!("Imported {count} pets, account stats and exact Moai levels. Live actions and combat stats are unavailable in saves."), false);
                                state.save(&ui);
                            }
                            Err(error) => status(&ui, &error, true),
                        }
                    }
                });
                return;
            }
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
