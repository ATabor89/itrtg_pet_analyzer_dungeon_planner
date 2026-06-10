# ITRTG Save File Format — Findings

Working notes for deserializing a full ITRTG save. Started 2026-06-09 from
`ManualSave_2026-06-09.txt` plus same-session in-game exports (Pet Stats, Main
Stats, Pet Equips, Dungeon Teams, Next Ats) used as ground truth for
cross-referencing.

Regenerate the expanded tree at any time:

```powershell
.\expand_save.ps1            # writes save_expanded.txt (+ save_layer1.txt, decompressed.bin)
```

## Container format (outer → inner)

1. The save file is base64 text **with 2 extra characters prepended** (here
   `V2`). Strip the first 2 characters before decoding. (Purpose unknown —
   possibly a version tag or junk salt. Worth checking against a second save.)
2. Base64-decode. Result: `[0..3]` little-endian `int32` = uncompressed length
   (here 292,296), `[4..]` = a standard **gzip** stream.
3. Gunzip → ASCII text which is itself **base64 again**.
4. Base64-decode → "layer 1" plaintext: a serialized object tree (see grammar).

So: `strip2(b64( len_le32 ++ gzip( b64( tree ) ) ))`.

## Serialization grammar

- A **struct** is `key:value;key:value;...`.
- Keys are single/double letters in declaration order `a..z` then `A..Z`;
  fields added in later game versions use zero-padded numeric keys
  (`001:`, `002:`, … `041:`).
- A key with **no colon and no value** (e.g. `...;c;d:1;`) is an empty field.
- **Nested structs are base64-encoded** and stored as the value.
- **Lists** are `&`-joined base64 elements (each element usually a struct).
- Numbers: invariant-culture ints/doubles, scientific notation (`7.3E+185`).
  Booleans: `True`/`False`.
- In some prose/log strings `:` is escaped as `(` (e.g. the server-time log in
  the root `006` field).

## Top-level map (root struct)

| key | meaning | evidence |
|-----|---------|----------|
| `c` | save unix timestamp (s) | 1781053129 ≈ 2026-06-09; `005` = same in ms |
| `e` | shadow clones block | `a`=current 10,000,061, `b`=max 10,000,000 ✓ |
| `g` | god title string | "Strongest Entity in the Universe?" |
| `h` | creations (28) | |
| `i` | monuments-ish (29) | |
| `j`,`k` | more building/might blocks (28/34) | |
| `p` | god-power block | `j`=available GP 1297 ✓, `v`=GP spent 60,852 ✓, `F`=total might 100,983 ✓; `Z.002`=crystal power 4,183 ✓ |
| `s` | god name | "RedactedAccount" |
| `y`,`z` | achievements (168 each: flag + id) | |
| `W` | player name | "RedactedGod" |
| `X` | **the whole pet system** | see below |
| `Z`,`001..006` | Steam id, account names, server timestamps | |

(Unlabeled root keys not yet investigated: `a`,`b`,`d` big doubles, `A`,`B`,`D`,
`K`,`O`,`P`,`Q`,`S`,`T`,`V` blocks, etc.)

## `X` — pet system

