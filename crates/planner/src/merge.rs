use std::collections::HashMap;

use itrtg_models::{
    Class, Element, ExportPet, RecommendedClass, WikiPet,
    resolve_wiki_name,
};

/// A pet with wiki reference data merged with the player's actual game data.
///
/// Either side can be missing:
/// - `wiki` is None if the pet exists in-game but isn't on the wiki yet
/// - `export` is None if the pet is on the wiki but not in the player's export
#[derive(Debug, Clone)]
pub struct MergedPet {
    /// Canonical display name (from wiki if available, otherwise resolved from export).
    pub name: String,
    pub wiki: Option<WikiPet>,
    pub export: Option<ExportPet>,
}

impl MergedPet {
    /// The pet's element (prefers export as ground truth, falls back to wiki).
    pub fn element(&self) -> Option<Element> {
        self.export
            .as_ref()
            .map(|e| e.element)
            .or_else(|| self.wiki.as_ref().map(|w| w.element))
    }

    /// The pet's actual evolved class (from export). None if unevolved or no export data.
    pub fn evolved_class(&self) -> Option<Class> {
        self.export.as_ref().and_then(|e| e.class)
    }

    /// Whether the pet is evolved (has a class assigned).
    pub fn is_evolved(&self) -> bool {
        self.evolved_class().is_some()
    }

    /// Whether the pet is unlocked in-game.
    pub fn is_unlocked(&self) -> bool {
        self.export.as_ref().is_some_and(|e| e.unlocked)
    }

    /// The wiki's recommended class, if we have wiki data.
    pub fn recommended_class(&self) -> Option<&RecommendedClass> {
        self.wiki.as_ref().map(|w| &w.recommended_class)
    }

    /// Whether this pet is a village pet (class doesn't matter for dungeons).
    pub fn is_village_pet(&self) -> bool {
        matches!(
            self.recommended_class(),
            Some(RecommendedClass::Village(_))
        )
    }

    /// Check if a given class is among this pet's recommended classes.
    /// Returns true for Single match, either side of a Dual, AllClasses, DungeonWildcard
    /// (for dungeon classes), and Wildcard. Returns false for Village and Special/Alternates.
    pub fn recommends_class(&self, target: &Class) -> bool {
        match self.recommended_class() {
            Some(RecommendedClass::Single(c)) => c == target,
            Some(RecommendedClass::Dual(a, b)) => {
                a == target || b == target
                    || *a == Class::Wildcard
                    || *b == Class::Wildcard
            }
            Some(RecommendedClass::AllClasses) => true,
            Some(RecommendedClass::DungeonWildcard) => matches!(
                target,
                Class::Defender
                    | Class::Supporter
                    | Class::Rogue
                    | Class::Assassin
                    | Class::Mage
            ),
            Some(RecommendedClass::Wildcard) => true,
            Some(RecommendedClass::Village(_)) => false,
            Some(RecommendedClass::Special) => false,
            Some(RecommendedClass::Alternates) => false,
            None => false,
        }
    }

    /// Whether this pet's element matches the target. `Element::All` (Chameleon) matches
    /// anything, and `None` target means "any element".
    pub fn matches_element(&self, target: Option<Element>) -> bool {
        let Some(target) = target else {
            return true; // "any" element
        };
        match self.element() {
            Some(Element::All) => true,
            Some(el) => el == target,
            None => false,
        }
    }
}

