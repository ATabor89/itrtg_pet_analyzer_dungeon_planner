//! Progression tracks: Creations (`i`), Monuments (`D`), Might (`V`), SpaceDim /
//! Light-Dimension elements (`009.b`) and the Divinity Generator upgrades
//! (`K.l`).
//!
//! They are structurally the same — a list whose elements carry an id (resolved
//! to a name), a level/amount and a few stat fields — so one config-driven
//! renderer serves them all. There is no typed model for these lists: values
//! read straight from the raw tree and edits stage by raw list index.
use save_parser::items;
use save_parser::labels::{CreationField, DivinityUpgradeField, MightField, MonumentField, SpaceDimField};
use save_parser::raw::Raw;
use save_parser::session::EditSession;

use crate::editor::common::{owned, scalar_noted, stage_button, Check, F};
use crate::form::{muted, spacer, text};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Track { Creations, Monuments, Might, SpaceDim, Divinity }

/// One column: its header, the field's key (relative to the element) and how it
/// edits (`None` = read-only).
type Column = (&'static str, &'static str, Option<Check>);

struct Spec {
    title: &'static str,
    blurb: &'static str,
    /// Root path of the list.
    base: &'static [&'static str],
    id_key: &'static str,
    resolve: fn(u32) -> Option<&'static str>,
    columns: &'static [Column],
    /// Section note after a staged edit.
    done: &'static str,
}

// `Finite` cells take BigDouble / scientific text, written verbatim so large
// magnitudes survive.
fn spec(track: Track) -> Spec {
    match track {
        Track::Creations => Spec {
            title: "Creations", blurb: "The clone-creation tracks (Light, Stone, …). Edit the current amount on hand.",
            base: &["i"], id_key: CreationField::Creation.key(), resolve: items::creation_name, done: "Staged Creations edit",
            columns: const { &[
                ("Current Amount", CreationField::CurrentAmount.key(), Some(Check::Finite)),
                ("Total Created", CreationField::TotalCreated.key(), None),
                ("Clone Cost", CreationField::CloneCost.key(), None),
            ] },
        },
        Track::Monuments => Spec {
            title: "Monuments", blurb: "God-power monuments. Edit the level and the upgrade level.",
            base: &["D"], id_key: MonumentField::Monument.key(), resolve: items::monument_name, done: "Staged Monuments edit",
            columns: const { &[
                ("Level", MonumentField::Level.key(), Some(Check::Uint)),
                ("Upgrade Level", MonumentField::UpgradeLevel.key(), Some(Check::Uint)),
                ("Next At", MonumentField::NextAt.key(), None),
            ] },
        },
        Track::Might => Spec {
            title: "Might", blurb: "The Might upgrades (Physical HP +, Battle Might +, …). Edit the level; usable-skill Mights show their cooldown (0 = ready).",
            base: &["V"], id_key: MightField::Might.key(), resolve: items::might_name, done: "Staged Might edit",
            columns: const { &[
                ("Level", MightField::Level.key(), Some(Check::Uint)),
                ("Cooldown (ms)", MightField::Cooldown.key(), None),
            ] },
        },
        Track::SpaceDim => Spec {
            title: "SpaceDim", blurb: "Light-Dimension (SpaceDim) elements. Edit the level.",
            base: &["009", "b"], id_key: SpaceDimField::Element.key(), resolve: items::spacedim_name, done: "Staged SpaceDim edit",
            columns: const { &[
                ("Level", SpaceDimField::Level.key(), Some(Check::Uint)),
                ("Clones Allocated", SpaceDimField::ClonesAllocated.key(), None),
            ] },
        },
        Track::Divinity => Spec {
            title: "Divinity Upgrade", blurb: "",
            base: &["K", "l"], id_key: DivinityUpgradeField::Upgrade.key(), resolve: items::divinity_upgrade_name, done: "Staged Divinity edit",
            columns: const { &[
                ("Level", DivinityUpgradeField::Level.key(), Some(Check::Uint)),
                ("Next At", DivinityUpgradeField::NextAt.key(), None),
                ("Spread", DivinityUpgradeField::Spread.key(), None),
            ] },
        },
    }
}

/// `(list index, id, name)` for every element of the track.
fn entries(session: &EditSession, spec: &Spec) -> Vec<(usize, u32, String)> {
    let Some(Raw::List(list)) = session.root().get_path(spec.base) else { return Vec::new() };
    (0..list.len()).map(|index| {
        let id: u32 = value(session, spec, index, spec.id_key).trim().parse().unwrap_or(0);
        (index, id, (spec.resolve)(id).map_or_else(|| format!("id {id}"), str::to_string))
    }).collect()
}

