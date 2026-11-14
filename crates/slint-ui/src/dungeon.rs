//! Dungeon commands and display projections. Solving and gear rules stay shared.
use itrtg_models::dungeon::{CounterCondition, DungeonRecommendations};
use itrtg_models::planner_config::PlannerConfig;
use itrtg_models::{Dungeon, Quality, normalize_for_lookup};
use itrtg_planner::dungeon_state::{
    ConstraintsState, EquipmentStandardOverride, EventOverride, ForcedEntry,
};
use itrtg_planner::dungeon_support::{equip_matches_rec, resolve_equip_name};
use itrtg_planner::{
    equipment,
    merge::MergedPet,
    solver::{self, Assignment, DungeonPlan, DungeonRequest, SolverConstraints},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

pub const DUNGEONS: [(Dungeon, &str); 5] = [
    (Dungeon::Scrapyard, "Scrapyard"),
    (Dungeon::WaterTemple, "Water Temple"),
    (Dungeon::Volcano, "Volcano"),
    (Dungeon::Mountain, "Mountain"),
    (Dungeon::Forest, "Forest"),
];
pub fn label(d: Dungeon) -> &'static str {
    DUNGEONS
        .iter()
        .find(|(key, _)| *key == d)
        .map(|(_, s)| *s)
        .unwrap_or("Unknown")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub selected: [bool; 5],
    pub depths: [u8; 5],
    pub constraints: ConstraintsState,
    pub events: Vec<EventOverride>,
    pub standards: BTreeMap<Dungeon, EquipmentStandardOverride>,
    pub inventory: BTreeMap<String, u8>,
    pub team_dungeons: BTreeMap<u8, Dungeon>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            selected: [false; 5],
            depths: [1; 5],
            constraints: Default::default(),
            events: Vec::new(),
            standards: BTreeMap::new(),
            inventory: BTreeMap::new(),
            team_dungeons: BTreeMap::new(),
        }
    }
}
impl Settings {
    pub fn validate(&mut self) -> Result<(), String> {
        for depth in &mut self.depths {
            *depth = (*depth).clamp(1, 4);
        }
        for pin in &mut self.constraints.forced {
            if pin.dungeon.is_none() {
                pin.slot = None;
            }
            if pin.slot.is_some_and(|s| !(1..=6).contains(&s)) {
                return Err("Dungeon slot must be 1–6".into());
            }
        }
        for standard in self.standards.values_mut() {
            standard.min_tier = standard.min_tier.map(|n| n.clamp(1, 4));
            standard.min_upgrade = standard.min_upgrade.map(|n| n.min(20));
        }
        Ok(())
    }
    pub fn constraints(&self) -> SolverConstraints {
        let mut out = SolverConstraints {
            event_overrides: self
                .events
                .iter()
                .map(|e| ((e.dungeon, e.depth, e.event.clone()), e.enabled))
                .collect(),
            ..Default::default()
        };
        if !self.constraints.enabled {
            return out;
        }
        out.forbidden = self.constraints.forbidden.iter().cloned().collect();
        out.whitelisted = self.constraints.whitelisted.iter().cloned().collect();
        for pin in &self.constraints.forced {
            if out.forbidden.contains(&pin.pet) {
                continue;
            }
            if let Some(d) = pin.dungeon {
                out.forced.entry(d).or_default().push(pin.pet.clone());
                if let Some(slot) = pin.slot {
                    out.forced_slots
                        .insert((d, pin.pet.clone()), slot.saturating_sub(1) as usize);
                }
            } else {
                out.forced_any.push(pin.pet.clone());
            }
        }
        out
    }
    pub fn import_constraints(&mut self, text: &str, pets: &[MergedPet]) -> Result<(), String> {
        let mut parsed: ConstraintsState = serde_yaml::from_str(text).map_err(|e| e.to_string())?;
        for name in parsed
            .forbidden
            .iter_mut()
            .chain(parsed.whitelisted.iter_mut())
        {
            *name = resolve(name, pets)?;
        }
        for pin in &mut parsed.forced {
            pin.pet = resolve(&pin.pet, pets)?;
            if pin.dungeon.is_none() {
                pin.slot = None;
            }
            if pin.slot.is_some_and(|s| !(1..=6).contains(&s)) {
                return Err("Dungeon slot must be 1–6. Previous constraints kept.".into());
            }
        }
        self.constraints = parsed;
        Ok(())
    }
    pub fn constrain(
        &mut self,
        kind: &str,
        name: &str,
        dungeon: i32,
        slot: i32,
        pets: &[MergedPet],
    ) -> Result<(), String> {
        let name = resolve(name, pets)?;
        self.constraints.forbidden.retain(|n| n != &name);
        self.constraints.whitelisted.retain(|n| n != &name);
        self.constraints.forced.retain(|p| p.pet != name);
        match kind {
            "forbid" => self.constraints.forbidden.push(name),
            "allow" => self.constraints.whitelisted.push(name),
            "force" => self.constraints.forced.push(ForcedEntry {
                pet: name,
                dungeon: DUNGEONS
                    .get(dungeon.wrapping_sub(1) as usize)
                    .map(|(d, _)| *d),
                slot: if dungeon > 0 && (1..=6).contains(&slot) {
                    Some(slot as u8)
                } else {
                    None
                },
            }),
            _ => {}
        }
        Ok(())
    }
    pub fn apply_teams(
        &mut self,
        teams: &[pet_importer::parser::DungeonTeam],
        choices: &[Option<Dungeon>],
        pets: &[MergedPet],
    ) -> Result<(), String> {
        if teams.len() != choices.len() {
            return Err("Parse teams again before applying.".into());
        }
        let mut mapped = HashSet::new();
        let mut names = HashSet::new();
        let mut added = Vec::new();
        let mut mapping = BTreeMap::new();
        for (team, choice) in teams.iter().zip(choices) {
            let Some(d) = choice else { continue };
            if !mapped.insert(*d) {
                return Err(format!("{} is assigned to more than one team.", label(*d)));
            }
            mapping.insert(team.index, *d);
            for m in &team.members {
                let name = resolve(&m.name, pets)?;
                if !names.insert(name.clone()) {
                    return Err(format!("{name} appears in more than one imported slot."));
                }
                added.push(ForcedEntry {
                    pet: name,
                    dungeon: Some(*d),
                    slot: Some(m.slot),
                });
            }
        }
        if mapped.is_empty() {
            return Err("Assign at least one team to a dungeon.".into());
        }
        self.constraints
            .forced
            .retain(|f| !f.dungeon.is_some_and(|d| mapped.contains(&d)) && !names.contains(&f.pet));
        self.constraints.forbidden.retain(|p| !names.contains(p));
        self.constraints.whitelisted.retain(|p| !names.contains(p));
        self.constraints.forced.extend(added);
        self.constraints.enabled = true;
        self.team_dungeons.extend(mapping);
        Ok(())
    }
}
fn resolve(input: &str, pets: &[MergedPet]) -> Result<String, String> {
    let key = normalize_for_lookup(input.trim());
    pets.iter()
        .find(|p| normalize_for_lookup(&p.name) == key)
        .or_else(|| {
            pets.iter().find(|p| {
                p.export
                    .as_ref()
                    .is_some_and(|e| normalize_for_lookup(&e.export_name) == key)
            })
        })
        .map(|p| p.name.clone())
        .ok_or_else(|| format!("Unknown pet: {input}. Previous constraints kept."))
}

