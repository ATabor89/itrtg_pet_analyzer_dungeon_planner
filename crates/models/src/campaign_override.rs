//! Curated campaign-bonus overrides.
//!
//! Many pets' campaign bonuses can't be parsed from the wiki infobox prose, or
//! depend on evolution/token state that the infobox doesn't encode (Hedgehog's
//! token boost, Lizard's evo swap, the per-form elementals, the "greater chance
//! to find X" prose pets). This file's types describe hand-authored corrections
//! that are applied *on top of* the parsed static baseline by the planner's
//! `campaign_bonuses` seam, conditioned on the pet's actual export state.
//!
//! Loaded from `data/campaign_overrides.yaml`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::CampaignType;

/// When an override rule applies, based on the pet's current export state.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverrideWhen {
    /// Always applies.
    #[default]
    Always,
    /// Only when the pet is evolved (has a class).
    Evolved,
    /// Only when the pet is not yet evolved.
    Unevolved,
    /// Only when the pet has been token-improved.
    TokenImproved,
    /// Only when the pet has not been token-improved.
    NotTokenImproved,
}

/// One override rule: a condition plus the operations it performs on the
/// per-campaign map. Within a rule, operations apply in the order
/// `set_all` → `set` → `add_all` → `add`: absolutes first (broad then
/// specific), then deltas (broad then specific).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CampaignOverrideRule {
    pub when: OverrideWhen,
    /// Set every campaign to this value (e.g. "all campaigns -75%").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set_all: Option<f32>,
    /// Set specific campaigns to absolute values.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub set: BTreeMap<CampaignType, f32>,
    /// Add this delta to *every* campaign (e.g. Cupid's token boost adds +30 to
    /// all, then `add` tops up Divinity).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_all: Option<f32>,
    /// Add deltas to specific campaigns, on top of the current value (e.g.
    /// Hedgehog's token boost adds +141 to growth and divinity).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub add: BTreeMap<CampaignType, f32>,
}

/// All curated overrides, keyed by canonical pet name.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CampaignOverrides(pub BTreeMap<String, Vec<CampaignOverrideRule>>);

