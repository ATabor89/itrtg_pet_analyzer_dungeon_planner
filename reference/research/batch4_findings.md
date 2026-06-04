# Batch 4 Pet Research Findings

## Hourglass
- **Element:** None specified (utility pet)
- **Special Mechanics:** First action each turn applies a speed debuff to enemies: 10% + 0.2% * CL, lasting 2 turns, stackable. Limited to one debuff application per turn. Effect is halved in boss rooms. Has 30% speed bonus but 10% penalties to health, attack, and defense.
- **Stat Modifiers:** +30% speed, -10% health, -10% attack, -10% defense
- **Synergies:** Useful in Earth Tower (Supporter evolution), Water D3 (two Supporter teams), Rogue evolution for Strategy Room speed bonuses
- **Anti-synergies:** Incompatible with Undine or Vermilion Pheasant on same team
- **Equipment Notes:** None specified
- **Token Improvement:** Not mentioned
- **Constraint Implications:** Hard anti-synergy constraint: cannot be on team with Undine or Vermilion Pheasant. Supporter evolution changes behavior from speed debuff to speed buff (1% per CL, max 50%). Stat modifiers (+30% speed, -10% HP/ATK/DEF) need to be factored into team calculations.

## Archer
- **Element:** None specified (physical)
- **Special Mechanics:** Chance to shoot two arrows at once when a bow is equipped: (20 + CL * 1.25)%, capped at 100%. Requires bow weapon to trigger double-hit mechanic. Deals 5x damage with Celestial Bow (special content).
- **Stat Modifiers:** None mentioned
- **Synergies:** Excels in lategame dungeons against Nanobots due to dual-attack capability
- **Anti-synergies:** None
- **Equipment Notes:** MUST equip a bow (specifically Hurricane Bow SSS+20 for evolution). Double-hit mechanic only works with bow weapons - this is a critical equipment constraint.
- **Token Improvement:** Not mentioned
- **Constraint Implications:** Equipment priority override: must use bow weapons. Class bonus is Assassin (+1.2% * CL single target damage). The bow requirement is a hard constraint that the planner should enforce.

## Witch
- **Element:** Water
- **Special Mechanics:** Increases water element of ALL party members by 0.5% * CL. Decreases water element of enemies she attacks by 0.5% * CL before her attacks hit (only for her own damage calculations). Extra +1.25% * CL damage from multi-target attacks when class is Mage.
- **Stat Modifiers:** None mentioned
- **Synergies:** Best mage in the game. Common D4 Water Temple strategy: solo Witch DPS with 3 supporters, 1 defender, 1 rogue. Works with Vampire and Todlicher Loffel for food campaigns.
- **Anti-synergies:** None mentioned
- **Equipment Notes:** Should prioritize water element equipment to maximize her water element scaling
- **Token Improvement:** Not mentioned
- **Constraint Implications:** Party-wide water element buff makes her valuable for any water-focused team. Her team-wide water element boost (+0.5% * CL to all party members) means she synergizes well with other water-element pets. Mage class strongly preferred.

## Succubus
- **Element:** None specified
- **Special Mechanics:** Single-target attacks provide self-healing equal to (CL / 3)% of maximum HP. This is lifesteal on single-target attacks.
- **Stat Modifiers:** None mentioned
- **Synergies:** Viable for Volcano D4 teams and Gram fight teams
- **Anti-synergies:** None mentioned
- **Equipment Notes:** None specified
- **Token Improvement:** Not mentioned
- **Constraint Implications:** Assassin class bonus (+1% * CL single target damage). Self-sustaining DPS due to lifesteal. Can be built as Rogue for speed-focused strategies.

## Stale Tortilla/Taco
- **Element:** Variable (depends on highest elemental resistance)
- **Special Mechanics:** When evolved and attacking, Taco grants buffs to unbuffed party members. Buff type depends on Taco's highest elemental resistance (priority: water > fire > earth > wind). Four buff types:
  - Fire buff: targets highest ATK pet, adds CL% damage
  - Water buff: targets lowest HP pet, heals CL*2%
  - Earth buff: targets lowest DEF pet, reduces damage by CL/2%
  - Wind buff: targets highest SPD pet, grants CL% bonus damage chance
  All effects cap at CL 50.
