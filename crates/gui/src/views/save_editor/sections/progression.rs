//! Progression-track sections: Creations (`i`), Monuments (`D`), Might (`V`),
//! and SpaceDim / Light-Dimension elements (`009.b`).
//!
//! These four are structurally the same — a root list whose elements carry an id
//! (resolved to a name), a level/amount, and a handful of stat fields — so they
//! share one config-driven renderer. Each track is its own left-nav section
//! (mirroring Gems), with a per-track [`TrackSpec`] selecting the base path, the
//! name resolver, and which columns are editable.
//!
//! No typed model for these lists, so values read straight from the raw tree via
//! the matching `save_block!` descriptors; edits stage by raw list index/path.

use std::collections::HashMap;

use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use save_parser::items;
use save_parser::labels::{
    CreationField, DivinityUpgradeField, MightField, MonumentField, SpaceDimField,
};
use save_parser::raw::Raw;

use crate::style;
use crate::views::save_editor::session::EditSession;

/// How a column's cell behaves.
#[derive(Clone, Copy, PartialEq)]
enum Cell {
    /// Read-only display.
    Ro,
    /// Editable, validated as a non-negative integer.
    Uint,
    /// Editable, validated as a number (accepts BigDouble / scientific text,
    /// written verbatim so large magnitudes survive).
    Big,
}

/// One column: a header, the field's sub-key (relative to the element, may be
/// dotted like `e.b`), and how it edits.
struct Col {
    label: &'static str,
    key: &'static str,
    cell: Cell,
}

/// A progression track: its root path, id resolver, and columns.
struct TrackSpec {
    title: &'static str,
    blurb: &'static str,
    /// Root path of the list (e.g. `["i"]`, `["009", "b"]`).
    base: &'static [&'static str],
    /// Sub-key of the element's id field (always `"a"` for these tracks).
    id_key: &'static str,
    resolve: fn(u32) -> Option<&'static str>,
    cols: &'static [Col],
    id_salt: &'static str,
}

const CREATIONS: TrackSpec = TrackSpec {
    title: "Creations",
    blurb: "The clone-creation tracks (Light, Stone, …). Edit the current amount on hand.",
    base: &["i"],
    id_key: CreationField::Creation.key(),
    resolve: items::creation_name,
    cols: &[
        Col { label: "Current Amount", key: CreationField::CurrentAmount.key(), cell: Cell::Big },
        Col { label: "Total Created", key: CreationField::TotalCreated.key(), cell: Cell::Ro },
        Col { label: "Clone Cost", key: CreationField::CloneCost.key(), cell: Cell::Ro },
    ],
    id_salt: "prog_creations",
};

const MONUMENTS: TrackSpec = TrackSpec {
    title: "Monuments",
    blurb: "God-power monuments. Edit the level and the upgrade level.",
    base: &["D"],
    id_key: MonumentField::Monument.key(),
    resolve: items::monument_name,
    cols: &[
        Col { label: "Level", key: MonumentField::Level.key(), cell: Cell::Uint },
        Col { label: "Upgrade Level", key: MonumentField::UpgradeLevel.key(), cell: Cell::Uint },
        Col { label: "Next At", key: MonumentField::NextAt.key(), cell: Cell::Ro },
    ],
    id_salt: "prog_monuments",
};

const MIGHT: TrackSpec = TrackSpec {
    title: "Might",
    blurb: "The Might upgrades (Physical HP +, Battle Might +, …). Edit the level.",
    base: &["V"],
    id_key: MightField::Might.key(),
    resolve: items::might_name,
    cols: &[
        Col { label: "Level", key: MightField::Level.key(), cell: Cell::Uint },
        Col { label: "Next At", key: MightField::NextAt.key(), cell: Cell::Ro },
    ],
    id_salt: "prog_might",
};

const SPACEDIM: TrackSpec = TrackSpec {
    title: "SpaceDim",
    blurb: "Light-Dimension (SpaceDim) elements. Edit the level.",
    base: &["009", "b"],
    id_key: SpaceDimField::Element.key(),
    resolve: items::spacedim_name,
    cols: &[
        Col { label: "Level", key: SpaceDimField::Level.key(), cell: Cell::Uint },
        Col { label: "Clones Allocated", key: SpaceDimField::ClonesAllocated.key(), cell: Cell::Ro },
    ],
    id_salt: "prog_spacedim",
};

const DIVINITY: TrackSpec = TrackSpec {
    title: "Divinity Upgrade",
    blurb: "",
    base: &["K", "l"],
    id_key: DivinityUpgradeField::Upgrade.key(),
    resolve: items::divinity_upgrade_name,
    cols: &[
        Col { label: "Level", key: DivinityUpgradeField::Level.key(), cell: Cell::Uint },
        Col { label: "Next At", key: DivinityUpgradeField::NextAt.key(), cell: Cell::Ro },
        Col { label: "Spread", key: DivinityUpgradeField::Spread.key(), cell: Cell::Ro },
    ],
    id_salt: "prog_divinity",
};

