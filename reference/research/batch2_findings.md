# Batch 2 Pet Research Findings

Research from ITRTG wiki pages for 25 pets. Focus on dungeon-relevant special mechanics.

---

## Chameleon
- **Element:** Water (but can freely change element via pet page)
- **Special Mechanics:** After evolution, counts as the correct element for dungeon events regardless of actual element setting. Functions as an element wildcard. Attack mechanic: MultiTargetLeech - attacks 4 times per action at ~0.25x damage each, healing itself for 100% of damage dealt.
- **Stat Modifiers:** None documented
- **Synergies:** Extreme flexibility - can satisfy any element requirement in dungeon events. Can fill neutral pet slots in events like "Portal from Beyond."
- **Anti-synergies:** None documented
- **Equipment Notes:** Element equipment can be any element since the pet counts as correct element regardless
- **Token Improvement:** Not documented
- **Class-Specific:** Wildcard class in dungeons
- **Constraint Implications:** MAJOR - This pet is a universal element wildcard after evolution. The planner should treat evolved Chameleon as able to satisfy ANY element requirement. The self-healing leech attack makes it self-sustaining. This is one of the most flexible pets for team composition.

---

## Undine
- **Element:** Water
- **Special Mechanics:** Percentage-based damage over time to all non-boss enemies. Scales with Water element stat and evolution level:
  - V3+: (1 + Water/500)% of enemy max HP per turn
  - V4: (3 + Water/400)% of enemy max HP
  - V5: (5 + Water/300)% of enemy max HP
  - Capped at 10% of enemy max HP
- **Stat Modifiers:** None base
- **Synergies:** Effective in Infinity Towers - ignores reflected damage in Neutral Tower, bypasses dodge in Wind Tower, overcomes high defense in Earth Tower
- **Anti-synergies:** CRITICAL - Undine's passive damage does NOT apply in Infinity Towers if paired with Hourglass pet, regardless of team order or tower type
- **Equipment Notes:** Prioritize Water element equipment to maximize the DoT scaling
- **Token Improvement:** Not documented
- **Class-Specific:** Rogue class bonus - extra 1.5% * class level item drop rate as Rogue
- **Constraint Implications:** MAJOR ANTI-SYNERGY with Hourglass in Infinity Towers. Planner must enforce: never pair Undine + Hourglass in Infinity Tower teams. Water element equipment priority for DoT scaling. Very strong in Infinity Towers otherwise due to bypassing multiple defensive mechanics.

---

## Gnome
- **Element:** Earth
- **Special Mechanics:** Adds a shield to ALL party members. Shield scales with evolution:
  - V1: Equal to Earth Element stat
  - V2: 3x Earth Element
  - V3: 5x Earth Element x (0.5 + 0.02 * Class Level)
  Shield resets fully each room. Cannot be healed by Supporters. Post-evolution: additional defense boost of (Earth Element / 300)% capped at 20% (requires 6000 element).
- **Stat Modifiers:** +10% protect chance as Defender; damage redirection up to 50% at high CL
- **Synergies:** Pairs well with any team as a universal shield provider. Shield resets per room making it consistently valuable.
- **Anti-synergies:** Supporters cannot heal the shield portion of HP
- **Equipment Notes:** Prioritize Earth element equipment to maximize shield strength
- **Token Improvement:** Not documented
- **Class-Specific:** Defender class - 1.5% * CL bonus, 10% protect chance increase
- **Constraint Implications:** Earth element equipment priority for shield scaling. Planner should note shields reset per room (always full value). The shield cannot be healed, so Supporter healing only applies to base HP. Strong defensive pick for any team composition.

---

## Salamander
- **Element:** Fire
- **Special Mechanics:** AoE heal every turn - heals entire party at start of every turn BEFORE phoenix feathers are used. Healing scales with Fire element stat, capping at 15% HP restoration at 1500 Fire element. Healing strength varies by evolution:
  - V0-V1: Ability inactive
  - V2: 1/8 strength
  - V3: 1/4 strength
  - V4 (non-Supporter): 1/2 strength
  - V4 (Supporter): Full strength