- **Stat Modifiers:** None mentioned
- **Synergies:** Flexible support that adapts based on equipment element
- **Anti-synergies:** None mentioned
- **Equipment Notes:** Element of equipped gear determines which buff Taco provides. This is a critical equipment-element interaction. Player should choose element gear based on desired buff type.
- **Token Improvement:** Not mentioned
- **Constraint Implications:** Unique buff mechanic tied to equipment element. Planner should allow users to choose which buff type they want and then recommend appropriate element equipment. This pet functions as a flexible support regardless of class.

## Cardboardbox
- **Element:** None specified
- **Special Mechanics:** Evolved form (Cardboard Cat) alters event rewards with 50/50 Good/Bad chance. Good: rewards * (1.1 + 0.007 * CL). Bad: rewards * (0.5 + 0.007 * CL). At CL 50: Good=145%, Bad=85%, Average=115%. CL 29 is breakeven point. Excludes Treasure and Lava-Filled Corridor events.
- **Stat Modifiers:** None mentioned
- **Synergies:** Stacks with Clam's GP doubling for God Power rewards
- **Anti-synergies:** Below CL 29, the pet is a net negative on dungeon event rewards
- **Equipment Notes:** None specified
- **Token Improvement:** Not mentioned
- **Constraint Implications:** Requires evolution AND CL 29+ to be beneficial. Planner should warn if CL is below 29. Synergy with Clam for GP events. Unlock requires killing every D3 boss at least once with it in the team.

## Clam
- **Element:** Water (implied by Nevermelting Ice evolution material)
- **Special Mechanics:** Doubles ALL God Power earned from dungeon events. Works evolved or unevolved. Only applies to the dungeon where Clam is active.
- **Stat Modifiers:** None mentioned
- **Synergies:** Stacks with Cardboardbox for GP event rewards. GP doubling relevant in: D1 Scrapyard (Cursed Chest), D2 Volcano (Cursed Chest), D3 Mountain (Portal From Beyond), D4 Water Temple (Ocean Path).
- **Anti-synergies:** Undine is a stronger Water Rogue in later dungeons (but Clam can be assigned to different class to coexist)
- **Equipment Notes:** None specified
- **Token Improvement:** Not mentioned
- **Constraint Implications:** Rogue class bonus (+0.6% * CL item drop rate). GP doubling is a passive team-wide effect, not class-dependent. Valuable utility pet for GP farming dungeons.

## Mysterious Egg / Golden Chicken
- **Element:** None specified
- **Special Mechanics:** Evolved form (Golden Chicken) increases damage taken instead of a party member by 1.5% * CL (50% max cap). Defender class adds extra 10% protect chance. Post-evolution: all pets get 2% per CL bonus to clone training exp (caps at 100% at CL 50).
- **Stat Modifiers:** None mentioned
- **Synergies:** Top choice for Defender slot
- **Anti-synergies:** None mentioned
- **Equipment Notes:** None specified
- **Token Improvement:** Not mentioned
- **Constraint Implications:** Defender class strongly preferred. The protect mechanic (absorb damage for teammates) makes this a top-tier tank pet. Very high evolution requirements (100,000 growth, 2M combined pet levels).

## Vesuvius
- **Element:** Fire (implied by Fire Stone evolution material)
- **Special Mechanics:** Distributes growth to weakest pet: 50% + 2% * CL of dungeon growth, capped at 200% at CL 75. Mage class bonus: +0.78% * CL multi-target attack damage.
- **Stat Modifiers:** None mentioned
- **Synergies:** Useful for leveling weak pets while running dungeons
- **Anti-synergies:** None mentioned
- **Equipment Notes:** None specified
- **Token Improvement:** Not mentioned
- **Constraint Implications:** Mage class preferred. Growth distribution is a passive utility effect that doesn't affect combat. Campaign penalties (-50% growth) but good for item/GP campaigns.

## Raven
- **Element:** None specified
- **Special Mechanics:** Increases dungeon item drop rate by 0.67% * CL if class is Rogue.
- **Stat Modifiers:** None mentioned
- **Synergies:** None mentioned
- **Anti-synergies:** None mentioned
- **Equipment Notes:** Requires Bursting Knives SSS+20 for evolution
- **Token Improvement:** Not mentioned
- **Constraint Implications:** Rogue class for item farming utility. Straightforward drop rate pet.

