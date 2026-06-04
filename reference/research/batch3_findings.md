# Batch 3 Pet Research Findings
# Pets: Rudolph, Santa, Elf, Pumpkin, Ghost, Nightmare, Question, Chocobear, Rose, Doughnut, Eagle, Penguin, Hermit Crab, Hedgehog, Phoenix, Wizard, Pegasus, Panda, UFO, Robot, Otter, FSM, Elephant, Vaccina, Corona

---

## Ghost
- **Element:** N/A (no element documented)
- **Special Mechanics:** Uses "Scare" instead of normal attacks. At the beginning of each dungeon round (always goes first, independent of speed), has a chance to reduce enemy attack and defense by 50% (30% for bosses). Success formula: Ghost attack / enemy defense * 0.6. Ghost deals NO damage itself. At Class Level 20, gains Twin Scare (two scare attempts per action, can target 2 enemies).
- **Stat Modifiers:** Scared enemies receive increased targeting priority from other pets.
- **Synergies:** Works well with high-damage pets since Scare debuffs enemy defense by 50%, amplifying all other pet damage. Pairs well with any DPS-heavy team.
- **Anti-synergies:** None documented, but being a non-damage pet means it takes a team slot without contributing direct damage.
- **Equipment Notes:** Cannot use knives. Recommended: Inferno Sword, Ego Sword, or T4 equivalents. Ghost relies on attack stat (for Scare success chance) rather than damage output.
- **Token Improvement:** Not documented.
- **Class-Specific:** Rogue class gains 0.65% extra item drop rate per Ghost's class level in dungeons.
- **Constraint Implications:** Ghost should always act first (speed-independent). Planner should note Ghost cannot equip knives. Ghost is a debuffer, not a damage dealer -- team needs other DPS. The Scare mechanic makes Ghost uniquely valuable for harder content where enemy stats are high.

## Nightmare
- **Element:** N/A
- **Special Mechanics:** All campaigns +200%, but reduces reward from other pets in the same campaign by (20 - 0.25 * class level)%. Removes reward cap in god power campaigns. Penalty reduction caps at CL76, with minimum floor of 1% reduction.
- **Stat Modifiers:** Massive campaign bonus (+200%) but penalizes teammates.
- **Synergies:** Best used in campaigns, not dungeon combat. God power campaign uncapping is unique.
- **Anti-synergies:** Penalizes all other pets in the same campaign. The penalty decreases with class level but never fully disappears (1% minimum).
- **Equipment Notes:** Evolution requires a +20 grade SSS Magic Stick (persists through evolution).
- **Token Improvement:** Not documented separately.
- **Class-Specific:** Adventurer class gains 0.9% bonus per class level to all campaign rewards.
- **Constraint Implications:** Campaign team planner should warn about the team penalty. Not primarily a dungeon pet. The penalty mechanic means Nightmare is best solo or with expendable pets in campaigns.

## Wizard
- **Element:** N/A
- **Special Mechanics:** Increases multi-target attack damage in dungeons by 0.63% * class level for Mage class. CRITICAL: When included in team against Evolved Balrog, the boss loses its healing ability. Recommended for first Balrog kill, then can be swapped out for higher-tier pets afterward.
- **Stat Modifiers:** -50% divinity campaigns, +75% multiplier campaigns.
- **Synergies:** Essential for Evolved Balrog fight (prevents boss healing). Pairs well with Mage class multi-target builds.
- **Anti-synergies:** Campaign penalty in divinity campaigns.
- **Equipment Notes:** None documented.
- **Token Improvement:** Not documented.
- **Class-Specific:** Mage class gets multi-target dungeon damage boost.
- **Constraint Implications:** Planner should flag Wizard as required/recommended for Evolved Balrog dungeon boss. This is a hard constraint -- without Wizard, Balrog heals. After first kill, constraint relaxes. Required for "Wizard In Love" quest (Tavern level 9).

