# Info about Pet Dungeons and Evolving

## When should I start dungeons and which dungeons should I do?

It is best to start dungeons as soon as you have 6 pets. First your pets are weak and you need to go into the newbie dungeon until they are about level 9-10. Even if they die at the beginning, they still keep the experience and will become stronger until they will eventually survive even 12 hours in a dungeon.

After your first team of 6 pets is level 9 or 10, you can start doing other dungeons. First you might want to do 1 room dungeons and if they survive, you can do longer dungeons. If you don't have time for that, you can also do longer runs in the beginning, the pets will automatically restart after they die, but they need a 1 hour break in between tries.

The recommended first dungeon is Mountain. It gives a good extra growth to your pets if you use 2 wind pets in the team and two of the first pets you likely get are wind pets. You keep all rewards from events even if your pets die afterwards.

## What should be first pets to use for dungeons and which pets to evolve into which class in the beginning?

Pets which only require tier 1 materials are the best pets to start out with. They are Frog (Supporter), Egg/Chicken (Assassin), Rabbit (Mage), Squirrel (Rogue), Armadillo (Defender), Bee (Alchemist) and Mouse, Bug, Mole, Cupid, Camel (One of them should be a blacksmith, otherwise you can also use different classes for them, they are often used as jack of all trades, not good at anything, but can be used for anything). The Ghost is also good in dungeons, even not evolved but harder to evolve.

## How is damage calculated?

Example for fire, the same applies for other elements. Neutral elemental pets will always attack with the element which has the highest difference between attacker element and target element.

ElementMod = (1 + FireValue of attacker / 100) / (1 + FireValue of target / 100). If the FireValue of the defender is < 0, then it will be changed to 0 and the negative value added to the FireValue of the attacker.

TargetDefMulti = 1 - TargetDefense / (TargetDefense + 200).

SpeedDamage = (Speed - Target Speed) / 2, can't be less than 0. For mages the speed damage is divided by 3 but they hit multiple times.

Damage = (AttackerAttack - TargetDefense / 2) * ElementMod * TargetDefMulti * Additional Multipliers + SpeedDamage.

Additional Multipliers are things like the supporter damage reduction, blacksmith bonus, assassin or mage class multiplier, events or traps.

## How are the turn order and actions calculated?

A pet has at base 1 action. Each speed gives 0.2% chance for an extra action. If a pet has more than 500 speed, it gives two actions and 0.1% for each speed above 500 chance for a third action. The turn order depends on the speed of a pet +/- 20%. Traps, events or skills can influence that. That results in ini, the number shown in dungeon logs at the beginning before each line and a higher number means a faster turn order.

Each action after the first one will halve the ini of the next action. For example a pet with 3 actions and 1500 speed will have the first action at ini ~1500, the second action at ini ~750 and the third action at ini ~375.

## How does the element system work?

Each pet has elemental stats which can be increased by gear. Each element is also strong against another element. Wind beats Earth, Earth beats Water, Water beats Fire, and Fire beats Wind. In general, when going into a dungeon, you want elemental stats of the dungeon type for defense and stats of the dungeon type counter for offense. For example, in the Volcano you should have fire stats especially on your Defender, and water pets with water gear, especially on your damage dealers.

Independent of the dungeon, damage dealers in general should mostly use their own element to increase damage while for the defensive they can wear other elements. Sometimes Inferno Swords are also good for damage dealers because of the raw attack stat.

## Do normal pet stats or growth influence dungeon stats?

Normal stats have no influence in dungeon stats. The total growth of a pet influences them slightly. Every 2000 total growth increases the dungeon stats by 1%. So a pet with 20,000 growth has 10% higher stats than a pet with 0 growth.

## What difficulty should I choose?

Start out with difficulty 0. If you are strong enough to survive 12 hours without losses, then try difficulty 1. You can try that sooner but it might be risky. Monsters in higher difficulties are harder and give more rewards. Depth 2 in difficulty 0 is about as hard as depth 1 in difficulty 8-10 and Depth 3 in difficulty 0 is about as hard as depth 2 in difficulty 8-10.

## What does sending items with your pets do?

