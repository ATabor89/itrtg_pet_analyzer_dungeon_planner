/**
 * Add stats, resistances, and new equipment to dungeon_recommendations.yaml
 * Uses simple string replacement approach.
 */

const fs = require('fs');
const path = require('path');

const yamlPath = path.join(__dirname, '..', 'data', 'dungeon_recommendations.yaml');
let yaml = fs.readFileSync(yamlPath, 'utf-8');
// Normalize line endings to \n for processing
yaml = yaml.replace(/\r\n/g, '\n');

// ============================================================================
// Helper: Replace an item block by appending stats after its last property
// ============================================================================
function addStatsToItem(yaml, itemKey, stats) {
  // Match the item key line and all subsequent indented lines (6-space indent)
  // Item blocks end with a blank line or another 4-space-indent item
  const pattern = new RegExp(
    `(    ${itemKey}:\\n(?:      [^\\n]*\\n)*?)(?=\\n    |\\n  \\w|\\n#|\\n\\n)`,
    ''
  );
  const match = yaml.match(pattern);
  if (!match) {
    console.log(`  WARNING: Could not find item: ${itemKey}`);
    return yaml;
  }

  // Don't add if already has stats
  if (match[1].includes('stats_a0:')) {
    return yaml;
  }

  let statsBlock = '';
  if (stats.hp !== null && stats.hp !== undefined) {
    statsBlock += `      stats_a0:\n`;
    statsBlock += `        hp: ${stats.hp}\n`;
    statsBlock += `        atk: ${stats.atk}\n`;
    statsBlock += `        def: ${stats.def}\n`;
    statsBlock += `        spd: ${stats.spd}\n`;
  }

  if (stats.res) {
    statsBlock += `      resistances:\n`;
    for (const [key, val] of Object.entries(stats.res)) {
      statsBlock += `        ${key}: ${val}\n`;
    }
  }

  if (stats.special_effect) {
    statsBlock += `      special_effect: "${stats.special_effect}"\n`;
  }

  if (stats.notes) {
    statsBlock += `      notes: "${stats.notes}"\n`;
  }

  if (statsBlock) {
    const original = match[1];
    const updated = original.trimEnd() + '\n' + statsBlock;
    yaml = yaml.replace(original, updated);
  }

  return yaml;
}

// ============================================================================
// STAT DATA
// ============================================================================

