//! Text projections for the detail panel. Calculations come from shared domain APIs.
use itrtg_models::{base_growth_for_displayed_target, displayed_growth, MAGIC_EGG_GROWTH_MULT};
use itrtg_planner::{analyzer::*, growth::{format_duration, CapRelation}, merge::MergedPet};
use crate::app::AppModel;

pub fn eta(hours: Option<f64>) -> String {
    match hours {
        Some(h) if h <= 0.0 => "Ready now".into(),
        Some(h) => format!("~{}", format_duration(h)),
        None => "Unavailable / unreachable with these tools".into(),
    }
}

pub fn sections(app: &AppModel, pet: &MergedPet) -> Vec<(String, String)> {
    let settings = app.settings();
    let context = app.campaign_context(&settings);
    let mult = app.multiplier();
    let rates = app.rates();
    let mut sections = Vec::new();
    if let Some(w) = &pet.wiki {
        sections.push(("REFERENCE".into(), format!("Unlock: {}\nEvolution difficulty: {} ({})\nToken improvable: {}\nClass bonus: {}",
            format_unlock_condition(&w.unlock_condition), w.evo_difficulty.base, w.evo_difficulty.with_conditions,
            if w.token_improvable { "Yes" } else { "No" }, w.class_bonus)));
        if let Some(ability) = &w.special_ability { sections.push(("SPECIAL ABILITY".into(), ability.clone())); }
    }
    if let Some(e) = &pet.export {
        sections.push(("YOUR PET".into(), format!("{} · CL {} · DL {}\n{}\nToken improved: {}\n{}\nWith PGC: {}\nWith PGC + Magic Egg: {}{}",
            e.class.map(|c| format!("{c:?}")).unwrap_or_else(|| "Unevolved".into()), e.class_level, e.dungeon_level,
            app.action_text(pet), if e.improved { "Yes" } else { "No" },
            if app.session.roster_from_save { "Combat stats: unavailable in save".into() } else { format!("HP {} / ATK {} / DEF {} / SPD {}", e.combat_stats.hp, e.combat_stats.attack, e.combat_stats.defense, e.combat_stats.speed) },
            format_number(displayed_growth(e.growth, mult)), format_number(e.growth_with_magic_egg_and_global_mult(mult)),
            pet.elemental_form().map(|f| format!("\nForm: {}V{}", f.name, f.version)).unwrap_or_default())));
        let equipment: Vec<_> = [("Weapon", &e.loadout.weapon), ("Armor", &e.loadout.armor), ("Accessory", &e.loadout.accessory)].into_iter()
            .filter_map(|(slot,item)| item.as_ref().map(|i| format!("{slot}: {} ({:?})", i.name, i.quality))).collect();
        sections.push(("EQUIPMENT".into(), if equipment.is_empty() { "None equipped".into() } else { equipment.join("\n") }));
    }
    if let Some(req) = pet.wiki.as_ref().and_then(|w| w.evo_requirements.as_ref()) {
        let basis = if req.growth.requires_base_growth() { "Base growth" } else { "Total growth" };
        let mut text = format!("{basis}: {}", growth_threshold_text(&req.growth, mult));
        if let Some(m) = &req.material { text.push_str(&format!("\nMaterial: {m}")); }
        if let Some(o) = &req.other { text.push_str(&format!("\nOther: {o}")); }
        if let Some(e) = pet.export.as_ref().filter(|e| e.class.is_none()) {
            text.push_str(&format!("\n\n{}\nNo egg: {}\nWith egg: {}", growth_needed_text(&req.growth, e.growth, mult),
                eta(pet.hours_to_evolve_with_growth_mult(&rates, false, mult)), eta(pet.hours_to_evolve_with_growth_mult(&rates, true, mult))));
            let target = if req.growth.requires_base_growth() { req.growth.value().max(0) as u64 } else { base_growth_for_displayed_target(req.growth.value().max(0) as u64, mult) };
            text.push_str(&cap_note(app, e.growth, target));
        }
        sections.push(("EVOLUTION REQUIREMENTS".into(), text));
    }
    if let Some(plan) = pet.elemental_evo_plan() {
        let mut text = format!("Form V{}\nMinimum growth this form: {}\nRemaining form gains: +{}\nProjected final growth: {}\n{}",
            plan.form, format_signed(plan.min_growth_for_form), format_signed(plan.remaining_form_gain),
            format_signed(plan.projected_final_growth), if plan.on_track { "On track for evolution".into() } else { format!("{} growth short of being on track", format_signed(plan.shortfall)) });
        if let Some(gain) = plan.next_upgrade_gain { text.push_str(&format!("\nNext form upgrade: +{}", format_signed(gain))); }
        sections.push(("ELEMENTAL FORM PROGRESS".into(), text));
    }
    if let Some(plan) = pet.aether_evo_plan(context.inputs.challenge_points, context.inputs.delirious_essence_fights) {
        let mut text = format!("+{} growth per Delirious Essence fight\n{} fights remaining at {} challenge points",
            format_number(plan.growth_per_fight.round() as u64), plan.fights_remaining, format_number(plan.challenge_points));
        if let Some(chp) = plan.chp_to_evolve_estimate { text.push_str(&format!("\n~{} challenge points for remaining fights to reach 55,555 (rough estimate; other growth held static)", format_number(chp.round().max(0.0) as u64))); }
        sections.push(("AETHER GROWTH".into(), text));
    }
    let breakdown = pet.campaign_bonus_breakdown(&context);
    let mut entries: Vec<_> = breakdown.total().into_iter().collect();
    entries.sort_by(|a,b| b.1.total_cmp(&a.1).then_with(|| campaign_label(a.0).cmp(campaign_label(b.0))));
    let mut text = entries.into_iter().map(|(c,v)| format!("{}: {v:+.2}%", campaign_label(c))).collect::<Vec<_>>().join("\n");
    if let Some(v) = breakdown.class { text.push_str(&format!("\nIncludes class: {v:+.2}% to every campaign")); }
    if let Some(v) = breakdown.equipment { text.push_str(&format!("\nIncludes equipment: {v:+.2}% to every campaign")); }
    if breakdown.class.is_some() || breakdown.equipment.is_some() {
        let innate = ALL_CAMPAIGNS.into_iter().filter_map(|c| breakdown.innate.get(&c).map(|v| format!("{} {v:+.2}%", campaign_label(c)))).collect::<Vec<_>>().join(", ");
        text.push_str(&format!("\nInnate: {}", if innate.is_empty() { "None" } else { &innate }));
    }
    if let Some(raw) = pet.wiki.as_ref().and_then(|w| w.campaign_bonus.as_ref()) { text.push_str(&format!("\n\nWiki: {}", raw.raw)); }
    if !text.is_empty() { sections.push(("CAMPAIGN BONUSES".into(), text)); }
    sections
}

