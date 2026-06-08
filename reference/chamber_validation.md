# Growth-chamber validation — real run (in progress)

Human-reference notes (not read by code). A live 12-hour Growth-campaign run from
the player's chamber, used to validate the campaign-bonus model and (when it
finishes) the Growth formula + special-pet layer. **Data is still arriving** —
this is the running log; the final per-pet contribution breakdown lands when the
campaign completes.

## What the game shows

For each pet in a campaign, the game shows **"total reward for this campaign"**,
where **100% is the default** (no bonus). So our computed bonus =
`in-game value − 100`. Penalties read below 100%.

UPC: the player has **8 Ultimate Pet Challenges** ⇒ UPC multi **+40%** (`5·8`).

## Snapshot (start of run)

| Pet | Growth | Equipment | CL / state | In-game | ⇒ bonus |
|-----|--------|-----------|-----------|---------|---------|
| **Otter** (recipient) | 55,258 | Magic Stick SSS+10 | CL 19 (98%) | 254% | +154 |
| Cupid | 55,330 | — (upgrading) | CL 20, token | 284% | +184 |
| Bag | 55,460 | Magic Stick SSS+10 | CL 16, token | 215% | +115 |
| Hedgehog | 55,557 | — (upgrading) | CL 22, token | 323% | +223 |
| Thunder Ball | 55,653 | Candy Cane + Merry Mantle + Christmas Boots (all SSS+20) | not evolved | 581% | +481 |
| Meteor | 55,848 | Magic Stick SSS+10 | CL 19 | 240% | +140 |
| Earth Eater | 55,935 | Magic Stick SSS+20 | not evolved, 82% | 232% | +132 |
| Sphinx | 56,169 | Magic Stick SSS+10 | CL 21, token | 220% | +120 |
| Pandora's Box | 57,127 | — (egg) | not evolved, 8 feedings | 100% (+43.42% campaign total) | +0 |
| Vampire | 57,302 | Candy Cane + Merry Mantle + Christmas Boots (all SSS+20) | not evolved | 570% | +470 |

Building blocks our model uses:
- **Magic Stick** SSS+10 = `50·(11/21)` = **26.19**; SSS+20 = **50**.
- **Event set** (SSS+20) = Candy Cane 101 + Merry Mantle 150 + Christmas Boots 150 = **401**.
- **Adventurer class** = `(2 + evo)·CL` (evo: Cupid 0.5, Sphinx 0.68, Hedgehog 0.58, Otter 0.8, Meteor 0.85, Bag 1.0). Only evolved-as-Adventurer pets get it; "not evolved" pets get none.

## Model vs. in-game (growth-campaign bonus)

| Pet | innate | + equip | + class (Adv) | = model | in-game | |
|-----|-------|---------|---------------|---------|---------|--|
| **Earth Eater** | 82 (locked, not token) | 50 | — (not evolved) | **132** | 132 | ✅ exact |
| **Thunder Ball** | 80 (Raiju) | 401 | — | **481** | 481 | ✅ exact |
| **Vampire** | 69 | 401 | — | **470** | 470 | ✅ exact |
| **Pandora** | 0 | — | — | **0** + 43.42 total | +0, +43.42 | ✅ exact |
| **Hedgehog** | 166 (25 + 141 token) | — | 56.76 `(2.58·22)` | **222.76** | 223 | ✅ (rounds) |
| **Sphinx** | 37.5 | 26.19 | 56.28 `(2.68·21)` | **119.97** | 120 | ✅ (rounds) |
| **Cupid** | 30 (token) + **104 partners** | — | 50 `(2.5·20)` | **184** | 184 | ✅ exact |
| **Otter** | 75 | 26.19 | 53.2 `(2.8·19)` | **154.39** | 154 | ✅ (rounds) |
| **Bag** | 41.27 (`10,950^0.4`, Wolf) | 26.19 | 48 `(3.0·16)` | **115.46** | 115 | ✅ (rounds) |
| **Meteor** | 59.23 (`25 + 4501^0.42`) | 26.19 | 54.15 `(2.85·19)` | **139.57** | 140 | ✅ (rounds) |

**All 10 pets now confirm the model.** Every evolved chamber pet is an Adventurer
(confirmed), so the class layer applies to each.

### What this confirms
A lot of the model lands **exactly** against live in-game numbers:
- **Stick formula** at SSS+10 (26.19) and SSS+20 (50).
- **Event-equipment** SSS+20 values (101 / 150 / 150 = 401).
- **Earth Eater** locked +82 (not token-improved).
- **Pandora's Box** special: `(57,127/5,000)·(3 + 8·0.1)` = **43.42%** total-campaign bonus, exactly.
- **Hedgehog** token-improved growth boost (+141 → 166).
- **Cupid's partner bonus** — the whole point of the recent change: **+104** (104 partnered pets) is required to hit 184. ✅
- **Adventurer class** `(2+evo)·CL` with the curated evo bonuses (Cupid 0.5, Sphinx 0.68, Hedgehog 0.58, Otter 0.8, Meteor 0.85).
- **Meteor's** `25 + hours^0.42` time-in-campaigns formula (new).

### Resolved (were flagged)
- **Meteor — was a missing formula, now modelled.** Its bonus isn't the wiki's
  static +25; the tooltip gives **`25 + hours_in_campaign^0.42`** (all campaigns).
  At 4,501 hours → 59.23, so 59.23 + 26.19 + 54.15 = 139.57 ≈ 140. ✅ Implemented
  (`meteor_campaign_hours` input). Its evo-bonus line also confirms our Adventurer
  math: "extra 16.15% (0.85 · class level)" = `0.85 · 19`.
- **Bag — confirmed.** Its growth bonus is `lowestUnlockedPet.growth^0.4` (cap
  100); the player's lowest unlocked pet is **Wolf ≈ 10,950 growth** →
  `10,950^0.4 = 41.27`, so 41.27 + 26.19 + 48 = 115.46 ≈ 115. ✅ (Bag also applies
  its token-improved **+5% free gift** to the recipient — part of the still-pending
  special-pet layer.)

## To complete when the run finishes

The finish screen gives the exact per-pet contribution breakdown. Capture:
1. **Final growth** of every pet right before completion (squeeze the last Moai ticks).
2. **Per-pet contribution** to the campaign (validates `(log15(end_growth) − 1.75)·factor` with `factor = 1.40 (UPC) · (1 + bonus/100) · 12`).
3. **Recipient (Otter) total gain**, including:
   - **Pandora's Box** +43.42% applied to the campaign total.
   - **Bag's** token-improved **+5%** free gift to the recipient.

These feed the **special-pet layer** (Pandora flat bonus, Bag gift) — currently
not in the simulator — and let us turn this into a hard end-to-end test
(`planner::campaign`) of one real chamber cycle.