## Elephant
- **Element:** N/A
- **Special Mechanics:** In dungeons, all enemies who attack Elephant get burned -- taking 3% of their max HP per action for 5 actions (1.5% for bosses). Absorbs additional dungeon damage for party members, scaling at 0.8% per class level. 10% boost to protect chance when class is Defender.
- **Stat Modifiers:** Pure dungeon pet -- no campaign bonuses at all.
- **Synergies:** Works well in Defender class teams. Burn effect is passive and triggers on being attacked, synergizing with tank role. Pairs well with other DPS pets who benefit from Elephant absorbing damage.
- **Anti-synergies:** None documented.
- **Equipment Notes:** None documented. Focus on defensive stats to maximize tanking.
- **Token Improvement:** Not documented.
- **Class-Specific:** Defender class gets 10% protect chance boost.
- **Constraint Implications:** Elephant is a dedicated tank/burn pet. Planner should position it to absorb hits. Burn damage is % based, making it scale well against high-HP enemies. No campaign value -- purely dungeon-focused.

## Robot
- **Element:** N/A
- **Special Mechanics:** +100% growth campaigns, +100% food campaigns. Blacksmith class: 0.75% * class level crafting speed/quality boost.
- **Stat Modifiers:** Strong campaign bonuses.
- **Synergies:** None documented for dungeons.
- **Anti-synergies:** CRITICAL: Robot must NOT be in the same campaign as Thunder Ball (Raiju). Thunder Ball's lightning causes Robot to malfunction, applying a heavy penalty to campaign results. This is a one-directional conflict -- Thunder Ball can grow Robot, but Robot cannot be in campaigns with Thunder Ball.
- **Equipment Notes:** None documented.
- **Token Improvement:** Crafting speed and quality for T4/T5 equipment increases by growth^0.121 * class level%.
- **Class-Specific:** Blacksmith class crafting bonus.
- **Constraint Implications:** Planner MUST enforce the Robot + Thunder Ball anti-synergy in campaigns. This is a hard constraint. The asymmetric nature (Thunder Ball can grow Robot but not vice versa) is unusual and should be noted.

## Corona
- **Element:** N/A
- **Special Mechanics:** 5% chance per hour to cause sickness to one pet. Sick pets spread sickness to others (5% hourly transmission) and lose 1 total growth per hour above 1500 total growth (floor of 500). Reduces campaign gains by 30% normally, increasing to 50% after evolution.
- **Stat Modifiers:** -30% campaign penalty (base), -50% after evolution. Growth loss from sickness mechanic.
- **Synergies:** Paired mechanically with Vaccina (creates vaccines for permanent immunity). Vaccina evolution requires Corona to be evolved first.
- **Anti-synergies:** Sickness spreads to ALL nearby pets. Campaign penalty affects the whole team.
- **Equipment Notes:** None documented.
- **Token Improvement:** Converts the campaign penalty into a +50% bonus for all campaigns. This is a dramatic reversal -- from -50% to +50%.
- **Class-Specific:** Adventurer class gains 0.7% * class level campaign bonus.
- **Constraint Implications:** Without token upgrade, Corona is actively harmful to campaigns. Sickness mechanic affects pet management broadly. Token upgrade is critical -- completely reverses the penalty. Vaccina evolution depends on Corona being evolved. Planner should warn about sickness risk to other pets.

## Vaccina
- **Element:** N/A
- **Special Mechanics:** +150% divinity campaigns. Creates Cure Potions that remove sickness. Vaccines grant permanent immunity.
- **Stat Modifiers:** Strong divinity campaign bonus.
- **Synergies:** Paired with Corona -- manages Corona's sickness mechanic by producing cures and vaccines.
- **Anti-synergies:** None documented.
- **Equipment Notes:** None documented.
- **Token Improvement:** Not documented.
- **Class-Specific:** Alchemist class: 0.65% * class level item creation speed.
- **Constraint Implications:** Evolution requires Corona to be evolved first. This is a dependency chain: Corona evolution -> Vaccina evolution. Planner should track this prerequisite.

## Santa
- **Element:** N/A
- **Special Mechanics:** Supporter class: increases heal power and buff effect for dungeons by 0.7% * class level. Can function as Angelic Supporter for Gram fight (Lucifer/Sandalaphon). Nothing-to-Chocolate conversion: each Nothing yields 1 + level/8 chocolates (max 6), up to 100 Nothings/day, caps at CL40.
- **Stat Modifiers:** +50% item campaigns, +50% food campaigns.
- **Synergies:** Can fill Angelic Supporter role in specific boss fights.
- **Anti-synergies:** Outclassed by Salamander (Fire Support) and Tanuki (Neutral Supporter) in most dungeon roles.
- **Equipment Notes:** None documented.
- **Token Improvement:** Not documented.
- **Class-Specific:** Supporter class heal/buff scaling.
- **Constraint Implications:** After CL40, Santa's chocolate conversion is maxed and wiki suggests reassigning to Material Factory. Planner should note Santa is generally outclassed by Salamander and Tanuki for dungeon support roles.

