//! Save Editor screen.
//!
//! A raw ITRTG save is loaded into an [`EditSession`] (the lossless tree — the
//! single mutable source of truth) and shown through structured sections and a
//! raw tree navigator. Edits are staged as a pending list and written to a *new*
//! file. The session holds the player's real identity fields in memory, so the
//! editor is deliberately **not** part of the persisted app session.
//!
//! Like the Growth Chamber, the screen is described as form rows: [`build`]
//! renders the current state, and each interactive cell's action mutates the
//! [`Editor`]. Work that needs the platform (file dialogs, the clipboard, the
//! planner) is queued as an [`Effect`] for the bindings to carry out.
use std::collections::HashMap;

use itrtg_models::{ExportPet, MainStats};
use save_parser::registry::{FieldRegistry, SectionId};
use save_parser::session::EditSession;

use crate::form::{button, check, muted, spacer, text, Act, Form, Row, Tone};

pub mod bulk;
pub mod common;
mod sections;

use sections::*;

/// Something the editor wants done outside itself.
pub enum Effect {
    /// Open a file picker and load the chosen save.
    PickSave,
    /// Offer `contents` to the user as a file called `name`.
    SaveFile { name: String, contents: String },
    /// Put text on the clipboard.
    Copy(String),
    /// Fill the planner (roster + account stats) from the loaded save.
    Populate,
}

#[derive(Default)]
pub struct Editor {
    pub session: Option<EditSession>,
    registry: FieldRegistry,
    pub section: SectionId,
    /// Load/save outcome, shown under the toolbar.
    pub status: Option<(String, bool)>,
    /// Outcome of the last edit in the active section.
    pub note: Option<(String, bool)>,
    pub effects: Vec<Effect>,
    /// Populate the planner whenever a save is loaded (persisted by the app).
    pub populate_on_load: bool,
    /// Whether the pending list is expanded; `None` = decide by its length.
    pending_open: Option<bool>,
    /// Bumped whenever the view should scroll to its jump row.
    pub jump_token: i32,
    pub(crate) partners: partners::State,
    pub(crate) challenges: challenges::State,
    pub(crate) gems: gems::State,
}

/// The editor rendered as form rows.
#[derive(Default)]
pub struct EditorForms {
    pub acts: HashMap<String, Act<Editor>>,
    pub header: Vec<Row>,
    pub pending: Vec<Row>,
    pub top: Vec<Row>,
    pub rows: Vec<Row>,
    pub jump: Option<usize>,
}

impl Editor {
    /// Install a freshly loaded session. Returns whether the planner should be
    /// populated from it (the "on load" preference).
    pub fn set_session(&mut self, session: EditSession) -> bool {
        let populate = self.populate_on_load;
        self.status = Some((match &session.source_name {
            Some(name) if !name.is_empty() => format!("Loaded {name}"),
            _ => "Save loaded".to_string(),
        }, false));
        let (registry, section) = (std::mem::take(&mut self.registry), self.section);
        // Everything typed against the previous save is dropped with it.
        *self = Editor { session: Some(session), registry, section, populate_on_load: populate, status: self.status.take(), jump_token: self.jump_token, ..Default::default() };
        populate
    }

    /// Planner-ready data from the loaded save, or `None` when no save is loaded
    /// or it did not fully parse: roster, account stats and owned Moai levels.
    pub fn extract_for_planner(&self) -> Option<(Vec<ExportPet>, MainStats, Vec<u32>)> {
        let save = self.session.as_ref()?.derived()?;
        Some((save_parser::save_to_export_pets(save), save_parser::save_to_main_stats(save), save_parser::moai_levels(save)))
    }

    /// Call after every action: refresh the typed view the sections read.
    pub fn settle(&mut self) {
        if let Some(session) = self.session.as_mut() { session.rederive_if_needed(); }
    }

    pub fn select_section(&mut self, index: usize) {
        if let Some(section) = SectionId::ALL.get(index) && *section != self.section {
            self.section = *section;
            self.note = None;
        }
    }

