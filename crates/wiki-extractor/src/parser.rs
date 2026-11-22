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

/// The pet's element, or `None` if the cell names none we know (including a
/// `?` placeholder). The caller falls back to Neutral and records a warning.
fn try_parse_element(cell: &str) -> Option<Element> {
    let lower = cell.to_lowercase();
    if lower.contains("fire") {
        Some(Element::Fire)
    } else if lower.contains("water") {
        Some(Element::Water)
    } else if lower.contains("wind") {
        Some(Element::Wind)
    } else if lower.contains("earth") {
        Some(Element::Earth)
    } else if lower.contains("neutral") {
        Some(Element::Neutral)
    } else if lower.contains("all") {
        Some(Element::All)
    } else {
        None
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

#[cfg(test)]
fn parse_recommended_class(cell: &str) -> RecommendedClass {
    try_parse_recommended_class(cell).unwrap_or(RecommendedClass::Wildcard)
}

/// The recommended class, or `None` if unrecognized (the caller falls back to
/// Wildcard and records a warning).
fn try_parse_recommended_class(cell: &str) -> Option<RecommendedClass> {
    let re_sup = Regex::new(r"<sup>\d+</sup>").unwrap();
    let cleaned = re_sup.replace_all(cell, "").trim().to_string();

    if cleaned == "Special" {
        return Some(RecommendedClass::Special);
    }
    if cleaned == "Alternates" {
        return Some(RecommendedClass::Alternates);
    }
    if cleaned == "All Classes" {
        return Some(RecommendedClass::AllClasses);
    }
    if cleaned == "Dungeon Wildcard" {
        return Some(RecommendedClass::DungeonWildcard);
    }
    if cleaned == "Tavern Wildcard" {
        return Some(RecommendedClass::Village("Tavern".to_string()));
    }
    if cleaned == "Wildcard" {
        return Some(RecommendedClass::Wildcard);
    }

    if let Some(rest) = cleaned.strip_prefix("Village") {
        let role = rest
            .trim()
            .trim_start_matches('(')
            .trim_end_matches(')')
            .trim()
            .to_string();
        return Some(RecommendedClass::Village(role));
    }

    if cleaned.contains('/') {
        let parts: Vec<&str> = cleaned.split('/').collect();
        if parts.len() == 2 {
            let a = parts[0].trim();
            let b = parts[1].trim();

            if b.to_lowercase() == "wildcard" {
                if let Some(cls) = parse_single_class(a) {
                    return Some(RecommendedClass::Dual(cls, Class::Wildcard));
                }
                return Some(RecommendedClass::Wildcard);
            }
            if a.to_lowercase() == "wildcard" {
                if let Some(cls) = parse_single_class(b) {
                    return Some(RecommendedClass::Dual(cls, Class::Wildcard));
                }
                return Some(RecommendedClass::Wildcard);
            }

            if let (Some(c1), Some(c2)) = (parse_single_class(a), parse_single_class(b)) {
                return Some(RecommendedClass::Dual(c1, c2));
            }
        }
    }

    let no_sup = Regex::new(r"<sup>.*?</sup>").unwrap();
    let final_cleaned = no_sup.replace_all(&cleaned, "").trim().to_string();

    if let Some(cls) = parse_single_class(&final_cleaned) {
        return Some(RecommendedClass::Single(cls));
    }

    None
}

/// The unlock condition, or `None` if unrecognized (the caller falls back to
/// SpecialTask and records a warning).
///
/// A cell listing alternatives ("Old Event Token or Pet Token", "[[Milestones]]
/// or Pet Token") becomes [`UnlockCondition::AnyOf`]; every alternative must
/// be recognized, or the whole cell counts as unrecognized.
fn try_parse_unlock_condition(cell: &str) -> Option<UnlockCondition> {
    // A whole-cell match wins, so ordinary text containing "or" (an item
    // gift, say) isn't split apart.
    if let Some(single) = try_parse_single_unlock(cell) {
        return Some(single);
    }
    let flat = strip_wiki_links(cell);
    let or_re = Regex::new(r"(?i)\s+or\s+").unwrap();
    let parts: Vec<&str> = or_re.split(flat.trim()).collect();
    if parts.len() > 1 {
        let methods = parts
            .iter()
            .map(|p| try_parse_single_unlock(p))
            .collect::<Option<Vec<_>>>()?;
        return Some(UnlockCondition::AnyOf(methods));
    }
    None
}

/// One unlock method (no "or").
fn try_parse_single_unlock(cell: &str) -> Option<UnlockCondition> {
    let trimmed = cell.trim();

    if trimmed == "Defeat Gods" {
        return Some(UnlockCondition::DefeatGods);
    }

    if let Some(rest) = trimmed.strip_prefix("Defeat P.Baal v")
        && let Ok(n) = rest.trim().parse::<u32>()
    {
        return Some(UnlockCondition::DefeatPBaalVersion(n));
    }

    if let Some(rest) = trimmed.strip_prefix("Defeat P.Baal ")
        && let Ok(n) = rest.trim().parse::<u32>()
    {
        return Some(UnlockCondition::DefeatPBaal(n));
    }

    if trimmed == "Special Task" {
        return Some(UnlockCondition::SpecialTask);
    }

    if trimmed == "Pet Token" {
        return Some(UnlockCondition::PetToken);
    }

    if trimmed == "Secret" {
        return Some(UnlockCondition::Secret);
    }

    if trimmed == "Special" {
        return Some(UnlockCondition::Special);
    }

    let stripped_links = strip_wiki_links(trimmed);
    if stripped_links.trim().eq_ignore_ascii_case("Old Event Token") {
        return Some(UnlockCondition::OldEventToken);
    }

    if stripped_links.trim() == "Milestones" {
        return Some(UnlockCondition::Milestones);
    }

    if stripped_links.to_lowercase().contains("tavern") && stripped_links.to_lowercase().contains("quest") {
        let re = Regex::new(r"rank\s+(\S+)\s+quest").unwrap();
        if let Some(cap) = re.captures(&stripped_links) {
            return Some(UnlockCondition::TavernQuest(cap[1].to_string()));
        }
        return Some(UnlockCondition::TavernQuest("SSS".to_string()));
    }

    if stripped_links.to_lowercase().contains("strategy room") {
        let re = Regex::new(r"[Ll]evel\s+(\d+)").unwrap();
        if let Some(cap) = re.captures(&stripped_links) {
            return Some(UnlockCondition::StrategyRoom(cap[1].parse().unwrap_or(0)));
        }
    }

    if trimmed.to_lowercase().contains("ancient mimic points") {
        let re = Regex::new(r"(\d+)\s+ancient mimic points").unwrap();
        if let Some(cap) = re.captures(&trimmed.to_lowercase()) {
            return Some(UnlockCondition::AncientMimicPoints(cap[1].parse().unwrap_or(0)));
        }
    }

    if trimmed.to_lowercase().contains("pets unlocked") {
        let re = Regex::new(r"(\d+)\s+[Pp]ets\s+[Uu]nlocked").unwrap();
        if let Some(cap) = re.captures(trimmed) {
            return Some(UnlockCondition::PetCount(cap[1].parse().unwrap_or(0)));
        }
    }

    if trimmed.to_lowercase().contains("defeat a d") && trimmed.to_lowercase().contains("boss") {
        let re = Regex::new(r"[Dd]efeat a (D\S+) [Bb]oss").unwrap();
        if let Some(cap) = re.captures(trimmed) {
            return Some(UnlockCondition::DungeonBoss(cap[1].to_string()));
        }
    }

    if trimmed.to_lowercase().starts_with("give it") {
        let gift = trimmed.strip_prefix("Give it").unwrap_or(trimmed).trim();
        return Some(UnlockCondition::ItemGift(gift.to_string()));
    }

    None
}

/// Parse the "Evo Difficulty" cell, e.g. "3(5)" or "3(4-5)".
///
/// Returns `None` when the wiki hasn't rated the pet yet (brand-new pets are
/// listed as `?(?)` until someone works out the difficulty) or the cell is in
/// a shape we don't know; `parse_pets_with_warnings` records which. (This used to fall
/// back to 1(1), which made unrated pets look like the easiest evolutions.)
fn parse_evo_difficulty(cell: &str) -> Option<EvoDifficulty> {
    let trimmed = cell.trim();
    let re = Regex::new(r"(\d+)\((\d+)(?:-(\d+))?\)").unwrap();
    if let Some(cap) = re.captures(trimmed) {
        let base: u8 = cap[1].parse().unwrap_or(1);
        let cond: u8 = if let Some(high) = cap.get(3) {
            high.as_str().parse().unwrap_or(1)
        } else {
            cap[2].parse().unwrap_or(1)
        };
        return Some(EvoDifficulty {
            base,
            with_conditions: cond,
        });
    }
    None
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

/// Parse the full wiki source into a list of WikiPet structs, discarding any
/// data warnings (see [`parse_pets_with_warnings`]).
pub fn parse_pets(source: &str) -> anyhow::Result<Vec<WikiPet>> {
    Ok(parse_pets_with_warnings(source)?.0)
}

/// Parse the full wiki source into WikiPet structs, plus a warning for every
/// cell that held a placeholder or a value we couldn't recognize.
///
/// A bad cell never fails the run: each field falls back to a safe default
/// (Neutral element, Wildcard class, SpecialTask unlock, unrated difficulty)
/// and the warning says so, so the refresh job can raise an alert instead.
pub fn parse_pets_with_warnings(source: &str) -> anyhow::Result<(Vec<WikiPet>, Vec<WikiDataWarning>)> {
    let mut pets = Vec::new();
    let mut warnings = Vec::new();

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

        let mut warn = |field: &str, detail: String| {
            warnings.push(WikiDataWarning { pet: name.clone(), field: field.to_string(), detail });
        };
        // Describes a cell we couldn't use and the fallback taken.
        let why = |cell: &str| {
            if cell.trim().is_empty() {
                "empty value".to_string()
            } else if is_placeholder(cell) {
                format!("placeholder value {:?}", cell.trim())
            } else {
                format!("unrecognized value {:?}", cell.trim())
            }
        };

        let element = try_parse_element(&cells[2]).unwrap_or_else(|| {
            warn("element", format!("{}; using Neutral", why(&cells[2])));
            Element::Neutral
        });
        let recommended_class = try_parse_recommended_class(&cells[3]).unwrap_or_else(|| {
            warn("recommended class", format!("{}; using Wildcard", why(&cells[3])));
            RecommendedClass::Wildcard
        });
        let class_bonus = parse_class_bonus(&cells[4]);
        if is_placeholder(&class_bonus) {
            warn("class bonus", format!("placeholder value {class_bonus:?}; kept as display text"));
        }
        let unlock_condition = try_parse_unlock_condition(&cells[5]).unwrap_or_else(|| {
            warn("unlock condition", format!("{}; using Special Task", why(&cells[5])));
            UnlockCondition::SpecialTask
        });
        let evo_difficulty = parse_evo_difficulty(&cells[6]);
        if evo_difficulty.is_none() {
            warn("evo difficulty", format!("{}; left unrated", why(&cells[6])));
        }
        let token_improvable = parse_improve(&cells[7]);
        let special_ability = parse_special_ability(&cells[9]);
        if let Some(sa) = &special_ability
            && is_placeholder(sa)
        {
            warn("special ability", format!("contains a placeholder: {sa:?}"));
        }

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
            campaign_bonus: None,
        });
    }

    Ok((pets, warnings))
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

    // The value may carry trailing text, which tells us the basis: most pets
    // render a bare number (total-growth threshold), but a base-growth pet like
    // Baby Carno renders "300000 base growth". Pull the leading integer for the
    // value and look for "base" to pick the variant.
    let growth_cell = evo_field(scope, "Total Growth")?;
    let value = parse_leading_int(&growth_cell)?;
    let growth = if growth_cell.to_ascii_lowercase().contains("base") {
        GrowthRequirement::Base(value)
    } else {
        GrowthRequirement::Total(value)
    };

    Some(EvoRequirements {
        growth,
        material: evo_field(scope, "Material").and_then(meaningful),
        other: evo_field(scope, "Other").and_then(meaningful),
    })
}

