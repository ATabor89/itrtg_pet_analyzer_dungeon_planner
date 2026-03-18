import { useState, useMemo, useCallback, useRef, useEffect } from "react";

// ─── WIKI DATABASE (153 pets from wiki source) ──────────────────────────────
const DB = [
  { name: "Mouse", element: "Earth", recClass: "Wildcard", classBonus: "+50% to food camps", unlock: "Defeat Gods", evoDiff: "1(1)", improve: false, special: "-" },
  { name: "Frog", element: "Water", recClass: "Supporter", classBonus: "0.5% x CL", unlock: "Defeat Gods", evoDiff: "1(1)", improve: false, special: "-" },
  { name: "Bee", element: "Wind", recClass: "Alchemist", classBonus: "0.5% x CL", unlock: "Defeat Gods", evoDiff: "1(1)", improve: true, special: "-" },
  { name: "Cupid", element: "Wind", recClass: "Adventurer/Wildcard", classBonus: "0.5% x CL", unlock: "Defeat Gods", evoDiff: "1(2)", improve: true, special: "-" },
  { name: "Egg/Chicken", element: "Wind", recClass: "Assassin", classBonus: "0.5% x CL", unlock: "Defeat Gods", evoDiff: "1(1)", improve: false, special: "-" },
  { name: "Armadillo", element: "Earth", recClass: "Defender", classBonus: "0.5% x CL", unlock: "Defeat Gods", evoDiff: "1(2)", improve: false, special: "-" },
  { name: "Squirrel", element: "Fire", recClass: "Rogue", classBonus: "0.5% x CL", unlock: "Defeat Gods", evoDiff: "1(1)", improve: true, special: "-" },
  { name: "Rabbit", element: "Earth", recClass: "Mage", classBonus: "0.51% x CL", unlock: "Defeat P.Baal 5", evoDiff: "1(2)", improve: true, special: "-" },
  { name: "Cat", element: "Neutral", recClass: "Assassin", classBonus: "0.51% x CL", unlock: "Defeat P.Baal 10", evoDiff: "1(1)", improve: true, special: "-" },
  { name: "Dog", element: "Neutral", recClass: "Defender", classBonus: "0.53% x CL", unlock: "Defeat P.Baal 15", evoDiff: "2(2)", improve: false, special: "-" },
  { name: "Fairy", element: "Wind", recClass: "Supporter", classBonus: "0.55% x CL", unlock: "Defeat P.Baal 20", evoDiff: "2(3)", improve: true, special: "-" },
  { name: "Dragon", element: "Fire", recClass: "Mage", classBonus: "0.57% x CL", unlock: "Defeat P.Baal 25", evoDiff: "2(2)", improve: false, special: "-" },
  { name: "Snake", element: "Water", recClass: "Alchemist", classBonus: "0.6% x CL", unlock: "Defeat P.Baal 30", evoDiff: "3(5)", improve: false, special: "-" },
  { name: "Shark", element: "Water", recClass: "Assassin/Adventurer", classBonus: "0.6% x CL", unlock: "Defeat P.Baal 35", evoDiff: "3(4-5)", improve: false, special: "-" },
  { name: "Octopus", element: "Water", recClass: "Blacksmith", classBonus: "0.65% x CL", unlock: "Defeat P.Baal 40", evoDiff: "3(3)", improve: true, special: "-" },
  { name: "Valkyrie", element: "Wind", recClass: "Defender", classBonus: "0.61% x CL", unlock: "Defeat P.Baal 45", evoDiff: "3(5)", improve: false, special: "-" },
  { name: "Slime", element: "Water", recClass: "Mage", classBonus: "0.62% x CL", unlock: "Defeat P.Baal 50", evoDiff: "3(4-5)", improve: false, special: "-" },
  { name: "Whale", element: "Water", recClass: "Defender", classBonus: "0.65% x CL", unlock: "Defeat P.Baal 55", evoDiff: "3(4)", improve: false, special: "-" },
  { name: "Hydra", element: "Water", recClass: "Adventurer", classBonus: "0.7% x CL", unlock: "Defeat P.Baal 60", evoDiff: "3(4)", improve: false, special: "-" },
  { name: "Pandora's Box", element: "Neutral", recClass: "Adventurer", classBonus: "0.9% x CL", unlock: "Defeat P.Baal 66", evoDiff: "4(6)", improve: false, special: "Extra Camp Buff" },
  { name: "Lucky Coin", element: "Earth", recClass: "Assassin/Rogue", classBonus: "1.5% x CL", unlock: "Defeat P.Baal 77", evoDiff: "5(7)", improve: true, special: "Bonus Damage" },
  { name: "Balrog", element: "Fire", recClass: "Mage/Wildcard", classBonus: "1.4% x CL", unlock: "Defeat P.Baal 88", evoDiff: "5(7)", improve: false, special: "Bonus Health" },
  { name: "Gray", element: "Neutral", recClass: "Special", classBonus: "New Pets", unlock: "Defeat P.Baal 100", evoDiff: "6(9)", improve: true, special: "Children" },
  { name: "Gray Child 1", element: "Neutral", recClass: "Alchemist/Adventurer/Blacksmith", classBonus: "0.8% x CL", unlock: "Gray (Special)", evoDiff: "-", improve: false, special: "Gray's 1st child clone" },
  { name: "Gray Child 2", element: "Neutral", recClass: "Any Dungeon Class", classBonus: "0.8% x CL", unlock: "Gray (Token Upgrade)", evoDiff: "-", improve: false, special: "Gray's 2nd child clone (token req)" },
  { name: "Ant Queen", element: "Fire", recClass: "Adventurer", classBonus: "2% x CL", unlock: "Defeat P.Baal v125", evoDiff: "6(9)", improve: false, special: "Uncap GP Camp Limit, increases ant gain" },
  { name: "Crocodile", element: "Earth", recClass: "Rogue", classBonus: "2% x CL", unlock: "Defeat P.Baal v150", evoDiff: "7(9)", improve: false, special: "Stunning attacks" },
  { name: "Big Burger", element: "Water", recClass: "Adventurer", classBonus: "2% x CL", unlock: "Defeat P.Baal v175", evoDiff: "6(8)", improve: false, special: "Food Camp Conversion" },
  { name: "Oni", element: "Fire", recClass: "Adventurer", classBonus: "2% x CL", unlock: "Defeat P.Baal v200", evoDiff: "8(15)", improve: false, special: "P.Baal Weakening" },
  { name: "Bug", element: "Fire", recClass: "Adventurer/Wildcard", classBonus: "0.5% x CL", unlock: "Special Task", evoDiff: "1(1)", improve: false, special: "-" },
  { name: "Mole", element: "Earth", recClass: "Adventurer/Wildcard", classBonus: "0.5% x CL", unlock: "Special Task", evoDiff: "1(2)", improve: false, special: "-" },
  { name: "Camel", element: "Fire", recClass: "Adventurer/Wildcard", classBonus: "0.51% x CL", unlock: "Special Task", evoDiff: "1(1)", improve: false, special: "-" },
  { name: "Goat", element: "Earth", recClass: "Rogue/Adventurer", classBonus: "0.58% x CL", unlock: "Special Task", evoDiff: "2(4)", improve: false, special: "-" },
  { name: "Ape", element: "Fire", recClass: "Assassin/Adventurer", classBonus: "0.65% x CL", unlock: "Special Task", evoDiff: "3(6)", improve: true, special: "-" },
  { name: "Flying Cloud", element: "Wind", recClass: "Adventurer/Supporter", classBonus: "0.75% x CL", unlock: "Special Task", evoDiff: "4(5)", improve: false, special: "-" },
  { name: "Holy ITRTG Book", element: "Fire", recClass: "All Classes", classBonus: "0.4% x CL", unlock: "Special Task", evoDiff: "5(6)", improve: true, special: "Crit Immunity and Class Change" },
  { name: "God Power (Pet)", element: "Fire", recClass: "Adventurer/Wildcard", classBonus: "0.53% x CL", unlock: "Special Task", evoDiff: "1(2)", improve: true, special: "Receive Some GP when Fed" },
  { name: "Turtle", element: "Earth", recClass: "Defender", classBonus: "0.6% x CL", unlock: "Special Task", evoDiff: "3(9)", improve: false, special: "-" },
  { name: "Afky Clone", element: "Neutral", recClass: "Adventurer", classBonus: "0.6% x CL", unlock: "Special Task", evoDiff: "3(6)", improve: true, special: "-" },
  { name: "Stone/Golem", element: "Earth", recClass: "Adventurer", classBonus: "100% All campaigns", unlock: "Special Task", evoDiff: "4(4)", improve: false, special: "-" },
  { name: "Chameleon", element: "All", recClass: "Dungeon Wildcard", classBonus: "All Elements", unlock: "Special Task", evoDiff: "3(3)", improve: false, special: "Freely Change Element" },
  { name: "Undine", element: "Water", recClass: "Rogue", classBonus: "1.5% x CL", unlock: "Special Task", evoDiff: "5(4-7)", improve: false, special: "AoE Damage" },
  { name: "Gnome", element: "Earth", recClass: "Defender", classBonus: "1.5% x CL", unlock: "Special Task", evoDiff: "5(4-7)", improve: false, special: "AoE Party Shield" },
  { name: "Salamander", element: "Fire", recClass: "Supporter", classBonus: "1.5% x CL", unlock: "Special Task", evoDiff: "5(4-7)", improve: false, special: "AoE Heals Every Turn" },
  { name: "Sylph", element: "Wind", recClass: "Mage", classBonus: "1.5% x CL", unlock: "Special Task", evoDiff: "5(4-7)", improve: false, special: "Extra Attacks" },
  { name: "Aether", element: "Neutral", recClass: "Adventurer", classBonus: "1.5% x CL", unlock: "Special Task", evoDiff: "5(4-7)", improve: false, special: "Special Boss Fight Improves" },
  { name: "Anteater", element: "Earth", recClass: "Blacksmith", classBonus: "0.7% x CL", unlock: "Special Task", evoDiff: "3(5)", improve: false, special: "Crafting Improves with Ants" },
  { name: "Gold Dragon", element: "Neutral", recClass: "Alchemist", classBonus: "0.65% x CL", unlock: "Special Task", evoDiff: "3(3)", improve: false, special: "Distributes 25% Growth to All Pets" },
  { name: "Living Draw", element: "Neutral", recClass: "Adventurer", classBonus: "0.8% x CL", unlock: "Special Task", evoDiff: "4(6)", improve: true, special: "-" },
  { name: "Black Hole Chan", element: "Neutral", recClass: "Mage/Adventurer", classBonus: "1.17% x CL", unlock: "Special Task", evoDiff: "5(7)", improve: false, special: "Scales with UBv4 Points" },
  { name: "Treasure/Mimic", element: "Neutral", recClass: "Assassin", classBonus: "1.25% x CL", unlock: "Special Task", evoDiff: "5(8)", improve: false, special: "Bonus Exp for RTI Pets" },
  { name: "Seed/Yggdrasil", element: "Earth", recClass: "Adventurer", classBonus: "1.7% x CL", unlock: "Special Task", evoDiff: "6(9)", improve: false, special: "-" },
  { name: "Wolf", element: "Neutral", recClass: "Adventurer", classBonus: "1.0% x CL", unlock: "Special Task", evoDiff: "5(6)", improve: false, special: "-" },
  { name: "Meteor", element: "Fire", recClass: "Adventurer", classBonus: "0.85% x CL", unlock: "Special Task", evoDiff: "4(5)", improve: false, special: "Camp Buff from Time in Camps" },
  { name: "Sphinx", element: "Earth", recClass: "Adventurer", classBonus: "0.67% x CL", unlock: "Special Task", evoDiff: "3(5)", improve: true, special: "Small Increase to Dungeon XP" },
  { name: "Aurelius", element: "Neutral", recClass: "Alchemist", classBonus: "0.51% x CL", unlock: "Special Task", evoDiff: "1(2)", improve: true, special: "Faster Elemental Crafting" },
  { name: "Hamster", element: "Earth", recClass: "Blacksmith", classBonus: "0.51% x CL", unlock: "Special Task", evoDiff: "1(1)", improve: true, special: "Crafts Earmuff Accessory" },
  { name: "Corona", element: "Wind", recClass: "Adventurer/Wildcard", classBonus: "0.7% x CL", unlock: "Special Task", evoDiff: "3(4)", improve: true, special: "Causes Pet Illness" },
  { name: "Baby Carno", element: "Neutral", recClass: "Assassin", classBonus: "2% x CL", unlock: "Secret", evoDiff: "7(5)", improve: true, special: "Growth from Food Camp" },
  { name: "Nothing (Other)", element: "Neutral", recClass: "All Classes", classBonus: "0.5% x CL", unlock: "Secret", evoDiff: "1(3)", improve: true, special: "Class Change" },
  { name: "Fool", element: "Neutral", recClass: "Defender", classBonus: "1.2% x CL", unlock: "Secret", evoDiff: "5(8)", improve: false, special: "Confuse Enemies" },
  { name: "Feather Pile/Owl", element: "Neutral", recClass: "Alternates", classBonus: "Special", unlock: "Secret", evoDiff: "1(5)", improve: true, special: "Varies" },
  { name: "Serow", element: "Wind", recClass: "Adventurer/Wildcard", classBonus: "0.65% x CL", unlock: "Secret", evoDiff: "3(4)", improve: true, special: "Saving Dungeon Consumables" },
  { name: "Rudolph", element: "Wind", recClass: "Rogue", classBonus: "0.55% x CL", unlock: "Pet Token", evoDiff: "3(4)", improve: false, special: "-" },
  { name: "Santa", element: "Fire", recClass: "Supporter", classBonus: "0.7% x CL", unlock: "Pet Token", evoDiff: "3(5)", improve: false, special: "Trade Nothings for Choco" },
  { name: "Elf", element: "Wind", recClass: "Blacksmith", classBonus: "0.9% x CL", unlock: "Pet Token", evoDiff: "5(8)", improve: false, special: "Bonus to Crafting (T3/4 Gear)" },
  { name: "Pumpkin", element: "Fire", recClass: "Adventurer", classBonus: "Chocolate", unlock: "Pet Token", evoDiff: "2(4)", improve: true, special: "Finds Choco in Food Campaigns" },
  { name: "Ghost", element: "Neutral", recClass: "Rogue", classBonus: "0.65% x CL", unlock: "Pet Token", evoDiff: "3(3)", improve: false, special: "Debuff Enemy Atk/Def" },
  { name: "Nightmare", element: "Fire", recClass: "Adventurer", classBonus: "0.9% x CL", unlock: "Pet Token", evoDiff: "5(6)", improve: false, special: "Uncap GP Camp Limit" },
  { name: "Question", element: "Neutral", recClass: "Alchemist", classBonus: "0.51% x CL", unlock: "Pet Token", evoDiff: "1(1)", improve: false, special: "-" },
  { name: "Chocobear", element: "Earth", recClass: "Adventurer", classBonus: "0.56% x CL", unlock: "Pet Token", evoDiff: "2(3)", improve: true, special: "Campaign Bonus when Fed Choco" },
  { name: "Rose", element: "Earth", recClass: "Alchemist", classBonus: "0.75% x CL", unlock: "Pet Token", evoDiff: "4(4)", improve: false, special: "Increased Enchanting Speed" },
  { name: "Doughnut", element: "Earth", recClass: "Adventurer/Wildcard", classBonus: "Mighty food", unlock: "Pet Token", evoDiff: "2(2)", improve: true, special: "Cheaper Mighty Food" },
  { name: "Eagle", element: "Wind", recClass: "Adventurer", classBonus: "0.52% x CL", unlock: "Pet Token", evoDiff: "1(2)", improve: false, special: "-" },
  { name: "Penguin", element: "Water", recClass: "Assassin", classBonus: "0.53% x CL", unlock: "Milestones/Pet Token", evoDiff: "2(3)", improve: false, special: "-" },
  { name: "Hermit Crab", element: "Water", recClass: "Blacksmith", classBonus: "0.51% x CL", unlock: "Milestones/Pet Token", evoDiff: "1(2)", improve: false, special: "-" },
  { name: "Hedgehog", element: "Fire", recClass: "Adventurer", classBonus: "0.58% x CL", unlock: "Pet Token", evoDiff: "2(2)", improve: true, special: "-" },
  { name: "Phoenix", element: "Fire", recClass: "Alchemist", classBonus: "0.6% x CL", unlock: "Pet Token", evoDiff: "3(4-5)", improve: false, special: "-" },
  { name: "Wizard", element: "Wind", recClass: "Mage/Wildcard", classBonus: "0.63% x CL", unlock: "Milestones/Pet Token", evoDiff: "3(4)", improve: false, special: "Debuffs the Balrog Boss" },
  { name: "Pegasus", element: "Wind", recClass: "Blacksmith", classBonus: "0.7% x CL", unlock: "Pet Token", evoDiff: "3(4)", improve: false, special: "-" },
  { name: "Panda", element: "Earth", recClass: "Supporter", classBonus: "0.67% x CL", unlock: "Pet Token", evoDiff: "3(3)", improve: false, special: "-" },
  { name: "UFO", element: "Neutral", recClass: "Adventurer", classBonus: "0.7% x CL", unlock: "Pet Token", evoDiff: "3(4)", improve: false, special: "-" },
  { name: "Robot", element: "Neutral", recClass: "Blacksmith/Adventurer", classBonus: "0.75% x CL", unlock: "Pet Token", evoDiff: "4(5-6)", improve: true, special: "T4 Gear Crafting Bonus" },
  { name: "Otter", element: "Water", recClass: "Adventurer", classBonus: "0.8% x CL", unlock: "Pet Token", evoDiff: "4(5)", improve: false, special: "Dungeon Materials when Fed" },
  { name: "FSM", element: "Wind", recClass: "Adventurer", classBonus: "0.85% x CL", unlock: "Pet Token", evoDiff: "4(4)", improve: false, special: "DivGen Boost" },
  { name: "Elephant", element: "Fire", recClass: "Defender", classBonus: "0.8% x CL", unlock: "Pet Token", evoDiff: "4(4)", improve: false, special: "Retaliatory Burn Damage" },
  { name: "Vaccina", element: "Earth", recClass: "Alchemist", classBonus: "0.65% x CL", unlock: "Pet Token", evoDiff: "3(5)", improve: false, special: "Can Craft Vaccines" },
  { name: "Firefox", element: "Fire", recClass: "Blacksmith", classBonus: "0.75% x CL", unlock: "Pet Token", evoDiff: "4(4)", improve: false, special: "Buffs Fire Element" },
  { name: "Beachball", element: "Water", recClass: "Adventurer", classBonus: "0.67% x CL", unlock: "Pet Token", evoDiff: "3(6)", improve: false, special: "Campaign Bonus Scales with Pet Stones" },
  { name: "Tanuki", element: "Neutral", recClass: "Supporter", classBonus: "0.68% x CL", unlock: "Pet Token", evoDiff: "3(3)", improve: false, special: "-" },
  { name: "Seal", element: "Water", recClass: "Alchemist", classBonus: "0.64% x CL", unlock: "Pet Token", evoDiff: "3(3)", improve: false, special: "-" },
  { name: "Raven", element: "Wind", recClass: "Rogue", classBonus: "0.67% x CL", unlock: "Pet Token", evoDiff: "3(4)", improve: false, special: "-" },
  { name: "Hourglass", element: "Wind", recClass: "Supporter", classBonus: "1.0% x CL", unlock: "Pet Token", evoDiff: "5(8)", improve: false, special: "Speed Buff to Party, Debuffs Enemies" },
  { name: "Archer", element: "Neutral", recClass: "Assassin", classBonus: "1.2% x CL", unlock: "Pet Token", evoDiff: "5(7)", improve: false, special: "Attack Twice with Bow" },
  { name: "Thunder Ball/Raiju", element: "Wind", recClass: "Adventurer", classBonus: "1.3% x CL", unlock: "Pet Token", evoDiff: "5(6)", improve: false, special: "-" },
  { name: "Bottle", element: "Fire", recClass: "Alchemist", classBonus: "0.9% x CL", unlock: "Pet Token", evoDiff: "5(5)", improve: false, special: "Free Alchemy Crafts" },
  { name: "Bag", element: "Neutral", recClass: "Adventurer", classBonus: "1.0% x CL", unlock: "Pet Token", evoDiff: "5(7)", improve: true, special: "Shares Growth Campaign to Weakest Pet" },
  { name: "Mysterious Egg", element: "Fire", recClass: "Defender", classBonus: "1.5% x CL", unlock: "Pet Token", evoDiff: "6(6)", improve: false, special: "Bonus Clone Pet Training Exp" },
  { name: "Succubus", element: "Fire", recClass: "Assassin", classBonus: "1.0% x CL", unlock: "Pet Token", evoDiff: "5(5)", improve: false, special: "HP Leech, Always Receives Growth Camp Gains" },
  { name: "Clam", element: "Water", recClass: "Rogue", classBonus: "0.6% x CL", unlock: "Pet Token", evoDiff: "3(3)", improve: false, special: "Double GP from Dungeon Events" },
  { name: "Earth Eater", element: "Earth", recClass: "Adventurer", classBonus: "1.32% x CL", unlock: "Pet Token", evoDiff: "5(8)", improve: true, special: "Buffable Campaign Bonus" },
  { name: "Portal", element: "Wind", recClass: "Adventurer", classBonus: "1.0% x CL", unlock: "Pet Token", evoDiff: "5(7)", improve: false, special: "DivGen ratios and Div GP buy" },
  { name: "Cardboardbox", element: "Wind", recClass: "Rogue", classBonus: "0.9% x CL", unlock: "Pet Token", evoDiff: "5(7)", improve: false, special: "Dungeon Event Reward Bonus/Nerf" },
  { name: "Unicorn", element: "Neutral", recClass: "Adventurer", classBonus: "1.2% x CL", unlock: "Pet Token", evoDiff: "5(7)", improve: false, special: "Crystal Power Bonus" },
  { name: "Stale Tortilla/Taco", element: "Neutral", recClass: "Alchemist", classBonus: "0.8% x CL", unlock: "Pet Token", evoDiff: "4(4)", improve: false, special: "Dungeon Alchemist with Buffs" },
  { name: "Witch", element: "Water", recClass: "Mage", classBonus: "1.25% x CL", unlock: "Pet Token", evoDiff: "5(7)", improve: false, special: "Buffs Water Element" },
  { name: "Sloth", element: "Earth", recClass: "Adventurer", classBonus: "1.25% x CL", unlock: "Pet Token", evoDiff: "5(8)", improve: false, special: "Longer Camps = More Buff" },
  { name: "Cocoa", element: "Neutral", recClass: "Alchemist", classBonus: "0.55% x CL", unlock: "Pet Token", evoDiff: "2(4)", improve: false, special: "Craft Chocolate" },
  { name: "Vesuvius", element: "Fire", recClass: "Mage", classBonus: "0.78% x CL", unlock: "Pet Token", evoDiff: "4(6)", improve: false, special: "Shares Dungeon Growth to Weakest Pet" },
  { name: "Swan", element: "Water", recClass: "Village (Fisher)", classBonus: "0.2% x CL", unlock: "Pet Token", evoDiff: "2(4)", improve: false, special: "Auto Fishing Speed Increase" },
  { name: "Void", element: "Neutral", recClass: "Alchemist", classBonus: "0.85% x CL", unlock: "Pet Token", evoDiff: "4(6)", improve: false, special: "Faster at Crafting Nothings" },
  { name: "Pignata", element: "Earth", recClass: "Wildcard", classBonus: "Rebirth Bacon", unlock: "Pet Token", evoDiff: "3(3)", improve: false, special: "Rebirth Bacon Generation" },
  { name: "Lizard/Zookeeper", element: "Water", recClass: "Adventurer", classBonus: "0.9% x CL", unlock: "Pet Token", evoDiff: "5(5)", improve: false, special: "Gives Growth in Food Camp" },
  { name: "Alien", element: "Water", recClass: "Assassin", classBonus: "0.85% x CL", unlock: "Pet Token", evoDiff: "4(4)", improve: false, special: "Increases Dungeon Event Rate" },
  { name: "Bat", element: "Neutral", recClass: "Blacksmith", classBonus: "1.0% x CL", unlock: "Pet Token", evoDiff: "5(7)", improve: false, special: "40% More Class Exp for Self" },
  { name: "Flying Eyeball", element: "Wind", recClass: "Adventurer", classBonus: "0.85% x CL", unlock: "Pet Token", evoDiff: "4(7)", improve: false, special: "Dungeon Adventurer" },
  { name: "Leviathan", element: "Water", recClass: "Defender", classBonus: "2% x CL", unlock: "Milestones", evoDiff: "7(12)", improve: false, special: "Counterattack" },
  { name: "Basilisk", element: "Earth", recClass: "Mage", classBonus: "1.5% x CL", unlock: "Special Task", evoDiff: "1(9)", improve: false, special: "AoE Element Debuff" },
  { name: "Cherub", element: "Wind", recClass: "Defender", classBonus: "2% x CL", unlock: "Pet Token", evoDiff: "6(10)", improve: false, special: "Stunning Attacks, Reduces Speed Dmg" },
  { name: "Tödlicher Löffel", element: "Neutral", recClass: "Mage", classBonus: "1.19% x CL", unlock: "Pet Token", evoDiff: "5(6)", improve: false, special: "Defense Debuff, Counts as All Elements" },
  { name: "Goblin", element: "Earth", recClass: "Adventurer", classBonus: "0.1% x CL", unlock: "Pet Token", evoDiff: "1(4)", improve: false, special: "-" },
  { name: "Koi", element: "Water", recClass: "Village (Fish Seller)", classBonus: "0.2% x CL", unlock: "Pet Token", evoDiff: "3(6)", improve: false, special: "Fish Seller Speed" },
  { name: "Decorator Crab", element: "Water", recClass: "Adventurer", classBonus: "1.75% x CL", unlock: "Pet Token", evoDiff: "6(8)", improve: false, special: "Extra Items from Item Camp" },
  { name: "Dwarf", element: "Fire", recClass: "Blacksmith", classBonus: "0.9% x CL", unlock: "Pet Token", evoDiff: "5(10)", improve: false, special: "Donate Gems, Better Crafter" },
  { name: "Caterpillar", element: "Wind", recClass: "Alchemist", classBonus: "0.55% x CL", unlock: "Pet Token", evoDiff: "2(2)", improve: false, special: "Faster Material Upgrade" },
  { name: "Bunny Girl", element: "Neutral", recClass: "Alchemist", classBonus: "1.17% x CL", unlock: "Pet Token", evoDiff: "5(7)", improve: false, special: "Bonus Speed Crafting Talisman/T3 Bars" },
  { name: "Elemental", element: "Neutral", recClass: "Blacksmith", classBonus: "2% x CL", unlock: "Tavern Rank SSS", evoDiff: "6(10)", improve: false, special: "Boosts Dungeon Team Elements & Own Dmg" },
  { name: "Mist Sphere", element: "Water", recClass: "Supporter", classBonus: "0.9% x CL", unlock: "Pet Token", evoDiff: "5(6)", improve: false, special: "Shields Pets it Heals" },
  { name: "Shadow Clone", element: "Fire", recClass: "Adventurer", classBonus: "Shadow Clones", unlock: "Pet Token", evoDiff: "4(5)", improve: false, special: "Can Create Shadow Clones" },
  { name: "Sniper", element: "Wind", recClass: "Assassin", classBonus: "2% x CL", unlock: "Pet Token", evoDiff: "6(10)", improve: false, special: "Attacks Once/Turn, High Damage" },
  { name: "White Tiger", element: "Wind", recClass: "Assassin", classBonus: "1.0% x CL", unlock: "Special", evoDiff: "5(5)", improve: true, special: "-" },
  { name: "Black Tortoise", element: "Water", recClass: "Defender", classBonus: "1.0% x CL", unlock: "Special", evoDiff: "5(5)", improve: true, special: "-" },
  { name: "Azure Dragon", element: "Earth", recClass: "Supporter", classBonus: "1.0% x CL", unlock: "Special", evoDiff: "5(5)", improve: true, special: "-" },
  { name: "Vermilion Pheasant", element: "Fire", recClass: "Rogue", classBonus: "1.0% x CL", unlock: "Special", evoDiff: "5(5)", improve: true, special: "-" },
  { name: "Tenko", element: "Neutral", recClass: "Adventurer", classBonus: "1.1% x CL", unlock: "Pet Token", evoDiff: "5(7)", improve: false, special: "-" },
  { name: "Vampire", element: "Fire", recClass: "Alchemist", classBonus: "1.75% x CL", unlock: "Pet Token", evoDiff: "6(6)", improve: true, special: "Crafting/Dungeon Alchemist" },
  { name: "Strategist", element: "Neutral", recClass: "Rogue", classBonus: "1.33% x CL", unlock: "Strategy Room Lv11", evoDiff: "7(9)", improve: true, special: "Improves Team Health" },
  { name: "Mermaid", element: "Water", recClass: "Supporter", classBonus: "2% x CL", unlock: "Pet Token", evoDiff: "7(7)", improve: false, special: "Improves Dungeon Team Stats" },
  { name: "Monk", element: "Neutral", recClass: "Village (Dojo)", classBonus: "0.25% x CL", unlock: "Pet Token", evoDiff: "6(7)", improve: false, special: "Dojo Pet" },
  { name: "Honeybadger", element: "Earth", recClass: "Assassin", classBonus: "2% x CL", unlock: "Pet Token", evoDiff: "7(11)", improve: false, special: "Immune to Stun, No Blacksmith Needed" },
  { name: "Pack Mule", element: "Neutral", recClass: "Village (Tavern)", classBonus: "0.2% x CL", unlock: "Pet Token", evoDiff: "3(3)", improve: false, special: "Increases Quest Rewards" },
  { name: "Anni Cake", element: "Earth", recClass: "Adventurer", classBonus: "1.38% x CL", unlock: "Pet Token", evoDiff: "5(8)", improve: false, special: "Bonus Pet Stats from Food Camp Time" },
  { name: "Llysnafedda", element: "Water", recClass: "Tavern Wildcard", classBonus: "0.65% x CL", unlock: "Pet Token", evoDiff: "3(3)", improve: false, special: "Leveling Busy Pets" },
  { name: "Ancient Mimic", element: "Neutral", recClass: "Assassin", classBonus: "2% x CL", unlock: "5000 Mimic Points", evoDiff: "8(15)", improve: false, special: "-" },
  { name: "Student", element: "Earth", recClass: "Alchemist", classBonus: "2% x CL", unlock: "Pet Token", evoDiff: "7(10)", improve: false, special: "-" },
  { name: "Hwangeum Pig", element: "Earth", recClass: "Assassin", classBonus: "2% x CL", unlock: "Pet Token", evoDiff: "6(8)", improve: false, special: "Bonus Damage, Wind Element Bonus" },
  { name: "Lamb", element: "Earth", recClass: "Village (Alchemy Hut)", classBonus: "0.5% x CL", unlock: "Pet Token", evoDiff: "2(3)", improve: false, special: "Bonus Output in Alchemy Hut" },
  { name: "Duragizer", element: "Neutral", recClass: "Wildcard", classBonus: "-", unlock: "Have 10 Pets Unlocked", evoDiff: "1(2)", improve: true, special: "No Penalty EXP Drain" },
  { name: "Nugget", element: "Neutral", recClass: "All Classes", classBonus: "0.3% x CL", unlock: "Defeat a D3-0 Boss", evoDiff: "4(6)", improve: false, special: "Class Change" },
  { name: "Arachne", element: "Wind", recClass: "Assassin", classBonus: "1.75% x CL", unlock: "Pet Token", evoDiff: "6(8)", improve: false, special: "Poison Attacks" },
  { name: "Simulacrum", element: "Wind", recClass: "Mage", classBonus: "2.0% x CL", unlock: "Pet Token", evoDiff: "6(9)", improve: false, special: "-" },
  { name: "Pixie Goatmother", element: "Wind", recClass: "Supporter", classBonus: "2.0% x CL", unlock: "Pet Token", evoDiff: "7(11)", improve: false, special: "Bonus Actions" },
  { name: "Fainting Capra", element: "Water", recClass: "Rogue", classBonus: "2.0% x CL", unlock: "Pet Token", evoDiff: "7(8)", improve: false, special: "Random DR, In-built Knife Effect" },
  { name: "Wolpertinger", element: "Wind", recClass: "Alchemist", classBonus: "1.32% x CL", unlock: "Pet Token", evoDiff: "5(6)", improve: false, special: "Tavernkeep Bonuses" },
  { name: "Skeleton", element: "Neutral", recClass: "Adventurer", classBonus: "0.83% x CL", unlock: "Pet Token", evoDiff: "5(6)", improve: false, special: "Boosts Pumpkin Chocolate" },
  { name: "Bear", element: "Fire", recClass: "Adventurer", classBonus: "0.75% x CL", unlock: "Give it 1000 Honey", evoDiff: "4(6)", improve: false, special: "Campaign Bonus for Honey" },
  { name: "Dark Gift", element: "Neutral", recClass: "Mage", classBonus: "2.2% x CL", unlock: "Pet Token", evoDiff: "7(9)", improve: false, special: "-" },
  { name: "Dorgegebelle", element: "Water", recClass: "Adventurer", classBonus: "1.7% x CL", unlock: "Pet Token", evoDiff: "6(7)", improve: false, special: "Bonus to Food, Starvation" },
];

