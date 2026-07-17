# ITRTG Pet Planner

A native and WebAssembly desktop-style planner for **Idling to Rule the Gods**.
It imports pet and account exports, combines them with curated wiki/game data,
and helps plan dungeon teams, equipment, gems, pet growth, and campaign runs.
It also includes a lossless structured save viewer/editor for reverse-engineering
and carefully scoped local edits.

## What is included

- Dungeon party assignment across depths, with equipment and gem suggestions.
- Pet analysis, evolution readiness, and growth estimates.
- A cycle-by-cycle Growth Chamber simulator with per-source attribution.
- Dungeon-log parsing and summaries.
- A structured save editor backed by a lossless raw tree and undoable edits.
- Importers for pet exports, main-stat exports, wiki data, and dungeon teams.
- `enum-sync`, a maintenance tool that finds new game enum ids after updates.

The planner treats the curated files in `data/` as machine-readable ground truth.
Research notes, game captures, and validation write-ups live in `reference/`.

## Run locally

The workspace uses the current stable Rust toolchain.

```sh
cargo run -p itrtg-gui
```

For the browser build, install [Trunk](https://trunkrs.dev/) and the WASM target:

```sh
rustup target add wasm32-unknown-unknown
cd crates/gui
trunk serve
```

## Verify changes

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
```

There are three known clippy warnings in planner test code. New changes should
not add warnings. The repository is intentionally not globally rustfmt-clean,
so avoid blanket formatting passes.

## Workspace layout

- `crates/models` — shared types and YAML schemas.
- `crates/planner` — party solving, equipment, merging, and growth/campaign math.
- `crates/gui` — egui/eframe native and WASM application.
- `crates/save-parser` — lossless save decoding, typed projections, editing,
  redaction, and export conversion.
- `crates/pet-importer` and `crates/wiki-extractor` — ingestion tools.
- `crates/enum-sync` — post-update enum auditing against decompiled game data.
- `data` — runtime YAML and dungeon-log fixtures.
- `reference` — human-readable research and validation material.

## Save-file privacy

Raw ITRTG saves contain account identifiers and player names, even when that
information is hidden inside nested compression and base64 layers. Never commit
a raw save. Create a checked, redacted fixture with:

```sh
cargo run -p save-parser --bin save-dump -- <raw-save> --redact <out-file>
```

The committed save fixtures are redacted, tests verify that they remain so, and
the optional pre-commit hook in `.githooks/pre-commit` provides an additional
guard. See `AGENTS.md` for the complete contributor workflow and privacy rules.

## Project status

This is an actively developed reverse-engineering tool. Confirmed mechanics are
backed by tests and captured game data; uncertain formulas are called out in the
relevant `reference/` status documents rather than silently treated as exact.

This project is not affiliated with the developer of Idling to Rule the Gods.