impl CampaignOverrides {
    /// Apply this pet's override rules (if any) to its `base` campaign map,
    /// given its current evolution/token state. Rules whose condition doesn't
    /// match are skipped; matching rules apply in file order.
    pub fn apply(
        &self,
        pet_name: &str,
        base: &mut BTreeMap<CampaignType, f32>,
        evolved: bool,
        improved: bool,
    ) {
        let Some(rules) = self.0.get(pet_name) else {
            return;
        };
        for rule in rules {
            let applies = match rule.when {
                OverrideWhen::Always => true,
                OverrideWhen::Evolved => evolved,
                OverrideWhen::Unevolved => !evolved,
                OverrideWhen::TokenImproved => improved,
                OverrideWhen::NotTokenImproved => !improved,
            };
            if !applies {
                continue;
            }
            if let Some(v) = rule.set_all {
                for c in CampaignType::ALL {
                    base.insert(c, v);
                }
            }
            for (c, v) in &rule.set {
                base.insert(*c, *v);
            }
            if let Some(v) = rule.add_all {
                for c in CampaignType::ALL {
                    *base.entry(c).or_insert(0.0) += v;
                }
            }
            for (c, v) in &rule.add {
                *base.entry(*c).or_insert(0.0) += *v;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overrides_from(yaml: &str) -> CampaignOverrides {
        serde_yaml::from_str(yaml).expect("override yaml should parse")
    }

    #[test]
    fn test_token_improved_add() {
        let ov = overrides_from(
            "Hedgehog:\n  - when: TokenImproved\n    add: { Growth: 141, Divinity: 141 }\n",
        );
        // Base from the wiki parse.
        let base = || BTreeMap::from([(CampaignType::Growth, 25.0), (CampaignType::Divinity, 25.0)]);

        // Improved: +141 each.
        let mut m = base();
        ov.apply("Hedgehog", &mut m, false, true);
        assert_eq!(m.get(&CampaignType::Growth), Some(&166.0));
        assert_eq!(m.get(&CampaignType::Divinity), Some(&166.0));

        // Not improved: unchanged.
        let mut m = base();
        ov.apply("Hedgehog", &mut m, false, false);
        assert_eq!(m.get(&CampaignType::Growth), Some(&25.0));
    }

    #[test]
    fn test_evolved_set_all_flip() {
        let ov = overrides_from(
            "Nothing (Other):\n  - when: Unevolved\n    set_all: -75\n  - when: Evolved\n    set_all: 75\n",
        );
        // Raw-only pet: empty base.
        let mut m = BTreeMap::new();
        ov.apply("Nothing (Other)", &mut m, false, false);
        assert_eq!(m.get(&CampaignType::Food), Some(&-75.0));
        assert_eq!(m.len(), 7);

        let mut m = BTreeMap::new();
        ov.apply("Nothing (Other)", &mut m, true, false);
        assert_eq!(m.get(&CampaignType::Food), Some(&75.0));
    }

    #[test]
    fn test_real_overrides_file_parses() {
        let yaml = include_str!("../../../data/campaign_overrides.yaml");
        let ov: CampaignOverrides =
            serde_yaml::from_str(yaml).expect("campaign_overrides.yaml should parse");

        // Hedgehog's token boost lands on the parsed +25 base.
        let mut m =
            BTreeMap::from([(CampaignType::Growth, 25.0), (CampaignType::Divinity, 25.0)]);
        ov.apply("Hedgehog", &mut m, false, true);
        assert_eq!(m.get(&CampaignType::Growth), Some(&166.0));

        // A prose correction with no base.
        let mut m = BTreeMap::new();
        ov.apply("Cat", &mut m, false, false);
        assert_eq!(m.get(&CampaignType::GodPower), Some(&50.0));

        // Evolution flip.
        let mut m = BTreeMap::new();
        ov.apply("Nothing (Other)", &mut m, true, false);
        assert_eq!(m.get(&CampaignType::Multiplier), Some(&75.0));
    }

    #[test]
    fn test_add_all_then_add_specific() {
        // Cupid: base +100 Divinity; token-improved adds +30 to all and +20
        // more to Divinity (so +50 there) → Divinity 150, everything else 30.
        let ov = overrides_from(
            "Cupid:\n  - when: Always\n    set: { Divinity: 100 }\n  - when: TokenImproved\n    add_all: 30\n    add: { Divinity: 20 }\n",
        );
        let mut m = BTreeMap::new();
        ov.apply("Cupid", &mut m, false, true);
        assert_eq!(m.get(&CampaignType::Divinity), Some(&150.0));
        assert_eq!(m.get(&CampaignType::Growth), Some(&30.0));
        assert_eq!(m.get(&CampaignType::GodPower), Some(&30.0));
        assert_eq!(m.len(), 7);

        // Not improved: just the base.
        let mut m = BTreeMap::new();
        ov.apply("Cupid", &mut m, false, false);
        assert_eq!(m, BTreeMap::from([(CampaignType::Divinity, 100.0)]));
    }

    #[test]
    fn test_set_correction_and_unknown_pet() {
        let ov = overrides_from("Cat:\n  - when: Always\n    set: { GodPower: 50 }\n");
        let mut m = BTreeMap::new();
        ov.apply("Cat", &mut m, false, false);
        assert_eq!(m.get(&CampaignType::GodPower), Some(&50.0));

        // Pet with no rules is untouched.
        let mut m = BTreeMap::from([(CampaignType::Food, 10.0)]);
        ov.apply("Frog", &mut m, false, false);
        assert_eq!(m, BTreeMap::from([(CampaignType::Food, 10.0)]));
    }
}
