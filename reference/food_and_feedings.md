# Food & feedings — growth from feeding pets

Human-reference notes (not read by code). Feeding pets is a meaningful, ongoing
growth source that the growth-chamber simulator needs to model — over days/weeks
of chambers it adds up a lot. This captures the mechanics and the open numbers.

## Feeding cadence

- Each food raises a pet's **hunger bar by 12.5%** (= **3 hours**); the bar caps
  at **24 h** (8 feedings from empty) and **empties full→empty in 24 h**.
- You can feed a pet when its bar is **below 87.5%** — i.e. **once every 3 hours**.
- ⚠️ **Feeding only happens when a pet is *not* in a campaign/dungeon.** So over a
  back-to-back chamber, the bar depletes during each `hours`-long campaign and you
  refill it **between rounds**: **`floor(hours / 3)` feedings per pet per round**
  (12 h → 4). You feed **every** pet, so this growth applies to the whole roster
  (chamber *and* bench), like Moai.
- **Modelling consequence:** feeding growth is applied **between rounds — after**
  each campaign's deposits, **not** folded into the end-of-run growth the campaign
  picks its recipient/contributions from. It compounds into the *next* round's
  campaign. (The engine does exactly this; `end_growth` is passive-only.)
- **Pandora's Box** also counts feedings for its *campaign-total* bonus (separate
  from the growth each feeding gives) — see `campaign_simulation.md` §5.

## Growth per feeding

Each feeding adds a flat growth amount that depends on the **food type** and a
stack of multipliers.

| Food | **Base** growth | Player's **current** (DPC+fishing baked in) |
|------|-----------------|----------------------------------------------|
| Free | 0.75 | 1.3 |
| Puny | 1.5 | 2.6 |
| Strong | 3 | 5.19 |
| Mighty | 4.5 | 7.79 |
| Chocolate | 6 | 10.38 |

The current/base ratio is **~1.733** across the board, so the multipliers scale
all food types equally: `effective = base · foodMultiplier`.

### The food multiplier — its parts

`foodMultiplier` combines (at least) these — **exact composition still to pin
down** (the player's 1.733 doesn't obviously decompose, so there may be another
factor like SpaceDim, or DPC/fishing combine non-additively):

1. **Day Pet Challenge (DPC)** — `+log2(highest pet multiplier)%` to food growth,
   **capped +100%**. *Example: highest pet multi 1.1 million% → log2(1.1e6) ≈ 20 →
   +20%.* The **Main-stats export** has `Day Pet Challenge highest multi`
   (sample `3.664 E+9` → `log2 ≈ 31.8` → +31.8%). Derivable.
2. **Fishing (Fish Power)** — `+ FishPower^0.25 · milestoneMult`%, **decaying
   linearly from 100% → 0% of the boost over 30 h into the rebirth**, capped at
   **+200%**. *Example: 1M FP → 31.6% at rebirth start, 18.96% at 12 h in.*
   - Main-stats: `Fish Power` (sample `1.050 E+6` → `^0.25 ≈ 32`) and
     `Fishing Level` (sample 14).
   - **Milestones:** Fishing **level 15 → +10% FP**, level **27 → +10% FP**
     (assumed **additive**, so a `milestoneMult` of `1.0 / 1.1 / 1.2`).
3. **SpaceDim** — also affects food growth. Details TBD (note for later).

> ⚠️ Sanity-check: DPC +31.8% and fishing +32% give `1 + 0.318 + 0.32 = 1.638`,
> not the observed **1.733** — and the player's fishing should be *below* its
> rebirth-start 32% (decayed), widening the gap. So the parts combine differently
> than naive-additive, or SpaceDim/another factor is in play. **Resolve before
> auto-deriving** the effective values; until then, take the effective per-food
> values as **user inputs** (defaulting to the player's current numbers).

## Modelling decision — simulate from the start of a rebirth

Both **fishing** (decays over 30 h) and **Pandora's feedings** (reset on rebirth)
are rebirth-relative. We don't reliably know "hours into the current rebirth" from
the export. So the simulator should model **from the start of a rebirth**:
fishing boost at 100%, Pandora feedings starting from the entered value, and a
user-input **typical rebirth length** to drive the decay/accumulation across the
run. This is realistic and a real lever:

- **Short rebirths** (e.g. 1 day) → more fishing boost on average, but Pandora
  never accumulates many feedings.
- **Long rebirths** (e.g. 7 days) → fishing decays away, but Pandora's feeding
  bonus maxes out.

## Gold Dragon — feeding broadcasts 25% to everyone

Feeding **Gold Dragon** gives **every** pet **25% of the growth he gains** from
that feeding — campaign-independent, and it adds up a lot (the Main-stats export
tracks it as `Growth from Golden Dragon`, sample `181,144`). So per round each pet
gets an extra `floor(hours/3) · 0.25 · goldDragonFoodGrowth`, on top of its own
feeding. He's best fed **chocolate**, and he has his **own food selector**.

- Modelled by folding `0.25 · gdFoodGrowth` into every pet's per-feeding growth
  (`per_feeding_growth = ownFood + 0.25·gdFood`), applied between rounds like any
  feeding.
- Minor approximation (per-pet food is deferred): Gold Dragon himself is fed the
  *general* food in the sim rather than his own — negligible (one pet), and the
  big effect is the broadcast to all.

## Related multipliers (for evolutions, not campaigns)

- **PGC (Pet Growth Challenge)** — all completed gives a **1.5× growth**
  multiplier. With the Magic Egg that's `1.5 · 1.3 = 1.95×` total. Affects
  **evolve thresholds** (the egg discount becomes `/1.95` instead of `/1.3` once
  PGC is done). The player has none yet — slot into the evo-readiness logic later.
- **SpaceDim** — affects food growth (above) and possibly more; TBD.

## Implementation status

- [x] **Feeding growth in the chamber engine** — each pet gains
  `floor(hours/3) · effectiveFoodGrowth` per round, applied to the whole roster
  **between rounds** (after the campaign, excluded from `end_growth`). The
  effective per-food value is a (user-set) input for now.
- [x] **Gold Dragon broadcast** — `+0.25 · gdFoodGrowth` per feeding to every
  pet, via its own food selector.
- [ ] **Auto-derive** the effective values from base × DPC × fishing (needs the
  composition pinned down + the SpaceDim factor).
- [ ] **Rebirth-relative decay** (fishing over 30 h, Pandora accumulation) driven
  by a "typical rebirth length" input, simulating from rebirth start.
- [ ] **Per-pet food** override (vs the global food type).
- [ ] **PGC** into evo thresholds; **SpaceDim** factor.
