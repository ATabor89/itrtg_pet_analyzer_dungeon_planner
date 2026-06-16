# Kongregate fresh-save purchase-diff experiments

The clean way to identify a save field for an upgrade: start from a
**zero-purchase Kongregate save**, buy a small *labelled* batch of one thing,
re-export, and diff the two trees. Every field that moved is attributable to
that batch.

The Kongregate "Save to file" export uses the LZF container (see
`../FINDINGS.md`), which the tooling now decodes. The save `.txt` files
themselves are **gitignored** (`*_SaveItRtG.txt`) — they're unredacted real
saves; only this log is committed.

## Workflow

```
# 1. Capture a fresh zero-purchase save in Kongregate → "Save to file".
# 2. Bankroll it (output must start with edited_; it's gitignored):
cargo run -q -p save-parser --bin save-edit -- <fresh>.txt edited_<fresh>.txt --stones 999999999 --gp 999999999
# 3. Import edited_<fresh>.txt into Kongregate, buy a labelled batch, re-export.
# 4. Diff the two trees (flatten + compare scalar leaves).
```

## Naming convention

`<state>-<YYYY-MM-DD>_SaveItRtG.txt`, where `<state>` describes what's been
bought since fresh, e.g. `FreshSave-WithPet`, `1DngExp2DngLoot`. Keep each
experiment's before/after pair together.

## Results

### 2026-06-16 — Dungeon Loot / Dungeon Exp

- Before: `FreshSave-WithPet-2026-06-16` (1 pet unlocked, on the pet-stones
  page, no purchases).
- After: `1DngExp2DngLoot-2026-06-16` (bought 1 Dungeon Exp + 2 Dungeon Loot).
- **Findings:**
  - `p.017`: 0 → 50 → **Dungeon Loot** (2 buys × +25%).
  - `p.019`: 0 → 25 → **Dungeon Exp** (1 buy × +25%).
  - `X.y` (pet stones): −750,000; `X.z`: 0 → +750,000 → **`X.z` = cumulative
    pet stones spent**. Cost check: 2·275k (Loot) + 200k (Exp) = 750,000 ✓
    (matches the wiki costs).
  - Incidental movers (ignore): timestamps (`c`, `005`), the god-kill +
    pet-unlock (`y/z` achievement flags, `o.*`, big doubles `b`/`d`), TBS
    countdown timers (`T.f[].d`), and the cluster of `x.*` / `p.013/L/S`
    time-based accumulators that all drift by the same amount.