    /// Encode the (optionally redacted) save, round-trip validate it, and hand
    /// it over to be written.
    fn export(&mut self, redacted: bool) {
        let Some(session) = self.session.as_ref() else { return };
        let encoded = if redacted { session.encode_redacted().map(|(encoded, _)| encoded) } else { Ok(session.encode()) };
        let checked = encoded.map_err(|e| format!("Encode failed: {e}")).and_then(|encoded| {
            let valid = if redacted { session.validate_encoded_redacted(&encoded) } else { session.validate_encoded(&encoded) };
            valid.map(|()| encoded).map_err(|e| format!("Validation failed — not written: {e}"))
        });
        match checked {
            Ok(contents) => self.effects.push(Effect::SaveFile { name: edited_filename(session.source_name.as_deref(), redacted), contents }),
            Err(error) => self.status = Some((error, true)),
        }
    }
}

/// The default export filename: `edited_` (or `edited_redacted_`) + the loaded
/// save's name, so an export stays tied to the save it came from. Falls back to
/// a generic name when the source is unknown, and never stacks prefixes.
pub fn edited_filename(source: Option<&str>, redacted: bool) -> String {
    let prefix = if redacted { "edited_redacted_" } else { "edited_" };
    match source {
        Some(name) if !name.is_empty() => {
            let stem = name.strip_prefix("edited_redacted_").or_else(|| name.strip_prefix("edited_")).unwrap_or(name);
            format!("{prefix}{stem}")
        }
        _ => format!("{prefix}save.txt"),
    }
}

pub fn section_titles() -> Vec<String> { SectionId::ALL.iter().map(|s| s.title().to_string()).collect() }

pub fn section_index(section: SectionId) -> usize { SectionId::ALL.iter().position(|s| *s == section).unwrap_or(0) }

/// Truncate a long value for the pending list.
fn trunc(value: &str) -> String {
    if value.chars().count() > 24 { format!("{}…", value.chars().take(23).collect::<String>()) } else { value.to_string() }
}

pub fn build(editor: &Editor) -> EditorForms {
    let mut form: Form<Editor> = Form::default();
    let mut out = EditorForms::default();
    header(&mut form, editor);
    out.header = form.take_rows();
    let Some(session) = editor.session.as_ref() else {
        out.acts = form.into_acts();
        return out;
    };
    pending(&mut form, editor, session);
    out.pending = form.take_rows();

    match editor.section {
        SectionId::Resources => resources::build(&mut form, session, &editor.registry),
        SectionId::Partners => partners::build(&mut form, session, &editor.partners),
        SectionId::AfkyGod => afky_god::build(&mut form, session),
        SectionId::Challenges => challenges::build(&mut form, session, &editor.challenges),
        SectionId::Campaigns => campaigns::build(&mut form, session),
        SectionId::Dungeons => dungeons::build(&mut form, session),
        SectionId::Planet => planet::build(&mut form, session),
        SectionId::Gems => gems::build(&mut form, session, &editor.gems),
        SectionId::Crystal => crystal::build(&mut form, session),
        SectionId::Fishing => fishing::build(&mut form, session),
        SectionId::Daily => daily::build(&mut form, session),
        other => { form.title(other.title()); form.note("This section is not available yet."); },
    }
    // The section's edit outcome sits at the bottom of its pinned rows.
    let mut rows = form.take_rows();
    let pinned = form.pinned.min(rows.len());
    out.rows = rows.split_off(pinned);
    out.top = rows;
    if let Some((message, error)) = &editor.note {
        let mut note: Form<Editor> = Form::default();
        note.note_tone(message.clone(), if *error { Tone::Error } else { Tone::Success });
        out.top.extend(note.take_rows());
    }
    out.jump = form.jump;
    out.acts = form.into_acts();
    out
}

