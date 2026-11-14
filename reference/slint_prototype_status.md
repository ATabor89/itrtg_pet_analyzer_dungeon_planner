# Slint port: status

`crates/slint-ui` is a second front end for the same planner. Every view of the
egui app (`crates/gui`) has a Slint counterpart; see `slint_feature_parity.md`
for the checklist. egui stays the default until the user chooses to adopt Slint:
the Pages deployment serves the egui app at the site root and the Slint build
beside it under `/slint/`.

## Implemented

`crates/slint-ui` builds the native `itrtg-slint` executable and the same UI for
WASM. Slint and its build dependency are pinned to 1.17.1. Both platforms use
Winit + FemtoVG. Native accessibility is enabled. Desktop preferred content size
is **1920 x 1080**, with a 1000 x 660 minimum; browser sizing follows its viewport.

- Bundled wiki reference, Pet Stats paste/file import, and historical example.
- Main Stats import updates supported account settings without replacing pets.
- Full-save import replaces the roster and fills supported account settings and
  exact Moai levels through the existing save-parser converters. Combat stats and
  live actions are labeled unavailable because the save cannot supply them.
  Native decoding runs on a worker; only the planner projection is persisted.
- Name, export-alias and ability search; ownership, element, evolution, unlock,
  recommended/current class, token improvement and campaign filters.
- Twelve sort modes, ascending/descending order, and time-estimate tiebreaks.
- Base/effective growth, PGC and Magic Egg estimates, evolution requirements,
  growth targets and time estimates, Moai levels, elemental-form progress.
- Equipment quality, special abilities, Aether estimates and computed campaign
  bonuses with their source breakdown and editable account inputs.
- Roster growth/top-50-DL summary and stable canonical-name selection.
- Isolated native/browser session persistence, wiki links and About attribution.

Load example only prefills the import dialog. Import explicitly replaces the
roster; Cancel keeps it. Imports parse completely before replacing state, and
empty, incomplete or duplicate-name pet imports are rejected atomically.
An export-missing pet remains unknown rather than automatically locked.
Readiness means growth readiness only; other evolution conditions appear separately.

The analyzer has passed a fresh-eyes source parity audit. Dungeon Logs now has
overview/party cards, rewards/events/depth stats, room charts, supporter party
damage/healing comparisons and room/all-room combat views. Both frontends use
the same extracted log parser and party ordering. Drag/drop stages supported
files for explicit import on native and WASM. See `slint_feature_parity.md` for
the detailed checklist.

The Growth Chamber and the Save Editor are ported too (below). Browser layout
and accessibility coverage remain incomplete; see validation.

Dungeon planning now includes selection/depth previews, constraints and slot pins,
event overrides, equipment standards, shared multi-dungeon solving, formation
cards, coverage/hazards, difficulty and shopping lists. Constraint YAML and mapped
Dungeon Teams imports validate before applying. Clipboard and game-format exports
are exposed. Settings persist separately; changed inputs mark plans stale until
Solve. Native solving uses a worker. The browser yields once before computing.

## Growth Chamber, Save Editor and the app shell

- **Import.** "Import clipboard" reads the clipboard and imports it in one
  click; the kind (Pet Stats, Main Stats, full save, dungeon log, wiki page
  source) is detected from the text. "Import data…" keeps the paste
  dialog, file picker and an explicit kind. Dropped files import directly. A
  full save opens in the Save Editor.
- **Growth Chamber.** State, recommendation, simulation and the run report are
  shared with egui through `itrtg_planner::chamber`; the Slint view only lays
  them out.
- **Save Editor.** All 25 sections, the pending-changes panel, save / redacted
  save, and "Populate planner". Edits go through `save_parser::EditSession`.
  egui's modal dialogs are inline rows here.