// ─── NAME MATCHING ───────────────────────────────────────────────────────────
// Explicit aliases: lowercase key → DB name
// For evolved names, abbreviations, and other non-obvious mappings
const NAME_ALIASES = {
  // Slash-name shortcuts
  "egg": "Egg/Chicken", "chicken": "Egg/Chicken",
  "stone": "Stone/Golem", "golem": "Stone/Golem",
  "treasure": "Treasure/Mimic", "mimic": "Treasure/Mimic",
  "seed": "Seed/Yggdrasil", "yggdrasil": "Seed/Yggdrasil",
  "lizard": "Lizard/Zookeeper", "zookeeper": "Lizard/Zookeeper",
  "featherpile": "Feather Pile/Owl", "owl": "Feather Pile/Owl",
  // Apostrophe / special chars
  "pandora": "Pandora's Box", "pandora's box": "Pandora's Box",
  "pandorasbox": "Pandora's Box", "pandoras box": "Pandora's Box",
  // Parenthetical names
  "godpower": "God Power (Pet)", "god power": "God Power (Pet)",
  "nothing": "Nothing (Other)",
  // Abbreviations
  "bhc": "Black Hole Chan",
  // German pet
  "spoon": "Tödlicher Löffel",
  "todlicherloffel": "Tödlicher Löffel", "todlicher loffel": "Tödlicher Löffel",
  // Evolved names → base DB name
  "raiju": "Thunder Ball/Raiju",
  "thunderball": "Thunder Ball/Raiju", "thunder ball": "Thunder Ball/Raiju",
  "reindeer": "Rudolph",
  "ancientmimic": "Ancient Mimic", "ancient mimic": "Ancient Mimic",
  "taco": "Stale Tortilla/Taco",
  "hwangeumpig": "Hwangeum Pig",
  "pixiegoatmother": "Pixie Goatmother",
  // Export name → display name
  "baphomate": "Dark Gift",
  // Golden Dragon variant
  "goldendragon": "Gold Dragon", "golden dragon": "Gold Dragon",
  // Shortened evolved/export names
  "book": "Holy ITRTG Book",
  "carno": "Baby Carno",
  "cloud": "Flying Cloud",
  "crab": "Hermit Crab",
  "pixiegoat": "Pixie Goatmother",
  "volcano": "Vesuvius",
  // Gray's children
  "graychild1": "Gray Child 1", "gray child 1": "Gray Child 1",
  "graychild2": "Gray Child 2", "gray child 2": "Gray Child 2",
  // Cardboard box: the DB name is "Cardboardbox" (one word, per wiki)
  // but export sends "CardboardBox" which stripped = "cardboardbox" matches
};

