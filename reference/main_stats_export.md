# The "Main stats" export — fields & uses

Human-reference notes (not read by code). The game has a second clipboard export
(button in-game) beyond the pet export we already parse: **"Main stats"**. A full
real sample lives next to this file as **`Main Stats Export.txt`**. It's plain
`Label: value` lines under blank-line-separated section headers. It carries a lot
we currently ask the user to type, plus data for features we haven't built.

**Scope note (deferred):** *parsing* this export is its own task — **not** done on
the docs branch. This file is the spec for that future importer. The richest
missing piece — pet **normal level / stats** (see `campaign_simulation.md` §1–2)
— is **still not here**; that needs a full **save-file** parse. Everything below
is what the Main-stats export *can* give us in the meantime.

Design stance: **auto-fill, don't lock out.** Where a value appears here, populate
the corresponding input automatically but keep the manual field editable (some
values change frequently; some users won't import).

---

## 1. Auto-fill for the campaign inputs we already added

These map straight onto existing `CampaignInputs` fields:

| Export line | Field | Pet |
|-------------|-------|-----|
| `Pet Stones: 250,882` | `pet_stones` | Beachball |
| `Ants: 187,331` | `ants` | Ant Queen |
| `Honey consumed by Bear: 0` | `honey` | **Bear** |
| `Challenge Points: 721` | `challenge_points` | Unicorn |
| `Earth Eater Earthlike planets eaten: 7.142 E+6` | `earth_eater_total_planets` | Earth Eater |
| `Overflow Challenges: 0 / 9,999` | `goblin_oc` | Goblin (evo) |
| `Ultimate Challenge Challenges: 0 / 67` | `goblin_ucc` | Goblin (campaign) |
| `Chp Stone Pet improvement: False` | `stone_campaign_upgrade` (bool) | Stone/Golem |

⚠️ **Two different "honeys."** `Honey consumed by Bear` is the **Bear** input.
The separate bottom-of-list `Honey: 765` is **Bee's** crafting stat (§3) — do not
confuse them.

**Not in this export (keep manual):** `beachball_given_stones` (Beachball
mouseover only), `couples` (Cupid), `delirious_essence_fights` (Aether Ring).

**Cap note:** Goblin's UCC bonus caps at 75 in our formula, but the challenge
itself shows `/ 67` max here — reconcile when wiring (maybe more unlock later).

### UPC multiplier — now derivable
`Ultimate Pet Challenges: 8 / 20` → the campaign **UPC multi** = `5% × completed`
(cap +100%): `8 → +40%`. See `campaign_simulation.md` §3. Auto-fill, allow
override.

### Afky Clone — now derivable
`Highest Afky God Power: 1.600 E+9` drives **Afky Clone's** campaign bonus
(formula in `campaign_bonus_design.md`). Auto-fill the "highest afky power" input.
(`Afky exp multi: 1,000.32` is a *separate* global multiplier — not this.)

### Museum bits
- `Pet campaigns: 0%` (Museum additive) — a **global** add to every pet-campaign
  reward; another input to the simulator.
- `Base Growth per hour: 2` — this **is the Moai statues** (each maxed statue =
  +1/hr). A value of **2 unambiguously means both statues owned at level 20**, so
  we can auto-populate the two Moai in that case. Other values (partial levels)
  can't be uniquely decomposed → fall back to manual.

---

## 2. Global aggregates (we already compute these)

`Unlocked Pets: 104`, `Evolved Pets: 80`, `Total Pet Dungeon Levels: 2,898` (top
50), `Total Pet growth: 3.290 E+6`. We derive all of these from the **pet**
export already, so only worth reading here as a **fallback when the pet import is
absent**. The one genuinely useful entry in this block is `Pet Stones` (above) —
it changes often, so importing beats re-typing.

---

## 3. Per-pet trackers (data for future per-pet mechanics)

Mostly relevant to **crafting bonuses** and **evolution requirements** — features
beyond campaigns. Logged so the importer knows what's here.

| Export line | Pet | Use |
|-------------|-----|-----|
| `Growth from Vesuvius: 0` | Vesuvius | Evolution-requirement tracker. |
| `Crystals for Dwarf: 0` | Dwarf | 3rd evo req (100× lvl-13 crystals — the game's most expensive req, usually token-skipped). Also feeds his crafting bonus. |
| `T3/T4/T5 gear bonus for Elf: 133/23/0 (max 2000 count)` | Elf | Crafting **speed + quality**: `0.1%/0.2%/0.3%` per T3/T4/T5 item, **additive with her Blacksmith class bonus**, capped at **2000** pieces total. (Matches her in-game 17.9% vs 10% evo threshold.) |
| `Acorns: 24,157` | Squirrel | Loot. Token-improved, he finds acorns in dungeons; drop-bonus multiplier `Acorns^0.4 / 10` (%). |
| `Honey: 765` | **Bee** | **Current honey held** (just an inventory count — *not* "consumed by Bear"). Feeds Bee's crafting-speed multiplier `1 + (honey^0.5)/100`, capped at **4×**. (Full crafting speed also needs his equipment + class. ⚠️ confirm whether Bee uses *held* or *lifetime* honey.) |
| `Ants: 187,331` *(also)* | **Anteater** | If Blacksmith, crafting **speed + quality** rise by `(ants_found / 50,000)%` — **additive**, *not* multiplicative with other bonuses. The *additional* boost **halves after 1M** ants and is raised to **^0.28 after 100M**. (Sample: `187,331 / 50,000 = 3.75%`, matching the in-game tip.) ⚠️ The export's `Ants` is the **found** count (matches Anteater's tip); confirm it's the same quantity Ant Queen's "held" input wants. |
| `Vampire Blood Potions consumed: 0` | Vampire | Growth mechanic — see §4. |
| `Chocobear hours: 4,734` | Chocobear | Drives his **campaign bonus** (hours-based) — see `campaign_bonus_design.md`. |
| `Caterpillar materials upgraded: 2,835` | Caterpillar | **2nd evolution**: at **40,000** materials upgraded he evolves cocoon → butterfly. |
| `Octopus Hammers: 0` | Octopus | Token upgrade — give hammers to boost his crafting speed/quality. |

---

## 4. Evolution-requirement data (non-growth)

Today evo-readiness is judged **solely on growth**. Several pets actually evolve /
unlock on **other** counters that this export exposes — a chance to make readiness
multi-criteria:

- **Valkyrie** — evolves at **2,000 bosses defeated**. `Dungeon Bosses defeated:
  2,213` → already met. *(Easy first candidate for non-growth readiness.)*
- **Wizard** — **unlocks** at **5,000 bosses defeated**.
- **Caterpillar** — 2nd evo at **40,000** `Caterpillar materials upgraded`.
- **Dwarf** — 3rd req is **100 lvl-13 crystals** (`Crystals for Dwarf`); brutal,
  usually token-skipped.
- **Anteater** — has an **ant-count** evolution requirement (`Ants`, the found
  count).
- **Pandora's Box** — 3rd req: **defeat P. Baal v66 in a Day Baal Challenge**, and
  it **cannot be token-skipped**. ⚠️ **Not extractable** — there's no "highest god
  in Day Baal Challenge" line in this export, so this stays a **user-only input**.
- **Vampire** — needs **1,000 Monster Blood**, earned by landing the **killing
  blow** in dungeons. ⚠️ **Monster Blood is *not* in this export** — only
  `Vampire Blood Potions consumed` is, which is a *different* thing (§ below).

`Dungeon Rooms beaten: 34,544` and `Dungeon Enemies defeated: 259,006` are
general progress counters, possibly useful for other thresholds.

### Vampire — full mechanic (for the record)
- **Monster Blood**: dropped on Vampire's killing blow in a dungeon. **1,000**
  needed to evolve. Not tracked in the export.
- **Blood Potions**: crafted from **6 Monster Blood + 1 Sun Stone**. Consuming
  them grants **100 base growth** (or **150 total growth** if all PGCs are done),
  up to a cap of **Dungeon Level × 10**. Pressing the button consumes **all**
  available potions at once.
- Strategy notes: consume immediately to raise her crafting-speed bonus, or
  stockpile for fast TGC runs. To avoid auto-consumption you can park potions as a
  dungeon-team **item**. If using her in dungeons, wait until she's at your Growth
  campaign's **floor** before consuming, to push her temporarily above the limit
  (cf. Carno ignoring its evo limit, Succubus' raised limit).

---

## 5. Challenges → UPC & Goblin (and more)

The `Challenges` section lists `completed / max` per challenge family. Relevant:

- `Ultimate Pet Challenges: 8 / 20` → **UPC multi** (§1).
- `Overflow Challenges: 0 / 9,999` → **Goblin** evo bonus (`goblin_oc`).
- `Ultimate Challenge Challenges: 0 / 67` → **Goblin** campaign (`goblin_ucc`).
- Many others (Monument Multi, Pet Level, Double Rebirth, …) may matter for future
  pet mechanics or global multipliers; not needed yet.
- `Day Pet Challenge highest multi: 3.664 E+9` — a multiplier to **how much
  growth food gives your pets**. We don't model growth-from-food, so it's not
  critical — but if the growth-chamber sim ever tracks **feedings** (for Pandora's
  Box's bonus and Chocobear's hour-bank), this is where the food-growth payoff
  would factor in. Park it.

---

## 6. Parsing notes & caveats

- **Number formats vary.** Plain (`250,882`), and **`X.XXX E+N`** with a *space*
  before a capital `E` and a `+` (e.g. `7.142 E+6`, `1.600 E+9`). Our
  `parse_flexible_number` strips whitespace and `f64` accepts `7.142E+6`, so this
  parses today — but add it to the parser's test cases when we wire the importer
  (it's the exact form the player flagged as "different from what I prefer").
- **Strategy Room does *not* expose the stat split.** Its section lists
  Health/Attack/Defense/Speed/elements (dungeon-oriented) and `4th lowest growth
  pet: Portal (11,377)`, but **not** the Physical/Mystic/Battle allocation the
  stats formula needs (`campaign_simulation.md` §2). That split stays a manual
  input (or awaits the save parse).
- **Dojo** is all `0%` here and gated behind CL 50 (player's top CL ≈ 28–29), so
  it's a non-factor now; revisit when relevant.
- `Chp Stone Pet improvement: False` **is** the **Stone/Golem** 1500-CP campaign
  upgrade (≈confirmed) — auto-fill `stone_campaign_upgrade` from it (§1).
- **Still blocked:** pet **normal level / stats** — not in this export. Full
  **save-file** parse remains the path for the stat-dependent campaigns.

---

## 7. Suggested importer phasing

1. Parse the campaign-input auto-fills (§1) + UPC + Afky power + Moai-from-Museum —
   immediate value, all feed features that already exist or are next up.
2. Surface the per-pet trackers (§3) and Museum `Pet campaigns` global add as the
   relevant pet features land.
3. Make **evo-readiness multi-criteria** (§4), starting with **Valkyrie** (bosses
   defeated) as the cleanest non-growth example.
4. (Separate, hard) Full **save-file** parse for normal level / stats.
