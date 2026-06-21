# Editor architecture review

Decision-oriented review of two raised refactors. **These are recommendations,
not committed work** — nothing here is implemented; the new editor sections
(#2–#5) are intentionally held until you pick a direction, since the choice
changes how they're built.

- **Part 1 — Model / type-driven correctness (#19)** — below.
- **Part 2 — UI framework / egui vs alternatives (#20)** — *(pending; will be
  appended next.)*

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
