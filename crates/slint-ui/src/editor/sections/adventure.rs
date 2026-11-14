//! Adventure: the adventure-mode inventory (`032.d`), cores (`032.G`), the
//! adventurer's per-class progression (`032.b.f`), side professions (`032.j`),
//! researches (`032.H.a`) and a read-only view of the equipped battle skills
//! (`032.b.g`).
//!
//! There is no typed model for these lists, so rows read straight from the raw
//! tree via the field descriptors; edits stage by raw list index. The battle
//! skills are read-only — their fields beyond skill id + level aren't pinned
//! down yet.
use std::collections::{HashMap, HashSet};

use save_parser::items::{self, unlock_plan, unlockable_classes};
use save_parser::labels::{AdvProfessionField, AdventureItemField, AdventurerField, ClassProgressionField, CoreField, ResearchField};
use save_parser::model;
use save_parser::raw::Raw;
use save_parser::session::EditSession;

use crate::editor::common::{matches, owned, pick, scalar_noted, stage, Check, F};
use crate::editor::Editor;
use crate::form::{button, input, link, muted, spacer, text, Mark, Tone};

const DONE: &str = "Staged adventure edit";
/// Alchemist also needs a profession level, which the unlock chain can't grant.
const ALCHEMIST: u32 = 32;

/// The "add item" / "add core" form.
pub struct Add {
    /// `false` adds an inventory item, `true` a core.
    core: bool,
    /// The chosen item id (inventory) or enemy id (core).
    id: u32,
    count: String,
    /// Core quality (cores only).
    quality: u32,
}

#[derive(Default)]
pub struct State {
    item_filter: String,
    research_filter: String,
    add: Option<Add>,
    /// The "Unlock class" form's chosen target class.
    unlock: Option<u32>,
    skills_open: bool,
}

/// Quality letter for an id (F…SSS), falling back to the number.
fn quality_label(quality: u32) -> String { items::quality_name(quality).map_or_else(|| quality.to_string(), str::to_string) }

fn qualities() -> Vec<(u32, String)> { (0..=8u32).map(|q| (q, quality_label(q))).collect() }

/// How many elements the list at `base` holds (0 when it is absent).
fn count(session: &EditSession, base: &[&str]) -> usize {
    match session.root().get_path(base) { Some(Raw::List(items)) => items.len(), _ => 0 }
}

/// The scalar at `base.<index>.<key>`, or "" when absent.
fn at(session: &EditSession, base: &[&str], index: usize, key: &str) -> String {
    let i = index.to_string();
    let mut path = base.to_vec();
    path.extend([i.as_str(), key]);
    session.value(&path).unwrap_or_default()
}

/// `(list index, id, name)` for every element of the list at `base`.
fn named(session: &EditSession, base: &[&str], id_key: &str, name: impl Fn(u32) -> String) -> Vec<(usize, u32, String)> {
    (0..count(session, base)).map(|index| {
        let id: u32 = at(session, base, index, id_key).trim().parse().unwrap_or(0);
        (index, id, name(id))
    }).collect()
}

const ITEMS: &[&str] = &["032", "d"];
const CORES: &[&str] = &["032", "G"];
const CLASSES: &[&str] = &["032", "b", "f"];
const PROFESSIONS: &[&str] = &["032", "j"];
const RESEARCH: &[&str] = &["032", "H", "a"];
const SKILLS: &[&str] = &["032", "b", "g"];

fn class_name(id: u32) -> String { items::adventure_class_name(id).map_or_else(|| format!("Class {id}"), str::to_string) }

fn class_rows(session: &EditSession) -> Vec<(usize, u32, String)> { named(session, CLASSES, ClassProgressionField::Class.key(), class_name) }

/// Researches, skipping the unused placeholder slot (id 0 has no name).
fn research_rows(session: &EditSession) -> Vec<(usize, u32, &'static str)> {
    (0..count(session, RESEARCH)).filter_map(|index| {
        let id: u32 = at(session, RESEARCH, index, ResearchField::Research.key()).trim().parse().unwrap_or(0);
        Some((index, id, model::research_name(id)?))
    }).collect()
}

/// Class id → current level, from the loaded class progression.
fn class_levels(session: &EditSession) -> HashMap<u32, u32> {
    class_rows(session).into_iter().filter_map(|(index, id, _)| Some((id, at(session, CLASSES, index, ClassProgressionField::Level.key()).trim().parse().ok()?))).collect()
}

