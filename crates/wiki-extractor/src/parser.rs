use regex::Regex;

use itrtg_models::*;

const WIKI_BASE: &str = "https://itrtg.wiki.gg/wiki/";

/// Wiki page slug overrides for pets whose page name differs from display name.
fn wiki_slug(name: &str) -> String {
    match name {
        "Student" => "Student_(pet)".to_string(),
        "Elemental" => "Elemental_(Pet)".to_string(),
        "Lizard/Zookeeper" => "Lizard".to_string(),
        _ => name.replace(' ', "_"),
    }
}

fn wiki_url(name: &str) -> String {
    format!("{}{}", WIKI_BASE, wiki_slug(name))
}

/// Extract the pet name from the wiki name cell.
/// Handles patterns like: [[Mouse]], [[Egg/Chicken]], [[Pandora's Box]],
/// [[Elemental (Pet)|Elemental]], [[Lizard|Lizard/Zookeeper]], [[Student (pet)|Student]]
fn parse_name(cell: &str) -> Option<String> {
    let re = Regex::new(r"\[\[([^\]|]+?)(?:\|([^\]]+))?\]\]").unwrap();
    for cap in re.captures_iter(cell) {
        let target = cap.get(1).unwrap().as_str();
        if target.to_lowercase().starts_with("file:") {
            continue;
        }
        if let Some(alias) = cap.get(2) {
            return Some(alias.as_str().trim().to_string());
        }
        return Some(target.trim().to_string());
    }
    None
}

fn parse_element(cell: &str) -> Element {
    let lower = cell.to_lowercase();
    if lower.contains("fire") {
        Element::Fire
    } else if lower.contains("water") {
        Element::Water
    } else if lower.contains("wind") {
        Element::Wind
    } else if lower.contains("earth") {
        Element::Earth
    } else if lower.contains("neutral") {
        Element::Neutral
    } else if lower.contains("all") || lower.trim() == "all" {
        Element::All
    } else {
        Element::Neutral
    }
}

fn parse_single_class(s: &str) -> Option<Class> {
    match s.trim().to_lowercase().as_str() {
        "adventurer" => Some(Class::Adventurer),
        "blacksmith" => Some(Class::Blacksmith),
        "alchemist" => Some(Class::Alchemist),
        "defender" => Some(Class::Defender),
        "supporter" => Some(Class::Supporter),
        "rogue" => Some(Class::Rogue),
        "assassin" => Some(Class::Assassin),
        "mage" => Some(Class::Mage),
        _ => None,
    }
}

fn parse_recommended_class(cell: &str) -> RecommendedClass {
    let re_sup = Regex::new(r"<sup>\d+</sup>").unwrap();
    let cleaned = re_sup.replace_all(cell, "").trim().to_string();

    if cleaned == "Special" {
        return RecommendedClass::Special;
    }
    if cleaned == "Alternates" {
        return RecommendedClass::Alternates;
    }
    if cleaned == "All Classes" {
        return RecommendedClass::AllClasses;
    }
    if cleaned == "Dungeon Wildcard" {
        return RecommendedClass::DungeonWildcard;
    }
    if cleaned == "Tavern Wildcard" {
        return RecommendedClass::Village("Tavern".to_string());
    }
    if cleaned == "Wildcard" {
        return RecommendedClass::Wildcard;
    }

    if let Some(rest) = cleaned.strip_prefix("Village") {
        let role = rest
            .trim()
            .trim_start_matches('(')
            .trim_end_matches(')')
            .trim()
            .to_string();
        return RecommendedClass::Village(role);
    }

    if cleaned.contains('/') {
        let parts: Vec<&str> = cleaned.split('/').collect();
        if parts.len() == 2 {
            let a = parts[0].trim();
            let b = parts[1].trim();

            if b.to_lowercase() == "wildcard" {
                if let Some(cls) = parse_single_class(a) {
                    return RecommendedClass::Dual(cls, Class::Wildcard);
                }
                return RecommendedClass::Wildcard;
            }
            if a.to_lowercase() == "wildcard" {
                if let Some(cls) = parse_single_class(b) {
                    return RecommendedClass::Dual(cls, Class::Wildcard);
                }
                return RecommendedClass::Wildcard;
            }

            if let (Some(c1), Some(c2)) = (parse_single_class(a), parse_single_class(b)) {
                return RecommendedClass::Dual(c1, c2);
            }
        }
    }

    let no_sup = Regex::new(r"<sup>.*?</sup>").unwrap();
    let final_cleaned = no_sup.replace_all(&cleaned, "").trim().to_string();

    if let Some(cls) = parse_single_class(&final_cleaned) {
        return RecommendedClass::Single(cls);
    }

    eprintln!("Warning: unrecognized class '{}', treating as Wildcard", cleaned);
    RecommendedClass::Wildcard
}