fn header(form: &mut Form<Editor>, editor: &Editor) {
    let mut cells = vec![
        button(form.on("load", |editor, _| editor.effects.push(Effect::PickSave)), "Load save…").w(110.0),
    ];
    let Some(session) = editor.session.as_ref() else {
        cells.push(spacer());
        form.row(cells);
        form.status(&editor.status);
        form.subtitle("No save loaded");
        form.note("Drop an ITRTG save file onto the window, copy one and press Import clipboard, or use “Load save…”.");
        form.note("Edits are staged as a pending list and written to a new file — your loaded save is never overwritten unless you pick it.");
        return;
    };
    cells.extend([
        button(form.on("save", |editor, _| editor.export(false)), "Save As… (full)").w(140.0)
            .hint("Write the full edited save — including your real identity — to a new file. This is the one that loads back into the game."),
        button(form.on("save-redacted", |editor, _| editor.export(true)), "Save redacted copy…").w(170.0)
            .hint("Write a copy with account identifiers scrubbed — for sharing."),
        spacer().w(12.0),
        button(form.on("populate", |editor, _| editor.effects.push(Effect::Populate)), "Populate planner").w(150.0)
            .hint("Use this save to fill the Pet Analyzer and Growth Chamber — the pet roster plus account-level inputs. Overwrites the current imported values."),
        check(form.on("populate-on-load", |editor, value| editor.populate_on_load = value == "1"), "on load", editor.populate_on_load).w(100.0)
            .hint("Automatically populate the planner whenever a save is loaded."),
        spacer(),
    ]);
    form.row(cells);

    let pets = session.derived().map(|d| d.pets.len().to_string()).unwrap_or_else(|| "—".into());
    let god_power = session.value(&["p", "j"]).unwrap_or_else(|| "—".into());
    let stones = session.value(&["X", "y"]).unwrap_or_else(|| "—".into());
    let count = session.change_count();
    let open = editor.pending_open.unwrap_or(count > 0 && count <= 12);
    let mut cells = Vec::new();
    if let Some(name) = session.source_name.as_deref().filter(|n| !n.is_empty()) {
        cells.push(text(name).tone(Tone::Bright).w((name.chars().count() as f32 * 7.5 + 12.0).min(360.0)));
    }
    cells.push(muted(session.format_label()).size(11.0).w(110.0));
    cells.push(muted(format!("{pets} pets · GP {god_power} · stones {stones}"))
        .hint(if session.derived().is_some() { "" } else { "Typed view unavailable for this save; raw editing still works." }));
    cells.push(text(format!("Pending changes ({count})")).tone(if count > 0 { Tone::Warning } else { Tone::Muted }).w(150.0));
    if count > 0 {
        cells.push(button(form.on("pending-toggle", move |editor, _| editor.pending_open = Some(!open)), if open { "Hide" } else { "Show" }).w(70.0));
    }
    form.row(cells);
    form.note_tone("⚠ A loaded save holds your real account identifiers in memory; they are never written to the app's saved settings. “Save As” keeps them (so it loads in-game); use “Save redacted copy” to share.", Tone::Warning);
    form.status(&editor.status);
}