/// A scalar cell at `base.<index>.<key>`.
fn cell(form: &F, session: &EditSession, base: &[&str], index: usize, key: &str, label: String, check: Check) -> crate::form::Cell {
    let i = index.to_string();
    let mut path = base.to_vec();
    path.extend([i.as_str(), key]);
    scalar_noted(form, session, &path, &label, check, DONE)
}

/// A sub-section's heading cells; the caller appends its buttons.
fn heading(title: &str, blurb: &str) -> Vec<crate::form::Cell> {
    let mut cells = vec![text(title).tone(Tone::Bright).size(13.0).w(150.0)];
    if !blurb.is_empty() { cells.push(muted(blurb).size(11.0).w(150.0)); }
    cells
}

pub fn build(form: &mut F, session: &EditSession, state: &State) {
    form.title("Adventure");
    if session.root().get_path(&["032"]).is_none() {
        form.note("No adventure data in this save.");
        return;
    }

    // --- Adventure inventory ---
    let item_rows = named(session, ITEMS, AdventureItemField::Item.key(), |id| items::adventure_item_name(id).map_or_else(|| format!("Item {id}"), str::to_string));
    let mut cells = heading("Adventure Inventory", "· edit item counts");
    cells.extend([
        button(form.on("adventure/add-item", |editor, _| {
            let id = items::known_adventure_items().first().map_or(0, |(id, _)| *id);
            editor.adventure.add = Some(Add { core: false, id, count: "1".into(), quality: 0 });
        }), "Add item").w(100.0),
        spacer(),
    ]);
    form.row(cells);
    form.row(vec![
        muted("Filter:").w(44.0),
        input(form.on("adventure/item-filter", |editor, value| editor.adventure.item_filter = value.to_string()), state.item_filter.as_str()).live().w(180.0),
        button(form.on("adventure/item-filter/clear", |editor, _| editor.adventure.item_filter.clear()), "× clear").w(80.0),
        muted(format!("{} items", item_rows.len())).size(11.0),
    ]);
    if let Some(add) = state.add.as_ref().filter(|add| !add.core) { add_form(form, add); }
    form.header(vec![text("Item").w(240.0), text("Count").w(160.0), spacer()]);
    let needle = state.item_filter.trim().to_lowercase();
    for (index, id, name) in item_rows.iter().filter(|(_, _, name)| matches(&needle, name)) {
        form.row(vec![
            text(name.as_str()).w(240.0).hint(format!("item id {id}")),
            cell(form, session, ITEMS, *index, AdventureItemField::Count.key(), format!("{name} count"), Check::Uint).w(160.0),
            spacer(),
        ]);
    }

    // --- Cores ---
    form.sep();
    let mut cells = heading("Cores", "· upgrade core quality");
    cells.extend([
        button(form.on("adventure/add-core", |editor, _| {
            let id = items::known_adventure_enemies().first().map_or(0, |(id, _)| *id);
            editor.adventure.add = Some(Add { core: true, id, count: "1".into(), quality: 8 });
        }), "Add core").w(100.0),
        spacer(),
    ]);
    form.row(cells);
    if let Some(add) = state.add.as_ref().filter(|add| add.core) { add_form(form, add); }
    let cores = named(session, CORES, CoreField::Enemy.key(), |id| items::adventure_enemy_name(id).map_or_else(|| format!("Enemy {id}"), str::to_string));
    if cores.is_empty() {
        form.note("No cores in this save.");
    } else {
        form.header(vec![text("Core (enemy)").w(240.0), text("Quality").w(120.0), text("Count").w(100.0), spacer()]);
        for (index, id, name) in &cores {
            let quality: u32 = at(session, CORES, *index, CoreField::Quality.key()).trim().parse().unwrap_or(0);
            let (path, label) = (owned(&["032", "G", &index.to_string(), CoreField::Quality.key()]), format!("{name} quality"));
            form.row(vec![
                text(name.as_str()).w(240.0).hint(format!("enemy id {id}")),
                pick(form, format!("adventure/core-quality/{index}"), qualities(), &quality, |q| quality_label(*q), move |editor, quality| {
                    if stage(editor, &path, &label, &quality.to_string()) { editor.note = Some((DONE.to_string(), false)); }
                }).w(120.0),
                text(at(session, CORES, *index, CoreField::Count.key())).mono().size(11.0).w(100.0),
                spacer(),
            ]);
        }
    }

    // --- Adventurer ---
    form.sep();
    form.subtitle("Adventurer");
    let active: u32 = session.value(&["032", "b", AdventurerField::Class.key()]).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    form.row(vec![text("Active class").w(120.0), muted(class_name(active))]);
    for (label, key) in [("Level", AdventurerField::Level.key()), ("Exp", AdventurerField::Exp.key())] {
        form.row(vec![text(label).w(120.0), scalar_noted(form, session, &["032", "b", key], label, Check::Uint, DONE).w(140.0), spacer()]);
    }

    // --- Professions (Crafting / Smithing / Alchemy / …) ---
    form.sep();
    form.subtitle("Professions");
    form.note("Side-profession level & exp. The next-level threshold is computed, so only level and exp are stored.");
    let professions = named(session, PROFESSIONS, AdvProfessionField::Profession.key(), |id| items::adventure_profession_name(id).map_or_else(|| format!("Profession {id}"), str::to_string));
    if professions.is_empty() {
        form.note("No professions in this save.");
    } else {
        form.header(vec![text("Profession").w(180.0), text("Level").w(110.0), text("Exp").w(160.0), spacer()]);
        for (index, id, name) in &professions {
            form.row(vec![
                text(name.as_str()).w(180.0).hint(format!("profession id {id}")),
                cell(form, session, PROFESSIONS, *index, AdvProfessionField::Level.key(), format!("{name} level"), Check::Uint).w(110.0),
                // Exp is a BigDouble: written verbatim so decimals / large magnitudes survive.
                cell(form, session, PROFESSIONS, *index, AdvProfessionField::Exp.key(), format!("{name} exp"), Check::Finite).w(160.0),
                spacer(),
            ]);
        }
    }

    // --- Class progression ---
    form.sep();
    let classes = class_rows(session);
    let present: HashSet<u32> = classes.iter().map(|(_, id, _)| *id).collect();
    let mut cells = heading("Class Progression", "");
    cells.extend([
        // Default to the first unlockable class not already present.
        button(form.on("adventure/unlock", move |editor, _| {
            editor.adventure.unlock = Some(unlockable_classes().into_iter().find(|(id, _)| !present.contains(id)).map_or(2, |(id, _)| id));
        }), "Unlock class").w(120.0).hint("Unlock a class by materializing its full prerequisite chain"),
        spacer(),
    ]);
    form.row(cells);
    form.note("Per-class level & exp; classes advance independently.");
    if let Some(target) = state.unlock { unlock_form(form, session, target); }
    if classes.is_empty() {
        form.note("No class progression in this save.");
    } else {
        form.header(vec![text("Class").w(200.0), text("Level").w(110.0), text("Exp").w(160.0), spacer()]);
        for (index, id, name) in &classes {
            form.row(vec![
                text(name.as_str()).w(200.0).hint(format!("class id {id}")),
                cell(form, session, CLASSES, *index, ClassProgressionField::Level.key(), format!("{name} level"), Check::Uint).w(110.0),
                cell(form, session, CLASSES, *index, ClassProgressionField::Exp.key(), format!("{name} exp"), Check::Uint).w(160.0),
                spacer(),
            ]);
        }
    }

    // --- Research ---
    form.sep();
    let research = research_rows(session);
    let mut cells = heading("Research", "");
    cells.extend([
        muted("Filter:").w(44.0),
        input(form.on("adventure/research-filter", |editor, value| editor.adventure.research_filter = value.to_string()), state.research_filter.as_str()).live().w(180.0),
        button(form.on("adventure/research-filter/clear", |editor, _| editor.adventure.research_filter.clear()), "× clear").w(80.0),
        spacer(),
    ]);
    form.row(cells);
    if research.is_empty() {
        form.note("No research in this save.");
    } else {
        form.header(vec![text("Research").w(220.0), text("Level").w(110.0), text("Max").w(90.0), text("Researching"), spacer()]);
        let needle = state.research_filter.trim().to_lowercase();
        for (index, id, name) in research.iter().filter(|(_, _, name)| matches(&needle, name)) {
            // The authoritative cap; the save's own per-entry max field is unreliable.
            let max = model::research_max(*id);
            let researching = at(session, RESEARCH, *index, ResearchField::InProgress.key()).trim() == "1";
            form.row(vec![
                text(*name).w(220.0).hint(format!("research id {id}")),
                cell(form, session, RESEARCH, *index, ResearchField::Level.key(), format!("{name} level"), max.map_or(Check::Uint, Check::UintMax)).w(110.0),
                muted(max.map_or_else(|| "—".to_string(), |m| m.to_string())).size(11.0).w(90.0),
                if researching { text("yes").tone(Tone::Warning).size(11.0) } else { muted("—").size(11.0) },
                spacer(),
            ]);
        }
    }

    // --- Battle skills (read-only; fields beyond id + level not yet identified) ---
    form.sep();
    let skills = named(session, SKILLS, "a", |id| items::adventure_skill_name(id).map_or_else(|| format!("Skill {id}"), str::to_string));
    form.row(vec![link(form.on("adventure/skills", |editor, _| editor.adventure.skills_open = !editor.adventure.skills_open),
        format!("{} Battle Skills ({}) — read-only", if state.skills_open { "▾" } else { "▸" }, skills.len())).tone(Tone::Bright)]);
    if state.skills_open {
        form.note("Equipped battle skills. Only the skill id and level are identified; edit the rest in the Raw Save Tree (032.b.g).");
        if skills.is_empty() {
            form.note("No battle skills.");
        } else {
            form.header(vec![text("Skill").w(220.0), text("Level"), spacer()]);
            for (index, id, name) in &skills {
                form.row(vec![text(name.as_str()).w(220.0).hint(format!("skill id {id}")), text(at(session, SKILLS, *index, "b")).mono().size(11.0), spacer()]);
            }
        }
    }
}