fn path<'a>(spec: &Spec, index: &'a str, key: &'a str) -> Vec<&'a str> {
    let mut path: Vec<&str> = spec.base.to_vec();
    path.push(index);
    path.extend(key.split('.'));
    path
}

fn value(session: &EditSession, spec: &Spec, index: usize, key: &str) -> String {
    session.value(&path(spec, &index.to_string(), key)).unwrap_or_default()
}

pub fn build(form: &mut F, session: &EditSession, track: Track) {
    let spec = spec(track);
    if track == Track::Divinity {
        form.title("Divinity Generator");
        if session.root().get_path(&["K"]).is_none() {
            form.note("No Divinity Generator data in this save.");
            return;
        }
        for (label, key) in [("Capacity In Use", "g"), ("Worker Clones", "c"), ("Stone Storage", "n")] {
            form.row(vec![text(label).w(160.0), muted(session.value(&["K", key]).unwrap_or_else(|| "—".into())).mono().size(11.0)]);
        }
        form.note("Capacity, worker clones, and stone storage are editable in the Resources section.");
        form.sep();
        form.subtitle("Upgrades");
    } else {
        form.title(spec.title);
        if session.root().get_path(spec.base).is_none() {
            form.note(if track == Track::Might { "No Might data in this save." } else { "No data for this track in this save." });
            return;
        }
        form.note(spec.blurb);
    }
    let rows = entries(session, &spec);
    if track == Track::Might {
        // Zero the cooldown and active timer on every usable-skill Might, so
        // they are immediately re-usable.
        let mut edits = Vec::new();
        for (index, _, name) in &rows {
            if !value(session, &spec, *index, MightField::Usable.key()).trim().eq_ignore_ascii_case("true") { continue; }
            let i = index.to_string();
            edits.push((owned(&["V", &i, MightField::Cooldown.key()]), format!("{name} cooldown"), "0".to_string()));
            edits.push((owned(&["V", &i, MightField::ActiveTimer.key()]), format!("{name} active timer"), "0".to_string()));
        }
        let done = format!("Reset {} cooldown(s)", edits.len() / 2);
        form.row(vec![
            stage_button(form, "might/reset-cooldowns", "Reset all cooldowns", edits, done).w(170.0)
                .hint("Make every usable-skill Might ready (Focused Breathing, Auras, etc.)"),
            spacer(),
        ]);
    }
    if rows.is_empty() {
        form.note("No entries.");
        return;
    }
    let mut header = vec![text(spec.title).w(220.0)];
    header.extend(spec.columns.iter().map(|(label, _, _)| text(*label).w(150.0)));
    header.push(spacer());
    form.header(header);
    form.pin();
    for (index, id, name) in rows {
        let i = index.to_string();
        let mut cells = vec![text(name.as_str()).w(220.0).hint(format!("id {id}"))];
        for (label, key, check) in spec.columns {
            cells.push(match check {
                Some(check) => scalar_noted(form, session, &path(&spec, &i, key), &format!("{name} {label}"), *check, spec.done),
                None => muted(value(session, &spec, index, key)).size(11.0),
            }.w(150.0));
        }
        cells.push(spacer());
        form.row(cells);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::tests::{fire, loaded, open};
    use save_parser::registry::SectionId;

    #[test]
    fn tracks_resolve_names_and_stage_levels() {
        let mut editor = loaded();
        let session = editor.session.as_ref().unwrap();
        let might = entries(session, &spec(Track::Might));
        assert_eq!(might[0].2, "Physical HP +");
        assert!(!entries(session, &spec(Track::Creations)).is_empty());
        assert_eq!(entries(session, &spec(Track::Divinity))[0].2, "Capacity");

        open(&mut editor, SectionId::Monuments);
        fire(&mut editor, "set/D.0.b", "abc");
        assert_eq!(editor.session.as_ref().unwrap().change_count(), 0);
        fire(&mut editor, "set/D.0.b", "321");
        assert_eq!(editor.session.as_ref().unwrap().value(&["D", "0", "b"]).unwrap(), "321");
        assert_eq!(editor.note.as_ref().unwrap().0, "Staged Monuments edit");
    }

    #[test]
    fn might_reset_zeroes_usable_cooldowns() {
        let mut editor = loaded();
        open(&mut editor, SectionId::Might);
        fire(&mut editor, "might/reset-cooldowns", "");
        let session = editor.session.as_ref().unwrap();
        let spec = spec(Track::Might);
        for (index, _, _) in entries(session, &spec) {
            if value(session, &spec, index, MightField::Usable.key()).eq_ignore_ascii_case("true") {
                assert_eq!(value(session, &spec, index, MightField::Cooldown.key()), "0");
            }
        }
        assert!(editor.note.as_ref().unwrap().0.starts_with("Reset "));
    }
}
