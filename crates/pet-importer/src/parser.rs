use regex::Regex;

use itrtg_models::*;

/// Parse a full pet stats export file into a list of ExportPet structs.
pub fn parse_export(source: &str) -> anyhow::Result<Vec<ExportPet>> {
    let mut pets = Vec::new();
    let mut lines = source.lines();

    // Skip the header line
    let header = lines.next().ok_or_else(|| anyhow::anyhow!("Empty export file"))?;
    if !header.starts_with("Name;") {
        anyhow::bail!("Unexpected header format: {}", header);
    }

    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let fields: Vec<&str> = line.split(';').collect();
        if fields.len() < 24 {
            eprintln!("Warning: skipping line with {} fields (expected 24): {}", fields.len(), &line[..line.len().min(60)]);
            continue;
        }

        let export_name = fields[0].to_string();
        let element = parse_element(fields[1]);
        let growth = parse_number(fields[2]);
        let dungeon_level = parse_number(fields[3]) as u32;
        let class = parse_class(fields[4]);
        let class_level = parse_number(fields[5]) as u32;

        let combat_stats = CombatStats {
            hp: parse_signed(fields[6]),
            attack: parse_signed(fields[7]),
            defense: parse_signed(fields[8]),
            speed: parse_signed(fields[9]),
        };

        let elemental_affinities = ElementalAffinities {
            water: parse_signed(fields[10]),
            fire: parse_signed(fields[11]),
            wind: parse_signed(fields[12]),
            earth: parse_signed(fields[13]),
            dark: parse_signed(fields[14]),
            light: parse_signed(fields[15]),
        };

        let weapon = parse_equipment(fields[16]);
        let armor = parse_equipment(fields[17]);
        let accessory = parse_equipment(fields[18]);
        let loadout = Loadout { weapon, armor, accessory };

        let action = parse_action(fields[19]);
        let unlocked = fields[20].trim().eq_ignore_ascii_case("yes");
        let improved = fields[21].trim().eq_ignore_ascii_case("yes");

        let other_raw = fields[22].trim();
        let other = if other_raw.is_empty() { None } else { Some(other_raw.to_string()) };

        let has_partner = fields[23].trim().eq_ignore_ascii_case("yes");

        pets.push(ExportPet {
            export_name,
            element,
            growth,
            dungeon_level,
            class,
            class_level,
            combat_stats,
            elemental_affinities,
            loadout,
            action,
            unlocked,
            improved,
            other,
            has_partner,
        });
    }

    Ok(pets)
}

fn parse_element(s: &str) -> Element {
    match s.trim() {
        "Fire" => Element::Fire,
        "Water" => Element::Water,
        "Wind" => Element::Wind,
        "Earth" => Element::Earth,
        "Neutral" => Element::Neutral,
        other => {
            eprintln!("Warning: unknown element '{}', defaulting to Neutral", other);
            Element::Neutral
        }
    }
}

fn parse_class(s: &str) -> Option<Class> {
    match s.trim() {
        "None" | "" => None,
        "Adventurer" => Some(Class::Adventurer),
        "Blacksmith" => Some(Class::Blacksmith),
        "Alchemist" => Some(Class::Alchemist),
        "Defender" => Some(Class::Defender),
        "Supporter" => Some(Class::Supporter),
        "Rogue" => Some(Class::Rogue),
        "Assassin" => Some(Class::Assassin),
        "Mage" => Some(Class::Mage),
        other => {
            eprintln!("Warning: unknown class '{}', treating as None", other);
            None
        }
    }
}

/// Parse a number that may contain commas as thousands separators.
fn parse_number(s: &str) -> u64 {
    let cleaned: String = s.trim().replace(',', "");
    cleaned.parse().unwrap_or(0)
}

/// Parse a signed number that may contain commas.
fn parse_signed(s: &str) -> i64 {
    let cleaned: String = s.trim().replace(',', "");
    cleaned.parse().unwrap_or(0)
}

