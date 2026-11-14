//! Fishing (root `025`): level/exp, owned rods, bait counts and fish caught.
//! Fish Power (`025.a`) lives in Resources. The rod/bait/fish lists are fixed
//! type-lists, so they are always real multi-element lists.
use save_parser::items;
use save_parser::labels::{FishingBaitField, FishingField, FishingFishField, FishingRodField};
use save_parser::raw::Raw;
use save_parser::session::EditSession;

use crate::editor::common::{owned, scalar_noted, stage_all, Check, F};
use crate::form::{check, spacer, text};

const DONE: &str = "Staged fishing edit";

fn mat_name(id: u32) -> String {
    items::material_name(id).map_or_else(|| format!("id {id}"), str::to_string)
}

fn list_len(session: &EditSession, key: &str) -> usize {
    match session.root().get_path(&["025", key]) { Some(Raw::List(items)) => items.len(), _ => 0 }
}

pub fn build(form: &mut F, session: &EditSession) {
    form.title("Fishing");
    if session.root().get_path(&["025"]).is_none() {
        form.note("No fishing data in this save.");
        return;
    }
    form.pin();
    let id_at = |list: &str, index: &str, key: &str| -> u32 {
        session.value(&["025", list, index, key]).and_then(|s| s.trim().parse().ok()).unwrap_or(0)
    };
    for (key, label) in [(FishingField::Level.key(), "Fishing Level"), (FishingField::Exp.key(), "Fishing Exp")] {
        form.row(vec![text(label).w(160.0), scalar_noted(form, session, &["025", key], label, Check::Finite, DONE).w(200.0), spacer()]);
    }
    // Selected rod/bait/pond, resolved for context.
    let selected = |key: &str| session.value(&["025", key]).and_then(|s| s.trim().parse::<u32>().ok());
    let pond = selected(FishingField::CurrentPond.key()).and_then(items::pond_name).map_or_else(|| "—".to_string(), str::to_string);
    let rod = selected(FishingField::SelectedRod.key()).map_or_else(|| "—".to_string(), mat_name);
    let bait = selected(FishingField::SelectedBait.key()).map_or_else(|| "—".to_string(), mat_name);
    form.note(format!("Pond: {pond}  ·  Rod: {rod}  ·  Bait: {bait}"));

    form.sep();
    form.subtitle("Rods");
    form.header(vec![text("Rod").w(240.0), text("Owned")]);
    for index in 0..list_len(session, "g") {
        let i = index.to_string();
        let name = mat_name(id_at("g", &i, FishingRodField::Rod.key()));
        let has = session.value(&["025", "g", &i, FishingRodField::Owned.key()]).is_some_and(|s| s.trim() != "0" && !s.trim().is_empty());
        let (path, label) = (owned(&["025", "g", &i, FishingRodField::Owned.key()]), format!("{name} owned"));
        let id = form.on(format!("fishing/rod/{index}"), move |editor, value| {
            stage_all(editor, &[(path.clone(), label.clone(), if value == "1" { "1" } else { "0" }.to_string())], DONE);
        });
        form.row(vec![text(name).w(240.0), check(id, "", has)]);
    }

    // Bait counts and fish caught share one "name + count" shape.
    let mut counts = |title: &str, list: &'static str, id_key: &str, count_key: &str| {
        form.sep();
        form.subtitle(title);
        form.header(vec![text("Item").w(240.0), text("Count")]);
        for index in 0..list_len(session, list) {
            let i = index.to_string();
            let name = mat_name(id_at(list, &i, id_key));
            form.row(vec![text(name.as_str()).w(240.0), scalar_noted(form, session, &["025", list, &i, count_key], &format!("{name} count"), Check::Uint, DONE).w(160.0), spacer()]);
        }
    };
    counts("Bait", "h", FishingBaitField::Bait.key(), FishingBaitField::Count.key());
    counts("Fish Caught", "i", FishingFishField::Fish.key(), FishingFishField::Caught.key());
}
