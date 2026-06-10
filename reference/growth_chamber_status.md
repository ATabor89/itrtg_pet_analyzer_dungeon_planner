# Growth Chamber — status, design decisions, and open work

Single source of truth for the **Growth Chamber** simulator: what it does, the
load-bearing decisions behind it, and everything still open. Human-reference (not
read by code). Start here before extending the chamber.

The chamber lives in two files plus supporting data:

- **`crates/planner/src/campaign.rs`** — the engine. Key items:
  `simulate_growth_chamber` (the per-cycle loop; takes a `ChamberRun` config),
  `growth_campaign_detailed` (per-pet contributions + recipient),
  `apply_growth_specials` (Pandora/Bag layer), `pandora_pct`,
  `fishing_boost_pct` + `fishing_decay`, `nightmare_malus`, `rebirth_schedule`,
  and the types `ChamberRun`, `ChamberPet`, `ChamberCycle`, `ChamberResult`,
  `GrowthBreakdown`, `SpecialPet`.
- **`crates/gui/src/views/chamber.rs`** — the view. `ChamberState` (persisted
  inputs), `show`, `chamber_pet`/`build_roster` (roster → sim bridge),
  `effective_export`/overrides (what-if editing), the pet cards, the run report,
  `apply_main_stats` (auto-fill), and the food/DPC/fishing/rebirth helpers.
- **`crates/models/src/main_stats.rs`** — parses the "Main stats" export (UPC,
  DPC multi, Fish Power, Fishing Level, …).
- **`crates/pet-importer/src/parser.rs`** — parses the pet export (see the Magic
  Egg note below).
- **`crates/planner/src/merge.rs`** — `campaign_bonus_for` (each pet's per-campaign
  bonus = innate + equipment + class), `event_equip_bonus`, `stick_bonus`, the
  Adventurer evo-bonus table, and the curated overrides.
- **`data/campaign_overrides.yaml`** — e.g. Nightmare's `set_all: 200`.

## What's implemented (and validated)

- **Campaign math** — per-pet contribution `(log15(growth) − 1.75) · (1+UPC) ·
  (1+bonus) · hours`, recipient = lowest end-of-run *total* growth, deposit summed
  into the recipient. **Reconciled to the decimal** against a real 12 h run
  (`reference/real_growth_campaign/`, test `pandora_egg_real_run_reconciles`).
- **Special pets** — **Pandora** (boosts the recipient's deposit by its
  growth/feeding %), **Bag** (gifts the global-lowest pet; token-improved = free
  5%, pre-token = steals 10%), **Nightmare** (subtractive team malus).
- **What-if editing** — per-pet override of equipment + CL on the card, recomputed
  live through a synthetic export (`effective_export` → `campaign_bonus_for`); no
  engine change. "Refresh from export" reverts.
- **Growth breakdown** — each pet's gain split into campaign / passive / feeding /
  Gold Dragon (sums to the total gain; total terms).
- **Rebirths** — `rebirth_schedule` runs full cycles + one shorter remainder cycle
  per rebirth (a campaign can't span a rebirth). Decimal length, Hours/Days/Weeks.
- **Rebirth-relative effects** — **fishing** food boost (decays over the first
  30 h) and **Pandora feedings** (accumulate, reset each rebirth).
- **Food auto-compute** — `BASE_FOOD × (1 + DPC%)`, fishing layered on in the sim.
- **Main-stats auto-fill** — UPC, PGC done/max, DPC multi, Fish Power, Fishing
  Level.
- **PGC** — the Patreon-God-Challenge growth multiplier (`pgc_growth_mult`:
  **+1% per completion, doubled once all are complete** — 24/25 → ×1.24 but
  25/25 → ×1.50) folds into each pet's `growth_multiplier` (stacks with the
  egg: at 25/25, 1.5 × 1.3 = 1.95×). Done/max auto-filled from the Main-stats
  `Patreon Gods Challenges` line.
- **Report** — elapsed time (h → days → years), per-pet time-to-target, and a
  linear-extrapolation **ETA** for pets that miss their target. The ETA rate is
  the pet's **recent** growth/cycle (`recent_rate_per_cycle`: last two
  recipient rotations, campaign income from the trace + uniform
  passive/feeding), since the whole-run average lags the rising reward curve
  and landed ~3 cycles late.
- **`exported_after_campaign` toggle** — suppresses the first cycle's passive (see
  the Moai-boundary note).

## Load-bearing design decisions (don't relearn these the hard way)