/// The raw text of the evolution block's growth-threshold cell, if the page
/// has one. Used to explain *why* [`parse_evo_requirements`] gave up (e.g. the
/// wiki shows `?` until the threshold is known).
pub fn evo_growth_cell(html: &str) -> Option<String> {
    let marker = html.find("Evolution Requirements")?;
    evo_field(&html[marker..], "Total Growth")
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

/// Parse the "Campaign Bonus" infobox row from a pet page's rendered HTML.
///
/// Unlike the evolution rows, the infobox pairs the "Campaign Bonus" and
/// "Special Ability" *labels* in one row and their *values* in the next row, so
/// we take the first table cell after the label row's `</tr>`.
///
/// Returns `None` when the pet has no campaign bonus ("None"/empty). Otherwise
/// the cleaned prose is kept in `raw` — display text only. Structured
/// percentages are not parsed from the prose; they're hand-curated in
/// `data/campaign_bonuses.yaml` (the planner's coverage test flags new pets
/// that need an entry there).
pub fn parse_campaign_bonus(html: &str) -> Option<CampaignBonus> {
    let marker = html.find("Campaign Bonus")?;
    let after = &html[marker..];
    let tr_end = after.find("</tr>")?;
    let rest = &after[tr_end + "</tr>".len()..];
    let cell_re = Regex::new(r"(?s)<td[^>]*>(.*?)</td>").unwrap();
    let raw = clean_html_value(cell_re.captures(rest)?.get(1)?.as_str());

    let check = raw.trim().trim_end_matches('.').trim().to_ascii_lowercase();
    if check.is_empty() || check == "none" || check == "-" {
        return None;
    }

    Some(CampaignBonus { raw })
}

/// Strip HTML tags, decode HTML entities, and collapse whitespace.
fn clean_html_value(s: &str) -> String {
    let tags = Regex::new(r"<[^>]*>").unwrap();
    let text = tags.replace_all(s, "");

    // Numeric entities (e.g. `&#160;` nbsp, `&#39;` apostrophe).
    let numeric = Regex::new(r"&#(\d+);").unwrap();
    let text = numeric.replace_all(&text, |c: &regex::Captures| {
        c[1]
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .map(String::from)
            .unwrap_or_default()
    });

    // Named entities; decode `&amp;` last so it can't re-form another entity.
    let text = text
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&");

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
    fn test_clean_html_value_entities() {
        // Numeric nbsp (the Beachball bug) collapses to a normal space.
        assert_eq!(clean_html_value("by sqrt(x) * 2&#160;%"), "by sqrt(x) * 2 %");
        // Named gt/lt.
        assert_eq!(clean_html_value("(-50% &gt; +50%)"), "(-50% > +50%)");
        // Numeric apostrophe + tag stripping.
        assert_eq!(clean_html_value("Bag&#39;s <b>bonus</b>"), "Bag's bonus");
        // Ampersand decoded last (not re-formed into another entity).
        assert_eq!(clean_html_value("A &amp; B"), "A & B");
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
        let ed = parse_evo_difficulty("3(4-5)").unwrap();
        assert_eq!(ed.base, 3);
        assert_eq!(ed.with_conditions, 5);
    }

    #[test]
    fn test_parse_evo_difficulty_simple() {
        let ed = parse_evo_difficulty("1(1)").unwrap();
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
        <tr><td colspan="2"><b>Total Growth</b> </td><td colspan="2">420</td></tr>
        <tr><td colspan="4"><i><b>Evolution Requirements</b></i></td></tr>
        <tr><td colspan="4" rowspan="3">Defeat <a href="/wiki/Gods">Hyperion</a></td>
        <td colspan="2"><b>Total Growth</b> </td><td colspan="2">100</td></tr>
        <tr><td colspan="2"><b>Material</b> </td><td colspan="2">5 Wood</td></tr>
        <tr><td colspan="2"><b>Other</b> </td><td colspan="2">100 Puny Food</td></tr>
    "#;

    #[test]
    fn test_parse_evo_requirements_basic() {
        let evo = parse_evo_requirements(MOUSE_EVO_HTML).expect("should parse");
        assert_eq!(
            evo.growth,
            GrowthRequirement::Total(100),
            "must use the Evolution Requirements threshold (total-growth), not the 420 stat"
        );
        assert_eq!(evo.material.as_deref(), Some("5 Wood"));
        assert_eq!(evo.other.as_deref(), Some("100 Puny Food"));
    }

    #[test]
    fn test_parse_evo_requirements_strips_links_and_commas() {
        // Sylph-style: large threshold and a wikilinked questline in "Other".
        let html = r#"
            <td colspan="4"><i><b>Evolution Requirements</b></i></td></tr>
            <tr><td colspan="2"><b>Total Growth</b> </td><td colspan="2">55,555</td></tr>
            <tr><td colspan="2"><b>Material</b> </td><td colspan="2">2778 <a href="/wiki/Bound_Feather">Bound Feather</a></td></tr>
            <tr><td colspan="2"><b>Other</b> </td><td colspan="2">Finish the <a href="/wiki/Sylph">Questline</a> (You <b>cannot</b> use a Pet Token)</td></tr>
        "#;
        let evo = parse_evo_requirements(html).expect("should parse");
        assert_eq!(evo.growth, GrowthRequirement::Total(55555));
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
            <tr><td colspan="2"><b>Total Growth</b> </td><td colspan="2">300000 <b>base growth</b> </td></tr>
            <tr><td colspan="2"><b>Material</b> </td><td colspan="2">3000 Magic Ore</td></tr>
            <tr><td colspan="2"><b>Other</b> </td><td colspan="2">none</td></tr>
        "#;
        let evo = parse_evo_requirements(html).expect("should parse despite suffix");
        assert_eq!(
            evo.growth,
            GrowthRequirement::Base(300000),
            "the 'base growth' marker must yield a Base requirement (Magic Egg won't help)"
        );
        assert!(!evo.growth.magic_egg_counts());
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

    #[test]
    fn test_parse_campaign_bonus_html() {
        let html = r#"
            <td colspan="4"><i><b>Campaign Bonus</b></i></td>
            <td colspan="4"><i><b>Special Ability</b></i></td></tr>
            <tr>
            <td colspan="4">+50% in item campaigns</td>
            <td colspan="4">some ability prose</td></tr>
        "#;
        let cb = parse_campaign_bonus(html).expect("should parse");
        assert_eq!(cb.raw, "+50% in item campaigns");

        // "None" → no bonus at all.
        let none_html = r#"<b>Campaign Bonus</b></i></td><td><i><b>Special Ability</b></i></td></tr><tr><td colspan="4">None</td></tr>"#;
        assert!(parse_campaign_bonus(none_html).is_none());
    }

    #[test]
    fn test_parse_evo_difficulty_unrated() {
        // New pets are listed as `?(?)` until the wiki rates them.
        assert_eq!(parse_evo_difficulty("?(?)"), None);
        assert_eq!(parse_evo_difficulty(" ? "), None);
    }

    /// Samael (added 2026-11-15) as the Pets table listed it, plus a made-up
    /// pet with placeholders in every structured column, in different styles
    /// (the wiki is user-edited, so unknowns aren't formatted consistently).
    /// Nothing fails; each bad cell falls back to a default and is reported.
    #[test]
    fn test_placeholders_fall_back_and_warn() {
        let source = r#"{| class="wikitable center sortable mw-collapsible"
|+
!Pet
!Name
|-
|[[File:Samael.png|80px|link=Samael]]
|[[Samael]]
|[[File:neutral.png|30x30px]]
|Blacksmith
|2.1% x CL
|Pet Token
|?(?)
|
|2026-11-15
|Skip T1-T3 to directly craft T4
|shinrais
|-
|[[File:Mystery.png|80px|link=Mystery]]
|[[Mystery]]
|???
|TBD
|?% x CL
|Unknown
|tbd
|
|2026-11-20
|Something (to be determined)
|
|}"#;
        let (pets, warnings) = parse_pets_with_warnings(source).unwrap();
        assert_eq!(pets.len(), 2);

        // Samael: only the difficulty is unknown.
        assert_eq!(pets[0].recommended_class, RecommendedClass::Single(Class::Blacksmith));
        assert_eq!(pets[0].evo_difficulty, None);
        let samael: Vec<_> = warnings.iter().filter(|w| w.pet == "Samael").collect();
        assert_eq!(samael.len(), 1, "{samael:?}");
        assert_eq!(samael[0].field, "evo difficulty");
        assert!(samael[0].detail.contains("placeholder"), "{}", samael[0].detail);

        // Mystery: every structured field falls back.
        let m = &pets[1];
        assert_eq!(m.element, Element::Neutral);
        assert_eq!(m.recommended_class, RecommendedClass::Wildcard);
        assert_eq!(m.unlock_condition, UnlockCondition::SpecialTask);
        assert_eq!(m.evo_difficulty, None);
        assert_eq!(m.class_bonus, "?% x CL");
        let mut fields: Vec<_> =
            warnings.iter().filter(|w| w.pet == "Mystery").map(|w| w.field.as_str()).collect();
        fields.sort();
        assert_eq!(
            fields,
            ["class bonus", "element", "evo difficulty", "recommended class", "special ability", "unlock condition"]
        );
        assert!(warnings.iter().all(|w| w.pet != "Mystery" || w.detail.contains("placeholder")));
    }

    #[test]
    fn test_unlock_alternatives() {
        use UnlockCondition::*;
        assert_eq!(try_parse_unlock_condition("Old Event Token or Pet Token"), Some(AnyOf(vec![OldEventToken, PetToken])));
        assert_eq!(
            try_parse_unlock_condition("[[Events#Old_Event_Tokens|Old Event Token]] or Pet Token"),
            Some(AnyOf(vec![OldEventToken, PetToken]))
        );
        assert_eq!(try_parse_unlock_condition("[[Milestones]] or Pet Token"), Some(AnyOf(vec![Milestones, PetToken])));
        assert_eq!(try_parse_unlock_condition("Defeat P.Baal 10 or Pet Token"), Some(AnyOf(vec![DefeatPBaal(10), PetToken])));
        assert_eq!(try_parse_unlock_condition("[[Milestones]]"), Some(Milestones));
        assert_eq!(try_parse_unlock_condition("Pet Token"), Some(PetToken));
        // A cell that parses whole isn't split on its "or".
        assert_eq!(try_parse_unlock_condition("Give it 1000 Honey or Mana"), Some(ItemGift("1000 Honey or Mana".into())));
        // One unknown alternative makes the whole cell unrecognized.
        assert_eq!(try_parse_unlock_condition("Mystery or Pet Token"), None);
    }

    #[test]
    fn test_evo_growth_cell_placeholder() {
        let html = r#"<td><b>Evolution Requirements</b></td></tr><tr>
<td colspan="2"><b>Total Growth</b>
</td>
<td colspan="2">?
</td></tr>"#;
        assert!(parse_evo_requirements(html).is_none());
        assert_eq!(evo_growth_cell(html).as_deref(), Some("?"));
    }

    /// Dullahan and Die/Dice (added 2026-10-30), verbatim from the wiki's Pets
    /// table and pet pages: unrated `?(?)` difficulty and a slash name.
    #[test]
    fn test_parse_new_pets_dullahan_and_die() {
        let source = r#"{| class="wikitable center sortable mw-collapsible"
|+
!Pet
!Name
!Element
!Recommended Class
!data-sort-type="number"|Class Bonus 
!Unlock Condition
!Evo Difficulty (Condition)
!Improve Available
!Release Date (Y/M/D)
!Special Ability
!Patreon / Creator
|-
|[[File:Dullahan.png|80px|link=Dullahan]]
|[[Dullahan]]
|[[File:neutral.png|30x30px]]
|Adventurer
|1.28% x CL
|Pet Token
|?(?)
|
|2026-10-30
|Bonus Campaign Rewards when paired with Nightmare
|
|-
|[[File:Die.png|80px|link=Die/Dice]]
|[[Die/Dice]]
|[[File:neutral.png|30x30px]]
|Alchemist
|0.64% x CL
|Pet Token
|?(?)
|
|2026-10-30
|Random ± Production Bonus
|blarke
|}"#;
        let pets = parse_pets(source).unwrap();
        assert_eq!(pets.len(), 2);

        let dullahan = &pets[0];
        assert_eq!(dullahan.name, "Dullahan");
        assert_eq!(dullahan.wiki_url, "https://itrtg.wiki.gg/wiki/Dullahan");
        assert_eq!(dullahan.element, Element::Neutral);
        assert_eq!(dullahan.recommended_class, RecommendedClass::Single(Class::Adventurer));
        assert_eq!(dullahan.unlock_condition, UnlockCondition::PetToken);
        assert_eq!(dullahan.evo_difficulty, None);

        let die = &pets[1];
        assert_eq!(die.name, "Die/Dice");
        assert_eq!(die.wiki_url, "https://itrtg.wiki.gg/wiki/Die/Dice");
        assert_eq!(die.recommended_class, RecommendedClass::Single(Class::Alchemist));
        assert_eq!(die.evo_difficulty, None);

        let dullahan_html = r#"<td colspan="4"><i><b>Campaign Bonus</b></i>
</td>
<td colspan="4"><i><b>Special Ability</b></i>
</td></tr>
<tr>
<td colspan="4">+77% to All Campaigns
</td>
<td colspan="4">Increases the bonus for all campaigns by an extra 1.28% * class level if your class is an adventurer.
</td></tr>
<tr>
<td colspan="4"><i><b>Unlock Criteria</b></i>
</td>
<td colspan="4"><i><b>Evolution Requirements</b></i>
</td></tr>
<tr>
<td colspan="4" rowspan="3"><a href="/wiki/Premium_Shop/Items#Pet_Token" title="Premium Shop/Items">Pet Token</a>
</td>
<td colspan="2"><b>Total Growth</b>
</td>
<td colspan="2">77777
</td></tr>
<tr>
<td colspan="2"><b>Material</b>
</td>
<td colspan="2">3889 Iron Bars
</td></tr>
<tr>
<td colspan="2"><b>Other</b>
</td>
<td colspan="2">Be in the same Campaign as Nightmare for 1000 Hours
</td></tr></tbody></table>"#;
        let evo = parse_evo_requirements(dullahan_html).unwrap();
        assert_eq!(evo.growth, GrowthRequirement::Total(77777));
        assert_eq!(evo.material.as_deref(), Some("3889 Iron Bars"));
        assert_eq!(evo.other.as_deref(), Some("Be in the same Campaign as Nightmare for 1000 Hours"));
        assert_eq!(parse_campaign_bonus(dullahan_html).unwrap().raw, "+77% to All Campaigns");

        let die_html = r#"<td colspan="4"><i><b>Campaign Bonus</b></i>
</td>
<td colspan="4"><i><b>Special Ability</b></i>
</td></tr>
<tr>
<td colspan="4">None.
</td>
<td colspan="4">Increases the speed of creating items by an extra (0.64 * class level) if your class is an alchemist.
</td></tr>
<tr>
<td colspan="4"><i><b>Unlock Criteria</b></i>
</td>
<td colspan="4"><i><b>Evolution Requirements</b></i>
</td></tr>
<tr>
<td colspan="4" rowspan="3"><a href="/wiki/Premium_Shop/Items#Pet_Token" title="Premium Shop/Items">Pet Token</a>
</td>
<td colspan="2"><b>Total Growth</b>
</td>
<td colspan="2">14000
</td></tr>
<tr>
<td colspan="2"><b>Material</b>
</td>
<td colspan="2">700 iron Bar
</td></tr>
<tr>
<td colspan="2"><b>Other</b>
</td>
<td colspan="2">Roll 6 in continuous crafting (<a href="/wiki/Material_Factory" title="Material Factory">Material Factory</a> or <a href="/wiki/Alchemy_Hut" title="Alchemy Hut">Alchemy Hut</a>) 5000 times
</td></tr></tbody></table>"#;
        let evo = parse_evo_requirements(die_html).unwrap();
        assert_eq!(evo.growth, GrowthRequirement::Total(14000));
        assert_eq!(evo.material.as_deref(), Some("700 iron Bar"));
        assert_eq!(
            evo.other.as_deref(),
            Some("Roll 6 in continuous crafting (Material Factory or Alchemy Hut) 5000 times")
        );
        assert!(parse_campaign_bonus(die_html).is_none());
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