/// Parse an equipment string from the export.
///
/// Formats:
///   "none" or "0"                    → None
///   "Feather Vest, S"                → name, quality (no upgrade, no enchant)
///   "Candy Cane + 20, SSS"           → name, +20, SSS quality
///   "Journeying Stick + 5, S (20)"   → name, +5, S quality, 20 enchant
///   "Flame Sword + 10, SSS (1)"      → name, +10, SSS quality, 1 enchant
fn parse_equipment(s: &str) -> Option<Equipment> {
    let trimmed = s.trim();
    if trimmed.eq_ignore_ascii_case("none") || trimmed == "0" || trimmed.is_empty() {
        return None;
    }

    // Regex: optional upgrade "+ N", required ", Quality", optional "(enchant)"
    // Pattern: "Name [+ N], Quality [(enchant)]"
    let re = Regex::new(
        r"^(.+?)(?:\s*\+\s*(\d+))?\s*,\s*(F|E|D|C|B|A|S|SS|SSS)(?:\s*\((\d+)\))?$"
    ).unwrap();

    if let Some(cap) = re.captures(trimmed) {
        let name = cap[1].trim().to_string();
        let upgrade_level = cap.get(2).and_then(|m| m.as_str().parse().ok());
        let quality = parse_quality(&cap[3]);
        let enchant_level = cap.get(4).and_then(|m| m.as_str().parse().ok());

        return Some(Equipment {
            name,
            upgrade_level,
            quality,
            enchant_level,
        });
    }

    eprintln!("Warning: couldn't parse equipment '{}', skipping", trimmed);
    None
}

fn parse_quality(s: &str) -> Quality {
    match s.trim() {
        "F" => Quality::F,
        "E" => Quality::E,
        "D" => Quality::D,
        "C" => Quality::C,
        "B" => Quality::B,
        "A" => Quality::A,
        "S" => Quality::S,
        "SS" => Quality::SS,
        "SSS" => Quality::SSS,
        _ => Quality::F,
    }
}

