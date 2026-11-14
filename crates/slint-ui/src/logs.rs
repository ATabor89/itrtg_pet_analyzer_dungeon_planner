//! Dungeon log presentation over the shared parser. Logs stay in memory only.
use itrtg_planner::log_parser::{self, DungeonLog, ItemEntry};
use crate::app::number;

#[derive(Default)]
pub struct LogModel {
    pub log: Option<DungeonLog>,
    pub pet: usize,
    pub metric: usize,
    pub room: usize,
    pub all_rooms: bool,
}

pub struct RoomRow {
    pub room: String, pub done: String, pub taken: String, pub healed: String, pub net: String,
    pub value: f32, pub party: f32, pub healing: f32,
}

fn numeric(text: &str) -> u64 { text.replace(',', "").parse().unwrap_or(0) }
fn signed(value: i128) -> String {
    if value < 0 { format!("-{}", number((-value).min(u64::MAX as i128) as u64)) }
    else { format!("+{}", number(value.min(u64::MAX as i128) as u64)) }
}

impl LogModel {
    pub fn replace(&mut self, log: DungeonLog) {
        self.log = Some(log);
        self.pet = 0;
        self.room = 0;
        self.all_rooms = false;
        self.select_pet(0);
    }
    pub fn select_pet(&mut self, index: usize) {
        self.pet = self.log.as_ref().map(|l| index.min(l.room_stats.len().saturating_sub(1))).unwrap_or(0);
        self.metric = match self.selected_class() { "Supporter" => 2, "Defender" => 1, _ => 0 };
    }
    fn selected_class(&self) -> &str {
        self.log.as_ref().and_then(|l| l.room_stats.get(self.pet).and_then(|r| l.pets.iter().find(|p| p.name == r.pet_name)))
            .map(|p| p.class.as_str()).unwrap_or("")
    }
    pub fn supporter(&self) -> bool { self.selected_class() == "Supporter" }
    pub fn party(&self) -> Vec<(String, String)> {
        let Some(log) = &self.log else { return Vec::new(); };
        let order = log_parser::compute_party_order(&log.pets, include_str!("../../../data/dungeon_recommendations.yaml"), &log.dungeon_name, &log.dungeon_level);
        let totals = log_parser::compute_totals(log);
        order.into_iter().map(|index| {
            let pet = &log.pets[index];
            let mut body = format!("{} · DL {} · CL {}\nGrowth {}", pet.class, pet.level, pet.class_level, pet.growth);
            if let Some(&(done,taken,healed)) = totals.get(&pet.name) {
                body.push_str(&format!("\nDealt {} · Taken {} · Healed {}", number(done), number(taken), number(healed)));
            }
            (pet.name.clone(),body)
        }).collect()
    }
    pub fn overview(&self) -> Vec<(String, String)> {
        let Some(log) = &self.log else { return Vec::new(); };
        let mut summary: Vec<_> = log.summary.xp_gained.iter().map(|(pet,xp)| format!("{pet}: +{xp} XP")).collect();
        if let Some((turns,room)) = log.summary.longest_turn { summary.push(format!("Longest fight: {turns} turns in room {room}")); }
        for text in [&log.summary.wipe_line,&log.summary.leech_line,&log.summary.free_exp_line].into_iter().flatten() { summary.push(text.clone()); }
        let mut result = vec![("Summary".into(), if summary.is_empty() { "No summary recorded.".into() } else { summary.join("\n") })];
        let events: Vec<_> = log.events.iter().map(|e| format!("Room {} · {} · {}{}",e.room,e.event_type,e.outcome,e.detail.as_ref().map(|v| format!("\n{v}")).unwrap_or_default()))
            .chain(log.deaths.iter().map(|d|format!("{} killed by {} — room {}, turn {}",d.pet_name,d.killed_by,d.room,d.turn))).collect();
        if !events.is_empty() { result.push(("Events & deaths".into(),events.join("\n\n"))); }
        for (label,items) in [("Items used",&log.items_used),("Items found",&log.items_found),("Items from events",&log.items_from_events)] {
            if !items.is_empty() { result.push((label.into(),items.iter().map(item_text).collect::<Vec<_>>().join("\n"))); }
        }
        for pet in &log.depth_stats {
            result.push((format!("Depth stats · {}",pet.pet_name),pet.depths.iter().map(|d| format!("Depth {} · Dealt {} · Taken {} · Healed {}",d.depth,d.damage_done,d.damage_taken,d.healed.as_deref().unwrap_or("—"))).collect::<Vec<_>>().join("\n")));
        }
        result
    }
    pub fn room_rows(&self) -> Vec<RoomRow> {
        let Some(log) = &self.log else { return Vec::new(); };
        let Some(pet) = log.room_stats.get(self.pet) else { return Vec::new(); };
        let mut party_taken = std::collections::HashMap::<u32,u64>::new();
        for pet in &log.room_stats { for room in &pet.rooms {
            let value = party_taken.entry(room.room).or_default();
            *value = value.saturating_add(numeric(&room.damage_taken));
        } }
        let values: Vec<_> = pet.rooms.iter().map(|r| match self.metric { 1 => numeric(&r.damage_taken), 2 => r.healed.as_deref().map(numeric).unwrap_or(0), _ => numeric(&r.damage_done) }).collect();
        let max = values.iter().copied().max().unwrap_or(0).max(1) as f64;
        let party_max = party_taken.values().copied().max().unwrap_or(0).max(1) as f64;
        pet.rooms.iter().zip(values).map(|(r,value)| {
            let done=numeric(&r.damage_done); let own_taken=numeric(&r.damage_taken);
            let healing=r.healed.as_deref().map(numeric).unwrap_or(0);
            let party=*party_taken.get(&r.room).unwrap_or(&0);
            let taken=if self.supporter() { party } else { own_taken };
            let net=if self.supporter() { healing as i128-taken as i128 } else { done as i128-taken as i128+healing as i128 };
            RoomRow { room:r.room.to_string(),done:r.damage_done.clone(),taken:number(taken),healed:r.healed.clone().unwrap_or_else(||"—".into()),net:signed(net),
                value:(value as f64/max) as f32,party:(party as f64/party_max) as f32,healing:(healing.min(party) as f64/party_max) as f32 }
        }).collect()
    }
    pub fn pet_choices(&self) -> Vec<(usize,String)> {
        let Some(log)=&self.log else { return Vec::new(); };
        let order=log_parser::compute_party_order(&log.pets,include_str!("../../../data/dungeon_recommendations.yaml"),&log.dungeon_name,&log.dungeon_level);
        let mut result: Vec<_>=order.into_iter().filter_map(|i| {
            let pet=&log.pets[i];
            log.room_stats.iter().position(|r|r.pet_name==pet.name).map(|index|(index,format!("{} ({})",pet.name,pet.class)))
        }).collect();
        for (index,pet) in log.room_stats.iter().enumerate() {
            if !result.iter().any(|(i,_)|*i==index) { result.push((index,pet.pet_name.clone())); }
        }
        result
    }
    pub fn combat(&self) -> Vec<(String,String)> {
        if self.all_rooms {
            return (0..self.log.as_ref().map(|l|l.rooms.len()).unwrap_or(0)).flat_map(|index|self.combat_room(index)).collect();
        }
        self.combat_room(self.room)
    }
    fn combat_room(&self, index: usize) -> Vec<(String,String)> {
        let Some(room)=self.log.as_ref().and_then(|l|l.rooms.get(index)) else { return Vec::new(); };
        let mut intro=Vec::new();
        if let Some(event)=&room.event_type { intro.push(format!("Event: {event}")); }
        intro.extend(room.pet_hp.iter().map(|(name,hp)|format!("{name}: {hp} HP")));
        if room.has_monster_header { intro.push(if room.monsters.is_empty() { "Monsters: none".into() } else { format!("Monsters: {}",room.monsters.iter().map(|(name,hp)|format!("{name}: {hp}")).collect::<Vec<_>>().join(", ")) }); }
        if !room.traps.is_empty() { intro.push(format!("Traps: {}",room.traps)); }
        intro.extend(room.notes.iter().cloned());
        let mut result=vec![(format!("Room {}",room.room_number),intro.join("\n"))];
        result.extend(room.turns.iter().map(|turn| {
            let body=turn.pet_hp_line.iter().cloned().chain(turn.actions.iter().cloned()).collect::<Vec<_>>().join("\n");
            (format!("Turn {}",turn.turn_number),body)
        }));
        if !room.drops.is_empty() { result.push(("Drops".into(),room.drops.join("\n"))); }
        result
    }
}
fn item_text(item: &ItemEntry) -> String {
    format!("{} × {}{}",item.count,item.name,item.detail.as_ref().map(|d|format!(" · {d}")).unwrap_or_default())
}
pub fn room_label(room: &log_parser::RoomCombatLog) -> String {
    let monsters=if room.has_monster_header { format!("{} monsters",room.monsters.len()) } else { "monsters unknown".into() };
    format!("Room {} · {monsters} · {} turns{}",room.room_number,room.turns.len(),room.event_type.as_ref().map(|e|format!(" · {e}")).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(name: &str) -> String {
        std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/dungeon_logs").join(name)).unwrap()
    }
    #[test]
    fn all_committed_logs_project_every_overview_and_combat_section() {
        for file in ["2026-03-29_21-33_WaterTemple.html","2026-03-29_21-33_Scrapyard.html","2026-03-29_21-33_Forest.html","death_2026-03-30_09-36_Scrapyard.html"] {
            let parsed=log_parser::parse_dungeon_log(&fixture(file)).unwrap();
            let mut state=LogModel::default(); state.replace(parsed);
            let log=state.log.as_ref().unwrap();
            assert_eq!(state.party().len(),log.pets.len());
            let all=state.overview().into_iter().map(|(_,body)|body).collect::<Vec<_>>().join("\n");
            for death in &log.deaths { assert!(all.contains(&death.pet_name)); }
            for items in [&log.items_used,&log.items_found,&log.items_from_events] {
                for item in items { assert!(all.contains(&item.name)); }
            }
            let expected: Vec<_>=log.rooms.iter().map(|r|(r.turns.len(),r.drops.clone())).collect();
            for (index,(turns,drops)) in expected.iter().enumerate() {
                state.room=index;
                let sections=state.combat();
                assert_eq!(sections.iter().filter(|(title,_)|title.starts_with("Turn ")).count(),*turns);
                for drop in drops { assert!(sections.iter().any(|(_,body)|body.contains(drop))); }
            }
        }
    }
    #[test]
    fn supporter_rows_compare_healing_against_party_taken() {
        use log_parser::{PetRoomStats,RoomStat};
        let mut log=log_parser::parse_dungeon_log(&fixture("2026-03-29_21-33_Forest.html")).unwrap();
        log.pets[0].name="Healer".into(); log.pets[0].class="Supporter".into();
        log.room_stats=vec![
            PetRoomStats {pet_name:"Healer".into(),rooms:vec![RoomStat {room:7,damage_done:"10".into(),damage_taken:"30".into(),healed:Some("80".into())}]},
            PetRoomStats {pet_name:"Tank".into(),rooms:vec![RoomStat {room:7,damage_done:"40".into(),damage_taken:"70".into(),healed:None}]},
        ];
        let mut state=LogModel::default();state.replace(log);
        assert_eq!(state.metric,2);assert!(state.supporter());
        let rows=state.room_rows();assert_eq!(rows[0].taken,"100");assert_eq!(rows[0].net,"-20");
        assert_eq!(rows[0].value,1.0);assert_eq!(rows[0].party,1.0);assert!((rows[0].healing-0.8).abs()<0.001);
        state.select_pet(1);assert!(!state.supporter());assert_eq!(state.metric,0);
        assert_eq!(state.room_rows()[0].net,"-30");
    }
    #[test]
    fn invalid_log_is_rejected_and_sparse_log_keeps_valid_empty_sections() {
        assert!(log_parser::parse_dungeon_log("not a log").is_err());
        let mut state=LogModel::default();assert!(state.overview().is_empty());assert!(state.combat().is_empty());
        state.replace(log_parser::parse_dungeon_log(&fixture("2026-03-29_21-33_Forest.html")).unwrap());
        state.log.as_mut().unwrap().room_stats.clear();state.select_pet(999);
        assert!(state.room_rows().is_empty());
        state.room=usize::MAX;assert!(state.combat().is_empty());
    }
    #[test]
    fn all_rooms_and_navigation_preserve_missing_monster_information() {
        let mut state=LogModel::default();state.replace(log_parser::parse_dungeon_log(&fixture("2026-03-29_21-33_Forest.html")).unwrap());
        let log=state.log.as_mut().unwrap();log.rooms[0].has_monster_header=false;log.rooms[0].monsters.clear();
        assert!(room_label(&log.rooms[0]).contains("monsters unknown"));
        log.rooms[0].has_monster_header=true;assert!(room_label(&log.rooms[0]).contains("0 monsters"));
        let expected=log.rooms.len();state.all_rooms=true;
        assert_eq!(state.combat().iter().filter(|(title,_)|title.starts_with("Room ")).count(),expected);
        for (index,label) in state.pet_choices() { assert!(label.contains(&state.log.as_ref().unwrap().room_stats[index].pet_name)); }
    }

}
