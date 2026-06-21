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
- **Status (updated 2026-06-20):** **per-challenge completion counts SOLVED +
  VALIDATED + WIRED.** They are a single list at `root.x.242` (`LLMCMCKAABP`
  `n242` → list of `KPLPGPEOFNB`): each element `a`=challenge id (`OIDDHCOBPLG`,
  77 values), `b`=completion count, `c`=difficulty, `d`=last-completed ms,
  `e`=flag. Validated against an in-game Steam capture (all 11 of the player's
  counts match: AAC=10, MMC=13, PLC=12, DRC/UPC=8, CPC/GPC/UUC=2, BHC/PMC/UBC=1)
  and wired into the editor as the **Challenge** block (id→name via
  `items::challenge_name`). **Self-correct:** the earlier "one scalar per
  challenge in `root.x` via `HAKKBJKMHFN`, candidate map `x.H`/`x.002`/…" was
  wrong (those keys are 0 in the capture) and is retracted.
- **ChP / Overflow Points — mechanism SOLVED 2026-06-20 (NOT user-gated after
  all).** ChP **total is a derived sum** over the completion list
  (`LLMCMCKAABP.NFHDEHOCFMP`, `Σ OIHGOPGKAJO`, floored) — no stored integer;
  computable. **Overflow Points "left" is a derived method**
  (`LLMCMCKAABP.JHDPFECPEOM()`). The **Overflow-Point upgrade levels are stored
  at `root.013`** (`HNFHEBJIPEL`, keys `a`–`n`; the `098` assignment is a decoy)
  and are now **WIRED** as the "Overflow Point Upgrades" block (13 of 14 fields
  labeled from the in-game ChP debug tooltip; `h` vestigial). See FINDINGS for
  the field map. The **ChP upgrade levels** (the ~22 the tooltip names) are
  scalar fields on `root.X` (`MLILKGIALMB`) — now **MAPPED + WIRED** as the
  "Challenge Point Upgrades" block (keys + labels in FINDINGS). ChP *spent* =
  Σ(level × cost) per `MLILKGIALMB.cs:894`.
