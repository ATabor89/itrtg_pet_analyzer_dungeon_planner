//! UI-independent prototype state. Domain calculations stay in models/planner.
//! Commands use canonical pet names, never a transient sorted row index.
use std::collections::HashSet;

use itrtg_models::{Element, ExportPet, RecommendedClass, WikiPet, pgc_growth_mult};
use itrtg_planner::merge::{self, EvoReadiness, MergedPet};
use serde::{Deserialize, Serialize};

pub const EXAMPLE_EXPORT: &str = include_str!("../fixtures/example_pet_stats.txt");
const WIKI: &str = include_str!("../../../data/wiki_pets.yaml");
const STATE_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ownership { #[default] All, Owned, Locked }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sort { #[default] Name, Growth, DungeonLevel, ClassLevel }

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub version: u32,
    pub pets: Vec<ExportPet>,
    pub query: String,
    pub ownership: Ownership,
    pub element: Option<Element>,
    pub sort: Sort,
    pub selected: Option<String>,
    pub pgc_done: u32,
    pub pgc_max: u32,
    pub source: String,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            version: STATE_VERSION, pets: Vec::new(), query: String::new(),
            ownership: Ownership::All, element: None, sort: Sort::Name,
            selected: None, pgc_done: 0, pgc_max: 25, source: "Wiki reference".into(),
        }
    }
}

impl Session {
    pub fn from_yaml(yaml: &str) -> Result<Self, String> {
        let mut session: Self = serde_yaml::from_str(yaml).map_err(|e| e.to_string())?;
        if session.version != STATE_VERSION {
            return Err(format!("Unsupported prototype settings version {}", session.version));
        }
        session.pgc_max = session.pgc_max.min(1000);
        session.pgc_done = session.pgc_done.min(session.pgc_max);
        validate_names(&session.pets)?;
        Ok(session)
    }
}

pub struct AppModel {
    pub session: Session,
    pub pets: Vec<MergedPet>,
    wiki: Vec<WikiPet>,
}

impl AppModel {
    pub fn new(session: Session) -> Result<Self, String> {
        let wiki = serde_yaml::from_str(WIKI).map_err(|e| e.to_string())?;
        let mut app = Self { session, pets: Vec::new(), wiki };
        app.rebuild();
        Ok(app)
    }

    fn rebuild(&mut self) {
        self.pets = merge::merge_pets(&self.wiki, &self.session.pets);
        self.reconcile_selection();
    }

    /// Parse completely before replacing the roster. The shared importer skips
    /// short rows; the UI rejects them so a damaged paste cannot erase pets.
    pub fn import(&mut self, source: &str, label: &str) -> Result<usize, String> {
        let source = source.trim_start_matches('\u{feff}').trim_start();
        if !source.starts_with("Name;") {
            return Err("Paste a Pet Stats export beginning with Name;. Full saves and other exports are not supported here yet.".into());
        }
        for (index, line) in source.lines().enumerate().skip(1) {
            if !line.trim().is_empty() && line.split(';').count() < 24 {
                return Err(format!("Incomplete pet data on line {}. Previous roster kept.", index + 1));
            }
        }
        let pets = pet_importer::parser::parse_export(source).map_err(|e| e.to_string())?;
        if pets.is_empty() { return Err("No pet rows found. Previous roster kept.".into()); }
        validate_names(&pets)?;
        let count = pets.len();
        self.session.pets = pets;
        self.session.source = label.into();
        self.rebuild();
        Ok(count)
    }

    pub fn multiplier(&self) -> f64 {
        pgc_growth_mult(self.session.pgc_done, self.session.pgc_max)
    }

