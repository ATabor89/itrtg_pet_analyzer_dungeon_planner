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

    if let Some(rest) = trimmed.strip_prefix("Defeat P.Baal v")
        && let Ok(n) = rest.trim().parse::<u32>()
    {
        return UnlockCondition::DefeatPBaalVersion(n);
    }

    if let Some(rest) = trimmed.strip_prefix("Defeat P.Baal ")
        && let Ok(n) = rest.trim().parse::<u32>()
    {
        return UnlockCondition::DefeatPBaal(n);
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
            // Populated separately by crawling each pet's page (see
            // `parse_evo_requirements`); the main table has no evo data.
            evo_requirements: None,
        });
    }

    Ok(pets)
}

/// Parse the "Evolution Requirements" block from a pet page's *rendered* HTML.
///
/// The infobox renders three labelled rows — "Total Growth" (the growth
/// threshold), "Material", and "Other" — as `<b>Label</b>` cells each followed
/// by a value cell. We scope the search to text *after* the "Evolution
/// Requirements" marker so we don't pick up the pet's own "Total Growth" stat
/// row higher in the infobox. Materials are template-computed and only exist in
/// the rendered page, which is why this works on HTML rather than raw wikitext.
///
/// Returns `None` if the block or a parseable growth threshold is absent.
pub fn parse_evo_requirements(html: &str) -> Option<EvoRequirements> {
    let marker = html.find("Evolution Requirements")?;
    let scope = &html[marker..];

    // The value may carry trailing text (e.g. Baby Carno's "300000 base
    // growth"), so pull the leading integer rather than parsing the whole cell.
    let total_growth = evo_field(scope, "Total Growth").and_then(|v| parse_leading_int(&v))?;

    Some(EvoRequirements {
        total_growth,
        material: evo_field(scope, "Material").and_then(meaningful),
        other: evo_field(scope, "Other").and_then(meaningful),
    })
}

/// Extract the first integer from a value cell, allowing thousands separators
/// and ignoring any trailing words (e.g. "300000 base growth" -> 300000).
fn parse_leading_int(s: &str) -> Option<i64> {
    let digits: String = s
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == ',')
        .filter(|c| *c != ',')
        .collect();
    digits.parse::<i64>().ok()
}

/// Drop empty or "none"/"-" placeholder values so display fields stay clean.
fn meaningful(s: String) -> Option<String> {
    let t = s.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("none") || t == "-" {
        None
    } else {
        Some(s)
    }
}

/// Find an evolution-requirement value by its bold label, returning the cleaned
/// text of the immediately following table cell.
fn evo_field(scope: &str, label: &str) -> Option<String> {
    // <b>Label</b> </td> <td ...> VALUE </td>
    let pattern = format!(
        r"(?s)<b>\s*{}\s*</b>\s*</td>\s*<td[^>]*>(.*?)</td>",
        regex::escape(label)
    );
    let re = Regex::new(&pattern).ok()?;
    let cap = re.captures(scope)?;
    Some(clean_html_value(cap.get(1)?.as_str()))
}