pub fn custom_target(app: &AppModel, pet: &MergedPet) -> String {
    let Some(export) = &pet.export else { return "Import pet data for an estimate.".into(); };
    let Some(target) = parse_growth_target(&app.session.analysis.custom_target) else { return "Enter a displayed-growth target to compare estimates.".into(); };
    let rates = app.rates();
    let base = base_growth_for_displayed_target(target, app.multiplier());
    let egg_base = base_growth_for_displayed_target(target, app.multiplier() * MAGIC_EGG_GROWTH_MULT);
    format!("No egg: {}\nWith egg: {}{}\nAssumes a dedicated pendant + your Moai; excludes feeding and campaigns.",
        eta(rates.hours_to_target(export.growth, base)), eta(rates.hours_to_target(export.growth, egg_base)), cap_note(app, export.growth, base))
}

fn cap_note(app: &AppModel, current: u64, target: u64) -> String {
    if current >= target { return String::new(); }
    let rates = app.rates();
    match rates.cap_relation(current, target) {
        CapRelation::BelowCap => String::new(),
        CapRelation::CrossesCap { hours_to_cap } => format!("\nPendant cap {}: {}. Above the cap, growth is Moai-only.", format_number(rates.pendant_cap), eta(hours_to_cap)),
        CapRelation::AboveCap => format!("\nAlready at/above pendant cap {}. Growth is Moai-only.", format_number(rates.pendant_cap)),
    }
}

pub fn rates_summary(app: &AppModel) -> String {
    let rates = app.rates();
    format!("Pendant: {} base growth/hr · Moai: {:.2}/hr · Pendant cap: {}\nEstimates assume a dedicated pendant and your Moai. Feeding and campaigns are excluded.",
        rates.evolved_pets, rates.moai_per_hour.abs(), if rates.pendant_cap == u64::MAX { "Unbounded (<10 pets)".into() } else { format_number(rates.pendant_cap) })
}

pub fn ranking_value(app: &AppModel, pet: &MergedPet) -> String {
    let settings = app.settings();
    match app.session.sort {
        SortColumn::TimeToEvolve => eta(pet.hours_to_evolve_with_growth_mult(&app.rates(), settings.evolve_sort_use_egg, app.multiplier())),
        SortColumn::TimeToTarget => eta(pet.hours_to_growth_with_mult(settings.global_growth_target, &app.rates(), app.multiplier())),
        _ => settings.filter_campaign.and_then(|c| pet.campaign_bonus_for(c, &app.campaign_context(&settings))).map(|v| format!("{v:+.2}%")).unwrap_or_else(|| "—".into()),
    }
}

pub fn row_status(app: &AppModel, pet: &MergedPet) -> String {
    let ownership = match &pet.export { Some(e) if e.unlocked => "Owned", Some(_) => "Locked", None => "No export" };
    let improved = if pet.export.as_ref().is_some_and(|e| e.improved) { " · Improved" }
        else if pet.wiki.as_ref().is_some_and(|w| w.token_improvable) { " · Token improvable" } else { "" };
    format!("{ownership} · {}{improved}", app.readiness(pet))
}

pub fn earth_eater_hint(app: &AppModel) -> String {
    let settings = app.settings();
    earth_eater_lock_hours(settings.campaign_inputs.earth_eater_total_planets as f64,
        settings.campaign_inputs.earth_eater_show_lifetime)
        .map(|hours| format!("Earth Eater: ~{} to permanent +82% lock at 1 planet/sec", format_duration(hours)))
        .unwrap_or_default()
}
