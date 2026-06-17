//! save-edit: apply scalar field edits to an ITRTG save and write a new,
//! game-loadable save. A single-player debug/cheat aid — the game's author
//! permits edited saves as long as high-score submission is turned off.
//!
//! Usage:
//!   save-edit <in-save> <out-save> [--gp <n>] [--set <path> <value>]...
//!
//! Examples:
//!   save-edit MySave.txt edited_save.txt --gp 999999999 --stones 999999999
//!   save-edit MySave.txt edited_save.txt --set p.025 75   # knock Camp Exp Boost down to disambiguate
//!   save-edit MySave.txt edited_save.txt --mul X.b.a=Salamander.E 10   # 10× Salamander's growth
//!   save-edit MySave.txt edited_save.txt --set X.Q.a=117.b 99999       # set material id 117's count
//!
//! `<path>` is a dotted raw-tree path (the same paths used in FINDINGS.md),
//! e.g. `p.j` (available GP), `p.025`, `S.b` (a TBS component level). A numeric
//! segment indexes a list — `X.Q.17.b` is material-inventory element 17's
//! count. The output
//! is written to a NEW file (never in place) and carries the save's REAL,
//! unredacted data — do not commit it (`.gitignore` covers `**/edited_*.txt`).

use std::process::ExitCode;

use save_parser::edit::{edit_save, named_target, EditOp, EquipGrant, MaterialGrant, ScalarEdit};

fn usage() {
    eprintln!("Usage: save-edit <in-save> <out-save> [--gp <n>] [--set <path> <value>] [--mul <path> <factor>]...");
    eprintln!("  <out-save>          must start with 'edited_' (kept gitignored; holds real data)");
    eprintln!("  --gp <n>            set available god power (p.j)");
    eprintln!("  --stones <n>        set pet stones (X.y)");
    eprintln!("  --set <path> <val>  set any scalar by dotted path, e.g. --set p.025 75");
    eprintln!("  --mul <path> <f>    multiply a numeric value, e.g. --mul X.b.a=Salamander.E 10");
    eprintln!("  --material <id> <n> set/add a material-inventory count by item id (X.Q)");
    eprintln!("  --equip <pet-idx> <slot> <type> <plus> <quality>");
    eprintln!("                      add an equipment instance and equip it (slot: e/f/g)");
    eprintln!("                      (list paths: index `X.Q.17.b` or selector `X.Q.a=117.b`)");
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Single pass: flags consume their own values; everything else is a
    // positional (so a flag value like `1234567` is never mistaken for one).
    let mut positionals: Vec<&str> = Vec::new();
    let mut edits: Vec<ScalarEdit> = Vec::new();
    let mut materials: Vec<MaterialGrant> = Vec::new();
    let mut equips: Vec<EquipGrant> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--gp" | "--stones" => {
                let name = &args[i][2..]; // strip "--"
                let Some(val) = args.get(i + 1) else {
                    eprintln!("--{name} needs a value");
                    return ExitCode::FAILURE;
                };
                let path = named_target(name).expect("named target");
                edits.push(ScalarEdit {
                    path: path.iter().map(|s| s.to_string()).collect(),
                    op: EditOp::Set(val.clone()),
                });
                i += 2;
            }
            "--set" => {
                let (Some(path), Some(val)) = (args.get(i + 1), args.get(i + 2)) else {
                    eprintln!("--set needs <path> <value>");
                    return ExitCode::FAILURE;
                };
                edits.push(ScalarEdit::set(path, val));
                i += 3;
            }
            "--mul" => {
                let (Some(path), Some(fac)) = (args.get(i + 1), args.get(i + 2)) else {
                    eprintln!("--mul needs <path> <factor>");
                    return ExitCode::FAILURE;
                };
                let Ok(factor) = fac.parse::<f64>() else {
                    eprintln!("--mul factor {fac:?} is not a number");
                    return ExitCode::FAILURE;
                };
                edits.push(ScalarEdit::mul(path, factor));
                i += 3;
            }
            "--material" => {
                let (Some(id), Some(count)) = (args.get(i + 1), args.get(i + 2)) else {
                    eprintln!("--material needs <item-id> <count>");
                    return ExitCode::FAILURE;
                };
                materials.push(MaterialGrant {
                    id: id.clone(),
                    count: count.clone(),
                });
                i += 3;
            }
            "--equip" => {
                let (Some(pet), Some(slot), Some(ty), Some(plus), Some(q)) = (
                    args.get(i + 1),
                    args.get(i + 2),
                    args.get(i + 3),
                    args.get(i + 4),
                    args.get(i + 5),
                ) else {
                    eprintln!("--equip needs <pet-index> <slot> <type> <plus> <quality>");
                    return ExitCode::FAILURE;
                };
                let (Ok(pet_index), Some(slot_ch), Ok(type_id), Ok(plus), Ok(quality)) = (
                    pet.parse::<u32>(),
                    slot.chars().next().filter(|c| "efg".contains(*c)),
                    ty.parse::<u32>(),
                    plus.parse::<u32>(),
                    q.parse::<u32>(),
                ) else {
                    eprintln!("--equip args invalid (slot must be e/f/g, others numeric)");
                    return ExitCode::FAILURE;
                };
                equips.push(EquipGrant { pet_index, slot: slot_ch, type_id, plus, quality });
                i += 6;
            }
            other if other.starts_with("--") => {
                eprintln!("unknown flag: {other}");
                usage();
                return ExitCode::FAILURE;
            }
            other => {
                positionals.push(other);
                i += 1;
            }
        }
    }

    let (in_path, out_path) = match positionals.as_slice() {
        [i, o] => (*i, *o),
        _ => {
            usage();
            return ExitCode::FAILURE;
        }
    };

    if edits.is_empty() && materials.is_empty() && equips.is_empty() {
        eprintln!("No edits requested (use --gp or --set).");
        usage();
        return ExitCode::FAILURE;
    }

    // Never edit in place — that would clobber the source save.
    let same = std::fs::canonicalize(in_path)
        .ok()
        .zip(std::fs::canonicalize(out_path).ok())
        .map(|(a, b)| a == b)
        .unwrap_or(in_path == out_path);
    if same {
        eprintln!("Refusing to write the output over the input save ({in_path}).");
        return ExitCode::FAILURE;
    }

    // The output carries REAL, unredacted identity data. Require an `edited_`
    // basename so it always matches the `**/edited_*.txt` gitignore rule and
    // can't be committed by accident in this public repo.
    let out_basename = std::path::Path::new(out_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if !out_basename.starts_with("edited_") {
        eprintln!(
            "Refusing to write {out_path}: the output filename must start with \
             'edited_' so it stays gitignored — it holds the save's real, \
             unredacted identity data. Try e.g. 'edited_save.txt'."
        );
        return ExitCode::FAILURE;
    }

    let raw = match std::fs::read_to_string(in_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to read {in_path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let (encoded, applied) = match edit_save(&raw, &edits, &materials, &equips) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Edit failed: {e:#}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = std::fs::write(out_path, &encoded) {
        eprintln!("Failed to write {out_path}: {e}");
        return ExitCode::FAILURE;
    }

    println!("Wrote edited save to {out_path}. Applied {} edit(s):", applied.len());
    for a in &applied {
        println!("  {:<10} {:?} -> {:?}", a.path, a.old, a.new);
    }
    println!(
        "\nThis is an UNREDACTED real save — do not commit it. Load it in-game with \
         high-score submission DISABLED."
    );
    ExitCode::SUCCESS
}
