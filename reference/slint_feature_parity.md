# Slint feature parity checklist

This checklist compares behavior with the existing egui frontend. Every view of
the egui app now has a Slint counterpart. It is not an adoption approval: egui
stays the default until the user chooses to switch.

## App shell and import

| Feature | Slint status |
| --- | --- |
| Tabs: Analyzer, Dungeon planner, Dungeon logs, Growth chamber, Save editor | Implemented |
| One-click "Import clipboard" (no dialog) | Implemented on native and browser; the kind is auto-detected |
| Auto-detected import: Pet Stats, Main Stats, full save, dungeon log HTML, wiki page source | Implemented for clipboard, paste dialog, file pick and file drop (egui's clipboard button takes Pet Stats only) |
| Dropped files import directly | Implemented (egui behavior); a save opens in the Save editor |
| Wiki refresh | Native: fetches and re-parses the wiki. Browser: paste or drop the wiki page source (the wiki sends no CORS headers) |
| Settings carried over from the egui app | First run seeds the session from egui's `app_state` (dungeons, inventory, standards, constraints, team mapping, event overrides, analyzer filters, chamber setup) |

## Analyzer

| Feature | Prototype status |
| --- | --- |
| Pet Stats paste and file import | Implemented; shared importer, atomic validation |
| Main Stats partial updates | Implemented; shared parsing and selective settings fill |
| Full-save roster/account/Moai import | Implemented; native file path verified |
| Ownership, element, evolution, unlock, class, improvement, campaign filters | Shared egui implementation |
| Name/alias/ability search | Shared implementation, whitespace trimmed |
| Twelve sort modes, direction, time tiebreaks | Shared implementation; clickable headers restored |
| Row ownership, growth readiness, improvement | Restored as compact row status |
| Base/effective growth comparison | Both values shown where different |
| Summary counts, total growth, top-50 DL, egg readiness | Implemented |
| Reference, unlock, special ability, equipment and class details | Implemented |
| Evolution requirements and ETAs, including locked pets | Implemented; regression test |
| Elemental forms and Aether estimates | Existing domain calculations |
| Custom target, pendant cap, Moai, PGC, egg settings | Implemented |
| Campaign totals and innate/class/equipment breakdown | Implemented |
| Earth Eater time to permanent lock | Shared helper restored |
| Wiki links | Implemented |
| Session restore | Separate Slint store; old sessions supported; first run seeded from egui settings |
| Drag/drop import | Implemented on native and browser; stages for explicit import |

Fresh-eyes source re-review found no remaining blocker or should-fix analyzer
feature gaps after restoring the row details and locked-pet estimates. Full
runtime certification remains separate: browser focus/clipboard, file-drop
interaction, narrow layouts and accessibility still need broader coverage.

## Dungeon Logs

| Feature | Prototype status |
| --- | --- |
| HTML file/paste import | Shared parser; invalid imports retain the previous log |
| Clear current log | Implemented; does not clear analyzer data |
| Dungeon/depth/rating/room count | Implemented |
| Party formation order, class, growth, DL/CL, aggregate stats | Shared ordering; overview cards |
| XP, longest fight, party wipe, leech/free XP | Implemented |
| Events, deaths, items used/found/from events | Implemented |
| Per-pet depth statistics | Implemented |
| Room pet selection in formation order with class labels | Implemented |
| Class-selected chart metric and per-room data | Implemented; optional metric override |
| Supporter party damage, healing comparison, signed net | Implemented; regression test |
| Combat room navigation, event/monster/turn summaries | Implemented; unknown monsters preserved |
| Pet HP, monsters, traps, notes, turns/actions, drops | Implemented |
| All-room combat inspection | Implemented with Show all rooms |
| Log storage | In memory only, matching the existing session lifetime |

Presentation differs: horizontal charts, a room selector and an all-rooms mode
replace egui's custom-painted charts and individual collapsing room headers.
All underlying log fields are retained. Raw HTML is parsed as text, never rendered
as executable browser content.

## Dungeon planner

| Feature | Prototype status |
| --- | --- |
| Five dungeons, depths 1–4, static preview and items/gems | Implemented |
| Multi-dungeon assignment without pet reuse | Existing shared solver; integration test |
| Forbid, whitelist, force any/team/exact slot, pause/clear | Implemented with owned-pet search |
| Events through the selected depth, optional defaults | Implemented; events remain active when pet constraints pause |
| Equipment suggestions, provenance, current gear, tier/quality/upgrade, gems | Existing engine and shared equipment matching; display standards configurable |
| Front/back formation, match quality, future classes, unlock suggestions | Implemented |
| Special mechanics, token/stat modifiers, team synergies/anti-synergies | Implemented |
| Team averages, per-depth difficulty, coverage, unavoidable hazards | Implemented |
| Shopping list: unlock/evolve/equipment/gems | Implemented |
| Constraint YAML import/export | Shared schema; atomic name/slot validation |
| Dungeon Teams import with remembered dungeon mapping | Staged, validated, scoped replacement; unrelated constraints retained |
| Solved-team export and clipboard | Shared game format; clipboard errors reported |
| Settings persistence | Separate prototype session; defaults for older sessions |
| Solve responsiveness | Desktop worker; browser deferred main-thread work |

Plans are explicit snapshots: changing roster/settings marks them stale and
requires Solve before exporting. Unlike egui's in-place pet-data refresh, this
avoids mixing old assignments with new inputs. This presentation/workflow
change remains subject to adoption review. Solved plans are not persisted.

Fresh-eyes source review found no blocking/should-fix issue. Native checks
covered setup/depth changes, solve, shopping/party cards, event changes and stale
export gating, mapped team import, and restart persistence. A copied session in
an isolated executable directory was used; the user's original session hash was
unchanged. Browser planner interactions, real clipboard transfer, very narrow
layouts, and exhaustive combinations remain unverified.

## Growth Chamber

The simulation, recommendation and report text live in `crates/planner`
(`chamber.rs`, `campaign.rs`) and are shared with egui, so both apps produce the
same numbers.

| Feature | Slint status |
| --- | --- |
| Chamber roster picker with search, growth and campaign-bonus columns | Implemented |
| Recommend chamber, add/remove pets, per-pet targets | Implemented |
| Cycle hours, max cycles, run until targets, exported-after-campaign | Implemented |
| UPC / PGC / DPC, fishing, food, Gold Dragon, Pandora feedings, adventurer XP | Implemented; filled from Main Stats or a save |
| Rebirth schedule | Implemented |
| Per-pet bonus overrides | Implemented |
| Run report: headline, averages, reward trend, by source, specials, per-pet blocks, class progress | Shared `chamber::report`; hints kept |
| Settings persistence | In the Slint session; results are not persisted (as in egui) |

## Save Editor

Editing goes through the shared `save_parser::EditSession`; the Slint side only
describes rows. All 25 sections are ported:

| Section | Slint status |
| --- | --- |
| Load (pick, drop, clipboard), Save As (full), Save redacted copy | Implemented; browser saves are downloads |
| Pending changes panel: scalar edits, created/added/deleted/pasted groups, undo, jump to tree | Implemented |
| Populate planner, and "on load" | Implemented; remembered |
| Resources, Crystal Factory, Fishing, Daily, Afky God, Campaigns, Dungeons, Planet / UBs, Challenges, Partners | Implemented |
| Pets: filters, selection, bulk ops, per-row overrides, class picker, give equipment | Implemented |
| Equipment: filters, bulk ops, create, equip/unequip, delete | Implemented |
| Inventory, Gems: bulk edits, add, delete | Implemented |
| Physical, Skills, Monsters: sortable tables, bulk ops | Implemented |
| Creations, Monuments, Might, SpaceDim, Divinity Generator | Implemented |
| Pet Village: buildings, quests, statues | Implemented |
| Adventure: inventory, cores, adventurer, professions, class unlocks, research, battle skills | Implemented |
| Raw Save Tree: expand/collapse, filter or reveal-in-place search, cross-reference links, copy path/value/node, paste over node | Implemented |

Presentation differs from egui in two places: its modal dialogs (create
equipment, add item, paste over node, …) are inline rows above the table, and a
row's context menu is a "…" button that opens a row of actions.

The account identifiers in a loaded save stay in memory only. They are never
written to the session; "Save As" keeps them and "Save redacted copy" strips them.

## Verified in a browser (headless Chromium, WASM build)

Clipboard import of a Pet Stats export and of a full save, Growth Chamber
recommend/run/report, Save Editor load, scalar edit, bulk pet edit, undo,
pending-change jump to the tree, tree search (filter and reveal), copy node and
paste over node from the clipboard, create equipment, both save downloads
(re-read with `save-dump`), Populate planner, and the first-run carry-over of
egui settings.

## Not yet verified

- Narrow layouts, keyboard-only navigation, browser accessibility (the browser
  build is a canvas), and browsers other than Chromium.
- Load time of the release bundle on Pages (about 23 MB before `wasm-opt`).
- Browser work runs on the main thread; a large import may pause the UI briefly.
  Desktop parsing and file reads use worker threads.

Keep the egui frontend available until the user chooses adoption.
