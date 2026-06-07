# Campaign-bonus feature — design & roadmap

Human-reference notes (not read by code). Captures how the campaign-bonus
feature is structured so later phases don't become a messy refactor.

## The seam (anti-refactor guarantee)

All UI (pet card, "Campaign boost" filter, "Sort by bonus") goes through **one**
method and never reads `WikiPet.campaign_bonus.per_campaign` directly:

```
MergedPet::campaign_bonuses() -> BTreeMap<CampaignType, f32>   // crates/planner/src/merge.rs
MergedPet::campaign_bonus_for(c) -> Option<f32>
```

Today it returns the **static parsed baseline** scraped from the wiki. Dynamic
and conditional adjustments get layered in *inside this method* (it will likely
gain a `&CampaignContext` argument). Because callers go through the seam, adding
those adjustments won't disturb the data model, the filter/sort, or the card —
filter/sort just start reflecting better numbers.

## Data sources, by pet kind

1. **Static** (≈68 pets) — parsed at scrape time into `per_campaign`
   (`wiki_pets.yaml`). Done (PR #18). Tuned against `campaign_bonus_survey.tsv`.
2. **Raw-only** (≈61 pets) — prose/dynamic/conditional the parser bails on. The
   cleaned `raw` string still displays. These get upgraded by the phases below.
3. **No bonus** (≈27 pets) — no `CampaignBonus`.

## Dynamic/conditional layer (future phases)

Two homes, chosen by what each pet actually needs:

- **Declarative overrides → a curated `data/campaign_overrides.yaml`** (hand-
  authored, like `pet_special_info.yaml`). This is where values the wiki infobox
  *doesn't* state get defined. Shape: condition + operation, e.g.
  - **Hedgehog** — `on_token_improved: set/add {Growth, Divinity}` (the big
    token boost; applied when `export.improved`). *The infobox only gives the
    +25/+25 base — the boost is defined here.*
  - **Lizard** — `on_evolved: swap Growth -> Food` (applied when evolved).
  - **Cat / prose pets** — `set {GodPower: 50}` corrections (curated; the wiki
    text is structurally identical to special pets like Doughnut, so they can
    only be told apart by hand).
  - **Per-form elementals / two-state pets** — Undine `-111 -> +111`,
    Nothing/Corona pre/post-evo: `on_evolved: set {...}`.
- **True formulas → code functions in the planner**, keyed by pet name, taking a
  `CampaignContext`:
  - **Bag** = `lowest *unlocked* pet's growth ^ 0.4` (from roster).
  - **Mermaid** = `-growth / 1000` (own export growth, capped).
  - **Lizard magnitude** = `(unlocked + evolved) ^ 0.5 * 10`.
  - **Beachball / Unicorn / Bear** = `sqrt(stones^1.00001 − stones)·2` (cap 200),
    `sqrt(challenge points)/2` (cap 100), `honey/500` (cap 100) — from
    **user-input fields** persisted in `AppState`.
  - **Cupid couples** = token-improved Cupid's per-couple bonus. The game only
    reports "current couples" (a pet can be coupled with itself, so couples ≠
    pets-in-couples). Compromise: a "current couples" user input, **+2% per
    couple** to all campaigns, on top of the flat token bonus already curated.
  - **Aether** = a user-input for "Delirious Essence fights completed" drives
    both (a) the all-campaign penalty: `-99%` reduced by `10%` per fight, maxing
    at `+1%` after 10 fights; and (b) an *added* growth-campaign bonus:
    `((elementals_unlocked + 5) / 10) * fights * (1 + 0.57 * log_1000(growth))`%,
    where growth clamps to 1 if non-positive and `elementals_unlocked` counts
    Undine/Gnome/Salamander/Sylph/Elemental **and Aether itself** (per the wiki
    source). `fights` = the Aether Ring's number (it starts at +0, so the number
    is the exact kill count).

`CampaignContext` carries the persisted user inputs + a roster reference; the
pet's own evolved/improved/growth state comes from its export.

## Elementals

Gnome/Salamander/Sylph/Undine now use a declarative **2-state approximation**
(curated): unevolved → worst (base) stage, evolved → best (final) stage, since
evolving requires full upgrades. Their intermediate quest stages are recorded as
yaml comments but can't yet be told apart from the export — revisit once we can
read a pet's upgrade stage (watch Undine's export columns when unlocking it).
Aether is Phase 3 (formula above). "Elemental" (the pet) is a flat +150 already.

## Phase status