fn parse_unlock_condition(cell: &str) -> UnlockCondition {
    let trimmed = cell.trim();

    if trimmed == "Defeat Gods" {
        return UnlockCondition::DefeatGods;
    }

    if let Some(rest) = trimmed.strip_prefix("Defeat P.Baal v") {
        if let Ok(n) = rest.trim().parse::<u32>() {
            return UnlockCondition::DefeatPBaalVersion(n);
        }
    }

    if let Some(rest) = trimmed.strip_prefix("Defeat P.Baal ") {
        if let Ok(n) = rest.trim().parse::<u32>() {
            return UnlockCondition::DefeatPBaal(n);
        }
    }

    if trimmed == "Special Task" {
        return UnlockCondition::SpecialTask;
    }

    if trimmed == "Pet Token" {
        return UnlockCondition::PetToken;
    }

    if trimmed == "Secret" {
        return UnlockCondition::Secret;
    }

    if trimmed == "Special" {
        return UnlockCondition::Special;
    }

    let stripped_links = strip_wiki_links(trimmed);
    if stripped_links.contains("Milestones") && stripped_links.contains("Pet Token") {
        return UnlockCondition::MilestonesOrPetToken;
    }

    if stripped_links.trim() == "Milestones" {
        return UnlockCondition::Milestones;
    }

    if stripped_links.to_lowercase().contains("tavern") && stripped_links.to_lowercase().contains("quest") {
        let re = Regex::new(r"rank\s+(\S+)\s+quest").unwrap();
        if let Some(cap) = re.captures(&stripped_links) {
            return UnlockCondition::TavernQuest(cap[1].to_string());
        }
        return UnlockCondition::TavernQuest("SSS".to_string());
    }

    if stripped_links.to_lowercase().contains("strategy room") {
        let re = Regex::new(r"[Ll]evel\s+(\d+)").unwrap();
        if let Some(cap) = re.captures(&stripped_links) {
            return UnlockCondition::StrategyRoom(cap[1].parse().unwrap_or(0));
        }
    }

    if trimmed.to_lowercase().contains("ancient mimic points") {
        let re = Regex::new(r"(\d+)\s+ancient mimic points").unwrap();
        if let Some(cap) = re.captures(&trimmed.to_lowercase()) {
            return UnlockCondition::AncientMimicPoints(cap[1].parse().unwrap_or(0));
        }
    }

    if trimmed.to_lowercase().contains("pets unlocked") {
        let re = Regex::new(r"(\d+)\s+[Pp]ets\s+[Uu]nlocked").unwrap();
        if let Some(cap) = re.captures(trimmed) {
            return UnlockCondition::PetCount(cap[1].parse().unwrap_or(0));
        }
    }

    if trimmed.to_lowercase().contains("defeat a d") && trimmed.to_lowercase().contains("boss") {
        let re = Regex::new(r"[Dd]efeat a (D\S+) [Bb]oss").unwrap();
        if let Some(cap) = re.captures(trimmed) {
            return UnlockCondition::DungeonBoss(cap[1].to_string());
        }
    }

    if trimmed.to_lowercase().starts_with("give it") {
        let gift = trimmed.strip_prefix("Give it").unwrap_or(trimmed).trim();
        return UnlockCondition::ItemGift(gift.to_string());
    }

    eprintln!("Warning: unrecognized unlock condition '{}', treating as SpecialTask", trimmed);
    UnlockCondition::SpecialTask
}

fn parse_evo_difficulty(cell: &str) -> EvoDifficulty {
    let trimmed = cell.trim();
    let re = Regex::new(r"(\d+)\((\d+)(?:-(\d+))?\)").unwrap();
    if let Some(cap) = re.captures(trimmed) {
        let base: u8 = cap[1].parse().unwrap_or(1);
        let cond: u8 = if let Some(high) = cap.get(3) {
            high.as_str().parse().unwrap_or(1)
        } else {
            cap[2].parse().unwrap_or(1)
        };
        return EvoDifficulty {
            base,
            with_conditions: cond,
        };
    }
    eprintln!("Warning: couldn't parse evo difficulty '{}', defaulting to 1(1)", trimmed);
    EvoDifficulty {
        base: 1,
        with_conditions: 1,
    }
}

fn parse_improve(cell: &str) -> bool {
    cell.to_lowercase().contains("yes")
}

