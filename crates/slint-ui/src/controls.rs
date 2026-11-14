//! Analyzer controls with stable keys; callbacks never depend on visual row order.
use itrtg_models::parse_flexible_number;
use itrtg_planner::analyzer::*;
use crate::app::{AppModel, Sort};

pub const SORTS: [Sort; 12] = [Sort::Name, Sort::Growth, Sort::DungeonLevel, Sort::ClassLevel,
    Sort::Element, Sort::RecClass, Sort::EvoDifficulty, Sort::Class, Sort::Action,
    Sort::TimeToEvolve, Sort::TimeToTarget, Sort::CampaignBonus];
const UNLOCK: [UnlockTypeFilter; 13] = [UnlockTypeFilter::All, UnlockTypeFilter::DefeatGods,
    UnlockTypeFilter::PBaal, UnlockTypeFilter::PetToken, UnlockTypeFilter::Milestones,
    UnlockTypeFilter::Special, UnlockTypeFilter::Secret, UnlockTypeFilter::TavernQuest,
    UnlockTypeFilter::StrategyRoom, UnlockTypeFilter::DungeonBoss, UnlockTypeFilter::PetCount,
    UnlockTypeFilter::ItemGift, UnlockTypeFilter::AncientMimic];
const RECOMMENDED: [RecClassFilter; 14] = [RecClassFilter::All, RecClassFilter::Adventurer,
    RecClassFilter::Blacksmith, RecClassFilter::Alchemist, RecClassFilter::Defender,
    RecClassFilter::Supporter, RecClassFilter::Rogue, RecClassFilter::Assassin, RecClassFilter::Mage,
    RecClassFilter::Wildcard, RecClassFilter::DungeonWildcard, RecClassFilter::AllClasses,
    RecClassFilter::Village, RecClassFilter::Special];
const CLASS: [MyClassFilter; 11] = [MyClassFilter::All, MyClassFilter::Unevolved,
    MyClassFilter::Adventurer, MyClassFilter::Blacksmith, MyClassFilter::Alchemist,
    MyClassFilter::Defender, MyClassFilter::Supporter, MyClassFilter::Rogue,
    MyClassFilter::Assassin, MyClassFilter::Mage, MyClassFilter::Wildcard];
const IMPROVED: [ImprovableFilter; 4] = [ImprovableFilter::All, ImprovableFilter::Improvable,
    ImprovableFilter::Improved, ImprovableFilter::NotImproved];

pub struct Choice {
    pub key: &'static str, pub label: &'static str, pub options: Vec<String>, pub current: usize,
}
fn position<T: PartialEq>(list: &[T], value: &T) -> usize { list.iter().position(|v| v == value).unwrap_or(0) }
pub fn choices(app: &AppModel) -> Vec<Choice> {
    let a = &app.session.analysis;
    vec![
        Choice { key: "evolved", label: "Evolution", options: ["All", "Evolved", "Unevolved"].map(String::from).to_vec(), current: match a.filter_evolved { None => 0, Some(true) => 1, Some(false) => 2 } },
        Choice { key: "unlock", label: "Unlock method", options: UNLOCK.map(|v| v.label().into()).to_vec(), current: position(&UNLOCK, &a.filter_unlock_type) },
        Choice { key: "recommended", label: "Recommended class", options: RECOMMENDED.map(|v| v.label().into()).to_vec(), current: position(&RECOMMENDED, &a.filter_rec_class) },
        Choice { key: "class", label: "Your class", options: CLASS.map(|v| v.label().into()).to_vec(), current: position(&CLASS, &a.filter_my_class) },
        Choice { key: "improved", label: "Token improvement", options: IMPROVED.map(|v| v.label().into()).to_vec(), current: position(&IMPROVED, &a.filter_improvable) },
        Choice { key: "campaign", label: "Campaign bonus > 0", options: std::iter::once("All".into()).chain(ALL_CAMPAIGNS.map(|c| campaign_label(c).into())).collect(), current: a.filter_campaign.map(|c| position(&ALL_CAMPAIGNS, &c) + 1).unwrap_or(0) },
    ]
}

pub fn change_choice(app: &mut AppModel, key: &str, index: usize) {
    let a = &mut app.session.analysis;
    match key {
        "evolved" => a.filter_evolved = [None, Some(true), Some(false)].get(index).copied().flatten(),
        "unlock" => a.filter_unlock_type = UNLOCK.get(index).copied().unwrap_or_default(),
        "recommended" => a.filter_rec_class = RECOMMENDED.get(index).copied().unwrap_or_default(),
        "class" => a.filter_my_class = CLASS.get(index).copied().unwrap_or_default(),
        "improved" => a.filter_improvable = IMPROVED.get(index).copied().unwrap_or_default(),
        "campaign" => a.filter_campaign = index.checked_sub(1).and_then(|i| ALL_CAMPAIGNS.get(i).copied()),
        "tiebreak" => a.time_sort_tiebreak = if index == 1 { TimeSortTiebreak::EvoDifficulty } else { TimeSortTiebreak::Growth },
        _ => {},
    }
    normalize_sort(app);
    app.reconcile_selection();
}

