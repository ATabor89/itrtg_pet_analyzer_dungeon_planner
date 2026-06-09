# Real Growth campaign — ground-truth capture

A complete, real **12‑hour Growth campaign** captured from the game, kept as
ground truth for validating the growth-chamber simulator. Human-reference only
(not read by code). The reconciliation it backs lives in a unit test:
`crates/planner/src/campaign.rs::pandora_egg_real_run_reconciles`.

## The run

- **10 chamber pets**, 12 h, **UPC +40%**.
- **Recipient: Bag** (lowest growth; token-**improved**, so its gift is the free
  5% and is not stolen from its own deposit).
- **Pandora's Box** carries a **Magic Egg** and had **16 feedings** (drives its
  special boost to the recipient).
- Feeding was **chocolate** for everyone: 10.38 growth/feeding, plus **Gold
  Dragon** broadcasting **2.60** (his 25%) to every pet.
- Meteor was at 4524.51 h before / 4536.51 after (its Adventurer "Other").

## Files

| File | What it is |
|------|------------|
| `1_export_before_campaign_end.txt` | Full pet export **~1 min before** the campaign finished. |
| `2_in_game_results.txt` | The simulator's output **and** the in-game campaign log, side by side. |
| `3_export_after_campaign.txt` | Full pet export **right after** the campaign deposited. |
| `4_export_after_feeding.txt` | Full pet export **after** the post-campaign feeding round. |

> ⚠️ The "before" export was taken ~1 min before the campaign **ended**, so it
> already contains that campaign's full 12 h of Moai passive growth (the reward
> had not yet deposited). Account for this when comparing — see below.

## What it confirmed

Bag's deposit, the per-pet contributions, and Pandora's special boost reconcile
to the game **to the decimal**:

| | sim | game |
|---|---|---|
| base total (9 contributors) | 1098.30 | 1098.3 |
| Pandora's own contribution | 38.58 | 38.57 |
| Bag's deposit (base × Pandora's +52.8%) | 1678.39 | 1678.4 |

In-game, Bag's growth went **55,678 → 57,357 (+1,679)**, matching the log's
"+1678.4" (= 1098.3 contributions + Pandora's +580 special).

### Magic Egg is not double-counted

The export "Growth" column is the **egg-inclusive total** the game displays and
uses. The importer (`pet-importer/src/parser.rs`) **divides the ×1.3 back out**
and stores the true base (Pandora's shown 57,410 → base 44,162); the chamber
re-applies `growth_multiplier = 1.3`, so the campaign reads 57,410 — exactly the
value the game uses. Seeding the *raw* 57,410 as if it were base (then ×1.3)
would inflate Bag's deposit to ~1,938; the pipeline does not do that.

### The only residual: first-cycle Moai (a boundary artifact)

The simulator ends Bag ~**+24** above the after-feeding export (`file 4`), and
the same ~+24 shows on *every* pet (a uniform offset — the signature of Moai,
not a per-pet formula error). It's the **first cycle's Moai passive growth**:
`file 1` already holds the finishing campaign's 12 h of Moai, but the sim starts
from that value and adds a fresh 12 h for cycle 1. (Comparing against `file 3`
instead adds a further ~+52 — the *next* feeding round, which the sim includes
in its cycle but the campaign-completion screen has not applied yet.)

This never compounds (a one-time constant offset) and is negligible over many
cycles. To validate cleanly, compare the **deposit** (exact match) or export
when pets are **idle/between campaigns** so no campaign Moai is pre-baked.
