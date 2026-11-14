//! Growth Chamber screen: form rows over the shared chamber logic
//! (`itrtg_planner::chamber`). Mirrors the egui view's inputs, pet cards, pet
//! picker and run report; nothing here computes growth.
use itrtg_models::{parse_flexible_number, CampaignType, ExportPet};
use itrtg_planner::campaign::SpecialPet;
use itrtg_planner::chamber::{self as logic, ChamberState, Tone as ReportTone, ACCESSORY_CHOICES, ARMOR_CHOICES, FOODS, QUALITIES, REBIRTH_UNITS, WEAPON_CHOICES};
use itrtg_planner::merge::CampaignContext;

use crate::app::AppModel;
use crate::form::{button, check, combo, gap, input, muted, options, spacer, text, Act, Form, Mark, Row, RowKind, Tone};

/// One pet card: its rows and whether it is a stale-entry warning stub.
pub struct Card { pub rows: Vec<Row>, pub warn: bool }

#[derive(Default)]
pub struct ChamberForms {
    pub acts: std::collections::HashMap<String, Act<AppModel>>,
    /// Pinned run controls.
    pub run: Vec<Row>,
    pub settings: Vec<Row>,
    pub report: Vec<Row>,
    pub cards: Vec<Card>,
    /// Tallest card in logical pixels, so the grid can use uniform cells.
    pub card_height: f32,
    pub picker: Vec<Row>,
}

/// Show whole numbers without a fraction; keep decimals the user typed.
fn num(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 { format!("{value:.0}") } else { format!("{value}") }
}

/// Parse a typed number; blank means 0 (the clear-with-backspace workflow).
fn parse(value: &str) -> Option<f64> {
    let t = value.trim();
    if t.is_empty() { return Some(0.0); }
    parse_flexible_number(t).filter(|v| v.is_finite())
}

fn parse_u32(value: &str, low: u32, high: u32) -> Option<u32> {
    parse(value).map(|v| (v.max(0.0).min(u32::MAX as f64) as u32).clamp(low, high))
}

fn state(app: &mut AppModel) -> &mut ChamberState { &mut app.session.chamber }

fn row_height(row: &Row) -> f32 {
    match row.kind { RowKind::Note => 22.0, RowKind::Separator => 9.0, RowKind::Title => 38.0, RowKind::Header => 24.0, RowKind::Editor => 190.0, RowKind::Normal => 34.0 }
}

/// Run `edit` against a pet's override, seeded from its effective export.
fn with_pet(app: &mut AppModel, name: &str, edit: impl FnOnce(&mut ChamberState, &ExportPet)) {
    let Some(pet) = app.pets.iter().find(|p| p.name == name) else { return };
    let Some(effective) = logic::effective_export(pet, &app.session.chamber) else { return };
    edit(&mut app.session.chamber, &effective);
}

pub fn run(app: &mut AppModel) {
    let settings = app.settings();
    let rates = app.rates();
    let context = CampaignContext { bonuses: &app.bonuses, roster: &app.pets, inputs: &settings.campaign_inputs, include_equipment: true, include_class: true };
    logic::run(&mut app.session.chamber, &app.pets, &context, &rates);
}

pub fn recommend(app: &mut AppModel) {
    let settings = app.settings();
    let context = CampaignContext { bonuses: &app.bonuses, roster: &app.pets, inputs: &settings.campaign_inputs, include_equipment: true, include_class: true };
    logic::recommend_chamber(&mut app.session.chamber, &app.pets, &context);
}

