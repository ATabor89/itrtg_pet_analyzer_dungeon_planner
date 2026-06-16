# Class-XP validation — ground-truth capture

A real **12 h Growth campaign** captured to validate the Growth Chamber's
**Adventurer class-experience / class-level** model (the phase-2 feature).
Human-reference only (not read by code). The reconciliation it backs lives in
`crates/planner/src/campaign.rs::class_xp_real_run_reconciles`.

> This is a *different* run from `reference/real_growth_campaign/` (which backs
> the deposit/Pandora reconciliation). This one happens to include a **class
> level-up**, which is what makes it useful for the class-XP model.

## The run

- **10 chamber pets**, 12 h, **UPC +40%**. **Recipient: Bag** (lowest growth).
- The pet stats export was taken **just before** the campaign deposited, so each
  Adventurer's Growth is its **pre-deposit** value (and already includes this
  run's ~12 h of Moai — so compare with passive = 0, like the other fixture).
- Of the 10 chamber pets, **7 are Adventurers** (Bag, Hedgehog, Aether, Sphinx,
  Meteor, Otter, Cupid). The other 3 — **Thunder Ball/Raiju, Pandora, Earth
  Eater** — are `None`-class.

## Files

| File | What it is |
|------|------------|
| `1_export_before_campaign_end.txt` | Full pet export just before the campaign deposited. |
| `2_in_game_results.txt` | The in-game campaign results page (contributions + class XP + the level-up). |

## What it confirms

The campaign class-XP formula is `250 · (1 + growth/20000) · hours · M`:

- **Effective multiplier `M` = ×4** (constant **1000**), fit to **±0.02%** across
  all 7 Adventurers. The player's maxed "Camp Exp Boost" pet stone is only +100%
  (×2); the other **×2 is an unidentified in-game source** (TBD — see the status
  doc's open item 10). In the chamber, set **"Adventurer XP ×" = 4.0**.
- **Growth basis is pre-deposit**, and the **recipient earns XP too**: Bag (the
  recipient) ended highest-growth from its own +1,746 deposit yet earned the
  *least* class XP (47,186), fitting its *pre*-deposit growth of 58,661.
- **XP depends only on growth** — not CL or gear: Aether (CL16, full event gear)
  and Sphinx (CL22, no gear) earn near-identical XP at near-identical growth.
- **Only Adventurers earn it**: Raiju / Pandora / Earth Eater (all `None`-class)
  earned **zero**.

XP earned vs. `1000·(1+g/20000)·12` (game value in parens):

| Pet | growth | game XP | formula |
|---|---|---|---|
| Bag (recipient) | 58,661 | 47,186 | 47,197 |
| Hedgehog | 58,692 | 47,204 | 47,215 |
| Aether | 58,694 | 47,205 | 47,217 |
| Sphinx | 58,704 | 47,211 | 47,222 |
| Meteor | 59,080 | 47,437 | 47,448 |
| Otter | 59,913 | 47,937 | 47,948 |
| Cupid | 59,923 | 47,943 | 47,954 |

### The level-up is applied *after* the reward

Hedgehog went **CL 22 → 23** this cycle. Its contribution to Bag (**144.25**)
matches the **CL 22** bonus, not CL 23 — so it contributed at its old level and
*then* leveled:

- bonus @ CL22 = innate 166 (25 base + 141 token) + Magic Stick +20 SSS 50 +
  class 2.58·22 (56.76) = **272.76%** → contribution `(log15(58692) − 1.75) ·
  1.4 · 3.7276 · 12` = **144.32** ✓ (game 144.25)
- bonus @ CL23 would be 275.34% → contribution **145.32** ✗

The sim reproduces this: it computes the contribution from the pre-level-up CL,
then accrues XP and levels up, so the higher bonus only applies the next cycle.