    pub fn visible(&self) -> Vec<&MergedPet> {
        let query = self.session.query.trim().to_lowercase();
        let mut pets: Vec<_> = self.pets.iter().filter(|pet| {
            let name_matches = pet.name.to_lowercase().contains(&query)
                || pet.export.as_ref().is_some_and(|e| e.export_name.to_lowercase().contains(&query));
            let ownership_matches = match self.session.ownership {
                Ownership::All => true,
                Ownership::Owned => pet.is_unlocked(),
                // Missing export data is unknown, not evidence that a pet is locked.
                Ownership::Locked => pet.export.as_ref().is_some_and(|e| !e.unlocked),
            };
            name_matches && ownership_matches
                && self.session.element.is_none_or(|el| pet.element() == Some(el))
        }).collect();
        pets.sort_by(|a, b| {
            let numeric = |pet: &MergedPet| pet.export.as_ref().map(|e| match self.session.sort {
                Sort::Growth => e.growth,
                Sort::DungeonLevel => u64::from(e.dungeon_level),
                Sort::ClassLevel => u64::from(e.class_level),
                Sort::Name => 0,
            });
            let order = if self.session.sort == Sort::Name {
                std::cmp::Ordering::Equal
            } else {
                numeric(b).cmp(&numeric(a)) // Descending, unknowns last.
            };
            order.then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        pets
    }

    pub fn reconcile_selection(&mut self) {
        let visible = self.visible();
        if !visible.iter().any(|p| Some(&p.name) == self.session.selected.as_ref()) {
            self.session.selected = visible.first().map(|p| p.name.clone());
        }
    }

    pub fn select(&mut self, name: &str) {
        if self.visible().iter().any(|p| p.name == name) {
            self.session.selected = Some(name.into());
        }
    }

    pub fn selected(&self) -> Option<&MergedPet> {
        self.pets.iter().find(|p| Some(&p.name) == self.session.selected.as_ref())
    }

    pub fn reset_filters(&mut self) {
        self.session.query.clear();
        self.session.ownership = Ownership::All;
        self.session.element = None;
        self.reconcile_selection();
    }

    pub fn readiness(&self, pet: &MergedPet) -> &'static str {
        if pet.is_evolved() { return "Evolved"; }
        match pet.evo_readiness_with_growth_mult(self.multiplier()) {
            Some(EvoReadiness::Ready) => "Growth ready",
            Some(EvoReadiness::ReadyWithEgg) => "Ready with egg",
            Some(EvoReadiness::NotYet) => "Needs growth",
            None => "Unknown",
        }
    }
}

fn validate_names(pets: &[ExportPet]) -> Result<(), String> {
    let mut names = HashSet::new();
    for pet in pets {
        let name = itrtg_models::resolve_wiki_name(&pet.export_name);
        if name.trim().is_empty() || !names.insert(name) {
            return Err("The export contains an empty or duplicate pet name. Previous roster kept.".into());
        }
    }
    Ok(())
}

pub fn recommended_class(pet: &MergedPet) -> String {
    match pet.recommended_class() {
        Some(RecommendedClass::Single(c)) => format!("{c:?}"),
        Some(RecommendedClass::Dual(a, b)) => format!("{a:?} / {b:?}"),
        Some(RecommendedClass::Village(role)) => format!("Village · {role}"),
        Some(RecommendedClass::Wildcard) => "Any class".into(),
        Some(RecommendedClass::DungeonWildcard) => "Any dungeon class".into(),
        Some(RecommendedClass::AllClasses) => "Freely switches classes".into(),
        Some(RecommendedClass::Special) => "Special".into(),
        Some(RecommendedClass::Alternates) => "Alternating forms".into(),
        None => "Unknown".into(),
    }
}

pub fn number(n: u64) -> String {
    let text = n.to_string();
    let mut result = String::new();
    for (i, ch) in text.chars().enumerate() {
        if i > 0 && (text.len() - i).is_multiple_of(3) { result.push(','); }
        result.push(ch);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example() -> AppModel {
        let mut app = AppModel::new(Session::default()).unwrap();
        app.import(EXAMPLE_EXPORT, "Example").unwrap();
        app
    }

    #[test]
    fn import_uses_shared_parser_and_merge_without_changing_growth() {
        let app = example();
        let parsed = pet_importer::parser::parse_export(EXAMPLE_EXPORT).unwrap();
        assert_eq!(app.session.pets.len(), parsed.len());
        for source in parsed {
            let name = itrtg_models::resolve_wiki_name(&source.export_name);
            let merged = app.pets.iter().find(|p| p.name == name).unwrap();
            assert_eq!(merged.export.as_ref().unwrap().growth, source.growth);
        }
    }

    #[test]
    fn failed_and_partial_imports_are_atomic() {
        let mut app = example();
        let before = serde_yaml::to_string(&app.session).unwrap();
        for input in ["bad", "Name;Element", "Name;Element\nMouse;Earth", &format!("{EXAMPLE_EXPORT}\nTruncated;Water")] {
            assert!(app.import(input, "Bad").is_err());
            assert_eq!(serde_yaml::to_string(&app.session).unwrap(), before);
        }
    }

    #[test]
    fn duplicate_aliases_cannot_create_ambiguous_selection() {
        let mut app = example();
        let first = EXAMPLE_EXPORT.lines().nth(1).unwrap();
        assert!(app.import(&format!("{EXAMPLE_EXPORT}\n{first}"), "Bad").is_err());
    }

    #[test]
    fn sorting_preserves_identity_and_filters_reconcile_selection() {
        let mut app = example();
        let selected = app.visible()[3].name.clone();
        app.select(&selected);
        app.session.sort = Sort::Growth;
        app.reconcile_selection();
        assert_eq!(app.selected().unwrap().name, selected);
        let growths: Vec<_> = app.visible().iter().filter_map(|p| p.export.as_ref().map(|e| e.growth)).collect();
        assert!(growths.windows(2).all(|pair| pair[0] >= pair[1]));
        app.session.query = "no such pet qqq".into();
        app.reconcile_selection();
        assert!(app.selected().is_none());
        app.reset_filters();
        assert!(app.selected().is_some());
    }

    #[test]
    fn no_export_is_unknown_not_locked() {
        let mut app = AppModel::new(Session::default()).unwrap();
        app.session.ownership = Ownership::Locked;
        assert!(app.visible().is_empty());
    }

    #[test]
    fn pgc_and_egg_delegate_to_existing_domain_rules() {
        let mut app = example();
        app.session.pgc_done = 24;
        assert_eq!(app.multiplier(), 1.24);
        app.session.pgc_done = 25;
        assert_eq!(app.multiplier(), 1.5);
        let pet = app.pets.iter().find(|p| p.export.as_ref().is_some_and(|e| e.has_magic_egg())).unwrap();
        let export = pet.export.as_ref().unwrap();
        assert_eq!(export.effective_growth_with_global_mult(app.multiplier()),
            itrtg_models::displayed_growth(export.growth, 1.3 * 1.5));
    }

    #[test]
    fn session_round_trip_retains_roster_filters_selection_and_pgc() {
        let mut app = example();
        app.session.query = "mouse".into();
        app.session.pgc_done = 25;
        app.reconcile_selection();
        let yaml = serde_yaml::to_string(&app.session).unwrap();
        let restored = AppModel::new(Session::from_yaml(&yaml).unwrap()).unwrap();
        assert_eq!(restored.selected().unwrap().name, app.selected().unwrap().name);
        assert_eq!(restored.multiplier(), 1.5);
        assert_eq!(restored.session.pets.len(), app.session.pets.len());
        assert!(Session::from_yaml("version: 999").is_err());
    }
}
