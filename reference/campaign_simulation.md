# Campaign simulation — formulas, mechanics & roadmap

Human-reference notes (not read by code). This is the knowledge base for the
**campaign-planner / simulator** stretch goal: given a team of up to 10 pets
assigned to a campaign, estimate the per-hour reward. It complements
`campaign_bonus_design.md` (which covers the *per-pet campaign multiplier* — the
number we already compute via `MergedPet::campaign_bonuses`). That multiplier is
**one input** to every formula below.

> **Faithful-transcription notice.** The formulas and worked examples here are
> recorded verbatim from in-game text / the player. Two of them do **not** cleanly
> reproduce their worked example with a naive reading (the stats ramp and
> Pandora's Box — flagged inline). Those are *open questions to pin down at
> implementation time*, not settled math. Don't treat them as verified.

---

## 1. The three kinds of pet levels

A pet has **three independent level tracks**. We have only ever modelled the
first two; the third is what several campaigns actually care about.

| Level | Earned by | Resets on rebirth? | In the export? |
|-------|-----------|--------------------|----------------|
| **Dungeon level** | Free-experience pool (mainly from running dungeons). | No (permanent) | ✅ `dungeon_level` |
| **Class level** | Running dungeons *if the class earns class XP there*; see below. | No (permanent) | ✅ `class_level` |
| **Normal level** | **Training** (allocating shadow clones — the pet fights them for XP) **or** the **Level campaign**. | **Yes (resets)** | ❌ **not exported** |

Dungeon XP applies **equally to dungeon and class level** for dungeon-compatible
classes. Class-XP exceptions:

- **Adventurer** — gets class XP from **running campaigns** (not dungeons).
- **Alchemist** — gets class XP from **alchemy crafting** (not dungeons).
- **Blacksmith** — works in dungeons, so earns class XP from **both** crafting
  **and** dungeons.
- A pet in a **Pet Village** slot also earns class XP.

(The class-XP source matters for *how* a pet gets levelled, not for the campaign
math directly — recorded for completeness.)

### Why the "normal level" gap is the big blocker

Normal level drives **pet stats** (next section), and stats drive several
campaign rewards. **Normal level is not in the pet export.** Until we can read
it, the stat-dependent campaigns can't be computed.

- **Deferred option:** revisit the full **save file** (richer than the export)
  to recover normal level (and possibly the strategy-room split). Explicitly
  deferred for now.

---

## 2. The stats system

Every pet has three stats: **Physical**, **Mystic**, **Battle**. Their sum is
the pet's **total stats**, shown on mouseover. Stats scale with **growth × normal
level**: higher growth ⇒ more stats per level. Pet stats feed the player's own
god multipliers/stats, and several campaigns key off them.

### Stated formula (verbatim)

> The stats of a pet are: `growth + (Level - 1) * 0.1 * growth / 3`. Every 100
> levels the increase rises `0.1 * growth` up to a total increase of
> `1 * growth / 3`. The strategy room in the pet village increases this and
> changes the split into the 3 stats.

Reading: the per-level increment starts at `0.1 * growth / 3` and ramps up every
100 levels, capping at a per-level increment of `1 * growth / 3` (coefficient
`0.1 → 1.0`, cap nominally at level ~1000). The **Strategy Room** then scales the
result and sets the **split** across the three stats (a user input — see §3).

### Worked example — Gnome (use as the regression target)

- Normal **Level 13,724**, **growth 72,266**.
- **Total stats: 3.538e9** — Physical **1.186e9**, Mystic **1.175e9**, Battle
  **1.175e9**.
- Strategy Room set to **104% Physical / 103% Mystic / 103% Battle** (the player
  only had +10% to allocate so far). This explains Physical > the others.

> ⚠️ **Open question (must resolve before implementing stats).** The stated
> formula, read naively (increment caps at `growth/3` per level ≈ 24,089, summed
> over ~13.7k levels), yields ≈ 3.3e8 total — about **10× short** of the observed
> 3.538e9. So either the `/3` is the 3-stat split (not part of the per-stat
> value), the coefficient caps higher than `1.0`, or another multiplier (rebirth/
> pet multiplier) applies to stats. The Gnome numbers above are the data point to
> fit against. **Do not ship a stats formula that doesn't reproduce them.**

---

## 3. Inputs common to the campaign formulas

Every campaign reward is, schematically:

```
reward = Σ_pets [ base_term(pet) ] · UPC_multi · pet_campaign_multi(pet) · hours
```

with per-campaign twists. Shared inputs:

- **`hours`** — the simulation window (or "per hour" with `hours = 1`).
- **`UPC multi`** — a global multiplier. **User input** (not derivable).
- **`pet_campaign_multi`** — the per-pet, per-campaign multiplier we already
  compute (`campaign_bonuses[campaign]`). ⚠️ *Confirm the form:* our value is a
  percentage like `+82`. As a multiplier that's presumably `(1 + bonus/100)`
  (e.g. `1.82`); verify against in-game before wiring.
- **Max 10 pets** per campaign.
- **`div gained per sec`** — **user input**, Divinity only. *Excludes* some
  boosts (crystals, worker clones, FSM).
- **Strategy-room split** (Physical/Mystic/Battle %) — **user input**, needed for
  stats.
- **Pet stats** — blocked on normal level (§1–2).

---

## 4. Per-campaign formulas

Legend: ✅ computable with current data + a couple of user inputs · ❌ blocked on
stats · ◐ partially computable.

### Growth ✅
Increases the **growth of the weakest participating pet**. The gain rises with
the *other* pets' growth.

```
growth gain (per contributing pet) = (log15(growth) - 1.75) · UPC · pet_growth_multi · hours
```

- The **weakest pet** (lowest growth) is the **recipient** and **does not
  contribute** its own term.
- No stats. Needs only growth (have it), `UPC` (input), `pet_growth_multi`, hours.
- See **Bag** and **Pandora's Box** in §5 for modifiers.

### Food ✅ (needs per-pet food-tier data)
Stats don't matter; **more unlocked pets ⇒ more food**.

```
chance for food (per pet) = food_tier_base · UPC · pet_food_multi · hours
food_tier_base:  mighty = 0.09,  strong = 0.3,  puny = 0.6
```

- **Each unlocked pet increases dropped food by 1%.**
- Needs a **per-pet food tier** classification (mighty/strong/puny) — not yet a
  structured field; sits near `pet_special_info.yaml` territory.

### Multiplier ◐ (rebirth part ✅, pet-multi part ❌) — *does not work in UAC*
Two outputs:

```
pet multiplier gained     = (stats < 10m ? stats^0.30·0.016 + 1
                                          : stats^0.25·0.018 + 2) · UPC · pet_multi_multi · hours / 5
rebirth multiplier gained = (log2(growth) - 7) · UPC · pet_multi_multi · hours / 2
```

- **Rebirth multiplier** needs only growth → ✅ computable now.
- **Pet multiplier** needs stats → ❌.

### Divinity ❌ (stats)
The higher the pets' stats, the more divinity.

```
div gain (per pet) = f(stats) · div_per_sec · UPC · pet_div_multi · hours
f(stats):  10k < stats < 10m :  (stats^0.4 + 300) · 2
           stats > 10m       :   stats^0.35 + 1580
```

- Needs **stats** and the **`div_per_sec`** user input (excludes crystals/worker
  clones/FSM boosts).

### Item ❌ (stats)
Search for **pet stones** (+ rare-item chance: godly liquid, chakra pill, lucky
draw). Pet stones found per pet are **tier-based on stats**:

```
stones (per pet) = tier(stats) · UPC · pet_item_multi · hours
tier: base 1; +1 after each of 10k, 100k, 500k, 1m, 5m, 10m, 50m stats;
      +2 after 100m stats; capped at 10.
```

### Level ◐ (stats term) 
Gain **normal levels**. Higher growth **and** stats ⇒ more.

```
levels gained = (log10(growth)^3 · 1.9 + stats^0.22 · 3) · UPC · pet_level_multi · hours
```

- The `growth` term is computable; the `stats` term is blocked → treat as ❌
  until stats land (the two terms are additive, so a "growth-only floor" is
  possible but misleading).

### God Power ❌ (stats)
Chance to find one or more god powers; higher **total** stats ⇒ higher chance.

```
gp found (per pet) = tier(stats) · UPC · pet_gp_multi · hours
tier: base 3%, rising with "tiered linearity" up to 12% at 100m stats.
```

- **One pet finds at most 1 GP**, *unless* **Nightmare** (always) or **Ant Queen**
  (**only once evolved**) is in the campaign — they **uncap** it.
- **FSM doubles** the GP gain — applied to the **total**: sum every pet's
  contribution, *then* double.

---

## 5. Special pet mechanics

### Llysnafedda — shadow-clone exp sharing (affects training / normal level)
> For each shadow clone defeated (and devoured), it gives exp to other pets. 50%
> exp goes to itself and 50% is shared among other pets and multiplied with
> `(1 + CL · 0.19)` (1000% at CL 100). At CL 18 → shares **221%** exp of defeated
> clones with other pets.

- Check: `1 + 18·0.19 = 4.42`; `0.5 · 4.42 = 2.21 = 221%`. ✔ (the 50% shared half
  is scaled by `1 + CL·0.19`).
- This accelerates **normal-level** gain during training, i.e. it feeds the stats
  system — relevant once we model levelling/stats, not a direct campaign term.

### Bag — Growth campaign only
- **Pre-token-improve:** takes **10% of the total** campaign growth and gives it
  to the lowest-growth pet. This 10% is **stolen from the campaign** — the normal
  recipient does **not** receive it.
- **Post-token-improve:** reduced to **5%**, but **not stolen** — the normal
  recipient keeps the full amount **and** the weakest pet gets the extra 5% as
  free/bonus growth.

### Pandora's Box — flat % bonus on the whole campaign total
> Increases all campaign rewards for campaigns it participates in by **3% per
> 5,000 growth**, capping at **100k growth**. Reaching the cap raises it to **4%
> per 5k**. Plus **+0.1% per feeding**, up to **+2% additional** (resets on
> rebirth or when hunger reaches 0).

- **Applied on top of the total** campaign result (compute the campaign sum, then
  add Pandora's %). Affects **every** campaign it joins.
- **Worked example:** 8 feedings, **57,114** growth → **+43.41%**.
  > ⚠️ Doesn't reconcile with a naive reading: `(57,114 / 5,000)·3% = 34.27%`,
  > `+ 8·0.1% = 35.07%` — not 43.41%. The per-5k rate that fits is ≈ 3.8%, or the
  > growth figure feeding the formula differs. **Pin down before implementing.**

### Nightmare / Ant Queen — God Power uncap
- **Nightmare:** **always** uncaps the GP campaign (pets can find >1 GP).
- **Ant Queen:** uncaps the GP campaign **only after she's evolved**.

### FSM — God Power doubling & Divinity exclusion
- **Doubles** total GP gain (sum all contributions, then ×2).
- Its boost is **excluded** from Divinity's `div_per_sec` (along with crystals /
  worker clones).

---

## 6. Computable-now vs blocked

| Campaign | Needs stats? | Status | Missing inputs/data |
|----------|-------------|--------|---------------------|
| **Growth** | No | ✅ | `UPC`, hours; Bag/Pandora layer |
| **Food** | No | ✅ | `UPC`, hours; per-pet food tier |
| **Multiplier (rebirth)** | No | ✅ | `UPC`, hours; *(no UAC)* |
| **Multiplier (pet)** | Yes | ❌ | stats |
| **Level** | Partly | ◐ | stats term |
| **Divinity** | Yes | ❌ | stats, `div_per_sec` |
| **Item** | Yes | ❌ | stats |
| **God Power** | Yes | ❌ | stats; Nightmare/AntQueen/FSM layer |

---

## 7. Proposed implementation roadmap

Staged so the stat-blocked work is cleanly isolated behind a black box.

1. **Team + inputs model.** A campaign-team selection (≤10 pets, excluding
   dungeon-allocated pets), plus the shared inputs (`UPC`, `hours`, and a
   `stats(pet) -> Option<f64>` black box that returns `None` until §1–2 is
   solved). Persist like `CampaignInputs`.
2. **Stat-independent campaigns.** Implement **Growth**, **Food**, and the
   **rebirth-multiplier** part — fully computable. Apply the **Pandora's Box**
   flat-% layer and the **Bag** growth-campaign steal/gift. Surface a per-campaign
   per-hour estimate in the UI.
3. **Unblock stats.** Recover **normal level** (save-file parse) + accept the
   **strategy-room split**; implement the stats formula *against the Gnome
   regression target*. Then light up Divinity, Item, Level (stats term),
   Multiplier (pet part), God Power — with the **Nightmare/Ant Queen uncap** and
   **FSM double** layers.
4. **Simulator extras** (longer horizon): Llysnafedda-accelerated levelling,
   Nightmare's team malus (already noted in `campaign_bonus_design.md`), and
   multi-campaign allocation/prioritisation.

## 8. Open questions to resolve

- **Stats ramp** vs the Gnome example (≈10× gap) — §2.
- **Pandora's Box** % vs its example (34% vs 43.41%) — §5.
- **`pet_campaign_multi` form** — is it `(1 + bonus/100)` or the raw `bonus`? — §3.
- **Per-pet food tiers** (mighty/strong/puny) — data source / curation.
- **Tier "tiered linearity"** for GP (exact interpolation 3%→12% to 100m) — §4.