pub type PlanCards = (String, Vec<(String, String)>, Vec<(String, String)>);
pub struct Model {
    pub recs: DungeonRecommendations,
    pub config: PlannerConfig,
    pub plans: Vec<DungeonPlan>,
    pub dirty: bool,
    pub search: String,
    pub pending_teams: Vec<pet_importer::parser::DungeonTeam>,
    pub team_choices: Vec<Option<Dungeon>>,
}
impl Model {
    pub fn new() -> Result<Self, String> {
        let catalog = serde_yaml::from_str(include_str!("../../../data/equipment_catalog.yaml"))
            .map_err(|e| e.to_string())?;
        let file = serde_yaml::from_str(include_str!("../../../data/dungeon_recommendations.yaml"))
            .map_err(|e| e.to_string())?;
        let config = serde_yaml::from_str(include_str!("../../../data/planner_config.yaml"))
            .map_err(|e| e.to_string())?;
        let special = serde_yaml::from_str(include_str!("../../../data/pet_special_info.yaml"))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            recs: DungeonRecommendations::new(catalog, file),
            config: PlannerConfig::new(config, special),
            plans: Vec::new(),
            dirty: false,
            search: String::new(),
            pending_teams: Vec::new(),
            team_choices: Vec::new(),
        })
    }
    pub fn solve(
        recs: &DungeonRecommendations,
        config: &PlannerConfig,
        settings: &Settings,
        pets: &[MergedPet],
    ) -> Vec<DungeonPlan> {
        let requests: Vec<_> = DUNGEONS
            .iter()
            .enumerate()
            .filter(|(i, _)| settings.selected[*i])
            .filter_map(|(i, (d, _))| {
                recs.dungeons.get(d).map(|data| DungeonRequest {
                    dungeon: *d,
                    depth: settings.depths[i],
                    data,
                })
            })
            .collect();
        let mut plans = solver::solve_multi(&requests, pets, &settings.constraints(), Some(config));
        for plan in &mut plans {
            equipment::enrich_equipment(plan, &recs.equipment, Some(config));
        }
        plans
    }
    pub fn events(&self, settings: &Settings) -> Vec<(usize, u8, String, bool, String)> {
        let mut rows = Vec::new();
        for (i, (d, name)) in DUNGEONS
            .iter()
            .enumerate()
            .filter(|(i, _)| settings.selected[*i])
        {
            for (depth, dd) in &self.recs.dungeons[d].depths {
                if *depth > settings.depths[i] {
                    continue;
                }
                for event in &dd.events {
                    let enabled = settings
                        .events
                        .iter()
                        .find(|e| e.dungeon == *d && e.depth == *depth && e.event == event.name)
                        .map(|e| e.enabled)
                        .unwrap_or(!event.optional);
                    rows.push((
                        i,
                        *depth,
                        event.name.clone(),
                        enabled,
                        format!(
                            "{name} D{depth} · {} · {}% · {}{}",
                            event.name,
                            event.chance_pct,
                            event
                                .countered_by
                                .iter()
                                .map(counter)
                                .collect::<Vec<_>>()
                                .join("; "),
                            if event.optional { " (optional)" } else { "" }
                        ),
                    ));
                }
            }
        }
        rows
    }
    pub fn preview(&self, settings: &Settings) -> Vec<(String, String)> {
        DUNGEONS.iter().enumerate().filter(|(i,_)|settings.selected[*i]).filter_map(|(i,(d,name))| {
            let depth=settings.depths[i]; let dd=self.recs.dungeons[d].depths.get(&depth)?;
            let r=&dd.requirements;
            let mut body=format!("Avg DL {} · CL {} · Minimum growth {}\n{} rooms × {} monsters · Gem level {}\n",r.dungeon_level_avg,r.class_level,r.total_growth.map(crate::app::number).unwrap_or_else(||"—".into()),dd.rooms,dd.monsters_per_room,dd.gem_level.map(|v|v.to_string()).unwrap_or_else(||"—".into()));
            for (i,slot) in dd.party.iter().enumerate() {
                body+=&format!("\nSlot {} ({}): {} / {}",i+1,if i<3 {"front"}else{"back"},optional(slot.class),optional(slot.element));
                if let Some(e)=&slot.equipment { body+=&format!("\n  {}",[e.weapon.as_deref(),e.armor.as_deref(),e.accessory.as_deref()].into_iter().flatten().map(|k|resolve_equip_name(k,Some(&self.recs.equipment))).collect::<Vec<_>>().join(" / ")); }
            }
            for (i,slot) in dd.party.iter().enumerate() {
                if let Some(g)=slot.equipment.as_ref().and_then(|e|e.gems.as_ref()) {
                    body+=&format!("\nSlot {} gems: Weapon {} / Armor {} / Accessory {}",i+1,optional(g.weapon),optional(g.armor),optional(g.accessory));
                }
            }
            body+=&format!("\n\nBring: {}",dd.party_items.iter().map(|item|format!("{} × {}",item.quantity,self.recs.items.get(&item.item).map(|i|i.name.as_str()).unwrap_or(&item.item))).collect::<Vec<_>>().join(", "));
            Some((format!("{name} D{depth} requirements"),body))
        }).collect()
    }
    pub fn cards(&self, settings: &Settings) -> Vec<PlanCards> {
        self.plans.iter().map(|plan| {
            let cards=plan.assignments.iter().map(|sa| {
                let mut body=format!("Required: {} / {}",optional(sa.slot.class),optional(sa.slot.element));
                let title=match &sa.assignment {
                    Assignment::Empty { suggestions }=> { body+=&format!("\nUnlock suggestions:\n{}",suggestions.iter().map(|s|format!("{} · {:?}",s.pet.name,s.unlock_difficulty)).collect::<Vec<_>>().join("\n"));format!("Slot {} · Empty",sa.position+1) },
                    Assignment::Filled { pet,quality }=> {
                        body+=&format!("\nMatch: {quality:?}");
                        if let Some(e)=&pet.export { body+=&format!("\n{} / {} · DL {} · CL {}\nBase growth {}",optional(e.class),optional(pet.element()),e.dungeon_level,e.class_level,crate::app::number(e.growth)); }
                        if let Some(future)=sa.future_class { body+=&format!("\nD{} eventually wants {:?}",future.from_depth,future.class); }
                        body+=&self.gear(sa,settings,plan);
                        if let Some(info)=self.config.special_info(&pet.name) {
                            for m in info.special_mechanics.iter().filter(|m|m.combat_relevant==Some(true)) { body+=&format!("\n{}: {}",m.name.as_deref().unwrap_or("Mechanic").replace('_'," "),m.description.as_deref().unwrap_or("")); }
                            if let Some(m)=&info.stat_modifiers {
                                for (name,v) in [("SPD",m.speed),("HP",m.hp),("ATK",m.attack),("DEF",m.defense)] {if let Some(v)=v {body+=&format!("\n{name} {v:+}%");}}
                            }
                            if pet.export.as_ref().is_some_and(|e|e.improved) && let Some(token)=&info.token_improvement {body+=&format!("\nToken improvement: {}",token.description.as_deref().unwrap_or("Improved"));}
                            for syn in info.team_synergies() { if let Some(other)=&syn.pet && other!=&pet.name && plan.assignments.iter().any(|a|matches!(&a.assignment,Assignment::Filled{pet,..} if pet.name.eq_ignore_ascii_case(other))) { body+=&format!("\nSynergy: {other}"); } }
                            for other in plan.assignments.iter().filter_map(|a|match &a.assignment {Assignment::Filled {pet:other,..} if other.name!=pet.name=>Some(other),_=>None}) {
                                if info.has_dungeon_anti_synergy_with(&other.name) {body+=&format!("\nAnti-synergy: {}",other.name);}
                            }
                        }
                        format!("Slot {} · {}",sa.position+1,pet.name)
                    }
                };
                (title,body)
            }).collect();
            let mut sections=Vec::new();
            let exports:Vec<_>=plan.assignments.iter().filter_map(|a|match &a.assignment {Assignment::Filled{pet,..}=>pet.export.as_ref(),_=>None}).collect();
            let avg=exports.iter().map(|e|u64::from(e.dungeon_level)).sum::<u64>()/6;
            let min_cl=exports.iter().map(|e|e.class_level).min().unwrap_or(0);
            let min_growth=exports.iter().map(|e|e.growth).min().unwrap_or(0);
            let mut stats=format!("{}/6 filled · Avg DL {avg} · Min CL {min_cl} · Min growth {}",exports.len(),crate::app::number(min_growth));
            for depth in 1..=plan.depth {
                if let Some(dd)=self.recs.dungeons[&plan.dungeon].depths.get(&depth) {
                    let r=&dd.requirements; let per=u64::from(r.levels_per_difficulty.last().copied().unwrap_or(5)).max(1);
                    stats+=&format!("\nD{depth}: difficulty {}{}{}",(avg.saturating_sub(u64::from(r.dungeon_level_avg))/per).min(10),if min_cl<r.class_level {format!(" · need CL {}",r.class_level)}else{String::new()},if r.total_growth.is_some_and(|g|min_growth<g) {" · growth below requirement"}else{""});
                }
            }
            sections.push(("Team & difficulty".into(),stats));
            if !plan.warnings.is_empty() { sections.push(("Coverage warnings".into(),plan.warnings.iter().map(|w|format!("D{} {:?} · {}: {}",w.source_depth,w.kind,w.name,w.detail)).collect::<Vec<_>>().join("\n"))); }
            let hazards:Vec<_>=self.recs.dungeons[&plan.dungeon].depths.iter().filter(|(d,_)|**d<=plan.depth).flat_map(|(d,dd)|dd.traps.iter().filter(|t|!t.countered_by.is_actionable()).map(move|t|format!("D{d} · {} ({}%): {}",t.name,t.chance_pct,t.countered_by.notes.as_deref().unwrap_or("Uncounterable")))).collect();
            if !hazards.is_empty() { sections.push(("Unavoidable hazards".into(),hazards.join("\n"))); }
            (format!("{} D{}",label(plan.dungeon),plan.depth),cards,sections)
        }).collect()
    }
    fn gear(&self, sa: &solver::SlotAssignment, settings: &Settings, plan: &DungeonPlan) -> String {
        let Some(rec) = &sa.equipment_suggestion else {
            return String::new();
        };
        let Assignment::Filled { pet, .. } = &sa.assignment else {
            return String::new();
        };
        let loadout = pet.export.as_ref().map(|e| &e.loadout);
        let s = settings.standards.get(&plan.dungeon);
        let tier = s.and_then(|s| s.min_tier).unwrap_or(1).max(plan.depth);
        let quality = s
            .and_then(|s| s.min_quality)
            .unwrap_or(Quality::S)
            .max(Quality::S);
        let upgrade = s.and_then(|s| s.min_upgrade).unwrap_or(10).max(10);
        let source = match rec.source {
            equipment::EquipmentSource::Static => "Dungeon recommendation".into(),
            equipment::EquipmentSource::Computed => "Class recommendation".into(),
            equipment::EquipmentSource::Propagated { from_depth } => format!("From D{from_depth} recommendation"),
        };
        let mut text = format!("\n\n{source} · minimum T{tier} {quality:?}+{upgrade}");
        let gems = rec.equipment.gems.as_ref();
        for (label, key, current, gem) in [
            (
                "Weapon",
                rec.equipment.weapon.as_deref(),
                loadout.and_then(|l| l.weapon.as_ref()),
                gems.and_then(|g| g.weapon),
            ),
            (
                "Armor",
                rec.equipment.armor.as_deref(),
                loadout.and_then(|l| l.armor.as_ref()),
                gems.and_then(|g| g.armor),
            ),
            (
                "Accessory",
                rec.equipment.accessory.as_deref(),
                loadout.and_then(|l| l.accessory.as_ref()),
                gems.and_then(|g| g.accessory),
            ),
        ] {
            let Some(key) = key else { continue };
            text += &format!(
                "\n{label}: {}",
                resolve_equip_name(key, Some(&self.recs.equipment))
            );
            if rec.source == equipment::EquipmentSource::Static
                && let Some(preferred) = self
                    .config
                    .special_info(&pet.name)
                    .and_then(|i| i.priority_element_override())
                && let Some(element) = self.recs.equipment.lookup(key).and_then(|e| e.element)
                && element != preferred
            {
                text += &format!(" · pet prefers {preferred:?} gear");
            }
            if let Some(cur) = current {
                let line = equip_matches_rec(&cur.name, key, Some(&self.recs.equipment));
                let cur_tier = self
                    .recs
                    .equipment
                    .find_key_by_name_exact(&cur.name)
                    .and_then(|k| self.recs.equipment.lookup(k))
                    .map(|e| e.tier)
                    .unwrap_or(0);
                text += &format!(
                    "\n  Have {} · T{cur_tier} {:?}+{} · {}",
                    cur.name,
                    cur.quality,
                    cur.upgrade_level.unwrap_or(0),
                    if line
                        && cur_tier >= tier
                        && cur.quality >= quality
                        && cur.upgrade_level.unwrap_or(0) >= upgrade
                    {
                        "meets standard"
                    } else {
                        "needs change/upgrade"
                    }
                );
            } else {
                text += "\n  Have: none";
            }
            if let Some(gem) = gem {
                text += &format!(
                    "\n  Gem {gem:?}: {}",
                    if current.and_then(|c| c.gem) == Some(gem) {
                        "matches"
                    } else {
                        "needed"
                    }
                );
            }
        }
        text
    }
    pub fn shopping(&self) -> Vec<(String, String)> {
        let mut unlock = Vec::new();
        let mut evolve = Vec::new();
        let mut gear = Vec::new();
        let mut gems = BTreeMap::new();
        for plan in &self.plans {
            for sa in &plan.assignments {
                match &sa.assignment {
                    Assignment::Empty { suggestions } => unlock.push(format!(
                        "{} slot {}: {}{}",
                        label(plan.dungeon),
                        sa.position + 1,
                        optional(sa.slot.class),
                        suggestions
                            .first()
                            .map(|s| format!(" · unlock {}", s.pet.name))
                            .unwrap_or_default()
                    )),
                    Assignment::Filled { pet, quality } => {
                        if *quality == solver::MatchQuality::Evolvable {
                            evolve.push(format!("{} → {}", pet.name, optional(sa.slot.class)));
                        }
                        if let Some(rec) = &sa.equipment_suggestion {
                            let loadout = pet.export.as_ref().map(|e| &e.loadout);
                            let gs = rec.equipment.gems.as_ref();
                            for (slot, key, cur, gem) in [
                                (
                                    "Weapon",
                                    rec.equipment.weapon.as_deref(),
                                    loadout.and_then(|l| l.weapon.as_ref()),
                                    gs.and_then(|g| g.weapon),
                                ),
                                (
                                    "Armor",
                                    rec.equipment.armor.as_deref(),
                                    loadout.and_then(|l| l.armor.as_ref()),
                                    gs.and_then(|g| g.armor),
                                ),
                                (
                                    "Accessory",
                                    rec.equipment.accessory.as_deref(),
                                    loadout.and_then(|l| l.accessory.as_ref()),
                                    gs.and_then(|g| g.accessory),
                                ),
                            ] {
                                if let Some(key) = key
                                    && !cur.is_some_and(|c| {
                                        equip_matches_rec(&c.name, key, Some(&self.recs.equipment))
                                    })
                                {
                                    gear.push(format!(
                                        "{slot}: {}{}",
                                        resolve_equip_name(key, Some(&self.recs.equipment)),
                                        cur.map(|c| format!(" (have: {})", c.name))
                                            .unwrap_or_default()
                                    ));
                                }
                                if let Some(gem) = gem
                                    && cur.and_then(|c| c.gem) != Some(gem)
                                {
                                    *gems.entry(gem).or_insert(0u32) += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        gear.sort();
        gear.dedup();
        evolve.sort();
        evolve.dedup();
        [
            ("Pets to unlock", unlock),
            ("Pets to evolve", evolve),
            ("Equipment needed", gear),
            (
                "Gems needed",
                gems.into_iter()
                    .map(|(g, n)| format!("{n} × {g:?}"))
                    .collect(),
            ),
        ]
        .into_iter()
        .filter(|(_, items)| !items.is_empty())
        .map(|(name, items)| (name.into(), items.join("\n")))
        .collect()
    }
}
fn optional<T: std::fmt::Debug>(value: Option<T>) -> String {
    value
        .map(|v| format!("{v:?}"))
        .unwrap_or_else(|| "Any".into())
}
fn counter(c: &CounterCondition) -> String {
    let mut s = Vec::new();
    if let Some(class) = c.class {
        s.push(format!("{class:?}"));
    }
    if let Some(e) = c.element {
        s.push(format!("{e:?}"));
    }
    if let Some(item) = &c.item {
        s.push(item.replace('_', " "));
    }
    if let Some(n) = c.count {
        s.push(format!("×{n}"));
    }
    if let Some(n) = c.quantity_per_clear {
        s.push(format!("{n} per clear"));
    }
    if let Some(note) = &c.notes {
        s.push(note.clone());
    }
    s.join(" + ")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> crate::app::AppModel {
        let mut app = crate::app::AppModel::new(Default::default()).unwrap();
        app.import(crate::app::EXAMPLE_EXPORT, "Example").unwrap();
        app
    }
    #[test]
    fn constraint_toggle_keeps_events_and_converts_slots() {
        let mut s = Settings::default();
        s.constraints.forced.push(ForcedEntry {
            pet: "Penguin".into(),
            dungeon: Some(Dungeon::WaterTemple),
            slot: Some(6),
        });
        s.events.push(EventOverride {
            dungeon: Dungeon::WaterTemple,
            depth: 2,
            event: "Treasure".into(),
            enabled: false,
        });
        assert_eq!(
            s.constraints().forced_slots[&(Dungeon::WaterTemple, "Penguin".into())],
            5
        );
        s.constraints.forbidden.push("Penguin".into());
        assert!(s.constraints().forced.is_empty());
        s.constraints.enabled = false;
        let c = s.constraints();
        assert!(c.forbidden.is_empty());
        assert_eq!(c.event_overrides.len(), 1);
    }
    #[test]
    fn invalid_imports_keep_constraints_and_aliases_resolve() {
        let app = app();
        let mut s = Settings::default();
        s.import_constraints("forbidden: [Penguin]", &app.pets)
            .unwrap();
        let before = serde_yaml::to_string(&s).unwrap();
        assert!(
            s.import_constraints(
                "forced: [{pet: Penguin, dungeon: WaterTemple, slot: 9}]",
                &app.pets
            )
            .is_err()
        );
        assert!(
            s.import_constraints("forbidden: [NoSuchPet]", &app.pets)
                .is_err()
        );
        assert_eq!(before, serde_yaml::to_string(&s).unwrap());
        let pet = app
            .pets
            .iter()
            .find(|p| p.export.as_ref().is_some_and(|e| e.export_name != p.name))
            .unwrap();
        assert_eq!(
            resolve(&pet.export.as_ref().unwrap().export_name, &app.pets).unwrap(),
            pet.name
        );
    }
    #[test]
    fn team_mapping_is_atomic_scoped_and_remembered() {
        let app = app();
        let mut s = Settings::default();
        s.constrain("force", "Penguin", 2, 1, &app.pets).unwrap();
        s.constrain("forbid", "Rabbit", 0, 0, &app.pets).unwrap();
        s.constrain("force", "Clam", 1, 3, &app.pets).unwrap();
        let teams = pet_importer::parser::parse_dungeon_teams(
            "---DungeonTeamsStart---0:Rabbit=6,;1:Penguin=1,;---DungeonTeamsEnd---",
        )
        .unwrap();
        let before = serde_yaml::to_string(&s).unwrap();
        assert!(
            s.apply_teams(
                &teams,
                &[Some(Dungeon::WaterTemple), Some(Dungeon::WaterTemple)],
                &app.pets
            )
            .is_err()
        );
        assert_eq!(before, serde_yaml::to_string(&s).unwrap());
        s.apply_teams(&teams, &[Some(Dungeon::WaterTemple), None], &app.pets)
            .unwrap();
        assert!(s.constraints.forbidden.is_empty());
        assert!(!s.constraints.forced.iter().any(|p| p.pet == "Penguin"));
        assert!(
            s.constraints
                .forced
                .iter()
                .any(|p| p.pet == "Clam" && p.slot == Some(3))
        );
        assert!(
            s.constraints
                .forced
                .iter()
                .any(|p| p.pet == "Rabbit" && p.slot == Some(6))
        );
        assert_eq!(s.team_dungeons[&0], Dungeon::WaterTemple);
    }
    #[test]
    fn solve_preserves_shared_assignment_and_equipment_behavior() {
        let mut app = app();
        let mut settings = Settings {
            selected: [true; 5],
            depths: [2; 5],
            ..Default::default()
        };
        settings
            .constrain("force", "Penguin", 2, 6, &app.pets)
            .unwrap();
        let model = &mut app.dungeon;
        let plans = Model::solve(&model.recs, &model.config, &settings, &app.pets);
        let requests: Vec<_> = DUNGEONS
            .iter()
            .map(|(d, _)| DungeonRequest {
                dungeon: *d,
                depth: 2,
                data: &model.recs.dungeons[d],
            })
            .collect();
        let mut expected = solver::solve_multi(
            &requests,
            &app.pets,
            &settings.constraints(),
            Some(&model.config),
        );
        for p in &mut expected {
            equipment::enrich_equipment(p, &model.recs.equipment, Some(&model.config));
        }
        assert_eq!(format!("{plans:?}"), format!("{expected:?}"));
        let mut names = HashSet::new();
        for p in &plans {
            for a in &p.assignments {
                if let Assignment::Filled { pet, .. } = &a.assignment {
                    assert!(names.insert(pet.name.clone()));
                }
            }
        }
        let water = plans
            .iter()
            .find(|p| p.dungeon == Dungeon::WaterTemple)
            .unwrap();
        assert!(
            matches!(&water.assignments[5].assignment,Assignment::Filled{pet,..} if pet.name=="Penguin")
        );
        model.plans = plans;
        assert_eq!(model.cards(&settings).len(), 5);
        assert_eq!(model.preview(&settings).len(), 5);
        assert!(!model.events(&settings).is_empty());
        assert!(!model.shopping().is_empty());
        let exported = itrtg_planner::dungeon_support::build_dungeon_teams_export(&model.plans);
        assert_eq!(
            pet_importer::parser::parse_dungeon_teams(&exported)
                .unwrap()
                .len(),
            5
        );
    }
    #[test]
    fn old_sessions_default_dungeons_and_roster_changes_mark_plans_stale() {
        let old = crate::app::Session::from_yaml("version: 1").unwrap();
        assert_eq!(old.dungeon.depths, [1; 5]);
        let mut app = app();
        app.session.dungeon.selected[0] = true;
        app.dungeon.plans = Model::solve(
            &app.dungeon.recs,
            &app.dungeon.config,
            &app.session.dungeon,
            &app.pets,
        );
        app.dungeon.dirty = false;
        app.import(crate::app::EXAMPLE_EXPORT, "Updated").unwrap();
        assert!(app.dungeon.dirty);
        let yaml = serde_yaml::to_string(&app.session).unwrap();
        assert!(!yaml.contains("assignments:"));
        assert!(
            crate::app::Session::from_yaml(&yaml)
                .unwrap()
                .dungeon
                .selected[0]
        );
    }
}
