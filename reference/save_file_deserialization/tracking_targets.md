# Save-field tracking targets (not yet located)

Currencies / consumables we want to find in the save. Method is the usual
before/after diff (buy or use some, re-export, diff scalar leaves — see
`Kongregate/experiments.md`). Most "currency" counts are likely dedicated
scalars in the `X` (pet) block or root; consumables may be material-inventory
(`X.Q`) entries.

| Target | What it is | Notes / where to look | Status |
|--------|-----------|------------------------|--------|
| ~~Pet Tokens~~ | Unlock / evolve pets | **DONE: `p.I`** (token-count diff 5→6). `SaveFile.pet_tokens`. | ✅ |
| ~~Class Change Tokens~~ (CC Token) | Re-class an evolved pet for free | **DONE: `p.023`** (diff 8→10). `SaveFile.class_change_tokens`. | ✅ |
| ~~Lucky Draws~~ | Gacha-style draw | **DONE: `p.K`** (unopened; 6→2→0 diff). `x.071` = opened lifetime. `SaveFile.lucky_draws`. | ✅ |
| ~~Ultimate Shadow Summon~~ | Summon currency/item | **DONE: `p.e`** (19→18, Steam diff). `SaveFile.ultimate_shadow_summon`. | ✅ |
| ~~Godly Liquid~~ | Consumable; ×2 creating speed | **DONE: `p.b`**. `SaveFile.godly_liquid`. | ✅ |
| ~~Godly Liquid V2~~ | Stronger variant | **DONE: `p.m`** (50→51). `SaveFile.godly_liquid_v2`. | ✅ |
| ~~Chakra Pill~~ | Consumable; ×2 building speed | **DONE: `p.d`** (156→146). `SaveFile.chakra_pill`. | ✅ |
| ~~Chakra Pill V2~~ | Stronger variant | **DONE: `p.n`** (47→48). `SaveFile.chakra_pill_v2`. | ✅ |

Easiest to diff on the **real save** (just gain/spend a little): Lucky Draws,
Ultimate Shadow Summon, Godly Liquid / V2, Chakra Pill / V2. Pet Tokens are easy
on either (buy with pet stones).

## Future

- **Individual challenges** — per-challenge completion counts. **Day-Pet
  Challenge** is the score-based one and lives at **`x.049`** (3,664,035,884 ≈
  the player's 3.664e9 ✓). The plain *completion counts* (AAC 10/25, Monument
  Multi 13/40, Pet Level 12/25, …) are **not yet located**: they are *not* in
  `root.x` (12 and 10 don't appear there) and *not* in `root.015` (that list is
  id/flag pairs `{a=id,b=1,c=0}`, not completions). Best path: run a challenge
  (e.g. UBC), watch one completion tick +1, diff to find the field/block, then
  mass-tweak completions deterministically to map the rest.
- **Baal / TBS progression** — the player is pushing to beat Baal to move more
  of these counters; revisit `root.T` (Baal Slayer), `root.S` (TBS levels),
  `root.P` (current god) deltas afterward.
