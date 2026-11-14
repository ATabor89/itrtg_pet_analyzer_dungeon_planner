//! Partners: pet pairings (`X.b[i].F` = partner's type id, `999` = none;
//! `G` = days partnered). Partnerships are mutual; the session keeps both sides
//! in step, so the picker only offers single pets (plus "self" and "None").
use save_parser::items;
use save_parser::session::EditSession;

use crate::editor::common::{matches, scalar, Check, F};
use crate::form::{button, combo, input, muted, options, spacer, text};

#[derive(Default)]
pub struct State { filter: String }

fn pet_name(type_id: u32) -> String {
    items::pet_type_name(type_id).map_or_else(|| format!("type {type_id}"), str::to_string)
}

pub fn build(form: &mut F, session: &EditSession, state: &State) {
    form.title("Partners");
    let Some(save) = session.derived() else {
        form.note("Typed pet roster unavailable for this save — use the Raw Save Tree.");
        return;
    };
    // (X.b index, type id, name, partner, days)
    let rows: Vec<(usize, u32, String, Option<u32>)> = save.pets.iter().enumerate().filter(|(_, p)| p.unlocked)
        .map(|(index, p)| (index, p.type_id, pet_name(p.type_id), p.partner_type_id)).collect();
    let mut singles: Vec<(u32, String)> = rows.iter().filter(|r| r.3.is_none()).map(|r| (r.1, r.2.clone())).collect();
    singles.sort_by(|a, b| a.1.cmp(&b.1));
    form.note(format!("{} unlocked pets · {} single. Partners are mutual; pick from single pets (or self). “Assign all” pairs the rest at once.", rows.len(), singles.len()));
    form.row(vec![
        button(form.on("partners/assign-all", |editor, _| {
            let Some(session) = editor.session.as_mut() else { return };
            editor.note = Some(match session.assign_all_partners() {
                Ok(n) => (format!("Assigned {n} pet(s)"), false),
                Err(error) => (format!("Assign failed: {error}"), true),
            });
        }), "Assign all partners").w(170.0).hint("Randomly pair every single pet (odd one out partners itself)"),
        muted("Filter:").w(44.0),
        input(form.on("partners/filter", |editor, value| editor.partners.filter = value.to_string()), state.filter.as_str()).live().w(180.0),
        button(form.on("partners/clear", |editor, _| editor.partners.filter.clear()), "× clear").w(80.0),
        spacer(),
    ]);
    form.header(vec![text("Pet").w(200.0), text("Partner").w(240.0), text("Days").w(100.0), spacer()]);
    form.pin();
    let needle = state.filter.trim().to_lowercase();
    for (index, type_id, name, partner) in rows.iter().filter(|r| matches(&needle, &r.2)) {
        // None, self, then every other single pet — plus the current partner so it shows.
        let mut choices: Vec<(Option<u32>, String)> = vec![(None, "None".into()), (Some(*type_id), format!("{name} (self)"))];
        choices.extend(singles.iter().filter(|(t, _)| t != type_id).map(|(t, n)| (Some(*t), n.clone())));
        if let Some(current) = partner && !choices.iter().any(|(c, _)| *c == Some(*current)) {
            choices.push((Some(*current), pet_name(*current)));
        }
        let selected = choices.iter().position(|(c, _)| c == partner).unwrap_or(0);
        let labels = options(choices.iter().map(|(_, label)| label.as_str()));
        let (index, pet) = (*index, name.clone());
        let id = form.on(format!("partners/set/{index}"), move |editor, value| {
            let Some(session) = editor.session.as_mut() else { return };
            let Some((choice, _)) = value.parse::<usize>().ok().and_then(|i| choices.get(i)) else { return };
            let result = match choice { None => session.clear_partner(index), Some(type_id) => session.set_partner(index, *type_id) };
            editor.note = Some(match result {
                Ok(()) => (format!("Set {pet}'s partner"), false),
                Err(error) => (format!("Edit failed: {error}"), true),
            });
        });
        let days = index.to_string();
        form.row(vec![
            text(name.as_str()).w(200.0).hint(format!("type id {type_id}")),
            combo(id, labels, selected).w(240.0),
            scalar(form, session, &["X", "b", &days, "G"], "partner days", Check::Uint).w(100.0),
            spacer(),
        ]);
    }
}
