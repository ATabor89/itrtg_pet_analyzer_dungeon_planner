//! save-edit: apply scalar field edits to an ITRTG save and write a new,
//! game-loadable save. A single-player debug/cheat aid — the game's author
//! permits edited saves as long as high-score submission is turned off.
//!
//! Usage:
//!   save-edit <in-save> <out-save> [--gp <n>] [--set <path> <value>]...
//!
//! Examples:
//!   save-edit MySave.txt edited_save.txt --gp 999999999
//!   save-edit MySave.txt edited_save.txt --set p.025 75   # knock Camp Exp Boost down to disambiguate
//!
//! `<path>` is a dotted raw-tree path (the same paths used in FINDINGS.md),
//! e.g. `p.j` (available GP), `p.025`, `S.b` (a TBS component level). The output
//! is written to a NEW file (never in place) and carries the save's REAL,
//! unredacted data — do not commit it (`.gitignore` covers `**/edited_*.txt`).

use std::process::ExitCode;

use save_parser::edit::{edit_save, named_target, ScalarEdit};

fn usage() {
    eprintln!("Usage: save-edit <in-save> <out-save> [--gp <n>] [--set <path> <value>]...");
    eprintln!("  <out-save>          must start with 'edited_' (kept gitignored; holds real data)");
    eprintln!("  --gp <n>            set available god power (p.j)");
    eprintln!("  --set <path> <val>  set any scalar by dotted path, e.g. --set p.025 75");
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Single pass: flags consume their own values; everything else is a
    // positional (so a flag value like `1234567` is never mistaken for one).
    let mut positionals: Vec<&str> = Vec::new();
    let mut edits: Vec<ScalarEdit> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--gp" => {
                let Some(val) = args.get(i + 1) else {
                    eprintln!("--gp needs a value");
                    return ExitCode::FAILURE;
                };
                let path = named_target("gp").expect("gp target");
                edits.push(ScalarEdit {
                    path: path.iter().map(|s| s.to_string()).collect(),
                    value: val.clone(),
                });
                i += 2;
            }
            "--set" => {
                let (Some(path), Some(val)) = (args.get(i + 1), args.get(i + 2)) else {
                    eprintln!("--set needs <path> <value>");
                    return ExitCode::FAILURE;
                };
                edits.push(ScalarEdit::parse(path, val));
                i += 3;
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

    if edits.is_empty() {
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

    let (encoded, applied) = match edit_save(&raw, &edits) {
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