## Earth Eater
- **Element:** Earth (implied by name and Special Wood evolution material)
- **Special Mechanics:** Campaign-focused: -80% to all campaigns base, can be fed Earthlike Planets to reach +82%. Resets on rebirth. Adventurer class: +1.32% * CL campaign bonus.
- **Stat Modifiers:** None mentioned (campaign only)
- **Synergies:** None mentioned for dungeons
- **Anti-synergies:** None mentioned
- **Equipment Notes:** None specified
- **Token Improvement:** Permanently reduces campaign penalty based on total Earthlike Planets fed across all rebirths (1% per 200k planets, caps at 32.4M planets for permanent +82%)
- **Constraint Implications:** Primarily a campaign pet. Token improvement is very significant but requires enormous investment. No special dungeon mechanics noted.

## Bag
- **Element:** None specified
- **Special Mechanics:** Campaign-focused: growth bonus based on weakest pet's growth (weakest_growth^0.4, max 100%). Redirects 10% of earned growth to weakest pet during growth campaigns.
- **Stat Modifiers:** None mentioned
- **Synergies:** None mentioned for dungeons
- **Anti-synergies:** None mentioned
- **Equipment Notes:** None specified
- **Token Improvement:** Reduces weakest pet growth deduction from 10% to 5% but makes it bonus growth (no longer deducted from normal rewards)
- **Constraint Implications:** Campaign utility pet. No special dungeon mechanics.

---

## Pets with No Special Dungeon Constraints

The following pets have no special dungeon mechanics, synergies, anti-synergies, or constraints worth noting for a dungeon team planner. They are primarily campaign/utility pets:

- **Firefox** - Fire element support, Blacksmith class bonus (+0.75% * CL crafting speed). No special dungeon mechanics beyond basic fire element.
- **Beachball** - Campaign-only pet (bonus based on pet stones deposited). Adventurer class. No dungeon relevance.
- **Tanuki** - Supporter class bonus (+0.68% * CL heal power and buff effect). No special dungeon mechanics.
- **Seal** - Alchemist class bonus (+0.64% * CL item creation speed). No special dungeon mechanics.
- **Bottle** - Alchemist class bonus (+0.9% * CL item creation speed). Crafting material savings mechanic. No dungeon combat relevance.
- **Portal** - Adventurer class. Divinity campaign bonus. Post-evolution amplifies divinity gains. No dungeon combat relevance.
- **Unicorn** - Adventurer class. Campaign bonus scales with Challenge Points. No dungeon combat relevance.
- **Sloth** - Adventurer class. Campaign bonus scales with campaign duration. Gets 2x growth from food. No dungeon combat relevance.
- **Cocoa** - Alchemist class. Can craft Chocolate when evolved as Alchemist. No dungeon combat relevance.
- **Wolf** - Adventurer class. Item campaign bonus scales with challenge completions. No dungeon combat relevance.
- **Swan** - Fishing speed bonus. No dungeon combat relevance.
- **Void** - Alchemist class. Crafts Nothing/Not Nothing faster. No dungeon combat relevance.
- **Thunder Ball/Raiju** - Adventurer class. Campaign bonus pet. No dungeon combat mechanics noted. Note: anti-synergy with Robot in campaigns (not dungeons).

---

## Summary of Key Dungeon Constraints for Planner

### Hard Anti-Synergies (must not be on same team):
- **Hourglass** cannot be with **Undine** or **Vermilion Pheasant**

### Stat Modifiers:
- **Hourglass**: +30% speed, -10% HP, -10% ATK, -10% DEF

### Equipment Constraints:
- **Archer**: Must equip bow weapons for double-hit mechanic to work
- **Stale Tortilla/Taco**: Equipment element determines buff type (water > fire > earth > wind priority)

### Class-Specific Behaviors:
- **Hourglass** as Supporter: changes from speed debuff to speed buff (1% per CL, max 50%)
- **Witch** as Mage: extra +1.25% * CL multi-target damage, party-wide +0.5% * CL water element
- **Succubus** as Assassin: +1% * CL single target damage, built-in lifesteal (CL/3)% max HP
- **Archer** as Assassin: +1.2% * CL single target damage, double-hit with bow
- **Golden Chicken** as Defender: +1.5% * CL damage absorption + extra 10% protect chance
- **Cardboardbox** evolved: event reward modification (needs CL 29+ to be net positive)
- **Clam** as Rogue: +0.6% * CL item drop rate + doubles GP from dungeon events
- **Stale Tortilla/Taco**: element-dependent buffs to party members when attacking

### Team Synergies:
- **Clam + Cardboardbox**: stacking GP event reward bonuses
- **Witch**: party-wide water element buff benefits all water-element team members
- **Witch D4 Water Temple meta**: Witch (Mage DPS) + 3 Supporters + 1 Defender + 1 Rogue