pub fn normalize_sort(app: &mut AppModel) {
    if (app.session.sort == Sort::TimeToTarget && app.session.analysis.global_growth_target == 0)
        || (app.session.sort == Sort::CampaignBonus && app.session.analysis.filter_campaign.is_none()) {
        app.session.sort = Sort::Name;
        app.session.ascending = None;
    }
}

pub struct NumberInput {
    pub key: &'static str, pub label: &'static str, pub value: String, pub hint: &'static str,
}
pub fn numbers(app: &AppModel) -> Vec<NumberInput> {
    let a = &app.session.analysis;
    let ci = &a.campaign_inputs;
    let rows = [
        ("target", "Displayed-growth target", a.global_growth_target, "0 = unset; used by time-to-target sorting"),
        ("moai1", "Moai #1 level", if a.moai[0].owned { a.moai[0].level as u64 } else { 0 }, "0 = not owned; levels 1–20"),
        ("moai2", "Moai #2 level", if a.moai[1].owned { a.moai[1].level as u64 } else { 0 }, "0 = not owned; levels 1–20"),
        ("stones", "Pet stones held", ci.pet_stones, "Beachball"),
        ("given", "Stones given to Beachball", ci.beachball_given_stones, "Permanent contribution"),
        ("chp", "Challenge points", ci.challenge_points, "Unicorn / Aether"),
        ("honey", "Honey given", ci.honey, "Bear"),
        ("ants", "Ants", ci.ants, "Ant Queen"),
        ("meteor", "Meteor campaign hours", ci.meteor_campaign_hours as u64, "Whole hours"),
        ("fights", "Delirious Essence fights", ci.delirious_essence_fights as u64, "Aether"),
        ("ucc", "UCCs completed", ci.goblin_ucc as u64, "Goblin; campaign bonus capped at 75"),
        ("oc", "Overflow Challenges", ci.goblin_oc as u64, "Goblin; evolution bonus capped at 470"),
    ];
    let mut values: Vec<_> = rows.into_iter().map(|(key,label,value,hint)| NumberInput { key, label, value: value.to_string(), hint }).collect();
    for (index, key) in ["moai1", "moai2"].into_iter().enumerate() {
        if a.moai[index].owned && a.moai[index].level == 0
            && let Some(input) = values.iter_mut().find(|n| n.key == key) {
            input.hint = "Imported owned level 0; applying 0 marks it unowned";
        }
    }
    values.push(NumberInput { key: "planets", label: "Earth Eater total planets", value: a.earth_eater_planets_text.clone(), hint: "Accepts 32.4e6; used when the +82% lock is off" });
    values
}

/// Commit valid inputs only, so partial typing cannot replace a saved value.
pub fn change_number(app: &mut AppModel, key: &str, text: &str) -> Result<(), String> {
    let value = if text.trim().is_empty() { 0.0 } else { parse_flexible_number(text).ok_or("Enter a finite number (for example 32.4e6).")? };
    if value < 0.0 || value >= u64::MAX as f64 { return Err("Enter a nonnegative number below 18,446,744,073,709,551,615.".into()); }
    if ["moai1", "moai2"].contains(&key) && (value > 20.0 || value.fract() != 0.0) {
        return Err("Moai level must be a whole number from 0 to 20 (0 = not owned).".into());
    }
    if ["meteor", "fights", "ucc", "oc"].contains(&key) && value > u32::MAX as f64 {
        return Err("This count exceeds the supported maximum (4,294,967,295).".into());
    }
    let v = value as u64;
    let a = &mut app.session.analysis;
    match key {
        "target" => a.global_growth_target = v,
        "moai1" | "moai2" => { let m = &mut a.moai[usize::from(key == "moai2")]; m.owned = v > 0; if m.owned { m.level = v as u8; } },
        "stones" => a.campaign_inputs.pet_stones = v,
        "given" => a.campaign_inputs.beachball_given_stones = v,
        "chp" => a.campaign_inputs.challenge_points = v,
        "honey" => a.campaign_inputs.honey = v,
        "ants" => a.campaign_inputs.ants = v,
        "meteor" => a.campaign_inputs.meteor_campaign_hours = v as u32,
        "fights" => a.campaign_inputs.delirious_essence_fights = v as u32,
        "ucc" => a.campaign_inputs.goblin_ucc = v as u32,
        "oc" => a.campaign_inputs.goblin_oc = v as u32,
        "planets" => a.earth_eater_planets_text = text.into(),
        "custom" => a.custom_target = text.into(),
        _ => return Err("Unknown setting".into()),
    }
    normalize_sort(app);
    app.reconcile_selection();
    Ok(())
}

pub fn change_flag(app: &mut AppModel, key: &str, value: bool) {
    let a = &mut app.session.analysis;
    match key {
        "egg" => a.evolve_sort_use_egg = value,
        "equipment" => a.include_equipment_bonus = value,
        "class" => a.include_class_bonus = value,
        "stone" => a.campaign_inputs.stone_campaign_upgrade = value,
        "earth-lock" => a.campaign_inputs.earth_eater_show_lifetime = !value,
        "ascending" => app.session.ascending = Some(value),
        _ => {},
    }
    app.reconcile_selection();
}
