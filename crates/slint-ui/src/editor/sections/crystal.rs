//! Crystal Factory: the single struct at `X.w` — Crystal Power (`a`), Energy
//! (`c`, spent on module upgrades), each module's level and stored grade-1
//! crystal count (`b.<i>.e.c`), and the equipped crystals' grades (`e`). The
//! module cost BigDoubles stay read-only.
use save_parser::items;
use save_parser::labels::{CrystalFactoryField, CrystalModuleField, CrystalStackField};
use save_parser::raw::Raw;
use save_parser::session::EditSession;

use crate::editor::common::{refs, scalar_noted, Check, F};
use crate::form::{muted, spacer, text};

const DONE: &str = "Staged Crystal Factory edit";

/// Raw path prefixes of the equipped crystals. `X.w.e` is a list; a single
/// crystal collapses to a lone struct, so both forms are handled.
fn equipped(session: &EditSession) -> Vec<Vec<String>> {
    let base = ["X", "w", "e"].map(String::from).to_vec();
    match session.root().get_path(&["X", "w", "e"]) {
        Some(Raw::List(items)) => (0..items.len()).map(|i| [base.clone(), vec![i.to_string()]].concat()).collect(),
        Some(Raw::Struct(_)) => vec![base],
        _ => Vec::new(),
    }
}

pub fn build(form: &mut F, session: &EditSession) {
    form.title("Crystal Factory");
    if session.root().get_path(&["X", "w"]).is_none() {
        form.note("No Crystal Factory data in this save.");
        return;
    }
    form.pin();
    for (key, label) in [(CrystalFactoryField::CrystalPower.key(), "Crystal Power"), (CrystalFactoryField::Energy.key(), "Energy")] {
        form.row(vec![text(label).w(160.0), scalar_noted(form, session, &["X", "w", key], label, Check::Finite, DONE).w(200.0), spacer()]);
    }
    form.row(vec![text("Progress").w(160.0), muted(session.value(&["X", "w", "d"]).unwrap_or_else(|| "—".into())).mono().size(11.0)]);

    form.sep();
    form.subtitle("Modules");
    let modules = match session.root().get_path(&["X", "w", "b"]) { Some(Raw::List(list)) => list.len(), _ => 0 };
    if modules == 0 { form.note("No modules."); }
    else { form.header(vec![text("Type").w(150.0), text("Level").w(100.0), text("Stored (g1)").w(130.0), text("Cost")]); }
    for index in 0..modules {
        let i = index.to_string();
        let at = |key: &str| session.value(&["X", "w", "b", &i, key]).unwrap_or_default();
        let grade: u32 = at(CrystalModuleField::Grade.key()).trim().parse().unwrap_or(0);
        let name = items::crystal_module_name(grade).map_or_else(|| format!("Grade {grade}"), str::to_string);
        // Stored grade-1 crystals live in the module's `e` stack; an unbuilt module has none.
        let stored = ["X", "w", "b", &i, "e", CrystalStackField::Count.key()];
        form.row(vec![
            text(name.as_str()).w(150.0).hint(format!("type id {grade}")),
            scalar_noted(form, session, &["X", "w", "b", &i, CrystalModuleField::Level.key()], &format!("{name} level"), Check::Uint, DONE).w(100.0),
            if session.value(&stored).is_some() { scalar_noted(form, session, &stored, &format!("{name} stored"), Check::Uint, DONE).w(130.0) } else { muted("—").w(130.0) },
            muted(at(CrystalModuleField::CostC.key())).mono().size(11.0),
        ]);
    }

    form.sep();
    form.subtitle("Equipped Crystals");
    let crystals = equipped(session);
    if crystals.is_empty() { form.note("No equipped crystals."); }
    else { form.header(vec![text("Crystal").w(150.0), text("Grade")]); }
    for prefix in crystals {
        let field = |key: &str| [prefix.clone(), vec![key.to_string()]].concat();
        let kind: u32 = session.value(&refs(&field(CrystalStackField::Crystal.key()))).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
        let name = items::crystal_module_name(kind).map_or_else(|| format!("Type {kind}"), str::to_string);
        let grade = field(CrystalStackField::Grade.key());
        form.row(vec![text(name.as_str()).w(150.0), scalar_noted(form, session, &refs(&grade), &format!("{name} grade"), Check::Uint, DONE).w(100.0), spacer()]);
    }
}
