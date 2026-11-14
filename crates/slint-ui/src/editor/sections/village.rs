//! Pet Village (`024`): the per-building scalars (levels, the Tavern's quest
//! points, the Strategy Room's stat multipliers), active Tavern quests and the
//! Museum statue levels. Worker-slot assignment and the building-state list stay
//! in the Raw Save Tree (niche); managers are shown read-only for context.
//!
//! There is no typed model for these structs, so values read straight from the
//! raw tree; edits stage by path / raw list index.
use save_parser::items;
use save_parser::raw::Raw;
use save_parser::session::EditSession;

use crate::editor::common::{owned, pick, refs, scalar_noted, stage, stage_button, Check, F};
use crate::form::{button, input, muted, spacer, text, Mark, Tone};

const DONE: &str = "Staged Pet Village edit";

/// An editable scalar: its label and its path in the save.
type Scalar = (&'static str, &'static [&'static str]);

/// A building group: a heading plus its editable scalars.
const BUILDINGS: &[(&str, &[Scalar])] = &[
    ("Tavern", &[
        ("Tavern Level", &["024", "b", "v"]),
        ("Active Level (difficulty)", &["024", "b", "m"]),
        ("Quest Points", &["024", "b", "d"]),
        ("Quests Per Day", &["024", "b", "i"]),
        ("Max Concurrent Quests", &["024", "b", "j"]),
    ]),
    ("Dojo", &[("Level", &["024", "d", "b"])]),
    ("Strategy Room", &[
        ("Level", &["024", "e", "b"]),
        ("Physical Multi %", &["024", "e", "e"]),
        ("Mystic Multi %", &["024", "e", "f"]),
        ("Battle Multi %", &["024", "e", "g"]),
    ]),
    ("Material Factory", &[("Level", &["024", "g", "a"])]),
    ("Alchemy Hut", &[("Level", &["024", "h", "a"])]),
];

/// The per-quest-type completion target (≈ 12 h). The timer counts up to it; set
/// the elapsed value at/above it to finish. The game tolerates overshoot.
const QUEST_COMPLETE_MS: u64 = 43_200_000;

struct Statue {
    /// Full raw path of the level field — list-index form (`024.f.a.<i>.a`) or,
    /// for a single statue stored as a lone struct, `024.f.a.a`.
    level_path: Vec<String>,
    name: String,
}

/// One active Tavern quest (`024.b.a.<i>` — or a lone struct for a single quest).
struct Quest {
    pet: String,
    quest: String,
    reward: String,
    /// Full raw path of the elapsed-timer field.
    timer_path: Vec<String>,
}

pub struct AddStatue { id: u32, level: String }

#[derive(Default)]
pub struct State { add: Option<AddStatue> }

/// The known museum statue ids (1..=11), for the picker.
fn known_statues() -> Vec<(u32, String)> {
    (1..=11u32).filter_map(|id| items::statue_name(id).map(|name| (id, name.to_string()))).collect()
}

/// The element paths of the list at `base`. A 1-element list re-parses as a lone
/// struct, so both forms are handled.
fn elements(session: &EditSession, base: &[&str]) -> Vec<Vec<String>> {
    match session.root().get_path(base) {
        Some(Raw::List(items)) => (0..items.len()).map(|i| { let mut path = owned(base); path.push(i.to_string()); path }).collect(),
        Some(Raw::Struct(_)) => vec![owned(base)],
        _ => Vec::new(),
    }
}

fn child(prefix: &[String], key: &str) -> Vec<String> {
    let mut path = prefix.to_vec();
    path.push(key.to_string());
    path
}

fn field(session: &EditSession, prefix: &[String], key: &str) -> String {
    session.value(&refs(&child(prefix, key))).unwrap_or_default()
}

fn statues(session: &EditSession) -> Vec<Statue> {
    // Statue fields: a = level, b = statue id.
    elements(session, &["024", "f", "a"]).into_iter().map(|prefix| {
        let id: u32 = field(session, &prefix, "b").trim().parse().unwrap_or(0);
        Statue { level_path: child(&prefix, "a"), name: items::statue_name(id).map_or_else(|| format!("Statue {id}"), str::to_string) }
    }).collect()
}

fn pet_label(id: u32) -> String { items::pet_type_name(id).map_or_else(|| format!("pet {id}"), str::to_string) }

fn quests(session: &EditSession) -> Vec<Quest> {
    // Quest fields: a = quest id, b = elapsed ms, c = pet id(s), d = reward roll.
    elements(session, &["024", "b", "a"]).into_iter().map(|prefix| {
        let quest_id: u32 = field(session, &prefix, "a").trim().parse().unwrap_or(0);
        // `c` is a pet-id list that collapses to a scalar for the usual
        // single-pet quest; handle either shape.
        let pets = child(&prefix, "c");
        let pet = match session.root().get_path(&refs(&pets)) {
            Some(Raw::List(items)) => (0..items.len())
                .filter_map(|i| session.value(&refs(&child(&pets, &i.to_string()))).and_then(|s| s.trim().parse::<u32>().ok()))
                .map(pet_label).collect::<Vec<_>>().join(", "),
            _ => field(session, &prefix, "c").trim().parse::<u32>().map(pet_label).unwrap_or_default(),
        };
        Quest {
            pet,
            quest: items::tavern_quest_name(quest_id).map_or_else(|| format!("Quest {quest_id}"), str::to_string),
            reward: field(session, &prefix, "d"),
            timer_path: child(&prefix, "b"),
        }
    }).collect()
}

/// A worker building's manager (`024.{g,h}.e`, pet type id; 999 = empty).
fn manager(session: &EditSession, building: &str) -> String {
    match session.value(&["024", building, "e"]).and_then(|s| s.trim().parse::<u32>().ok()) {
        None | Some(999) => "—".to_string(),
        Some(id) => items::pet_type_name(id).map_or_else(|| format!("type {id}"), str::to_string),
    }
}

pub fn build(form: &mut F, session: &EditSession, state: &State) {
    form.title("Pet Village");
    if session.root().get_path(&["024"]).is_none() {
        form.note("No Pet Village data in this save.");
        return;
    }
    for (name, fields) in BUILDINGS {
        form.subtitle(*name);
        for (label, path) in *fields {
            form.row(vec![text(*label).w(200.0), scalar_noted(form, session, path, label, Check::Finite, DONE).w(160.0), spacer()]);
        }
        match *name {
            "Material Factory" => { form.note(format!("Manager: {}", manager(session, "g"))); }
            "Alchemy Hut" => { form.note(format!("Manager: {}", manager(session, "h"))); }
            "Tavern" => {
                // Stored, and recomputed from quest points on quest/load events —
                // editing it sets the rank directly.
                let current: u32 = session.value(&["024", "b", "e"]).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                let ranks = (0..=9u32).map(|id| (id, items::quest_rank_name(id).unwrap_or("?").to_string())).collect();
                form.row(vec![
                    text("Quest Rank").w(200.0),
                    pick(form, "village/quest-rank", ranks, &current, |id| format!("Rank {id}"), |editor, rank| {
                        if stage(editor, &owned(&["024", "b", "e"]), "Quest Rank", &rank.to_string()) { editor.note = Some((DONE.to_string(), false)); }
                    }).w(100.0),
                    spacer(),
                ]);
            }
            _ => {}
        }
    }

    form.sep();
    let active = quests(session);
    let complete = QUEST_COMPLETE_MS.to_string();
    let mut heading = vec![text("Tavern — Active Quests").tone(Tone::Bright).size(13.0).w(200.0)];
    if !active.is_empty() {
        let edits = active.iter().map(|q| (q.timer_path.clone(), format!("{} force complete", q.quest), complete.clone())).collect();
        heading.push(stage_button(form, "village/quests/complete-all", "Complete all", edits, DONE).w(120.0).hint("Force-complete every active quest"));
    }
    heading.push(spacer());
    form.row(heading);
    if active.is_empty() {
        form.note("No active quests.");
    } else {
        form.header(vec![text("Pet").w(170.0), text("Quest").w(170.0), text("Elapsed (ms)").w(140.0), text("Roll").w(70.0), spacer()]);
        for quest in &active {
            let id = format!("village/quests/complete/{}", quest.timer_path.join("."));
            form.row(vec![
                text(quest.pet.as_str()).w(170.0),
                text(quest.quest.as_str()).w(170.0),
                scalar_noted(form, session, &refs(&quest.timer_path), &format!("{} timer", quest.quest), Check::Uint, DONE).w(140.0),
                muted(quest.reward.as_str()).size(11.0).w(70.0),
                stage_button(form, id, "Force complete", vec![(quest.timer_path.clone(), format!("{} force complete", quest.quest), complete.clone())], DONE).w(130.0)
                    .hint("Set the elapsed timer past the ~12h target so the quest finishes"),
                spacer(),
            ]);
        }
    }

    form.sep();
    form.row(vec![
        text("Museum Statues").tone(Tone::Bright).size(13.0).w(200.0),
        button(form.on("village/statues/add", |editor, _| {
            editor.village.add = Some(AddStatue { id: known_statues().first().map_or(1, |(id, _)| *id), level: "20".into() });
        }), "Add statue").w(110.0),
        spacer(),
    ]);
    if let Some(add) = &state.add { add_form(form, add); }
    let owned_statues = statues(session);
    if owned_statues.is_empty() {
        form.note("No statues.");
        return;
    }
    form.header(vec![text("Statue").w(240.0), text("Level").w(100.0), spacer()]);
    for statue in &owned_statues {
        form.row(vec![
            text(statue.name.as_str()).w(240.0),
            scalar_noted(form, session, &refs(&statue.level_path), &format!("{} level", statue.name), Check::Uint, DONE).w(100.0),
            spacer(),
        ]);
    }
}

fn add_form(form: &mut F, add: &AddStatue) {
    let valid = add.level.trim().parse::<u32>().is_ok_and(|level| level <= 20);
    form.row(vec![
        text("Add statue:").tone(Tone::Bright).w(80.0),
        pick(form, "village/statues/add/pick", known_statues(), &add.id, |id| format!("id {id}"), |editor, id| {
            if let Some(add) = editor.village.add.as_mut() { add.id = *id; }
        }).w(220.0),
        muted("Level (max 20):").w(96.0),
        input(form.on("village/statues/add/level", |editor, value| {
            if let Some(add) = editor.village.add.as_mut() { add.level = value.to_string(); }
        }), add.level.as_str()).live().w(70.0),
        button(form.on("village/statues/add/confirm", |editor, _| {
            let Some(add) = editor.village.add.take() else { return };
            let (Some(session), Ok(level)) = (editor.session.as_mut(), add.level.trim().parse::<u32>()) else { return };
            let label = format!("{} statue", items::statue_name(add.id).unwrap_or("Statue"));
            editor.note = Some(match session.add_statue(add.id, level, label) {
                Ok(()) => ("Added statue".to_string(), false),
                Err(error) => (format!("Add failed: {error}"), true),
            });
        }), "Add").primary().enabled(valid).w(70.0),
        button(form.on("village/statues/add/cancel", |editor, _| editor.village.add = None), "Cancel").w(80.0),
        muted("Statues aren't unique — you can own two of each.").size(10.0),
    ]).mark = Mark::Selected;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::tests::{fire, loaded, open};
    use save_parser::container::encode_container;
    use save_parser::raw::Field;
    use save_parser::registry::SectionId;

    fn scalar(text: &str) -> Field { Field::Value(Raw::Scalar(text.into())) }
    fn base64(raw: Raw) -> Field { Field::Value(Raw::Base64(Box::new(raw))) }

    fn quest(a: &str, b: &str, c: &str, d: &str) -> Raw {
        Raw::Struct(vec![("a".into(), scalar(a)), ("b".into(), scalar(b)), ("c".into(), scalar(c)), ("d".into(), scalar(d))])
    }

    fn tavern(quests: Raw) -> EditSession {
        let tavern = Raw::Struct(vec![("a".into(), Field::Value(quests))]);
        let village = Raw::Struct(vec![("b".into(), base64(tavern))]);
        let root = Raw::Struct(vec![("024".into(), base64(village))]);
        EditSession::load(&encode_container(&root.serialize(), "V2"), None).unwrap()
    }

    #[test]
    fn quests_resolve_in_list_and_lone_struct_form() {
        let two = tavern(Raw::List(vec![quest("10", "10432650", "37", "25"), quest("11", "10428150", "128", "67")]));
        let rows = quests(&two);
        assert_eq!((rows[0].pet.as_str(), rows[0].quest.as_str()), ("Ape", "AntQueen"));
        assert_eq!(rows[0].timer_path, ["024", "b", "a", "0", "b"]);
        assert_eq!((rows[1].pet.as_str(), rows[1].quest.as_str()), ("PackMule", "MagicTalk"));
        // One quest: a 1-element list collapses to a lone struct on load, so the
        // timer path targets the struct's field directly.
        let one = tavern(Raw::List(vec![quest("11", "10428150", "128", "67")]));
        let rows = quests(&one);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].timer_path, ["024", "b", "a", "b"]);
        assert_eq!(one.value(&refs(&rows[0].timer_path)).as_deref(), Some("10428150"));
    }

    #[test]
    fn first_statue_survives_a_reload_as_a_lone_struct() {
        let museum = Raw::Struct(vec![("b".into(), scalar("1"))]);
        let village = Raw::Struct(vec![("f".into(), base64(museum))]);
        let root = Raw::Struct(vec![("024".into(), base64(village))]);
        let mut session = EditSession::load(&encode_container(&root.serialize(), "V2"), None).unwrap();
        session.add_statue(8, 15, "Halloween 2025 statue").unwrap();
        let reloaded = EditSession::load(&session.encode(), None).unwrap();
        let rows = statues(&reloaded);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Halloween 2025");
        assert_eq!(rows[0].level_path, ["024", "f", "a", "a"]);
        assert_eq!(reloaded.value(&refs(&rows[0].level_path)).as_deref(), Some("15"));
    }

    #[test]
    fn buildings_rank_and_statues_edit_through_the_form() {
        let mut editor = loaded();
        open(&mut editor, SectionId::Village);
        fire(&mut editor, "set/024.d.b", "77");
        assert_eq!(editor.session.as_ref().unwrap().value(&["024", "d", "b"]).unwrap(), "77");
        assert_eq!(editor.note.as_ref().unwrap().0, DONE);
        fire(&mut editor, "village/quest-rank", "9");
        assert_eq!(editor.session.as_ref().unwrap().value(&["024", "b", "e"]).unwrap(), "9");
        let before = statues(editor.session.as_ref().unwrap()).len();
        fire(&mut editor, "village/statues/add", "");
        fire(&mut editor, "village/statues/add/level", "12");
        fire(&mut editor, "village/statues/add/confirm", "");
        assert_eq!(statues(editor.session.as_ref().unwrap()).len(), before + 1);
        assert_eq!(editor.session.as_ref().unwrap().added_statues().len(), 1);
    }
}
