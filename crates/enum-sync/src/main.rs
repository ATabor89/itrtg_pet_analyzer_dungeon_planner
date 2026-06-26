//! enum-sync — after an ITRTG update, surface game-enum ids that the Rust
//! tables in `save-parser` don't yet know about.
//!
//! The game's `Assembly-CSharp.dll` is obfuscated, but enum *members* and their
//! values survive (only the type name rotates per build). This tool extracts
//! every enum from a decompiled dump and, for each enum we mirror in Rust,
//! reports the delta — ids present in the game but missing from our tables —
//! plus a paste-ready snippet. It never edits Rust; you review and apply.
//!
//! Inputs are game-derived (the DLL, the decompile dump) and stay gitignored;
//! this crate is committed. See `reference/save_file_deserialization/` and
//! CLAUDE.md for the redaction/privacy rules around game artifacts.
//!
//! ## Usage
//!
//! ```text
//! enum-sync --dump <dir>                 # diff a decompile dump vs the Rust tables
//! enum-sync --dll  <Assembly-CSharp.dll> # decompile to a temp dir first, then as --dump
//! enum-sync --diff <old_dir> <new_dir>   # diff enums between two dumps (what an update changed)
//!
//!   --only <key>   restrict to one registered enum (e.g. pets)
//!   --strict       exit non-zero if any actionable (missing-id) delta is found
//! ```

use anyhow::{Context, Result, bail};
use enum_sync::parse;
use enum_sync::registry;
use enum_sync::registry::{KnownEnum, REGISTRY};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

enum Mode {
    AgainstRust { dump: PathBuf },
    /// Decompile a DLL to a temp dir, then diff against the Rust tables.
    Dll { dll: PathBuf },
    DiffDumps { old: PathBuf, new: PathBuf },
}

struct Args {
    mode: Mode,
    only: Option<String>,
    strict: bool,
}

fn run() -> Result<ExitCode> {
    let args = parse_args()?;

    if let Some(only) = &args.only
        && !REGISTRY.iter().any(|k| k.key == *only)
    {
        bail!(
            "unknown --only key '{only}'. known: {}",
            REGISTRY.iter().map(|k| k.key).collect::<Vec<_>>().join(", ")
        );
    }

    let actionable = match &args.mode {
        Mode::AgainstRust { dump } => report_against_rust(dump, args.only.as_deref())?,
        Mode::Dll { dll } => {
            let tmp = decompile_to_temp(dll)?;
            report_against_rust(tmp.path(), args.only.as_deref())?
            // tmp drops here → temp decompile dir is removed.
        }
        Mode::DiffDumps { old, new } => report_diff_dumps(old, new, args.only.as_deref())?,
    };

    if args.strict && actionable > 0 {
        Ok(ExitCode::FAILURE)
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn parse_args() -> Result<Args> {
    let mut only = None;
    let mut strict = false;
    let mut mode: Option<Mode> = None;

    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--dump" => {
                let d = it.next().context("--dump needs a directory")?;
                mode = Some(Mode::AgainstRust { dump: d.into() });
            }
            "--dll" => {
                let d = it.next().context("--dll needs a path")?;
                mode = Some(Mode::Dll { dll: d.into() });
            }
            "--diff" => {
                let old = it.next().context("--diff needs <old> <new>")?;
                let new = it.next().context("--diff needs <old> <new>")?;
                mode = Some(Mode::DiffDumps { old: old.into(), new: new.into() });
            }
            "--only" => only = Some(it.next().context("--only needs a key")?),
            "--strict" => strict = true,
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            other => bail!("unexpected argument: {other} (try --help)"),
        }
    }

    let mode = mode.context("need one of --dump <dir>, --dll <path>, or --diff <old> <new>")?;
    Ok(Args { mode, only, strict })
}

fn print_usage() {
    eprintln!(
        "enum-sync — diff game enums against the save-parser Rust tables\n\n\
         usage:\n  \
         enum-sync --dump <dir>                 diff a decompile dump vs the Rust tables\n  \
         enum-sync --dll  <Assembly-CSharp.dll> decompile (ilspycmd) to a temp dir, then diff\n  \
         enum-sync --diff <old_dir> <new_dir>   diff enums between two dumps\n\n\
         options:\n  \
         --only <key>   restrict to one enum ({})\n  \
         --strict       exit non-zero if any id is missing from the Rust tables",
        REGISTRY.iter().map(|k| k.key).collect::<Vec<_>>().join(", ")
    );
}

/// Diff a dump against the live Rust tables. Returns the count of actionable
/// (missing-id) deltas across all reported enums.
fn report_against_rust(dump: &Path, only: Option<&str>) -> Result<usize> {
    let enums = parse::parse_dir(dump)
        .with_context(|| format!("reading decompile dump at {}", dump.display()))?;
    println!(
        "Parsed {} enums from {}\n",
        enums.len(),
        dump.display()
    );

    // An explicit `--only <key>` means "show me everything for this one", so
    // list a partial table's full delta instead of summarizing it.
    let force_full = only.is_some();
    let mut actionable = 0;
    for known in REGISTRY.iter().filter(|k| only.is_none_or(|o| o == k.key)) {
        actionable += report_one_against_rust(known, &enums, force_full);
    }

    if actionable == 0 {
        println!("All registered enums are in sync with the Rust tables. ✓");
    } else {
        println!(
            "\n{actionable} missing id(s) — paste the snippet(s) above into the matching \
             table in crates/save-parser/src/items.rs."
        );
    }
    Ok(actionable)
}

