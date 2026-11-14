//! Planet / Ultimate Beings: the planet's top-level scalars (`T.d` level, `T.h`
//! unspent Baal Power) plus two distinct boss lists that share ids and names:
//! `T.f`, the regular respawning Ultimate Beings, and `T.k`, the Ultimate Being
//! V2 bosses (defeated once per rebirth; their defeats drive a multiplier).
use save_parser::items;
use save_parser::labels::{PlanetField, UbField, UbV2Field};
use save_parser::raw::Raw;
use save_parser::session::EditSession;

use crate::editor::common::{owned, scalar_noted, stage_button, Check, F};
use crate::form::{muted, spacer, text};

const DONE: &str = "Staged planet edit";

pub fn ub_name(id: u32) -> String {
    items::ultimate_being_name(id).map_or_else(|| format!("UB {id}"), str::to_string)
}

fn list_len(session: &EditSession, key: &str) -> usize {
    match session.root().get_path(&["T", key]) { Some(Raw::List(items)) => items.len(), _ => 0 }
}

pub fn build(form: &mut F, session: &EditSession) {
    form.title("Planet / Ultimate Beings");
    if session.root().get_path(&["T"]).is_none() {
        form.note("No planet data in this save.");
        return;
    }
    form.note("The planet level and unspent Baal Power are top-level values. The Ultimate Beings respawn on a timer (“Force spawn” sets the countdown to 0); the Ultimate Being V2 bosses are defeated once per rebirth, and their cumulative defeats drive the “Multi from Ultimate Beings” bonus.");
    form.pin();
    for (key, label) in [(PlanetField::Level.key(), "Planet Level"), (PlanetField::BaalPower.key(), "Unspent Baal Power")] {
        form.row(vec![text(label).w(160.0), scalar_noted(form, session, &["T", key], label, Check::Finite, DONE).w(200.0), spacer()]);
    }

    form.sep();
    form.subtitle("Ultimate Beings");
    let count = list_len(session, "f");
    if count == 0 { form.note("No Ultimate Beings in this save."); }
    else { form.header(vec![text("Ultimate Being").w(170.0), text("Kill count").w(120.0), text("God power gained").w(170.0), text("Spawn in").w(90.0), spacer()]); }
    for index in 0..count {
        let i = index.to_string();
        let get = |key: &str| session.value(&["T", "f", &i, key]).unwrap_or_default();
        let id: u32 = get(UbField::Ub.key()).trim().parse().unwrap_or(0);
        let name = ub_name(id);
        let spawn_ms: f64 = get(UbField::SpawnCountdown.key()).trim().parse().unwrap_or(0.0);
        form.row(vec![
            text(name.as_str()).w(170.0).hint(format!("UB id {id}")),
            scalar_noted(form, session, &["T", "f", &i, UbField::KillCount.key()], &format!("{name} kill count"), Check::Uint, DONE).w(120.0),
            // A BigDouble; kept as raw text (it can exceed f64).
            text(get(UbField::GodPowerGained.key())).size(11.0).w(170.0),
            text(format!("{:.1} min", spawn_ms / 60_000.0)).size(11.0).w(90.0),
            if spawn_ms > 0.0 {
                stage_button(form, format!("planet/spawn/{index}"), "Force spawn",
                    vec![(owned(&["T", "f", &i, UbField::SpawnCountdown.key()]), format!("{name} spawn countdown"), "0".to_string())], DONE)
                    .w(110.0).hint("Set the spawn countdown to 0 so this UB spawns next tick")
            } else { spacer().w(110.0) },
            spacer(),
        ]);
    }

    form.sep();
    form.subtitle("Ultimate Beings V2");
    form.note("Defeated once per rebirth; defeats accumulate across rebirths.");
    let count = list_len(session, "k");
    if count == 0 { form.note("No Ultimate Being V2 data in this save."); }
    else { form.header(vec![text("Ultimate Being V2").w(170.0), text("Defeats").w(120.0), text("State")]); }
    for index in 0..count {
        let i = index.to_string();
        let get = |key: &str| session.value(&["T", "k", &i, key]).unwrap_or_default();
        let id: u32 = get(UbV2Field::Ub.key()).trim().parse().unwrap_or(0);
        let name = format!("{} V2", ub_name(id));
        form.row(vec![
            text(name.as_str()).w(170.0).hint(format!("UB id {id}")),
            scalar_noted(form, session, &["T", "k", &i, UbV2Field::Defeats.key()], &format!("{name} defeats"), Check::Uint, DONE).w(120.0),
            muted(get(UbV2Field::State.key())).size(11.0),
        ]);
    }
}
