//! save-dump: decode an ITRTG save file and print either the full expanded
//! tree (for format exploration) or a typed summary.
//!
//! Usage:
//!   save-dump <save-file> [--tree]

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let tree_mode = args.iter().any(|a| a == "--tree");
    let path = match args.iter().find(|a| !a.starts_with("--")) {
        Some(p) => p.clone(),
        None => {
            eprintln!("Usage: save-dump <save-file> [--tree]");
            return ExitCode::FAILURE;
        }
    };

    let raw = match std::fs::read_to_string(&path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to read {path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    if tree_mode {
        let plaintext = match save_parser::container::decode_to_plaintext(&raw) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to decode save: {e:#}");
                return ExitCode::FAILURE;
            }
        };
        print!("{}", save_parser::tree::parse(&plaintext).dump());
        return ExitCode::SUCCESS;
    }

    let save = match save_parser::parse_save(&raw) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to parse save: {e:#}");
            return ExitCode::FAILURE;
        }
    };

    println!(
        "Save for {} / god {} (unix {})",
        save.player_name.as_deref().unwrap_or("?"),
        save.god_name.as_deref().unwrap_or("?"),
        save.saved_at_unix.unwrap_or(0),
    );
    println!(
        "{} pets ({} unlocked), {} equipment items, {} material stacks, pet stones: {}",
        save.pets.len(),
        save.pets.iter().filter(|p| p.unlocked).count(),
        save.equipment.len(),
        save.materials.len(),
        save.pet_stones.unwrap_or(0),
    );
    for team in &save.dungeon_teams {
        let members: Vec<String> = team
            .pet_type_ids
            .iter()
            .map(|id| {
                save.pet_by_type_id(*id)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| format!("#{id}"))
            })
            .collect();
        println!(
            "Team in {} (dungeon {}, depth {}): {}",
            team.dungeon_name,
            team.dungeon_id,
            team.depth,
            members.join(", ")
        );
    }
    let unknown_mats: Vec<&save_parser::model::MaterialStack> = save
        .materials
        .iter()
        .filter(|m| m.name().is_none())
        .collect();
    println!("\nMaterials ({} unidentified ids):", unknown_mats.len());
    let mut mats: Vec<&save_parser::model::MaterialStack> = save.materials.iter().collect();
    mats.sort_by_key(|m| m.item_id);
    for mat in mats {
        match mat.name() {
            Some(name) => println!("  {:>4}  {:<24} x{}", mat.item_id, name, mat.count),
            None => println!("  {:>4}  ???                      x{}", mat.item_id, mat.count),
        }
    }

    println!("\nTop pets by growth:");
    let mut by_growth: Vec<&save_parser::model::SavePet> = save.pets.iter().collect();
    by_growth.sort_by(|a, b| b.growth.total_cmp(&a.growth));
    for pet in by_growth.iter().take(15) {
        println!(
            "  {:<20} growth {:>12.0}  dungeon lv {:>3}  {}",
            pet.name,
            pet.growth,
            pet.dungeon_level,
            pet.class.map(|c| format!("{c:?}")).unwrap_or_default(),
        );
    }
    ExitCode::SUCCESS
}