/// The staged changes with their undo buttons (empty while collapsed).
fn pending(form: &mut Form<Editor>, editor: &Editor, session: &EditSession) {
    let count = session.change_count();
    if !editor.pending_open.unwrap_or(count > 0 && count <= 12) || count == 0 { return; }
    for (index, edit) in session.pending().iter().enumerate() {
        // Resolve the staged path (which may carry a `field=value` selector) to
        // the all-index form the tree navigator uses.
        let path: Vec<&str> = edit.path.iter().map(String::as_str).collect();
        let resolved = session.root().resolve_index_path(&path).map(|p| p.join("."));
        let target = resolved.clone().unwrap_or_default();
        form.row(vec![
            text(edit.label.as_str()).tone(Tone::Bright).w(220.0),
            muted(edit.path.join(".")).mono().size(11.0),
            text(format!("{} → {}", trunc(&edit.original), trunc(&edit.new))).mono().size(11.0).w(330.0),
            button(form.on(format!("pending/tree/{index}"), move |editor, _| {
                editor.section = SectionId::RawTree;
                editor.note = None;
                let _ = &target;
            }), "↪ tree").w(70.0).enabled(resolved.is_some()).hint("Reveal this field in the Raw Save Tree"),
            button(form.on(format!("pending/undo/{index}"), move |editor, _| {
                if let Some(session) = editor.session.as_mut() { let _ = session.undo(index); }
            }), "undo").w(60.0),
        ]);
    }
    // The non-scalar changes: `(heading, tone, labels, undo)`.
    type Undo = fn(&mut EditSession, usize);
    let groups: [(&str, Tone, &str, Vec<String>, Undo); 10] = [
        ("Created equipment", Tone::Success, "+ ", session.added().iter().map(|a| format!("{}   #{}", a.label, a.instance_id)).collect(), |s, i| s.undo_added(i)),
        ("Added items", Tone::Success, "+ ", session.added_materials().iter().map(|m| format!("{}   id {}", m.label, m.item_id)).collect(), |s, i| s.undo_added_material(i)),
        ("Added gems", Tone::Success, "+ ", session.added_gems().iter().map(|g| g.label.clone()).collect(), |s, i| s.undo_added_gem(i)),
        ("Added challenges", Tone::Success, "+ ", session.added_challenges().iter().map(|c| c.label.clone()).collect(), |s, i| s.undo_added_challenge(i)),
        ("Added adventure items", Tone::Success, "+ ", session.added_adventure_items().iter().map(|a| a.label.clone()).collect(), |s, i| s.undo_added_adventure_item(i)),
        ("Added cores", Tone::Success, "+ ", session.added_cores().iter().map(|c| c.label.clone()).collect(), |s, i| s.undo_added_core(i)),
        ("Added statues", Tone::Success, "+ ", session.added_statues().iter().map(|s| s.label.clone()).collect(), |s, i| s.undo_added_statue(i)),
        ("Unlocked classes", Tone::Success, "+ ", session.added_class_progression().iter().map(|c| c.label.clone()).collect(), |s, i| s.undo_added_class_progression(i)),
        ("Deleted", Tone::Error, "− ", session.removed().iter().map(|r| r.label.clone()).collect(), |s, i| s.undo_removed(i)),
        ("Pasted subtrees", Tone::Accent, "⎘ ", session.tree_edits().iter().map(|t| t.path.join(".")).collect(), |s, i| s.undo_tree_edit(i)),
    ];
    for (group, (heading, tone, mark, labels, undo)) in groups.into_iter().enumerate() {
        if labels.is_empty() { continue; }
        form.header(vec![text(heading)]);
        for (index, label) in labels.into_iter().enumerate() {
            form.row(vec![
                text(format!("{mark}{label}")).tone(tone),
                button(form.on(format!("pending/undo-{group}/{index}"), move |editor, _| {
                    if let Some(session) = editor.session.as_mut() { undo(session, index); }
                }), "undo").w(60.0),
            ]);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn reference_save() -> String {
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../reference/save_file_deserialization/ManualSave_2026-06-09.txt")).unwrap()
    }

    pub fn loaded() -> Editor {
        let mut editor = Editor::default();
        editor.set_session(EditSession::load(&reference_save(), Some("ManualSave_2026-06-09.txt".into())).unwrap());
        editor
    }

    /// Run the action registered under `id`, as a click/commit would.
    pub fn fire(editor: &mut Editor, id: &str, value: &str) {
        let act = build(editor).acts.get(id).unwrap_or_else(|| panic!("no action {id}")).clone();
        act(editor, value);
        editor.settle();
    }

    pub fn open(editor: &mut Editor, section: SectionId) { editor.select_section(section_index(section)); }

    #[test]
    fn edited_filename_prepends_once_and_falls_back() {
        assert_eq!(edited_filename(Some("ManualSave_2026-06-09.txt"), false), "edited_ManualSave_2026-06-09.txt");
        assert_eq!(edited_filename(Some("ManualSave_2026-06-09.txt"), true), "edited_redacted_ManualSave_2026-06-09.txt");
        assert_eq!(edited_filename(None, false), "edited_save.txt");
        assert_eq!(edited_filename(Some(""), true), "edited_redacted_save.txt");
        assert_eq!(edited_filename(Some("edited_ManualSave.txt"), false), "edited_ManualSave.txt");
        assert_eq!(edited_filename(Some("edited_redacted_ManualSave.txt"), true), "edited_redacted_ManualSave.txt");
        assert_eq!(edited_filename(Some("edited_ManualSave.txt"), true), "edited_redacted_ManualSave.txt");
    }

    #[test]
    fn every_section_renders_for_the_reference_save() {
        let mut editor = loaded();
        for (index, section) in SectionId::ALL.iter().enumerate() {
            editor.select_section(index);
            let forms = build(&editor);
            let first = forms.top.first().or(forms.rows.first()).unwrap_or_else(|| panic!("{section:?} rendered nothing"));
            assert!(!first.cells[0].text.is_empty(), "{section:?}");
        }
        assert_eq!(section_titles().len(), SectionId::ALL.len());
    }

    #[test]
    fn scalar_edits_stage_coalesce_validate_and_undo() {
        let mut editor = loaded();
        let before = editor.session.as_ref().unwrap().value(&["X", "y"]).unwrap();
        assert!(build(&editor).pending.is_empty());
        fire(&mut editor, "set/X.y", "not a number");
        assert_eq!(editor.session.as_ref().unwrap().change_count(), 0, "invalid input is rejected");
        fire(&mut editor, "set/X.y", "123456");
        fire(&mut editor, "set/X.y", "654321");
        let session = editor.session.as_ref().unwrap();
        assert_eq!(session.change_count(), 1);
        assert_eq!(session.value(&["X", "y"]).unwrap(), "654321");
        let forms = build(&editor);
        assert_eq!(forms.pending.len(), 1);
        assert!(forms.pending[0].cells[2].text.ends_with("→ 654321"));
        assert!(forms.header.iter().any(|r| r.cells.iter().any(|c| c.text == "Pending changes (1)")));
        fire(&mut editor, "pending/undo/0", "");
        assert_eq!(editor.session.as_ref().unwrap().value(&["X", "y"]).unwrap(), before);
        assert_eq!(editor.session.as_ref().unwrap().change_count(), 0);
    }

    #[test]
    fn export_validates_and_names_the_file_after_its_source() {
        let mut editor = loaded();
        fire(&mut editor, "set/X.y", "777");
        fire(&mut editor, "save", "");
        let Some(Effect::SaveFile { name, contents }) = editor.effects.pop() else { panic!("no file offered") };
        assert_eq!(name, "edited_ManualSave_2026-06-09.txt");
        let reloaded = EditSession::load(&contents, None).unwrap();
        assert_eq!(reloaded.value(&["X", "y"]).unwrap(), "777");
        fire(&mut editor, "save-redacted", "");
        let Some(Effect::SaveFile { name, .. }) = editor.effects.pop() else { panic!("no redacted file offered") };
        assert_eq!(name, "edited_redacted_ManualSave_2026-06-09.txt");
    }

    #[test]
    fn loading_resets_per_save_state_but_keeps_the_preference() {
        let mut editor = loaded();
        fire(&mut editor, "populate-on-load", "1");
        fire(&mut editor, "set/X.y", "5");
        open(&mut editor, SectionId::Daily);
        assert!(editor.set_session(EditSession::load(&reference_save(), None).unwrap()));
        assert_eq!(editor.session.as_ref().unwrap().change_count(), 0);
        assert_eq!(editor.section, SectionId::Daily);
        assert_eq!(editor.status.as_ref().unwrap().0, "Save loaded");
        assert!(editor.extract_for_planner().is_some_and(|(pets, _, _)| !pets.is_empty()));
        fire(&mut editor, "populate", "");
        assert!(matches!(editor.effects.pop(), Some(Effect::Populate)));
    }

    #[test]
    fn daily_buttons_stage_ready_values() {
        let mut editor = loaded();
        open(&mut editor, SectionId::Daily);
        fire(&mut editor, "daily/all-ready", "");
        let session = editor.session.as_ref().unwrap();
        assert_eq!(session.value(&["p", "L"]).unwrap(), "0");
        assert_eq!(session.value(&["p", "S"]).unwrap(), "-1");
        assert_eq!(editor.note.as_ref().unwrap().0, "Staged daily edit");
    }

    #[test]
    fn partners_stay_mutual_through_the_picker() {
        let mut editor = loaded();
        open(&mut editor, SectionId::Partners);
        fire(&mut editor, "partners/assign-all", "");
        let save = editor.session.as_ref().unwrap().derived().unwrap();
        assert!(save.pets.iter().filter(|p| p.unlocked).all(|p| p.partner_type_id.is_some()));
        // Unlink the first unlocked pet through its picker ("None" is index 0).
        let index = save.pets.iter().position(|p| p.unlocked).unwrap();
        let partner = save.pets[index].partner_type_id.unwrap();
        fire(&mut editor, &format!("partners/set/{index}"), "0");
        let save = editor.session.as_ref().unwrap().derived().unwrap();
        assert_eq!(save.pets[index].partner_type_id, None);
        assert!(save.pets.iter().all(|p| p.type_id != partner || p.partner_type_id.is_none() || p.type_id == save.pets[index].type_id));
        fire(&mut editor, "partners/filter", "zzz-nothing");
        assert!(build(&editor).rows.is_empty());
    }
}
