# Normal Pet Stats — Investigation (2026-06-10)

Goal: compute a pet's *normal* stats (Physical/Mystic/Battle — the ones
campaigns use), which reset with the normal level at rebirth. This was a
primary motivation for the save deserialization: the exports don't carry
them, but the save does.

Inputs: the second save (`second_save/ManualSave_2026-06-10.txt`) plus
same-day in-game readings the user transcribed (slight Moai growth drift
between readings is expected).

## In-game tooltip (total growth mouseover)

> The stats of a pet are: growth + (Level - 1) * 0.1 * growth / 3. Every 100
> levels the increase rises 0.1 * growth up to a total increase of
> 1 * growth / 3. The strategy room in the pet village increase this and
> change the split into the 3 stats.

Interpretation that fits the data: per normal level the pet gains
0.1×growth in (total) stats during the first 100 levels, 0.2×growth during
the next 100, … rising every 100 levels and capping at 1.0×growth per level
from level ~1000 on. Each of the three stats gets a third of the total.
So for L > ~1000: `inc(L) ≈ 450 + (L − 901) = L − 451` (in units of growth),
and per-stat base = `growth × (1 + inc(L)) / 3`.

## Confirmed save-field meanings (pet struct)

| field | meaning | evidence |
|---|---|---|
| `g` | **normal level** | matches displayed level exactly for all 7 pets checked (Gnome 13,724; Anni Cake 10,861; Fire Fox & Swan 2,052; …) |
| `j` | **current Physical stat × 10** | Gnome j=36,881,717,678 ↔ displayed 3.688e9; matches all 6 pets to display precision |
| `p` | = 556 × `o` (exact, all pets) | derived, not independent |
| `q` | = 550 × `o` (exact, all pets) | derived |
| `r` | = 10 × `o` (exact, all pets) | derived |
| `o` | unknown accumulator | the only independent value behind p/q/r; **not** proportional to current stats across pets (Fire Fox vs Swan at the same level differ in o/growth ratio), so it likely encodes history (e.g. accumulated stat exp), not a snapshot |
| `h` | level/exp-state related | Fire Fox and Swan (both level 2,052) have *identical* h = 4,949,217,393.54; not monotonic in level across pets, so not a pure level function — possibly current exp toward next level |

Notable: p/q = 556/550 = 1.010909… for every pet. This is *not* the
Strategy Room Physical/Mystic ratio (104/103 = 1.00971), so p and q are not
simply the current Physical/Mystic stats. The displayed Physical/Mystic
ratio *does* equal the SR ratio (verified on all pets).

## Display-side model (fits to <0.1%)

```
PerStat(Physical) ≈ growth × (1 + inc(L)) / 3 × SR_physical × M
```

- SR_physical here = 1.04 (user allocated SR level 1's 10% as 104/103/103;
  default is 103/103/103).
- M = global multiplier ≈ **11.015** for this account, decomposing almost
  exactly as `(1 + 9.49) × 1.05 = 11.0145`:
  - Anni Cake's stat boost, "Current Bonus: 949%" (10% + 0.1%×CL per hour in
    food campaigns, capped 3653%, **resets on rebirth** — so M is
    rebirth-dependent!).
  - ×1.05 presumed from the GP "stats multi" purchase (Main Stats: "GP spent
    for stats multi: 2500").
- Using exact j values from the save, M comes out 11.0150–11.0153 for the
  level-10k+ pets and 11.0166–11.0204 for lower-level pets — i.e. there is a
  small level-dependent residual, meaning the `inc(L)` staircase boundaries
  above are off by a bit (off-by-one block edges or rounding inside the
  staircase). Worth nailing down with a low-level pet series (levels 1–1000)
  where the staircase dominates.

### Reference data points (displayed values, 2026-06-10)

| Pet | Level | Growth | Physical | Mystic | Battle |
|---|---|---|---|---|---|
| Gnome | 13,724 | 72,766 | 3.688e9 | 3.652e9 | 3.652e9 |
| Hedgehog | 11,276 | 57,714 | 2.385e9 | 2.362e9 | 2.362e9 |
| Anni Cake | 10,861 | 53,237 | (total stats 6.308e9) | | |
| Aether | 10,156 | 48,744 | 1.806e9 | 1.789e9 | 1.789e9 |
| Fire Fox | 2,052 | 28,913 | 176.957e6 | 175.256e6 | 175.256e6 |
| Swan | 2,052 | 24,354 | 149.055e6 | 147.622e6 | 147.622e6 |

Gnome total-stats mouseover: 10.993e9 (= P+M+B ✓).

## The feedback loop to god stats

Pet stats feed the god's multipliers: "Each stat of all pets adds 0.01% to
your own stats." Observed (2026-06-10):

- Physical: 20.680e9% — "From pet stats 837.654e6% × 24.69 (campaigns)";
  rebirth multi from stats (999% + 8,795% from campaigns) × 1 (Dojo) = 9,794%.
- Mystic: 20.481e9% — pet stats part 829.600e6% × 24.69.
- Battle: same as Mystic.

## Anni Cake (stat-multiplier special)

> Boosts the physical, mystic and battle stats of all your pets by
> (10% + 0.1% × class level) per hour in food campaigns. Resets on rebirth,
> capped at 3653%. Current Bonus: 949%.

Snapshot: Hunger 80.62%, Level 10,861, Total Growth 53,237, Total Stats
6.308e9, campaign bonuses +365% (food) +365% (multiplier), total reward for
campaign 465%. Evo bonus: +1.38% × CL to all campaigns if Adventurer.

## Leveling (how normal levels are gained)

- Training: allocate shadow clones to fight the pet; exp → levels.
  Llysnafedda special: when it trains, it shares exp with every other pet.
- The Level campaign levels the assigned pets.
- Normal level (and Anni Cake bonus, and therefore most of M) resets at
  rebirth.

## Open questions

- Exact `inc(L)` staircase boundaries (the <0.5% residual at low levels).
- Meaning of `o` (and thus p/q/r) and `h`; why the constants 556/550/10.
- Decompose M exactly (is the 1.05 really the GP stats multi? does Museum or
  ChP contribute?).
- Mystic/Battle presumably stored nowhere (derived from Physical via SR
  ratios) — j only covers Physical; verify on an account with asymmetric SR.
