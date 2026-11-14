//! Shared pure dungeon presentation helpers.
use crate::solver::{Assignment, DungeonPlan};
use itrtg_models::dungeon::EquipmentCatalog;

pub fn team_export_name(pet: &crate::merge::MergedPet) -> String {
    pet.export
        .as_ref()
        .map(|e| e.export_name.clone())
        .unwrap_or_else(|| pet.name.replace(' ', ""))
}

pub fn build_dungeon_teams_export(plans: &[DungeonPlan]) -> String {
    let teams: Vec<Vec<(String, u8)>> = plans
        .iter()
        .map(|plan| {
            plan.assignments
                .iter()
                .filter_map(|a| match &a.assignment {
                    Assignment::Filled { pet, .. } => {
                        Some((team_export_name(pet), (a.position + 1) as u8))
                    }
                    Assignment::Empty { .. } => None,
                })
                .collect()
        })
        .collect();
    format_dungeon_teams(&teams)
}

pub fn format_dungeon_teams(teams: &[Vec<(String, u8)>]) -> String {
    let mut out = String::from("---DungeonTeamsStart---\n");
    let mut team_index = 0u8;
    for team in teams {
        if team.is_empty() {
            continue;
        }
        let mut members = team.clone();
        members.sort_by_key(|(_, slot)| *slot);
        out.push_str(&format!("{team_index}:"));
        for (name, slot) in &members {
            out.push_str(&format!("{name}={slot},"));
        }
        out.push(';');
        team_index += 1;
    }
    out.push_str("---DungeonTeamsEnd---");
    out
}

pub fn equip_matches_rec(
    current_name: &str,
    rec_key: &str,
    catalog: Option<&EquipmentCatalog>,
) -> bool {
    let Some(cat) = catalog else {
        // No catalog: fall back to name substring match
        let cur_lower = current_name.to_lowercase();
        let rec_name = rec_key.replace('_', " ").to_lowercase();
        return cur_lower.contains(&rec_name) || rec_name.contains(&cur_lower);
    };

    // Find the current equipment's catalog key by name
    if let Some(cur_key) = cat.find_key_by_name_exact(current_name) {
        // Check if it's the same item or an upgrade of the recommendation
        if cat.is_same_line(cur_key, rec_key) {
            return true;
        }
    }

    // Fallback: name substring match (handles items not in catalog)
    let rec_name = resolve_equip_name(rec_key, catalog).to_lowercase();
    let cur_lower = current_name.to_lowercase();
    cur_lower.contains(&rec_name) || rec_name.contains(&cur_lower)
}

pub fn resolve_equip_name(key: &str, catalog: Option<&EquipmentCatalog>) -> String {
    if let Some(cat) = catalog
        && let Some(entry) = cat.lookup(key)
    {
        return entry.name.clone();
    }
    // Humanize generic keys: "generic_t2_s10" → "Generic T2"
    if let Some(rest) = key.strip_prefix("generic_t") {
        let tier: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !tier.is_empty() {
            return format!("Generic T{tier}");
        }
    }
    // Fallback: humanize the key
    key.replace('_', " ")
}
