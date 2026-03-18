use serde::{Deserialize, Serialize};

/// The primary element of a pet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Element {
    Neutral,
    Fire,
    Water,
    Wind,
    Earth,
    /// Chameleon can freely change its element.
    All,
}

/// A pet class. These are the actual in-game classes.
///
/// `Wildcard` is not a real in-game class — it is used in [`RecommendedClass::Dual`]
/// to indicate the pet can viably be evolved as whatever the player needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Class {
    Adventurer,
    Blacksmith,
    Alchemist,
    Defender,
    Supporter,
    Rogue,
    Assassin,
    Mage,
    /// Not a real class — used in Dual recommendations to indicate the pet can
    /// be evolved as whatever the player needs.
    Wildcard,
}

/// Mapping from export names (as they appear in the game's pet stats export)
/// to their canonical wiki display names.
///
/// Only entries that differ are listed; most pets match by simple normalization
/// (removing spaces, e.g. "BunnyGirl" → "Bunny Girl").
pub fn export_name_to_wiki_name(export_name: &str) -> Option<&'static str> {
    match export_name {
        "Egg" => Some("Egg/Chicken"),
        "Stone" => Some("Stone/Golem"),
        "Mimic" => Some("Treasure/Mimic"),
        "Yggdrasil" => Some("Seed/Yggdrasil"),
        "Lizard" => Some("Lizard/Zookeeper"),
        "Owl" => Some("Feather Pile/Owl"),
        "Raiju" => Some("Thunder Ball/Raiju"),
        "Reindeer" => Some("Rudolph"),
        "Pandora" => Some("Pandora's Box"),
        "GodPower" => Some("God Power (Pet)"),
        "Nothing" => Some("Nothing (Other)"),
        "BHC" => Some("Black Hole Chan"),
        "Book" => Some("Holy ITRTG Book"),
        "Carno" => Some("Baby Carno"),
        "Cloud" => Some("Flying Cloud"),
        "Crab" => Some("Hermit Crab"),
        "Spoon" => Some("Tödlicher Löffel"),
        "Volcano" => Some("Vesuvius"),
        "GoldDragon" => Some("Gold Dragon"),
        "Baphomate" => Some("Dark Gift"),
        "PixieGoat" => Some("Pixie Goatmother"),
        "Student" => Some("Student"),
        "Elemental" => Some("Elemental"),
        _ => None,
    }
}

/// Normalize an export name to wiki name. Falls back to inserting spaces
/// before capital letters for CamelCase names (e.g. "BunnyGirl" → "Bunny Girl").
pub fn resolve_wiki_name(export_name: &str) -> String {
    if let Some(mapped) = export_name_to_wiki_name(export_name) {
        return mapped.to_string();
    }

    // Insert spaces before uppercase letters that follow lowercase letters
    // e.g. "HoneyBadger" → "Honey Badger", "EarthEater" → "Earth Eater"
    let mut result = String::with_capacity(export_name.len() + 4);
    for (i, ch) in export_name.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            let prev = export_name.as_bytes()[i - 1] as char;
            if prev.is_lowercase() {
                result.push(' ');
            }
        }
        result.push(ch);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_simple() {
        assert_eq!(resolve_wiki_name("Mouse"), "Mouse");
    }

    #[test]
    fn test_resolve_camel_case() {
        assert_eq!(resolve_wiki_name("HoneyBadger"), "Honey Badger");
        assert_eq!(resolve_wiki_name("EarthEater"), "Earth Eater");
        assert_eq!(resolve_wiki_name("BunnyGirl"), "Bunny Girl");
        assert_eq!(resolve_wiki_name("FlyingEyeball"), "Flying Eyeball");
        assert_eq!(resolve_wiki_name("ShadowClone"), "Shadow Clone");
        assert_eq!(resolve_wiki_name("CardboardBox"), "Cardboard Box");
    }

    #[test]
    fn test_resolve_explicit_mappings() {
        assert_eq!(resolve_wiki_name("Egg"), "Egg/Chicken");
        assert_eq!(resolve_wiki_name("BHC"), "Black Hole Chan");
        assert_eq!(resolve_wiki_name("Spoon"), "Tödlicher Löffel");
        assert_eq!(resolve_wiki_name("GodPower"), "God Power (Pet)");
        assert_eq!(resolve_wiki_name("Reindeer"), "Rudolph");
        assert_eq!(resolve_wiki_name("Baphomate"), "Dark Gift");
        assert_eq!(resolve_wiki_name("PixieGoat"), "Pixie Goatmother");
    }

    #[test]
    fn test_resolve_already_correct() {
        assert_eq!(resolve_wiki_name("Cupid"), "Cupid");
        assert_eq!(resolve_wiki_name("Vampire"), "Vampire");
        assert_eq!(resolve_wiki_name("FSM"), "FSM");
    }
}
