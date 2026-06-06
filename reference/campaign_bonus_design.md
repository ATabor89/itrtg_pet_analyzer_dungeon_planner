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

`CampaignContext` carries the persisted user inputs + a roster reference; the
pet's own evolved/improved/growth state comes from its export.

## Elementals (their own sub-phase)

Aether/Salamander/Gnome/Sylph/Undine are the spiciest. Undine and the
Gnome/Sylph evolved forms fit the declarative `on_evolved` model. Aether needs a
dedicated formula (penalty shrinks with boss kills / growth / owning the other
elementals — likely a user-input for the boss-kill count). Salamander is literal
`+???%` on the wiki, so it stays raw-only until the wiki states a number.

## Phase status

- [x] **1a** — scrape + conservative parser + data (PR #18).
- [x] **1b** — card display, "Campaign boost" filter, "Sort by bonus".
- [~] **2a** — override mechanism (`campaign_overrides.yaml` + `CampaignContext`
      seam) with a focused high-confidence curated set (Hedgehog token, Nothing/
      Corona evo flips, the clear "more levels / divinity / god power" prose
      corrections). Set-all / set / add ops × Always / Evolved / Unevolved /
      TokenImproved conditions.
- [ ] **2b** — expand curation: the rest of the unambiguous prose pets, the
      remaining per-form elementals (Undine, evolved Gnome/Sylph). Lizard's
      evo-swap waits on its Phase-3 magnitude formula.
- [ ] **3** — export/user-input formulas (Bag, Mermaid; stones/points/honey
      inputs for Beachball/Unicorn/Bear).
- [ ] **stretch** — campaign planner: allocate pets to prioritized campaigns,
      excluding dungeon-allocated pets, optionally suggesting unlockables; later,
      simulate growth/reward outcomes.
