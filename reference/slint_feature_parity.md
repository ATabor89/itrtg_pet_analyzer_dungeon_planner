# Slint feature parity checklist

This checklist compares behavior with the existing egui frontend. It is not an
adoption or merge approval. The prototype remains isolated on feat/slint-prototype.

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
| Session restore | Separate prototype store; old sessions supported |
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
layouts, and exhaustive combinations remain unverified. Inventory/data refresh
and cross-view save-state import remain part of application integration work.

## Remaining application views and integration

- Dungeon planner: implemented; broader platform and integration validation pending.
- Growth Chamber: not migrated; read growth_chamber_status.md before touching it.
- Save editor: not migrated; preserve staged edits, validation, round-trip and privacy rules.
- Data refresh/export, full cross-view state migration and release/deployment checks: pending.
- Browser work currently uses deferred callbacks on its main thread; large imports
  may pause the UI. Desktop parsing/file reads use worker threads.

Keep the original egui frontend available until all application features and
platform checks pass and the user chooses adoption.
