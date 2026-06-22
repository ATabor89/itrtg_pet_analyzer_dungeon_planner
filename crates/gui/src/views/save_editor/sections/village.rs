//! Pet Village section: root `024`. Edits the per-building scalars (levels, the
//! Tavern's quest points, the Strategy Room's stat multipliers) and the Museum
//! statue levels. Worker-slot assignment and the building-state list stay in the
//! Raw Save Tree (niche); managers are shown read-only for context.
//!
//! No typed model for these structs, so values read straight from the raw tree;
//! edits stage by path / raw list index.

use std::collections::HashMap;

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use save_parser::items;
use save_parser::raw::Raw;

use crate::style;
use crate::views::save_editor::session::EditSession;

/// One editable building scalar: a display label and its dotted path under `024`.
struct ScalarField {
    label: &'static str,
    path: &'static [&'static str],
}

/// A building group: a heading plus its editable scalar fields.
struct Building {
    name: &'static str,
    fields: &'static [ScalarField],
}

const BUILDINGS: &[Building] = &[
    Building {
        name: "Tavern",
        fields: &[
            ScalarField { label: "Level", path: &["024", "b", "b"] },
            ScalarField { label: "Quest Points", path: &["024", "b", "d"] },
            ScalarField { label: "Quests Per Day", path: &["024", "b", "i"] },
            ScalarField { label: "Max Concurrent Quests", path: &["024", "b", "j"] },
        ],
    },
    Building {
        name: "Dojo",
        fields: &[ScalarField { label: "Level", path: &["024", "d", "b"] }],
    },
    Building {
        name: "Strategy Room",
        fields: &[
            ScalarField { label: "Level", path: &["024", "e", "b"] },
            ScalarField { label: "Physical Multi %", path: &["024", "e", "e"] },
            ScalarField { label: "Mystic Multi %", path: &["024", "e", "f"] },
            ScalarField { label: "Battle Multi %", path: &["024", "e", "g"] },
        ],
    },
    Building {
        name: "Material Factory",
        fields: &[ScalarField { label: "Level", path: &["024", "g", "a"] }],
    },
    Building {
        name: "Alchemy Hut",
        fields: &[ScalarField { label: "Level", path: &["024", "h", "a"] }],
    },
];

struct StatueRow {
    /// Full raw path of the level (`a`) field — either list-index form
    /// (`024.f.a.<i>.a`) or, for a single statue stored as a lone struct,
    /// `024.f.a.a`.
    level_path: Vec<String>,
    name: String,
    level: String,
}

/// State for the "Add statue" modal.
#[derive(Default)]
struct AddStatueState {
    open: bool,
    id: u32,
    level: String,
}

#[derive(Default)]
pub struct VillageEditState {
    /// Buffers for the building scalar fields, keyed by dotted path.
    scalar_buffers: HashMap<String, String>,
    /// Per-row buffers for statue levels, keyed by the level field's dotted path.
    statue_buffers: HashMap<String, String>,
    add_statue: AddStatueState,
    status: Option<(String, bool)>,
}

/// The known museum statue ids (the `JBGNCMHGOFI` enum, 1..=11), for the picker.
fn known_statues() -> Vec<(u32, &'static str)> {
    (1..=11u32).filter_map(|id| items::statue_name(id).map(|n| (id, n))).collect()
}

fn read_statues(session: &EditSession) -> Vec<StatueRow> {
    // A single statue re-parses as a lone struct (a 1-element `&`-list collapses),
    // so handle both the list form and the lone-struct form.
    let prefixes: Vec<Vec<String>> = match session.root().get_path(&["024", "f", "a"]) {
        Some(Raw::List(items)) => (0..items.len())
            .map(|i| vec!["024".to_string(), "f".into(), "a".into(), i.to_string()])
            .collect(),
        Some(Raw::Struct(_)) => vec![vec!["024".to_string(), "f".into(), "a".into()]],
        _ => return Vec::new(),
    };
    prefixes
        .into_iter()
        .map(|prefix| {
            // MUSEUM_STATUE_FIELDS: a = level, b = statue id.
            let val = |k: &str| {
                let mut path = prefix.clone();
                path.push(k.to_string());
                let p: Vec<&str> = path.iter().map(String::as_str).collect();
                session.value(&p).unwrap_or_default()
            };
            let id: u32 = val("b").trim().parse().unwrap_or(0);
            let mut level_path = prefix.clone();
            level_path.push("a".to_string());
            StatueRow {
                level_path,
                name: items::statue_name(id).map_or_else(|| format!("Statue {id}"), str::to_string),
                level: val("a"),
            }
        })
        .collect()
}

