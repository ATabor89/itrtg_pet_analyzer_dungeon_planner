/// Parser for ITRTG dungeon log HTML files.
///
/// These files are plain HTML with `<br>` line breaks and `<b>` bold tags,
/// containing structured dungeon run data (pets, stats, items, events, combat).
use std::collections::HashMap;

// =============================================================================
// Data structures
// =============================================================================

#[derive(Debug, Clone)]
pub struct DungeonLog {
    pub pets: Vec<PetInfo>,
    pub dungeon_name: String,
    pub dungeon_level: String,
    pub rating: String,
    pub room_count: u32,
    pub summary: SummaryInfo,
    pub items_used: Vec<ItemEntry>,
    pub items_found: Vec<ItemEntry>,
    pub items_from_events: Vec<ItemEntry>,
    pub events: Vec<EventInfo>,
    pub deaths: Vec<DeathInfo>,
    pub depth_stats: Vec<PetDepthStats>,
    pub room_stats: Vec<PetRoomStats>,
    pub rooms: Vec<RoomCombatLog>,
}

#[derive(Debug, Clone)]
pub struct PetInfo {
    pub name: String,
    pub class: String,
    pub growth: String,
    pub level: u32,
    pub class_level: u32,
}

#[derive(Debug, Clone)]
pub struct SummaryInfo {
    pub xp_gained: Vec<(String, String)>,
    pub longest_turn: Option<(u32, u32)>,
    pub leech_line: Option<String>,
    pub free_exp_line: Option<String>,
    /// Set when the whole party was wiped, e.g.
    /// "Your whole party died in room 32, turn 14 and they lost all items they found!"
    pub wipe_line: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ItemEntry {
    pub count: u32,
    pub name: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EventInfo {
    pub room: u32,
    pub event_type: String,
    pub outcome: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeathInfo {
    pub pet_name: String,
    pub killed_by: String,
    pub room: u32,
    pub turn: u32,
}

#[derive(Debug, Clone)]
pub struct PetDepthStats {
    pub pet_name: String,
    pub depths: Vec<DepthStat>,
}

#[derive(Debug, Clone)]
pub struct DepthStat {
    pub depth: u32,
    pub damage_done: String,
    pub damage_taken: String,
    pub healed: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PetRoomStats {
    pub pet_name: String,
    pub rooms: Vec<RoomStat>,
}

#[derive(Debug, Clone)]
pub struct RoomStat {
    pub room: u32,
    pub damage_done: String,
    pub damage_taken: String,
    pub healed: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RoomCombatLog {
    pub room_number: u32,
    /// Set when the room is an event room (e.g. "Treasure", "Fog", "Wounded Pet").
    pub event_type: Option<String>,
    pub pet_hp: Vec<(String, String)>,
    /// Empty when the log has no Monsters: header (some event rooms omit it).
    pub monsters: Vec<(String, String)>,
    /// `true` when a Monsters: header was present (even if zero monsters).
    pub has_monster_header: bool,
    pub traps: String,
    /// Pre-combat notes: event narrative, modifier notices (e.g. "Mist Sphere decreases...").
    pub notes: Vec<String>,
    pub turns: Vec<TurnInfo>,
    pub drops: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TurnInfo {
    pub turn_number: u32,
    pub pet_hp_line: Option<String>,
    pub actions: Vec<String>,
}

// =============================================================================
// Parsing
// =============================================================================

/// Strip HTML tags from a string.
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(c);
        }
    }
    out
}

/// Split HTML content by `<br>` tags into lines, stripping tags from each line.
fn html_to_lines(html: &str) -> Vec<String> {
    // Split on <br> (and <br/> and <br />) variants
    let normalized = html
        .replace("<br/>", "<br>")
        .replace("<br />", "<br>")
        .replace("<BR>", "<br>");

    normalized
        .split("<br>")
        .map(|s| strip_tags(s).trim().to_string())
        .collect()
}

pub fn parse_dungeon_log(html: &str) -> Result<DungeonLog, String> {
    let lines = html_to_lines(html);
    let lines: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();

    let mut idx = 0;

    // Skip leading blank lines
    while idx < lines.len() && lines[idx].is_empty() {
        idx += 1;
    }

    // --- Pets ---
    if idx >= lines.len() || !lines[idx].starts_with("Pets used:") {
        return Err("Expected 'Pets used:' at start of log".into());
    }
    idx += 1;

    let mut pets = Vec::new();
    while idx < lines.len() && !lines[idx].is_empty() {
        if let Some(pet) = parse_pet_line(lines[idx]) {
            pets.push(pet);
        }
        idx += 1;
    }
    // Build the name lookup immediately so it can be used throughout the parser.
    let pet_names: Vec<String> = pets.iter().map(|p| p.name.clone()).collect();

    // Skip blanks
    while idx < lines.len() && lines[idx].is_empty() {
        idx += 1;
    }

    // --- Dungeon result header ---
    let (dungeon_name, dungeon_level, rating, room_count) = if idx < lines.len()
        && lines[idx].starts_with("Dungeon result for")
    {
        parse_dungeon_header(lines[idx])
    } else {
        ("Unknown".into(), "?".into(), "".into(), 0)
    };
    idx += 1;

    // Skip blanks
    while idx < lines.len() && lines[idx].is_empty() {
        idx += 1;
    }

    // Skip "Summary" header if present
    if idx < lines.len() && lines[idx] == "Summary" {
        idx += 1;
    }
    while idx < lines.len() && lines[idx].is_empty() {
        idx += 1;
    }

    // --- XP and summary ---
    let mut xp_gained = Vec::new();
    let mut longest_turn = None;
    let mut leech_line = None;
    let mut free_exp_line = None;
    let mut wipe_line = None;

    while idx < lines.len() {
        let line = lines[idx];
        if line.is_empty() {
            // Check if next non-empty line is still summary-related
            let mut peek = idx + 1;
            while peek < lines.len() && lines[peek].is_empty() {
                peek += 1;
            }
            if peek < lines.len()
                && (lines[peek].starts_with("Longest")
                    || lines[peek].starts_with("Your")
                    || lines[peek].contains("gained a total"))
            {
                idx += 1;
                continue;
            }
            break;
        }

        if line.contains("gained a total of") {
            if let Some(pos) = line.find(" gained a total of ") {
                let name = line[..pos].to_string();
                let rest = &line[pos + " gained a total of ".len()..];
                let xp = rest.replace(" experience.", "").trim().to_string();
                xp_gained.push((name, xp));
            }
        } else if line.starts_with("Longest turn counter:") {
            // "Longest turn counter: 5 (room 13)"
            let rest = line.strip_prefix("Longest turn counter: ").unwrap_or("");
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 3 {
                let turns = parts[0].parse().unwrap_or(0);
                let room_num = rest
                    .split("(room ")
                    .nth(1)
                    .and_then(|s| s.split(')').next())
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                longest_turn = Some((turns, room_num));
            }
        } else if line.contains("exp leech weapon") {
            leech_line = Some(line.to_string());
        } else if line.contains("free exp pool") {
            free_exp_line = Some(line.to_string());
        } else if line.contains("whole party died") || line.contains("all items they found") {
            wipe_line = Some(line.to_string());
        } else if line.starts_with("Items Used:")
            || line.starts_with("You found")
            || line.starts_with("Event in Room")
        {
            break;
        }
        idx += 1;
    }

    while idx < lines.len() && lines[idx].is_empty() {
        idx += 1;
    }

    // SummaryInfo is constructed *after* the items sections so that any
    // leech/free-exp lines that appear after "Items Used:" are also captured.

    // --- Items Used ---
    let mut items_used = Vec::new();
    if idx < lines.len() && lines[idx] == "Items Used:" {
        idx += 1;
        while idx < lines.len() && !lines[idx].is_empty() {
            if let Some(item) = parse_item_line(lines[idx]) {
                items_used.push(item);
            }
            idx += 1;
        }
    }

    while idx < lines.len() && lines[idx].is_empty() {
        idx += 1;
    }

    // --- Items found (without events) ---
    let mut items_found = Vec::new();
    if idx < lines.len() && lines[idx].starts_with("You found (without events)") {
        idx += 1;
        while idx < lines.len() && !lines[idx].is_empty() {
            if let Some(item) = parse_item_line(lines[idx]) {
                items_found.push(item);
            }
            idx += 1;
        }
    }

    while idx < lines.len() && lines[idx].is_empty() {
        idx += 1;
    }

    // --- Items found (from events) ---
    let mut items_from_events = Vec::new();
    if idx < lines.len() && lines[idx].starts_with("You found (from events)") {
        idx += 1;
        while idx < lines.len() && !lines[idx].is_empty() {
            if let Some(item) = parse_item_line(lines[idx]) {
                items_from_events.push(item);
            }
            idx += 1;
        }
    }

    while idx < lines.len() && lines[idx].is_empty() {
        idx += 1;
    }

    // Some logs print leech/free-exp summary lines *after* the Items sections
    // (e.g. death logs where "You found" sections are absent).  Consume any
    // such lingering summary / narrative lines before the Events section.
    // We stop as soon as we reach an "Event in Room" line, a "was killed by"
    // death line, or a line that looks like the start of depth stats.
    while idx < lines.len() && !lines[idx].is_empty() {
        let line = lines[idx];
        if line.starts_with("Event in Room") || line.contains("was killed by") {
            break;
        }
        let trimmed = line.trim_end_matches(':');
        if pet_names.contains(&trimmed.to_string()) {
            break; // beginning of depth-stats section
        }
        // Capture leech / free-exp lines wherever they appear.
        if line.contains("exp leech weapon") {
            leech_line = Some(line.to_string());
        } else if line.contains("free exp pool") {
            free_exp_line = Some(line.to_string());
        }
        // All other unrecognised narrative lines are silently consumed.
        idx += 1;
    }

    while idx < lines.len() && lines[idx].is_empty() {
        idx += 1;
    }

    // Build SummaryInfo here, after all items and post-items summary lines have
    // been consumed, so leech_line / free_exp_line are fully populated.
    let summary = SummaryInfo {
        xp_gained,
        longest_turn,
        leech_line,
        free_exp_line,
        wipe_line,
    };

    // --- Events ---
    let mut events = Vec::new();
    while idx < lines.len() && lines[idx].starts_with("Event in Room") {
        if let Some(ev) = parse_event_line(lines[idx]) {
            events.push(ev);
        }
        idx += 1;
    }

    while idx < lines.len() && lines[idx].is_empty() {
        idx += 1;
    }

    // --- Deaths ---
    let mut deaths = Vec::new();
    while idx < lines.len() && lines[idx].contains("was killed by") {
        if let Some(d) = parse_death_line(lines[idx]) {
            deaths.push(d);
        }
        idx += 1;
    }

    // After individual pet deaths the log may include a party-wipe notification
    // such as "All pets died in room 32, turn 14 from a combined, ultimate
    // attack." which does not contain "was killed by".  Skip any such
    // narrative lines before the depth-stats section without losing our place.
    while idx < lines.len() {
        let line = lines[idx];
        if line.is_empty() {
            idx += 1;
            continue;
        }
        // Reached depth-stats section (a pet name followed by ':').
        let trimmed = line.trim_end_matches(':');
        if pet_names.contains(&trimmed.to_string()) {
            break;
        }
        // Reached combat-log section.
        if line.starts_with("Room ") && !line.contains("damage") {
            break;
        }
        // Any other non-blank line here is a narrative/wipe notification — skip.
        idx += 1;
    }

    // --- Depth stats ---
    let mut depth_stats = Vec::new();

    while idx < lines.len() {
        let line = lines[idx];
        // Check if this line is a pet name followed by ':'
        let trimmed = line.trim_end_matches(':');
        if pet_names.contains(&trimmed.to_string()) {
            let pet_name = trimmed.to_string();
            idx += 1;
            let mut depths = Vec::new();
            while idx < lines.len() && lines[idx].starts_with("Depth ") {
                if let Some(ds) = parse_depth_stat_line(lines[idx]) {
                    depths.push(ds);
                }
                idx += 1;
            }
            if !depths.is_empty() {
                depth_stats.push(PetDepthStats { pet_name, depths });
            }
            // Skip blanks between pets
            while idx < lines.len() && lines[idx].is_empty() {
                idx += 1;
            }
            // Check if next section is room stats (starts with pet name + Room N)
            if idx < lines.len() {
                let next_trimmed = lines[idx].trim_end_matches(':');
                if pet_names.contains(&next_trimmed.to_string()) {
                    // Peek ahead: is the line after this "Room N:" ?
                    if idx + 1 < lines.len() && lines[idx + 1].starts_with("Room ") {
                        break;
                    }
                }
            }
        } else {
            break;
        }
    }

    // --- Room stats ---
    let mut room_stats = Vec::new();

    while idx < lines.len() {
        let line = lines[idx];
        let trimmed = line.trim_end_matches(':');
        if pet_names.contains(&trimmed.to_string()) {
            let pet_name = trimmed.to_string();
            idx += 1;
            let mut rooms = Vec::new();
            while idx < lines.len() && lines[idx].starts_with("Room ") {
                // Make sure it's a stat line (contains "damage done") not a combat log room header
                if lines[idx].contains("damage done") || lines[idx].contains("damage taken") {
                    if let Some(rs) = parse_room_stat_line(lines[idx]) {
                        rooms.push(rs);
                    }
                } else {
                    break;
                }
                idx += 1;
            }
            if !rooms.is_empty() {
                room_stats.push(PetRoomStats { pet_name, rooms });
            }
            while idx < lines.len() && lines[idx].is_empty() {
                idx += 1;
            }
        } else {
            break;
        }
    }

    while idx < lines.len() && lines[idx].is_empty() {
        idx += 1;
    }

    // --- Detailed combat logs ---
    let mut rooms = Vec::new();
    while idx < lines.len() {
        if lines[idx].starts_with("Room ") && !lines[idx].contains("damage") {
            let room = parse_room_combat(&lines, &mut idx);
            rooms.push(room);
        } else {
            idx += 1;
        }
    }

    Ok(DungeonLog {
        pets,
        dungeon_name,
        dungeon_level,
        rating,
        room_count,
        summary,
        items_used,
        items_found,
        items_from_events,
        events,
        deaths,
        depth_stats,
        room_stats,
        rooms,
    })
}

// =============================================================================
// Line parsers
// =============================================================================

fn parse_pet_line(line: &str) -> Option<PetInfo> {
    // "Rudolph, Rogue: Growth 8,634, Level 43, Class Level 10"
    let comma_pos = line.find(", ")?;
    let name = line[..comma_pos].to_string();
    let rest = &line[comma_pos + 2..];

    let colon_pos = rest.find(": ")?;
    let class = rest[..colon_pos].to_string();
    let stats = &rest[colon_pos + 2..];

    // Verify "Growth" prefix is present
    let _ = stats.strip_prefix("Growth ")?;
    // Growth can have commas in the number, so find the next ", Level"
    let level_pos = stats.find(", Level ")?;
    let growth = stats["Growth ".len()..level_pos].to_string();
    let after_level = &stats[level_pos + ", Level ".len()..];
    let class_level_pos = after_level.find(", Class Level ")?;
    let level: u32 = after_level[..class_level_pos].parse().ok()?;
    let class_level: u32 = after_level[class_level_pos + ", Class Level ".len()..]
        .parse()
        .ok()?;

    Some(PetInfo {
        name,
        class,
        growth,
        level,
        class_level,
    })
}

fn parse_dungeon_header(line: &str) -> (String, String, String, u32) {
    // "Dungeon result for Scrapyard 2 (8/1/0/0), 15 rooms"
    let rest = line
        .strip_prefix("Dungeon result for ")
        .unwrap_or(line);

    let paren_start = rest.find('(').unwrap_or(rest.len());
    let before_paren = rest[..paren_start].trim();

    // Split name and level: last word before '(' is the level number
    let parts: Vec<&str> = before_paren.split_whitespace().collect();
    let (name, level) = if parts.len() >= 2 {
        if parts.last().unwrap().chars().all(|c| c.is_ascii_digit()) {
            (
                parts[..parts.len() - 1].join(" "),
                parts.last().unwrap().to_string(),
            )
        } else {
            (before_paren.to_string(), "?".to_string())
        }
    } else {
        (before_paren.to_string(), "?".to_string())
    };

    let rating = rest
        .get(paren_start..)
        .and_then(|s| s.find(')').map(|end| s[..=end].to_string()))
        .unwrap_or_default();

    let room_count = rest
        .rsplit(", ")
        .next()
        .and_then(|s| s.strip_suffix(" rooms"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    (name, level, rating, room_count)
}

fn parse_item_line(line: &str) -> Option<ItemEntry> {
    // "4 x Torch (29 + 32 left)" or "8 x Herb"
    let x_pos = line.find(" x ")?;
    let count: u32 = line[..x_pos].trim().parse().ok()?;
    let rest = &line[x_pos + 3..];
    let (name, detail) = if let Some(paren) = rest.find(" (") {
        (
            rest[..paren].to_string(),
            Some(rest[paren + 1..].trim_end_matches(')').to_string()),
        )
    } else {
        (rest.to_string(), None)
    };
    Some(ItemEntry {
        count,
        name,
        detail,
    })
}

fn parse_event_line(line: &str) -> Option<EventInfo> {
    // "Event in Room 6, Wounded Pet: failed to clear."
    // "Event in Room 10, Fog: cleared with success. Your experience is increased by 100%"
    let rest = line.strip_prefix("Event in Room ")?;
    let comma = rest.find(", ")?;
    let room: u32 = rest[..comma].parse().ok()?;
    let after_room = &rest[comma + 2..];
    let colon = after_room.find(": ")?;
    let event_type = after_room[..colon].to_string();
    let outcome_rest = &after_room[colon + 2..];

    let (outcome, detail) = if let Some(dot) = outcome_rest.find(". ") {
        (
            outcome_rest[..dot].to_string(),
            Some(outcome_rest[dot + 2..].trim_end_matches('.').to_string()),
        )
    } else {
        (outcome_rest.trim_end_matches('.').to_string(), None)
    };

    Some(EventInfo {
        room,
        event_type,
        outcome,
        detail,
    })
}

fn parse_death_line(line: &str) -> Option<DeathInfo> {
    // "Dragon was killed by Microbots in room 12, turn 2."
    let killed_pos = line.find(" was killed by ")?;
    let pet_name = line[..killed_pos].to_string();
    let rest = &line[killed_pos + " was killed by ".len()..];
    let in_room = rest.find(" in room ")?;
    let killed_by = rest[..in_room].to_string();
    let after_room = &rest[in_room + " in room ".len()..];
    let comma = after_room.find(", turn ")?;
    let room: u32 = after_room[..comma].parse().ok()?;
    let turn: u32 = after_room[comma + ", turn ".len()..]
        .trim_end_matches('.')
        .parse()
        .ok()?;
    Some(DeathInfo {
        pet_name,
        killed_by,
        room,
        turn,
    })
}

fn parse_depth_stat_line(line: &str) -> Option<DepthStat> {
    // "Depth 1: 3,277 damage done, 0 damage taken."
    // "Depth 2: 1,141 damage done, 1,016 damage taken, healed 8,134 hp."
    let rest = line.strip_prefix("Depth ")?;
    let colon = rest.find(": ")?;
    let depth: u32 = rest[..colon].parse().ok()?;
    let stats = &rest[colon + 2..];

    let done_end = stats.find(" damage done")?;
    let damage_done = stats[..done_end].to_string();

    let taken_start = stats.find(", ")? + 2;
    let taken_end = stats[taken_start..].find(" damage taken").map(|p| taken_start + p)?;
    let damage_taken = stats[taken_start..taken_end].to_string();

    let healed = if let Some(heal_start) = stats.find("healed ") {
        let hp_val = &stats[heal_start + "healed ".len()..];
        Some(hp_val.trim_end_matches(" hp.").trim_end_matches('.').to_string())
    } else {
        None
    };

    Some(DepthStat {
        depth,
        damage_done,
        damage_taken,
        healed,
    })
}

fn parse_room_stat_line(line: &str) -> Option<RoomStat> {
    // "Room 1: 791 damage done, 0 damage taken."
    let rest = line.strip_prefix("Room ")?;
    let colon = rest.find(": ")?;
    let room: u32 = rest[..colon].parse().ok()?;
    let stats = &rest[colon + 2..];

    let done_end = stats.find(" damage done")?;
    let damage_done = stats[..done_end].to_string();

    let taken_start = stats.find(", ")? + 2;
    let taken_end = stats[taken_start..].find(" damage taken").map(|p| taken_start + p)?;
    let damage_taken = stats[taken_start..taken_end].to_string();

    let healed = if let Some(heal_start) = stats.find("healed ") {
        let hp_val = &stats[heal_start + "healed ".len()..];
        Some(hp_val.trim_end_matches(" hp.").trim_end_matches('.').to_string())
    } else {
        None
    };

    Some(RoomStat {
        room,
        damage_done,
        damage_taken,
        healed,
    })
}

/// Parse a list of "Name N,NNN HP" entries from a room header line.
///
/// Numbers in HP values may contain commas (e.g. "Armadillo 3,901 HP"), so we
/// cannot split the whole line on ", ". Instead we find each " HP" occurrence
/// and work backwards to extract the number and name.
fn parse_creature_hp_list(line: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut remaining = line;

    while let Some(hp_pos) = remaining.find(" HP") {
        let creature_str = &remaining[..hp_pos];
        // The HP number is the last whitespace-separated token.
        if let Some(space) = creature_str.rfind(' ') {
            let name = creature_str[..space].to_string();
            let hp = creature_str[space + 1..].to_string();
            result.push((name, hp));
        }
        // Advance past " HP" and the optional ", " separator.
        remaining = &remaining[hp_pos + 3..]; // len(" HP") == 3
        if remaining.starts_with(", ") {
            remaining = &remaining[2..];
        } else {
            break;
        }
    }

    result
}

/// Returns true if `line` looks like the start of a combat turn or action.
fn is_combat_start(line: &str) -> bool {
    // Turn header: "Turn 1", "Turn 1:", "Turn 2: Rudolph hp:..."
    if line.starts_with("Turn ") {
        return true;
    }
    // Priority-ordered action: "842: Squirrel attacked..."
    // A number optionally with commas, followed by ": "
    if let Some(colon) = line.find(": ") {
        let prefix = &line[..colon];
        if prefix.chars().all(|c| c.is_ascii_digit() || c == ',') && !prefix.is_empty() {
            return true;
        }
    }
    false
}

fn parse_room_combat(lines: &[&str], idx: &mut usize) -> RoomCombatLog {
    // "Room 1" header
    let header = lines[*idx];
    let room_number: u32 = header
        .strip_prefix("Room ")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    *idx += 1;

    // -------------------------------------------------------------------------
    // Pre-combat scan — flexible order.
    //
    // Normal rooms:     Pets → Monsters → Traps → [notes] → Turn N
    // Event rooms:      "Event Xxx:" → narrative → Pets → Monsters → Traps → Turn N
    // Treasure/Fog evt: "Event Xxx:" → narrative → Pets → Turn N  (no Monsters/Traps)
    //
    // We scan every line until the first Turn/action line, routing each line to
    // the appropriate bucket.  Blanks are skipped silently.
    // -------------------------------------------------------------------------
    let mut pet_hp: Vec<(String, String)> = Vec::new();
    let mut monsters: Vec<(String, String)> = Vec::new();
    let mut has_monster_header = false;
    let mut traps = "None".to_string();
    let mut notes: Vec<String> = Vec::new();
    let mut event_type: Option<String> = None;

    loop {
        if *idx >= lines.len() {
            break;
        }
        let line = lines[*idx];

        // Blank lines — skip silently.
        if line.is_empty() {
            *idx += 1;
            continue;
        }

        // Start of combat — stop pre-combat scan.
        if is_combat_start(line) {
            break;
        }

        // End of room from the outside — stop.
        if (line.starts_with("Room ") && !line.contains("damage"))
            || line.contains(" gave ")
            || line.contains(" dropped ")
        {
            break;
        }

        if line.starts_with("Pets:") {
            let rest = line.strip_prefix("Pets: ").unwrap_or("");
            pet_hp = parse_creature_hp_list(rest);
            *idx += 1;
        } else if line.starts_with("Monsters:") {
            let rest = line.strip_prefix("Monsters: ").unwrap_or("");
            monsters = parse_creature_hp_list(rest);
            has_monster_header = true;
            *idx += 1;
        } else if line.starts_with("Traps:") {
            traps = line.strip_prefix("Traps: ").unwrap_or("None").to_string();
            *idx += 1;
        } else {
            // Event header or narrative, modifier notice, etc.
            // Detect "Event Xxx:" lines (e.g. "Event Fog:", "Event Treasure:").
            if let Some(rest) = line.strip_prefix("Event ") {
                let kind = rest.trim_end_matches(':').to_string();
                event_type = Some(kind);
            }
            notes.push(line.to_string());
            *idx += 1;
        }
    }

    // Turns
    let mut turns = Vec::new();
    let mut current_turn: Option<TurnInfo> = None;

    while *idx < lines.len() {
        let line = lines[*idx];

        if line.is_empty() {
            *idx += 1;
            // Check if we've hit drops or next room
            while *idx < lines.len() && lines[*idx].is_empty() {
                *idx += 1;
            }
            if *idx >= lines.len() {
                break;
            }
            // If next non-empty line is a new room, drops, or end of combat
            if lines[*idx].starts_with("Room ") && !lines[*idx].contains("damage") {
                break;
            }
            if lines[*idx].contains(" gave ") || lines[*idx].contains(" dropped ") {
                break;
            }
            continue;
        }

        if line.starts_with("Turn ") {
            if let Some(turn) = current_turn.take() {
                turns.push(turn);
            }
            let turn_num = line
                .strip_prefix("Turn ")
                .and_then(|s| s.split(':').next())
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            let hp_line = if line.contains("hp:") || line.contains("HP") {
                Some(line.to_string())
            } else {
                None
            };
            current_turn = Some(TurnInfo {
                turn_number: turn_num,
                pet_hp_line: hp_line,
                actions: Vec::new(),
            });
            *idx += 1;
            continue;
        }

        // Check for drops (end of combat)
        if line.contains(" gave ") || line.contains(" dropped ") {
            break;
        }

        // Check for next room
        if line.starts_with("Room ") && !line.contains("damage") {
            break;
        }

        // Combat action line
        if let Some(turn) = current_turn.as_mut() {
            turn.actions.push(line.to_string());
        }
        *idx += 1;
    }

    if let Some(turn) = current_turn.take() {
        turns.push(turn);
    }

    // Drops
    let mut drops = Vec::new();
    while *idx < lines.len() {
        let line = lines[*idx];
        if line.is_empty() {
            *idx += 1;
            // Skip blanks but peek for more drops
            while *idx < lines.len() && lines[*idx].is_empty() {
                *idx += 1;
            }
            if *idx < lines.len()
                && (lines[*idx].contains(" gave ") || lines[*idx].contains(" dropped "))
            {
                continue;
            }
            break;
        }
        if line.contains(" gave ") || line.contains(" dropped ") {
            drops.push(line.to_string());
            *idx += 1;
        } else {
            break;
        }
    }

    RoomCombatLog {
        room_number,
        event_type,
        pet_hp,
        monsters,
        has_monster_header,
        traps,
        notes,
        turns,
        drops,
    }
}

/// Compute aggregate stats from room stats.
pub fn compute_totals(log: &DungeonLog) -> HashMap<String, (u64, u64, u64)> {
    let mut totals: HashMap<String, (u64, u64, u64)> = HashMap::new();
    for pet_rooms in &log.room_stats {
        let entry = totals.entry(pet_rooms.pet_name.clone()).or_default();
        for r in &pet_rooms.rooms {
            let done = r.damage_done.replace(',', "").parse::<u64>().unwrap_or(0);
            let taken = r.damage_taken.replace(',', "").parse::<u64>().unwrap_or(0);
            let healed = r
                .healed
                .as_ref()
                .map(|h| h.replace(',', "").parse::<u64>().unwrap_or(0))
                .unwrap_or(0);
            entry.0 += done;
            entry.1 += taken;
            entry.2 += healed;
        }
    }
    totals
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a minimal valid log HTML from `<br>` separated lines.
    fn make_log(body: &str) -> String {
        // The parser splits on <br> and strips tags, so raw text with <br> works.
        body.to_string()
    }

    // -------------------------------------------------------------------------
    // strip_tags / html_to_lines
    // -------------------------------------------------------------------------

    #[test]
    fn test_strip_tags() {
        assert_eq!(strip_tags("<b>hello</b> world"), "hello world");
        assert_eq!(strip_tags("no tags"), "no tags");
        assert_eq!(strip_tags("<br/>"), "");
    }

    #[test]
    fn test_html_to_lines() {
        let lines = html_to_lines("one<br>two<BR>three<br/>four");
        assert_eq!(lines, vec!["one", "two", "three", "four"]);
    }

    // -------------------------------------------------------------------------
    // Individual line parsers
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_pet_line() {
        let pet = parse_pet_line("Rudolph, Rogue: Growth 8,634, Level 43, Class Level 10")
            .expect("should parse");
        assert_eq!(pet.name, "Rudolph");
        assert_eq!(pet.class, "Rogue");
        assert_eq!(pet.growth, "8,634");
        assert_eq!(pet.level, 43);
        assert_eq!(pet.class_level, 10);
    }

    #[test]
    fn test_parse_pet_line_none_class() {
        let pet = parse_pet_line("Bat, None: Growth 100, Level 1, Class Level 0")
            .expect("should parse");
        assert_eq!(pet.class, "None");
        assert_eq!(pet.level, 1);
    }

    #[test]
    fn test_parse_dungeon_header() {
        let (name, level, rating, rooms) =
            parse_dungeon_header("Dungeon result for Scrapyard 2 (8/1/0/0), 15 rooms");
        assert_eq!(name, "Scrapyard");
        assert_eq!(level, "2");
        assert_eq!(rating, "(8/1/0/0)");
        assert_eq!(rooms, 15);
    }

    #[test]
    fn test_parse_dungeon_header_multi_word_name() {
        let (name, level, _, rooms) =
            parse_dungeon_header("Dungeon result for Water Temple 3 (5/2/1/0), 20 rooms");
        assert_eq!(name, "Water Temple");
        assert_eq!(level, "3");
        assert_eq!(rooms, 20);
    }

    #[test]
    fn test_parse_item_line() {
        let item = parse_item_line("4 x Torch (29 + 32 left)").expect("should parse");
        assert_eq!(item.count, 4);
        assert_eq!(item.name, "Torch");
        assert!(item.detail.is_some());

        let item2 = parse_item_line("8 x Herb").expect("should parse");
        assert_eq!(item2.count, 8);
        assert_eq!(item2.name, "Herb");
        assert!(item2.detail.is_none());
    }

    #[test]
    fn test_parse_event_line() {
        let ev = parse_event_line("Event in Room 6, Wounded Pet: failed to clear.")
            .expect("should parse");
        assert_eq!(ev.room, 6);
        assert_eq!(ev.event_type, "Wounded Pet");
        assert_eq!(ev.outcome, "failed to clear");
        assert!(ev.detail.is_none());

        let ev2 = parse_event_line(
            "Event in Room 10, Fog: cleared with success. Your experience is increased by 100%",
        )
        .expect("should parse");
        assert_eq!(ev2.room, 10);
        assert_eq!(ev2.outcome, "cleared with success");
        assert_eq!(
            ev2.detail.as_deref(),
            Some("Your experience is increased by 100%")
        );
    }

    #[test]
    fn test_parse_death_line() {
        let d = parse_death_line("Dragon was killed by Microbots in room 12, turn 2.")
            .expect("should parse");
        assert_eq!(d.pet_name, "Dragon");
        assert_eq!(d.killed_by, "Microbots");
        assert_eq!(d.room, 12);
        assert_eq!(d.turn, 2);
    }

    #[test]
    fn test_parse_depth_stat_line() {
        let ds = parse_depth_stat_line("Depth 1: 3,277 damage done, 0 damage taken.")
            .expect("should parse");
        assert_eq!(ds.depth, 1);
        assert_eq!(ds.damage_done, "3,277");
        assert_eq!(ds.damage_taken, "0");
        assert!(ds.healed.is_none());
    }

    #[test]
    fn test_parse_depth_stat_with_healed() {
        let ds = parse_depth_stat_line(
            "Depth 2: 1,141 damage done, 1,016 damage taken, healed 8,134 hp.",
        )
        .expect("should parse");
        assert_eq!(ds.depth, 2);
        assert_eq!(ds.damage_done, "1,141");
        assert_eq!(ds.damage_taken, "1,016");
        assert_eq!(ds.healed.as_deref(), Some("8,134"));
    }

    #[test]
    fn test_parse_room_stat_line() {
        let rs =
            parse_room_stat_line("Room 1: 791 damage done, 0 damage taken.").expect("should parse");
        assert_eq!(rs.room, 1);
        assert_eq!(rs.damage_done, "791");
        assert_eq!(rs.damage_taken, "0");
    }

    #[test]
    fn test_parse_creature_hp_list() {
        let list = parse_creature_hp_list("Rudolph 1,234 HP, Dog 567 HP");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], ("Rudolph".into(), "1,234".into()));
        assert_eq!(list[1], ("Dog".into(), "567".into()));
    }

    #[test]
    fn test_parse_creature_hp_list_single() {
        let list = parse_creature_hp_list("Slime 999 HP");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0], ("Slime".into(), "999".into()));
    }

    // -------------------------------------------------------------------------
    // Full log parsing — minimal log
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_minimal_log() {
        let html = make_log(
            "<br>Pets used: \
             <br>Alpha, Rogue: Growth 100, Level 5, Class Level 2\
             <br>Beta, Mage: Growth 200, Level 10, Class Level 4\
             <br>\
             <br>Dungeon result for Forest 1 (3/0/0/0), 3 rooms\
             <br>\
             <br>Summary\
             <br>\
             <br>Alpha gained a total of 500 experience.\
             <br>Beta gained a total of 500 experience.\
             <br>\
             <br>Longest turn counter: 2 (room 3)\
             <br>",
        );
        let log = parse_dungeon_log(&html).expect("should parse minimal log");
        assert_eq!(log.pets.len(), 2);
        assert_eq!(log.pets[0].name, "Alpha");
        assert_eq!(log.pets[1].name, "Beta");
        assert_eq!(log.dungeon_name, "Forest");
        assert_eq!(log.dungeon_level, "1");
        assert_eq!(log.room_count, 3);
        assert_eq!(log.summary.xp_gained.len(), 2);
        assert_eq!(log.summary.xp_gained[0].1, "500");
        assert_eq!(log.summary.longest_turn, Some((2, 3)));
    }

    #[test]
    fn test_parse_log_with_wipe() {
        let html = make_log(
            "<br>Pets used: \
             <br>Alpha, Rogue: Growth 100, Level 5, Class Level 2\
             <br>\
             <br>Dungeon result for Scrapyard 2 (8/1/0/0), 32 rooms\
             <br>\
             <br>Summary\
             <br>\
             <br>Alpha gained a total of 1,000 experience.\
             <br>\
             <br>Longest turn counter: 5 (room 16)\
             <br>Your whole party died in room 32, turn 14 and they lost all items they found!\
             <br>",
        );
        let log = parse_dungeon_log(&html).expect("should parse wipe log");
        assert!(log.summary.wipe_line.is_some());
        assert!(log.summary.wipe_line.unwrap().contains("whole party died"));
    }

    #[test]
    fn test_parse_log_with_deaths_and_party_wipe_narrative() {
        // Simulates: individual deaths followed by an "All pets died" narrative
        // line that does NOT contain "was killed by".
        let html = make_log(
            "<br>Pets used: \
             <br>Alpha, Rogue: Growth 100, Level 5, Class Level 2\
             <br>Beta, Mage: Growth 200, Level 10, Class Level 4\
             <br>\
             <br>Dungeon result for Scrapyard 2 (8/1/0/0), 32 rooms\
             <br>\
             <br>Alpha gained a total of 1,000 experience.\
             <br>Beta gained a total of 1,000 experience.\
             <br>\
             <br>Alpha was killed by MURDER in room 16, turn 1.\
             <br>Beta was killed by MURDER in room 16, turn 4.\
             <br>All pets died in room 32, turn 14 from a combined, ultimate attack.\
             <br>\
             <br>Alpha:\
             <br>Depth 1: 100 damage done, 50 damage taken.\
             <br>\
             <br>Beta:\
             <br>Depth 1: 200 damage done, 30 damage taken.\
             <br>",
        );
        let log = parse_dungeon_log(&html).expect("should parse death + wipe log");
        assert_eq!(log.deaths.len(), 2);
        assert_eq!(log.deaths[0].pet_name, "Alpha");
        assert_eq!(log.deaths[1].pet_name, "Beta");
        // Depth stats should still be parsed correctly after the wipe narrative line.
        assert_eq!(log.depth_stats.len(), 2);
    }

    #[test]
    fn test_parse_log_with_leech_and_free_exp() {
        let html = make_log(
            "<br>Pets used: \
             <br>Alpha, Rogue: Growth 100, Level 5, Class Level 2\
             <br>\
             <br>Dungeon result for Forest 1 (3/0/0/0), 3 rooms\
             <br>\
             <br>Alpha gained a total of 500 experience.\
             <br>\
             <br>Items Used:\
             <br>4 x Torch (29 + 32 left)\
             <br>\
             <br>Your 2 pets with an exp leech weapon outside dungeons received a total of 450 experience additionally.\
             <br>Your free exp pool gained 1,629 (921 from patreons) experience.\
             <br>",
        );
        let log = parse_dungeon_log(&html).expect("should parse leech/free exp log");
        assert!(log.summary.leech_line.is_some());
        assert!(log.summary.free_exp_line.is_some());
        assert_eq!(log.items_used.len(), 1);
        assert_eq!(log.items_used[0].name, "Torch");
    }

    // -------------------------------------------------------------------------
    // compute_totals
    // -------------------------------------------------------------------------

    #[test]
    fn test_compute_totals() {
        let log = DungeonLog {
            pets: vec![],
            dungeon_name: String::new(),
            dungeon_level: String::new(),
            rating: String::new(),
            room_count: 0,
            summary: SummaryInfo {
                xp_gained: vec![],
                longest_turn: None,
                leech_line: None,
                free_exp_line: None,
                wipe_line: None,
            },
            items_used: vec![],
            items_found: vec![],
            items_from_events: vec![],
            events: vec![],
            deaths: vec![],
            depth_stats: vec![],
            room_stats: vec![PetRoomStats {
                pet_name: "Alpha".into(),
                rooms: vec![
                    RoomStat {
                        room: 1,
                        damage_done: "1,000".into(),
                        damage_taken: "200".into(),
                        healed: Some("50".into()),
                    },
                    RoomStat {
                        room: 2,
                        damage_done: "500".into(),
                        damage_taken: "100".into(),
                        healed: None,
                    },
                ],
            }],
            rooms: vec![],
        };
        let totals = compute_totals(&log);
        let (done, taken, healed) = totals.get("Alpha").expect("should have Alpha");
        assert_eq!(*done, 1500);
        assert_eq!(*taken, 300);
        assert_eq!(*healed, 50);
    }

    // -------------------------------------------------------------------------
    // Real file parsing (integration-style, only runs when files exist)
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_real_scrapyard_log() {
        let path = "data/dungeon_logs/2026-03-29_21-33_Scrapyard.html";
        let html = match std::fs::read_to_string(path) {
            Ok(h) => h,
            Err(_) => return, // skip if file not present
        };
        let log = parse_dungeon_log(&html).expect("should parse real Scrapyard log");
        assert_eq!(log.dungeon_name, "Scrapyard");
        assert_eq!(log.dungeon_level, "2");
        assert_eq!(log.room_count, 15);
        assert_eq!(log.pets.len(), 6);
        assert_eq!(log.deaths.len(), 1);
        assert_eq!(log.deaths[0].pet_name, "Dragon");
        assert_eq!(log.events.len(), 2);
        assert!(!log.rooms.is_empty());
        assert!(!log.depth_stats.is_empty());
        assert!(!log.room_stats.is_empty());
    }

    #[test]
    fn test_parse_real_death_log() {
        let path = "data/dungeon_logs/death_2026-03-30_09-36_Scrapyard.html";
        let html = match std::fs::read_to_string(path) {
            Ok(h) => h,
            Err(_) => return,
        };
        let log = parse_dungeon_log(&html).expect("should parse real death log");
        assert_eq!(log.dungeon_name, "Scrapyard");
        assert_eq!(log.pets.len(), 6);
        assert!(log.summary.wipe_line.is_some());
        assert!(log.deaths.len() >= 2);
        // Leech/free-exp should be captured even when they appear after Items sections
        assert!(log.summary.leech_line.is_some());
        assert!(log.summary.free_exp_line.is_some());
        assert!(!log.depth_stats.is_empty());
        assert!(!log.room_stats.is_empty());
    }

    #[test]
    fn test_parse_real_forest_log() {
        let path = "data/dungeon_logs/2026-03-29_21-33_Forest.html";
        let html = match std::fs::read_to_string(path) {
            Ok(h) => h,
            Err(_) => return,
        };
        let log = parse_dungeon_log(&html).expect("should parse real Forest log");
        assert_eq!(log.dungeon_name, "Forest");
        assert!(!log.rooms.is_empty());
    }

    #[test]
    fn test_parse_real_water_temple_log() {
        let path = "data/dungeon_logs/2026-03-29_21-33_WaterTemple.html";
        let html = match std::fs::read_to_string(path) {
            Ok(h) => h,
            Err(_) => return,
        };
        let log = parse_dungeon_log(&html).expect("should parse real WaterTemple log");
        // WaterTemple is two words — parser should handle it
        assert!(
            log.dungeon_name.contains("Water") || log.dungeon_name.contains("Temple"),
            "Expected 'Water Temple' but got '{}'",
            log.dungeon_name
        );
    }
}