- **Remaining (optional polish, C#-tractable):** compute + display the derived
  ChP total (Σ over `x.242`), ChP spent (Σ level×cost), and Overflow-Points-left
  (`JHDPFECPEOM()`) as read-outs in the editor; label the few non-display ChP
  upgrades (`FJBKFKPMHGB`/`MPPMMMAOKIC`/`LEODLEACJPA`) if an anchor turns up.
- **Need from user:** nothing required. A single in-game ChP-total / ChP-spent
  number would be a nice cross-check once the derived read-outs are added.

### 2. Adventure mode (skills, classes, exp, stats, crafting/smithing/alchemy)  — HIGH
- **What:** the full Adventure-mode subsystem (`root.032`): adventurer skills,
  classes, experience, stats, and the crafting/smithing/alchemy progression.
- **Gain:** a whole secondary game mode currently only partly mapped (researches,
  inventory `032.d`, cores `032.G` done).
- **Status:** **structurally complete (2026-06-19).** All `root.032` sub-blocks
  decoded: the single adventurer (`032.b`, class/level/exp/skills wired into the
  save-editor), and every list key labeled — `c` adventure equipment, `d`
  inventory, `j` AdvSkill, `m` AdvPotion, `G` cores, `L` MonsterCoreProgress, `R`
  bestiary battle-stats, `U` AdventureArea. Content enums (class/skill/enemy/
  material/craft-gear/recipe) are in `items.rs`, diff-verified vs C#. Only `z` =
  `NKAIFFEMNBO` (not string-deserialized) and a couple bool flags remain unnamed
  — per-instance state the planner doesn't model.
- **Need from user:** nothing required. An Adventure-mode stats screenshot would
  let us cross-check *derived* stats if we later model them, but the structure is
  done.

### 3. Growth Chamber unexplained ×2 (the "rogue"/stray multiplier)  — MEDIUM
- **What:** the chamber sim's `adv_xp_mult = 4.0` = (Camp Exp Boost ×2) × an
  **unexplained ×2** (`campaign.rs` / `growth_chamber_status.md`). "Rogue" was the
  user's word for *rogue/unexplained*, **not** the Rogue class.
- **Important (user-confirmed 2026-06-19):** **only the Adventurer class gives a
  campaign bonus**; Rogue (and the other classes) do **not**. Pets' own innate
  per-campaign bonuses are already accounted for in the chamber logic. So the
  mystery ×2 is **not** a class bonus.
- **Correction:** an earlier pass wrongly tied this to a "per-class campaign
  multiplier" in `CIEAPBPBCLL.KEPCFJNJDPJ`/`NOAKCBPMNDD`. That method is a
  **dungeon** calc — it switches on the pet's *element* and combines elemental
  stats; the Rogue branch there is its **dungeon-loot** bonus (per the Rogue
  tooltip), not a campaign reward. Disregard that lead.
- **Status:** the ×2 is still genuinely unidentified. Per `growth_chamber_status.md`
  the candidates are a 2nd (non-Camp-Exp-Boost) upgrade, a research/god/Patreon
  perk, or the wiki `250` base being outdated — i.e. a global campaign-XP
  multiplier, not class- or pet-specific.
- **Need from user (cheap, in-game):** the actual in-game per-cycle Adventurer
  class-XP for one Growth campaign at a known total growth + hours (and which
  Camp-Exp-Boost level is active) — back-solving `XP / (250·(1+growth/20000)·hours)`
  pins the true multiplier and reveals whether the wiki `250`/×2 is the gap.

### 4. Remaining unidentified root scalars  — LOW/medium
- **What:** the leftover unnamed root fields. Types now pinned (all **scalars**,
  not struct sub-blocks — corrected): `b`/`d` = BigDouble; `l`/`m`/`n` = int
  (default 1000, save 50/50/50; **Shadow-clone stat divisors — now pinned
  precisely:** `l`=Attack, `m`=Defense, `n`=HP, i.e. clone stat = your stat ÷
  divisor; lower = stronger clones); `q`/`t`/`u` = bool; `r`/`s` = string
  (**both PII account ids — now pinned + redacted**, see below);
  `U` = long = **"rested" tripled-crafting-speed timer** (now pinned).
- **Gain:** the "entire save file" goal (the big sub-blocks are all already mapped).
- **Status:** `l`/`m`/`n`, `U`, `r`, `s` are **done**. `l`/`m`/`n` = clone
  Attack/Defense/HP divisors (clone tooltip + property getters). `U`
  (`LIKCHFOLHKI`, ms) = the rested-bonus timer: while >0 crafting speed is
  tripled; banks 1/6 of offline time capped at 3 h on return, drains during
  active play (`OfflineCalc.cs:277`, `UpdateStats.cs:5053`). **`r`
  (`OBBCNEEELEN`) = Kongregate user id (`KongUserId`) and `s` (`DJJMJOHIHPO`) =
  account UserName/KongName — both PII; `r` had been missed by redaction and
  leaked into 9 committed saves, now fixed (added to `redact::IDENTITY_FIELDS`,
  saves re-redacted). The leaked value still lives in git HISTORY → a history
  rewrite/force-push is still needed to fully purge (user-gated, flagged).** The
  remaining scalars: **`d` (`DILEDALONFM`, BigDouble) = vestigial** (only in the
  (de)serializer, never read — like the pet `z`/`D` flags); **`b`
  (`GDIIPNIODAD`, BigDouble) = a load-time pending value** (transferred to a root
  `CCIDPLFLDIA` via the `ENLNNIPOEBB` recompute flag, then zeroed; player meaning
  unclear — left unnamed). Only the bools `q`/`t`/`u` still want a per-field
  chase; low value individually.
- **CORRECTION (2026-06-20): struct sub-blocks DO remain unopened at root** — the
  earlier "none remain" was wrong (it only covered the *scalars*). A coverage-gap
  sweep of the fixture's root keys vs. the editor registry found **~10 unmapped
  root structs + 4 unmapped lists** (see "Root-level coverage gap" in FINDINGS):
  e.g. `root.029` = Ultimate Overflow Point upgrades (marker `UltimateOverflowBoosts`,
  sibling of the now-wired `013`), `root.O` = Events, `root.P` = PBaal,
  `root.Q` = Avatar options, `root.012` = UBV4 battle state, `root.014` = RTI
  bonuses, `root.027` = Multiverse, and `root.y`/`z`/`A`/`B` = Achievement-state
  lists (`OEFPEEEHKDB`, `{a:bool, b:int}`). These are the real remaining
  C#-tractable mapping targets — each its own subsystem/PR.
- **Need from user:** occasional targeted before/after Steam saves to pin a
  specific scalar's meaning if one matters.

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
