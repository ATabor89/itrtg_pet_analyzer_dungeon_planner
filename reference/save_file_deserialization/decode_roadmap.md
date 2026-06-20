# Save-decode roadmap & data requests

The goal (user, 2026-06-19): **break down essentially the entire save file** via
the decompiled C# (see `cs_crosscheck_handoff.md` for method). This file tracks
the remaining work and — per the user's request — **what each item is, what we
gain, and what (if anything) is needed from the user** to finish it.

Working method recap: the real per-class deserializer is `EBOFJJHOOLP`; full
field/enum state is in `FINDINGS.md`; the gitignored `_cs_decomp/_PROGRESS.md`
is the running scratch log. **In-game displays are ground truth** when more
precise than our derivations (the display may round; e.g. equipment formulas are
pulled straight from tooltips). Use **Steam saves only** for real values
(committed Kongregate saves are heavily edited).

## Done (for context)
Pet struct 100% decoded; full equipment (`MBBDNNAMMHO`) + material
(`NCPJFPLCPPK`) enum tables; pet-type/form/class/element/campaign/feeding/quality
enums; `t`=Favorite Camp/`u`=Hate Camp; `X.028`=achievements/milestones catalog
(`FGGLKEBGPHJ`). Bugs fixed: elemental bars, equipment `d`/`h` editor mislabel,
dungeon-exp DL9 off-by-one. Formulas pinned: growth pendant cap, normal-stats
`inc(L)` staircase, **equipment campaign-boost** (below).

---

## Equipment campaign-boost formula — SOLVED (from C#, verified vs user data)
General (effect `CDGBDDEPIJE.CampaignBoost`, `DOBKHNKLLLM` value getter):

```
boost% = base × (1 + quality_id) × (1 + plus) × factor
```
- `quality_id`: F=0 … SSS=8 (Ult=9) → the "quality multiplier" is `1 + id` (1…9).
- `plus`: upgrade level → `1 + plus`.
- `factor`: 1 for normal campaign gear (Magic Stick, Candy Cane); special crafting
  tiers/SoulSword use 6/12/3 — not the campaign-boost items.
- `base` is per type: **Magic Stick = 0.2646** (tooltip-confirmed; SSS+20 →
  0.2646·9·21 = **50.0%** = its "up to 50%"). **Candy Cane = 0.5292** (= 2× Magic
  Stick).

**Candy Cane SSS overrides** (it's the only +30-capable item; game doesn't show
the formula): exact `+20→101`, `+25→125`, `+30→150`; every other plus uses the
general formula (so SSS+21 = 0.5292·9·22 = 104.78 ≈ user's 104.76 ✓).

Sibling effect formulas in the same getter (for later): BonusExp =
`min(2.5·plus, 8)·quality_id`; BonusGrowth = `min(1.5·plus, 8)·quality_id`.

**TODO (next):** read each event item's `base` (OICKAGGCPGA per type) from the C#
to give exact boosts at *both* purchase tiers (pet-stone S+10 and cash SSS+20)
and all interim upgrade levels; promote to `items.rs`/model so the planner can
show partially-upgraded event gear. **Need from user:** ideally one in-game
data point per event item at a known quality/plus to sanity-check each `base`
(the C# gives it, but a single mouse-over per item is a cheap cross-check).

---

## Remaining targets (priority order)

### 1. Challenges, Challenge Points (ChP), Overflow Points  — HIGH, user priority
- **What:** the challenge system — per-challenge completion/levels, ChP
  total/used/available, Overflow Points.
- **Gain:** big unmodeled progression axis; ChP upgrades feed many bonuses.
- **Status:** ChP "available" shown to be *derived* (no stored scalar) in earlier
  probing; Overflow Points likely a stored counter inside the statistics block
  `ACDDNFHBJCD` (root `O`). Needs C# tracing of the challenge block + the ChP
  total/used/derive logic.
- **Need from user:** a before/after **Steam** save pair around a known change —
  e.g. complete one challenge (or spend/earn a known # of ChP / Overflow Points)
  and export immediately before and after. Note the exact in-game ChP
  total/used/available and Overflow Points at each capture. That pins the fields
  fast (the value-shape search already ruled out plain storage for ChP-available).

### 2. Adventure mode (skills, classes, exp, stats, crafting/smithing/alchemy)  — HIGH
- **What:** the full Adventure-mode subsystem (`root.032`): adventurer skills,
  classes, experience, stats, and the crafting/smithing/alchemy progression.
- **Gain:** a whole secondary game mode currently only partly mapped (researches,
  inventory `032.d`, cores `032.G` done).
- **Status:** decode the `032` sub-blocks via the C# (find the `root.032` class
  and its sub-structs). Mostly C#-tractable; enums for skills/classes likely exist.
- **Need from user:** likely nothing to start (C# + the existing Steam saves);
  may want an Adventure-mode stats screenshot to cross-check derived stats.

### 3. Growth Chamber "rogue bonus" (the ×2 / loose end)  — MEDIUM, user-flagged
- **What:** the unidentified extra multiplier in the chamber sim (`campaign.rs`);
  user reframed it as a **Rogue** bonus — likely a Rogue-class campaign-reward
  multiplier.
- **Gain:** makes the Growth Chamber projection exact.
- **Status (2026-06-19 investigation):** the intuition is **structurally
  confirmed** — there *is* a per-class campaign-reward multiplier in the C#:
  `CIEAPBPBCLL.KEPCFJNJDPJ(pet)` applies class bonuses, and for a **Rogue** pet it
  multiplies the reward by `NOAKCBPMNDD(pet)` (Blacksmith+Meteor, Alchemist, and
  None+Panda have their own cases). **Blocked here:** `NOAKCBPMNDD`'s constants are
  obfuscator decoys (garbage like 1276.0/1954.0/1632.0), so the exact Rogue
  multiplier value can't be read off the decompile, and `KEPCFJNJDPJ` has no clear
  external caller in the dump (campaign-vs-dungeon reward path unconfirmed). So the
  chamber sim's `adv_xp_mult` stays the empirical 4.0 for now.
- **Need from user (cheap, in-game):** run a **Growth campaign** with an
  otherwise-identical pet **as Rogue vs another class** and compare the class-XP /
  growth gained — that directly gives the Rogue multiplier and tells us whether it
  IS the chamber's mystery ×2. (Or note the in-game per-class campaign-bonus % on
  a Rogue pet's tooltip.)

### 4. Remaining unidentified save blocks  — MEDIUM/large
- **What:** large unmapped regions still in the root/sub-structs — root scalars
  `b`/`d` (BigDouble), `l`/`m`/`n` (int, =50/50/50, stat-cap divisors of unclear
  meaning), `U` (long), `q`/`r`/`s`/`t`/`u` (struct sub-blocks not yet opened),
  plus any block not yet in `FINDINGS.md`'s root map.
- **Gain:** the "entire save file" goal.
- **Status:** open each unnamed root sub-block class via the C# deserializer and
  map it (same method as the pet/equipment work). Exploratory but tractable.
- **Need from user:** for value-meaning disambiguation, occasional targeted
  before/after Steam saves.

### 5. Pet growth components `d`/`e`/`f` source  — LOW
- **What:** the additive stored growth components (total = E+d+e+f); 0 for every
  pet in current saves.
- **Need from user:** a Steam save with a pet that has **nonzero `d`/`e`/`f`**
  (e.g. right after an event/item growth grant) to correlate the source.

---

## How to provide a save (reminder)
Steam saves only; redact before sharing/committing per CLAUDE.md
(`save-dump --redact`). For before/after probes, capture both, note the exact
in-game numbers that changed, and what action caused the change.
