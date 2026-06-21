# Editor architecture review

Decision-oriented review of two raised refactors. **These are recommendations,
not committed work** — nothing here is implemented; the new editor sections
(#2–#5) are intentionally held until you pick a direction, since the choice
changes how they're built.

- **Part 1 — Model / type-driven correctness (#19)** — below.
- **Part 2 — UI framework / egui vs alternatives (#20)** — further below.

**TL;DR of both:** Do the model work (Part 1, **Option A**, low-risk, subsumes
#13 and makes the new sections cheaper). **Keep egui** (Part 2) — the three
concrete wants (scaling, Ctrl+scroll zoom, table sorting) are unwired egui
features, not egui limitations; a framework migration would rewrite ~13k LoC of
view code to solve problems egui already solves. Spend the migration budget on a
small **shared sortable-table helper** instead.

---

## Part 1 — Model / type-driven correctness (#19)

### The problem, quantified

For one save field, its raw key + type + label + constraints are encoded in **up
to four independent places the compiler does not link**:

1. `model.rs` parsing — imperative `get_u32(node, "e")` (the key + Rust type).
2. `labels.rs` — `lbl!("e", "Enchant Level")` (key → display label + `Resolve`).
3. each section editor — the key re-spelled in `set_scalar(&["X","R",&i,"e"])`
   **plus** a per-section key enum (`EField::key()`, `Field::keys()`).
4. constraints — clamps hardcoded in the section (`min(20)`), repeated in the
   label text, with no model-side bound at all.

(`registry.rs` is *not* a fourth copy for block fields — it derives them from
`labels.rs::BLOCKS`. The genuine triple-encoding is model + labels + section.)

**Counts:** ~178 raw-key accessor calls in `model.rs`; **271** `lbl!`/`lblr!`
entries across 43 tables / 46 blocks in `labels.rs`; 74 explicit registry
scalars; ~10 non-test hardcoded `set_scalar` key paths + 3 per-section key enums.

**Worked example (enchant `e`):** the key `"e"` is written literally in 3 files
(model.rs:1452, labels.rs:200, equipment.rs:46/767) and its 0–20 range in 2
(equipment.rs:566 + the label string). `w.d.a` (pet class) is spelled three
*different* ways: nested `.get()` calls, the dotted `"w.d.a"`, and the slice
`["w","d","a"]` — no shared constant.

### Correctness gaps the current design allows

There is **no compile-time link** between "the model parses key X" and "the tree
labels key X". The only net is one runtime test (`every_registry_path_resolves`)
that checks *seeded* paths exist in the fixture — it does **not** check that every
parsed key is labeled, nor catch a typo that lands on a real sibling key (the
`labels.rs` doc already warns the maintainer to "cross-check by hand"). So you
can: add a model field that's silently unlabeled; mislabel onto a real sibling;
or drift a section's key enum (which the coverage test doesn't touch at all).
Constraints (quality 0–8, enchant 0–20) live only in the UI and are bypassed
entirely by raw-tree edits.

### Constraints are duplicated and UI-only

Element ids (0–4) are encoded **4×** (equipment.rs, pets.rs, model.rs,
`Resolve::Element`); class ids 3×; quality/enchant clamps 2–3× each. Nothing
declares "this field is a u32 in 0..=20" once for both the tree and the sections.

### Options

| Option | Scope | ~LoC | Risk | Compile-time coverage | What it buys |
|---|---|---|---|---|---|
| **A — enrich the descriptor** | add `kind`+`range` to `FieldLabel`; data-drive section clamps from it; add a parse↔label coverage test | 250–400, mechanical | **Low** (additive) | No (better runtime test) | one source for constraints, shared tree+section validation, real (runtime) parse/label coverage |
| **B — canonical per-block enum + `SaveField` trait** | one enum per block (key/label/range/resolve); `from_node` and the label table derive from it; sections consume it | 600–900 | **Medium** (touches load-bearing `from_tree`) | **Yes** for keyed fields (missing arm = compile error) | one declaration/field + compile-time exhaustiveness |
| **C — `#[derive(SaveModel)]` proc-macro** | annotate model structs; macro emits parse + labels + registry | 1000+, new crate | **High** (macro debugging; irregular key shapes fight it) | **Yes**, full | maximal dedup, full compile-time coverage |

### Recommendation

**Do Option A now; treat Option B as the eventual target only if compile-time
exhaustiveness becomes worth the churn; skip Option C.**

Rationale: the duplication is real but **already half-consolidated** — the
271-entry `labels.rs` table is the natural single descriptor, and `registry.rs`
proves block fields can be *derived* rather than duplicated. Option A captures
~70% of the benefit (shared constraints across the tree **and** sections, plus a
real parse↔label coverage check that turns today's "cross-check by hand" into an
automated test) for the least risk and **zero churn to `from_tree`**, which every
consumer (planner + GUI) depends on. Option B is the right target if/when
compile-time exhaustiveness is the goal, but its value is lower for a
single-maintainer reverse-engineering tool where fields are added a few at a time.
Option C is over-engineering: the irregular key shapes the macro would have to
handle — nested `w.d.*`, the `t/u` offset-by-1, `999` sentinels, gem-element-
only-if-level>0, `list_or_single` — are exactly where proc-macros become a
maintenance liability.

**Concrete first step (Option A), if approved:** add `kind: FieldKind` and
`range: Option<RangeInclusive<i64>>` to `FieldLabel`; have the section clamps and
the raw-tree numeric editor read the range from the descriptor; add a test
asserting the set of keys `from_tree` reads equals the set `labels.rs` labels
(per block). This also directly subsumes task **#13** (range limits) — limits
become a property of the field, enforced everywhere, instead of per-section code.

### Interaction with the other open work

- **#13 (range limits)** folds into Option A (limits become descriptor data).
- **#2–#5 (new sections)** benefit from Option A: a data-driven section can be
  largely generated from the descriptor (label + kind + range), so building five
  more sections is much cheaper *after* A than before. This is the main argument
  for doing A before the new sections.
- **#20 (UI framework)** is orthogonal to A (A is model-layer), but a
  descriptor-driven model also makes a *future* UI swap cheaper (sections become
  data, not hand-built widgets). Assessed in Part 2.

---

## Part 2 — UI framework / major UI refactor (#20)

### The question

You raised four wants — **DPI/scaling, Ctrl+scroll zoom, table sorting,
composability** — and asked whether they justify moving off egui (to Slint ≥1.0,
a web stack, or staying). The honest finding up front: **three of the four wants
are things egui already does and we simply haven't wired up**, and the fourth
(composability) is a code-organization concern that a framework swap would not
fix for free. So the framing "egui can't do X, therefore migrate" doesn't hold
for the X's on the list.

### What we actually have (measured)

- **~17.7k LoC** in `crates/gui`, of which **~13.1k is view code** woven directly
  into egui (`dungeon.rs` 2.9k, `analyzer.rs` 2.4k, `chamber.rs` 2.0k,
  `log_viewer.rs` 1.3k, `save_editor/*` ~4.4k). **1,433** egui API call sites
  across 19 files.
- **~1.0k LoC is framework-agnostic** and ports as-is: `state.rs` (pure serde
  YAML, **zero** egui types), `data.rs`, `platform.rs`, most of `app.rs`. All the
  *brains* live in `crates/planner` / `crates/models` and never touch the UI.
- Custom painting is **minimal** (~32 sites, simple rects/circles/galleys for
  element badges and status dots) — not a porting blocker.
- WASM ships today via eframe's canvas + Trunk → GitHub Pages, with **21**
  `cfg(target_arch="wasm32")` seams already abstracting file I/O and storage.
  This pipeline works and is free.

**Migration cost, concretely:** a framework swap rewrites the ~13.1k LoC of view
code (the agnostic ~1k survives). For a single-maintainer reverse-engineering
tool, that is the entire GUI rewritten with no new *capability* delivered — only
a different substrate.

### The four wants, against egui's actual capabilities

| Want | Status in our code | egui 0.31 reality | Verdict |
|---|---|---|---|
| **DPI / scaling** | Not wired. Window hardcoded `[1400×850]`, fonts hardcoded (10/11/12px). `grep` for `pixels_per_point`/`zoom_factor` → **0 hits**. | egui scales the whole UI by `ctx.pixels_per_point` (auto-detected from the OS/monitor) and respects per-monitor DPI. | **egui already does this.** We just never set/exposed it. Small task, not a migration. |
| **Ctrl+scroll zoom** | Not wired (0 hits). | egui has built-in zoom: `options.zoom_with_keyboard` + Ctrl+scroll is handled when `zoom_factor` is enabled; `ctx.set_zoom_factor()` is one call. | **egui already does this.** ~10-line wiring task. |
| **Table sorting** | Only the **analyzer** table sorts (custom `SortColumn` enum + clickable headers). The 6 save-editor tables are static. | `egui_extras::TableBuilder` (already a dep) gives layout; sorting is app-side logic, which we've already written once and can factor out. | **Not a framework problem.** Build a shared sortable-table helper; reuse the analyzer's proven pattern. |
| **Composability** | Views are big `fn update()`-driven modules; tabs routed by a `Tab` enum. Works, but the large views are monolithic. | Immediate-mode is inherently function-composition; egui doesn't impose a component tree but doesn't prevent one either. | **A code-organization issue**, addressable in egui by extracting widget fns / a small section trait (the save-editor `sections/*` already does this). A retained-mode framework *enforces* components but at the cost of the rewrite + a less-natural fit for this dense, data-grid-heavy tool. |

### The alternatives, briefly and fairly

- **Slint (≥1.x):** mature, real DPI/scaling, declarative `.slint` markup, good
  native + (improving) WASM. But: it's **retained-mode with its own DSL**, so
  100% of the ~13.1k view LoC is rewritten in a new language; data-grid / dense
  table ergonomics are weaker than `egui_extras` today; the Rust↔.slint property
  bridge adds a layer; and the licensing (royalty-free GPL/community or paid
  commercial) is a consideration for a public repo even if fine here. Big cost,
  and it does not uniquely unlock any of the four wants.
- **Web stack (Leptos/Yew/Dioxus, or TS+React):** best-in-class for
  tables/zoom/scaling/composability and you already deploy to Pages. But a Rust
  web framework is *still* a full view rewrite; a TS/React frontend is a rewrite
  **plus** a new language **plus** a Rust→WASM/JS boundary for the planner, or a
  reimplementation of the planner in TS. Largest cost of the three. Justified
  only if the long-term goal is a polished public web product, not a personal
  power-tool.
- **Stay on egui:** zero rewrite; the four wants are 1 small task (zoom+scale) +
  1 small helper (sortable tables) + ongoing extraction for composability. The
  WASM/Pages pipeline keeps working. Risk: egui's ceiling for *visual polish* and
  complex retained layouts is lower than Slint/web — but this tool is a dense
  data editor, which is squarely egui's strength.

### Recommendation

**Stay on egui. Do not migrate.** Instead, spend a small fraction of what a
migration would cost on three targeted improvements:

1. **Wire zoom + scaling** (~½ day): enable `zoom_factor` / Ctrl+scroll, stop
   hardcoding font sizes (derive from text styles so they scale), and let
   `pixels_per_point` do its job. Delivers two of the four wants outright.
2. **Shared sortable-table helper** (~1–2 days): extract the analyzer's
   `SortColumn`/clickable-header pattern into a reusable `views/widgets`
   helper, then adopt it in the save-editor tables. Delivers the third want and
   pays down duplication.
3. **Incremental composability** (ongoing, opportunistic): keep extracting the
   big views into smaller widget fns / the existing `Section` pattern as you
   touch them. No big-bang refactor.

Reasons a migration is the wrong call here: (a) it solves none of the four wants
that egui doesn't already solve; (b) it rewrites ~13k LoC for a single maintainer
with no capability gain; (c) it risks the working WASM/Pages deploy; (d) the
tool's nature — dense, immediate, data-grid-heavy, internal — is exactly where
egui is strongest and retained-mode frameworks are least differentiated.

**Revisit only if** the goal changes from "personal power-tool" to "polished
public web app for other players," in which case a web stack (not Slint) becomes
the serious candidate and the planner-as-WASM-library boundary should be designed
deliberately. That's a product decision, not a UI-tech decision.

### Interaction with Part 1

Part 1 (Option A) and this recommendation **compound**: a descriptor-driven model
makes the save-editor sections increasingly *data* (label + kind + range), so the
sortable-table helper can render straight from descriptors, and the new sections
(#2–#5) become largely declarative. So the sequencing is: **Option A first**,
then the small egui improvements (zoom/scale, table helper) ride on top, then the
new sections. None of this requires or benefits from a framework change.