// ─── WIKI LINKS ──────────────────────────────────────────────────────────────
const WIKI_BASE = "https://itrtg.wiki.gg/wiki/";
const WIKI_OVERRIDES = {
  "Student": "Student_(pet)",
  "Elemental": "Elemental_(Pet)",
  "Lizard/Zookeeper": "Lizard",
  "Gray Child 1": "Gray",
  "Gray Child 2": "Gray",
};
function wikiUrl(name) {
  if (!name) return null;
  const slug = WIKI_OVERRIDES[name] || name.replace(/ /g, "_");
  return WIKI_BASE + encodeURI(slug);
}

// Build a reverse lookup: lowercase-nospaces DB name → DB name
const DB_NOSPACE_MAP = {};
DB.forEach(d => {
  DB_NOSPACE_MAP[d.name.toLowerCase().replace(/\s+/g, "")] = d.name;
  // Also index each slash segment
  d.name.split("/").forEach(seg => {
    const k = seg.trim().toLowerCase().replace(/\s+/g, "");
    if (!DB_NOSPACE_MAP[k]) DB_NOSPACE_MAP[k] = d.name;
  });
});

function matchDbName(gameName) {
  if (!gameName) return null;
  const lower = gameName.toLowerCase().trim();
  const noSpaces = lower.replace(/\s+/g, "");

  // 1) Explicit alias (exact)
  if (NAME_ALIASES[lower]) return NAME_ALIASES[lower];
  // 2) Explicit alias (no-spaces)
  if (NAME_ALIASES[noSpaces]) return NAME_ALIASES[noSpaces];
  // 3) Direct case-insensitive match
  const direct = DB.find(d => d.name.toLowerCase() === lower);
  if (direct) return direct.name;
  // 4) Pre-built no-space lookup (handles "BunnyGirl"→"Bunny Girl", etc)
  if (DB_NOSPACE_MAP[noSpaces]) return DB_NOSPACE_MAP[noSpaces];

  return null;
}