/// The add-item / add-core form. Pickers are alphabetized.
fn add_form(form: &mut F, add: &Add) {
    let mut choices: Vec<(u32, String)> = if add.core { items::known_adventure_enemies() } else { items::known_adventure_items() }
        .into_iter().map(|(id, name)| (id, name.to_string())).collect();
    choices.sort_by(|a, b| a.1.cmp(&b.1));
    let mut cells = vec![
        text(if add.core { "Add core:" } else { "Add item:" }).tone(Tone::Bright).w(70.0),
        muted(if add.core { "Enemy:" } else { "Item:" }).w(44.0),
        pick(form, "adventure/add/pick", choices, &add.id, |id| format!("id {id}"), |editor, id| {
            if let Some(add) = editor.adventure.add.as_mut() { add.id = *id; }
        }).w(260.0),
        muted("Count:").w(44.0),
        input(form.on("adventure/add/count", |editor, value| {
            if let Some(add) = editor.adventure.add.as_mut() { add.count = value.to_string(); }
        }), add.count.as_str()).live().w(110.0),
    ];
    if add.core {
        cells.push(muted("Quality:").w(50.0));
        cells.push(pick(form, "adventure/add/quality", qualities(), &add.quality, |q| quality_label(*q), |editor, quality| {
            if let Some(add) = editor.adventure.add.as_mut() { add.quality = *quality; }
        }).w(90.0));
    }
    cells.extend([
        button(form.on("adventure/add/confirm", |editor, _| {
            let (Some(session), Some(add)) = (editor.session.as_mut(), editor.adventure.add.take()) else { return };
            let result = if add.core {
                let label = format!("{} core", items::adventure_enemy_name(add.id).unwrap_or("Enemy"));
                session.set_core(add.id, add.count.trim(), add.quality, label).map(|created| if created { "Added core" } else { "Updated existing core" })
            } else {
                let label = format!("{} count", items::adventure_item_name(add.id).unwrap_or("Item"));
                session.set_adventure_item(add.id, add.count.trim(), label).map(|created| if created { "Added adventure item" } else { "Updated existing item" })
            };
            editor.note = Some(match result { Ok(message) => (message.to_string(), false), Err(error) => (format!("Add failed: {error}"), true) });
        }), "Add").primary().enabled(add.count.trim().parse::<u64>().is_ok()).w(70.0),
        button(form.on("adventure/add/cancel", |editor, _| editor.adventure.add = None), "Cancel").w(80.0),
        muted("If the entry already exists, this updates it instead.").size(10.0),
    ]);
    form.row(cells).mark = Mark::Selected;
}

