//! UI-independent prototype state. Domain calculations stay in models/planner.
//! Commands use canonical pet names, never a transient sorted row index.
use std::collections::HashSet;

use itrtg_models::{Element, ExportPet, RecommendedClass, WikiPet, CampaignBonusRules, parse_flexible_number, pgc_growth_mult};
use itrtg_planner::{analyzer::{self, AnalyzerState}, growth::GrowthRates};
pub use itrtg_planner::analyzer::SortColumn as Sort;
use itrtg_planner::merge::{self, EvoReadiness, MergedPet};
use serde::{Deserialize, Serialize};

pub const EXAMPLE_EXPORT: &str = include_str!("../fixtures/example_pet_stats.txt");
const WIKI: &str = include_str!("../../../data/wiki_pets.yaml");
const STATE_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ownership { #[default] All, Owned, Locked }


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
    pub analysis: AnalyzerState,
    pub ascending: Option<bool>,
    /// Full saves cannot supply live action or derived combat stats.
    pub roster_from_save: bool,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            version: STATE_VERSION, pets: Vec::new(), query: String::new(),
            ownership: Ownership::All, element: None, sort: Sort::Name,
            selected: None, pgc_done: 0, pgc_max: 25, source: "Wiki reference".into(),
            analysis: AnalyzerState::default(), ascending: None, roster_from_save: false,
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
        for moai in &mut session.analysis.moai { moai.level = moai.level.min(20); }
        validate_names(&session.pets)?;
        Ok(session)
    }
}

pub struct AppModel {
    pub session: Session,
    pub pets: Vec<MergedPet>,
    wiki: Vec<WikiPet>,
    pub bonuses: CampaignBonusRules,
}

impl AppModel {
    pub fn new(session: Session) -> Result<Self, String> {
        let wiki = serde_yaml::from_str(WIKI).map_err(|e| e.to_string())?;
        let bonuses = serde_yaml::from_str(include_str!("../../../data/campaign_bonuses.yaml")).map_err(|e| e.to_string())?;
        let mut app = Self { session, pets: Vec::new(), wiki, bonuses };
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
            return Err("Paste a Pet Stats export beginning with Name;. Choose the matching import type for Main Stats or a full save.".into());
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
        self.session.roster_from_save = false;
        self.rebuild();
        Ok(count)
    }

    /// Apply a completely decoded projection atomically. Raw save/account identity
    /// fields never enter the persisted prototype session.
    pub fn apply_save(&mut self, imported: PreparedSave) -> Result<usize, String> {
        if imported.pets.is_empty() { return Err("No pet rows found in save. Previous roster kept.".into()); }
        validate_names(&imported.pets)?;
        let mut settings = self.settings();
        settings.apply_main_stats(&imported.stats);
        for (index, statue) in settings.moai.iter_mut().enumerate() {
            match imported.moai.get(index) {
                Some(&level) => { statue.owned = true; statue.level = level.min(20) as u8; },
                None => statue.owned = false,
            }
        }
        let count = imported.pets.len();
        self.session.pgc_max = settings.pgc_max.min(1000);
        self.session.pgc_done = settings.pgc_done.min(self.session.pgc_max);
        self.session.analysis = settings;
        self.session.pets = imported.pets;
        self.session.source = "Imported full save".into();
        self.session.roster_from_save = true;
        self.rebuild();
        Ok(count)
    }

    pub fn action_text(&self, pet: &MergedPet) -> String {
        if self.session.roster_from_save { return "Unavailable in save".into(); }
        pet.export.as_ref().map(|e| analyzer::format_action(&e.action)).unwrap_or_else(|| "—".into())
    }

    pub fn multiplier(&self) -> f64 {
        pgc_growth_mult(self.session.pgc_done, self.session.pgc_max)
    }

