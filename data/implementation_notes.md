# Next Features

## 1. Dungeon Planner - Total Counts

Add something to the dungeon planner that shows the total counts of missing things. The primary concern here is to see a single "shopping list" of all pets, equipment, and gems that are missing.

Note that I have yet to actually add any gems to equipment, so we're still not entirely sure how this will appear in the imported data. We'll likely want to add a gem slot to the equipment model that can be referenced here. If we add this to the model now, we can simply default it to `None` and come back to this later to determine the changes needed in the parser.

Bonus: While working on this, consider making the "Select Dungeons" section collapsible, similar to the Pet Constraints. This will allow for better screen real estate once the dungeons/teams are actually planned.

## 2. Pet Pairing Constraints

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

## 3. Custom Pet Information / Special Abilities

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