- [x] **1a** — scrape + conservative parser + data (PR #18).
- [x] **1b** — card display, "Campaign boost" filter, "Sort by bonus".
- [~] **2a** — override mechanism (`campaign_overrides.yaml` + `CampaignContext`
      seam) with a focused high-confidence curated set (Hedgehog token, Nothing/
      Corona evo flips, the clear "more levels / divinity / god power" prose
      corrections). Set-all / set / add ops × Always / Evolved / Unevolved /
      TokenImproved conditions.
- [x] **2b** — expanded curation: Cupid token, Holy ITRTG Book / Fainting Capra
      (campaign clauses split from non-campaign prose), Nightmare, Slime, Clam
      (Item only), Baby Carno (pre/post-evo); the food/item pets confirmed
      in-game (Mouse evo-flip, Robot, Squirrel, Goat, Octopus, Mole, Rudolph);
      and the elemental 2-state progression (Gnome/Salamander/Sylph/Undine).
      Pumpkin has no inherent bonus (chocolate-finding). Lizard's evo-swap waits on
      its Phase-3 magnitude formula.
- [x] **3a** — export-only formulas (`apply_campaign_formulas` + a `roster` ref
      on `CampaignContext`): Bag (lowest unlocked growth ^0.4), Mermaid
      (-growth/1000, cap -333), Lizard ((unlocked+evolved)^0.5×10, to Growth
      unevolved / Food evolved).
- [x] **3b** — user-input formulas: a persisted `CampaignInputs` struct + a
      "Campaign bonus inputs" panel (held + Beachball-given stones / challenge
      points / honey / ants / couples / Delirious-Essence fights), wired into
      `CampaignContext`. Beachball, Unicorn, Bear, Ant Queen, Cupid's couples,
      and **Aether** (the two-part penalty + growth formula). *Campaign-bonus
      formula coverage is now complete for every modellable pet.*
- [ ] **stretch** — campaign planner: allocate pets to prioritized campaigns,
      excluding dungeon-allocated pets, optionally suggesting unlockables; later,
      simulate growth/reward outcomes.

## Class & equipment bonuses (toggleable)

Beyond the pet's *innate* campaign bonus, two more sources stack on top in-game,
surfaced as **toggles** (default off) so you can plan around durable innate
bonuses. They layer in via the same `campaign_bonuses` seam (additive, all
campaigns), gated by `CampaignContext` flags.

- [x] **Equipment — sticks.** Walking / Journeying / Magic / Legendary, equipped
  in the weapon slot. `value = cap · (rank/9) · ((1+upgrade)/21)` (so SSS+20 hits
  the cap exactly; caps 16.67 / 33.33 / 50 / 100). `Quality::campaign_rank`
  (F=1…SSS=9). The `include_equipment` flag + a "+ equipment" checkbox. Verified
  vs in-game (Otter Magic SSS+10 = 26.19%).
- [x] **Class — Adventurer.** Base `2% · CL` to all campaigns when the pet is an
  Adventurer (`class_campaign_bonus`, `include_class` flag + "+ class" checkbox;
  Robot CL8 → 16%), **plus** the per-pet Adventurer **evo bonus** added to the
  base: `(2 + evo) · CL` (e.g. Hedgehog +0.58 → 56.76% at CL22; game shows 57).
  The 44 evo-bonus values are a curated `ADVENTURER_EVO_BONUS` table in
  `merge.rs`, guarded by `test_adventurer_evo_bonus_names_exist` (every key must
  resolve to a real pet).
- [~] **Event equipment** — scanned across all 3 slots alongside sticks. No clean
  formula (Candy Cane is +101% at SSS+20, +104.76% at +21, +150% at +30 — doesn't
  fit the stick curve), so only the as-purchased **SSS+20** values are plugged in
  (`event_equip_bonus`: Candy Cane 101, Merry Mantle 150, Christmas Boots 150);
  other levels return None rather than guess. Extend with more items/levels as
  values are known.

## Deferred odds and ends

- **Event campaigns** — a possible 8th `CampaignType`. Currently unmodeled, so
  Clam's "+50% Event" is dropped (we keep its Item 35). Adding it ripples through
  the exhaustive `CampaignType` matches (`format_action`, `action_sort_key`,
  `campaign_label`) and the "all campaigns" vs "all non-event" distinction —
  `CampaignType::ALL` (used by `set_all` / parser "all campaigns") should stay
  the 7 standard, with Event set only by explicit clauses. Low priority (rarely
  appears in-game).
- **Nightmare's team malus** — reduces *other* pets' campaign contribution by
  `(20 - 0.25 * class_level)%` each (min 1%; 20% unevolved). Not Nightmare's own
  bonus (already +200 all), but the campaign **simulator** stretch will need it.
- **Pumpkin** — no inherent campaign bonus; finds chocolate, so a simulator
  should still favor it for food campaigns.
## Input-driven special cases (done)

These three needed bespoke handling beyond the override/formula norm; all are
now modelled (`apply_campaign_formulas` + `class_campaign_bonus`, with new
`CampaignInputs` fields and a `parse_flexible_number` helper for big numbers).

- **Stone/Golem** — evolved flat **+100% all campaigns** is curated
  (`when: Evolved set_all: 100`). **Unevolved** it ramps with growth:
  `-100% + 20% per 5000 growth`, capped at `0%` (25000 growth) — a code formula.
  A **1500-CP campaign upgrade** (`stone_campaign_upgrade` checkbox) adds another
  **+100% all campaigns** on top of either state.
- **Goblin** — two parts. (1) **UCCs** (`goblin_ucc`, cap 75) add **+1% to every
  campaign** on top of its curated base (`-100` growth/item, `+150` divinity,
  `+50` others), reaching the documented `-25 / +225 / +125` at the cap. (2) Its
  **Adventurer evo bonus is dynamic** — the `0.1` base climbs with **Overflow
  Challenges** (`goblin_oc`, cap 470): the first 100 add `0.008` each, 101..=470
  add `0.001622` each, reaching the full `1.5`/CL at the cap. Handled in
  `class_campaign_bonus` (the only pet whose evo bonus reads `ctx`).
- **Earth Eater** — flat all-campaign bonus ramping `-80% → +82%`. The token
  upgrade does **not** stack a second bonus; it *lowers the per-rebirth starting
  penalty* based on lifetime planets, eventually removing it entirely and locking
  him at `+82%`. Since each rebirth is otherwise fed to the `+82%` cap in ~1.35h,
  his realistic in-play value is **`+82%`**, shown by default (a checked
  **"Lock at +82%"** box, stored inverted as `earth_eater_show_lifetime`).
  Unchecking it — on a **token-improved** pet with a **total** entered — reveals
  the lower *permanent* value (`-80% + 1% per 200k`, cap `+82%` at 32.4M), with a
  "~time to lock @1/s" estimate. The total is a flexible-notation text input
  (`earth_eater_total_planets`, `32.4e6`). Locked-at-82 is the planning default
  because the permanent value is only ever a floor under what feeding achieves.