/// Unlock `target`: create or raise every class its prerequisite chain needs.
fn unlock(editor: &mut Editor, target: u32) {
    let Some(session) = editor.session.as_mut() else { return };
    let name = items::adventure_class_name(target).unwrap_or("class");
    // The plan uses non-lowering sets, so a stale read can only under-raise.
    let Some(steps) = unlock_plan(target, &class_levels(session)) else {
        editor.note = Some(("That class has no known unlock chain.".to_string(), true));
        return;
    };
    let (mut created, mut raised) = (0, 0);
    for step in &steps {
        let label = format!("unlock {name}: {} L{}", items::adventure_class_name(step.class_id).unwrap_or("class"), step.level);
        match session.set_class_progression(step.class_id, step.level, step.tier, label) {
            Ok(true) => created += 1,
            Ok(false) => raised += 1,
            Err(error) => { editor.note = Some((format!("Unlock failed: {error}"), true)); return; }
        }
    }
    editor.note = Some((format!("Unlocked {name} ({created} created, {raised} raised)"), false));
}

/// The "Unlock class" form: pick a target class, preview the entries that will
/// be created / raised (its full prerequisite chain), and confirm.
fn unlock_form(form: &mut F, session: &EditSession, target: u32) {
    let choices = unlockable_classes().into_iter().map(|(id, name)| (id, name.to_string())).collect();
    let plan = unlock_plan(target, &class_levels(session));
    let (preview, tone) = match &plan {
        Some(steps) if steps.is_empty() => ("Already unlocked — nothing to do.".to_string(), Tone::Muted),
        Some(steps) => (format!("Will create / raise: {}", steps.iter()
            .map(|step| format!("{} → level {}", items::adventure_class_name(step.class_id).unwrap_or("?"), step.level)).collect::<Vec<_>>().join(" · ")), Tone::Normal),
        None => ("No known unlock chain for this class.".to_string(), Tone::Error),
    };
    form.row(vec![
        text("Unlock class:").tone(Tone::Bright).w(90.0),
        pick(form, "adventure/unlock/pick", choices, &target, |id| format!("id {id}"), |editor, id| editor.adventure.unlock = Some(*id)).w(220.0),
        button(form.on("adventure/unlock/confirm", |editor, _| {
            if let Some(target) = editor.adventure.unlock.take() { unlock(editor, target); }
        }), "Unlock").primary().enabled(plan.as_ref().is_some_and(|steps| !steps.is_empty())).w(80.0),
        button(form.on("adventure/unlock/cancel", |editor, _| editor.adventure.unlock = None), "Cancel").w(80.0),
        spacer(),
    ]).mark = Mark::Selected;
    form.note_tone(preview, tone);
    if target == ALCHEMIST {
        form.note_tone("Alchemist also needs the Alchemy profession ≥ 30 (set it under Professions).", Tone::Warning);
    }
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
    fn entry(fields: &[(&str, &str)]) -> Raw { Raw::Struct(fields.iter().map(|(key, value)| (key.to_string(), scalar(value))).collect()) }

    /// A `032` block (base64-wrapped like a real save) with research, class
    /// progression, battle skills and professions — two entries per list so
    /// they don't collapse to a lone struct.
    fn session() -> EditSession {
        let research = Raw::List(vec![
            // id 0 is the unused placeholder slot.
            entry(&[("a", "0"), ("b", "0"), ("f", "0")]),
            // God HP is uncapped; `f` is bogus to prove the save's max field is ignored.
            entry(&[("a", "1"), ("b", "5"), ("f", "10"), ("c", "1")]),
            // Core Quality is capped at 800 even though `f` reads 0.
            entry(&[("a", "7"), ("b", "40"), ("f", "0"), ("c", "0")]),
        ]);
        let classes = Raw::List(vec![entry(&[("a", "1"), ("b", "12"), ("c", "644")]), entry(&[("a", "4"), ("b", "35"), ("c", "11164")])]);
        let skills = Raw::List(vec![entry(&[("a", "1"), ("b", "19")]), entry(&[("a", "11"), ("b", "11")])]);
        let adventurer = Raw::Struct(vec![("e".into(), scalar("20")), ("f".into(), Field::Value(classes)), ("g".into(), Field::Value(skills))]);
        let professions = Raw::List(vec![entry(&[("a", "1"), ("b", "20"), ("c", "11.89")]), entry(&[("a", "5"), ("b", "14"), ("c", "934.54")])]);
        let block = Raw::Struct(vec![
            ("H".into(), base64(Raw::Struct(vec![("a".into(), Field::Value(research))]))),
            ("b".into(), base64(adventurer)),
            ("j".into(), Field::Value(professions)),
        ]);
        let root = Raw::Struct(vec![("032".into(), base64(block))]);
        EditSession::load(&encode_container(&root.serialize(), "V2"), None).unwrap()
    }

    fn editor() -> Editor {
        let mut editor = Editor::default();
        editor.set_session(session());
        open(&mut editor, SectionId::Adventure);
        editor
    }

    #[test]
    fn rows_resolve_names_and_skip_the_research_placeholder() {
        let session = session();
        let research = research_rows(&session);
        assert_eq!(research.iter().map(|r| r.2).collect::<Vec<_>>(), ["God HP", "Core Quality"]);
        assert_eq!((model::research_max(research[0].1), model::research_max(research[1].1)), (None, Some(800)));
        let classes = class_rows(&session);
        assert_eq!((classes[0].2.as_str(), classes[1].2.as_str()), ("Newbie", "Student"));
        assert_eq!(class_levels(&session), HashMap::from([(1, 12), (4, 35)]));
        let skills = named(&session, SKILLS, "a", |id| items::adventure_skill_name(id).unwrap_or("?").to_string());
        assert_eq!((skills[0].2.as_str(), skills[1].2.as_str()), ("Basic Attack", "Magic Arrow"));
        assert_eq!(at(&session, PROFESSIONS, 1, "c"), "934.54");
        assert_eq!((quality_label(8), quality_label(0), quality_label(99)), ("SSS".into(), "F".into(), "99".into()));
    }

    #[test]
    fn research_is_capped_and_exp_keeps_decimals() {
        let mut editor = editor();
        fire(&mut editor, "set/032.H.a.2.b", "801");
        assert_eq!(editor.session.as_ref().unwrap().change_count(), 0, "above the cap is rejected");
        fire(&mut editor, "set/032.H.a.2.b", "800");
        fire(&mut editor, "set/032.j.1.c", "1234.5");
        let session = editor.session.as_ref().unwrap();
        assert_eq!(session.value(&["032", "H", "a", "2", "b"]).unwrap(), "800");
        assert_eq!(session.value(&["032", "j", "1", "c"]).unwrap(), "1234.5");
        assert_eq!(editor.note.as_ref().unwrap().0, DONE);
    }

    #[test]
    fn unlocking_a_class_materializes_its_chain() {
        let mut editor = editor();
        fire(&mut editor, "adventure/unlock", "");
        // Rogue (20) needs Adventurer 35 → Thief 55 & Archer 55; Newbie 12 ≥ 10 already.
        let rogue = unlockable_classes().iter().position(|(id, _)| *id == 20).unwrap();
        fire(&mut editor, "adventure/unlock/pick", &rogue.to_string());
        fire(&mut editor, "adventure/unlock/confirm", "");
        let levels = class_levels(editor.session.as_ref().unwrap());
        assert_eq!((levels[&2], levels[&5], levels[&6], levels[&20], levels[&1]), (35, 55, 55, 1, 12));
        assert_eq!(editor.note.as_ref().unwrap().0, "Unlocked Rogue (4 created, 0 raised)");
        assert!(editor.adventure.unlock.is_none());
    }

    #[test]
    fn items_and_cores_add_through_the_form_on_a_real_save() {
        let mut editor = loaded();
        open(&mut editor, SectionId::Adventure);
        fire(&mut editor, "adventure/add-core", "");
        fire(&mut editor, "adventure/add/count", "3");
        fire(&mut editor, "adventure/add/confirm", "");
        assert!(matches!(editor.note.as_ref().unwrap().0.as_str(), "Added core" | "Updated existing core"));
        fire(&mut editor, "adventure/add-item", "");
        fire(&mut editor, "adventure/add/confirm", "");
        assert!(matches!(editor.note.as_ref().unwrap().0.as_str(), "Added adventure item" | "Updated existing item"));
        fire(&mut editor, "adventure/skills", "");
        assert!(editor.adventure.skills_open);
        let all = crate::editor::build(&editor).rows.len();
        fire(&mut editor, "adventure/item-filter", "zzz-nothing");
        fire(&mut editor, "adventure/research-filter", "zzz-nothing");
        assert!(crate::editor::build(&editor).rows.len() < all);
    }
}
