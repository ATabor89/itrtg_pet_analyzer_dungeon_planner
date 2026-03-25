# Batch 1 Pet Research Findings

Research from ITRTG wiki pages for 25 pets. Focus on dungeon-relevant special mechanics.

---

## Frog
- **Element:** Ice (evolution requires Ice materials)
- **Special Mechanics:** Supporter class bonus - increases heal power and buff effect for dungeons by an extra 0.5 * class level
- **Stat Modifiers:** None documented
- **Synergies:** Works well in supporter-heavy teams that rely on healing/buffing
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Not documented
- **Class-Specific:** Favors Supporter class; bonus only activates as Supporter
- **Constraint Implications:** Planner should flag this pet as optimal for Supporter class; bonus is wasted in other classes

---

## Rabbit
- **Element:** Earth
- **Special Mechanics:** Mage class bonus - increases multi-target attack damage in dungeons by 0.51% * class level as Mage
- **Stat Modifiers:** Earth equipment provides speed bonus: 5% * equipment tier per Earth gear piece. Flat +50% attack bonus from all Earth equipment. Fire gems in Earth gear also get this bonus.
- **Synergies:** Strong in Water Temple (Earth advantage vs water enemies). Used in Earth Tower lategame.
- **Anti-synergies:** Weak in Forest dungeons at Depth 4 (replaced by Basilisk/Todlicher Loffel)
- **Equipment Notes:** PRIORITY OVERRIDE - Should use Earth equipment with Fire gems for maximum attack bonus. Tier 3 Earth armor with level 10 Fire gem grants 45% additional attack instead of standard 30%.
- **Token Improvement:** Grants the Earth equipment scaling described above
- **Constraint Implications:** Major equipment priority override. Planner must prioritize Earth element equipment for Rabbit. Fire gems should be slotted into Earth gear. Element-equipment synergy is a unique mechanic not shared by most pets.

---

## Cat
- **Element:** Neutral
- **Special Mechanics:** Assassin class bonus - increases single-target attack damage by 0.51% * class level as Assassin
- **Stat Modifiers:** None base
- **Synergies:** Useful in D3 Mountain (neutral pet requirements)
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Three significant effects:
  1. **Frozen Aura Negation** - negates frozen aura of Ancient Mimics
  2. **Summoned Enemy Autokill** - instantly kills summoned/replicated enemies if their HP < Cat's Attack x 2
  3. **Chain Action** - killing a summoned enemy grants Cat an extra action next turn plus 40% speed damage bonus for current room
- **Constraint Implications:** Token-improved Cat is extremely valuable against Ancient Mimics and summoned enemies. Planner should note Cat's unique role in dungeons with Ancient Mimics (e.g., D4 Scrapyard for Nanobots). Attack stat threshold matters for autokill mechanic.

---

## Armadillo
- **Element:** Not specified
- **Special Mechanics:** Defender class bonus - increases damage taken instead of party member by 0.5% * class level; +10% protect chance as Defender
- **Stat Modifiers:** +100% critical chance during TBS when evolved
- **Synergies:** Works with Defender class teams needing a tank
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Not documented
- **Constraint Implications:** Defender-class-specific tank pet. The protect chance bonus only works as Defender class.

---

## Dog
- **Element:** Not specified
- **Special Mechanics:** Defender class bonus - 0.53% defense multiplier per class level; 10% increased chance to absorb damage for party members (Defender only)
- **Stat Modifiers:** None for dungeon combat directly
- **Synergies:** Defender class tank teams
- **Anti-synergies:** Has a campaign penalty (-50% multiplier campaign effectiveness)
- **Equipment Notes:** None
- **Token Improvement:** Not documented
- **Constraint Implications:** Another Defender-specific tank. Similar to Armadillo but with defense multiplier instead of damage absorption scaling.

---

## Fairy
- **Element:** Not specified
- **Special Mechanics:** Supporter class bonus - increases heal power and buff effect by 0.55% * class level as Supporter
- **Stat Modifiers:** None
- **Synergies:** Supporter-heavy teams for healing
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Once per dungeon round, prevents a single pet death from one attack (leaving pet at 1 HP). Does NOT block subsequent hits in the same round or multi-attack scenarios.
- **Constraint Implications:** Token-improved Fairy provides a unique death prevention mechanic. Planner should note this is a once-per-round effect and doesn't work against multi-hit attacks. Valuable as a safety net in harder dungeons.

---

## Dragon
- **Element:** Fire (evolution requires Fire Stones)
- **Special Mechanics:** Mage class bonus - increases multi-target attack damage by 0.57% * class level as Mage
- **Stat Modifiers:** None documented for dungeons
- **Synergies:** None documented
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Not documented
- **Constraint Implications:** Straightforward Mage pet with higher scaling than Rabbit/Slime (0.57% vs 0.51%/0.62%). No special constraints.

---