## Rudolph
- **Element:** N/A
- **Special Mechanics:** Rogue class: increases dungeon item drop rate by 0.55% * class level.
- **Stat Modifiers:** +35% rare items/pet stones in item campaigns, +35% food/quality in food campaigns.
- **Synergies:** None documented.
- **Anti-synergies:** None documented.
- **Equipment Notes:** None documented.
- **Token Improvement:** Not documented.
- **Class-Specific:** Rogue-exclusive dungeon item drop bonus.
- **Constraint Implications:** Only relevant for Rogue class dungeon farming. Non-rogue players get no dungeon benefit.

## Penguin
- **Element:** N/A
- **Special Mechanics:** Assassin class: increases single-target attack damage in dungeons by 0.53% * class level.
- **Stat Modifiers:** +75% god power campaigns.
- **Synergies:** Best with Assassin class single-target builds.
- **Anti-synergies:** None documented.
- **Equipment Notes:** None documented.
- **Token Improvement:** Not documented.
- **Class-Specific:** Assassin single-target damage boost.
- **Constraint Implications:** Class-restricted bonus. Only useful in dungeons for Assassin class. Single-target only -- no multi-target benefit.

## Elf
- **Element:** N/A
- **Special Mechanics:** Blacksmith class: 0.9% * class level crafting speed/quality. Crafting bonus from equipped items: T3 0.1%/piece, T4 0.2%/piece, T5 0.3%/piece (cap 2000 pieces). Dwarf's crafting bonus is limited by Elf's bonus.
- **Stat Modifiers:** +150% food campaigns, +75% item campaigns.
- **Synergies:** Dwarf's crafting is capped by Elf's level -- Elf is a prerequisite for maximizing Dwarf.
- **Anti-synergies:** None documented.
- **Equipment Notes:** None documented.
- **Token Improvement:** Not documented.
- **Class-Specific:** Blacksmith crafting bonus.
- **Constraint Implications:** Elf gates Dwarf's crafting potential. If using Dwarf for crafting, Elf must be leveled first/alongside.

---

## Pets with No Special Dungeon Constraints

The following pets have no special mechanics that significantly affect dungeon team planning. They are primarily campaign/crafting utility pets:

- **Pumpkin** - Campaign-only pet (chocolate finding in food campaigns, Adventurer class). No dungeon combat mechanics.
- **Question** - Alchemist class item creation speed bonus (0.51% * CL). +42% to multiple campaign types. No dungeon mechanics.
- **Chocobear** - Campaign multiplier pet (+50% all campaigns from banked hours, Adventurer class). No dungeon mechanics.
- **Rose** - Alchemist class item creation speed (0.75% * CL). +25% enchant speed. Campaign bonuses only.
- **Doughnut** - Food campaign specialist (Mighty Food finding). No dungeon mechanics.
- **Eagle** - Adventurer class all-campaign bonus (0.52% * CL). +75% item campaign rare items. No dungeon mechanics.
- **Hermit Crab** - Blacksmith crafting bonus (0.51% * CL). +50% multiplier campaigns. Described as "kind of weak."
- **Hedgehog** - Adventurer class all-campaign bonus (0.58% * CL). +25% divinity/growth campaigns (141% boost with token). No dungeon mechanics.
- **Phoenix** - Alchemist class item creation speed (0.6% * CL). +50% pet levels in level campaigns. Thematic revival mechanic mentioned but no implementation details for dungeons.
- **Pegasus** - Blacksmith crafting bonus (0.7% * CL). +40% pet levels in level campaigns. No dungeon mechanics.
- **Panda** - Supporter class heal/buff bonus (0.67% * CL). +300% food campaigns. Standard supporter scaling, no unique mechanic.
- **UFO** - Adventurer class all-campaign bonus (0.7% * CL). +75% multiplier campaigns. No dungeon mechanics.
- **Otter** - Adventurer class all-campaign bonus (0.8% * CL). +75% growth, +125% multiplier campaigns. Gives random dungeon materials when fed. No dungeon combat mechanics.
- **FSM** - Adventurer class all-campaign bonus (0.85% * CL). Doubles god power for all campaign members. Divinity generator bonus of log2(Total Stats)%. No dungeon combat mechanics.
