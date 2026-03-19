use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Dungeon;

/// The type of campaign a pet can be sent on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CampaignType {
    Growth,
    Divinity,
    Food,
    Item,
    Level,
    Multiplier,
    GodPower,
}

/// Village jobs and their sub-roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VillageJob {
    Fishing(Option<String>),
    MaterialFactory(Option<String>),
    AlchemyHut,
    Dojo,
    StrategyRoom,
    Questing(Option<String>),
}

/// What a pet is currently doing.
///
/// Serializes to a flat string for YAML compatibility, e.g.:
///   "Idle", "Campaign: Growth", "Dungeon: Scrapyard",
///   "Crafting", "Village: Fishing (fisher)"
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PetAction {
    Idle,
    Campaign(CampaignType),
    Dungeon(Dungeon),
    Crafting,
    Village(VillageJob),
}

impl Serialize for PetAction {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let s = match self {
            PetAction::Idle => "Idle".to_string(),
            PetAction::Crafting => "Crafting".to_string(),
            PetAction::Campaign(ct) => {
                let name = match ct {
                    CampaignType::Growth => "Growth",
                    CampaignType::Divinity => "Divinity",
                    CampaignType::Food => "Food",
                    CampaignType::Item => "Item",
                    CampaignType::Level => "Level",
                    CampaignType::Multiplier => "Multiplier",
                    CampaignType::GodPower => "God Power",
                };
                format!("Campaign: {name}")
            }
            PetAction::Dungeon(d) => {
                let name = match d {
                    Dungeon::NewbieGround => "Newbie Ground",
                    Dungeon::Scrapyard => "Scrapyard",
                    Dungeon::WaterTemple => "Water Temple",
                    Dungeon::Volcano => "Volcano",
                    Dungeon::Mountain => "Mountain",
                    Dungeon::Forest => "Forest",
                };
                format!("Dungeon: {name}")
            }
            PetAction::Village(vj) => {
                let detail = match vj {
                    VillageJob::Fishing(sub) => match sub {
                        Some(s) => format!("Fishing ({s})"),
                        None => "Fishing".to_string(),
                    },
                    VillageJob::MaterialFactory(sub) => match sub {
                        Some(s) => format!("Material Factory ({s})"),
                        None => "Material Factory".to_string(),
                    },
                    VillageJob::AlchemyHut => "Alchemy Hut".to_string(),
                    VillageJob::Dojo => "Dojo".to_string(),
                    VillageJob::StrategyRoom => "Strategy Room".to_string(),
                    VillageJob::Questing(sub) => match sub {
                        Some(s) => format!("Questing ({s})"),
                        None => "Questing".to_string(),
                    },
                };
                format!("Village: {detail}")
            }
        };
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for PetAction {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;

        if s == "Idle" {
            return Ok(PetAction::Idle);
        }
        if s == "Crafting" {
            return Ok(PetAction::Crafting);
        }

        if let Some(rest) = s.strip_prefix("Campaign: ") {
            let ct = match rest {
                "Growth" => CampaignType::Growth,
                "Divinity" => CampaignType::Divinity,
                "Food" => CampaignType::Food,
                "Item" => CampaignType::Item,
                "Level" => CampaignType::Level,
                "Multiplier" => CampaignType::Multiplier,
                "God Power" => CampaignType::GodPower,
                _ => return Err(serde::de::Error::custom(format!("unknown campaign: {rest}"))),
            };
            return Ok(PetAction::Campaign(ct));
        }

        if let Some(rest) = s.strip_prefix("Dungeon: ") {
            let d = match rest {
                "Newbie Ground" => Dungeon::NewbieGround,
                "Scrapyard" => Dungeon::Scrapyard,
                "Water Temple" => Dungeon::WaterTemple,
                "Volcano" => Dungeon::Volcano,
                "Mountain" => Dungeon::Mountain,
                "Forest" => Dungeon::Forest,
                _ => return Err(serde::de::Error::custom(format!("unknown dungeon: {rest}"))),
            };
            return Ok(PetAction::Dungeon(d));
        }

        if let Some(rest) = s.strip_prefix("Village: ") {
            let vj = if rest == "Alchemy Hut" {
                VillageJob::AlchemyHut
            } else if rest == "Dojo" {
                VillageJob::Dojo
            } else if rest == "Strategy Room" {
                VillageJob::StrategyRoom
            } else if let Some(inner) = rest.strip_prefix("Fishing") {
                let sub = inner.trim().trim_start_matches('(').trim_end_matches(')').trim();
                VillageJob::Fishing(if sub.is_empty() { None } else { Some(sub.to_string()) })
            } else if let Some(inner) = rest.strip_prefix("Material Factory") {
                let sub = inner.trim().trim_start_matches('(').trim_end_matches(')').trim();
                VillageJob::MaterialFactory(if sub.is_empty() { None } else { Some(sub.to_string()) })
            } else if let Some(inner) = rest.strip_prefix("Questing") {
                let sub = inner.trim().trim_start_matches('(').trim_end_matches(')').trim();
                VillageJob::Questing(if sub.is_empty() { None } else { Some(sub.to_string()) })
            } else {
                return Err(serde::de::Error::custom(format!("unknown village job: {rest}")));
            };
            return Ok(PetAction::Village(vj));
        }

        Err(serde::de::Error::custom(format!("unknown action: {s}")))
    }
}
