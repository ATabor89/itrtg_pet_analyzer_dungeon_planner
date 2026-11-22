//! Post-refresh audit of the scraped wiki data.
//!
//! The weekly wiki refresh must stay green when the wiki has gaps (a brand-new
//! pet is usually listed with `?` placeholders for a while), but those gaps
//! shouldn't pass silently either. This module collects everything that needs
//! a human look into one list of [`WikiDataWarning`]s: the extractor's own
//! parse warnings, placeholder text in the scraped data, unrated evolution
//! difficulties, and pets whose campaign bonus has no curated entry yet. The
//! `wiki-audit` binary turns that list into a job summary, workflow
//! annotations, and an issue body.

use std::collections::BTreeSet;

use itrtg_models::{is_placeholder, CampaignBonusRules, WikiDataWarning, WikiPet};

use crate::merge::{CAMPAIGN_FORMULA_PETS, CAMPAIGN_RAW_ONLY_PETS};

/// Field name used for the "needs a curated campaign-bonus entry" finding.
pub const UNCURATED_CAMPAIGN_FIELD: &str = "campaign bonus (uncurated)";

/// Pets with campaign-bonus text that are not covered by a curated entry, a
/// code formula, or the raw-only list. Each one needs an entry in
/// `data/campaign_bonuses.yaml` (or a spot on one of the lists).
pub fn uncurated_campaign_pets<'a>(wiki: &'a [WikiPet], rules: &CampaignBonusRules) -> Vec<&'a str> {
    wiki.iter()
        .filter(|p| {
            p.campaign_bonus.is_some()
                && !rules.0.contains_key(&p.name)
                && !CAMPAIGN_FORMULA_PETS.contains(&p.name.as_str())
                && !CAMPAIGN_RAW_ONLY_PETS.contains(&p.name.as_str())
        })
        .map(|p| p.name.as_str())
        .collect()
}

/// Everything about the scraped data that needs a human look.
///
/// `parse_warnings` are the extractor's own (they know the raw cell text, so
/// they come first and win over the data-level checks for the same pet and
/// field). The result is deduplicated on (pet, field) and sorted by pet.
pub fn audit(
    wiki: &[WikiPet],
    rules: &CampaignBonusRules,
    parse_warnings: &[WikiDataWarning],
) -> Vec<WikiDataWarning> {
    let mut out: Vec<WikiDataWarning> = parse_warnings.to_vec();
    let mut add = |pet: &str, field: &str, detail: String| {
        out.push(WikiDataWarning { pet: pet.to_string(), field: field.to_string(), detail });
    };

    for p in wiki {
        if p.evo_difficulty.is_none() {
            add(&p.name, "evo difficulty", "unrated on the wiki; left unrated".to_string());
        }
        if is_placeholder(&p.class_bonus) {
            add(&p.name, "class bonus", format!("placeholder value {:?}", p.class_bonus));
        }
        if let Some(sa) = &p.special_ability
            && is_placeholder(sa)
        {
            add(&p.name, "special ability", format!("contains a placeholder: {sa:?}"));
        }
        if let Some(cb) = &p.campaign_bonus
            && is_placeholder(&cb.raw)
        {
            add(&p.name, "campaign bonus", format!("contains a placeholder: {:?}", cb.raw));
        }
        if let Some(evo) = &p.evo_requirements {
            for (field, value) in [("evo material", &evo.material), ("evo other", &evo.other)] {
                if let Some(v) = value
                    && is_placeholder(v)
                {
                    add(&p.name, field, format!("contains a placeholder: {v:?}"));
                }
            }
        }
    }

    for name in uncurated_campaign_pets(wiki, rules) {
        let raw = wiki
            .iter()
            .find(|p| p.name == name)
            .and_then(|p| p.campaign_bonus.as_ref())
            .map(|c| c.raw.as_str())
            .unwrap_or_default();
        add(
            name,
            UNCURATED_CAMPAIGN_FIELD,
            format!("no entry in data/campaign_bonuses.yaml yet; shown as raw text only. Wiki says: {raw:?}"),
        );
    }

    let mut seen = BTreeSet::new();
    out.retain(|w| seen.insert((w.pet.clone(), w.field.clone())));
    out.sort_by(|a, b| a.pet.cmp(&b.pet));
    out
}

