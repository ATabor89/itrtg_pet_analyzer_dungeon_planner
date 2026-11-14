//! Daily: the once-a-day timers, pack counts and Bonus Points. The timers are
//! countdowns in milliseconds, so "ready" is 0 (the signed daily pack: -1).
use save_parser::session::EditSession;

use crate::editor::common::{owned, scalar, stage_all, Check, F};
use crate::form::{button, spacer, text};

const TIMERS: [(&[&str], &str, &str); 3] = [
    (&["p", "L"], "Free Draw Timer (ms)", "0"),
    (&["p", "013"], "Bonus Pack Timer (ms)", "0"),
    (&["p", "S"], "Daily Pack Timer (ms)", "-1"),
];

pub fn build(form: &mut F, session: &EditSession) {
    form.title("Daily");
    if session.root().get_path(&["p"]).is_none() {
        form.note("No daily data in this save.");
        return;
    }
    form.note("Timers are countdowns in milliseconds — “Set ready” makes one claimable now; to shift the tick time, set it to (next tick − now) in ms.");
    form.row(vec![
        button(form.on("daily/all-ready", |editor, _| {
            let edits: Vec<_> = TIMERS.iter().map(|(path, _, ready)| (owned(path), "Daily timer".to_string(), ready.to_string())).collect();
            stage_all(editor, &edits, "Staged daily edit");
        }), "Set all ready").w(130.0).hint("Make the free draw and both packs claimable now"),
        spacer(),
    ]);
    form.pin();
    let timer = |form: &mut F, index: usize| {
        let (path, label, ready) = TIMERS[index];
        let id = form.on(format!("daily/ready/{index}"), move |editor, _| {
            stage_all(editor, &[(owned(path), label.to_string(), ready.to_string())], "Staged daily edit");
        });
        form.row(vec![text(label).w(190.0), scalar(form, session, path, label, Check::Finite).w(200.0),
            button(id, "Set ready").w(100.0).hint("Make this claimable now (sets the countdown to ready)"), spacer()]);
    };
    let plain = |form: &mut F, path: &[&str], label: &str| {
        form.row(vec![text(label).w(190.0), scalar(form, session, path, label, Check::Finite).w(200.0), spacer()]);
    };
    timer(form, 0);
    timer(form, 1);
    plain(form, &["p", "012"], "Bonus Packs Left");
    timer(form, 2);
    plain(form, &["p", "T"], "Daily Packs Left");
    plain(form, &["X", "q"], "Bonus Points");
}
