# CLAUDE.md

Guidance for AI coding agents working in this repo. Read this first each session.

## What this is

A desktop/WASM **dungeon-team planner** for the idle game ITRTG. It ingests the
player's pet export + scraped wiki data, then recommends party compositions,
equipment, and gems for each dungeon/depth. Rust workspace; the GUI is
egui/eframe (runs natively and as WASM via Trunk → GitHub Pages).

The user is comfortable with Rust and is not a UI specialist — prefer clear,
well-tested logic and concise UI code that follows existing patterns.

## Architecture map

- `crates/models` — shared data types + YAML schemas. Dungeon/equipment/event
  data (`dungeon.rs`), pet wiki/export types, `planner_config.rs`. No business
  logic beyond parsing/helpers.
- `crates/planner` — the brains:
  - `solver.rs` — party assignment (which pet fills which slot, across
    dungeons), look-around hints, elemental matchup, coverage checks.
  - `equipment.rs` — equipment/gem recommendation + tier handling.
  - `merge.rs` — merges wiki + export into `MergedPet`.
- `crates/gui` — egui app. `views/dungeon.rs` is the main planner view;
  `views/analyzer.rs`, `views/log_viewer.rs` are secondary. `state.rs` is the
  persisted `AppState`.
- `crates/wiki-extractor`, `crates/pet-importer` — data ingestion tools.

### Key data files (`data/`)

- `dungeon_recommendations.yaml` — per-dungeon, per-depth party/equipment/event
  data. **Treat the in-game recommendations here as ground truth.** Depths 1–4.
- `equipment_catalog.yaml` — all gear, with `tier`, `element`, `upgraded_from`
  chains.
- `planner_config.yaml` — class→equipment selection rules (no hardcoded gear
  logic in Rust; tweak here).
- `pet_special_info.yaml` — per-pet quirks (synergies, locked classes, etc.).
- `wiki_pets.yaml` — scraped wiki data (auto-refreshed weekly).
- `app_state.yaml` — per-user state; **untracked**, do not commit.
- `dungeon_logs/` — sample dungeon log HTML used as `log_parser` test fixtures
  and the file-dialog default directory.

`data/` holds only machine-consumed files. Human-reference material (wiki page
text, in-game help text, research notes, the old JS analyzer) lives in the
top-level `reference/` directory — none of it is read by code.

## Verify before committing

```
cargo test --workspace
cargo clippy --workspace --all-targets
```

- There are **3 pre-existing clippy warnings** in `itrtg-planner` lib-test code
  (too-many-arguments, needless-borrow, useless-vec). They are not from current
  work — ignore them, but make sure you add **no new** warnings.
- Do **not** run `cargo fmt` across a crate — this repo is not rustfmt-clean and
  a blanket format produces huge unrelated diffs. Match the surrounding style by
  hand.
- For solver/equipment changes, add unit tests in the same file. Reproduce
  reported bugs as a failing test first, then fix.

## Workflow conventions (the user relies on these)

1. **Never commit directly to `main`.** Branch first: `feat/<short-name>` (or
   `fix/...`). Make the change with tests, verify (above).
2. **`git status` / review the diff before staging.** Don't `git add -A`
   blindly — the user sometimes has unrelated edits in the working tree; stage
   only your files so they don't get bundled into your commit.
3. **Fresh-eyes review for substantive changes.** Before merging anything with
   real logic, spin up a *no-context* reviewer: the `general-purpose` Agent with
   a prompt that tells it to review `git diff main...HEAD` with fresh eyes and
   report findings by severity (BLOCKER / SHOULD-FIX / NIT) plus a one-line
   verdict. Address the findings, then merge. Skip the review only for trivial,
   low-risk changes (docs, a pure display tweak) — and say so in the PR.
4. **PR + merge.** Open a PR (`gh pr create`), then `gh pr merge <n> --merge`.
   After merging: `git checkout main && git pull --ff-only`, then delete the
   branch locally and on origin. It's just the user and an agent on this repo,
   so the PR exists mainly as the review checkpoint and changelog.
5. Confirm `cargo build --workspace` on `main` after merging.

## Commit conventions

- Imperative subject; a body explaining the *why* for non-trivial changes.
- End commit messages with:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`
- End PR bodies with the Claude Code attribution line.
- Don't bundle unrelated changes into one commit.
- Heads-up: git will warn `LF will be replaced by CRLF` on Windows — harmless.

## Planner design notes (high level)

- **Ground truth vs. hints:** a depth's explicit slot class/element is fixed.
  Only *wildcard* (`any`) fields get filled by heuristics.
- **Look-around:** wildcard slots are biased toward neighboring depths'
  recommendations (deeper first, then shallower) since a dungeon team is grown
  across depths. Purely a tiebreaker; never changes which pets are eligible.
- **Greedy tiebreak order** (pet for a slot): match quality → evo difficulty →
  synergy → look-around hint → readiness (bucketed DL/CL) → elemental matchup →
  growth. A post-assignment swap pass repairs greedy mis-placements for
  elemental matchup.
- **Elemental matchup** (wildcard-element slots only): offensive classes
  (Mage/Assassin/Rogue) prefer to counter the dungeon element; defensive
  (Defender/Supporter) prefer to match it; the dungeon's strong element against
  the pet is penalized.
- When in doubt about a heuristic's intent, check recent PR descriptions and
  commit history — design rationale is recorded there.