pub fn build(app: &AppModel) -> ChamberForms {
    let mut form: Form<AppModel> = Form::default();
    let st = &app.session.chamber;
    let settings = app.settings();
    let rates = app.rates();
    let context = CampaignContext { bonuses: &app.bonuses, roster: &app.pets, inputs: &settings.campaign_inputs, include_equipment: true, include_class: true };
    let mut out = ChamberForms::default();

    // --- Pinned run controls ---
    let count = st.chamber.len();
    let over = count > 10;
    let run_id = form.on("run", |app, _| run(app));
    let recommend_id = form.on("recommend", |app, _| recommend(app));
    form.row(vec![
        button(run_id, "\u{25B6} Run").primary().enabled(!over).w(90.0),
        button(recommend_id, "Recommend chamber").w(160.0).hint("Fill the chamber with the top 10 by Growth bonus (tiebreak: growth)"),
        text(format!("{count}/10 in chamber{}", if over { " — too many!" } else { "" })).tone(if over { Tone::Warning } else { Tone::Muted }),
    ]);
    out.run = form.take_rows();

    // --- Global run parameters ---
    let mut cells = vec![
        muted("Hours:").w(46.0),
        input(form.on("hours", |app, v| if let Some(n) = parse_u32(v, 1, 12) { state(app).hours = n; }), st.hours.to_string()).w(56.0),
        muted("UPC:").w(36.0).hint("Ultimate Pet Challenges completed — each adds +5% to every pet's campaign contribution (capped at 100%). Auto-filled when you import a Main-stats export."),
        input(form.on("upc", |app, v| { let max = state(app).upc_max; if let Some(n) = parse_u32(v, 0, max) { state(app).upc_done = n; } }), st.upc_done.to_string()).w(56.0),
        muted(format!("/ {}  \u{2192} +{:.0}%", st.upc_max, st.upc_pct())).w(100.0),
        muted("PGC:").w(36.0).hint("Patreon God Challenges completed — +1% growth per completion, doubled once all are done (24/25 → ×1.24, 25/25 → ×1.50). Stacks with the Magic Egg. Auto-filled when you import a Main-stats export."),
        input(form.on("pgc", |app, v| { let max = state(app).pgc_max; if let Some(n) = parse_u32(v, 0, max) { state(app).pgc_done = n; } }), st.pgc_done.to_string()).w(56.0),
        muted(format!("/ {}  \u{2192} ×{:.2} growth", st.pgc_max, st.pgc_mult())).w(140.0),
        muted("Pandora feedings:").w(116.0).hint("Pandora's starting feeding count. It climbs as she's fed each cycle (bonus caps at 20) and resets at the start of each rebirth."),
        input(form.on("pandora", |app, v| if let Some(n) = parse_u32(v, 0, 20) { state(app).pandora_feedings = n; }), st.pandora_feedings.to_string()).w(56.0),
    ];
    // Global counterpart of a card's "Refresh from export": shown when any input
    // auto-filled from a Main-stats import has been edited away from it.
    if st.deviates_from_import() {
        cells.push(text("edited").tone(Tone::Warning).w(44.0));
        cells.push(button(form.on("reset-import", |app, _| state(app).reset_to_import()), "Reset to import").w(130.0)
            .hint("Discard edits — reset UPC, PGC, DPC multi, Fish Power and Fishing level to their imported Main-stats values"));
    }
    cells.push(spacer());
    form.row(cells);

    // --- Food (per-feeding growth, fed to every pet) and the DPC boost ---
    form.row(vec![
        muted("Food:").w(46.0),
        combo(form.on("food", |app, v| if let Ok(i) = v.parse::<usize>() { state(app).food_choice = i.min(FOODS.len() - 1); }), options(FOODS), st.food_choice).w(120.0),
        text(format!("{:.2} growth/feeding", st.food_growth())).w(140.0),
        muted(format!("× {} feedings/cycle", st.hours / 3)).w(130.0),
        muted("DPC highest multi:").w(120.0).hint("Day Pet Challenge highest pet multiplier (a %). Boosts food growth by log2(multi), capped 100%. Auto-filled from a Main-stats export."),
        input(form.on("dpc", |app, v| if let Some(n) = parse(v) { state(app).dpc_highest_multi = n.clamp(0.0, 1e18); }), num(st.dpc_highest_multi)).w(150.0),
        muted(format!("\u{2192} +{:.2}% to food", st.dpc_boost_pct())),
    ]);

    // --- Gold Dragon (25% of his food growth goes to every pet) ---
    let dragon_options = options(std::iter::once("None").chain(FOODS));
    form.row(vec![
        muted("Gold Dragon food:").w(120.0).hint("Feeding Gold Dragon gives every pet 25% of the growth he gains."),
        combo(form.on("dragon", |app, v| if let Ok(i) = v.parse::<usize>() { state(app).gold_dragon_food = i.checked_sub(1).filter(|&i| i < FOODS.len()); }), dragon_options, st.gold_dragon_food.map_or(0, |i| i + 1)).w(120.0),
        muted(if st.gold_dragon_food.is_some() { format!("(+{:.2}/feeding to all)", st.gold_dragon_broadcast()) } else { String::new() }),
    ]);

    // --- Rebirths (cycle scheduling) ---
    let mut cells = vec![
        check(form.on("rebirth", |app, v| state(app).rebirth_enabled = v == "1"), "Model rebirths", st.rebirth_enabled).w(150.0)
            .hint("Campaigns can't span a rebirth, so each rebirth runs full cycles then one shorter cycle for the leftover time."),
        muted("every").w(40.0),
        input(form.on("rebirth-value", |app, v| if let Some(n) = parse(v) { state(app).rebirth_value = n.clamp(0.1, 100_000.0); }), num(st.rebirth_value)).w(80.0).enabled(st.rebirth_enabled),
        combo(form.on("rebirth-unit", |app, v| if let Ok(i) = v.parse::<usize>() { state(app).rebirth_unit = i.min(REBIRTH_UNITS.len() - 1); }), options(REBIRTH_UNITS.iter().map(|(label, _)| *label)), st.rebirth_unit).w(100.0).enabled(st.rebirth_enabled),
    ];
    // Effective whole-hour length — shown when it's a conversion or a partial
    // hour gets truncated (e.g. 7.03 d ⭢ 168 h, 7.5 h ⭢ 7 h).
    if st.rebirth_unit != 0 || st.rebirth_value.fract() != 0.0 {
        cells.push(muted(format!("= {} h", st.rebirth_total_hours())).w(80.0));
    }
    if st.rebirth_enabled {
        let cycle = st.hours.clamp(1, 12);
        let rebirth = st.rebirth_total_hours();
        // Compact schedule: collapse the full cycles into a count and only show
        // the remainder when there is one (e.g. "14 × 12 h + 1 h").
        let full = cycle.min(rebirth);
        let (n_full, rest) = (rebirth / full, rebirth % full);
        let mut pattern = if n_full <= 1 { format!("{full} h") } else { format!("{n_full} × {full} h") };
        if rest > 0 { pattern += &format!(" + {rest} h"); }
        cells.push(if rebirth < cycle {
            text(format!("\u{26A0} rebirth ({rebirth} h) < cycle — cycles clamped to {pattern}")).tone(Tone::Warning)
        } else {
            muted(format!("\u{2192} {pattern} per rebirth"))
        });
    } else {
        cells.push(spacer());
    }
    form.row(cells);

    // --- Fishing (Fish Power food boost, decays over a rebirth's first 30 h) ---
    let boost = st.fishing_boost();
    form.row(vec![
        muted("Fish Power:").w(76.0).hint("Boosts all food growth by FishPower^0.25 × milestone, decaying to 0 over the first 30 h of each rebirth. 0 = off."),
        input(form.on("fish-power", |app, v| if let Some(n) = parse(v) { state(app).fish_power = n.clamp(0.0, 1e15); }), num(st.fish_power)).w(160.0),
        muted("Fishing level:").w(90.0).hint("Milestones: +10% at level 15, another +10% at 27."),
        input(form.on("fishing-level", |app, v| if let Some(n) = parse_u32(v, 0, 100) { state(app).fishing_level = n; }), st.fishing_level.to_string()).w(56.0),
        if boost <= 0.0 { spacer() } else if st.rebirth_enabled {
            muted(format!("\u{2192} +{boost:.1}% food at rebirth start, fading over 30 h"))
        } else {
            text("needs rebirth modeling to apply").tone(Tone::Warning)
        },
    ]);

    // --- Run mode ---
    form.row(vec![
        muted("Max cycles:").w(76.0),
        input(form.on("max-cycles", |app, v| if let Some(n) = parse_u32(v, 1, 1_000_000) { state(app).max_cycles = n; }), st.max_cycles.to_string()).w(90.0),
        muted("Adventurer XP ×:").w(110.0).hint("Multiplier on Adventurer campaign class XP (pet-stone / ChP Adventurer-XP purchases). Base XP per cycle is 250·(1 + total growth/20000)·hours. 1.0 = no purchases; 0 turns class-level modelling off."),
        input(form.on("adv-xp", |app, v| if let Some(n) = parse(v) { state(app).adv_xp_mult = n.clamp(0.0, 1000.0); }), num(st.adv_xp_mult)).w(70.0),
        check(form.on("stop-early", |app, v| state(app).run_until_targets = v == "1"), "Stop early when all targets reached", st.run_until_targets).w(270.0)
            .hint("Off: always run the full max cycles. On: stop as soon as every target is hit (max cycles still caps it)."),
        check(form.on("after-campaign", |app, v| state(app).exported_after_campaign = v == "1"), "Export taken at a campaign's end", st.exported_after_campaign)
            .hint("On: the first cycle adds no passive (Moai) growth, since an export captured right when a campaign finished already includes it. Avoids double-counting that ~one cycle of Moai."),
    ]);
    out.settings = form.take_rows();

    // --- Report ---
    out.report = logic::report(st).into_iter().map(|line| {
        let tone = match line.tone {
            ReportTone::Bright => Tone::Bright, ReportTone::Normal => Tone::Normal, ReportTone::Muted => Tone::Muted,
            ReportTone::Accent => Tone::Accent, ReportTone::Success => Tone::Success, ReportTone::Warning => Tone::Warning,
        };
        // The shared report uses an arrow the browser's fallback font lacks.
        Row { cells: vec![text(line.text.replace('\u{2B62}', "\u{2192}")).tone(tone).size(line.size + 1.0).hint(line.hint.unwrap_or_default())], kind: RowKind::Note, ..Default::default() }
    }).collect();

    // --- Pet cards ---
    let malus = logic::chamber_nightmare_malus(st, &app.pets);
    for name in &st.chamber {
        let pet = app.pets.iter().find(|p| &p.name == name);
        let key = |what: &str| format!("pet/{name}/{what}");
        // An entry can go stale after a re-import (renamed/missing pet, locked,
        // no export). Show a removable stub instead of skipping silently — a
        // skipped ghost still counts toward the 10-slot cap.
        if let Some(reason) = logic::chamber_entry_issue(pet) {
            let owned = name.clone();
            form.row(vec![text(name.as_str()).tone(Tone::Bright).size(13.0)]);
            form.note_tone(reason, Tone::Warning);
            form.row(vec![button(form.on(key("remove"), move |app, _| {
                logic::remove_entries(&mut app.session.chamber, &app.pets, std::slice::from_ref(&owned));
            }), "Remove from chamber")]);
            out.cards.push(Card { rows: form.take_rows(), warn: true });
            continue;
        }
        let Some(pet) = pet else { continue };
        let Some(sim) = logic::chamber_pet(pet, &context, &rates, st) else { continue };
        let Some(effective) = logic::effective_export(pet, st) else { continue };
        let breakdown = logic::growth_bonus_breakdown(pet, &context, st);
        let is_nightmare = matches!(sim.special, Some(SpecialPet::Nightmare { .. }));

        // Name + a special tag describing the pet's campaign role.
        let tag = match sim.special {
            Some(SpecialPet::Pandora { .. }) => text("amplifies deposit").tone(Tone::Accent),
            Some(SpecialPet::Bag { .. }) => text("gifts the lowest").tone(Tone::Accent),
            Some(SpecialPet::Nightmare { .. }) => text(format!("\u{2212}{:.2} to others", malus.unwrap_or(0.0))).tone(Tone::Warning),
            None => spacer(),
        };
        form.row(vec![text(name.as_str()).tone(Tone::Bright).size(13.0), tag.size(10.0).w(104.0)]);
        // Total growth + Growth bonus — reduced live by a chamber Nightmare's
        // malus (Nightmare's own bonus is not reduced).
        let shown_bonus = if is_nightmare { sim.campaign_bonus_pct } else { sim.campaign_bonus_pct - malus.unwrap_or(0.0) };
        form.push(RowKind::Note, vec![text(format!("total {:.0}   +{:.0}% growth", sim.growth * sim.growth_multiplier, shown_bonus)).size(12.0)]);
        // Where the bonus comes from — only when a flat layer contributes.
        if breakdown.class.is_some() || breakdown.equipment.is_some() {
            let innate = breakdown.innate.get(&CampaignType::Growth).copied().unwrap_or(0.0);
            let mut parts = vec![format!("innate {innate:+.0}")];
            if let Some(v) = breakdown.class { parts.push(format!("class {v:+.0}")); }
            if let Some(v) = breakdown.equipment { parts.push(format!("equip {v:+.0}")); }
            form.note(parts.join("  ·  "));
        }
        if !is_nightmare && let Some(m) = malus {
            form.note_tone(format!("Nightmare \u{2212}{m:.2}"), Tone::Warning);
        }
        // Passive/hr (read-only) + editable class level.
        let owned = name.clone();
        form.row(vec![
            // `+ 0.0` turns a negative zero into "0.0".
            muted(format!("passive {:.1}/hr   ·   CL", sim.passive_per_hour + 0.0)).size(11.0),
            input(form.on(key("cl"), move |app, v| if let Some(level) = parse_u32(v, 0, 200) {
                with_pet(app, &owned, |state, eff| logic::set_class_level(state, &owned, eff, level));
            }), effective.class_level.to_string()).w(64.0),
        ]);
        // Editable equipment. An item outside the campaign choices stays listed
        // (and selected) so the picker shows what is actually equipped.
        let slot = |form: &mut Form<AppModel>, label: &str, what: &str, choices: &[&str], current: Option<&str>,
                    apply: fn(&mut ChamberState, &str, &ExportPet, &str)| {
            let current = current.unwrap_or("none");
            let mut list: Vec<String> = choices.iter().map(|s| s.to_string()).collect();
            if !list.iter().any(|c| c == current) { list.push(current.to_string()); }
            let index = list.iter().position(|c| c == current).unwrap_or(0);
            let (owned, picks) = (name.clone(), list.clone());
            let id = form.on(key(what), move |app, v| {
                let Some(choice) = v.parse::<usize>().ok().and_then(|i| picks.get(i)) else { return };
                with_pet(app, &owned, |state, eff| apply(state, &owned, eff, choice));
            });
            form.row(vec![muted(label).w(20.0).mono(), combo(id, options(list), index)]);
        };
        let weapon = effective.loadout.weapon.as_ref();
        slot(&mut form, "W", "weapon", &WEAPON_CHOICES, weapon.map(|e| e.name.as_str()), logic::set_weapon);
        // Quality + upgrade, only for a stick.
        if let Some(stick) = weapon.filter(|w| logic::is_stick(w)) {
            let (for_quality, for_upgrade) = (name.clone(), name.clone());
            form.row(vec![
                gap(20.0),
                combo(form.on(key("quality"), move |app, v| if let Some(q) = v.parse::<usize>().ok().and_then(|i| QUALITIES.get(i).copied()) {
                    with_pet(app, &for_quality, |state, eff| logic::set_weapon_quality(state, &for_quality, eff, q));
                }), options(QUALITIES.iter().map(|q| format!("{q:?}"))), QUALITIES.iter().position(|q| *q == stick.quality).unwrap_or(0)).w(84.0),
                muted("+").w(10.0),
                input(form.on(key("upgrade"), move |app, v| if let Some(up) = parse_u32(v, 0, 20) {
                    with_pet(app, &for_upgrade, |state, eff| logic::set_weapon_upgrade(state, &for_upgrade, eff, up as u8));
                }), stick.upgrade_level.unwrap_or(0).to_string()).w(56.0),
                spacer(),
            ]);
        }
        slot(&mut form, "A", "armor", &ARMOR_CHOICES, effective.loadout.armor.as_ref().map(|e| e.name.as_str()), logic::set_armor);
        slot(&mut form, "Ac", "accessory", &ACCESSORY_CHOICES, effective.loadout.accessory.as_ref().map(|e| e.name.as_str()), logic::set_accessory);
        // Override status + refresh (only when this pet has been edited).
        if st.overrides.contains_key(name) {
            let owned = name.clone();
            form.row(vec![
                text("edited").tone(Tone::Warning).w(44.0),
                button(form.on(key("refresh"), move |app, _| { state(app).overrides.remove(&owned); }), "Refresh from export")
                    .hint("Discard edits — reset equipment + CL to the current export"),
            ]);
        }
        let owned = name.clone();
        form.row(vec![
            muted("target:").w(48.0),
            input(form.on(key("target"), move |app, v| if let Some(target) = parse(v) {
                logic::set_target(state(app), &owned, target.max(0.0) as u64);
            }), st.targets.get(name).copied().unwrap_or(0).to_string()),
        ]);
        out.cards.push(Card { rows: form.take_rows(), warn: false });
    }
    out.card_height = out.cards.iter().map(|c| c.rows.iter().map(row_height).sum::<f32>()).fold(0.0, f32::max) + 20.0;

    // --- Pet picker ---
    form.row(vec![
        muted("Pets (by Growth bonus):").w(150.0),
        input(form.on("search", |app, v| state(app).search = v.to_string()), st.search.as_str()).live().hint("filter…").w(180.0),
        spacer(),
    ]);
    for entry in logic::picker_entries(&app.pets, &context, &st.search) {
        let name = entry.pet.name.clone();
        let member = st.chamber.iter().any(|n| n == &name);
        let owned = name.clone();
        let id = form.on(format!("pick/{name}"), move |app, v| { logic::set_membership(state(app), &owned, v == "1"); });
        // Enforce the 10-pet cap: can't tick an 11th.
        form.row(vec![
            check(id, name, member).enabled(member || count < 10).w(240.0),
            muted(format!("+{:.0}%  ({})", entry.bonus, entry.growth)),
        ]).mark = if member { Mark::Selected } else { Mark::None };
    }
    out.picker = form.take_rows();
    out.acts = form.into_acts();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Session, EXAMPLE_EXPORT};
    use crate::form::Kind;

    fn app() -> AppModel {
        let mut app = AppModel::new(Session::default()).unwrap();
        app.import(EXAMPLE_EXPORT, "Example").unwrap();
        app
    }

    fn fire(app: &mut AppModel, id: &str, value: &str) {
        let act = build(app).acts.get(id).unwrap_or_else(|| panic!("no action {id}")).clone();
        act(app, value);
    }

    #[test]
    fn recommend_fills_ten_cards_and_run_produces_a_report() {
        let mut app = app();
        assert!(build(&app).cards.is_empty());
        fire(&mut app, "recommend", "");
        let forms = build(&app);
        assert_eq!(forms.cards.len(), 10);
        assert!(forms.report.is_empty());
        fire(&mut app, "max-cycles", "5");
        fire(&mut app, "run", "");
        let forms = build(&app);
        assert!(forms.report[0].cells[0].text.starts_with("Ran 5 cycles"), "{}", forms.report[0].cells[0].text);
    }

    #[test]
    fn inputs_clamp_and_ignore_garbage() {
        let mut app = app();
        fire(&mut app, "hours", "40");
        assert_eq!(app.session.chamber.hours, 12);
        fire(&mut app, "hours", "abc");
        assert_eq!(app.session.chamber.hours, 12);
        fire(&mut app, "hours", "");
        assert_eq!(app.session.chamber.hours, 1, "blank clears to the range minimum");
        fire(&mut app, "dragon", "5");
        assert_eq!(app.session.chamber.gold_dragon_food, Some(4));
        fire(&mut app, "dragon", "0");
        assert_eq!(app.session.chamber.gold_dragon_food, None);
        fire(&mut app, "dpc", "3.664e9");
        assert!((app.session.chamber.dpc_boost_pct() - 31.77).abs() < 0.05);
    }

    #[test]
    fn card_edits_create_an_override_that_refresh_discards() {
        let mut app = app();
        fire(&mut app, "recommend", "");
        let name = app.session.chamber.chamber[0].clone();
        let index = WEAPON_CHOICES.iter().position(|w| *w == "Legendary Stick").unwrap();
        fire(&mut app, &format!("pet/{name}/weapon"), &index.to_string());
        let weapon = app.session.chamber.overrides[&name].loadout.weapon.clone().unwrap();
        assert_eq!(weapon.name, "Legendary Stick");
        // A stick exposes quality/upgrade controls, and the card says "edited".
        fire(&mut app, &format!("pet/{name}/upgrade"), "7");
        assert_eq!(app.session.chamber.overrides[&name].loadout.weapon.as_ref().unwrap().upgrade_level, Some(7));
        fire(&mut app, &format!("pet/{name}/target"), "123456");
        assert_eq!(app.session.chamber.targets[&name], 123_456);
        fire(&mut app, &format!("pet/{name}/refresh"), "");
        assert!(app.session.chamber.overrides.is_empty());
    }

    #[test]
    fn picker_filters_and_enforces_the_cap() {
        let mut app = app();
        fire(&mut app, "recommend", "");
        let forms = build(&app);
        let unticked = forms.picker.iter().skip(1).find(|r| !r.cells[0].checked).unwrap();
        assert!(unticked.cells[0].disabled, "an 11th pet cannot be ticked");
        let member = app.session.chamber.chamber[0].clone();
        fire(&mut app, &format!("pick/{member}"), "0");
        assert_eq!(app.session.chamber.chamber.len(), 9);
        fire(&mut app, "search", "zzzz-no-such-pet");
        assert_eq!(build(&app).picker.len(), 1, "only the filter row remains");
        assert_eq!(build(&app).picker[0].cells[1].kind, Kind::Input);
    }

    #[test]
    fn stale_entries_render_a_removable_stub() {
        let mut app = app();
        app.session.chamber.chamber.push("Not A Pet".into());
        app.session.chamber.targets.insert("Not A Pet".into(), 5);
        let forms = build(&app);
        assert!(forms.cards[0].warn);
        fire(&mut app, "pet/Not A Pet/remove", "");
        assert!(app.session.chamber.chamber.is_empty());
        assert!(app.session.chamber.targets.is_empty());
    }
}
