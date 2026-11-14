//! Dungeons: the active dungeon runs (`X.P`). A run counts elapsed ms (`b`) up
//! to a target (`c`); setting elapsed = target force-completes it so its loot is
//! ready on next load. Team composition lives in the Dungeon Planner.
use save_parser::items;
use save_parser::labels::ActiveDungeonField as Field;
use save_parser::raw::Raw;
use save_parser::session::EditSession;

use crate::editor::common::{fmt_hours, owned, percent, scalar_noted, stage_button, Check, F};
use crate::form::{muted, spacer, text};

pub fn build(form: &mut F, session: &EditSession) {
    form.title("Dungeons");
    form.note("Active dungeon runs. Force-complete a run (set elapsed = target) so its loot is ready to collect on next load. Team composition lives in the Dungeon Planner.");
    let count = match session.root().get_path(&["X", "P"]) { Some(Raw::List(items)) => items.len(), _ => 0 };
    if count == 0 {
        form.note("No active dungeon runs in this save.");
        return;
    }
    // (index, name, dungeon id, depth, elapsed, target, team)
    let runs: Vec<(String, String, u32, u32, f64, u64, String)> = (0..count).map(|index| {
        let i = index.to_string();
        let get = |key: &str| session.value(&["X", "P", &i, key]).unwrap_or_default();
        let dungeon: u32 = get(Field::DungeonId.key()).trim().parse().unwrap_or(0);
        let name = items::dungeon_name(dungeon).map(str::to_string).unwrap_or_else(|| format!("Dungeon {dungeon}"));
        // Elapsed/target are the game's float type; parse leniently.
        let elapsed: f64 = get(Field::Elapsed.key()).trim().parse().unwrap_or(0.0);
        let target = get(Field::TargetDuration.key()).trim().parse::<f64>().map(|f| f as u64).unwrap_or(0);
        let (depth, team) = (get(Field::Depth.key()).trim().parse().unwrap_or(0), get(Field::TeamIndex.key()));
        (i, name, dungeon, depth, elapsed, target, team)
    }).collect();
    let complete = |run: &(String, String, u32, u32, f64, u64, String)| (owned(&["X", "P", &run.0, "b"]), format!("{} run elapsed", run.1), run.5.to_string());
    let all: Vec<_> = runs.iter().filter(|r| r.5 > 0).map(complete).collect();
    let done = format!("Completed {} run(s)", all.len());
    form.row(vec![stage_button(form, "dungeons/complete-all", "Complete all runs", all, done).w(160.0).hint("Force-complete every active run (set elapsed = target)"), spacer()]);
    form.header(vec![text("Dungeon").w(150.0), text("Depth").w(50.0), text("Elapsed (ms)").w(130.0), text("Target").w(150.0), text("%").w(50.0), spacer().w(130.0), text("Team")]);
    form.pin();
    for run in &runs {
        let (index, name, dungeon, depth, elapsed, target, team) = run;
        let label = format!("{name} run elapsed");
        form.row(vec![
            text(name.as_str()).w(150.0).hint(format!("dungeon id {dungeon}")),
            text(format!("D{depth}")).w(50.0),
            scalar_noted(form, session, &["X", "P", index, "b"], &label, Check::Finite, "Staged dungeon-run elapsed").w(130.0),
            text(format!("{target} ({})", fmt_hours(*target as f64))).mono().size(11.0).w(150.0),
            text(format!("{:.0}%", percent(*elapsed, *target))).w(50.0),
            if *target > 0 {
                stage_button(form, format!("dungeons/complete/{index}"), "Force complete", vec![complete(run)], "Staged dungeon-run elapsed").w(130.0)
                    .hint("Set elapsed = target so the run is ready to collect")
            } else { spacer().w(130.0) },
            muted(format!("#{team}")).mono().size(11.0),
        ]);
    }
}