    /// Use the same filter/sort implementation as egui. Keep v1 session keys
    /// readable while advanced settings are added without losing existing imports.
    pub fn settings(&self) -> AnalyzerState {
        let mut settings = self.session.analysis.clone();
        settings.search = self.session.query.trim().into();
        settings.filter_unlocked = match self.session.ownership { Ownership::All => None, Ownership::Owned => Some(true), Ownership::Locked => Some(false) };
        settings.filter_element = self.session.element;
        settings.sort_column = self.session.sort;
        settings.sort_ascending = self.session.ascending.unwrap_or(settings.sort_column.default_ascending());
        settings.pgc_done = self.session.pgc_done;
        settings.pgc_max = self.session.pgc_max;
        settings.campaign_inputs.earth_eater_total_planets = parse_flexible_number(&settings.earth_eater_planets_text).unwrap_or(0.0).max(0.0) as u64;
        settings
    }

    pub fn rates(&self) -> GrowthRates {
        let levels: Vec<_> = self.session.analysis.moai.iter().filter(|m| m.owned).map(|m| m.level).collect();
        GrowthRates::compute(&self.pets, &levels)
    }

    pub fn campaign_context<'a>(&'a self, settings: &'a AnalyzerState) -> merge::CampaignContext<'a> {
        merge::CampaignContext { bonuses: &self.bonuses, roster: &self.pets,
            inputs: &settings.campaign_inputs, include_equipment: settings.include_equipment_bonus,
            include_class: settings.include_class_bonus }
    }

    pub fn visible(&self) -> Vec<&MergedPet> {
        let settings = self.settings();
        let context = self.campaign_context(&settings);
        let mut pets = analyzer::filter_and_sort(&self.pets, &settings, &self.rates(), &context);
        // Preserve the prototype's distinction between unknown and explicitly locked.
        pets.retain(|p| self.session.ownership != Ownership::Locked || p.export.is_some());
        pets
    }

