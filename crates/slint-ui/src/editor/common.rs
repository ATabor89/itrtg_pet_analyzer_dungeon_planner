//! Helpers shared by the editor sections: validated scalar cells and staging.
use save_parser::session::EditSession;

use super::Editor;
use crate::form::{check, input, muted, Cell, Form};

pub type F = Form<Editor>;

pub fn owned(path: &[&str]) -> Vec<String> { path.iter().map(|s| s.to_string()).collect() }
pub fn refs(path: &[String]) -> Vec<&str> { path.iter().map(String::as_str).collect() }

/// What a typed value must look like before it is staged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    /// Any number the game writes: integers, decimals, `7.3E+185`, `1,234`.
    Number,
    /// A non-negative whole number.
    Uint,
    /// A whole number, possibly negative.
    Int,
    /// A finite decimal number.
    Finite,
    /// Any non-empty text.
    Text,
}

impl Check {
    pub fn ok(self, value: &str) -> bool {
        match self {
            Check::Number => itrtg_models::parse_flexible_number(value).is_some(),
            Check::Uint => value.parse::<u64>().is_ok(),
            Check::Int => value.parse::<i64>().is_ok(),
            Check::Finite => value.parse::<f64>().is_ok_and(f64::is_finite),
            Check::Text => !value.is_empty(),
        }
    }
}

/// Stage one scalar edit; failures land in the section note. Returns success.
pub fn stage(editor: &mut Editor, path: &[String], label: &str, value: &str) -> bool {
    let Some(session) = editor.session.as_mut() else { return false };
    match session.set_scalar(&refs(path), label, value) {
        Ok(()) => true,
        Err(error) => { editor.note = Some((format!("Edit failed: {error}"), true)); false }
    }
}

/// Stage several scalar edits as one step, then report `done`.
pub fn stage_all(editor: &mut Editor, edits: &[(Vec<String>, String, String)], done: &str) {
    let mut ok = !edits.is_empty();
    for (path, label, value) in edits { ok &= stage(editor, path, label, value); }
    if ok { editor.note = Some((done.to_string(), false)); }
}

/// An input bound to the scalar at `path`. Committing a valid, changed value
/// stages it; anything else snaps back to the stored value.
pub fn scalar(form: &F, session: &EditSession, path: &[&str], label: &str, check: Check) -> Cell {
    scalar_noted(form, session, path, label, check, "")
}

/// [`scalar`], reporting `done` in the section note when an edit is staged.
pub fn scalar_noted(form: &F, session: &EditSession, path: &[&str], label: &str, check: Check, done: &'static str) -> Cell {
    let Some(current) = session.value(path) else { return muted("—") };
    let (path, label) = (owned(path), label.to_string());
    let id = form.on(format!("set/{}", path.join(".")), move |editor, value| {
        let value = value.trim();
        let unchanged = editor.session.as_ref().and_then(|s| s.value(&refs(&path))).as_deref() == Some(value);
        if !unchanged && check.ok(value) && stage(editor, &path, &label, value) && !done.is_empty() {
            editor.note = Some((done.to_string(), false));
        }
    });
    input(id, current).mono()
}

/// A button that stages fixed scalar edits, then reports `done`.
pub fn stage_button(form: &F, id: impl Into<String>, caption: &str, edits: Vec<(Vec<String>, String, String)>, done: impl Into<String>) -> Cell {
    let done = done.into();
    crate::form::button(form.on(id, move |editor, _| stage_all(editor, &edits, &done)), caption)
}

/// A picker over typed choices: selecting an entry hands its value to `apply`.
/// `current` is added to the list (labelled by `fallback`) when it isn't offered.
pub fn pick<T: Clone + PartialEq + 'static>(form: &F, id: impl Into<String>, mut choices: Vec<(T, String)>, current: &T,
    fallback: impl FnOnce(&T) -> String, apply: impl Fn(&mut Editor, &T) + 'static) -> Cell {
    if !choices.iter().any(|(value, _)| value == current) { choices.push((current.clone(), fallback(current))); }
    let index = choices.iter().position(|(value, _)| value == current).unwrap_or(0);
    let labels = crate::form::options(choices.iter().map(|(_, label)| label.as_str()));
    let id = form.on(id, move |editor, value| {
        if let Some((choice, _)) = value.parse::<usize>().ok().and_then(|i| choices.get(i)) { apply(editor, choice); }
    });
    crate::form::combo(id, labels, index)
}

/// Format a millisecond span as hours (e.g. "11.5h").
pub fn fmt_hours(ms: f64) -> String { format!("{:.1}h", ms / 3_600_000.0) }

/// Completion percentage (0–100) of an elapsed-vs-target timer; 0 for a zero target.
pub fn percent(elapsed_ms: f64, target_ms: u64) -> f64 {
    if target_ms == 0 { 0.0 } else { (elapsed_ms / target_ms as f64 * 100.0).clamp(0.0, 100.0) }
}

/// A checkbox bound to the `True`/`False` scalar at `path`.
pub fn flag(form: &F, session: &EditSession, path: &[&str], label: &str, caption: &str) -> Cell {
    let Some(current) = session.value(path) else { return muted("—") };
    let (path, label) = (owned(path), label.to_string());
    let id = form.on(format!("flag/{}", path.join(".")), move |editor, value| {
        stage(editor, &path, &label, if value == "1" { "True" } else { "False" });
    });
    check(id, caption, current.eq_ignore_ascii_case("true"))
}

/// Case-insensitive substring filter; an empty needle matches everything.
pub fn matches(needle: &str, haystack: &str) -> bool {
    needle.is_empty() || haystack.to_lowercase().contains(needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timers_format_and_clamp() {
        assert_eq!(fmt_hours(43_200_000.0), "12.0h");
        assert_eq!(fmt_hours(0.0), "0.0h");
        assert!((percent(41_300_000.0, 43_200_000) - 95.6).abs() < 0.1);
        assert_eq!(percent(100.0, 0), 0.0);
        assert_eq!(percent(99_999_999.0, 43_200_000), 100.0);
    }

    #[test]
    fn checks_accept_what_the_game_writes() {
        assert!(Check::Number.ok("7.3E+185") && Check::Number.ok("1,234") && !Check::Number.ok("abc"));
        assert!(Check::Uint.ok("42") && !Check::Uint.ok("-1") && !Check::Uint.ok("1.5"));
        assert!(Check::Int.ok("-1") && !Check::Int.ok("x"));
        assert!(Check::Finite.ok("1.5e3") && !Check::Finite.ok("inf") && !Check::Finite.ok(""));
        assert!(Check::Text.ok("a") && !Check::Text.ok(""));
    }
}