fn parse_special_ability(cell: &str) -> Option<String> {
    let trimmed = cell.trim().trim_start_matches('-').trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_class_bonus(cell: &str) -> String {
    let re_sup = Regex::new(r"<sup>\d+</sup>").unwrap();
    let cleaned = re_sup.replace_all(cell.trim(), "");
    let cleaned = cleaned.trim().to_string();
    let cleaned = cleaned
        .replace("<nowiki>", "")
        .replace("</nowiki>", "");
    cleaned.trim().to_string()
}

/// Strip [[Link|Display]] to just "Display" or "Link" if no display.
fn strip_wiki_links(s: &str) -> String {
    let re = Regex::new(r"\[\[(?:[^\]|]*\|)?([^\]]+)\]\]").unwrap();
    let result = re.replace_all(s, "$1");
    result.replace("<br>", " ").replace("<br/>", " ").replace("<br />", " ")
}

/// Parse the full wiki source into a list of WikiPet structs.
pub fn parse_pets(source: &str) -> anyhow::Result<Vec<WikiPet>> {
    let mut pets = Vec::new();

    let table_start = source.find("{| class=\"wikitable");
    let table_end = source.rfind("|}");

    let (table_start, table_end) = match (table_start, table_end) {
        (Some(s), Some(e)) => (s, e),
        _ => anyhow::bail!("Could not find the pet table in the wiki source"),
    };

    let table = &source[table_start..=table_end + 1];
    let rows: Vec<&str> = table.split("\n|-").collect();

    for row in rows.iter().skip(1) {
        let row = row.trim();
        if row.is_empty() || row.starts_with("|}") {
            continue;
        }

        let cells = split_cells(row);

        if cells.len() < 10 {
            continue;
        }

        let name = match parse_name(&cells[1]) {
            Some(n) => n,
            None => {
                match parse_name(&cells[0]) {
                    Some(n) => n,
                    None => {
                        eprintln!("Warning: couldn't parse name from row, skipping: {:?}", &cells[..2.min(cells.len())]);
                        continue;
                    }
                }
            }
        };

        let element = parse_element(&cells[2]);
        let recommended_class = parse_recommended_class(&cells[3]);
        let class_bonus = parse_class_bonus(&cells[4]);
        let unlock_condition = parse_unlock_condition(&cells[5]);
        let evo_difficulty = parse_evo_difficulty(&cells[6]);
        let token_improvable = parse_improve(&cells[7]);
        let special_ability = parse_special_ability(&cells[9]);

        let url = wiki_url(&name);

        pets.push(WikiPet {
            name,
            wiki_url: url,
            element,
            recommended_class,
            class_bonus,
            unlock_condition,
            evo_difficulty,
            token_improvable,
            special_ability,
        });
    }

    Ok(pets)
}

/// Split a table row into individual cells.
fn split_cells(row: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut first = true;

    for line in row.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('|') && !trimmed.starts_with("||") {
            if !first {
                cells.push(current.trim().to_string());
            }
            first = false;
            let content = trimmed.trim_start_matches('|');
            let sub_cells: Vec<&str> = content.split("||").collect();
            if sub_cells.len() > 1 {
                current = sub_cells[0].trim().to_string();
                for sc in &sub_cells[1..] {
                    cells.push(current.trim().to_string());
                    current = sc.trim().to_string();
                }
            } else {
                current = content.trim().to_string();
            }
        } else if trimmed.starts_with("||") {
            let content = trimmed.trim_start_matches('|');
            let sub_cells: Vec<&str> = content.split("||").collect();
            cells.push(current.trim().to_string());
            current = sub_cells[0].trim().to_string();
            for sc in &sub_cells[1..] {
                cells.push(current.trim().to_string());
                current = sc.trim().to_string();
            }
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(trimmed);
        }
    }

    if !current.is_empty() {
        cells.push(current.trim().to_string());
    }

    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_name_simple() {
        assert_eq!(parse_name("[[Mouse]]"), Some("Mouse".to_string()));
    }

    #[test]
    fn test_parse_name_with_alias() {
        assert_eq!(
            parse_name("[[Elemental (Pet)|Elemental]]"),
            Some("Elemental".to_string())
        );
    }

    #[test]
    fn test_parse_name_slash() {
        assert_eq!(
            parse_name("[[Egg/Chicken]]"),
            Some("Egg/Chicken".to_string())
        );
    }

    #[test]
    fn test_parse_evo_difficulty() {
        let ed = parse_evo_difficulty("3(4-5)");
        assert_eq!(ed.base, 3);
        assert_eq!(ed.with_conditions, 5);
    }

    #[test]
    fn test_parse_evo_difficulty_simple() {
        let ed = parse_evo_difficulty("1(1)");
        assert_eq!(ed.base, 1);
        assert_eq!(ed.with_conditions, 1);
    }

    #[test]
    fn test_parse_recommended_class_dual() {
        match parse_recommended_class("Assassin/Adventurer") {
            RecommendedClass::Dual(Class::Assassin, Class::Adventurer) => {}
            other => panic!("Expected Dual(Assassin, Adventurer), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_recommended_class_village() {
        match parse_recommended_class("Village (Fisher)") {
            RecommendedClass::Village(role) => assert_eq!(role, "Fisher"),
            other => panic!("Expected Village(Fisher), got {:?}", other),
        }
    }
}