| key | meaning | evidence |
|-----|---------|----------|
| `b` | list of 158 pets | count matches Pet Stats export exactly, same order |
| `c` | Puny Food count | 123,548 = inventory transcription exact (2nd save) |
| `d` | Strong Food count | 16,276 ✓ exact |
| `e` | Mighty Food count | 7,239 ✓ exact |
| `v` | Chocolate count | 9,989 ✓ exact (was the "10062 unknown" in save 1) |
| `002` | **gem inventory**: list of {`a`=element id, `b`=gem level, `c`=count} | all 7 stacks match the transcription exactly; same element ids as pets (0=N,1=F,2=W,3=E,4=Wi) |
| `x` | list of 8 **campaign slots** | `d` = `&`-joined pet ids (10 per slot), `e` = 43,200,000 ms = 12 h, `f` = total bonus, `c` = timestamp, `i` = RNG seed |
| `y` | pet stones | 267,028 ✓ Main Stats |
| `P` | list of 3 **active dungeon runs** | `a` = dungeon id (2,3,5), `c` = 43,200,000 ms, `d` = depth-ish, seeds in `e`/`j` |
| `Q` | list of 69 — **material inventory** (`a`=item id, `b`=count) | export-confirmed: 117=Ant 192,164 ✓, 159=Strategy Book 2,840 ✓, 166=Honey 787 ✓, 174=Acorn 24,727 ✓. Full id→name table (incl. the prior project's identifications: Herb/Iron Ore/…/Soul of Sylph) lives in `crates/save-parser/src/items.rs` |
| `R` | list of 209 — **owned pet equipment** | see equipment struct |
| `S` | list of 3 — **dungeon teams** | `a` = 6 pet ids (`&`-joined, matched via pet `k`), `b` = dungeon id, `c` = pending loot (item id+count), `d` = depth, `i` = dungeon name ("Scrapyard", "Water Temp", "Forest") |
| `T` | list of 23 | ? (per-entry: id-ish `a`, float `c`, nested gear-like `g`) |
| `Z` | a saved 6-pet team preset | `a` = pet ids, `h` = slot order |
| `028` | list of 737 (id + 2 flags) | ? possibly catalog unlock flags |

## Pet struct (`X.b[i]`)

Confirmed against Salamander / Gnome / Sylph / Vampire and the whole dungeon
team roster:

| key | meaning | evidence |
|-----|---------|----------|
| `a` | display name (see name mapping below) | |
| `g` | **normal level** (resets at rebirth) | matches displayed level exactly for all 7 pets checked 2026-06-10 |
| `j` | **current normal Health** (= 10 × Physical; recomputed live) | Gnome j ↔ displayed Health 36.885e9 / Physical 3.688e9; moved ~+30% between the two saves (Anni Cake accumulation) |
| `o`,`p`,`q`,`r` | **training-clone stats** (Physical/Mystic/Battle/HP) — a snapshot from when training was last configured | ratios exactly match the user's global per-mille settings (1/556/550) and the HP=10×Physical rule; bit-identical across both saves while `j` moved |
| `h` | **current exp toward the next normal level** | matches in-game "Current exp" exactly (Gnome 1.115e12, Fire Fox/Swan 4.949e9); only moves while training |
| `G` | **days partnered** | +1 for every partnered pet between the two saves |
| `H` | **village working experience, in ms** | Lamb 9,375,772,300 ms ↔ ~108d 12h reading (exact to seconds); Santa likewise; explains Cat's old "10920" (≈11 s worked) |
| `k` | **internal pet type id** — the id used by team/campaign lists | team ids resolve: 89=Salamander, 25=Rudolph, 2=Cat, 0=Mouse, 803=Serow |
| `l` | unlocked flag | locked pets `False` ✓ export Unlocked column |
| `m` | timer ms: 86,400,000 (locked) / 34,976,500 (all unlocked pets) | shared countdown — next growth tick? |
| `v` | **dungeon team slot** (1–6, 0 = none) | matches all 18 team members ✓ |
| `w` | dungeon sub-struct, see below | |
| `E` | **growth** (float; exports round it). Stored *without* the Magic Egg ×1.3 — true base growth. | 66841.359… → "66,841" ✓ for every pet; Pandora's box `E` = 44334.321043064, the exact base the Magic Egg validation derived (export shows 57,635 = ×1.3 rounded) |
| `F` | **partner pet type id** (999 = none) | 158/158 match export Partner column; pairs are mutual (Cat↔Dog, Vampire↔Succubus, Gnome↔Dwarf) |
| `G` | partner-related counter (bond level?) | only nonzero when F≠999 |
| `H` | ? (only Cat: 10,920) | |
| `d`,`e`,`f`,`n`,`s`,`t`,`u`,`x`,`y`,`z`,`A`–`D` | ? | t: Vampire=1, Dog=4, Penguin=7; y: Gnome=14, Salamander=19, Sylph=24 |

For the normal-stats formula work (display-side model, the Anni Cake
multiplier, open staircase questions), see `normal_stats_investigation.md`.

### Pet dungeon sub-struct (`w`)

| key | meaning | evidence |
|-----|---------|----------|
| `a` | element: 0=Neutral, 1=Fire, 2=Water, 3=Earth, 4=Wind | matches export Element for all checked |
| `b` | dungeon level | Salamander 101 ✓, Sylph 117 ✓ |
| `c` | dungeon exp | |
| `d` | class struct: `a`=class id, `b`=class level, `c`=class exp | Salamander Supporter 24 ✓, Sylph Mage 24 ✓ |
| `e`,`f`,`g` | weapon / armor / accessory **equip instance id** (0 = none) | 704/766/787 = Pet Equips export `Salamander=704,766,787` ✓ |

### Class ids (from pairing all 158 pets with the export, zero conflicts)

0=None, 1=Blacksmith, 2=Alchemist, 3=Adventurer, 4=Defender, 5=Supporter,
6=Rogue, 7=Assassin, 8=Mage.

## `root.x` — the global tracker block

A flat struct of ~360 numeric-keyed counters holding both global statistics
and the per-pet special trackers shown in pet tooltips. Identified keys are
in `crates/save-parser/src/model.rs` (`trackers` module), confirmed by
diffing the two saves against tooltip readings — every user-predicted
day-over-day delta matched (Meteor `234`: 4548.117 → 4572.111, exactly +24
campaign hours; Mule `310`: 123 → 124; Chocobear `089`: 4826.09 → 4874.09;
Serow `324`: 7037 → 7552; …).

Per-pet trackers: 089 Chocobear banked hours, 169 Pandora feedings (observed
**negative** −28 just after rebirth), 185 Earth Eater planets total, 186
Aether boss kills (the Aether Ring "+28"), 216 Pignata bashes, 218 God Power
campaign hours, 234 Meteor campaign hours, 259 Caterpillar materials, 310
Mule quests, 311 Gold Dragon bonus growth, 324 Serow items saved, 336 Bag
bonus growth. Globals: 013 AFK clones killed, 049 day-pet-challenge multi,
071 lucky draws opened, 074 crystal power, 078/079/080 dungeon
bosses/enemies/rooms, 129 total might.

Not found yet: Anni Cake's current bonus % is stored nowhere obvious —
candidate `x.138` (a seconds counter: 950.0 h at save 2 vs the 949% reading;
needs one synchronized reading of bonus% at save time to confirm or kill).

## `X.T` — the crafting roster (23 entries)

One entry per crafting pet: `a` = crafter pet id, `c` = crafting progress
(float; resets when an item completes), `k` = 0 alchemist / 1 blacksmith.
Alchemists: `e` = **material id being brewed** (Bee → 166 Honey, Phoenix →
15 Health Potion, Void → 16 Health Potion X, Supreme Taco & Bunny Girl → 32
Wise Talisman, …). Blacksmiths: `d` = **equipment type id being forged** —
in save 2: six smiths on 51 (Magic Stick), one on 54 (Magic Pot), one on 86
(Ear Muffs), exactly matching the +6/+1/+1 inventory drift observed during
the manual transcription. Blacksmith `l` = unidentified small id (queue or
last-forged type?).

## Equipment struct (`X.R[i]`)

Example: instance 704 = "Inferno Sword + 10, SSS, Wind gem lv 10":

| key | meaning | evidence |
|-----|---------|----------|
| `a` | item type id (21 = Inferno Sword, 51 = Magic Stick, 47 = …) | |
| `b` | plus level (+10/+20/+13); stat multiplier = 1 + 0.05×plus | ✓ export + wiki scaling rule |
| `c` | quality: A=5 baseline, ±1/step (8=SSS, 7=SS, 6=S, 5=A, 4=B verified; 3=C, 2=D inferred); stat multiplier = 1 + (c−5)×0.1 | ✓ export + wiki ("C +8 ⇒ 0.8 × 1.4"; "SSS +20 ⇒ 1.3 × 2.0 = 2.6") |
| `d`, `h` | instance id (both equal in every entry checked) | referenced by pet `w.e/f/g` and Pet Equips export |
| `e` | 20 when export shows "(20)" suffix, else 0 | awakening/max-plus cap? |
| `f` | gem level (10) | ✓ |
| `g` | gem element (4 = Wind gem, same element ids) | ✓ |
| `i` | 0 always so far | ? |

## Save name → export name mapping

The save stores the pet's *display name*; exports normalize. Differences found
(index = position in both lists):

```
Pandora's box → Pandora        Thunder Ball → Raiju        Young Basilisk → Basilisk
Pixie Goatmother → PixieGoat   Feather Pile → Owl          Seed → Yggdrasil
Supreme Taco → StaleTortilla   Flying Cloud → Cloud        Fire Fox → Firefox
Chicken → Egg                  Answer → Question           Hermit Crab → Crab
Pigñata → Pignata              Rudolph → Reindeer          Tödlicher Löffel → Spoon
Vesuvius → Volcano             Treasure → Mimic            Black Hole Chan → BHC
Holy ITRTG Book → Book         V. Pheasant → VermilionPheasant
Baby Carno → Carno             Gray1 → GrayChild1          Gray2 → GrayChild2
Dark Gift → Baphomate
```

Plus all multi-word names have spaces stripped in exports (`Ancient Mimic` →
`AncientMimic`, `Mist Sphere` → `MistSphere`, …). Note `Pigñata` has a real
`ñ` in the save (UTF-8).

## Cross-reference checks that passed

- Growth (`E`) matches Pet Stats export for every pet (export rounds).
- All three equip slots match Pet Equips export ids exactly.
- Dungeon teams: membership (`S[].a` ↔ pet `k`), slots (pet `v`), dungeon
  names, depths — all match the Dungeon Teams export.
- GP, GP spent, total might, crystal power, pet stones, strategy books, ants,
  acorns all found at expected values.

## Open questions / next steps

- Pet fields `d,e,f,g,h,j,n,o,p,q,r,s,t,u,x,y,z,A–D,H` — meaning unknown.
  `g/h/j/o/p/q/r` presumably the stat-bonus / total-exp accumulators behind the
  computed HP/Attack/etc. shown in exports (those stats are *not* stored).
- HP/Attack/Defense/Speed/elemental affinities from the Pet Stats export do
  not appear literally in the save → derived at runtime. If we ever need them,
  we either keep using the export or reverse the formulas.
- `X.v` (10,062), `X.z` (13,253,888), `X.T` (23 entries), `X.028` (737 ids),
  pet `t`/`u` — unidentified.
- Material id ↔ name: mostly solved in `crates/save-parser/src/items.rs`
  (prior-project table + export-confirmed + the 2026-06-10 full inventory
  transcription: 16/17 = Health Potion X/S, 19 = Antidote — correcting the
  prior table's "Nothing"; real Nothing is 119 — and 21 = Torch).
  Remaining: the five count-1 ids {130,160,164,167,168} ↔ {Not Nothing,
  Absolutely Nothing, Aether Ring +28, Food Journal One, Food Journal Two}
  (set known, assignment unknown), and the T4 materials 131–135 ↔
  {Mythril, Ocean/Sun/Sky/Jungle Stone} (all still count 32 in both saves —
  needs the counts to diverge, e.g. after crafting with one of them).
- Equipment *type* id ↔ name: solved for everything equipped — 32 types in
  `items.rs::equipment_type_name`, derived by joining Pet Stats gear strings
  ↔ Pet Equips instance ids ↔ the save's `R` instance→type map (zero vote
  conflicts), plus Storm Bow via its unique count. Remaining: the nine
  unequipped 1-count types {5,8,22,23,26,30,41,52,56} ↔ {Iron Pot, Water
  Spear, Flood Spear, Leeching Sword, Tree Axe, Hurricane Bow, Flame Armor,
  Flood Armor, Tree Bracelet}, and 44 = {Magic Hammer | Storm Ring} —
  equipping any of them once would resolve it.
- The 2 leading junk chars: constant? random? Compare with another save.
- Re-serialization (writing a save) untested — only needed if we ever want to
  edit saves, which is out of scope for the planner.
- Token/evolution state (export "Other" column: GnomeFinal, SylphFinal, fight
  counts, day counters) — not yet located in the pet struct (candidates: the
  unknown `t`,`u`,`x`,`y`,`z`,`A–D`,`H` fields).

## Rust parser

`crates/save-parser` implements all of this: `container` (outer layers),
`tree` (generic grammar → `Node`), `model` (typed `SaveFile` with pets,
equipment, materials, teams, campaigns; unknown fields stay reachable via the
raw `Node`s). Its `tests/real_save.rs` cross-checks the reference save against
the exports — when a field meaning gets identified, promote it from the raw
tree into the typed model there. `cargo run -p save-parser --bin save-dump --
<save> [--tree]` replaces `expand_save.ps1` for exploration.

## Files here

- `ManualSave_2026-06-09.txt` — first raw save (input).
- `* Export.txt` — same-session in-game exports (ground truth).
- `second_save/` — 2026-06-10 save + re-exports + **full manual inventory
  transcription** (materials, gems, equipment counts) — the richest ground
  truth so far.
- `normal_stats_investigation.md` — the normal-stats formula work.
- `expand_save.ps1` — decoder/expander script (superseded by
  `save-dump --tree` but kept for history).
- `save_layer1.txt`, `decompressed.bin`, `save_expanded.txt` — derived
  artifacts (regenerable; the expanded tree is the one to read).