    pub fn import_main_stats(&mut self, source: &str) -> Result<String, String> {
        let stats = itrtg_models::parse_main_stats(source.trim_start_matches('\u{feff}'))?;
        let mut settings = self.settings();
        let applied = settings.apply_main_stats(&stats);
        if applied.is_empty() { return Err("No supported Main Stats values found. Existing settings kept.".into()); }
        self.session.pgc_max = settings.pgc_max.min(1000);
        self.session.pgc_done = settings.pgc_done.min(self.session.pgc_max);
        self.session.analysis = settings;
        self.reconcile_selection();
        Ok(format!("Updated {}. Pet roster kept.", applied.join(", ")))
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
        let a = &mut self.session.analysis;
        a.filter_evolved = None;
        a.filter_unlock_type = Default::default();
        a.filter_rec_class = Default::default();
        a.filter_my_class = Default::default();
        a.filter_improvable = Default::default();
        a.filter_campaign = None;
        if self.session.sort == Sort::CampaignBonus { self.session.sort = Sort::Name; self.session.ascending = None; }
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

/// Only planner data crosses the worker boundary; never the decoded raw tree.
pub struct PreparedSave {
    pub pets: Vec<ExportPet>,
    pub stats: itrtg_models::MainStats,
    pub moai: Vec<u32>,
}

pub fn prepare_save(raw: &str) -> Result<PreparedSave, String> {
    // Do not echo parser errors: malformed decoded fields may include account data.
    let save = save_parser::parse_save(raw).map_err(|_| "Could not decode this full save. Existing roster and settings kept.".to_string())?;
    let pets = save_parser::save_to_export_pets(&save);
    if pets.is_empty() { return Err("No pet rows found in save. Previous roster kept.".into()); }
    validate_names(&pets)?;
    Ok(PreparedSave { pets, stats: save_parser::save_to_main_stats(&save), moai: save_parser::moai_levels(&save) })
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
        let growths: Vec<_> = app.visible().iter().filter_map(|p| p.export.as_ref().map(|e| e.effective_growth_with_global_mult(app.multiplier()))).collect();
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

    #[test]
    fn original_prototype_sessions_load_with_new_settings_defaults() {
        let session = Session::from_yaml("version: 1\nquery: mouse\nsort: Growth\npgc_done: 25\npgc_max: 25\n").unwrap();
        let app = AppModel::new(session).unwrap();
        assert_eq!(app.session.query, "mouse");
        assert_eq!(app.multiplier(), 1.5);
        assert!(!app.settings().sort_ascending);
        assert!(!app.settings().include_equipment_bonus);
    }

    #[test]
    fn main_stats_updates_present_settings_without_replacing_roster() {
        let mut app = example();
        app.session.analysis.campaign_inputs.ants = 42;
        let pets = serde_yaml::to_string(&app.session.pets).unwrap();
        app.import_main_stats("Idling to Rule the Gods\nPet Stones: 250,882\nPatreon Gods Challenges: 25 / 25").unwrap();
        assert_eq!(app.session.analysis.campaign_inputs.pet_stones, 250_882);
        assert_eq!(app.session.analysis.campaign_inputs.ants, 42);
        assert_eq!(app.multiplier(), 1.5);
        assert_eq!(serde_yaml::to_string(&app.session.pets).unwrap(), pets);
        let before = serde_yaml::to_string(&app.session).unwrap();
        assert!(app.import_main_stats("Idling to Rule the Gods\nUnrelated: 1").is_err());
        assert_eq!(serde_yaml::to_string(&app.session).unwrap(), before);
    }

    #[test]
    fn advanced_filter_combinations_use_domain_rules_and_reset_keeps_settings() {
        use crate::controls;
        let mut app = example();
        controls::change_choice(&mut app, "class", 3); // Blacksmith
        assert!(!app.visible().is_empty());
        assert!(app.visible().iter().all(|p| p.evolved_class() == Some(itrtg_models::Class::Blacksmith)));
        controls::change_choice(&mut app, "evolved", 2); // Unevolved cannot be Blacksmith
        assert!(app.visible().is_empty());
        controls::change_number(&mut app, "moai1", "20").unwrap();
        app.reset_filters();
        assert_eq!(app.visible().len(), app.pets.len());
        assert_eq!(app.rates().moai_per_hour, 1.0);
    }

    #[test]
    fn campaign_filters_and_sort_use_curated_bonuses() {
        let mut app = example();
        crate::controls::change_choice(&mut app, "campaign", 1); // Growth
        app.session.sort = Sort::CampaignBonus;
        let settings = app.settings();
        let context = app.campaign_context(&settings);
        let bonuses: Vec<_> = app.visible().into_iter().map(|p| p.campaign_bonus_for(itrtg_models::CampaignType::Growth, &context).unwrap()).collect();
        assert!(!bonuses.is_empty());
        assert!(bonuses.iter().all(|b| *b > 0.0));
        assert!(bonuses.windows(2).all(|pair| pair[0] >= pair[1]));
    }

    #[test]
    fn settings_reject_invalid_numbers_and_preserve_previous_values() {
        let mut app = example();
        crate::controls::change_number(&mut app, "planets", "32.4e6").unwrap();
        assert_eq!(app.settings().campaign_inputs.earth_eater_total_planets, 32_400_000);
        let before = serde_yaml::to_string(&app.session).unwrap();
        for (key, text) in [("planets", "oops"), ("target", "-3"), ("moai1", "21"), ("moai2", "1.5"), ("ucc", "5e10"), ("target", "NaN")] {
            assert!(crate::controls::change_number(&mut app, key, text).is_err());
            assert_eq!(serde_yaml::to_string(&app.session).unwrap(), before);
        }
        crate::controls::change_number(&mut app, "target", "55,555").unwrap();
        app.session.sort = Sort::TimeToTarget;
        let times: Vec<_> = app.visible().iter().map(|p| p.hours_to_growth_with_mult(55_555, &app.rates(), app.multiplier()).unwrap_or(f64::INFINITY)).collect();
        assert!(times.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn shared_search_finds_abilities_and_detail_projection_covers_special_pets() {
        let mut app = example();
        let named = app.pets.iter().find(|p| p.wiki.as_ref().and_then(|w| w.special_ability.as_ref()).is_some_and(|s| !s.is_empty())).unwrap();
        let name = named.name.clone();
        app.session.query = named.wiki.as_ref().unwrap().special_ability.clone().unwrap();
        assert!(app.visible().iter().any(|p| p.name == name));
        app.session.query = "  Mouse  ".into();
        assert!(app.visible().iter().any(|p| p.name == "Mouse"));
        app.reset_filters();
        for pet in &app.pets {
            let sections = crate::details::sections(&app, pet);
            if pet.export.is_some() { assert!(sections.iter().any(|(t,_)| t == "EQUIPMENT")); }
            if pet.elemental_evo_plan().is_some() { assert!(sections.iter().any(|(t,_)| t == "ELEMENTAL FORM PROGRESS")); }
        }
    }
    fn reference_save() -> String {
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"),
            "/../../reference/save_file_deserialization/ManualSave_2026-06-09.txt")).unwrap()
    }

    #[test]
    fn full_save_import_matches_shared_converters_and_preserves_manual_inputs() {
        let raw = reference_save();
        let expected = save_parser::parse_save(&raw).unwrap();
        let expected_pets = save_parser::save_to_export_pets(&expected);
        let expected_stats = save_parser::save_to_main_stats(&expected);
        let mut app = example();
        app.session.analysis.campaign_inputs.beachball_given_stones = 12345;
        let count = app.apply_save(prepare_save(&raw).unwrap()).unwrap();
        assert_eq!(count, expected_pets.len());
        assert_eq!(serde_yaml::to_string(&app.session.pets).unwrap(), serde_yaml::to_string(&expected_pets).unwrap());
        assert_eq!(app.session.analysis.campaign_inputs.pet_stones, expected_stats.pet_stones.unwrap());
        assert_eq!(app.session.analysis.campaign_inputs.beachball_given_stones, 12345);
        assert_eq!((app.session.pgc_done, app.session.pgc_max), expected_stats.patreon_god_challenges.unwrap());
        let levels = save_parser::moai_levels(&expected);
        for (index, statue) in app.session.analysis.moai.iter().enumerate() {
            assert_eq!(statue.owned, index < levels.len());
            if statue.owned { assert_eq!(u32::from(statue.level), levels[index].min(20)); }
        }
        assert!(app.session.roster_from_save);
        let serialized = serde_yaml::to_string(&app.session).unwrap();
        assert!(!serialized.contains("RedactedAccount"));
        assert!(!serialized.contains("RedactedGod"));
        let restored = AppModel::new(Session::from_yaml(&serialized).unwrap()).unwrap();
        let pet = restored.pets.iter().find(|p| p.export.is_some()).unwrap();
        assert_eq!(restored.action_text(pet), "Unavailable in save");
        let detail = crate::details::sections(&restored, pet).into_iter().map(|(_,body)| body).collect::<Vec<_>>().join("\n");
        assert!(detail.contains("Combat stats: unavailable in save"));
        app.import(EXAMPLE_EXPORT, "Pet Stats").unwrap();
        assert!(!app.session.roster_from_save);
    }

    #[test]
    fn failed_full_save_imports_keep_roster_and_settings_atomically() {
        let mut app = example();
        let before = serde_yaml::to_string(&app.session).unwrap();
        for raw in ["", "not a save", EXAMPLE_EXPORT] {
            assert!(prepare_save(raw).and_then(|data| app.apply_save(data)).is_err());
            assert_eq!(serde_yaml::to_string(&app.session).unwrap(), before);
        }
        let mut duplicate = prepare_save(&reference_save()).unwrap();
        duplicate.pets.push(duplicate.pets[0].clone());
        assert!(app.apply_save(duplicate).is_err());
        let mut empty = prepare_save(&reference_save()).unwrap();
        empty.pets.clear();
        assert!(app.apply_save(empty).is_err());
        assert_eq!(serde_yaml::to_string(&app.session).unwrap(), before);
    }

    #[test]
    fn main_stats_keeps_save_provenance_and_zero_level_moai_survives_reload() {
        let mut app = example();
        app.apply_save(prepare_save(&reference_save()).unwrap()).unwrap();
        app.session.analysis.moai[0] = analyzer::MoaiStatue { owned: true, level: 0 };
        app.import_main_stats("Idling to Rule the Gods\nPet Stones: 123").unwrap();
        let restored = Session::from_yaml(&serde_yaml::to_string(&app.session).unwrap()).unwrap();
        assert!(restored.roster_from_save);
        assert!(restored.analysis.moai[0].owned);
        assert_eq!(restored.analysis.moai[0].level, 0);
    }

}