- **Stat Modifiers:** Campaign bonus scales from -197% (V0) to +89% (V4)
- **Synergies:** Enables non-neutral team compositions for advance farming. Provides post-trap-damage healing before enemy turns.
- **Anti-synergies:** None documented
- **Equipment Notes:** Prioritize Fire element equipment to maximize healing (cap at 1500 fire element)
- **Token Improvement:** Not documented
- **Class-Specific:** Supporter class bonus - 1.5% * CL heal power increase. Can be reclassed as Rogue while retaining AoE healing at reduced effectiveness.
- **Constraint Implications:** MAJOR - Healing is massively stronger as V4 Supporter (full strength vs half for non-Supporter). Fire element equipment priority for heal scaling. Unique mechanic: heals BEFORE phoenix feather checks, making it a pre-emptive healer. Planner should strongly favor Supporter class for Salamander.

---

## Sylph
- **Element:** Wind
- **Special Mechanics:** As Mage, gains bonus attacks based on Wind element: one extra hit per 450 Wind element, capped at 7 extra hits (at 3150 Wind element). At CL100 Mage: up to 15 total attacks (3 base + 5 from CL + 7 bonus). Wind element of equipped items increased by 50% (e.g., +100 wind becomes +150).
- **Stat Modifiers:** Starts with -33,333 growth penalty, making early evolutions very difficult
- **Synergies:** Extremely strong Mage DPS with high Wind element investment
- **Anti-synergies:** The -33,333 starting growth makes team requirements like "all pets need 5k growth" (Shark evolution) very hard
- **Equipment Notes:** PRIORITY OVERRIDE - Wind element equipment gets 50% bonus. Should stack Wind element gear for both the element scaling and extra hit thresholds.
- **Token Improvement:** Not documented
- **Class-Specific:** Mage class - extra attacks scale with Wind element. By far the strongest as Mage.
- **Constraint Implications:** MAJOR equipment override - Wind element equipment gets 50% bonus, so always prioritize Wind gear. Extra hit thresholds at 450/900/1350/1800/2250/2700/3150 Wind element create breakpoints the planner could track. Starting growth penalty of -33,333 is a major constraint for growth-gated content.

---

## Aether
- **Element:** Neutral
- **Special Mechanics:** Campaign bonus starts at -99% and improves based on: defeating Delirious Essence of the Forgotten boss, total growth, and having Undine/Gnome/Salamander/Sylph unlocked. Improvement formula involves all four elemental pets.
- **Stat Modifiers:** None documented for dungeons
- **Synergies:** REQUIRES Undine, Gnome, Salamander, and Sylph to be unlocked for full campaign bonus scaling
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Not documented
- **Class-Specific:** Adventurer class bonus - 1.5% * CL for all campaigns
- **Constraint Implications:** The dependency on all four elemental spirits (Undine, Gnome, Salamander, Sylph) being unlocked is a soft prerequisite. Planner could note this as a progression gate. The Delirious Essence boss fight recommends: 2 Defenders, 2 Supporters, 2 DPS (Assassin or Mage) with neutral-element non-DPS pets.

---

## Ape
- **Element:** Fire
- **Special Mechanics:** Token upgrade transforms Ape into "Super Ape" on first dungeon action. Lasts (Class Level / 5) actions. During transformation:
  - Class Level% boost to ALL stats
  - Double hits per attack (except Mages retain normal hit count)
  - Activation limited to once per room/floor
- **Stat Modifiers:** During Super Ape: +CL% to all stats (e.g., +100% at CL100)
- **Synergies:** Rogue Ape works well for speed-focused encounters (Earth Tower, Sluggernaught). Good for fights needing high early burst damage (Lernean Hydra in Gram 4).
- **Anti-synergies:** Not useful as Mage (no double hits for Mages)
- **Equipment Notes:** None specific
- **Token Improvement:** SIGNIFICANT - The Super Ape transformation is the entire special mechanic. Not very useful until CL 50+ due to duration scaling.
- **Class-Specific:** Assassin class base bonus - 0.65% * CL single-target damage. But Super Ape works with any class except Mage gets reduced benefit.
- **Constraint Implications:** Token-improved Ape has a temporary but powerful burst. Planner should note: Mage class gets reduced benefit (no double hits). Best as Rogue or Assassin. Burst lasts CL/5 actions, so duration scales with class level. Once-per-room activation.