/// Resolve a worker building's manager (`024.{g,h}.e`, pet type id; 999 = empty).
fn manager_label(session: &EditSession, building: &str) -> String {
    match session.value(&["024", building, "e"]).and_then(|s| s.trim().parse::<u32>().ok()) {
        None | Some(999) => "—".to_string(),
        Some(id) => items::pet_type_name(id).map_or_else(|| format!("type {id}"), str::to_string),
    }
}

pub fn show(ui: &mut egui::Ui, session: &mut EditSession, st: &mut VillageEditState) {
    ui.heading("Pet Village");

    if session.root().get_path(&["024"]).is_none() {
        ui.label(RichText::new("No Pet Village data in this save.").color(style::TEXT_MUTED));
        return;
    }

    if let Some((msg, err)) = &st.status {
        let color = if *err { style::ERROR } else { style::SUCCESS };
        ui.label(RichText::new(msg).color(color).size(12.0));
    }

    let statues = read_statues(session);
    let factory_mgr = manager_label(session, "g");
    let alchemy_mgr = manager_label(session, "h");

    let mut edits: Vec<(Vec<String>, String, String)> = Vec::new();

    for b in BUILDINGS {
        ui.add_space(4.0);
        ui.label(RichText::new(b.name).strong());
        egui::Grid::new(("village_b", b.name)).num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
            for f in b.fields {
                scalar_editor(ui, session, st, f.path, f.label, &mut edits);
                ui.end_row();
            }
        });
        // Worker buildings: show the manager (read-only).
        if b.name == "Material Factory" {
            ui.label(RichText::new(format!("Manager: {factory_mgr}")).color(style::TEXT_MUTED).size(11.0));
        } else if b.name == "Alchemy Hut" {
            ui.label(RichText::new(format!("Manager: {alchemy_mgr}")).color(style::TEXT_MUTED).size(11.0));
        }
    }

    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(RichText::new("Museum Statues").strong());
        if ui.button("➕ Add statue").clicked() {
            let id = known_statues().first().map_or(1, |(id, _)| *id);
            st.add_statue = AddStatueState { open: true, id, level: "20".into() };
        }
    });
    if statues.is_empty() {
        ui.label(RichText::new("No statues.").color(style::TEXT_MUTED));
    } else {
        statue_table(ui, st, &statues, &mut edits);
    }

    // Apply.
    let mut ok = false;
    let mut had_err = false;
    for (path, label, value) in edits {
        let p: Vec<&str> = path.iter().map(String::as_str).collect();
        match session.set_scalar(&p, label, &value) {
            Ok(_) => ok = true,
            Err(e) => {
                had_err = true;
                st.status = Some((format!("Edit failed: {e}"), true));
            }
        }
    }
    if ok && !had_err {
        st.status = Some(("Staged Pet Village edit".to_string(), false));
    }

    // Add-statue modal.
    if let Some((id, level)) = add_statue_window(ui.ctx(), &mut st.add_statue) {
        let label = format!("{} statue", items::statue_name(id).unwrap_or("Statue"));
        st.status = Some(match session.add_statue(id, level, label) {
            Ok(()) => ("Added statue".to_string(), false),
            Err(e) => (format!("Add failed: {e}"), true),
        });
    }
}

