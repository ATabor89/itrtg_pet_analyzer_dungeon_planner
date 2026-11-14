//! Afky God: the single struct at `X.t` (the AFK god that fires at clones).
use save_parser::labels::{AfkyGodField, FieldKind};
use save_parser::session::EditSession;

use crate::editor::common::{scalar, Check, F};
use crate::form::{spacer, text};

pub fn build(form: &mut F, session: &EditSession) {
    form.title("Afky God");
    if session.root().get_path(&["X", "t"]).is_none() {
        form.note("No Afky God data in this save.");
        return;
    }
    form.note("The AFK god that fires at clones for idle experience.");
    form.pin();
    for field in AfkyGodField::ALL {
        let mut path: Vec<&str> = vec!["X", "t"];
        path.extend(field.key().split('.'));
        // Counts are whole numbers; everything else is a BigDouble written verbatim.
        let check = if field.kind() == FieldKind::UInt { Check::Uint } else { Check::Finite };
        form.row(vec![text(field.label()).w(180.0), scalar(form, session, &path, field.label(), check).w(240.0), spacer()]);
    }
}