---

## Turtle
- **Element:** Earth
- **Special Mechanics:** Defender class bonus - increases defense by 0.6% * CL and increases chance to take damage instead of party member by 10% as Defender.
- **Stat Modifiers:** -25% effectiveness in 1-hour campaigns, +25% in 12-hour campaigns
- **Synergies:** Defender class tank teams
- **Anti-synergies:** Poor for short campaigns
- **Equipment Notes:** None
- **Token Improvement:** Not documented
- **Class-Specific:** Defender - defense scaling + protect chance
- **Constraint Implications:** Standard Defender pet. The 10% protect chance increase is Defender-class-only. Better for longer dungeon runs.

---

## Holy ITRTG Book
- **Element:** Fire
- **Special Mechanics:** Doubles attack power and grants critical hit immunity when fighting other gods. Class flexibility: lets you change classes without loss or cost, and counts for the right class in dungeon events.
- **Stat Modifiers:** Doubles attack vs gods, crit immunity vs gods
- **Synergies:** Universal class compatibility - works with all classes
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Tavern quest bonus - increases CL experience gains by 0.15% * CL in participating quests. Guarantees inclusion in required pet lineups for tavern quests.
- **Class-Specific:** Works across ALL classes with 0.4% * CL bonus. The class change/count mechanic is unique.
- **Constraint Implications:** The "counts for right class in dungeon events" mechanic is important - this pet satisfies class requirements regardless of its actual class. Planner should treat this similarly to Chameleon's element wildcard but for classes. Endgame pet with very high unlock requirements.

---

## Treasure/Mimic
- **Element:** Neutral
- **Special Mechanics:** Damage bonus from Mimic Points: (Mimic Points / 100)% up to 50%, then (Mimic Points^0.4 + 20)% beyond that. Mimic Points earned from defeating mimics in D3 dungeons (requires "Nothing" item). RTI mechanic: increases exp of all RTI pets by 5% * Class Level.
- **Stat Modifiers:** Scaling damage bonus from Mimic Points
- **Synergies:** Very strong in RTI for passive exp multiplier to all pets
- **Anti-synergies:** CRITICAL WARNING - "Opens Pandora if used in the same campaign" which reverts bonus into penalty
- **Equipment Notes:** None
- **Token Improvement:** Not documented
- **Class-Specific:** Assassin class - 1.25% * CL single-target dungeon damage
- **Constraint Implications:** MAJOR ANTI-SYNERGY - Cannot be used in certain campaign combinations (Pandora reversion). Planner must enforce campaign restrictions. The RTI exp boost (5% * CL to all RTI pets) makes this pet very valuable for progression. Needs "Nothing" item equipped to earn Mimic Points.

---

## Black Hole Chan
- **Element:** Neutral
- **Special Mechanics:** Element bonuses scale with UBv4 points: 2*(25 + sqrt(U^1.01)) capped at 400, where U = UBv4 points. Evolved bonus also scales with fastest ITRTGV4 clear time. Receives 6 * UBv4 tier growth when a UBv4 is defeated.
- **Stat Modifiers:** Doubled dungeon element bonuses compared to campaign bonuses
- **Synergies:** Very strong dungeon pet due to neutral typing and scalable growth from V4 progression
- **Anti-synergies:** None documented
- **Equipment Notes:** None specific - neutral element works universally
- **Token Improvement:** Not documented
- **Class-Specific:** Mage class bonus - 1.17% * CL damage increase
- **Constraint Implications:** Power scales with UBv4 progression. Planner could track UBv4 points as a power indicator. Neutral element makes it universally deployable. Described as "a very strong dungeon pet."

---

## Goat
- **Element:** Earth
- **Special Mechanics:** Rogue class bonus - increases dungeon item drop rate by 0.58% * CL as Rogue. Cannot find any food in food campaigns. +100% pet stones in item campaigns, +100% rare item chance.
- **Stat Modifiers:** None for dungeons
- **Synergies:** Item farming teams
- **Anti-synergies:** Completely useless in food campaigns (0% food finding)
- **Equipment Notes:** None
- **Token Improvement:** Not documented
- **Class-Specific:** Rogue - item drop rate scaling
- **Constraint Implications:** Rogue-specific item farmer. The food campaign penalty is notable but doesn't affect dungeon performance directly.