fn report_one_against_rust(
    known: &KnownEnum,
    enums: &[parse::ParsedEnum],
    force_full: bool,
) -> usize {
    let Some((matched, score)) = registry::match_enum(known, enums) else {
        println!("== {} ==  (no matching enum located in dump — skipped)", known.key);
        return 0;
    };

    let fingerprint = registry::rust_fingerprint(known);
    let by_value = matched.by_value();

    // Actionable: enum members with no Rust entry at that value, ignoring
    // `None`-style sentinels the tables omit by design.
    let mut missing: Vec<(i64, &str)> = by_value
        .iter()
        .filter(|(v, n)| !fingerprint.contains_key(v) && !registry::is_sentinel(n))
        .map(|(v, n)| (*v, *n))
        .collect();
    missing.sort();

    // Informational: same value, different normalized spelling (usually a
    // deliberate display rename in the Rust table, occasionally a real change).
    let mut mismatched: Vec<(i64, &str, String)> = by_value
        .iter()
        .filter_map(|(v, n)| {
            fingerprint
                .get(v)
                .filter(|rust| *rust != &registry::normalize(n))
                .map(|rust| (*v, *n, rust.clone()))
        })
        .collect();
    mismatched.sort();

    let partial = known.coverage == registry::Coverage::Partial && !force_full;
    println!(
        "== {} ==  (obfuscated type `{}`, {} members, score {}{})",
        known.key,
        matched.type_name,
        matched.members.len(),
        score,
        if partial { ", partial table" } else { "" }
    );

    if missing.is_empty() {
        println!("  in sync — no missing ids");
    } else if partial {
        // Curated subset: summarize rather than nag with a full snippet.
        println!(
            "  partial table — {} game id(s) not mirrored (by design); \
             run `--only {} --dump` to list, or `--diff` to see what an update added",
            missing.len(),
            known.key
        );
    } else {
        println!("  MISSING from Rust ({}):", missing.len());
        for (v, name) in &missing {
            println!("    {v} = {name}");
        }
        println!("  snippet:");
        for (v, name) in &missing {
            println!("        {v} => \"{name}\",");
        }
    }

    if !mismatched.is_empty() {
        println!("  name differs (likely intentional display rename):");
        for (v, dump_name, rust_name) in &mismatched {
            println!("    {v}: dump=`{dump_name}` rust(normalized)=`{rust_name}`");
        }
    }
    println!();

    // Partial tables never contribute to the actionable count.
    if partial { 0 } else { missing.len() }
}

/// Diff the same logical enum between two dumps (old → new). This is the
/// "what did the update add?" view and the tool's own validation oracle.
fn report_diff_dumps(old: &Path, new: &Path, only: Option<&str>) -> Result<usize> {
    let old_enums =
        parse::parse_dir(old).with_context(|| format!("reading {}", old.display()))?;
    let new_enums =
        parse::parse_dir(new).with_context(|| format!("reading {}", new.display()))?;
    println!(
        "Comparing {} ({} enums) → {} ({} enums)\n",
        old.display(),
        old_enums.len(),
        new.display(),
        new_enums.len()
    );

    let mut changed = 0;
    for known in REGISTRY.iter().filter(|k| only.is_none_or(|o| o == k.key)) {
        let old_match = registry::match_enum(known, &old_enums);
        let new_match = registry::match_enum(known, &new_enums);
        let (Some((old_e, _)), Some((new_e, _))) = (old_match, new_match) else {
            continue; // enum not present in one or both dumps — nothing to say
        };

        let old_by = old_e.by_value();
        let new_by = new_e.by_value();

        let mut added: Vec<(i64, &str)> =
            new_by.iter().filter(|(v, _)| !old_by.contains_key(v)).map(|(v, n)| (*v, *n)).collect();
        let mut removed: Vec<(i64, &str)> =
            old_by.iter().filter(|(v, _)| !new_by.contains_key(v)).map(|(v, n)| (*v, *n)).collect();
        added.sort();
        removed.sort();

        if added.is_empty() && removed.is_empty() {
            continue;
        }
        changed += added.len() + removed.len();

        println!(
            "== {} ==  old `{}` ({}) → new `{}` ({})",
            known.key,
            old_e.type_name,
            old_e.members.len(),
            new_e.type_name,
            new_e.members.len()
        );
        for (v, n) in &added {
            println!("  + {v} = {n}");
        }
        for (v, n) in &removed {
            println!("  - {v} = {n}");
        }
        println!();
    }

    if changed == 0 {
        println!("No registered enum changed between the two dumps.");
    }
    Ok(changed)
}

/// A temp directory removed when dropped (so a decompile dump never lingers).
struct TempDir(PathBuf);

impl TempDir {
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Decompile `dll` into a fresh temp directory with `ilspycmd`. Best-effort:
/// requires `ilspycmd` on PATH. The directory is cleaned up automatically.
fn decompile_to_temp(dll: &Path) -> Result<TempDir> {
    if !dll.exists() {
        bail!("DLL not found: {}", dll.display());
    }
    // Unique-enough without rand/Date: the process id.
    let dir = std::env::temp_dir().join(format!("enum-sync-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let guard = TempDir(dir.clone());

    println!("Decompiling {} → {} (ilspycmd)…", dll.display(), dir.display());
    let status = std::process::Command::new("ilspycmd")
        .arg(dll)
        .arg("-o")
        .arg(&dir)
        .status()
        .context("running ilspycmd (is it installed and on PATH?)")?;
    if !status.success() {
        bail!("ilspycmd exited with {status}");
    }
    Ok(guard)
}