/// Merge wiki pet data with export pet data using name resolution.
///
/// Returns a list of `MergedPet` entries covering the union of both datasets.
pub fn merge_pets(wiki_pets: &[WikiPet], export_pets: &[ExportPet]) -> Vec<MergedPet> {
    // Build a lookup from wiki name → wiki pet
    let wiki_by_name: HashMap<String, &WikiPet> = wiki_pets
        .iter()
        .map(|w| (w.name.clone(), w))
        .collect();

    // Track which wiki pets got matched
    let mut matched_wiki: HashMap<String, bool> = wiki_pets
        .iter()
        .map(|w| (w.name.clone(), false))
        .collect();

    let mut merged = Vec::new();

    // First, process all export pets and try to match them to wiki entries
    for export in export_pets {
        let wiki_name = resolve_wiki_name(&export.export_name);
        let wiki = wiki_by_name.get(&wiki_name).copied().cloned();

        if wiki.is_some() {
            matched_wiki.insert(wiki_name.clone(), true);
        }

        merged.push(MergedPet {
            name: wiki_name,
            wiki,
            export: Some(export.clone()),
        });
    }

    // Then, add wiki-only pets (not present in the player's export)
    for wiki_pet in wiki_pets {
        if !matched_wiki.get(&wiki_pet.name).copied().unwrap_or(false) {
            merged.push(MergedPet {
                name: wiki_pet.name.clone(),
                wiki: Some(wiki_pet.clone()),
                export: None,
            });
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use itrtg_models::*;

    fn make_wiki_pet(name: &str, element: Element, rec_class: RecommendedClass) -> WikiPet {
        WikiPet {
            name: name.to_string(),
            wiki_url: format!("https://itrtg.wiki.gg/wiki/{}", name.replace(' ', "_")),
            element,
            recommended_class: rec_class,
            class_bonus: "0.5% x CL".to_string(),
            unlock_condition: UnlockCondition::PetToken,
            evo_difficulty: EvoDifficulty { base: 1, with_conditions: 1 },
            token_improvable: false,
            special_ability: None,
        }
    }

    fn make_export_pet(name: &str, element: Element, class: Option<Class>) -> ExportPet {
        ExportPet {
            export_name: name.to_string(),
            element,
            growth: 1000,
            dungeon_level: 10,
            class,
            class_level: 5,
            combat_stats: CombatStats { hp: 100, attack: 50, defense: 30, speed: 40 },
            elemental_affinities: ElementalAffinities {
                water: 0, fire: 0, wind: 0, earth: 0, dark: 0, light: 0,
            },
            loadout: Loadout { weapon: None, armor: None, accessory: None },
            action: PetAction::Idle,
            unlocked: true,
            improved: false,
            other: None,
            has_partner: false,
        }
    }

    #[test]
    fn test_merge_basic() {
        let wiki = vec![
            make_wiki_pet("Mouse", Element::Earth, RecommendedClass::Wildcard),
            make_wiki_pet("Frog", Element::Water, RecommendedClass::Single(Class::Supporter)),
        ];
        let export = vec![
            make_export_pet("Mouse", Element::Earth, None),
        ];

        let merged = merge_pets(&wiki, &export);
        assert_eq!(merged.len(), 2);

        // Mouse: both wiki and export
        assert!(merged[0].wiki.is_some());
        assert!(merged[0].export.is_some());

        // Frog: wiki only
        assert!(merged[1].wiki.is_some());
        assert!(merged[1].export.is_none());
    }

    #[test]
    fn test_merge_name_resolution() {
        let wiki = vec![
            make_wiki_pet("Egg/Chicken", Element::Wind, RecommendedClass::Single(Class::Assassin)),
        ];
        let export = vec![
            make_export_pet("Egg", Element::Wind, Some(Class::Assassin)),
        ];

        let merged = merge_pets(&wiki, &export);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "Egg/Chicken");
        assert!(merged[0].wiki.is_some());
        assert!(merged[0].export.is_some());
    }

    #[test]
    fn test_recommends_class() {
        let wiki = vec![
            make_wiki_pet("Frog", Element::Water, RecommendedClass::Single(Class::Supporter)),
            make_wiki_pet("Chameleon", Element::All, RecommendedClass::DungeonWildcard),
            make_wiki_pet("Swan", Element::Water, RecommendedClass::Village("Fisher".to_string())),
        ];
        let merged = merge_pets(&wiki, &[]);

        // Frog recommends Supporter but not Mage
        assert!(merged[0].recommends_class(&Class::Supporter));
        assert!(!merged[0].recommends_class(&Class::Mage));

        // Chameleon recommends any dungeon class
        assert!(merged[1].recommends_class(&Class::Defender));
        assert!(merged[1].recommends_class(&Class::Mage));
        assert!(!merged[1].recommends_class(&Class::Adventurer));

        // Swan is a village pet, never recommends for dungeon
        assert!(!merged[2].recommends_class(&Class::Supporter));
        assert!(merged[2].is_village_pet());
    }
}
