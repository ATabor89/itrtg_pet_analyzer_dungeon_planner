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

- **Individual challenges** — track per-challenge state (completions, the spent
  `Chp X boost` levels) once past Baal. Candidate blocks: `root.015` (list of
  ~118 {id, …}) and the `root.y/z/A/B` achievement-style lists.
- **Baal / TBS progression** — the player is pushing to beat Baal to move more
  of these counters; revisit `root.T` (Baal Slayer), `root.S` (TBS levels),
  `root.P` (current god) deltas afterward.