## Shark
- **Element:** Not specified (likely Water given thematic)
- **Special Mechanics:** Assassin class bonus - increases single-target attack damage by 0.6% * class level as Assassin
- **Stat Modifiers:** None documented
- **Synergies:** None documented
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Not documented
- **Constraint Implications:** Straightforward Assassin pet with 0.6% scaling. Higher than Cat/Egg-Chicken.

---

## Valkyrie
- **Element:** Not specified
- **Special Mechanics:** Defender class bonus - increases damage taken instead of party member by 0.61% * class level; +10% protect chance as Defender
- **Stat Modifiers:** None documented
- **Synergies:** Defender class teams
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Not documented
- **Constraint Implications:** Higher-tier Defender pet (0.61% scaling). Evolution requires 2,000 total dungeon bosses defeated.

---

## Slime
- **Element:** Not specified
- **Special Mechanics:** Mage class bonus - increases multi-target attack damage by 0.62% * class level as Mage
- **Stat Modifiers:** None documented
- **Synergies:** None documented
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Not documented
- **Constraint Implications:** Highest mage scaling in this batch (0.62%) aside from Balrog. No special constraints.

---

## Whale
- **Element:** Not specified (likely Water)
- **Special Mechanics:** Defender class bonus - increases damage taken instead of party member by 0.65% * class level; +10% protect chance as Defender
- **Stat Modifiers:** None documented
- **Synergies:** Defender class teams
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Not documented
- **Constraint Implications:** Highest Defender scaling in this batch (0.65%). Same protect chance bonus as Armadillo/Valkyrie.

---

## Lucky Coin
- **Element:** Not specified
- **Special Mechanics:**
  1. **True Damage** - deals extra damage after attacks that ignores defense and multipliers. Proc chances: 7 dmg (30%), 77 dmg (50%), 777 dmg (10%), 7777 dmg (10%, evolved only)
  2. **Drop Rate Scaling** - boosts dungeon loot based on total cumulative damage dealt: (total_damage^0.3 / 10)%, caps at 100% at 10 billion total damage
- **Stat Modifiers:** Assassin bonus 1.5% per CL (base), upgraded to 2% per CL with token
- **Synergies:** Used in D4 Forest for wood generation. Used in Balrog Horn farming teams.
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Gains Rogue bonus (1.28% per CL) AND improves Assassin bonus to 2% per CL. This means it benefits from BOTH Assassin and Rogue class bonuses.
- **Constraint Implications:** Very unique pet - true damage mechanic bypasses normal combat calculations. Dual-class bonus (Assassin + Rogue) is unusual. Drop rate scaling based on cumulative damage makes it increasingly valuable over time. Planner should note this pet's value increases with long-term use.

---

## Balrog
- **Element:** Fire (thematic)
- **Special Mechanics:**
  1. **HP Scaling from Horns** - HP multiplier based on Horns of Balrog owned: base 10% + 0.1% per horn up to 900, then 100% + (Horns-900)^0.8 * 0.1% above 900
  2. **Strategy Room HP** - HP divided by 2.25 in Strategy Room to counteract horn farming
  3. **Self-Healing** - Evolved Balrog has large self-heal that can be PREVENTED by including Wizard pet in team
- **Stat Modifiers:** Massive HP scaling based on horn count
- **Synergies:** Wizard pet prevents Balrog's self-heal (important for fighting Evolved Balrog as enemy)
- **Anti-synergies:** None for team composition
- **Equipment Notes:** Shiny Stone and Magic Ore weaken Evolved Balrog encounters; Shiny Stone reduces multi-target attack chance
- **Token Improvement:** Not documented for player's Balrog
- **Class-Specific:** Mage variant provides 1.4% * class level multi-target damage (highest mage scaling in batch)
- **Constraint Implications:** Complex pet. Horn count is a major progression variable affecting HP. When fighting Evolved Balrog as dungeon boss, bringing Wizard pet and Shiny Stone/Magic Ore items is important. As player pet, Mage class gives extremely high 1.4% scaling. Planner needs to track horn count for accurate HP calculations.

---

## Gray
- **Element:** Not specified
- **Special Mechanics:**
  1. **Cannot directly participate in dungeons** - base Gray cannot be in dungeon teams
  2. **Grayling Children** - evolved Gray produces children that CAN be in dungeons
  3. **First child classes:** Adventurer, Blacksmith, or Alchemist
  4. **Token upgrade second child classes:** Assassin, Supporter, Rogue, Mage, Defender
  5. **XP sharing** - children send 60% of earned XP to Mother Gray; children share dungeon level with mother
  6. **Class flexibility** - children can switch class and element freely outside active combat
- **Stat Modifiers:** Children receive 0.8% * CL bonus scaling
- **Synergies:** Children benefit from each other's XP gains (effectively 120% XP each)
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Enables second child with full class access (Assassin, Supporter, Rogue, Mage, Defender)
- **Constraint Implications:** MAJOR constraint - Gray herself cannot be in dungeons. Only her Grayling children can. First child has limited class options; second child (token) has full options. Planner needs special handling: Gray is not a dungeon pet, but her children are. Children have flexible class/element switching.

---

