/// Mapping from export names (as they appear in the game's pet stats export)
/// to their canonical wiki display names.
///
/// Only entries that differ are listed; most pets match by simple normalization
/// (removing spaces, e.g. "BunnyGirl" → "Bunny Girl").
pub fn export_name_to_wiki_name(export_name: &str) -> Option<&'static str> {
    match export_name {
        // Slash names (export uses one side, wiki uses both)
        "Egg" => Some("Egg/Chicken"),
        "Stone" => Some("Stone/Golem"),
        "Mimic" => Some("Treasure/Mimic"),
        "Yggdrasil" => Some("Seed/Yggdrasil"),
        "Lizard" => Some("Lizard/Zookeeper"),
        "Owl" => Some("Feather Pile/Owl"),
        "Raiju" => Some("Thunder Ball/Raiju"),
        "StaleTortilla" => Some("Stale Tortilla/Taco"),
        // Completely different names
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
        // Gray's children (export uses no space)
        "GrayChild1" => Some("Gray Child 1"),
        "GrayChild2" => Some("Gray Child 2"),
        // Case differences where CamelCase splitting doesn't help
        "Ufo" => Some("UFO"),
        // Same name but needs explicit mapping to avoid false positive
        "Student" => Some("Student"),
        "Elemental" => Some("Elemental"),
        // Wiki uses single word, export uses CamelCase that would produce spaces
        "BeachBall" => Some("Beachball"),
        "CardboardBox" => Some("Cardboardbox"),
        "HoneyBadger" => Some("Honeybadger"),
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
    // e.g. "EarthEater" → "Earth Eater", "BunnyGirl" → "Bunny Girl"
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

/// Build a normalized lookup key from a name (lowercase, no spaces/punctuation).
/// Used by the merge logic for fuzzy matching.
pub fn normalize_for_lookup(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
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
        assert_eq!(resolve_wiki_name("EarthEater"), "Earth Eater");
        assert_eq!(resolve_wiki_name("BunnyGirl"), "Bunny Girl");
        assert_eq!(resolve_wiki_name("FlyingEyeball"), "Flying Eyeball");
        assert_eq!(resolve_wiki_name("ShadowClone"), "Shadow Clone");
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
    fn test_resolve_problem_names() {
        // These were causing duplicate entries before
        assert_eq!(resolve_wiki_name("BeachBall"), "Beachball");
        assert_eq!(resolve_wiki_name("CardboardBox"), "Cardboardbox");
        assert_eq!(resolve_wiki_name("HoneyBadger"), "Honeybadger");
        assert_eq!(resolve_wiki_name("Ufo"), "UFO");
        assert_eq!(resolve_wiki_name("GrayChild1"), "Gray Child 1");
        assert_eq!(resolve_wiki_name("GrayChild2"), "Gray Child 2");
        assert_eq!(resolve_wiki_name("StaleTortilla"), "Stale Tortilla/Taco");
    }

    #[test]
    fn test_resolve_already_correct() {
        assert_eq!(resolve_wiki_name("Cupid"), "Cupid");
        assert_eq!(resolve_wiki_name("Vampire"), "Vampire");
        assert_eq!(resolve_wiki_name("FSM"), "FSM");
    }
}