// ─── STYLING ─────────────────────────────────────────────────────────────────
const ELEM = {
  Neutral: { bg: "#22222e", text: "#9999bb", border: "#3a3a55" },
  Fire:    { bg: "#2e1a1a", text: "#ff8855", border: "#5a2a1a" },
  Water:   { bg: "#1a1a2e", text: "#55aaff", border: "#1a2a5a" },
  Wind:    { bg: "#1a2e1a", text: "#55dd88", border: "#1a5a2a" },
  Earth:   { bg: "#2e2a1a", text: "#ccaa55", border: "#5a4a1a" },
  Dark:    { bg: "#1a1a22", text: "#aa66dd", border: "#3a2255" },
  Light:   { bg: "#2e2e1a", text: "#dddd66", border: "#55551a" },
  All:     { bg: "#2a1a2e", text: "#dd88dd", border: "#4a2a55" },
};

function evoDiffNum(s) { const m = s.match(/^(\d+)/); return m ? parseInt(m[1]) : 99; }
function evoDiffCond(s) { const m = s.match(/\((\d+)/); return m ? parseInt(m[1]) : 99; }
function parseNumber(s) { if (!s || s.trim() === "") return 0; return parseInt(s.replace(/,/g, ""), 10) || 0; }

function parsePetData(raw) {
  const lines = raw.trim().split("\n").filter(l => l.trim());
  if (lines.length === 0) return [];
  let dataLines = lines;
  const first = lines[0].toLowerCase();
  if (first.includes("name") && first.includes("element") && first.includes("growth")) dataLines = lines.slice(1);
  return dataLines.map(line => {
    const p = line.split(";");
    if (p.length < 10) return null;
    return {
      name: p[0]?.trim() || "", element: p[1]?.trim() || "Neutral",
      growth: parseNumber(p[2]), dungeonLevel: parseNumber(p[3]),
      class: p[4]?.trim() || "", classLevel: parseNumber(p[5]),
      hp: parseNumber(p[6]), attack: parseNumber(p[7]),
      defense: parseNumber(p[8]), speed: parseNumber(p[9]),
      weapon: p[16]?.trim() || "", armor: p[17]?.trim() || "", accessory: p[18]?.trim() || "",
      action: p[19]?.trim() || "", unlocked: p[20]?.trim() === "Yes",
      improvement: p[21]?.trim() || "", other: p[22]?.trim() || "",
    };
  }).filter(Boolean);
}

function fmt(n) { return n.toLocaleString(); }

function ElemBadge({ el }) {
  const c = ELEM[el] || ELEM.Neutral;
  return <span style={{ display: "inline-block", padding: "1px 8px", borderRadius: 3, background: c.bg, color: c.text, border: `1px solid ${c.border}`, fontSize: 11, fontWeight: 600 }}>{el}</span>;
}

function EvoBadge({ diff }) {
  const n = evoDiffNum(diff);
  const color = n <= 2 ? "#4ade80" : n <= 4 ? "#d4a44a" : n <= 6 ? "#e8825d" : "#ef4444";
  return <span style={{ display: "inline-block", padding: "1px 7px", borderRadius: 3, background: `${color}18`, color, border: `1px solid ${color}40`, fontSize: 11, fontWeight: 600, fontFamily: "monospace" }}>{diff}</span>;
}

function StatusBadge({ label, color, outline }) {
  return <span style={{ display: "inline-block", padding: "1px 7px", borderRadius: 3, fontSize: 10, fontWeight: 700, letterSpacing: "0.04em", textTransform: "uppercase", background: outline ? "transparent" : `${color}20`, color, border: `1px solid ${color}${outline ? "60" : "40"}` }}>{label}</span>;
}

// ─── MULTISELECT ─────────────────────────────────────────────────────────────
const DUNGEON_CLASSES = new Set(["Assassin", "Defender", "Mage", "Rogue", "Supporter"]);
const DUNGEON_REC_CLASSES = new Set([...DUNGEON_CLASSES, "Wildcard", "Dungeon Wildcard", "All Classes"]);

function MultiSelect({ options, selected, onChange, presets, label }) {
  // selected: null = all, Set = specific selections
  const [open, setOpen] = useState(false);
  const ref = useRef(null);
  useEffect(() => {
    if (!open) return;
    const handler = (e) => { if (ref.current && !ref.current.contains(e.target)) setOpen(false); };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  const allSet = useMemo(() => new Set(options), [options]);
  const active = selected || allSet;
  const isAll = !selected || selected.size === allSet.size;
  const dungeonSet = presets?.dungeonSet || DUNGEON_CLASSES;
  const dungeonCount = [...dungeonSet].filter(c => allSet.has(c)).length;
  const isDungeon = !isAll && presets?.dungeon && selected?.size === dungeonCount && [...dungeonSet].filter(c => allSet.has(c)).every(c => selected.has(c));

  let buttonLabel = "All";
  if (isDungeon) buttonLabel = `Dungeon (${dungeonCount})`;
  else if (!isAll) buttonLabel = `${active.size} of ${allSet.size}`;

  const toggle = (val) => {
    const next = new Set(active);
    if (next.has(val)) next.delete(val); else next.add(val);
    onChange(next.size === allSet.size ? null : next);
  };

  const btnStyle = (isActive) => ({
    padding: "3px 8px", fontSize: 10, fontWeight: 600, border: `1px solid ${isActive ? "#5a3a8a" : "#252535"}`,
    borderRadius: 3, cursor: "pointer", background: isActive ? "#2a1a4a" : "#0c0c16", color: isActive ? "#c9a0ff" : "#666680",
  });

  return (
    <div ref={ref} style={{ position: "relative", flex: "1 1 100px" }}>
      <label style={{ display: "block", fontSize: 9, fontWeight: 700, color: "#444460", textTransform: "uppercase", letterSpacing: "0.1em", marginBottom: 3 }}>{label}</label>
      <button onClick={() => setOpen(!open)} style={{ width: "100%", padding: "6px 8px", background: "#0c0c16", border: `1px solid ${open ? "#5a3a8a" : "#252535"}`, borderRadius: 4, color: isAll ? "#b0b0c8" : "#c9a0ff", fontSize: 12, cursor: "pointer", textAlign: "left", fontFamily: "inherit", display: "flex", justifyContent: "space-between", alignItems: "center", boxSizing: "border-box" }}>
        <span>{buttonLabel}</span>
        <span style={{ fontSize: 8, color: "#555", marginLeft: 4 }}>{open ? "▲" : "▼"}</span>
      </button>
      {open && (
        <div style={{ position: "absolute", top: "100%", left: 0, marginTop: 2, zIndex: 20, background: "#0e0e1a", border: "1px solid #2a2a40", borderRadius: 5, padding: "8px 0", minWidth: 180, maxHeight: 280, overflowY: "auto", boxShadow: "0 8px 24px rgba(0,0,0,0.6)" }}>
          {/* Preset buttons */}
          <div style={{ display: "flex", gap: 4, padding: "0 8px 6px", borderBottom: "1px solid #1a1a28", flexWrap: "wrap" }}>
            <button onClick={() => { onChange(null); }} style={btnStyle(isAll)}>All</button>
            <button onClick={() => { onChange(new Set()); }} style={btnStyle(!isAll && active.size === 0)}>None</button>
            {presets?.dungeon && <button onClick={() => { const d = new Set([...dungeonSet].filter(c => allSet.has(c))); onChange(d.size === allSet.size ? null : d); }} style={btnStyle(isDungeon)}>Dungeon</button>}
          </div>
          {/* Checkboxes */}
          {options.map(opt => (
            <label key={opt} onClick={() => toggle(opt)} style={{ display: "flex", alignItems: "center", gap: 8, padding: "4px 10px", cursor: "pointer", fontSize: 11, color: active.has(opt) ? "#c0c0d4" : "#444460", userSelect: "none" }}
              onMouseEnter={e => e.currentTarget.style.background = "#14142a"} onMouseLeave={e => e.currentTarget.style.background = "transparent"}>
              <span style={{ width: 14, height: 14, borderRadius: 3, border: `1px solid ${active.has(opt) ? "#5a3a8a" : "#333350"}`, background: active.has(opt) ? "#2a1a4a" : "transparent", display: "flex", alignItems: "center", justifyContent: "center", fontSize: 10, flexShrink: 0 }}>
                {active.has(opt) && <span style={{ color: "#c9a0ff" }}>✓</span>}
              </span>
              {opt}
            </label>
          ))}
        </div>
      )}
    </div>
  );
}

// ─── MAIN ────────────────────────────────────────────────────────────────────
export default function PetAnalyzer() {
  const [rawData, setRawData] = useState("");
  const [imported, setImported] = useState(null);
  const [search, setSearch] = useState("");
  const [elementFilter, setElementFilter] = useState("All");
  const [viewFilter, setViewFilter] = useState("all");
  const [classFilter, setClassFilter] = useState(null); // null = all, Set = selected
  const [myClassFilter, setMyClassFilter] = useState(null);
  const [improveFilter, setImproveFilter] = useState("all");
  const [specialFilter, setSpecialFilter] = useState("all");
  const [unlockFilter, setUnlockFilter] = useState("All");
  const [sortKey, setSortKey] = useState("name");
  const [sortDir, setSortDir] = useState("asc");
  const [expanded, setExpanded] = useState(null);
  const [showImport, setShowImport] = useState(true);

  const handleImport = useCallback(() => {
    const pets = parsePetData(rawData);
    if (pets.length > 0) { setImported(pets); setShowImport(false); }
  }, [rawData]);

  const merged = useMemo(() => {
    const result = DB.map(dbPet => {
      let myPet = null;
      if (imported) {
        myPet = imported.find(ip => matchDbName(ip.name) === dbPet.name);
      }
      return { db: dbPet, my: myPet || null };
    });
    if (imported) {
      imported.forEach(ip => {
        if (!matchDbName(ip.name)) result.push({ db: null, my: ip });
      });
    }
    return result;
  }, [imported]);

  const elements = useMemo(() => ["All", ...Array.from(new Set(DB.map(d => d.element))).sort()], []);
  const classes = useMemo(() => { const s = new Set(); DB.forEach(d => d.recClass.split("/").forEach(c => s.add(c.trim()))); return Array.from(s).sort(); }, []);
  const unlockTypes = useMemo(() => ["All", ...Array.from(new Set(DB.map(d => d.unlock))).sort()], []);
  const myClasses = useMemo(() => {
    if (!imported) return [];
    const s = new Set();
    imported.forEach(p => { if (p.class && p.class !== "" && p.class !== "None" && p.class !== "none") s.add(p.class); });
    return Array.from(s).sort();
  }, [imported]);

  const filtered = useMemo(() => {
    let items = [...merged];
    if (search) { const q = search.toLowerCase(); items = items.filter(({ db, my }) => { const n = db?.name || my?.name || ""; const sp = db?.special || ""; const rc = db?.recClass || ""; const mc = my?.class || ""; const ac = my?.action || ""; return n.toLowerCase().includes(q) || sp.toLowerCase().includes(q) || rc.toLowerCase().includes(q) || mc.toLowerCase().includes(q) || ac.toLowerCase().includes(q); }); }
    if (elementFilter !== "All") items = items.filter(({ db, my }) => (db?.element || my?.element || "") === elementFilter);
    if (classFilter) items = items.filter(({ db }) => db && db.recClass.split("/").some(c => classFilter.has(c.trim())));
    if (myClassFilter) items = items.filter(({ my }) => my && myClassFilter.has(my.class));
    if (unlockFilter !== "All") items = items.filter(({ db }) => db && db.unlock === unlockFilter);
    if (improveFilter === "yes") items = items.filter(({ db }) => db?.improve);
    if (improveFilter === "no") items = items.filter(({ db }) => db && !db.improve);
    if (specialFilter === "yes") items = items.filter(({ db }) => db && db.special !== "-");
    if (specialFilter === "no") items = items.filter(({ db }) => db && db.special === "-");
    if (viewFilter === "unlocked") items = items.filter(({ my }) => my?.unlocked);
    if (viewFilter === "locked") items = items.filter(({ my }) => !my || !my.unlocked);
    if (viewFilter === "unevolved") items = items.filter(({ my }) => my?.unlocked && (!my.class || my.class === "" || my.class === "None" || my.class === "none"));
    if (viewFilter === "evolved") items = items.filter(({ my }) => my?.unlocked && my.class && my.class !== "" && my.class !== "None" && my.class !== "none");
    if (viewFilter === "unmatched") items = items.filter(({ db }) => !db);
    items.sort((a, b) => {
      let cmp = 0;
      switch (sortKey) {
        case "name": cmp = (a.db?.name || a.my?.name || "").localeCompare(b.db?.name || b.my?.name || ""); break;
        case "element": cmp = (a.db?.element || "").localeCompare(b.db?.element || ""); break;
        case "recClass": cmp = (a.db?.recClass || "").localeCompare(b.db?.recClass || ""); break;
        case "evoDiff": {
          cmp = evoDiffNum(a.db?.evoDiff || "99") - evoDiffNum(b.db?.evoDiff || "99");
          if (cmp === 0) cmp = evoDiffCond(a.db?.evoDiff || "99") - evoDiffCond(b.db?.evoDiff || "99");
          break;
        }
        case "evoCond": {
          cmp = evoDiffCond(a.db?.evoDiff || "99") - evoDiffCond(b.db?.evoDiff || "99");
          if (cmp === 0) cmp = evoDiffNum(a.db?.evoDiff || "99") - evoDiffNum(b.db?.evoDiff || "99");
          break;
        }
        case "classBonus": { const pa = parseFloat((a.db?.classBonus || "0").replace(/[^0-9.]/g, "")) || 0; const pb = parseFloat((b.db?.classBonus || "0").replace(/[^0-9.]/g, "")) || 0; cmp = pa - pb; break; }
        case "growth": cmp = (a.my?.growth || 0) - (b.my?.growth || 0); break;
        case "dungeonLevel": cmp = (a.my?.dungeonLevel || 0) - (b.my?.dungeonLevel || 0); break;
        case "classLevel": cmp = (a.my?.classLevel || 0) - (b.my?.classLevel || 0); break;
        default: cmp = 0;
      }
      // Secondary sort: growth as tiebreaker (same direction as primary)
      if (cmp === 0 && sortKey !== "growth") cmp = (a.my?.growth || 0) - (b.my?.growth || 0);
      return sortDir === "asc" ? cmp : -cmp;
    });
    return items;
  }, [merged, search, elementFilter, classFilter, myClassFilter, unlockFilter, improveFilter, specialFilter, viewFilter, sortKey, sortDir]);

  const stats = useMemo(() => {
    if (!imported) return null;
    const unlocked = imported.filter(p => p.unlocked);
    const evolved = unlocked.filter(p => p.class && p.class !== "" && p.class !== "None" && p.class !== "none");
    return { total: imported.length, unlocked: unlocked.length, evolved: evolved.length, unevolved: unlocked.length - evolved.length, totalGrowth: unlocked.reduce((s, p) => s + p.growth, 0), totalDungeon: unlocked.reduce((s, p) => s + p.dungeonLevel, 0) };
  }, [imported]);

  const handleSort = (key) => { if (sortKey === key) setSortDir(d => d === "asc" ? "desc" : "asc"); else { setSortKey(key); setSortDir(key === "name" || key === "element" || key === "recClass" ? "asc" : "desc"); } };

  const SortTh = ({ label, k, w, align }) => {
    const active = sortKey === k;
    return <th onClick={() => handleSort(k)} style={{ cursor: "pointer", padding: "8px 6px", textAlign: align || "left", fontWeight: 600, fontSize: 10, letterSpacing: "0.06em", textTransform: "uppercase", color: active ? "#c9a0ff" : "#666680", borderBottom: active ? "2px solid #c9a0ff" : "2px solid #1a1a28", userSelect: "none", whiteSpace: "nowrap", width: w, background: "#0c0c18", position: "sticky", top: 0, zIndex: 2 }}>{label} {active ? (sortDir === "asc" ? "▲" : "▼") : ""}</th>;
  };

  const sel = { padding: "6px 8px", background: "#0c0c16", border: "1px solid #252535", borderRadius: 4, color: "#b0b0c8", fontSize: 12, outline: "none", minWidth: 0, flex: "1 1 100px" };
  const isEvolved = (my) => my && my.class && my.class !== "" && my.class !== "None" && my.class !== "none";

  return (
    <div style={{ fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', 'Consolas', monospace", background: "#08080e", color: "#c0c0d4", minHeight: "100vh", padding: 0 }}>
      {/* HEADER */}
      <div style={{ background: "linear-gradient(180deg, #10101c 0%, #08080e 100%)", borderBottom: "1px solid #1a1a2a", padding: "16px 20px" }}>
        <div style={{ maxWidth: 1400, margin: "0 auto" }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", flexWrap: "wrap", gap: 12 }}>
            <div>
              <h1 style={{ margin: 0, fontSize: 18, fontWeight: 700, color: "#d8c8ff", letterSpacing: "0.08em" }}>⚔ ITRTG PET ANALYZER</h1>
              <p style={{ margin: "2px 0 0", fontSize: 11, color: "#444460" }}>
                {imported ? `${filtered.length} shown · ${stats?.unlocked} unlocked · ${stats?.evolved} evolved · ${stats?.unevolved} unevolved` : `${DB.length} pets in database · paste your data to compare`}
              </p>
            </div>
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              {imported && stats && (
                <div style={{ display: "flex", gap: 12, fontSize: 12, marginRight: 8 }}>
                  <span style={{ color: "#555" }}>Growth: <span style={{ color: "#ffd700", fontWeight: 600 }}>{fmt(stats.totalGrowth)}</span></span>
                  <span style={{ color: "#555" }}>Dng Lvs: <span style={{ color: "#55aaff", fontWeight: 600 }}>{fmt(stats.totalDungeon)}</span></span>
                </div>
              )}
              <button onClick={() => setShowImport(!showImport)} style={{ padding: "6px 14px", background: imported ? "#1a1a2a" : "#2a1a4a", border: `1px solid ${imported ? "#2a2a3a" : "#4a2a7a"}`, borderRadius: 4, color: imported ? "#888" : "#c9a0ff", cursor: "pointer", fontSize: 11, fontWeight: 600 }}>{imported ? "RE-IMPORT" : "IMPORT DATA"}</button>
              {imported && <button onClick={() => { setImported(null); setRawData(""); setShowImport(true); }} style={{ padding: "6px 14px", background: "#2a1a1a", border: "1px solid #4a2a2a", borderRadius: 4, color: "#e85d5d", cursor: "pointer", fontSize: 11, fontWeight: 600 }}>CLEAR</button>}
            </div>
          </div>
        </div>
      </div>

      <div style={{ maxWidth: 1400, margin: "0 auto", padding: "0 20px 40px" }}>
        {/* IMPORT */}
        {showImport && (
          <div style={{ margin: "16px 0", padding: 16, background: "#0c0c16", border: "1px solid #1e1e30", borderRadius: 6 }}>
            <div style={{ fontSize: 11, color: "#555570", marginBottom: 8, fontWeight: 600, letterSpacing: "0.06em", textTransform: "uppercase" }}>Paste game export (semicolon-delimited)</div>
            <textarea value={rawData} onChange={e => setRawData(e.target.value)} placeholder="Name;Element;Growth;Dungeon Level;Class;Class Level;HP;Attack;Defense;Speed;..." style={{ width: "100%", height: 120, padding: 10, background: "#06060c", border: "1px solid #1a1a2a", borderRadius: 4, color: "#aab", fontSize: 12, fontFamily: "inherit", resize: "vertical", outline: "none", boxSizing: "border-box" }} />
            <div style={{ display: "flex", gap: 8, marginTop: 8, alignItems: "center" }}>
              <button onClick={handleImport} disabled={!rawData.trim()} style={{ padding: "7px 20px", background: rawData.trim() ? "#2a1a4a" : "#151520", border: `1px solid ${rawData.trim() ? "#5a3a8a" : "#222"}`, borderRadius: 4, color: rawData.trim() ? "#c9a0ff" : "#444", cursor: rawData.trim() ? "pointer" : "default", fontSize: 12, fontWeight: 600 }}>IMPORT</button>
              {imported && <span style={{ fontSize: 11, color: "#4ade80" }}>✓ {imported.length} pets loaded</span>}
            </div>
          </div>
        )}

        {/* FILTERS */}
        <div style={{ margin: "16px 0", padding: "12px 16px", background: "#0a0a14", border: "1px solid #1a1a28", borderRadius: 6, display: "flex", flexWrap: "wrap", gap: 10, alignItems: "flex-end" }}>
          <div style={{ flex: "2 1 180px", minWidth: 140 }}>
            <label style={{ display: "block", fontSize: 9, fontWeight: 700, color: "#444460", textTransform: "uppercase", letterSpacing: "0.1em", marginBottom: 3 }}>Search</label>
            <input value={search} onChange={e => setSearch(e.target.value)} placeholder="Name, class, special..." style={{ ...sel, width: "100%", boxSizing: "border-box" }} />
          </div>
          {imported && <div style={{ flex: "1 1 120px" }}><label style={{ display: "block", fontSize: 9, fontWeight: 700, color: "#444460", textTransform: "uppercase", letterSpacing: "0.1em", marginBottom: 3 }}>Roster</label><select value={viewFilter} onChange={e => setViewFilter(e.target.value)} style={sel}><option value="all">All Pets</option><option value="unlocked">My Unlocked</option><option value="locked">Not Unlocked</option><option value="unevolved">Unlocked + Unevolved</option><option value="evolved">Unlocked + Evolved</option><option value="unmatched">Unmatched Imports</option></select></div>}
          <div style={{ flex: "1 1 100px" }}><label style={{ display: "block", fontSize: 9, fontWeight: 700, color: "#444460", textTransform: "uppercase", letterSpacing: "0.1em", marginBottom: 3 }}>Element</label><select value={elementFilter} onChange={e => setElementFilter(e.target.value)} style={sel}>{elements.map(e => <option key={e} value={e}>{e}</option>)}</select></div>
          <MultiSelect options={classes} selected={classFilter} onChange={setClassFilter} presets={{ dungeon: true, dungeonSet: DUNGEON_REC_CLASSES }} label="Rec. Class" />
          {imported && <MultiSelect options={myClasses} selected={myClassFilter} onChange={setMyClassFilter} presets={{ dungeon: true }} label="My Class" />}
          <div style={{ flex: "1 1 100px" }}><label style={{ display: "block", fontSize: 9, fontWeight: 700, color: "#444460", textTransform: "uppercase", letterSpacing: "0.1em", marginBottom: 3 }}>Unlock</label><select value={unlockFilter} onChange={e => setUnlockFilter(e.target.value)} style={sel}>{unlockTypes.map(u => <option key={u} value={u}>{u}</option>)}</select></div>
          <div style={{ flex: "0 1 90px" }}><label style={{ display: "block", fontSize: 9, fontWeight: 700, color: "#444460", textTransform: "uppercase", letterSpacing: "0.1em", marginBottom: 3 }}>Improvable</label><select value={improveFilter} onChange={e => setImproveFilter(e.target.value)} style={sel}><option value="all">Any</option><option value="yes">Yes</option><option value="no">No</option></select></div>
          <div style={{ flex: "0 1 90px" }}><label style={{ display: "block", fontSize: 9, fontWeight: 700, color: "#444460", textTransform: "uppercase", letterSpacing: "0.1em", marginBottom: 3 }}>Special</label><select value={specialFilter} onChange={e => setSpecialFilter(e.target.value)} style={sel}><option value="all">Any</option><option value="yes">Has Special</option><option value="no">No Special</option></select></div>
        </div>

        {/* TABLE */}
        <div style={{ overflowX: "auto", borderRadius: 6, border: "1px solid #1a1a28" }}>
          <table style={{ width: "100%", borderCollapse: "collapse", background: "#0a0a14", minWidth: imported ? 1100 : 800 }}>
            <thead><tr style={{ background: "#0c0c18" }}>
              <SortTh label="Name" k="name" w="140px" />
              <SortTh label="Elem" k="element" w="70px" />
              <SortTh label="Rec. Class" k="recClass" w="120px" />
              <SortTh label="Bonus" k="classBonus" w="90px" />
              <SortTh label="Evo" k="evoDiff" w="55px" />
              <SortTh label="Cond" k="evoCond" w="55px" />
              <th style={{ padding: "8px 6px", fontSize: 10, color: "#444460", textTransform: "uppercase", letterSpacing: "0.06em", fontWeight: 600, borderBottom: "2px solid #1a1a28", width: 40, textAlign: "center", background: "#0c0c18", position: "sticky", top: 0, zIndex: 2 }}>Imp</th>
              {imported && <><th style={{ padding: "8px 2px", borderBottom: "2px solid #1a1a28", width: 1, background: "#151525", position: "sticky", top: 0, zIndex: 2 }} /><SortTh label="Growth" k="growth" w="80px" align="right" /><SortTh label="Dng" k="dungeonLevel" w="50px" align="right" /><SortTh label="Class" k="classLevel" w="90px" /><SortTh label="Action" k="" w="70px" /></>}
            </tr></thead>
            <tbody>
              {filtered.map(({ db, my }, i) => {
                const name = db?.name || my?.name || "???";
                const key = name + i;
                const isExp = expanded === key;
                const evolved = isEvolved(my);
                const rowBg = i % 2 === 0 ? "#0a0a14" : "#0c0c18";
                const hasData = !!my;
                const dimmed = imported && !hasData;
                return [
                  <tr key={key} onClick={() => setExpanded(isExp ? null : key)} style={{ cursor: "pointer", background: isExp ? "#12121e" : rowBg, opacity: dimmed ? 0.4 : 1, transition: "background 0.1s" }} onMouseEnter={e => e.currentTarget.style.background = "#14142a"} onMouseLeave={e => e.currentTarget.style.background = isExp ? "#12121e" : rowBg}>
                    <td style={{ padding: "6px 6px", borderBottom: "1px solid #111120", fontWeight: 600, fontSize: 12, color: "#d0d0e8" }}>
                      <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
                        {name}
                        {imported && my?.unlocked && !evolved && <StatusBadge label="unevolved" color="#e8a05d" outline />}
                        {imported && my?.unlocked && evolved && <StatusBadge label="evolved" color="#4ade80" outline />}
                        {imported && hasData && !my?.unlocked && <StatusBadge label="locked" color="#ef4444" outline />}
                      </div>
                    </td>
                    <td style={{ padding: "6px 6px", borderBottom: "1px solid #111120" }}><ElemBadge el={db?.element || my?.element || "?"} /></td>
                    <td style={{ padding: "6px 6px", borderBottom: "1px solid #111120", fontSize: 11, color: "#8888aa" }}>{db?.recClass || "-"}</td>
                    <td style={{ padding: "6px 6px", borderBottom: "1px solid #111120", fontSize: 11, color: "#777790" }}>{db?.classBonus || "-"}</td>
                    <td style={{ padding: "6px 6px", borderBottom: "1px solid #111120" }}>{db ? <EvoBadge diff={db.evoDiff} /> : "-"}</td>
                    <td style={{ padding: "6px 6px", borderBottom: "1px solid #111120", fontSize: 11, color: "#777790", textAlign: "center" }}>{db ? db.evoDiff.match(/\((.+)\)/)?.[1] || "-" : "-"}</td>
                    <td style={{ padding: "6px 6px", borderBottom: "1px solid #111120", textAlign: "center", fontSize: 11 }}>{db?.improve ? <span style={{ color: "#4ade80" }}>✓</span> : <span style={{ color: "#333" }}>—</span>}</td>
                    {imported && <>
                      <td style={{ padding: 0, borderBottom: "1px solid #111120", width: 1, background: "#151525" }} />
                      <td style={{ padding: "6px 6px", borderBottom: "1px solid #111120", textAlign: "right", fontFamily: "monospace", fontSize: 12, fontWeight: 600, color: hasData ? (my.growth > 25000 ? "#ffd700" : my.growth > 10000 ? "#ddbb44" : "#aab") : "#333" }}>{hasData ? fmt(my.growth) : "—"}</td>
                      <td style={{ padding: "6px 6px", borderBottom: "1px solid #111120", textAlign: "right", fontFamily: "monospace", fontSize: 12, color: hasData ? "#55aaff" : "#333" }}>{hasData ? fmt(my.dungeonLevel) : "—"}</td>
                      <td style={{ padding: "6px 6px", borderBottom: "1px solid #111120", fontSize: 11 }}>{hasData && evolved ? <span style={{ color: "#d0d0e8" }}>{my.class} <span style={{ color: "#555" }}>Lv{my.classLevel}</span></span> : <span style={{ color: "#333" }}>—</span>}</td>
                      <td style={{ padding: "6px 6px", borderBottom: "1px solid #111120", fontSize: 10, color: "#666680" }}>{hasData ? (my.action || "—") : "—"}</td>
                    </>}
                  </tr>,
                  isExp && (
                    <tr key={key + "-d"} style={{ background: "#0e0e1a" }}>
                      <td colSpan={imported ? 12 : 7} style={{ padding: "12px 16px", borderBottom: "1px solid #1a1a2a" }}>
                        <div style={{ display: "flex", flexWrap: "wrap", gap: 20 }}>
                          <div style={{ flex: "1 1 300px" }}>
                            <div style={{ fontSize: 10, fontWeight: 700, color: "#555570", textTransform: "uppercase", letterSpacing: "0.1em", marginBottom: 6 }}>Wiki Reference</div>
                            {db ? <div style={{ display: "grid", gridTemplateColumns: "auto 1fr", gap: "3px 12px", fontSize: 11 }}>
                              <span style={{ color: "#555" }}>Wiki:</span> <a href={wikiUrl(db.name)} target="_blank" rel="noopener noreferrer" onClick={e => e.stopPropagation()} style={{ color: "#8888dd", textDecoration: "underline", textDecorationColor: "#444" }}>{db.name} ↗</a>
                              <span style={{ color: "#555" }}>Unlock:</span> <span style={{ color: "#aab" }}>{db.unlock}</span>
                              <span style={{ color: "#555" }}>Evo Difficulty:</span> <span><EvoBadge diff={db.evoDiff} /></span>
                              <span style={{ color: "#555" }}>Rec. Class:</span> <span style={{ color: "#c9a0ff" }}>{db.recClass}</span>
                              <span style={{ color: "#555" }}>Class Bonus:</span> <span style={{ color: "#aab" }}>{db.classBonus}</span>
                              <span style={{ color: "#555" }}>Improvable:</span> <span style={{ color: db.improve ? "#4ade80" : "#666" }}>{db.improve ? "Yes" : "No"}</span>
                              <span style={{ color: "#555" }}>Special:</span> <span style={{ color: db.special !== "-" ? "#e0d0ff" : "#444" }}>{db.special}</span>
                            </div> : <span style={{ color: "#444", fontSize: 11 }}>Not in database — may be an evolved name</span>}
                          </div>
                          {imported && my && <div style={{ flex: "1 1 300px" }}>
                            <div style={{ fontSize: 10, fontWeight: 700, color: "#555570", textTransform: "uppercase", letterSpacing: "0.1em", marginBottom: 6 }}>My Pet Data</div>
                            <div style={{ display: "grid", gridTemplateColumns: "auto 1fr", gap: "3px 12px", fontSize: 11 }}>
                              <span style={{ color: "#555" }}>Growth:</span> <span style={{ color: "#ffd700", fontWeight: 600 }}>{fmt(my.growth)}</span>
                              <span style={{ color: "#555" }}>Dungeon Lv:</span> <span style={{ color: "#55aaff" }}>{fmt(my.dungeonLevel)}</span>
                              <span style={{ color: "#555" }}>Class:</span> <span style={{ color: evolved ? "#4ade80" : "#666" }}>{evolved ? `${my.class} Lv${my.classLevel}` : "Not evolved"}</span>
                              <span style={{ color: "#555" }}>Stats:</span> <span><span style={{ color: "#c66" }}>HP {fmt(my.hp)}</span>{" · "}<span style={{ color: "#e83" }}>ATK {fmt(my.attack)}</span>{" · "}<span style={{ color: "#48d" }}>DEF {fmt(my.defense)}</span>{" · "}<span style={{ color: "#4c8" }}>SPD {fmt(my.speed)}</span></span>
                              <span style={{ color: "#555" }}>Equipment:</span> <span style={{ color: "#777" }}>{[my.weapon, my.armor, my.accessory].filter(e => e && e !== "none" && e !== "").join(" · ") || "None"}</span>
                              <span style={{ color: "#555" }}>Action:</span> <span style={{ color: "#777" }}>{my.action || "—"}</span>
                              <span style={{ color: "#555" }}>Unlocked:</span> <span style={{ color: my.unlocked ? "#4ade80" : "#ef4444" }}>{my.unlocked ? "Yes" : "No"}</span>
                              {my.improvement && <><span style={{ color: "#555" }}>Improvement:</span> <span style={{ color: "#c9a0ff" }}>{my.improvement}</span></>}
                              {my.other && <><span style={{ color: "#555" }}>Other:</span> <span style={{ color: "#888" }}>{my.other}</span></>}
                            </div>
                          </div>}
                          {imported && my && db && <div style={{ flex: "1 1 200px" }}>
                            <div style={{ fontSize: 10, fontWeight: 700, color: "#555570", textTransform: "uppercase", letterSpacing: "0.1em", marginBottom: 6 }}>Analysis</div>
                            <div style={{ fontSize: 11, color: "#888", lineHeight: 1.6 }}>
                              {!evolved && my.unlocked && <div style={{ color: "#e8a05d" }}>⚠ Unlocked but not evolved · Evo difficulty: <EvoBadge diff={db.evoDiff} /></div>}
                              {evolved && !db.recClass.includes("Wildcard") && !db.recClass.includes("All") && !db.recClass.split("/").some(c => c.trim() === my.class) && <div style={{ color: "#ddbb44" }}>⚡ Class mismatch: you have {my.class}, wiki recommends {db.recClass}</div>}
                              {evolved && db.recClass.split("/").some(c => c.trim() === my.class) && <div style={{ color: "#4ade80" }}>✓ Evolved into recommended class</div>}
                              {db.improve && (!my.improvement || my.improvement === "" || my.improvement === "0" || my.improvement.toLowerCase() === "no") && <div style={{ color: "#55aaff" }}>↑ Token improvable (not yet improved)</div>}
                              {db.special !== "-" && <div style={{ color: "#aa88dd" }}>★ {db.special}</div>}
                            </div>
                          </div>}
                        </div>
                      </td>
                    </tr>
                  ),
                ];
              })}
              {filtered.length === 0 && <tr><td colSpan={imported ? 12 : 7} style={{ padding: 40, textAlign: "center", color: "#333350" }}>No pets match your filters.</td></tr>}
            </tbody>
          </table>
        </div>
        <div style={{ marginTop: 10, fontSize: 10, color: "#2a2a40", textAlign: "right" }}>Click any row for details{imported ? " · DB pets dim if not in your roster" : " · Import data for roster comparison"}</div>
      </div>
    </div>
  );
}