/// Strip HTML tags, decode the few entities that show up in pet infoboxes, and
/// collapse whitespace.
fn clean_html_value(s: &str) -> String {
    let tags = Regex::new(r"<[^>]*>").unwrap();
    let text = tags.replace_all(s, "");
    let text = text
        .replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&#039;", "'")
        .replace("&quot;", "\"")
        .replace("&nbsp;", " ");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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

    // Mirrors the real rendered infobox: the pet's own "Total Growth" stat row
    // appears *before* the Evolution Requirements block, which has its own
    // "Total Growth" (the threshold). The parser must pick the latter.
    const MOUSE_EVO_HTML: &str = r#"
        <tr><td colspan="2"><b>Total Growth</b></td><td colspan="2">420</td></tr>
        <tr><td colspan="4"><i><b>Evolution Requirements</b></i></td></tr>
        <tr><td colspan="4" rowspan="3">Defeat <a href="/wiki/Gods">Hyperion</a></td>
        <td colspan="2"><b>Total Growth</b></td><td colspan="2">100</td></tr>
        <tr><td colspan="2"><b>Material</b></td><td colspan="2">5 Wood</td></tr>
        <tr><td colspan="2"><b>Other</b></td><td colspan="2">100 Puny Food</td></tr>
    "#;

    #[test]
    fn test_parse_evo_requirements_basic() {
        let evo = parse_evo_requirements(MOUSE_EVO_HTML).expect("should parse");
        assert_eq!(evo.total_growth, 100, "must use the Evolution Requirements threshold, not the 420 stat");
        assert_eq!(evo.material.as_deref(), Some("5 Wood"));
        assert_eq!(evo.other.as_deref(), Some("100 Puny Food"));
    }

    #[test]
    fn test_parse_evo_requirements_strips_links_and_commas() {
        // Sylph-style: large threshold and a wikilinked questline in "Other".
        let html = r#"
            <td colspan="4"><i><b>Evolution Requirements</b></i></td></tr>
            <tr><td colspan="2"><b>Total Growth</b></td><td colspan="2">55,555</td></tr>
            <tr><td colspan="2"><b>Material</b></td><td colspan="2">2778 <a href="/wiki/Bound_Feather">Bound Feather</a></td></tr>
            <tr><td colspan="2"><b>Other</b></td><td colspan="2">Finish the <a href="/wiki/Sylph">Questline</a> (You <b>cannot</b> use a Pet Token)</td></tr>
        "#;
        let evo = parse_evo_requirements(html).expect("should parse");
        assert_eq!(evo.total_growth, 55555);
        assert_eq!(evo.material.as_deref(), Some("2778 Bound Feather"));
        assert_eq!(
            evo.other.as_deref(),
            Some("Finish the Questline (You cannot use a Pet Token)")
        );
    }

    #[test]
    fn test_parse_evo_requirements_absent() {
        assert!(parse_evo_requirements("<p>no infobox here</p>").is_none());
    }

    #[test]
    fn test_parse_evo_requirements_base_growth_suffix_and_none() {
        // Baby Carno: threshold cell carries trailing "base growth" markup, and
        // its "Other" is the literal "none".
        let html = r#"
            <td colspan="4"><i><b>Evolution Requirements</b></i></td></tr>
            <tr><td colspan="2"><b>Total Growth</b></td><td colspan="2">300000 <b>base growth</b></td></tr>
            <tr><td colspan="2"><b>Material</b></td><td colspan="2">3000 Magic Ore</td></tr>
            <tr><td colspan="2"><b>Other</b></td><td colspan="2">none</td></tr>
        "#;
        let evo = parse_evo_requirements(html).expect("should parse despite suffix");
        assert_eq!(evo.total_growth, 300000);
        assert_eq!(evo.material.as_deref(), Some("3000 Magic Ore"));
        assert_eq!(evo.other, None, "literal 'none' should be dropped");
    }

    #[test]
    fn test_parse_recommended_class_village() {
        match parse_recommended_class("Village (Fisher)") {
            RecommendedClass::Village(role) => assert_eq!(role, "Fisher"),
            other => panic!("Expected Village(Fisher), got {:?}", other),
        }
    }

    /// One-shot helper: fetch wiki, parse pets, write to data/wiki_pets.yaml.
    /// Run with: cargo test -p wiki-extractor --features cli generate_wiki_pets_yaml -- --ignored
    #[test]
    #[ignore]
    #[cfg(feature = "cli")]
    fn generate_wiki_pets_yaml() {
        let url = "https://itrtg.wiki.gg/wiki/Pets?action=raw";
        let client = reqwest::blocking::Client::builder()
            .user_agent("pet_extractor/0.1.0 (ITRTG tool)")
            .build()
            .expect("failed to build HTTP client");
        let resp = client.get(url).send().expect("failed to fetch wiki");
        assert!(resp.status().is_success(), "HTTP {}", resp.status());
        let source = resp.text().expect("failed to read response body");

        let pets = parse_pets(&source).expect("failed to parse wiki pets");
        println!("Parsed {} pets from wiki", pets.len());

        let yaml = serde_yaml::to_string(&pets).expect("failed to serialize to YAML");
        std::fs::write("../../data/wiki_pets.yaml", &yaml)
            .expect("failed to write data/wiki_pets.yaml");
        println!("Wrote data/wiki_pets.yaml ({} bytes)", yaml.len());
    }
}
