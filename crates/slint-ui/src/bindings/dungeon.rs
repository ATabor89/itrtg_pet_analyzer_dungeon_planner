use super::*;
use crate::dungeon::{self, DUNGEONS};

impl Controller {
    pub(super) fn render_dungeon(&self, ui: &MainWindow) {
        let settings = &self.app.session.dungeon;
        ui.set_dungeon_choices(
            Rc::new(VecModel::from(
                DUNGEONS
                    .iter()
                    .enumerate()
                    .map(|(i, (d, name))| {
                        let standard = settings.standards.get(d);
                        crate::DungeonChoice {
                            name: (*name).into(),
                            selected: settings.selected[i],
                            depth: i32::from(settings.depths[i]),
                            tier: i32::from(
                                standard
                                    .and_then(|s| s.min_tier)
                                    .unwrap_or(1)
                                    .max(settings.depths[i]),
                            ),
                            quality: match standard.and_then(|s| s.min_quality) {
                                Some(itrtg_models::Quality::SS) => 1,
                                Some(itrtg_models::Quality::SSS) => 2,
                                _ => 0,
                            },
                            upgrade: i32::from(
                                standard.and_then(|s| s.min_upgrade).unwrap_or(10).max(10),
                            ),
                        }
                    })
                    .collect::<Vec<_>>(),
            ))
            .into(),
        );
        ui.set_dungeon_events(
            Rc::new(VecModel::from(
                self.app
                    .dungeon
                    .events(settings)
                    .into_iter()
                    .map(|(d, depth, name, enabled, label)| crate::DungeonEvent {
                        dungeon: d as i32,
                        depth: i32::from(depth),
                        name: name.into(),
                        enabled,
                        label: label.into(),
                    })
                    .collect::<Vec<_>>(),
            ))
            .into(),
        );
        ui.set_dungeon_preview(log_sections(self.app.dungeon.preview(settings)));
        ui.set_dungeon_results(
            Rc::new(VecModel::from(
                self.app
                    .dungeon
                    .cards(settings)
                    .into_iter()
                    .map(|(title, cards, sections)| crate::DungeonResult {
                        title: title.into(),
                        rows: Rc::new(VecModel::from(
                            cards
                                .chunks(3)
                                .map(|c| crate::LogPartyRow {
                                    cards: log_sections(c.to_vec()),
                                })
                                .collect::<Vec<_>>(),
                        ))
                        .into(),
                        sections: log_sections(sections),
                    })
                    .collect::<Vec<_>>(),
            ))
            .into(),
        );
        ui.set_dungeon_shopping(log_sections(self.app.dungeon.shopping()));
        ui.set_dungeon_teams(
            Rc::new(VecModel::from(
                self.app
                    .dungeon
                    .pending_teams
                    .iter()
                    .zip(&self.app.dungeon.team_choices)
                    .map(|(team, choice)| crate::TeamMapping {
                        label: format!(
                            "Team {}: {}",
                            team.index + 1,
                            team.members
                                .iter()
                                .map(|m| format!("{} #{}", m.name, m.slot))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                        .into(),
                        choice: choice
                            .and_then(|d| DUNGEONS.iter().position(|(key, _)| *key == d))
                            .map(|i| i as i32 + 1)
                            .unwrap_or(0),
                    })
                    .collect::<Vec<_>>(),
            ))
            .into(),
        );
        ui.set_dungeon_dirty(self.app.dungeon.dirty);
        ui.set_dungeon_constraints_enabled(settings.constraints.enabled);
        let c = &settings.constraints;
        let mut constraints: Vec<_> = c
            .forbidden
            .iter()
            .map(|name| crate::PetConstraint {
                name: name.into(),
                label: format!("Forbid · {name}").into(),
            })
            .chain(c.whitelisted.iter().map(|name| crate::PetConstraint {
                name: name.into(),
                label: format!("Whitelist · {name}").into(),
            }))
            .collect();
        constraints.extend(c.forced.iter().map(|f| {
            crate::PetConstraint {
                name: f.pet.as_str().into(),
                label: format!(
                    "Force · {} · {} · {}",
                    f.pet,
                    f.dungeon.map(dungeon::label).unwrap_or("Any team"),
                    f.slot
                        .map(|s| format!("Slot {s}"))
                        .unwrap_or_else(|| "Best slot".into())
                )
                .into(),
            }
        }));
        ui.set_dungeon_constraints(Rc::new(VecModel::from(constraints)).into());
        let mut pets: Vec<_> = self
            .app
            .pets
            .iter()
            .filter(|p| {
                p.is_unlocked()
                    && p.name
                        .to_lowercase()
                        .contains(&self.app.dungeon.search.to_lowercase())
            })
            .map(|p| p.name.clone())
            .collect();
        pets.sort();
        ui.set_dungeon_pets(strings(pets));
    }
    fn dungeon_changed(&mut self, ui: &MainWindow) {
        self.app.dungeon.dirty = !self.app.dungeon.plans.is_empty();
        self.render_dungeon(ui);
        self.save(ui);
    }
}
pub(super) fn wire(ui: &MainWindow, controller: &Rc<RefCell<Controller>>) {
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_dungeon_change(move |key, index, value| {
        let Some(ui) = weak.upgrade() else { return };
        if ui.get_import_busy() {
            return;
        }
        let mut c = state.borrow_mut();
        if key == "team" {
            if let Some(choice) = c.app.dungeon.team_choices.get_mut(index as usize) {
                *choice = DUNGEONS
                    .get(value.wrapping_sub(1) as usize)
                    .map(|(d, _)| *d);
            }
            c.render_dungeon(&ui);
            return;
        }
        let settings = &mut c.app.session.dungeon;
        let Some((d, _)) = DUNGEONS.get(index as usize) else {
            return;
        };
        match key.as_str() {
            "selected" => settings.selected[index as usize] = value != 0,
            "depth" => settings.depths[index as usize] = value.clamp(1, 4) as u8,
            "quality" => {
                settings.standards.entry(*d).or_default().min_quality = Some(match value {
                    1 => itrtg_models::Quality::SS,
                    2 => itrtg_models::Quality::SSS,
                    _ => itrtg_models::Quality::S,
                })
            }
            "tier" => {
                settings.standards.entry(*d).or_default().min_tier = Some(value.clamp(1, 4) as u8)
            }
            "upgrade" => {
                settings.standards.entry(*d).or_default().min_upgrade =
                    Some(value.clamp(10, 20) as u8)
            }
            "constraints" => settings.constraints.enabled = value != 0,
            _ => return,
        }
        c.dungeon_changed(&ui);
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_dungeon_constrain(move |key, name, d, slot| {
        let Some(ui) = weak.upgrade() else { return };
        if ui.get_import_busy() {
            return;
        }
        let mut c = state.borrow_mut();
        let app = &mut c.app;
        match app
            .session
            .dungeon
            .constrain(&key, &name, d, slot, &app.pets)
        {
            Ok(()) => c.dungeon_changed(&ui),
            Err(e) => status(&ui, &e, true),
        }
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_dungeon_event(move |index, depth, name, value| {
        let Some(ui) = weak.upgrade() else { return };
        if ui.get_import_busy() {
            return;
        }
        let Some((d, _)) = DUNGEONS.get(index as usize) else {
            return;
        };
        let mut c = state.borrow_mut();
        let events = &mut c.app.session.dungeon.events;
        events.retain(|e| {
            !(e.dungeon == *d && i32::from(e.depth) == depth && e.event == name.as_str())
        });
        events.push(itrtg_planner::dungeon_state::EventOverride {
            dungeon: *d,
            depth: depth as u8,
            event: name.to_string(),
            enabled: value,
        });
        c.dungeon_changed(&ui);
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_dungeon_action(move |key, text| {
        let Some(ui) = weak.upgrade() else { return };
        if ui.get_import_busy() {
            return;
        }
        if key == "solve" {
            let c = state.borrow();
            if !c.app.session.dungeon.selected.iter().any(|v| *v) {
                status(&ui, "Select at least one dungeon in Setup.", true);
                return;
            }
            let (recs, config, settings, pets) = (
                c.app.dungeon.recs.clone(),
                c.app.dungeon.config.clone(),
                c.app.session.dungeon.clone(),
                c.app.pets.clone(),
            );
            drop(c);
            ui.set_import_busy(true);
            status(&ui, "Solving selected dungeons…", false);
            let (weak, state) = (ui.as_weak(), state.clone());
            platform::run_background(
                move || Ok(dungeon::Model::solve(&recs, &config, &settings, &pets)),
                move |result| {
                    let Some(ui) = weak.upgrade() else { return };
                    ui.set_import_busy(false);
                    match result {
                        Ok(plans) => {
                            let mut c = state.borrow_mut();
                            c.app.dungeon.plans = plans;
                            c.app.dungeon.dirty = false;
                            c.render_dungeon(&ui);
                            status(
                                &ui,
                                "Plans updated. Slots 1–3 are front row; 4–6 are back row.",
                                false,
                            );
                        }
                        Err(error) => status(&ui, &error, true),
                    }
                },
            );
            return;
        }
        if key == "copy" {
            let weak = ui.as_weak();
            platform::copy_text(text.to_string(), move |result| {
                if let Some(ui) = weak.upgrade() {
                    match result {
                        Ok(()) => status(&ui, "Copied to clipboard.", false),
                        Err(e) => status(&ui, &e, true),
                    }
                }
            });
            return;
        }
        let mut c = state.borrow_mut();
        match key.as_str() {
            "search" => {
                c.app.dungeon.search = text.to_string();
                c.render_dungeon(&ui);
            }
            "clear-plans" => {
                c.app.dungeon.plans.clear();
                c.app.dungeon.dirty = false;
                c.render_dungeon(&ui);
            }
            "parse-teams" => match pet_importer::parser::parse_dungeon_teams(&text) {
                Ok(teams) => {
                    c.app.dungeon.team_choices = teams
                        .iter()
                        .map(|t| c.app.session.dungeon.team_dungeons.get(&t.index).copied())
                        .collect();
                    c.app.dungeon.pending_teams = teams;
                    c.render_dungeon(&ui);
                    status(
                        &ui,
                        "Assign each team to a dungeon, then Apply mapped teams.",
                        false,
                    );
                }
                Err(e) => {
                    c.app.dungeon.pending_teams.clear();
                    c.app.dungeon.team_choices.clear();
                    c.render_dungeon(&ui);
                    status(&ui, &e.to_string(), true);
                }
            },
            "apply-teams" => {
                let app = &mut c.app;
                match app.session.dungeon.apply_teams(
                    &app.dungeon.pending_teams,
                    &app.dungeon.team_choices,
                    &app.pets,
                ) {
                    Ok(()) => {
                        c.app.dungeon.pending_teams.clear();
                        c.app.dungeon.team_choices.clear();
                        c.dungeon_changed(&ui);
                        status(
                            &ui,
                            "Mapped teams imported. Unrelated constraints kept. Solve to apply.",
                            false,
                        );
                    }
                    Err(e) => status(&ui, &e, true),
                }
            }
            "clear" => {
                c.app.session.dungeon.constraints = Default::default();
                c.dungeon_changed(&ui);
            }
            "import" => {
                let app = &mut c.app;
                match app.session.dungeon.import_constraints(&text, &app.pets) {
                    Ok(()) => {
                        c.dungeon_changed(&ui);
                        status(&ui, "Constraints imported. Solve to apply them.", false);
                    }
                    Err(e) => status(&ui, &e, true),
                }
            }
            "export" => match serde_yaml::to_string(&c.app.session.dungeon.constraints) {
                Ok(text) => {
                    ui.set_dungeon_transfer(text.into());
                    status(&ui, "Constraint export is ready in the text box.", false);
                }
                Err(e) => status(&ui, &e.to_string(), true),
            },
            "teams" if !c.app.dungeon.dirty => {
                if !c.app.dungeon.plans.iter().any(|plan| plan.assignments.iter().any(|a|
                    matches!(a.assignment, itrtg_planner::solver::Assignment::Filled { .. }))) {
                    status(&ui, "No filled teams to export. Import a roster and solve first.", true);
                    return;
                }
                ui.set_dungeon_transfer(
                    itrtg_planner::dungeon_support::build_dungeon_teams_export(
                        &c.app.dungeon.plans,
                    )
                    .into(),
                );
                status(&ui, "Dungeon Teams export is ready in the text box.", false);
            }
            _ => {}
        }
    });
}