#[derive(Default)]
pub struct ProgEditState {
    /// Per-cell buffers keyed by the field's dotted raw path.
    buffers: HashMap<String, String>,
    status: Option<(String, bool)>,
}

/// Public entry points — one per left-nav section.
pub fn show_creations(ui: &mut egui::Ui, session: &mut EditSession, st: &mut ProgEditState) {
    show(ui, session, st, &CREATIONS);
}
pub fn show_monuments(ui: &mut egui::Ui, session: &mut EditSession, st: &mut ProgEditState) {
    show(ui, session, st, &MONUMENTS);
}
pub fn show_might(ui: &mut egui::Ui, session: &mut EditSession, st: &mut ProgEditState) {
    show(ui, session, st, &MIGHT);
}
pub fn show_spacedim(ui: &mut egui::Ui, session: &mut EditSession, st: &mut ProgEditState) {
    show(ui, session, st, &SPACEDIM);
}

/// One row: id + name + the column values (in `spec.cols` order).
struct Row {
    index: usize,
    id: u32,
    name: String,
    values: Vec<String>,
}

fn read_rows(session: &EditSession, spec: &TrackSpec) -> Vec<Row> {
    let Some(Raw::List(list)) = session.root().get_path(spec.base) else {
        return Vec::new();
    };
    (0..list.len())
        .map(|index| {
            let idx = index.to_string();
            let at = |key: &str| {
                let mut path: Vec<&str> = spec.base.to_vec();
                path.push(&idx);
                path.extend(key.split('.'));
                session.value(&path).unwrap_or_default()
            };
            let id: u32 = at(spec.id_key).trim().parse().unwrap_or(0);
            Row {
                index,
                id,
                name: (spec.resolve)(id).map_or_else(|| format!("id {id}"), str::to_string),
                values: spec.cols.iter().map(|c| at(c.key)).collect(),
            }
        })
        .collect()
}

fn show(ui: &mut egui::Ui, session: &mut EditSession, st: &mut ProgEditState, spec: &TrackSpec) {
    ui.heading(spec.title);

    if session.root().get_path(spec.base).is_none() {
        ui.label(RichText::new("No data for this track in this save.").color(style::TEXT_MUTED));
        return;
    }

    ui.label(RichText::new(spec.blurb).color(style::TEXT_MUTED).size(11.0));
    ui.separator();

    show_status(ui, st);

    let mut edits: Vec<(Vec<String>, String, String)> = Vec::new();
    render_track(ui, &mut st.buffers, session, spec, &mut edits);
    apply(session, st, spec.title, edits);
}

/// Divinity Generator (`K`): the K.l upgrade tracks (editable level), plus a
/// read-only context header for the capacity / clones / storage scalars (which
/// are editable in the Resources section, so they're not duplicated here).
pub fn show_divinity(ui: &mut egui::Ui, session: &mut EditSession, st: &mut ProgEditState) {
    ui.heading("Divinity Generator");

    if session.root().get_path(&["K"]).is_none() {
        ui.label(RichText::new("No Divinity Generator data in this save.").color(style::TEXT_MUTED));
        return;
    }

    let ctx = |path: &[&str]| session.value(path).unwrap_or_else(|| "—".into());
    egui::Grid::new("divinity_ctx").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
        for (label, key) in [("Capacity In Use", "g"), ("Worker Clones", "c"), ("Stone Storage", "n")] {
            ui.label(label);
            ui.label(RichText::new(ctx(&["K", key])).color(style::TEXT_MUTED).monospace().size(11.0));
            ui.end_row();
        }
    });
    ui.label(
        RichText::new("Capacity, worker clones, and stone storage are editable in the Resources section.")
            .color(style::TEXT_MUTED)
            .size(10.0),
    );
    ui.separator();
    ui.label(RichText::new("Upgrades").strong());

    show_status(ui, st);

    let mut edits: Vec<(Vec<String>, String, String)> = Vec::new();
    render_track(ui, &mut st.buffers, session, &DIVINITY, &mut edits);
    apply(session, st, "Divinity", edits);
}

fn show_status(ui: &mut egui::Ui, st: &ProgEditState) {
    if let Some((msg, err)) = &st.status {
        let color = if *err { style::ERROR } else { style::SUCCESS };
        ui.label(RichText::new(msg).color(color).size(12.0));
    }
}

