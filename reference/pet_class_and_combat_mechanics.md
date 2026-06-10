# Pet Classes, Dungeon Combat, and Stat Formulas

Captured 2026-06-10 from in-game tooltips/help text (transcribed by the user)
and the wiki's "Dungeons" page ("Dungeon Stats and Combat" section). This is
the mechanics reference for dungeon-stat math, class behavior, and exp
formulas. For *normal* (non-dungeon) pet stats, see
`save_file_deserialization/normal_stats_investigation.md`.

## Class stat modifiers and abilities

Standard modifiers only — a pet's unique evo bonus generally adds more
percentage per class level on top of these.

| Class | HP | Attack | Defense | Speed | Ability |
|---|---|---|---|---|---|
| Blacksmith | 120% | 110% | 120% | 40% | Forges new equip and improves existing equip. Forging speed and quality +2% × CL. Buffs pet damage of non-mages in dungeons by 1% × CL. |
| Alchemist | 80% | 100% | 80% | 110% | Brews potions for dungeons; crafts lower-tier items into higher tiers. Crafting speed +2% × CL. |
| Adventurer | 100% | 100% | 100% | 100% | +2% × CL to all campaign stats. |
| Defender | 120% | 40% | 150% | 40% | 50% chance to protect other pets; takes (10 + CL)% of the damage for them (maxed at CL 25). After CL 25, HP +1% per additional CL. |
| Supporter | 80% | 70% | 100% | 130% | Heals: `(5 + Attack × (CL × 0.1) + (1 + CL, max 20) × (target max HP / 100)) × supporter bonus`. From CL 10, reduces damage taken of all party members by 1% × CL (max 50%). |
| Rogue | 80% | 120% | 60% | 160% | Rare-loot chance and loot amount +3% × CL. |
| Assassin | 70% | 130% | 70% | 140% | +5% × CL damage to single targets. |
| Mage | 40% | 150% | 40% | 120% | Deals only (35% + 1% × CL) damage, but hits 3 + 1-per-20-CL enemies at once. Elemental damage only (Fire/Water/Wind/Earth). |

(CL = class level.)

## Dungeon stat formulas (wiki, confirmed phrasing)

```
Health             = ((10 + 24*DL) * (1 + TotalGrowth/200000) * EquipMod * DojoMod + StrategyRoomMod) * ClassMod
Attack/Def/Speed   = ((1 + 2.4*DL) * (1 + TotalGrowth/200000) * EquipMod * DojoMod + StrategyRoomMod) * ClassMod
```

- Equivalent help-text phrasing: every 2,000 total growth = +1% dungeon stats.
- Defender exception: the *Health* class mod gains +(CL−25)/100 above CL 25
  (e.g. CL 55 ⇒ 1.2 + 0.30 = 1.5 for Health only).
- Strategy Room mod applies only to pets currently in a dungeon team
  (0 otherwise).
- Equip mods are summed across the three pieces before applying
  (e.g. +169% +72% −24% ⇒ ×3.17).

### Element levels

- Neutral pet: each element level = 0.75 × DL.
- Non-neutral pet: own element = 50 + 3 × DL, weak element = −50, other two = 0.
  (Matches the save: a fresh pet starts with +50 own / −50 weak — pet struct
  `w` confirms the constants.)
- Equipment elemental bonuses are *added*, then Dojo/Strategy Room element
  multipliers are applied (negatives get worse when multiplied).
- Weakness wheel (weak to the element above it): fire→water, wind→fire,
  earth→wind, water→earth.

## Combat (wiki)

- Speed: at 0 you act once/round; up to 500 ⇒ (speed/5)% chance of a 2nd
  action; 500–1500 ⇒ ((speed−500)/10)% chance of a 3rd; cap 3 actions at 1500.
  Faster actors tend to act first.
- Action priority: special ability → heal (Supporter, if someone is hurt
  enough) → attack a random enemy (Mages hit several).
- Single-target damage pipeline:
  1. Attacker's Attack.
  2. − half of defender's Defense.
  3. Element used: attacker's own; Neutral attackers pick the element with the
     largest (attacker level − defender level) gap.
  4. × elemental factor `(1 + A/100) / (1 + D/100)`; if D < 0, add |D| to A
     and set D = 0.
  5. × defense factor `1 − D/(D + 200)`.
  6. Class-specific factors.
  7. + speed damage: if attacker speed > defender speed:
     (difference / 2) × Dojo speed-damage bonus(?).
  8. Back-row non-Mage/non-Sniper: −20% damage.

## Pet detail screen derived values (in-game tooltips)

- **Damage Potential** (damage classes): vs 0 def / 0 elements per turn.
  `Attack * (1 + attack element / 100) * class damage multi * hits * pet
  specials + speed damage`. Counter/burn not included. Observed: Sylph 91,541
  (heavily equipment-dependent).
- **Survivability** (all pets): vs an attack from an enemy with 100 element
  vs the pet's lowest element. `Health / def multiplier * (1 + lowest
  element / 100)`. Observed: Sylph 907.
- **Heal Potential** (Supporters): appears to be the theoretical max (the
  detail-page value), distinct from the live "Heal Power" shown on the
  dungeon screen. Observed pairs (potential vs live):
  - Salamander: 3,083 + 57.57% target HP vs Heal Power 1,473 + 27.5%
    (plus unique: heals all party members 4.41% max HP at start of turn).
  - Frog: 2,354 + 44.9% vs Heal Power 1,180 + 22.51%.

## Experience tables (wiki)

- Dungeon level: new pets are DL 1; 10 exp to DL 2.
  Exp for DL n−1 → n: `10*(n-1)^2` (n < 10), `10*(n-1)^2.25` (n ≥ 10).
- Class level: newly evolved = CL 1; 3,000 exp to CL 2.
  Exp for CL n−1 → n: `1000 + 2000*(n-1)^2` (n > 1).
  A drained pet can sit at CL 0; 1,000 exp back to CL 1.
- Dungeon exp is split among surviving party members at room end (Ego Swords
  excepted). Evolved pets usually receive equal class exp, except:
  Adventurers gain class exp from campaigns, Alchemists from crafting;
  Blacksmiths gain from both crafting and dungeons.
- Crafting/campaign class exp:
  `250 * (1 + growth/20000) * hours * (1 + crafter speed / 100)`
  (pet stone / ChP Adventurer-XP purchases applied afterward).
