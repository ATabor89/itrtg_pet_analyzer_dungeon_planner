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
  - **Beachball / Unicorn / Bear** = `sqrt(stones)`, `sqrt(challenge points)/2`,
    `honey/500` — from **user-input fields** persisted in `AppState`.
  - **Cupid couples** = token-improved Cupid's per-couple bonus. The game only
    reports "current couples" (a pet can be coupled with itself, so couples ≠
    pets-in-couples). Compromise: a "current couples" user input, **+2% per
    couple** to all campaigns, on top of the flat token bonus already curated.
  - **Aether** = a user-input for "Delirious Essence fights completed" drives
    both (a) the all-campaign penalty: `-99%` reduced by `10%` per fight, maxing
    at `+1%` after 10 fights; and (b) an *added* growth-campaign bonus:
    `((elementals_unlocked + 5) / 10) * fights * (1 + 0.57 * log_1000(growth))`%,
    where growth clamps to 1 if non-positive and `elementals_unlocked` counts
    Undine/Gnome/Salamander/Sylph/Elemental.

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
- [~] **3b** — user-input formulas: a persisted `CampaignInputs` struct + a
      "Campaign bonus inputs" panel (stones / challenge points / honey / ants /
      couples / Delirious-Essence fights), wired into `CampaignContext`. Done:
      Beachball, Unicorn, Bear, Ant Queen, and Cupid's couples. Remaining: Aether
      (its two-part formula — the all-campaign penalty *and* the growth bonus from
      fights × elementals-unlocked × log_1000(growth)).
- [ ] **stretch** — campaign planner: allocate pets to prioritized campaigns,
      excluding dungeon-allocated pets, optionally suggesting unlockables; later,
      simulate growth/reward outcomes.

## Future: class & equipment bonuses (toggleable)

Beyond the pet's *innate* campaign bonus, two more sources stack on top in-game:

- **Class (Adventurer):** Adventurers add a bonus to all campaigns, and pets with
  an Adventurer **evo bonus** get extra. Computable from the export's class +
  the wiki evo-bonus data. Interesting because a pet with a lower innate bonus
  but a strong Adventurer evo bonus can overtake a higher-innate one.
- **Equipment:** Walking / Journeying / Magic / Legendary sticks (and some
  limited event gear) boost campaign gains — the boost depends on the item
  tier/quality/upgrade *and* the pet (e.g. a Magic Stick SSS+10 gives Earth
  Eater +50% but Otter +26.19%). The game exposes the formulas (to be provided).

Design intent: surface these as **toggles**, defaulting off, because planning
around a pet's durable innate bonus is usually better than around whatever stick
it happens to hold now. They'd layer in via the same `campaign_bonuses` seam
(class first, then equipment), gated by the toggles — so the filter/sort/card
pick them up only when the user opts in. Not built yet; tracked here.

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
