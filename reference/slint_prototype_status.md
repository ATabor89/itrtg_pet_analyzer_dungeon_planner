# Slint prototype: first analyzer checkpoint

This is an experimental parallel frontend on `feat/slint-prototype`. Do not
merge or replace the existing egui application until the migration is complete
and the user chooses to adopt it. The existing Pages deployment still builds
`crates/gui` from `main`.

## Scope

`crates/slint-ui` builds the `itrtg-slint` native executable and the same UI for
WASM. Slint and its build dependency are pinned to 1.17.1. Winit + FemtoVG is
used on both platforms, with native accessibility enabled.

The first screen has:

- Bundled wiki reference and a clearly labeled historical example roster.
- Pet Stats import by paste or file, through the existing `pet-importer`.
- Name/export-alias search, ownership and element filters, numeric sorting.
- Stable selection by canonical name and a scrollable pet-detail panel.
- Base and effective growth, evolution growth readiness, and PGC settings.
- Separate prototype session persistence on native and WASM.
- An About dialog with the Slint attribution widget.

This is **not feature parity** with the existing analyzer. Dungeon planning,
Growth Chamber, logs, save editing, campaign calculations, additional analyzer
filters, equipment details, export, and full-save/main-stats import still belong
to subsequent milestones. Wiki notes are explicitly prose, not computed bonuses.

## Architecture to preserve

- `src/app.rs`: ordinary Rust application state and commands, with no Slint or
  OS dependencies. Calls `merge_pets`, existing growth methods, and the existing
  export parser. Does not reimplement game formulas.
- `src/bindings.rs`: creates Slint row/detail projections and translates UI
  callbacks into commands. Retains one `VecModel`; selection-only and PGC edits
  do not replace the row model. There is one authoritative Rust session.
- `src/platform.rs`: isolated persistence, native background file picking, WASM
  asynchronous picking, and browser window sizing. All UI updates stay on the
  UI thread. Further expensive calculations should follow this separation.
- `ui/app.slint`: declarative layout, reusable badges/stat cards, table, details,
  and dialogs. Keep formulas out of this file.

Selection uses pet names, never sorted row indexes. Filters reconcile selection
to a visible pet (or clear it for no matches). An export-missing pet is unknown,
not automatically locked. Missing values display as unknown rather than zero.

The importer parses completely before replacing the roster. Empty, incomplete,
and duplicate-name imports are rejected without losing current state. Other
field interpretation follows the existing importer, including its limitations.

PGC and Magic Egg calculations call the same models used by the existing app.
Readiness means **growth readiness only**; materials and other conditions are
displayed separately. PGC maximum is editable, matching the existing analyzer.

## Persistence and fixtures

Native settings: `slint-prototype-state.yaml` beside the prototype executable.
Writes use a temporary file and rename. Browser settings: localStorage key
`itrtg_slint_prototype_v1`. Neither path is the egui state store. Unknown or
malformed settings disable persistence for that run so the file is preserved.

`fixtures/example_pet_stats.txt` is a byte-identical copy of the already
committed `reference/save_file_deserialization/second_save/Pet Stats Export.txt`
(June 2026). It contains pet rows, not a raw account save. Runtime fixtures live
in the crate so the human-reference directory remains outside runtime inputs.
Imports contain no network submission, and the prototype does not scrape live
wiki pages. The imported roster is stored locally to allow restart/reload.

## Run and verify

From the prototype worktree root:

```powershell
cargo run -p itrtg-slint
cargo test --workspace
cargo clippy --workspace --all-targets
```

For a browser preview, from `crates/slint-ui`:

```powershell
$env:NO_COLOR = 'true' # Trunk rejects the host's NO_COLOR=1 value.
trunk serve --address 127.0.0.1 --port 8087
```

Open `http://127.0.0.1:8087`. For a static build use `trunk build --release`.
Trunk and the `wasm32-unknown-unknown` Rust target must be installed. Serving is
loopback-only. Do not run the production Pages deployment for this prototype.

The current local worktree is nested under the original checkout's ignored
`target/slint-prototype`. Run commands there, not in the original main checkout.
The original checkout's dirty files were archived and hashed before work began.

## Next checkpoint / effort handoff

Once this screen's native/browser behavior has been checked, Medium is a
reasonable setting for extending analyzer fields and reusable presentation
components using these established patterns. Use High again when tackling the
save tree/editor, shared persisted-state migration, or a difficult browser bug.

Before Growth Chamber changes, read `growth_chamber_status.md`. Preserve all
save-editor staging/validation rules when that migration begins. Do not copy
logic out of the domain crates to speed up UI porting.

Open validation work for later milestones: mobile/narrow layouts, large-save
editor performance, full keyboard/focus behavior, release bundle/load-time
measurements, other browser engines, and full feature parity. The first screen
has a 1000×660 minimum native layout; phone support is not yet designed.
