# Next Features

## 1. Gem Recommendation Enhancement

Consider enhancing the algorithm to recommend gems. Use the following reference data from the wiki.

### Gem Stat Bonuses

| Gem Type | Bonus |
|----------|-------|
| Neutral | Increases all element stats by `1 * Gem Level * Equipment Tier` |
| Water | Increases health by `1% * Gem Level * Equipment Tier` |
| Fire | Increases attack by `1% * Gem Level * Equipment Tier` |
| Wind | Increases speed by `1% * Gem Level * Equipment Tier` |
| Earth | Increases defense by `1% * Gem Level * Equipment Tier` |

### Gem Guidelines by Class

> **Note:** This is a baseline guideline and will change depending on gear and dungeon progress. Gems are often used to compensate for equipment/class weaknesses. Gem placement on specific equipment slots doesn't matter for stat purposes, but keeping them consistent (e.g., fire on weapons, water/earth on accessories) makes swapping easier during reforging/upgrading.

- **Mage:** 1 Fire, 2 Water. Swap a Water for Neutral if more elements are needed.
- **Defender:** All Earth (v4s) / All Water (dungeons). A mix of Earth/Water works if you don't want to re-gear between v4s and dungeons.
- **Supporter:** 1 Fire, 1 Water, and either 1 Wind or Neutral depending on need. 2–3 Fire gems can help low-level Supporters who can't yet heal a pet fully.
- **Rogue:** 1 Fire, 1 Water, and either another Fire or a Wind gem. Rogues usually have Knives, and extra speed helps with defense reduction.
- **Assassin:** 2 Fire, 1 Water. If low speed, swap a Fire for Wind. Swap Fire for Neutral to help with later dungeons if needed.
- **Blacksmith:** Very flexible. 1 Fire, 1 Earth, 1 Water for defensive. Swap Earth for Wind for more offensive.
- **Wind/Neutral gems (general):** Used to address speed or elemental problems. Neutral gems are both offensive and defensive and can replace either Water or Fire. Often replaced by Fire/Water as the pet levels up and equipment is enchanted, but keep a stockpile.

### Implementation Notes

This does **not** mean the algorithm must change. Review these heuristics and consider whether any changes would produce smarter recommendations.

---

## 2. Dungeon Planner Display Improvements

Improve the pet card display in the dungeon planner to show more useful information at a glance.

### Equipment Comparison

For each pet card, show the pet's **current equipment** and flag any differences from the recommended equipment:

- If current equipment **matches** the recommendation, show it with a positive indicator (e.g., checkmark).
- If current equipment **differs**, show both current and recommended in a hybrid view.
- Highlight differences in **equipment quality and upgrade level**. For example, if the recommendation is S+10 and the pet currently has S+4, that gap should be visually obvious so we can identify what still needs upgrading.

### Additional Pet Info on Cards

Display relevant pet metadata on the cards:

- Dungeon level
- Class level
- Any other stats that help at a glance

### Design Goal

Keep the display informative without cluttering it. Use your judgment on layout — the hybrid view (match vs. diff) should handle most cases cleanly.

---

## 3. Pet Pairing Constraints

Add support for positive and negative pairing constraints between pets. These will be provided manually in a data file (format TBD — consider what works best).

### Constraint Types

- **Positive pairing (class-level):** Pet should be paired with a certain class.
- **Positive pairing (pet-level):** Pet should be paired with a specific other pet by name.
- **Negative pairing (pet-level):** Pet should **not** be paired with specific other pets.

### Motivating Example: Hourglass

From the wiki:

> Hourglass's first action each turn will debuff enemy speed. Additional actions will be performed normally (attacking or healing, based on pets' health).
>
> If evolved as a Supporter, Hourglass will give pets a speed bonus (1% per Hourglass CL, max 50%) instead of the normal Supporter damage reduction. Because of this, it's suggested to pair Hourglass with a second Supporter, which can provide damage reduction.
>
> Hourglass has a 30% bonus to its speed stat, and a 10% penalty to its health, attack and defense stats.
>
> If Hourglass and Undine or Vermilion Pheasant are on the same team, Undine's passive DoT and Pheasant's Speed Damage Bonus will not ever be applied, regardless of the order they are added or what dungeon they are in.

**Derived constraints for Hourglass:**

- **Positive:** Should be paired with a Supporter (class-level).
- **Negative:** Should not be paired with Undine or Vermilion Pheasant (pet-level).

### Future Use

This system should be generic enough to handle synergy pairings between any two pets (e.g., two pets that buff each other and should always be on the same team).

---

## 4. Custom Pet Information / Special Abilities

Add support for custom per-pet information that can influence team building and equipment recommendations.

### Motivating Example: Sylph

From the wiki:

> Multiplies the wind element it receives from equip by 1.5. In dungeons she hits once more every 450 wind element, up to 7 extra hits.

Given this, it would be reasonable to **prioritize wind** for Sylph even if that deviates from the standard class-based recommendation.

### Challenge

The main difficulty is creating a data format that can adequately represent the **unique bonuses** of individual pets in a way the planner can actually act on. These abilities are highly varied and don't follow a single pattern.

### Possible Approaches

- A structured format with known fields (e.g., `stat_priority_override`, `element_scaling`, `extra_hits_per_element`) that covers common cases.
- A freeform notes field for human reference, with optional structured fields the algorithm can consume.
- A combination: structured data for what the algorithm can use, plus freeform text for context.

Determine the best approach during implementation.