Pets will automatically use healing potions when they are damaged. Other items can be used to bypass traps or complete events.

## What are events?

Events have a set chance to randomly occur inside dungeons. If you meet the requirements for an event something good will happen, if you do not meet the event requirements something bad will happen. You can refer to the in-game tooltips to find the exact requirements.

## How do you evolve a pet?

Evolving a pet requires 3 things. First you need a certain amount of growth which varies for each pet. Second, you need a certain amount of materials from dungeons that in general is more the more growth is required. Third, you need to complete a condition that is different for each pet such as giving the mouse 100 puny food.

## Can you skip the condition with a Pet Token?

Using a Pet Token allows you to skip the special condition for most pets. You still need to have the required growth and dungeon materials to use a token to evolve them. Some special pets, like the elementals who have a questline, can't be skipped with a token. If you try to evolve a pet, have enough growth and base materials, but did not meet the special condition, the game would always ask if you want to skip it the special condition with a token.

## What does evolving pets do?

When you evolve a pet you can give it one of eight different classes. These classes make pets more effective at various things. Adventurers get a bonus to campaigns, Blacksmiths create equip, Alchemists make potions and other useful things for your pets, Defenders soak up damage in dungeons, Supporters heal and reduce the damage your pets take (after class level 10), Rogues give you more loot, Assassins deal high single target damage, and Mages deal AoE damage.

## What are class levels?

After you evolve a pet and choose a class it will gain class xp by doing its specialty. Most pets gain class xp from fighting in dungeons, Alchemists gain xp by crafting, and Adventurers gain class xp by campaigning. Blacksmiths have a lot of usability and are the only class which gains experience in both dungeons and from crafting.

## Can you have multiple classes?

Each pet can only have one class, but any pet can have any class. Pets usually have a class they are best at, but that might not always be the best choice, for example if it is good in a campaign you need it instead. The class bonus is just a little extra and if a pet chooses a different class, the loss isn't that high.

## Can you change classes?

Yes, but you need to pay 1000 tier 2 items of the same element. You can skip the class change cost with a pet token. Changing classes will cause the pet to lose their class experience and half of it is sent to the free exp pool. The exception to this is the Holy Book which can change classes at will without cost or penalty. You can also use a Class Change Token to change a class without any penalty. For every 10 pets you evolve you will receive one Class Change Token. That should cover some mistakes you might make, but it is also possible to buy more with Pet Stones.

## What are gems and what can I do with them?

You can find gems from challenge dungeons. Your blacksmiths can embed gems to any equip of your choice. They can supplement the stats of your equip and cover up weaknesses.

## What equip should I forge for my pets?

Mages or Assassin usually want their own element as equip for the highest damage potential. Gems should be as needed for survival and damage. A fire mage probably wants mostly hp gems while an earth mage wants attack and speed gems. They need gems most out of all classes so you should embed gems to their equip first.

Supporters want speed and attack, so fire and wind equip.

Defenders want a metal armor and accessory and either a pot or knives for the weapon. A metal weapon is also fine but the knives are usually better for some extra damage.

Rogues want knives as a weapon and armor and accessory mostly for speed and survival.

## How do infinity towers work?

Infinity towers have enemies who get harder every floor and there is no end. You can earn points depending on the floor you have beaten and you can buy a party slot and T4 materials from the points. The floors also drop T3 materials (1% drop chance * floor) and T4 materials (0.05% chance * floor). The drop rate is set and can't be increased by rogues, talismans or other boosts. The floors can get hard and might require very different strategies than normal dungeons to beat higher floors.

Each floor will take one hour. If you lose the battle in a floor, your team will rest one hour and then resume at one floor below to continue until the time is up. You still keep the points from all the floors you have beaten even if you die afterwards. At default you will always go up by one floor after you beat one but with pet stones you can get an upgrade to farm the same floor without going up.

The infinity towers are balanced for teams who can do normal dungeons D3-10 with mimics. So it is advisable to not invest much into the towers before that.

The special abilities from Elephant, Undine and Hourglass work in the towers, but Undines ability will only work if Hourglass is not in the same team.