/// Render one track's table, collecting edits. (Heading/blurb/status/apply are
/// the caller's job, so this can be shared by `show` and `show_divinity`.)
fn render_track(
    ui: &mut egui::Ui,
    buffers: &mut HashMap<String, String>,
    session: &EditSession,
    spec: &TrackSpec,
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    let rows = read_rows(session, spec);
    if rows.is_empty() {
        ui.label(RichText::new("No entries.").color(style::TEXT_MUTED));
        return;
    }

    let mut table = TableBuilder::new(ui)
        .striped(true)
        .id_salt(spec.id_salt)
        .column(Column::initial(200.0)); // name
    for _ in spec.cols {
        table = table.column(Column::initial(150.0));
    }
    table
        .header(20.0, |mut h| {
            h.col(|ui| {
                ui.label(RichText::new(spec.title).strong().size(12.0));
            });
            for c in spec.cols {
                h.col(|ui| {
                    ui.label(RichText::new(c.label).strong().size(12.0));
                });
            }
        })
        .body(|body| {
            body.rows(24.0, rows.len(), |mut tr| {
                let row = &rows[tr.index()];
                let idx = row.index.to_string();
                tr.col(|ui| {
                    ui.label(&row.name).on_hover_text(format!("id {}", row.id));
                });
                for (ci, c) in spec.cols.iter().enumerate() {
                    let current = &row.values[ci];
                    tr.col(|ui| match c.cell {
                        Cell::Ro => {
                            ui.label(RichText::new(current).color(style::TEXT_MUTED).size(11.0));
                        }
                        Cell::Uint | Cell::Big => {
                            let mut path: Vec<&str> = spec.base.to_vec();
                            path.push(&idx);
                            path.extend(c.key.split('.'));
                            edit_cell(
                                ui,
                                buffers,
                                &path,
                                current,
                                format!("{} {}", row.name, c.label),
                                c.cell,
                                edits,
                            );
                        }
                    });
                }
            });
        });
}

/// Apply staged edits to the session, recording status on `st`.
fn apply(
    session: &mut EditSession,
    st: &mut ProgEditState,
    title: &str,
    edits: Vec<(Vec<String>, String, String)>,
) {
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
        st.status = Some((format!("Staged {title} edit"), false));
    }
}

/// An editable cell backed by `buffers[path]`. Stages on a valid, changed value:
/// `Uint` requires a non-negative integer; `Big` requires any parseable number
/// (written verbatim so BigDoubles / scientific notation survive).
fn edit_cell(
    ui: &mut egui::Ui,
    buffers: &mut HashMap<String, String>,
    path: &[&str],
    current: &str,
    label: String,
    cell: Cell,
    edits: &mut Vec<(Vec<String>, String, String)>,
) {
    let key = path.join(".");
    let buf = buffers.entry(key).or_insert_with(|| current.to_string());
    let resp = ui.add(egui::TextEdit::singleline(buf).desired_width(120.0));
    if resp.changed() {
        let v = buf.trim();
        let valid = match cell {
            Cell::Uint => v.parse::<u64>().is_ok(),
            Cell::Big => v.parse::<f64>().is_ok_and(f64::is_finite),
            Cell::Ro => false,
        };
        if valid && v != current.trim() {
            edits.push((path.iter().map(|s| s.to_string()).collect(), label, v.to_string()));
        }
    } else if !resp.has_focus() && buf.trim() != current.trim() {
        *buf = current.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use save_parser::container::encode_container;
    use save_parser::raw::Field;

    fn sc(s: &str) -> Field {
        Field::Value(Raw::Scalar(s.into()))
    }

    /// A root with Might (`V`) holding two elements so the list doesn't collapse.
    fn might_session() -> EditSession {
        let v = Raw::List(vec![
            Raw::Struct(vec![("a".into(), sc("0")), ("b".into(), sc("256")), ("m".into(), sc("0"))]),
            Raw::Struct(vec![("a".into(), sc("4")), ("b".into(), sc("512")), ("m".into(), sc("0"))]),
        ]);
        let root = Raw::Struct(vec![("V".into(), Field::Value(v))]);
        EditSession::load(&encode_container(&root.serialize(), "V2"), None).unwrap()
    }

    #[test]
    fn reads_might_rows_with_resolved_names() {
        let s = might_session();
        let rows = read_rows(&s, &MIGHT);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "Physical HP +");
        assert_eq!(rows[0].values[0], "256"); // level column
        assert_eq!(rows[1].name, "Battle Might +");
    }

    #[test]
    fn missing_track_reads_empty() {
        let s = might_session();
        // No Creations block in this save.
        assert!(read_rows(&s, &CREATIONS).is_empty());
    }

    #[test]
    fn reads_divinity_upgrade_rows() {
        // K.l: the 3 divinity-generator upgrade tracks.
        let l = Raw::List(vec![
            Raw::Struct(vec![("a".into(), sc("0")), ("b".into(), sc("512")), ("f".into(), sc("512")), ("g".into(), sc("1"))]),
            Raw::Struct(vec![("a".into(), sc("1")), ("b".into(), sc("256")), ("f".into(), sc("256")), ("g".into(), sc("2"))]),
        ]);
        let k = Raw::Struct(vec![("l".into(), Field::Value(l))]);
        let root = Raw::Struct(vec![("K".into(), Field::Value(Raw::Base64(Box::new(k))))]);
        let s = EditSession::load(&encode_container(&root.serialize(), "V2"), None).unwrap();
        let rows = read_rows(&s, &DIVINITY);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "Capacity");
        assert_eq!(rows[0].values[0], "512"); // level
        assert_eq!(rows[1].name, "Divinity Gain");
    }
}