/// Render findings as Markdown (a job summary or an issue body), one section
/// per pet. Empty input renders an "all clear" line.
pub fn render_markdown(findings: &[WikiDataWarning]) -> String {
    if findings.is_empty() {
        return "No unknown or placeholder values in the wiki data.\n".to_string();
    }
    let pets: BTreeSet<&str> = findings.iter().map(|w| w.pet.as_str()).collect();
    let mut md = format!(
        "The wiki refresh found {} item(s) across {} pet(s) that it filled with a \
         fallback or that need curating. The data was still refreshed; these \
         clear on their own once the wiki (or `data/campaign_bonuses.yaml`) is updated.\n",
        findings.len(),
        pets.len()
    );
    for pet in pets {
        md.push_str(&format!("\n### {pet}\n\n"));
        for w in findings.iter().filter(|w| w.pet == pet) {
            md.push_str(&format!("- **{}**: {}\n", w.field, w.detail));
        }
    }
    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use itrtg_models::{CampaignBonus, Element, EvoDifficulty, RecommendedClass, UnlockCondition};

    fn pet(name: &str) -> WikiPet {
        WikiPet {
            name: name.to_string(),
            wiki_url: String::new(),
            element: Element::Neutral,
            recommended_class: RecommendedClass::Wildcard,
            class_bonus: "1% x CL".to_string(),
            unlock_condition: UnlockCondition::PetToken,
            evo_difficulty: Some(EvoDifficulty { base: 1, with_conditions: 1 }),
            token_improvable: false,
            special_ability: None,
            evo_requirements: None,
            campaign_bonus: None,
        }
    }

    #[test]
    fn clean_pet_has_no_findings() {
        let rules = CampaignBonusRules::default();
        assert!(audit(&[pet("Mouse")], &rules, &[]).is_empty());
    }

    #[test]
    fn flags_new_pet_unknowns_and_dedupes_parse_warnings() {
        let mut new = pet("Newbie");
        new.evo_difficulty = None;
        new.class_bonus = "?% x CL".to_string();
        new.campaign_bonus = Some(CampaignBonus { raw: "+?% in Growth campaigns".to_string() });
        let parse = vec![WikiDataWarning {
            pet: "Newbie".to_string(),
            field: "evo difficulty".to_string(),
            detail: "placeholder value \"?(?)\"; left unrated".to_string(),
        }];
        let rules = CampaignBonusRules::default();
        let findings = audit(&[pet("Mouse"), new], &rules, &parse);

        let fields: Vec<&str> = findings.iter().map(|w| w.field.as_str()).collect();
        assert_eq!(fields, ["evo difficulty", "class bonus", "campaign bonus", UNCURATED_CAMPAIGN_FIELD]);
        assert!(findings.iter().all(|w| w.pet == "Newbie"));
        // The extractor's warning (with the raw cell text) wins the dedupe.
        assert!(findings[0].detail.contains("?(?)"));

        let md = render_markdown(&findings);
        assert!(md.contains("### Newbie"));
        assert!(md.contains("4 item(s) across 1 pet(s)"));
    }

    #[test]
    fn curated_formula_and_raw_only_pets_are_covered() {
        let mut curated = pet("Curated");
        curated.campaign_bonus = Some(CampaignBonus { raw: "+10% Growth".to_string() });
        let mut formula = pet(CAMPAIGN_FORMULA_PETS[0]);
        formula.campaign_bonus = curated.campaign_bonus.clone();
        let mut raw_only = pet(CAMPAIGN_RAW_ONLY_PETS[0]);
        raw_only.campaign_bonus = curated.campaign_bonus.clone();
        let rules: CampaignBonusRules =
            serde_yaml::from_str("Curated:\n  - when: Always\n    set: { Growth: 10 }\n").unwrap();
        let wiki = [curated, formula, raw_only];
        assert!(uncurated_campaign_pets(&wiki, &rules).is_empty());
    }
}
