# enum-sync

After an ITRTG game update, surface game-enum ids (pets, adventure classes,
equipment, materials, …) that the `save-parser` Rust tables don't yet know
about.

The game ships an obfuscated `Assembly-CSharp.dll`. Enum **type** names are
randomized and rotate every build, but enum **member** names and their integer
values survive obfuscation (the game serializes them). So this tool parses every
enum out of a decompiled dump and identifies each one **by content** — matching
the dump enum whose `{value → member}` pairs best overlap a Rust lookup table —
never by the (useless) type name.

It reports; it never edits Rust. You review the delta and paste the snippet.

## Inputs are gitignored, this crate is not

The DLL and the decompiled `.cs` dump are copyrighted game artifacts and stay
out of git (`.gitignore` covers `_cs_decomp*/`). The crate — parser, registry,
tests — is committed like any other workspace member.

## Usage

```sh
# Decompile the DLL once (or let --dll do it for you):
ilspycmd "F:/SteamLibrary/.../Managed/Assembly-CSharp.dll" -o _cs_decomp_new

# What did this update change? (highest signal — zero curation noise)
cargo run -p enum-sync -- --diff _cs_decomp _cs_decomp_new

# Full audit: which game ids are missing from the Rust tables right now?
cargo run -p enum-sync -- --dump _cs_decomp_new

# One enum, full detail (lists even a deliberately-partial table):
cargo run -p enum-sync -- --dump _cs_decomp_new --only equipment_type

# Decompile + audit in one step (temp dir, auto-cleaned):
cargo run -p enum-sync -- --dll ".../Assembly-CSharp.dll"

#   --strict   exit non-zero if any id is missing from a Complete table (CI gate)
```

### Which mode when

- **`--diff <old> <new>`** is the post-update workflow. It compares two dumps and
  reports exactly what the patch added/removed per enum. Keep the previous dump
  around (e.g. `_cs_decomp`) and decompile the new one beside it
  (`_cs_decomp_new`); the diff is then noise-free.
- **`--dump <dir>`** audits one dump against the live Rust tables. Tables flagged
  `Partial` in the registry (skills, craftable gear, materials, decorative
  village pieces) are deliberately curated subsets, so their gaps are summarized
  rather than listed. `None`-style sentinel members are ignored.

## Extending

Add a line to `REGISTRY` in [`src/registry.rs`](src/registry.rs): the CLI key,
the `save_parser::items::*_name` lookup, an id-scan bound, and whether the table
aims to be `Complete` or `Partial`. No type names, no fingerprints to maintain —
the Rust table is the fingerprint.

## Validation

`tests/dumps.rs` runs against the real dumps when present (skipped otherwise):
the 2026-06 update must diff to exactly `pets: +Boar=152`, with the Monk class
already present beforehand. `src/parse.rs` and `src/registry.rs` carry unit tests
that always run.