const allStats = {
  // === WEAPONS ===
  // T1
  iron_sword:      { hp: 2,  atk: 15, def: 3,   spd: -5,  res: null },
  fire_sword:      { hp: 0,  atk: 20, def: -5,  spd: 0,   res: { fire: 10, water: -30 } },
  water_spear:     { hp: 0,  atk: 15, def: 0,   spd: 0,   res: { water: 10, earth: -30 } },
  wood_axe:        { hp: 5,  atk: 10, def: 0,   spd: 0,   res: { earth: 10, wind: -30 } },
  feather_bow:     { hp: 0,  atk: 11, def: -3,  spd: 7,   res: { wind: 10, fire: -30 } },
  forging_hammer:  { hp: 0,  atk: 10, def: 0,   spd: 0,   res: { fire: 10 }, special_effect: "Increases Blacksmith crafting quality and speed up to 16.67%" },
  walking_stick:   { hp: 0,  atk: 1,  def: 0,   spd: 9,   res: { earth: 10 }, special_effect: "Increases campaign bonus for equipped pet up to 16.67%" },
  iron_pot:        { hp: 0,  atk: 0,  def: 10,  spd: 0,   res: { fire: 2, water: 2, wind: 2, earth: 2 }, special_effect: "Increases Alchemist crafting speed up to 16.67%" },
  training_sword:  { hp: 4,  atk: 6,  def: 0,   spd: 0,   res: { fire: -5, water: 10, wind: -5, earth: -5 }, special_effect: "Increases exp gained, steals up to 33.33% of gained exp from other pets" },
  howling_knives:  { hp: -3, atk: 7,  def: -3,  spd: 7,   res: { fire: -10, wind: 10 }, special_effect: "Lowers enemy defense up to 0.33% per hit (non-effective on Mages)" },

  // T2
  steel_sword:     { hp: 5,  atk: 18, def: 7,   spd: -10, res: null },
  flame_sword:     { hp: 0,  atk: 30, def: -10, spd: 0,   res: { fire: 20, water: -30 } },
  flood_spear:     { hp: 0,  atk: 20, def: 0,   spd: 0,   res: { water: 20, earth: -30 } },
  tree_axe:        { hp: 7,  atk: 13, def: 0,   spd: 0,   res: { earth: 20, wind: -30 } },
  storm_bow:       { hp: 0,  atk: 15, def: -5,  spd: 10,  res: { wind: 20, fire: -30 } },
  shaping_hammer:  { hp: 0,  atk: 20, def: 0,   spd: 0,   res: { fire: 20 }, special_effect: "Increases Blacksmith crafting quality and speed up to 33.33%" },
  journeying_stick:{ hp: 0,  atk: 2,  def: 0,   spd: 18,  res: { earth: 20 }, special_effect: "Increases campaign bonus for equipped pet up to 33.33%" },
  steel_pot:       { hp: 0,  atk: 0,  def: 20,  spd: 0,   res: { fire: 5, water: 5, wind: 5, earth: 5 }, special_effect: "Increases Alchemist crafting speed up to 33.33%" },
  leeching_sword:  { hp: 8,  atk: 12, def: 0,   spd: 0,   res: { fire: -7, water: 20, wind: -7, earth: -7 }, special_effect: "Increases exp gained, steals up to 66.67% of gained exp from other pets" },
  thundering_knives:{ hp: -6, atk: 14, def: -6, spd: 14,  res: { fire: -20, wind: 20 }, special_effect: "Lowers enemy defense up to 0.67% per hit (non-effective on Mages)" },

  // T3
  titanium_sword:  { hp: 8,  atk: 22, def: 10,  spd: -15, res: { fire: 5, water: 5, wind: 5, earth: 5 } },
  inferno_sword:   { hp: 0,  atk: 40, def: -15, spd: 0,   res: { fire: 30, water: -30 } },
  tsunami_spear:   { hp: 0,  atk: 25, def: 0,   spd: 0,   res: { water: 30, earth: -30 } },
  forest_axe:      { hp: 10, atk: 15, def: 0,   spd: 0,   res: { earth: 30, wind: -30 } },
  hurricane_bow:   { hp: 0,  atk: 22, def: -10, spd: 13,  res: { wind: 30, fire: -30 } },
  magic_hammer:    { hp: 0,  atk: 30, def: 0,   spd: 0,   res: { fire: 20 }, special_effect: "Increases Blacksmith crafting quality and speed up to 50%" },
  magic_stick:     { hp: 0,  atk: 3,  def: 0,   spd: 27,  res: { earth: 30 }, special_effect: "Increases campaign bonus for equipped pet up to 50%" },
  magic_pot:       { hp: 0,  atk: 0,  def: 30,  spd: 0,   res: { fire: 7, water: 7, wind: 7, earth: 7 }, special_effect: "Increases Alchemist crafting speed up to 50%" },
  ego_sword:       { hp: 10, atk: 20, def: 0,   spd: 0,   res: { fire: -10, water: 30, wind: -10, earth: -10 }, special_effect: "Increases exp gained, steals up to 100% of gained exp from other pets" },
  bursting_knives: { hp: -9, atk: 21, def: -9,  spd: 21,  res: { fire: -30, wind: 30 }, special_effect: "Lowers enemy defense up to 1% per hit (non-effective on Mages)" },

  // T4
  mythril_shield:  { hp: 30, atk: -20, def: 55, spd: -10, res: { fire: 5, water: 5, wind: 5, earth: 5 } },
  sun_sword:       { hp: 0,  atk: 65, def: -10, spd: 0,   res: { fire: 90, water: -90 } },
  ocean_spear:     { hp: 0,  atk: 55, def: 0,   spd: 0,   res: { water: 90, earth: -90 } },
  jungle_axe:      { hp: 10, atk: 45, def: 10,  spd: -10, res: { earth: 90, wind: -90 } },
  sky_bow:         { hp: 0,  atk: 40, def: -10, spd: 25,  res: { wind: 90, fire: -90 } },
  demonic_sword:   { hp: null },
  divine_bow:      { hp: null },
  legend_hammer:   { hp: 0,  atk: 45, def: 0,   spd: 0,   res: { fire: 30 }, special_effect: "Increases Blacksmith crafting quality and speed up to 100%" },
  legendary_stick: { hp: 0,  atk: 5,  def: 0,   spd: 40,  res: { earth: 60 }, special_effect: "Increases campaign bonus for equipped pet up to 100%" },
  legendary_pot:   { hp: 0,  atk: 0,  def: 45,  spd: 0,   res: { fire: 15, water: 15, wind: 15, earth: 15 }, special_effect: "Increases Alchemist crafting speed up to 100%" },
  soul_sword:      { hp: 15, atk: 30, def: 0,   spd: 0,   res: { fire: -20, water: 60, wind: -20, earth: -20 }, special_effect: "Increases exp gained, steals up to 100%. Increases god HP regen by up to 5%" },
  exploding_knives:{ hp: -15, atk: 40, def: -15, spd: 35, res: { fire: -60, wind: 60 }, special_effect: "Lowers enemy defense and elements up to 1% per hit (non-effective on Mages, elements capped at 50%)" },

  // T5
  ele_twin_dagger: { hp: -10, atk: 80, def: -10, spd: 45, res: { fire: 50, water: 50, wind: 50, earth: 50 } },

  // === ARMOR ===
  // T1
  iron_vest:       { hp: 15, atk: 0,  def: 10, spd: -5, res: null },
  fire_vest:       { hp: 5,  atk: 8,  def: 7,  spd: 0,  res: { fire: 20, water: -40 } },
  water_vest:      { hp: 15, atk: 0,  def: 5,  spd: 0,  res: { water: 20, earth: -40 } },
  wooden_vest:     { hp: 10, atk: 0,  def: 10, spd: 0,  res: { earth: 20, wind: -40 } },
  feather_vest:    { hp: 6,  atk: 0,  def: 7,  spd: 7,  res: { wind: 20, fire: -40 } },

  // T2
  steel_armor:     { hp: 25, atk: 0,  def: 15, spd: -10, res: null },
  flame_armor:     { hp: 7,  atk: 13, def: 10, spd: 0,   res: { fire: 40, water: -40 } },
  flood_armor:     { hp: 20, atk: 0,  def: 10, spd: 0,   res: { water: 40, earth: -40 } },
  tree_armor:      { hp: 15, atk: 0,  def: 15, spd: 0,   res: { earth: 40, wind: -40 } },
  storm_armor:     { hp: 8,  atk: 0,  def: 11, spd: 11,  res: { wind: 40, fire: -40 } },

  // T3
  titanium_armor:  { hp: 35, atk: 0,  def: 20, spd: -15, res: { fire: 5, water: 5, wind: 5, earth: 5 } },
  inferno_armor:   { hp: 10, atk: 15, def: 15, spd: 0,   res: { fire: 60, water: -40 } },
  tsunami_armor:   { hp: 25, atk: 0,  def: 15, spd: 0,   res: { water: 60, earth: -40 } },
  forest_armor:    { hp: 20, atk: 0,  def: 20, spd: 0,   res: { earth: 60, wind: -40 } },
  hurricane_armor: { hp: 10, atk: 0,  def: 15, spd: 15,  res: { wind: 60, fire: -40 } },

  // T4
  mythril_armor:   { hp: 40, atk: 0,  def: 35, spd: -5,  res: { fire: 5, water: 5, wind: 5, earth: 5 } },
  sun_armor:       { hp: 10, atk: 30, def: 25, spd: 5,   res: { fire: 90, water: -90 } },
  ocean_armor:     { hp: 30, atk: 5,  def: 25, spd: 10,  res: { water: 90, earth: -90 } },
  jungle_armor:    { hp: 30, atk: 5,  def: 35, spd: 0,   res: { earth: 90, wind: -90 } },
  sky_armor:       { hp: 15, atk: 5,  def: 25, spd: 25,  res: { wind: 90, fire: -90 } },
  demonic_armor:   { hp: null },
  divine_armor:    { hp: null },
  robe_of_economy: { hp: 20, atk: 0,  def: 15, spd: 30,  res: { fire: -50, water: -20, wind: 90, earth: -20 }, special_effect: "With Mana Cape on an alchemist, decreases T4 material crafting costs" },

  // === ACCESSORIES ===
  // T1
  iron_ring:       { hp: 5,  atk: 5,   def: 5,   spd: -5,  res: null },
  fire_gloves:     { hp: -5, atk: 15,  def: -5,  spd: 5,   res: { fire: 25, water: -50 } },
  water_necklace:  { hp: 0,  atk: 5,   def: 5,   spd: 0,   res: { water: 25, earth: -50 } },
  wood_bracelet:   { hp: 5,  atk: 0,   def: 5,   spd: 0,   res: { earth: 25, wind: -50 } },
  feather_ring:    { hp: 0,  atk: -5,  def: 0,   spd: 15,  res: { wind: 25, fire: -50 } },

  // T2
  steel_ring:      { hp: 7,  atk: 7,   def: 7,   spd: -7,  res: null },
  flame_gloves:    { hp: -7, atk: 21,  def: -7,  spd: 7,   res: { fire: 50, water: -50 } },
  flood_necklace:  { hp: 0,  atk: 7,   def: 7,   spd: 0,   res: { water: 50, earth: -50 } },
  tree_bracelet:   { hp: 7,  atk: 0,   def: 7,   spd: 0,   res: { earth: 50, wind: -50 } },
  storm_ring:      { hp: 0,  atk: -7,  def: 0,   spd: 21,  res: { wind: 50, fire: -50 } },

  // T3
  titanium_ring:   { hp: 10, atk: 10,  def: 10,  spd: -10, res: { fire: 10, water: 10, wind: 10, earth: 10 } },
  inferno_gloves:  { hp: -10, atk: 30, def: -10, spd: 10,  res: { fire: 75, water: -50 } },
  tsunami_necklace:{ hp: 0,  atk: 10,  def: 10,  spd: 0,   res: { water: 75, earth: -50 } },
  forest_bracelet: { hp: 10, atk: 0,   def: 10,  spd: 0,   res: { earth: 75, wind: -50 } },
  hurricane_ring:  { hp: 0,  atk: -10, def: 0,   spd: 30,  res: { wind: 75, fire: -50 } },
  alchemist_cape:  { hp: 6,  atk: 6,   def: 6,   spd: 6,   res: { fire: 13, water: 13, wind: 13, earth: 13 }, special_effect: "Allows alchemists to craft Tier 3 materials" },

  // T4
  mythril_ring:    { hp: 25, atk: 0,   def: 30,  spd: -10, res: { fire: 25, water: 25, wind: 25, earth: 25 } },
  sun_gloves:      { hp: -10, atk: 45, def: -10, spd: 20,  res: { fire: 150, water: -100 } },
  ocean_necklace:  { hp: 0,  atk: 25,  def: 20,  spd: 0,   res: { water: 150, earth: -100 } },
  jungle_bracelet: { hp: 25, atk: 0,   def: 25,  spd: 0,   res: { earth: 150, wind: -100 } },
  sky_ring:        { hp: 0,  atk: -10, def: 0,   spd: 55,  res: { wind: 150, fire: -100 } },
  demonic_ring:    { hp: null },
  divine_ring:     { hp: null },
  mana_cape:       { hp: 12, atk: 12,  def: 12,  spd: 12,  res: { fire: 24, water: 24, wind: 24, earth: 24 }, special_effect: "Allows alchemists to craft Tier 3 and Tier 4 materials if quality is SSS" },
};