/// The "Add statue" modal. Returns `Some((statue_id, level))` on Add.
fn add_statue_window(ctx: &egui::Context, st: &mut AddStatueState) -> Option<(u32, u32)> {
    if !st.open {
        return None;
    }
    let mut result = None;
    let mut close = false;
    let mut window_open = true;
    egui::Window::new("Add Statue")
        .collapsible(false)
        .resizable(false)
        .open(&mut window_open)
        .show(ctx, |ui| {
            let opts = known_statues();
            let selected = opts
                .iter()
                .find(|(id, _)| *id == st.id)
                .map_or_else(|| format!("id {}", st.id), |(_, n)| (*n).to_string());
            ui.horizontal(|ui| {
                ui.label("Statue:");
                egui::ComboBox::from_id_salt("village_add_statue")
                    .selected_text(selected)
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for (id, name) in &opts {
                            ui.selectable_value(&mut st.id, *id, *name);
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Level:");
                ui.add(egui::TextEdit::singleline(&mut st.level).desired_width(80.0));
                ui.label(RichText::new("(max 20)").color(style::TEXT_MUTED).size(10.0));
            });
            ui.label(
                RichText::new("Statues aren't unique — you can own two of each.")
                    .color(style::TEXT_MUTED)
                    .size(10.0),
            );
            ui.separator();
            ui.horizontal(|ui| {
                let level_ok = st.level.trim().parse::<u32>().is_ok_and(|n| n <= 20);
                if ui.add_enabled(level_ok, egui::Button::new("Add")).clicked() {
                    result = Some((st.id, st.level.trim().parse().unwrap()));
                    close = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
    st.open = window_open && !close;
    result
}

/// A labeled, validated editable scalar; stages an edit into `edits` on change.
fn scalar_editor(
    ui: &mut egui::Ui,
    session: &EditSession,
    st: &mut VillageEditState,
    path: &[&str],
    label: &str,
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    ui.label(label);
    let key = path.join(".");
    let current = session.value(path).unwrap_or_default();
    let buf = st.scalar_buffers.entry(key).or_insert_with(|| current.clone());
    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(140.0));
    if resp.lost_focus() {
        let v = buf.trim().to_string();
        if v != current && v.parse::<f64>().is_ok() {
            edits.push((path.iter().map(|s| s.to_string()).collect(), label.to_string(), v));
        }
    } else if !resp.has_focus() && buf.as_str() != current {
        *buf = current;
    }
}

fn statue_table(
    ui: &mut egui::Ui,
    st: &mut VillageEditState,
    rows: &[StatueRow],
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    TableBuilder::new(ui)
        .striped(true)
        .id_salt("village_statues")
        .column(Column::initial(240.0))
        .column(Column::remainder())
        .header(20.0, |mut h| {
            for t in ["Statue", "Level"] {
                h.col(|ui| {
                    ui.label(RichText::new(t).strong().size(12.0));
                });
            }
        })
        .body(|body| {
            body.rows(24.0, rows.len(), |mut tr| {
                let row = &rows[tr.index()];
                tr.col(|ui| {
                    ui.label(&row.name);
                });
                tr.col(|ui| {
                    let key = row.level_path.join(".");
                    let buf = st.statue_buffers.entry(key).or_insert_with(|| row.level.clone());
                    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(80.0));
                    if resp.changed() {
                        let v = buf.trim();
                        if v.parse::<u64>().is_ok() && v != row.level.trim() {
                            edits.push((
                                row.level_path.clone(),
                                format!("{} level", row.name),
                                v.to_string(),
                            ));
                        }
                    } else if !resp.has_focus() && buf.trim() != row.level.trim() {
                        *buf = row.level.clone();
                    }
                });
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use save_parser::container::encode_container;
    use save_parser::raw::Field;

    fn sc(s: &str) -> Field {
        Field::Value(Raw::Scalar(s.into()))
    }
    fn b64(r: Raw) -> Field {
        Field::Value(Raw::Base64(Box::new(r)))
    }

    /// Add a statue to an EMPTY museum, save+reload, and confirm it still shows.
    /// A 1-element list collapses to a lone struct on reload — this is the exact
    /// scenario "Add statue" targets, so read_statues must handle it.
    #[test]
    fn add_first_statue_survives_reload_as_lone_struct() {
        // Pet Village with a museum (024.f) but NO statues yet, nested base64
        // like a real save.
        let museum = Raw::Struct(vec![("b".into(), sc("1"))]); // some field, no `a` list
        let village = Raw::Struct(vec![("f".into(), b64(museum))]);
        let root = Raw::Struct(vec![("024".into(), b64(village))]);
        let mut s = EditSession::load(&encode_container(&root.serialize(), "V2"), None).unwrap();
        s.add_statue(8, 15, "Halloween 2025 statue").unwrap();

        // Round-trip through a save+reload so the 1-element list collapses.
        let encoded = s.encode();
        let reloaded = EditSession::load(&encoded, None).unwrap();
        let rows = read_statues(&reloaded);
        assert_eq!(rows.len(), 1, "the single statue must still be visible after reload");
        assert_eq!(rows[0].name, "Halloween 2025");
        assert_eq!(rows[0].level, "15");
        // The level edit path must target the lone-struct field, not an index.
        assert_eq!(rows[0].level_path, vec!["024", "f", "a", "a"]);
        // And it's editable via that path.
        let p: Vec<&str> = rows[0].level_path.iter().map(String::as_str).collect();
        assert_eq!(reloaded.value(&p).as_deref(), Some("15"));
    }

    #[test]
    fn buildings_cover_the_expected_groups() {
        let names: Vec<&str> = BUILDINGS.iter().map(|b| b.name).collect();
        assert_eq!(names, ["Tavern", "Dojo", "Strategy Room", "Material Factory", "Alchemy Hut"]);
        // Every field path is rooted at 024.
        for b in BUILDINGS {
            for f in b.fields {
                assert_eq!(f.path[0], "024", "{} {}", b.name, f.label);
            }
        }
    }
}