- **Magic Egg / base-vs-total growth.** The **pet importer divides the egg's ×1.3
  back out** (`parser.rs`, the `has_magic_egg` block) and stores **true base**
  growth. The chamber keeps `growth` (base) + `growth_multiplier` (egg = 1.3) and
  uses **total = base × mult** for the campaign formula, recipient selection, Bag,
  Pandora, and targets; **rewards, passive, and feeding land on base**. Never
  multiply the egg in twice — feeding the raw export value as if it were base
  inflates Pandora and the deposit (this was a false-alarm "bug" during
  validation). `effective_export` applies overrides on a clone; `chamber_pet`
  recomputes the bonus from it.
  **Export rounding (validated in-game 2026-06, Pandora's Box):** growth is
  fractional internally (base 44334.321…, total 57634.617… = base × 1.3
  exactly) and the export shows the **rounded** total (57,635) — normal
  rounding, not floor. So the importer's `round(export / 1.3)` is the right
  inverse, recovering base to within ±1; exact recovery would need the save
  file, and ±1 is noise at campaign magnitudes. Don't revisit.
- **Moai boundary artifact.** An export captured at a campaign's *end* already
  contains that campaign's ~12 h of Moai. The sim, starting from that value, would
  re-add a full cycle of passive (~+24/pet). The `exported_after_campaign` toggle
  zeroes the first cycle's passive to fix it. Non-compounding; matters most for
  short, near-target rushes and exact validation.
- **Food composition is MULTIPLICATIVE:** effective food = `base × (1 + DPC) ×
  (1 + fishing·decay)`. Additive was wrong (observed 1.733 vs additive 1.638;
  multiplicative 1.318 × 1.32 = 1.740). The sim applies `fishing_mult` as a
  multiplier on `food_per_feeding` (= base×DPC) — keep it multiplicative.
- **DPC boost** = `log2(highest pet multiplier)` capped 100% (the multiplier is a
  *percent number*, e.g. 3.664e9). Use the **accurate** log — the in-game tooltip
  rounds (32% shown, 31.77% actual; the actual reproduces the food values exactly).
- **Fishing** = `FishPower^0.25 × milestone` (×1.0/1.1/1.2 at fishing level
  <15/15/27, additive milestones), capped 200%, decaying linearly to 0 over a
  rebirth's first 30 h, evaluated at the cycle midpoint. **Only applies with
  rebirths modelled.**
- **Pandora feedings** accumulate `+floor(hours/3)` per cycle (bonus caps at 20),
  reset at each rebirth start; the input is the *starting* count. Pandora's own
  **+200%** is in `campaign_overrides.yaml` (`set_all: 200`), not a special.
- **Nightmare malus** is **subtractive**: `(20 − 0.25·CL)` points off every *other*
  chamber pet's bonus, floored at 1. Implemented from the wiki — see open items.
- **Rebirth assumption:** simulate **from the start of a rebirth** (fishing at
  full, Pandora at the entered count). Cycle 0 is a full cycle.

## Open / deferred work

Roughly highest-leverage first. Each has enough context to start cold.

1. **PGC follow-ups.** The chamber side is done (see above). Still open:
   (a) **evo thresholds / analyzer** — `evo_readiness`, `hours_to_evolve`, and
   the analyzer's growth-target ETA only discount the Magic Egg (`/1.3`); with
   PGC the discount becomes `/(1.3 × pgc_growth_mult)` and `effective_growth`
   gains the global PGC multiplier — needs done/max threaded into those
   `MergedPet` seams.
   (b) **importer check** — verify whether the *pet* export's growth column
   includes the PGC multiplier (the way it includes the egg's ×1.3). If so,
   `pet-importer` must divide it back out (like the `has_magic_egg` block) or
   every stored base growth inflates — and the chamber, folding the multiplier
   in again, would double-count. The importer can't detect PGC on its own
   (it's in the Main-stats export, not the pet export), so this likely needs a
   flag/setting. Since the bonus is **gradual** (+1%/completion), this becomes
   testable as soon as the **first** challenge is completed: compare a pet's
   export growth right before/after. Player is at 0/25 today.
2. **Event-gear levels.** Candy Cane / Merry Mantle / Christmas Boots are pinned to
   **SSS+20** (in `weapon_for`/`armor_editor`/`accessory_editor`, and
   `merge.rs::event_equip_bonus` only scores SSS+20). The override model already
   stores arbitrary `Equipment` levels — so the work is (a) replace
   `event_equip_bonus` with real level formulas once known, (b) un-gate the
   quality/upgrade sub-row (currently `is_stick(w)`-only) for event gear.
3. **Per-cycle click-through pop-out** (the "fun" one). Step through each cycle
   individually. The sim already records `ChamberResult.trace` (per-cycle
   recipient, recipient_gain, bag_gift, contributions, hours).
4. **Nightmare validation + God-Power uncap.** The malus is from the wiki text, not
   a live capture — confirm the **ordering** vs Pandora/Bag with a real
   Growth-campaign-with-Nightmare export (he's CL 17, parked in God Power for now).
   Separately, the **GP uncap** (Nightmare always; Ant Queen after evolving) is
   unmodelled — only relevant if the God Power campaign is ever simulated.
5. **SpaceDim factor.** The reference flags SpaceDim as affecting food growth (TBD).
   Currently `base × DPC × fishing` matches in-game exactly, so it's either baked
   into the pet multiplier/DPC or negligible for the player. Revisit if food
   values diverge.
6. **Per-pet food override.** One global food type today (Gold Dragon already has
   its own selector). Could allow per-pet food.
7. **Limited-item caps (phase 3).** At most ~2 of Magic Egg / Growing Love Pendant
   / each event piece across chambers (generalize the old `pendant < 2` gate).
   Spec is fuzzy — "how many you own" isn't cleanly in the export. Low priority.
8. **Fresh-rebirth validation of fishing/Pandora.** Both modelled from tooltips;
   a real fresh-rebirth capture (fishing is dormant for the player now — rebirth
   too old) would confirm them.

## Validation assets

- `reference/real_growth_campaign/` — a complete real 12 h run (before/after
  exports + in-game log) with a README; backs `pandora_egg_real_run_reconciles`.
- `reference/campaign_simulation.md` — the campaign formulas (Growth + §5
  special-pet layer).
- `reference/food_and_feedings.md` — feeding/Gold-Dragon/fishing/DPC mechanics and
  the (now mostly-resolved) food-multiplier composition.
- `reference/chamber_validation.md` — the earlier per-pet bonus validation snapshot.