---

## Big Burger
- **Element:** Not specified
- **Special Mechanics:** Food campaign specialty - converts all Puny and Strong Food into Mighty Food when in food campaigns. Campaign bonuses: +333% food, +222% level, +111% multiplier.
- **Stat Modifiers:** None for dungeons
- **Synergies:** Strong synergy with Lizard pet (which grants growth to food camp pets based on discovered food quality)
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Not documented
- **Class-Specific:** Adventurer class - 2.0% * CL bonus to all campaigns
- **Constraint Implications:** Campaign-focused pet, no direct dungeon combat mechanics. The food conversion is campaign-only.

---

## Oni
- **Element:** Not specified
- **Special Mechanics:** Decreases stats of P.Baals after v150 by 15% (30% after evolution). Scales: 1 per 40,000 growth up to 50 P.Baals at 2M base growth.
- **Stat Modifiers:** None for dungeons directly
- **Synergies:** P.Baal progression teams
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Not documented
- **Class-Specific:** Adventurer class - 2.0% * CL campaign bonus
- **Constraint Implications:** P.Baal-specific utility, not direct dungeon combat mechanics. Growth investment affects how many P.Baals are debuffed.

---

## Gold Dragon
- **Element:** Neutral
- **Special Mechanics:** When fed, all owned pets receive 25% of the growth Gold Dragon receives. This is a passive growth distribution mechanic.
- **Stat Modifiers:** None for dungeons
- **Synergies:** Benefits ALL other pets through growth sharing when fed
- **Anti-synergies:** None documented
- **Equipment Notes:** Requires Titanium Armor, Sword, and Ring equipped for evolution (items retained)
- **Token Improvement:** Not documented
- **Class-Specific:** Alchemist class - 0.65% * CL item creation speed
- **Constraint Implications:** Growth sharing mechanic is relevant for progression planning but not direct dungeon combat. Evolution requires specific equipment (Titanium set).

---

## Seed/Yggdrasil
- **Element:** Earth
- **Special Mechanics:** Campaign bonus of 1% Growth based on highest RTI god killed. Very high total growth potential (120,000 evolved).
- **Stat Modifiers:** None documented for dungeons
- **Synergies:** RTI progression dependent
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Not documented
- **Class-Specific:** Adventurer class - 1.7% * CL campaign bonus (one of the highest CL scaling rates)
- **Constraint Implications:** High growth potential makes it a strong stat stick. Evolution requires defeating P.Baal v150 (cannot be bypassed with Pet Token). RTI-progression-gated.

---

## Pets With No Special Dungeon Constraints

The following pets have standard class bonuses but no special mechanics, unique synergies, anti-synergies, or equipment overrides that would require special handling in a dungeon team planner:

- **Bug** - Adventurer class, 0.5% * CL all campaigns. Random campaign bonus. Commonly wildcarded into Blacksmith. No dungeon combat mechanics.
- **Mole** - Adventurer class, 0.5% * CL all campaigns. 50% worse food campaign chance. No dungeon combat mechanics.
- **Camel** - Adventurer class, 0.51% * CL all campaigns. 60% better god power campaign chance. No dungeon combat mechanics.
- **Flying Cloud** - Supporter class bonus, 0.75% * CL heal/buff power. Wind element. Standard Supporter behavior, no unique mechanics beyond class bonus.
- **God Power (Pet)** - Fire element, Adventurer class, 0.53% * CL all campaigns. Token gives GP from feeding. Campaign-focused, no dungeon combat mechanics.
- **Afky Clone** - Neutral element, Adventurer class, 0.6% * CL all campaigns. Campaign bonus scaling from Afky God minigame. No dungeon combat mechanics.
- **Stone/Golem** - Earth element, starts at -100% campaign penalty, scales to +100% at full growth. Challenge Point upgradeable. No dungeon combat mechanics.
- **Anteater** - Earth element, Blacksmith class, 0.7% * CL crafting speed. Ants found in all dungeons but this is a material drop, not a combat mechanic.
- **Living Draw** - Neutral element, Adventurer class, 0.8% * CL all campaigns. Item campaign bonus scaling with lucky draws. No dungeon combat mechanics.
