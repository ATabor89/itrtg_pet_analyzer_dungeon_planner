//! Resources & currencies: flat scalar fields driven by the field registry.
use save_parser::registry::{FieldKind, FieldRegistry, SectionId};
use save_parser::session::EditSession;

use crate::editor::common::{flag, scalar, Check, F};
use crate::form::{muted, spacer, text, Tone};

pub fn build(form: &mut F, session: &EditSession, registry: &FieldRegistry) {
    form.title(SectionId::Resources.title());
    form.note("Large counts are edited as text and written verbatim, so 17-digit values and scientific-notation doubles keep full precision.");
    form.pin();
    for field in registry.for_section(SectionId::Resources) {
        let label = text(field.name).tone(Tone::Bright).w(260.0).hint(field.help.unwrap_or_default());
        let editor = if session.value(&field.path).is_none() {
            muted("— (absent in this save)")
        } else {
            match field.kind {
                FieldKind::Bool => flag(form, session, &field.path, field.name, ""),
                FieldKind::Number => scalar(form, session, &field.path, field.name, Check::Number).w(300.0),
                FieldKind::Text => scalar(form, session, &field.path, field.name, Check::Text).w(300.0),
            }
        };
        form.row(vec![label, editor, spacer()]);
    }
}