- **Wiki refresh.** Native fetches the wiki. The browser cannot (no CORS
  headers), so it accepts the wiki page source by paste or drop. The pet table
  has no evolution requirements (only the extractor's crawl does), so a refresh
  keeps the ones already known, and `data/wiki_pets.yaml` is rewritten only
  when a validated table actually differs.
- **First run.** With no Slint session yet, the egui app's saved settings are
  read once (`data/app_state.yaml` natively, the `app_state` localStorage key in
  the browser) and seed the session. The egui state is never written.

### The form renderer

The chamber and the editor are built from data-driven rows instead of
per-screen Slint code. `src/form.rs` has the builders (`Form`, `Row`, `Cell`)
and `ui/form.slint` renders them; `FormModel` diffs rows in place so controls
keep focus while the model re-renders. Rust builds the rows from the current
state after every action and registers one closure per interactive cell.
Non-live inputs report a draft while typing, which is committed before any
other action, so "type a value, click Apply" works without leaving the field.
A draft belongs to the render it was typed against: renders that do not come
from the form's own actions (tab or section switch, an import finishing) flush
it first, and installing a new render's actions drops whatever is left. Buttons
that act on "row N of a list" must capture the row's identity, not its
position, because the draft commit that runs first can change the list.

The bundled Inter font is the only font in the browser build. Glyphs outside it
(for example U+2B62, U+25B8, U+22EF) render as boxes there; stay within Inter.

## Shared architecture

- `crates/planner/src/analyzer.rs` holds analyzer state, filters, sorting and pure
  display helpers extracted from egui. Existing serialized analyzer fields are
  preserved. Search additionally recognizes export aliases.
- egui retains its rendering and AppState persistence adapter, using the same
  analyzer types and behavior as Slint.
- Slint `src/app.rs` owns ordinary Rust state and commands. `controls.rs` exposes
  keyed settings; `details.rs` projects domain calculations into display sections.
- `src/bindings.rs` translates UI callbacks and updates a retained `VecModel`.
  Selection-only changes leave rows intact; PGC changes refresh effective growth
  and reconcile sorting/selection. There is one authoritative Rust session.
- `src/platform.rs` isolates persistence, background native file picking,
  asynchronous browser file picking and external links. UI updates stay on the
  UI thread; expensive future calculations should preserve this separation.
- `ui/app.slint` contains declarative layouts and reusable visual components.
  Game formulas stay in the existing domain crates.

## Persistence and fixtures

A save loaded in the Save Editor (the decoded tree, with its account
identifiers) lives in memory only and is never written to the session. "Save
As" writes the full save; "Save redacted copy" strips the identifiers. The
planner's own full-save import keeps only the planner projection.

Native settings are `slint-prototype-state.yaml` beside the executable, written
via temporary file and rename. Browser storage uses `itrtg_slint_prototype_v1`.
These are separate from egui state, which is only read (first-run seeding).
Older version-1 sessions remain readable with defaults for new settings. Unknown/malformed settings disable
persistence for that run to preserve the original file.

`fixtures/example_pet_stats.txt` copies the previously committed June 2026 Pet
Stats export, not a raw account save. Imports stay local and are not submitted
over the network. The prototype does not scrape live wiki pages.

## Run

```
cargo run -p itrtg-slint
cargo test --workspace
cargo clippy --workspace --all-targets
```

Browser build, from `crates/slint-ui` (Trunk and the wasm32-unknown-unknown
target are required):

```
trunk serve            # http://127.0.0.1:8087
trunk build --release
```

On Windows, Trunk rejects the host's `NO_COLOR=1`; set `$env:NO_COLOR = 'true'`.

## Validation (2026-11-14)

- Workspace tests: **645 passed, one ignored**, 75 of them in the Slint crate
  (form renderer, chamber rows, every Save Editor section against the redacted
  reference save, import routing, first-run seeding).
- Clippy: no warnings in the Slint crate; the remaining workspace warnings
  predate this work.
- A no-context review of the whole branch found no blockers; its findings
  (draft lifetime, undo by position, the wiki refresh write, CI missing
  fontconfig) are fixed with regression tests.
- Native and WASM builds succeed.
- Headless Chromium against the WASM build: the flows listed under "Verified in
  a browser" in `slint_feature_parity.md`. Both downloaded saves were re-read
  with `save-dump`; the redacted one passes `--check`.
- The release WASM build loads and imports a save in Chromium. It is about
  23 MB before `wasm-opt` (7.7 MB gzipped); load time on Pages is unmeasured.
- The native build starts and renders under a virtual display (Xvfb, software
  GL). Not exercised on a real desktop this round: file dialogs, the native
  clipboard and window behavior.

## Validation (2026-09-15)

- Dungeon planner source review passed; preview gems and extracted documentation
  were corrected. Native isolated-session checks covered depth selection, solving,
  equipment/shopping cards, event stale-state handling, team mapping/import,
  pinned slot after restart, and persistence. User session hash stayed unchanged.
- Five dungeon tests cover solver/equipment result equivalence, no reused pets,
  exact pins, atomic imports, scoped replacement and old/new session handling.
- Workspace tests: **590 passed, one ignored, zero failures**, including twenty-seven
  Slint model tests. Added coverage includes old sessions, atomic partial Main
  Stats updates, advanced filters, campaign ranking, invalid numeric settings,
  target sorting, ability/alias search and detail projections. Full-save tests
  cover converter parity, atomic failure, preserved manual inputs, provenance
  persistence and absence of account identity fields in stored session data.
- Workspace/all-target clippy passed with only three existing warnings in
  save-parser/planner. No new warnings.
- Native debug and Trunk WASM development builds succeeded.
- The analyzer parity regression tests cover locked-pet estimates, row markers,
  reference abilities and Earth Eater limits. Log tests exercise all four HTML
  fixtures, supporter net/overlay calculations, all-room combat, unknown monster
  headers and file-drop routing.
- Native log file import, overview/wipe summary, supporter room statistics and
  all-room combat were checked visually. Chart bars were explicitly anchored at
  the left edge after a visual alignment issue was found.
- Fresh-eyes analyzer and logs reviews passed after their findings were fixed.
- Windows app opened at 1920 x 1080 content size and restored the user's current
  roster and PGC settings. Class filtering/reset, settings dialog, equipment and
  evolution details, scrolling, and base/effective growth were checked visually.
- Edge on a separate test origin imported the historical example and a Main
  Stats pet-stone value while retaining the roster; reload restored the session.
  Earlier native checks covered rejected pet imports and PGC persistence.
- Native full-save file selection completed against the committed redacted June 9
  fixture in a separate session: 158 records, 104 owned, 80 evolved. The user
  session was kept separate and restored afterward.
- The updated WASM build rendered in Edge. Full-save import completion in the
  browser remains unverified; decoding currently yields once then runs on the
  browser thread, so large saves may briefly pause the UI.
- Fresh-eyes review found no blocking or should-fix issues. Its search-trimming
  nit was fixed with test coverage. Existing egui tests pass after extraction.
  Full-save integration review also passed; its owned-level-zero Moai display
  nit was clarified in the settings hint.

Browser automation sometimes times out clicking Import while the text editor
has focus. Moving focus with Tab before clicking worked; the underlying focus
issue is not resolved. Automation's multiline typing also required explicit
Return keys. Browser Main Stats PGC interaction, real clipboard, native/browser drag-drop and browser file selection,
full keyboard navigation, narrow layouts and other browser engines remain
unverified. Wrapped detail text was corrected and visually checked.

Release bundle size/load time and browser accessibility have not been validated;
the browser currently exposes a canvas surface. Persistence serializes the roster
on each settings change; debounce before extending it to larger saved state.

## Next steps

- Try the Slint build in daily use (`/slint/` on Pages) and decide on adoption.
- If adopted: make it the Pages root, retire `crates/gui`, and move the shared
  session to a debounced save (the roster is serialized on each settings change).
- Narrow layouts, keyboard navigation and browser accessibility.
