//! save-dump: decode an ITRTG save file and print either the full expanded
//! tree (for format exploration) or a typed summary.
//!
//! Usage:
//!   save-dump <save-file> [--tree]
//!   save-dump <save-file> --reencode <out-file>
//!
//! `--reencode` decodes the save and writes a faithfully re-serialized copy
//! (parse → lossless `raw` tree → serialize → container) with **no** redaction
//! — it exists to confirm the game accepts a round-tripped save. `--redact`
//! does the same but scrubs the root identity fields (Steam id, account names,
//! god name) first, refusing to write if any redacted value still appears.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let tree_mode = args.iter().any(|a| a == "--tree");
    let arg_after = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1).cloned())
    };
    let reencode_out = arg_after("--reencode");
    let redact_out = arg_after("--redact");
    let path = match args.iter().find(|a| !a.starts_with("--")) {
        Some(p) => p.clone(),
        None => {
            eprintln!("Usage: save-dump <save-file> [--tree]");
            eprintln!("       save-dump <save-file> --reencode <out-file>");
            eprintln!("       save-dump <save-file> --redact <out-file>");
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

    if let Some(out) = reencode_out {
        let decoded = match save_parser::container::decode_container(&raw) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Failed to decode save: {e:#}");
                return ExitCode::FAILURE;
            }
        };
        let reserialized = save_parser::raw::parse(&decoded.plaintext).serialize();
        if reserialized != decoded.plaintext {
            // The round-trip tests guard this, but never write a save we know
            // diverges from the original tree.
            eprintln!(
                "Refusing to write: re-serialized tree differs from the original \
                 ({} vs {} bytes). This is a bug in the serializer.",
                reserialized.len(),
                decoded.plaintext.len()
            );
            return ExitCode::FAILURE;
        }
        let encoded = save_parser::container::encode_container(&reserialized, &decoded.prefix);
        if let Err(e) = std::fs::write(&out, &encoded) {
            eprintln!("Failed to write {out}: {e}");
            return ExitCode::FAILURE;
        }
        eprintln!(
            "Wrote re-encoded save to {out} ({} chars). Tree round-trips exactly; \
             gzip layer differs from the original but decodes to identical plaintext.",
            encoded.len()
        );
        return ExitCode::SUCCESS;
    }

    if let Some(out) = redact_out {
        let decoded = match save_parser::container::decode_container(&raw) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Failed to decode save: {e:#}");
                return ExitCode::FAILURE;
            }
        };
        let mut root = save_parser::raw::parse(&decoded.plaintext);
        let changes = save_parser::redact::redact_identity(&mut root);
        let reserialized = root.serialize();

        // Safety net: none of the values we just removed may survive anywhere
        // in the tree (e.g. mirrored inside a nested block we did not redact).
        let needles: Vec<&str> = changes.iter().map(|c| c.old.as_str()).collect();
        let residual = save_parser::redact::residual_hits(&reserialized, &needles);
        if !residual.is_empty() {
            eprintln!(
                "Refusing to write: redacted value(s) still present in the save: {residual:?}. \
                 An identity value is mirrored somewhere redaction did not reach."
            );
            return ExitCode::FAILURE;
        }
        if changes.is_empty() {
            eprintln!("Warning: no identity fields found to redact (already clean?).");
        }

        let encoded = save_parser::container::encode_container(&reserialized, &decoded.prefix);
        if let Err(e) = std::fs::write(&out, &encoded) {
            eprintln!("Failed to write {out}: {e}");
            return ExitCode::FAILURE;
        }
        eprintln!("Wrote redacted save to {out}. Redacted {} field(s):", changes.len());
        for c in &changes {
            eprintln!("  {:<3} {:?} -> {:?}", c.key, c.old, c.new);
        }
        return ExitCode::SUCCESS;
    }

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