## Crocodile
- **Element:** Not specified
- **Special Mechanics:**
  1. **Stun** - 10% chance to stun enemies for 3 actions per attack
  2. **Rogue class bonus** - increases drop rate by 2% * class level as Rogue
- **Stat Modifiers:** None documented
- **Synergies:** Long combat encounters maximize stun proc chance
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Not documented
- **Constraint Implications:** Unique crowd control mechanic (stun). Planner should note stun utility for harder dungeons. Evolution requires 3,333 enemy stuns. High rogue scaling (2% per CL).

---

## Pandora's Box
- **Element:** Not specified
- **Special Mechanics:** Campaign multiplier pet - multiplies overall campaign result (total of all pets' contributions). Scales with growth: 3% per 5,000 growth up to 100k, then 4% per 5,000. Feeding adds up to 2% (0.1% per feed, max 20). Adventurer bonus: 0.9% * CL.
- **Anti-synergies:** **MUST NOT be used with Mimic** - bonus turns into penalty when paired with Mimic
- **Constraint Implications:** CRITICAL anti-synergy with Mimic. Planner must enforce this constraint. Primarily a campaign pet, not a direct dungeon combat pet. Evolution requires P.Baal v66 in Day Baal Challenge (cannot be bypassed with tokens).

---

## Egg/Chicken
- **Element:** Not specified
- **Special Mechanics:** Assassin class bonus - increases single-target attack damage by 0.5% * class level as Assassin
- **Stat Modifiers:** None documented
- **Synergies:** None documented
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Not documented
- **Constraint Implications:** Basic Assassin pet. Low scaling (0.5%). Evolution requires "Chicken God/Goddess" title from fleeing Ultimate Being v2.

---

## Ant Queen
- **Element:** Not specified
- **Special Mechanics:**
  1. Increases ants found in dungeons by 0.5% * class level
  2. Evolved: uncaps god power found in god power campaigns (shared with Nightmare pet)
  3. Campaign bonus scales with ant count: (ants^0.27)% in divinity and god power campaigns
  4. Adventurer class bonus: 2% * CL for all campaigns
- **Stat Modifiers:** None for dungeon combat
- **Synergies:** Shares god power uncap ability with Nightmare (freeing either for other roles)
- **Anti-synergies:** None documented
- **Equipment Notes:** None
- **Token Improvement:** Not documented
- **Constraint Implications:** Primarily a campaign/farming pet. Ant collection in dungeons is a passive benefit. Evolution requires 10 million ants from dungeon exploration.

---

# Pets with No Special Dungeon Constraints

The following pets have only basic class bonuses and no special mechanics, equipment overrides, synergies, or constraints worth noting for the planner:

- **Mouse** - Evolution wildcard (class-independent bonuses). No dungeon mechanics documented.
- **Bee** - Alchemist crafting speed bonus only. No dungeon combat mechanics.
- **Cupid** - Adventurer campaign bonus only. No dungeon combat mechanics.
- **Snake** - Alchemist crafting speed bonus only. No dungeon combat mechanics.
- **Octopus** - Blacksmith crafting bonus only. No dungeon combat mechanics.
- **Hydra** - Adventurer campaign bonus only (0.7% * CL). No dungeon combat mechanics.
- **Squirrel** - Rogue drop rate bonus (0.5% * CL). Token adds acorn collection scaling drop rate. Minimal combat impact.

---

# Summary of Key Planner Constraints Identified

1. **Equipment Priority Override:** Rabbit should prioritize Earth equipment with Fire gems (unique element scaling)
2. **Anti-synergy Rule:** Pandora's Box MUST NOT be paired with Mimic (bonus becomes penalty)
3. **Cannot Dungeon:** Gray cannot participate in dungeons directly; only her Grayling children can
4. **Token-Gated Mechanics:** Cat (autokill/chain action), Fairy (death prevention), Lucky Coin (dual-class bonus), Gray (second child)
5. **Unique Combat Mechanics:** Lucky Coin (true damage), Crocodile (stun), Cat (autokill summoned enemies), Fairy (death prevention)
6. **Scaling Variables:** Balrog HP scales with horn count; Lucky Coin drop rate scales with cumulative damage; Ant Queen scales with ant count
7. **Class-Favored Pets:**
   - Supporter: Frog (0.5%), Fairy (0.55%)
   - Assassin: Egg/Chicken (0.5%), Cat (0.51%), Shark (0.6%), Lucky Coin (1.5%/2.0%)
   - Mage: Rabbit (0.51%), Dragon (0.57%), Slime (0.62%), Balrog (1.4%)
   - Defender: Armadillo (0.5%+10% protect), Dog (0.53%+10% protect), Valkyrie (0.61%+10% protect), Whale (0.65%+10% protect)
   - Rogue: Squirrel (0.5%), Crocodile (2.0%), Lucky Coin (1.28% token)
   - Adventurer: Hydra (0.7%), Pandora's Box (0.9%), Ant Queen (2.0%)
   - Alchemist: Bee (0.5%), Snake (0.6%)
   - Blacksmith: Octopus (0.65%)
