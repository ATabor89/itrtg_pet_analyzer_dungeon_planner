//! Challenges: view and edit challenge completions (`x.242`) and add new
//! completion entries. Challenge Points are recomputed by the game from this
//! list, so editing completions is how ChP is granted or adjusted.
use std::collections::HashSet;

use save_parser::items;
use save_parser::raw::Raw;
use save_parser::session::EditSession;

use crate::editor::common::{matches, owned, pick, scalar, stage, Check, F};
use crate::form::{button, input, muted, spacer, text, Mark, Tone};

/// Difficulty ids (challenge field `c`).
const DIFFICULTIES: [(&str, u32); 4] = [("Normal", 0), ("Hard", 1), ("Root", 2), ("Mixed", 3)];
/// Highest challenge enum id; 49 is `UNUSED`.
const MAX_CHALLENGE_ID: u32 = 76;
const UNUSED_CHALLENGE_ID: u32 = 49;
const SCORE_HINT: &str = "Day challenge: ChP comes from your high score (a Statistics-block stat), not this completion count. Edit the score in the Raw Save Tree / Statistics (the labeled “(ChP basis)” field).";

fn difficulties() -> Vec<(u32, String)> { DIFFICULTIES.iter().map(|(label, id)| (*id, label.to_string())).collect() }

fn challenge_label(id: u32) -> String {
    items::challenge_name(id).map_or_else(|| format!("Challenge {id}"), str::to_string)
}

pub struct Add { challenge: u32, completions: String, difficulty: u32 }

#[derive(Default)]
pub struct State { filter: String, add: Option<Add> }

/// `(list index, challenge id)` for every entry.
fn entries(session: &EditSession) -> Vec<(usize, u32)> {
    let Some(Raw::List(items)) = session.root().get_path(&["x", "242"]) else { return Vec::new() };
    (0..items.len()).map(|index| (index, session.value(&["x", "242", &index.to_string(), "a"]).and_then(|s| s.trim().parse().ok()).unwrap_or(0))).collect()
}

/// First challenge id (skipping `UNUSED`) not already present, for the Add form's default.
fn first_unused(session: &EditSession) -> u32 {
    let present: HashSet<u32> = entries(session).into_iter().map(|(_, id)| id).collect();
    (1..=MAX_CHALLENGE_ID).find(|id| *id != UNUSED_CHALLENGE_ID && !present.contains(id)).unwrap_or(1)
}

pub fn build(form: &mut F, session: &EditSession, state: &State) {
    let default = first_unused(session);
    form.title_with("Challenges", vec![
        button(form.on("challenges/add", move |editor, _| {
            editor.challenges.add = Some(Add { challenge: default, completions: "1".into(), difficulty: 0 });
        }), "Add challenge").w(130.0),
        spacer(),
    ]);
    form.note("Challenge Points are recomputed from these completions (normal challenges: completions × a fixed value; Day challenges score-based, so editing their count won't change ChP).");
    let rows = entries(session);
    form.row(vec![
        muted("Filter name:").w(76.0),
        input(form.on("challenges/filter", |editor, value| editor.challenges.filter = value.to_string()), state.filter.as_str()).live().w(180.0),
        button(form.on("challenges/clear", |editor, _| editor.challenges.filter.clear()), "× clear").w(80.0),
        muted(format!("{} challenges", rows.len())).size(11.0),
    ]);
    if let Some(add) = &state.add { add_form(form, add); }
    if rows.is_empty() {
        form.note("No challenge completions in this save. Use “Add challenge”.");
        return;
    }
    form.header(vec![text("Challenge").w(260.0), text("Completions").w(120.0), text("Difficulty").w(150.0), spacer()]);
    form.pin();
    let needle = state.filter.trim().to_lowercase();
    for (index, id) in rows {
        let name = challenge_label(id);
        if !matches(&needle, &name) { continue; }
        let i = index.to_string();
        let difficulty: u32 = session.value(&["x", "242", &i, "c"]).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
        let (path, label) = (owned(&["x", "242", &i, "c"]), format!("{name} difficulty"));
        let score_based = items::challenge_is_score_based(id);
        form.row(vec![
            text(if score_based { format!("{name}  (score-based)") } else { name.clone() }).w(260.0).hint(if score_based { SCORE_HINT } else { "" }),
            scalar(form, session, &["x", "242", &i, "b"], &format!("{name} completions"), Check::Uint).w(120.0),
            pick(form, format!("challenges/difficulty/{index}"), difficulties(), &difficulty, |d| format!("Difficulty {d}"),
                move |editor, choice| { stage(editor, &path, &label, &choice.to_string()); }).w(150.0),
            muted(format!("id {id}")).mono().size(10.0),
        ]);
    }
}

fn add_form(form: &mut F, add: &Add) {
    // Sorted by name so the ~75-entry list is easy to scan.
    let mut choices: Vec<(u32, String)> = (1..=MAX_CHALLENGE_ID).filter(|id| *id != UNUSED_CHALLENGE_ID).map(|id| (id, challenge_label(id))).collect();
    choices.sort_by(|a, b| a.1.cmp(&b.1));
    let edit = |apply: fn(&mut Add, &u32)| move |editor: &mut crate::editor::Editor, value: &u32| {
        if let Some(add) = editor.challenges.add.as_mut() { apply(add, value); }
    };
    let valid = add.completions.trim().parse::<u64>().is_ok();
    form.row(vec![
        text("Add challenge:").tone(Tone::Bright).w(100.0),
        pick(form, "challenges/add/pick", choices, &add.challenge, |id| challenge_label(*id), edit(|add, id| add.challenge = *id)).w(260.0),
        muted("Completions:").w(80.0),
        input(form.on("challenges/add/completions", |editor, value| {
            if let Some(add) = editor.challenges.add.as_mut() { add.completions = value.to_string(); }
        }), add.completions.as_str()).live().w(100.0),
        muted("Difficulty:").w(64.0),
        pick(form, "challenges/add/difficulty", difficulties(), &add.difficulty, |d| format!("Difficulty {d}"), edit(|add, d| add.difficulty = *d)).w(120.0),
        button(form.on("challenges/add/confirm", |editor, _| {
            let Some(add) = editor.challenges.add.take() else { return };
            let Some(session) = editor.session.as_mut() else { return };
            let name = challenge_label(add.challenge);
            editor.note = Some(match session.set_challenge(add.challenge, add.completions.trim(), add.difficulty, format!("{name} completions")) {
                Ok(true) => (format!("Added {name}"), false),
                Ok(false) => (format!("Updated {name} (already present)"), false),
                Err(error) => (format!("Add failed: {error}"), true),
            });
        }), "Add").primary().enabled(valid).w(70.0),
        button(form.on("challenges/add/cancel", |editor, _| editor.challenges.add = None), "Cancel").w(80.0),
        spacer(),
    ]).mark = Mark::Selected;
    form.note("If the challenge already has an entry, this updates its completion count.");
    if items::challenge_is_score_based(add.challenge) {
        form.note_tone("⚠ This is a Day challenge: its ChP is based on your highest SCORE, not the completion count. Setting completions here won't change its ChP — edit the score in the Raw Save Tree / Statistics (the labeled “(ChP basis)” field).", Tone::Warning);
    }
}