/// Parse the action field from the export.
///
/// Examples from real data:
///   "      " (whitespace)              → Idle
///   "Growth"                           → Campaign(Growth)
///   "Food"                             → Campaign(Food)
///   "Scrapyard"                        → Dungeon(Scrapyard)
///   "Forest"                           → Dungeon(Forest)
///   "Crafting"                         → Crafting
///   "Fishing, fisher"                  → Village(Fishing("fisher"))
///   "Fishing, seller"                  → Village(Fishing("seller"))
///   "Material Factory, producing "     → Village(MaterialFactory("producing"))
///   "Questing, Water Droplets"         → Village(Questing("Water Droplets"))
fn parse_action(s: &str) -> PetAction {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return PetAction::Idle;
    }

    // Campaigns
    match trimmed {
        "Growth" => return PetAction::Campaign(CampaignType::Growth),
        "Divinity" => return PetAction::Campaign(CampaignType::Divinity),
        "Food" => return PetAction::Campaign(CampaignType::Food),
        "Item" => return PetAction::Campaign(CampaignType::Item),
        "Level" => return PetAction::Campaign(CampaignType::Level),
        "Multiplier" => return PetAction::Campaign(CampaignType::Multiplier),
        "GP" | "God Power" => return PetAction::Campaign(CampaignType::GodPower),
        _ => {}
    }

    // Dungeons
    match trimmed {
        "Newbie Ground" => return PetAction::Dungeon(Dungeon::NewbieGround),
        "Scrapyard" => return PetAction::Dungeon(Dungeon::Scrapyard),
        "Water Temple" => return PetAction::Dungeon(Dungeon::WaterTemple),
        "Volcano" => return PetAction::Dungeon(Dungeon::Volcano),
        "Mountain" => return PetAction::Dungeon(Dungeon::Mountain),
        "Forest" => return PetAction::Dungeon(Dungeon::Forest),
        _ => {}
    }

    if trimmed == "Crafting" {
        return PetAction::Crafting;
    }

    // Village jobs with sub-roles: "JobName, detail"
    if let Some((job, detail)) = trimmed.split_once(',') {
        let job = job.trim();
        let detail = detail.trim();
        let sub = if detail.is_empty() { None } else { Some(detail.to_string()) };

        match job {
            "Fishing" => return PetAction::Village(VillageJob::Fishing(sub)),
            "Material Factory" => return PetAction::Village(VillageJob::MaterialFactory(sub)),
            "Questing" => return PetAction::Village(VillageJob::Questing(sub)),
            "Alchemy Hut" | "Alchemist Hut" => return PetAction::Village(VillageJob::AlchemyHut),
            _ => {}
        }
    }

    // Village jobs without sub-roles
    match trimmed {
        "Alchemy Hut" | "Alchemist Hut" => return PetAction::Village(VillageJob::AlchemyHut),
        "Dojo" => return PetAction::Village(VillageJob::Dojo),
        "Strategy Room" => return PetAction::Village(VillageJob::StrategyRoom),
        _ => {}
    }

    // Fishing/village without comma separator
    if trimmed.starts_with("Fishing") {
        return PetAction::Village(VillageJob::Fishing(None));
    }

    eprintln!("Warning: unrecognized action '{}', treating as Idle", trimmed);
    PetAction::Idle
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_equipment_full() {
        let eq = parse_equipment("Journeying Stick + 5, S (20)").unwrap();
        assert_eq!(eq.name, "Journeying Stick");
        assert_eq!(eq.upgrade_level, Some(5));
        assert_eq!(eq.quality, Quality::S);
        assert_eq!(eq.enchant_level, Some(20));
    }

    #[test]
    fn test_parse_equipment_no_enchant() {
        let eq = parse_equipment("Candy Cane + 20, SSS").unwrap();
        assert_eq!(eq.name, "Candy Cane");
        assert_eq!(eq.upgrade_level, Some(20));
        assert_eq!(eq.quality, Quality::SSS);
        assert_eq!(eq.enchant_level, None);
    }

    #[test]
    fn test_parse_equipment_no_upgrade() {
        let eq = parse_equipment("Feather Vest, S").unwrap();
        assert_eq!(eq.name, "Feather Vest");
        assert_eq!(eq.upgrade_level, None);
        assert_eq!(eq.quality, Quality::S);
        assert_eq!(eq.enchant_level, None);
    }

    #[test]
    fn test_parse_equipment_none() {
        assert!(parse_equipment("none").is_none());
        assert!(parse_equipment("0").is_none());
    }

    #[test]
    fn test_parse_action_campaign() {
        assert_eq!(parse_action("Growth"), PetAction::Campaign(CampaignType::Growth));
        assert_eq!(parse_action("Food"), PetAction::Campaign(CampaignType::Food));
        assert_eq!(parse_action("Multiplier"), PetAction::Campaign(CampaignType::Multiplier));
    }

    #[test]
    fn test_parse_action_dungeon() {
        assert_eq!(parse_action("Scrapyard"), PetAction::Dungeon(Dungeon::Scrapyard));
        assert_eq!(parse_action("Forest"), PetAction::Dungeon(Dungeon::Forest));
    }

    #[test]
    fn test_parse_action_village() {
        assert_eq!(
            parse_action("Fishing, fisher"),
            PetAction::Village(VillageJob::Fishing(Some("fisher".to_string())))
        );
        assert_eq!(
            parse_action("Questing, Water Droplets"),
            PetAction::Village(VillageJob::Questing(Some("Water Droplets".to_string())))
        );
        // "Alchemist Hut, producing" from export data
        assert_eq!(
            parse_action("Alchemist Hut, producing"),
            PetAction::Village(VillageJob::AlchemyHut)
        );
        assert_eq!(
            parse_action("Alchemy Hut"),
            PetAction::Village(VillageJob::AlchemyHut)
        );
    }

    #[test]
    fn test_parse_action_idle() {
        assert_eq!(parse_action("      "), PetAction::Idle);
        assert_eq!(parse_action(""), PetAction::Idle);
    }

    #[test]
    fn test_parse_number_with_commas() {
        assert_eq!(parse_number("223,132"), 223132);
        assert_eq!(parse_number("1,074"), 1074);
        assert_eq!(parse_number("42"), 42);
    }

    #[test]
    fn test_parse_signed_negative() {
        assert_eq!(parse_signed("-50"), -50);
        assert_eq!(parse_signed("2,053"), 2053);
    }
}