// ============================================================================
// Step 1: Fix names and tiers
// ============================================================================
yaml = yaml.replace(
  '    bursting_knives:\n      name: "Bursting Knives"',
  '    burning_knives:\n      name: "Burning Knives"'
);

yaml = yaml.replace(
  /ele_twin_dagger:[\s\S]*?tier: 4/,
  (m) => m.replace('tier: 4', 'tier: 5')
);

yaml = yaml.replace(
  /ele_twin_dagger:[\s\S]*?element: All/,
  (m) => m.replace('element: All', 'element: Neutral')
);

// ============================================================================
// Step 2: Add stats to each existing item
// ============================================================================
let count = 0;
for (const [key, stats] of Object.entries(allStats)) {
  if (stats.hp === null) continue; // skip items without wiki stats
  yaml = addStatsToItem(yaml, key, stats);
  count++;
}
console.log(`Added stats to ${count} existing items`);

// ============================================================================
// Step 3: Add new equipment items
// ============================================================================

// Add Ear Muffs after Howling Knives in T1 weapons
const earMuffsBlock = `
    ear_muffs:
      name: "Ear Muffs"
      type: Weapon
      tier: 1
      element: Neutral
      source: crafted_by_hamster
      dungeon_relevant: false
      special_effect: "Improves crafting speed of any Blacksmith or Alchemist (max +10% at SSS+20)"
      stats_a0:
        hp: 0
        atk: 0
        def: 0
        spd: 0
      notes: "Can only be forged by Hamster. Purely a crafting utility item."
`;

