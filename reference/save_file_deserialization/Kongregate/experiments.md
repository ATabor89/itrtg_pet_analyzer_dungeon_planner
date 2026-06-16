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

### 2026-06-16 — upgrade sweep (steps 2–11, one buy each)

Each save is one purchase past the previous (plus heavy GP/clone/creating/
building leveling = noise). Narrowing each diff to `root.p.*` (minus the
continuous GP-progress fields `j/v/h/i/a/b/c/d/L/S/013/N/m/n`) isolates the buy:

| step | bought | field | move |
|------|--------|-------|------|
| 2 | Crafting Boost | `p.020` | 0 → 25 |
| 3 | Crafting Queue Slot | `X.032` | 0 → 1 (not in `p`; pet block) |
| 4 | Improved Campaign Cancel | `p.010` | False → True |
| 5 | Optimal Campaigns | `p.033` | 0 → 1 |
| 6 | Pet Half Stats (GP) | `p.Y` | False → True |
| 7 | Config Half Stats ("Custom Pet Clones") | `p.008` | False → True |
| 8 | Auto Worker Clones | `p.034` | 0 → 1 |
| 9 | Improved Next At For Challenges (also grants base Improved Next At) | `p.B` + `p.U` | both False → True |
| 10 | Inventory Space (+50) | `p.018` | 50 → 100 |
| 11 | Item Slot (3→4) | `p.021` | 3 → 4 |

Cost cross-checks held (e.g. Crafting Queue Slot `X.y` −500,000 = wiki 500k).
Step 9 flips two flags because buying "Improved Next At For Challenges" also
grants the base "Improved Next At" GP upgrade (player confirmed base shows as
already-purchased on the God Power page); `p.B`/`p.U` are that pair, base vs
for-challenges TBD.

Not yet captured:
- **Crystal Improve** (+25%, a *different* field from `p.020`) — needs crystals
  unlocked first; also a real-money purchase (the pet-stone and cash versions
  are mutually exclusive).
- **Refrigerator** — needs Nevermelt Ice + pet stones, or real money.
- **Auto Select Camp** — doesn't appear in the UI yet; likely gated behind
  having run a campaign. A snapshot was kept with a campaign running; reloading
  after it completes may expose this (and other campaign-completion unlocks).

Real-money upgrades (Refrigerator, Crystal Improve, …) can't be exercised by
buying, so they stay unmapped unless acquired some other way.

**Method note:** restrict each consecutive diff to `root.p.*` (the upgrade
block) and ignore the continuous GP-progress keys; the one-time upgrade flip
then stands out even under heavy leveling noise.