// Insert after howling_knives block, before T2 comment
yaml = yaml.replace(
  /(    howling_knives:[\s\S]*?)(\n\n    # -- Tier 2 --)/,
  (match, p1, p2) => p1 + '\n' + earMuffsBlock + p2
);

// Add premium/unique/quest weapons after ele_twin_dagger
const newWeaponsBlock = `
    # -- Unique / Premium / Quest Weapons --
    gram:
      name: "Gram"
      type: Weapon
      tier: 4
      element: Neutral
      source: patreon_boss_reward
      max_quantity: 1
      dungeon_relevant: true
      special_effect: "Increases dungeon XP by 20% + 2% per Patreon boss defeated. Power scales with bosses defeated."
      stats_a0:
        hp: 0
        atk: 18
        def: 0
        spd: 0
      resistances:
        fire: 7
        water: 7
        wind: 7
        earth: 7
      notes: "Obtained stats are ~73% atk, 82 all res. Atk = 18% + level/2%. Disables Flying Eyeball and Bat bonuses if any teammate equips it."

    candy_cane:
      name: "Candy Cane"
      type: Weapon
      tier: 4
      element: Neutral
      source: premium_event
      max_quantity: 2
      dungeon_relevant: true
      special_effect: "Increases campaign bonus for equipped pet by up to 150%"
      stats_a0:
        hp: 0
        atk: 5
        def: 0
        spd: 40
      resistances:
        fire: 10
        water: 10
        wind: 10
        earth: 10
      notes: "2022 Christmas event. Available via IAP or pet stones. High speed makes it decent for dungeons."

    magic_egg_weapon:
      name: "Magic Egg"
      type: Weapon
      tier: 4
      element: Neutral
      source: premium_event
      max_quantity: 2
      dungeon_relevant: true
      special_effect: "Increases current growth and growth from dungeon events by 30%. Shortcuts evolution growth requirements."
      stats_a0:
        hp: 10
        atk: 30
        def: 10
        spd: 10
      resistances:
        fire: 10
        water: 10
        wind: 10
        earth: 10
      notes: "2023 Easter event. Available via IAP or pet stones. Well-rounded stats make it a solid dungeon weapon."

    godly_hammer:
      name: "Godly Hammer"
      type: Weapon
      tier: 4
      element: Earth
      source: premium_event
      max_quantity: 2
      dungeon_relevant: true
      special_effect: "Increases building speed by 25% (additive) and 10% (multiplicative)"
      stats_a0:
        hp: 0
        atk: 0
        def: 35
        spd: 10
      resistances:
        fire: 0
        water: -120
        wind: 0
        earth: 160
      notes: "2023 Halloween event. Available via IAP or pet stones. Massive +160 Earth element ideal for token-improved Rabbit."

    haposti:
      name: "Haposti"
      type: Weapon
      tier: 4
      element: Neutral
      source: tavern_quest
      max_quantity: 1
      dungeon_relevant: true
      special_effect: "Confers the bonuses of a Hammer, a Pot, and a Stick simultaneously"
      stats_a0:
        hp: 15
        atk: 15
        def: 15
        spd: 15
      resistances:
        fire: 10
        water: 10
        wind: 10
        earth: 10
      notes: "Tavern quest reward. Only 1 obtainable. Balanced stats plus triple crafting/campaign utility."

    wonder_axe:
      name: "WonderAxe"
      type: Weapon
      tier: 5
      element: Neutral
      source: tavern_quest
      max_quantity: 1
      dungeon_relevant: true
      special_effect: "Non-Mage user has 10% chance to ignore enemy defense"
      stats_a0:
        hp: 10
        atk: 75
        def: 10
        spd: 10
      resistances:
        fire: 50
        water: 50
        wind: 50
        earth: 50
      notes: "Tavern quest reward. Only 1 obtainable. Best-in-slot weapon for most non-Mage roles."
`;

// Find the end of the weapons section (before armor section)
yaml = yaml.replace(
  /(    ele_twin_dagger:[\s\S]*?)(\n\n  armor:)/,
  (match, p1, p2) => p1 + '\n' + newWeaponsBlock + p2
);

// Add premium/unique/quest armor after robe_of_economy
const newArmorBlock = `
    # -- Unique / Premium / Quest Armor --
    rune_patch:
      name: "Rune Patch"
      type: Armor
      tier: 4
      element: Neutral
      source: ultimate_stats_challenge
      max_quantity: 1
      dungeon_relevant: true
      special_effect: "Crafting speed/quality by USC completed (max 25%). At max, also +25% non-mage or +50% mage damage if equipped by blacksmith."
      stats_a0:
        hp: 30
        atk: 10
        def: 20
        spd: 0
      resistances:
        fire: 10
        water: 10
        wind: 10
        earth: 10
      notes: "Not upgradeable by crafting. Obtained stats: 41% hp, 14% atk, 27% def, 78 all res. Only 1 obtainable."

    creators_vest:
      name: "Creators Vest"
      type: Armor
      tier: 4
      element: Fire
      source: premium_event
      max_quantity: 2
      dungeon_relevant: true
      special_effect: "Increases creating speed by 25% (additive) and 10% (multiplicative)"
      stats_a0:
        hp: 10
        atk: 35
        def: 0
        spd: 0
      resistances:
        fire: 160
        water: 0
        wind: -120
        earth: 0
      notes: "2023 Summer event. Available via IAP or pet stones. Very high attack for armor."

    merry_mantle:
      name: "Merry Mantle"
      type: Armor
      tier: 4
      element: Water
      source: premium_event
      max_quantity: 2
      dungeon_relevant: true
      special_effect: "Increases campaign bonus for equipped pet by up to 150%"
      stats_a0:
        hp: 17
        atk: 0
        def: 28
        spd: 0
      resistances:
        fire: -15
        water: 80
        wind: 5
        earth: 5
      notes: "2023 Christmas event. Available via IAP or pet stones. Strong defensive armor."

    learning_coat:
      name: "Learning Coat"
      type: Armor
      tier: 4
      element: Water
      source: premium_event
      max_quantity: 2
      dungeon_relevant: true
      special_effect: "Increases class XP gain of pet outside dungeon by 50% (additive to Owl/Bat bonuses)"
      stats_a0:
        hp: 15
        atk: 15
        def: 15
        spd: 15
      resistances:
        fire: 20
        water: 160
        wind: 20
        earth: -150
      notes: "2023 Valentine event. Available via IAP or pet stones. Balanced stats but extreme earth vulnerability."

    enlightment_vest:
      name: "EnlightmentVest"
      type: Armor
      tier: 5
      element: Neutral
      source: tavern_quest
      max_quantity: 1
      dungeon_relevant: true
      special_effect: "Increases crafting speed by 50% (multiplicative)"
      stats_a0:
        hp: 10
        atk: 10
        def: 10
        spd: 75
      resistances:
        fire: 50
        water: 50
        wind: 50
        earth: 50
      notes: "Tavern quest reward. Only 1 obtainable. Massive speed bonus."
`;

yaml = yaml.replace(
  /(    robe_of_economy:[\s\S]*?)(\n\n  accessories:)/,
  (match, p1, p2) => p1 + '\n' + newArmorBlock + p2
);

// Add premium/unique/quest accessories after mana_cape
const newAccessoriesBlock = `
    # -- Unique / Premium / Quest Accessories --
    spectrometers:
      name: "Spectrometers"
      type: Accessory
      tier: 4
      element: Neutral
      source: premium_event
      max_quantity: 2
      dungeon_relevant: true
      special_effect: "Allows any alchemist to craft T3/T4 materials. Chance to not consume resources (stacks with Bottle)."
      stats_a0:
        hp: 6
        atk: 19
        def: 6
        spd: 19
      resistances:
        fire: 5
        water: 5
        wind: 15
        earth: 15
      notes: "2022 Summer event. Available via IAP or pet stones."

    master_gloves:
      name: "Master Gloves"
      type: Accessory
      tier: 4
      element: Wind
      source: premium_event
      max_quantity: 2
      dungeon_relevant: false
      special_effect: "Increases Blacksmith crafting speed by up to 30%"
      stats_a0:
        hp: 25
        atk: 0
        def: 0
        spd: 25
      resistances:
        fire: -160
        water: 10
        wind: 140
        earth: 10
      notes: "2022 Halloween event. Available via IAP or pet stones. Crafting utility only - terrible fire res makes it unsuitable for dungeons."

    growing_love_pendant:
      name: "Growing Love Pendant"
      type: Accessory
      tier: 4
      element: Earth
      source: premium_event
      max_quantity: 2
      dungeon_relevant: false
      special_effect: "Once per hour, gives up to 1 base growth per evolved pet. Wearer must not be in top 10 by total growth."
      stats_a0:
        hp: 55
        atk: 0
        def: 0
        spd: 0
      resistances:
        fire: -40
        water: -40
        wind: -40
        earth: 220
      notes: "2024 Valentine event. Available via IAP or pet stones. Growth utility - extreme negative res makes it poor for dungeons."

    christmas_boots:
      name: "Christmas Boots"
      type: Accessory
      tier: 4
      element: Wind
      source: premium_event
      max_quantity: 2
      dungeon_relevant: true
      special_effect: "Increases campaign bonus for equipped pet by up to 150%"
      stats_a0:
        hp: 0
        atk: 0
        def: 0
        spd: 50
      resistances:
        fire: -20
        water: 20
        wind: 80
        earth: -20
      notes: "2024 Christmas event. Available via IAP or pet stones. Excellent speed accessory for dungeons."
`;

// Find the end of accessories section (before items catalog)
yaml = yaml.replace(
  /(    mana_cape:[\s\S]*?)(\n\n# -{10,})/,
  (match, p1, p2) => p1 + '\n' + newAccessoriesBlock + p2
);

// ============================================================================
// Step 4: Update header comment to include T5
// ============================================================================
yaml = yaml.replace(
  '# Equipment tiers: T1 (Basic), T2 (Shaping), T3 (Magic), T4 (Legendary)',
  '# Equipment tiers: T1 (Basic), T2 (Shaping), T3 (Magic), T4 (Legendary), T5 (Mythic)'
);

// Write back with original Windows line endings
yaml = yaml.replace(/\n/g, '\r\n');
fs.writeFileSync(yamlPath, yaml, 'utf-8');
console.log('Done!');
