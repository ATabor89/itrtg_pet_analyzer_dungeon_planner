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

Two platform variants wrap the **same** inner `base64(tree)`:

### Steam: `strip2(b64( len_le32 ++ gzip( b64( tree ) ) ))`

1. The save file is base64 text **with 2 extra characters prepended** (here
   `V2`). Strip the first 2 characters before decoding. (Purpose unknown —
   possibly a version tag or junk salt. Worth checking against a second save.)
2. Base64-decode. Result: `[0..3]` little-endian `int32` = uncompressed length
   (here 292,296), `[4..]` = a standard **gzip** stream.
3. Gunzip → ASCII text which is itself **base64 again**.
4. Base64-decode → "layer 1" plaintext: a serialized object tree (see grammar).

### Kongregate / web ("Save to file"): `b64( LZF( b64(tree) ) )`

No junk prefix; **LZF** compression (Marc Lehmann's liblzf, the `CLZF2.cs` the
community editor uses — *not* gzip/deflate/zlib/LZString) instead of gzip+length.
Decoded 2026-06-16 from a fresh-game export (`Kongregate/`). Notes:

- LZF has no header: a stream of chunks, each led by one control byte —
  `<32` = literal run of `ctrl+1` bytes, `≥32` = back-reference (`crate::lzf`).
- The "Save to file" export **concatenates two `base64(tree)` blobs** (live +
  backup) separated by non-base64 bytes; decode only the leading base64 run =
  the first, complete tree.
- This file format is **not redactable by our tooling on its own** and carries
  identity → gitignored (`**/*_SaveItRtG.txt`). It decodes fine; we just don't
  re-encode LZF.

`container::decode_container` auto-detects the variant (`ContainerFormat`).
Re-encoding always emits the Steam (`V2` gzip) container, which **both** builds
accept on import — so an edited web save loads in either. (No LZF *compressor*
is implemented; it isn't needed for that round trip.)

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
| `h` | creation-adjacent list (28: a=id, b≈9.99M decreasing, c=1) | shape known, meaning not pinned |
| `i` | **creations** (29, Shadow Clone … Universe — `items::creation_name`) | `d` = current amount (Shadow Clone entry = the clone count ✓), `e` = clone cost, `i` = next-at ✓, `g` = **total created** — matches the in-game mouseovers (Earthlike Planet exactly 3,893; Shadow Clones 38.6M; Galaxy 980k vs ~1.1M read later). Divinity-bought copies do *not* increment `g` (hence the frozen Earthlike Planet count, and why Earth Eater's 7.35M eaten ≫ created). `f/h/j` = flags (auto-create settings?) |
| `j`,`k` | building blocks (28/34); `j.b` = decreasing thresholds (deltas −500·n), parallel in shape to `h` — **not** the monument upgrades | |
| `D` | **monuments** (9, Mighty Statue … White Hole — `items::monument_name`) | `b` = **current level** (equals next-at once reached — clones spill onward; Black Hole mid-build: level 110 vs next-at 140 ✓ user expectation), `g` = next-at, `h` = spread ratio; `f`=building, `c`=clones allocated, `d`=progress. Monument *upgrade* levels/next-ats are stored elsewhere — still unlocated |
| `V` | **mights** (14 — `items::might_name`) | `b` = **current level** (specials all 64 = the user's "at level 64" ✓; Σ levels = 3,200 = the in-game White Tiger unlock progress ✓ — 25,000 needed, resets per rebirth), `m` = next-at, `n` = spread ratio. Specials 8–13: `e`=True, `g` = base unleash duration (+1 s/level: 30+64 = 94 s ✓), `i`/`j`/`k` = HP-recovery/Attack/Mystic % (Transformation Aura 200/200/200 ✓). "Total Might" (100,983) is the all-rebirths global at `x.129`/`p.F` |
| `O` | statistics mirror | `O.030` = light clones ✓ |
| `009` | **SpaceDim** (Light Dimension) | `009.a` = light clones (= the count allocated to the one active element); `009.b` = the 20-element list (see its section); `009.e` = spacedim total. Per-element levels confirmed against the 2026-06-13 notes (Fusion Torch 18→70, Dyson 22→23, Quantum Genesis 2→6) |
| `K` | **Divinity Generator** | `K.g` = total divinity (huge double); `K.l` = list of 3 upgrades, `b` = level (all three 81→188 between the 2026-06-13 saves ✓), `c` = cost-ish, `d` = accumulator, `g` = multiplier (1/2/2) |
| `P` | **current god fight** (≠ `X.P` active dungeon runs) | `P.c` = current god number = highest **P. Baal defeated + 1** (44→49 as the user went from "defeated 43" to "defeated 48" ✓); `P.b` = that god's HP-scale double (grows ~10× per few gods), `P.d` = `P.b`/10 |
| `T` | **Baal Slayer block** | `T.h` = **unspent Baal Power** (0→334 ✓); `T.f` = list of 5 staggered countdown timers (1 h apart); `T.k` = list of 5 TBS tracks; see open items |
| `024` | quests block | `024.d` = Quest Points (4,553 ✓ s2 export) |
| `025` | fishing block | `025.a` = Fish Power (1,270,255 ≈ displayed 1.270e6 ✓) |
| `p` | god-power block, see its own section below | `j`=available GP, `v`=GP spent, `F`=total might, `002`=crystal power |
| `s` | **linked account login name** (Steam/Kongregate) — NOT the god name | player-confirmed; redacted in committed saves |
| `y`,`z` | achievements (168 each: flag + id) | |
| `W` | **in-game god (deity) name** | player-confirmed; redacted in committed saves |
| `X` | **the whole pet system** | see below |
| `Z` (bool), `001`=Steam id64, `002`=Steam persona name, `003`=account/guest id (`a_…`), `004`=Steam display name, `005`=save ms-timestamp, `006`=init log | identity in `001..004` redacted in committed saves; `005`/`006` kept | player-confirmed |

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
| `z` | **cumulative pet stones spent** | fresh-save diff: `y` −750,000 / `z` +750,000 when buying 2 Dungeon Loot + 1 Dungeon Exp (= 2·275k + 200k) ✓ |
| `Y` (capital) | **free experience** — *candidate* | 30,855 (advanced save) vs 26 (fresh) — behaves like an accumulating pool; the player reported free exp = 26 on the fresh save. Best-guess, pending in-game confirmation (the other 26-valued scalars `root.e.g`=0-in-advanced and `032.b.j` fit worse). |
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
| `c` | dungeon exp — **current toward the next DL**, resets on level-up | matches the in-game "current / needed" display exactly (Salamander 147,749, Succubus 130,099); requirement = 10·DL^2.25 (DL≥10), verified to the digit |
| `d` | class struct: `a`=class id, `b`=class level, `c`=class exp (current toward next CL, same semantics) | Salamander was CL 24 at 1,144,938/1,153,000 in save 1, CL 25 with a reset counter in save 2; requirement = 1000+2000·CL² verified exactly |
| `e`,`f`,`g` | weapon / armor / accessory **equip instance id** (0 = none) | 704/766/787 = Pet Equips export `Salamander=704,766,787` ✓ |

### Class ids (from pairing all 158 pets with the export, zero conflicts)

0=None, 1=Blacksmith, 2=Alchemist, 3=Adventurer, 4=Defender, 5=Supporter,
6=Rogue, 7=Assassin, 8=Mage.

## `root.p` — the god-power block

Numeric keys (`001`–`035`) are siblings of the letter keys inside `p`.
Confirmed (cross-save diff vs the two Main Stats exports):

| key | meaning | evidence |
|---|---|---|
| `j` | available GP | 1,297 → 1,662, +365 = GP/h gain between saves ✓ |
| `v` | total GP spent | 60,852 in both ✓ |
| `h` | GP-purchased **creating** speed % | 45,000 in both 2026-06-13 saves (CS left untouched) |
| `i` | GP-purchased **building** speed % | 45,000 → 50,000 exactly as the user upgraded BS (and only BS) between the 2026-06-13 saves — disambiguates `h`=creating from `i`=building |
| `q` | **creation count from god power** (the milestone-relevant number; export "Creation Count: 166" = base 1 + this; equipped-crystal bonuses are separate) | 165 in both saves ✓ |
| `F` | total might (lives on its own screen, mirrored here) | 100,983 ✓ |
| `002` | crystal power (only changes on rebirth — equipped crystals convert then) | 4,183 ✓ |
| `C` | **statistics multi** — exactly 2^50 = 1.1259e15, matching the displayed "1.125e15 x" | three-way lock with `017`/`019` = 50 doublings × 50 GP = the 2,500 GP spent |
| `017`, `019` | **NOT** the doubling count — `017` = **Dungeon Loot** %, `019` = **Dungeon Exp** % (pet-stone upgrades), confirmed 2026-06-16 by a fresh-save purchase diff. The doubling count is stored elsewhere; `C` = 2^50 captures the multi directly. | |
| `r`,`s`,`t`,`u` | **unused-GP stat allocation %** = **physical / mystic / battle / creating** respectively | resolved 2026-06-13: the user skewed the split to 25/21/22/27 (physical/mystic/battle/creating) and the fields moved to `r`=25, `s`=21, `t`=22, `u`=27 ✓ |
| `E` | **TBS double-points chance** % | **confirmed** 2026-06-16 (save-edit `p.E`=91 → in-game "Chance for double points in TBS: 91%"). Its old "pair" `025` is actually Camp Exp Boost, not a TBS twin. |
| `D` | **TBS extra white-area pixels** | **confirmed** 2026-06-16 (`p.D`=6 → in-game "Extra Pixels for the white area: 6"). `p.I` (also 3 here) is **not** a twin — it's **Pet Tokens** (token-count diff, 5→6). |
| `I` | **Pet Tokens** (unlock/evolve currency) | **confirmed** 2026-06-16 (fresh-save diff 5→6). |
| `023` | **Class Change Tokens** | **confirmed** 2026-06-16 (fresh-save diff 8→10). |
| `y`,`z` | TBS keep-on-rebirth % (user: 80%) | both 80; **`y` is NOT the displayed value** — setting `p.y`=61 left "TBS progress kept after rebirthing" at 80%, so the live field is `z` (or a cap). Retest by editing `z`. |
| `w`,`e` | TBS level-loss-on-miss / "Chance to lose progress" | `w` PERM 20, `e` drifts (20→19). Editing `p.w`=17 showed "Chance to lose progress in TBS: 39%" — the display looks *derived* (≈ base − `w`), so `w` reduces lose-chance but isn't shown literally. Not fully pinned. |
| `027` | **identity unknown** — the "= `j` + 18" claim was a two-save coincidence (2026-06-13: `027`=62→67 while `j`=2931→1511, so `027` ≠ `j`+18). It moved +5, same delta as the P. Baal kills (43→48), but its absolute (62/67) doesn't match a Baal count and `P.c` already tracks that cleanly — so leave unidentified, don't repeat the x.138 mistake | |
| `003` | delta (+21) matches Lucky Draws *opened* delta | absolute value 7,659 ≠ 3,053 though — id unclear |
| `L`, `S`, `013` | ms timers, all advanced by the same +38.8e6 ms | plausibly time-since-rebirth (~12 h at save 1) |
| `T`, `012` | 44 → 43, decrementing counter | countdown of something |
| booleans (`k,l,o,p,B,G,J,U,V,Y,Z,008,010,011,033,034`) | GP toggle purchases — the user owns pet half stats, Improved 'Next At', Preselect Might (among others); `H`=False/`035`=0 candidates for the unowned Divinity GP buy | not individually mapped |

Note on terminology: the GP "stats multi" purchase doubles the *statistics
multi* (the rebirth-multiplier input tracked on the statistics page) — it
has nothing to do with pet stats. And the unused-GP allocation boosts the
four *god* stats. "Stats" now has five meanings in this game.

Other singles found while anchoring exports: `x.k` = rebirths (137),
`O.030` = light clones (68,581 → 68,681 ✓ both saves — bought with Baal
Power, not GP, but mirrored here). `root.O` appears to be another
statistics block (spacedim totals etc. nearby in `009`).

Cheap disambiguation experiments (each = one action + one save): buy one
creating-speed or building-speed level → splits `h`/`i`; buy one
statistics-multi doubling → splits `017`/`019`; skew the GP allocation →
names `r/s/t/u`; change any TBS setting → splits its candidate pair.

TBS pair hypothesis (user): the duplicated values could be current/max,
where max is the GP-purchase cap and current can exceed it via challenge
point upgrades (e.g. the ChP −1%/level rebirth-level-loss upgrade). The
user has maxed the GP side, so a GP purchase can't test this — a ChP
upgrade purchase could.

The save **editor now exists** (`save-edit`, see below): the cleanest way to
nudge resource-gated upgrades is to grant currency and buy/sell in-game, or to
knock a *maxed* field down directly. This already resolved one collision:
`save-edit <in> edited.txt --set p.025 75` dropped the in-game **Camp Exp Boost**
to +75% while the TBS double-points % held at 100%, confirming `p.025` = Camp
Exp Boost and `p.E` = the TBS field (see below). The fresh-save purchase-diff
method (a zero-purchase Kongregate save bought up one upgrade at a time) then
mapped the bulk of the pet-stone and GP upgrades — see the table below and
`Kongregate/experiments.md`.

### Pet-stone *permanent upgrades* also live in `root.p`

Despite the "god-power block" label, `root.p` holds **all** permanent
purchases — god-power buys *and* pet-stone buys sit together in its numeric
keys. Isolated by diffing the five committed saves for keys that never change
(rebirth-independent). The block was **validated** when `p.001` ticked
**5 → 6** between the 06-13 and 06-16 saves — exactly the player buying the
last "Max Crystal".

Most rows below were **confirmed 2026-06-16** by a fresh-zero-purchase
Kongregate save bought up one upgrade at a time, diffing each step (the bulk in
`Kongregate/experiments.md`):

| key | kind | upgrade | confidence |
|-----|------|---------|-----------|
| `p.001` | count | **Max Crystal** (crystals equippable, cap 6) | **Confirmed** (5→6) |
| `p.018` | count | **Inventory Space** (equipment limit, +50/buy) | **Confirmed** (50→100) |
| `p.021` | count | **Item Slot** (dungeon party-item slots, cap 8) | **Confirmed** (3→4) |
| `p.017` | % | **Dungeon Loot** (+25%/buy, cap +50%) | **Confirmed** (0→50) |
| `p.019` | % | **Dungeon Exp** (+25%/buy, cap +50%) | **Confirmed** (0→25) |
| `p.020` | % | **Crafting Boost** (+25%, single) | **Confirmed** (0→25) |
| `p.025` | % | **Camp Exp Boost** (+25%/buy, cap +100%) | **Confirmed** (save-edit diff) |
| `p.010` | bool | **Improved Campaign Cancel** | **Confirmed** (F→T) |
| `p.033` | 0/1 | **Optimal Campaigns** | **Confirmed** (0→1) |
| `p.034` | 0/1 | **Auto Worker Clones** | **Confirmed** (0→1) |
| `p.008` | bool | **Config Half Stats** ("Custom Pet Clones") | **Confirmed** (F→T) |
| `p.Y` | bool | **Pet Half Stats** (GP upgrade; prereq for Config Half Stats) | **Confirmed** (F→T) |
| `p.B`, `p.U` | bool | **Improved Next At** (base, GP) + **Improved Next At For Challenges** — buying the for-challenges upgrade also grants the base one (player confirmed base reads as already-purchased on the GP page), so both flip together; which key is base vs for-challenges is TBD | **Confirmed** (F→T) |
| `p.I` | currency | **Pet Tokens** (5→6 diff) | **Confirmed** |
| `p.023` | currency | **Class Change Tokens** (8→10 diff) | **Confirmed** |
| `p.016` `p.030` `p.014` | — | unidentified, permanent | — |

Plus **`X.032`** (count) = **Crafting Queue Slot** (0→1, cost 500k ✓) — outside
`p`, in the pet block. Not yet bought (need crystals): **Crystal Improve**
(+25%, a *different* field from `p.020`), **Refrigerator** (also needs
Nevermelt Ice). Other one-time GP/pet booleans (Refrigerator, Auto Select Camp,
Dungeon Team, Crystal Slot, …) sit among the remaining `root.p` `True` flags.

Promoted to the typed model: `PermanentUpgrades` gains `dungeon_loot_pct`,
`dungeon_exp_pct`, `crafting_boost_pct` (alongside `max_crystal`,
`inventory_limit`, `item_slots`, `camp_exp_boost_pct`); `SaveFile` gains
`pet_stones_spent` (`X.z`) and `crafting_queue_slots` (`X.032`). The boolean
toggles stay reachable via the raw tree (`root.get_path(&["p","010"])`).

**`p.025` = Camp Exp Boost (the Growth Chamber's missing ×2) — CONFIRMED.** The
chamber sim's adventurer class-XP multiplier is `250 × 4.0`, where maxed Camp
Exp Boost explains one ×2 (`growth_chamber_status.md`); reading `p.025` lets
that be auto-derived (`mult ×= 1 + p.025/100`). Confirmed 2026-06-16 with
`save-edit --set p.025 75`: in-game Camp Exp Boost dropped to **+75%** (one of
its four +25% levels) while the Baal-Slayer double-points chance stayed at
100% — so `p.025` is Camp Exp Boost and the colliding `p.E` is the TBS field,
not a pair. The chamber's *other* ×2 remains unidentified (see
[`project_chamber_class_xp`] / `growth_chamber_status.md`).

The same reasoning held for `p.017`/`p.019` = 50/50, now **confirmed** as
**Dungeon Loot** (`017`) & **Dungeon Exp** (`019`) by a fresh-save purchase diff
(2026-06-16): on a zero-purchase Kongregate save, buying 2 Dungeon Loot moved
`p.017` 0→50 and 1 Dungeon Exp moved `p.019` 0→25. Same diff resolved
**`X.z` = cumulative pet stones spent** (`X.y` down / `X.z` up by exactly
750,000 = 2·275k + 200k). The stat-multi doubling count is stored elsewhere;
`p.C` = 2^50 captures the multi directly.

This fresh-zero-purchase-Kongregate-save diff is the workflow for the rest of
the upgrade fields: bankroll the save (`save-edit --stones`), buy a small,
labelled batch, re-export, diff. See `Kongregate/experiments.md`.

Consumables, for contrast, are **not** here: every consumable pet-stone item
(Elixir, Phoenix Feather, Flying Boots, Torch, bombs, keys, runes, talismans,
Rebirth Bacon, Ale, Strategy Book, …) is a quantity in the material inventory
`X.Q` (or the food fields `X.c/d/e`, chocolate `X.v`, gems `X.002`) — already
decoded into `SaveFile.materials` / foods. `X.013` (list of 8) is the *equipped*
party-item loadout.

## `root.x` — the global tracker block

A flat struct of ~360 numeric-keyed counters holding both global statistics
and the per-pet special trackers shown in pet tooltips. Identified keys are
in `crates/save-parser/src/model.rs` (`trackers` module), confirmed by
diffing the two saves against tooltip readings — every user-predicted
day-over-day delta matched (Meteor `234`: 4548.117 → 4572.111, exactly +24
campaign hours; Mule `310`: 123 → 124; Chocobear `089`: 4826.09 → 4874.09;
Serow `324`: 7037 → 7552; …).

Per-pet trackers: 089 Chocobear banked hours, 169 Pandora feedings (observed
**negative** −28 just after rebirth), 185 Earth Eater planets total
(lifetime; the per-rebirth twin lives at root `018` — identical deltas
across the saves, +42,574, and the 1/s rate matches the user's timeline), 186
Aether boss kills (the Aether Ring "+28"), 216 Pignata bashes, 218 God Power
campaign hours, 234 Meteor campaign hours, 259 Caterpillar materials, 310
Mule quests, 311 Gold Dragon bonus growth, 324 Serow items saved, 336 Bag
bonus growth. Globals: 013 AFK clones killed, 049 day-pet-challenge multi,
071 lucky draws opened, 074 crystal power, 078/079/080 dungeon
bosses/enemies/rooms, 129 total might.

Anni Cake's bonus: stored **directly at root `033`** as a fractional
percent — save 1: 709.0245829717 (exactly the user's predicted "709%"),
save 2: 948.969027416145 (displays as 949%), delta ≈ +239.94 = ~24 h of
food campaigns at 10%/h, fractional residue from early-cancelled campaigns.
Only advances when a campaign completes (or is cancelled-with-rewards).

⚠ Lesson logged: `x.138` was briefly misidentified as this bonus because
`floor(x.138/3600)` happened to equal 949 in save 2 — save 1 disproves it
(911 ≠ 709). A single-save numeric match, even an exact-looking one, is not
identification; always check both saves. `x.138` is back to unidentified
(some food/campaign-time-shaped counter).

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

### Gem-embed tasks carry the owner-pet link (2026-06-13)

A blacksmith *embedding a gem into an already-equipped item* is structurally
different from forging a new one, and that entry is where the
blacksmith↔owning-pet link lives. Worked example: at the 2026-06-13 save 1
Hermit Crab (pet id 35) was embedding a level-10 water gem into **Fairy's**
Titanium Armor:

```
a = 35            crafter pet id (Hermit Crab)
c = 1496524.85    crafting progress
g = { … h = 767 } the equipment INSTANCE being modified (Fairy's armor, id 767)
k = 4             OWNER pet type id (Fairy's k = 4) — the link
l = 4             (same; owner again)
n = { a = 2, b = 10 }   the gem: a = element (2 = Water), b = level (10)
```

So while an item is in the smith it is **checked out of `X.R`** (instance 767
has no standalone `X.R` entry in this save — it is held inside the `g`
substruct of the crafting entry) and the pet's slot reads empty (Fairy's
`w.f` = 0). The game re-equips it on completion because the crafting entry
stores the owner: `k` = the owner pet's type id. This is why `k` here is 4
(Fairy) rather than the 0/1 alchemist/blacksmith flag that every *forging*
entry carries — for a forge the new item just goes to inventory, so no owner
is recorded and `k` is free to mean the class flag. `n` = the gem being set.

## `root.032` — adventure mode (partial)

`032.d` = adventure inventory (id, count); `032.c` = adventure gear list;
`032.H.a` = **researches** (44 entries, id 0 is a placeholder): `a`=id,
`b`=level, `c`=in-progress flag, `d`=progress, `f`=max level. Ids 1–43
match the Main Stats export's "Researches" section in order, verified
43/43; exactly two entries had `c`=1, matching "Research Slots Level: 2".
Notably **id 28 = Pet Stats** ("multiplies the stats your pets gain from
growth, not dungeon stats", +1%/level) — at level 5 this is the ×1.05
factor in the normal-stats global multiplier.

## `root.009` — SpaceDim / Light Dimension (decoded 2026-06-13)

`009.a` = light clones (whatever is allocated to the active element). `009.b`
= the **20-element list**, in the in-game display order:

| key | meaning |
|-----|---------|
| `a` | element id = display order (1 = Controlled Entropy … 20 = Self Replicating AI) |
| `b` | light clones allocated to this element (only the active one is nonzero) |
| `c` | **level** |
| `d` | **next-at** (clones to next level) |
| `e` | progress / accumulated clones toward next level |
| `f` | **spread** (the 20…1 priority value shown in-game) |

Verified end-to-end against the 2026-06-13 notes: save 1 Quantum Genesis
`c`=2, Fusion Torch `c`=18 (b=70,362 — all light clones), Dyson `c`=22; save 2
Fusion Torch `c`=70, Dyson 23, Quantum Genesis 6, etc. — and every `d`/`f`
matches the notes' Next-At/Spread columns. Levels reset per rebirth.

## `root.T` — Baal Slayer (decoded 2026-06-13)

The Baal Slayer (TBS) block. Confirmed: `T.h` = **unspent Baal Power**
(0 → 334 between the two 2026-06-13 saves ✓). `T.f` = list of 5 entries with
`d` = a staggered countdown timer (1 h / 3,600,000 ms apart; all decrement in
real time). `T.k` = list of 5 TBS tracks (`a`≈100, `b`={39,27,25,21,16}→
{40,28,26,22,17} across 06-13→06-16, `c`=1..5, `i`={18,18,18,27,27}) — these
are **not** the on-screen component levels (see `root.S` below).

## `root.S` — Baal Slayer component levels (decoded 2026-06-16)

The five TBS body-part levels, one per letter key, stored as the displayed
level directly. Resolved by setting each part to a **distinct** level for the
2026-06-16 capture (`save_pet_stone_tbs/notes.txt`); every earlier save had all
five at **126** (the "all five 126" reading that the old `T.k` guess failed to
match).

| key | part | 06-16 value |
|-----|------|-------------|
| `S.b` | **Eyes** (player levels these *mirrored* → counts 4× in score) | 125 |
| `S.d` | **Wings** | 127 |
| `S.e` | **Tail** | 128 |
| `S.f` | **Feet** | 130 |
| `S.c` | **Mouth** | 132 |

`S.a` = 99.56472 (constant across every save — not a level), `S.g` = 0:
unidentified. The displayed **score** is derived, not stored —
`4·eyes + wings + tail + feet + mouth = 4·125 + 127 + 128 + 130 + 132 = 1017` ✓
(eyes ×4 because mirrored). Crit-chance (1%/mirrored-eye-level, capped 100%) and
crit-damage (617% before the +30% from SpaceDim Controlled Entropy lv120 ⇒
647%) are likewise computed, not stored. Promoted to `SaveFile.tbs_levels`
([`TbsLevels`], with `score()`). The *mirror* flag has not been located.

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
- `X.v` (10,062), `X.T` (23 entries), `X.028` (737 ids), pet `t`/`u` —
  unidentified. (`X.z` resolved: cumulative pet stones spent.)
- Material id ↔ name: mostly solved in `crates/save-parser/src/items.rs`
  (prior-project table + export-confirmed + the 2026-06-10 full inventory
  transcription: 16/17 = Health Potion X/S, 19 = Antidote — correcting the
  prior table's "Nothing"; real Nothing is 119 — and 21 = Torch).
  Remaining: the five count-1 ids {130,160,164,167,168} ↔ {Not Nothing,
  Absolutely Nothing, Aether Ring +28, Food Journal One, Food Journal Two}
  (set known, assignment unknown). The **T4 materials are resolved** (save-edit
  probe 2026-06-16, counts 41–45 read off in-game): 131=Sun Stone, 132=Jungle
  Stone, 133=Sky Stone, 134=Mythril, 135=Ocean Stone — now in `items.rs`.
- Equipment *type* id ↔ name: solved for everything equipped, derived by
  joining Pet Stats gear strings ↔ Pet Equips instance ids ↔ the save's `R`
  instance→type map (zero vote conflicts). 2026-06-13 the user equipped five
  of the previously-ambiguous types in-game and read them off save 2's
  instance→type map: **5 = Flame Armor, 8 = Flood Armor, 22 = Water Spear,
  41 = Tree Bracelet, 44 = Storm Ring** (resolving the 44 = {Magic Hammer |
  Storm Ring} tie). Now in `items.rs::equipment_type_name`. Still ambiguous:
  {23,26,30,52,56} ↔ {Iron Pot, Flood Spear, Leeching Sword, Tree Axe,
  Hurricane Bow} (set known, assignment unknown) — equip one to resolve.
- Challenge dungeons "available" (3/10 → 2/10 after using one attempt
  2026-06-13) is **not** a stored integer — no field went 3→2. It is computed
  (regen timer + used-counter), like an energy bar. Not yet located.
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

The 2026-06-13 decodes are promoted into the typed `SaveFile`: `spacedim`
(`Vec<SpaceDimElement>`, with `spacedim_by_id` / `SpaceDimElement::name`),
`divinity_generator` (`DivinityGenerator` + `DivinityUpgrade`), `baal_power`,
`current_god_number` (+ `pbaal_defeated()`), `gp_creating_speed_pct` /
`gp_building_speed_pct`, and `gp_allocation` (`GpAllocation`). The two
rebirth saves are regression-tested in `tests/real_save.rs`.

The 2026-06-16 decodes add `tbs_levels` (`TbsLevels`, with `score()`) and
`permanent_upgrades` (`PermanentUpgrades`). The 06-16 save is in
`tests/real_save.rs` (TBS levels, the 5→6 Max Crystal move, the high-confidence
upgrades) and in the round-trip / redaction guards.

## Re-serialization and redaction

`container::encode_container` + the lossless `raw` module (`raw::Raw`) invert
the decode so a save can be re-serialized exactly (round-trip verified on
every reference save; the game accepts a re-encoded save). Note the analytic
`tree::Node` is intentionally lossy — empty fields `k:;` and bare `k;` both
become `Leaf("")`, and real saves use both — so `raw`, not `Node`, is what
round-trips.

### `save-edit` — the save editor

`crates/save-parser/src/edit.rs` + the `save-edit` bin apply scalar field
overrides to a save and re-encode it (game-loadable). It builds on `raw`'s new
`set_scalar_path(&["p","025"], "75")`, which navigates the base64-wrapped
nested structs and overwrites a single scalar, leaving every other byte intact.
A list segment is either a numeric **index** (`X.Q.17.b`) or a `field=value`
**selector** that picks the element by content (`X.b.a=Salamander.E`,
`X.Q.a=117.b`) — so pets and materials are editable by name/id without looking
up an index. (Paths are dot-delimited throughout, so a selector value can't
contain a literal `.` — fine for integer ids and element names.) `--mul`
multiplies a numeric value in place (integers stay integers, floats stay floats).

```
save-edit <in> edited_save.txt --gp 999999999 --stones 999999999  # named targets
save-edit <in> edited_save.txt --set p.025 75                     # dotted path
save-edit <in> edited_save.txt --set X.Q.a=117.b 99999           # material by id
save-edit <in> edited_save.txt --mul X.b.a=Salamander.E 10       # 10× a pet's growth
save-edit <in> edited_save.txt --material 2 400000               # add/set an X.Q stack
save-edit <in> edited_save.txt --equip 65 e 51 20 8              # add+equip Magic Stick SSS+20 (weapon)
save-edit <in> edited_save.txt --add-equip 79 20 8              # add Legendary Hammer SSS+20 to inventory (unequipped)
```

`--material <id> <count>` upserts an `X.Q` inventory stack (adds it if absent;
creates the list on a fresh account). `--equip <pet-idx> <slot e/f/g> <type>
<plus> <quality>` creates a new `X.R` equipment instance (`{a:type,b:plus,
c:quality,d/h:new id,e:20,f:0,g:0,i:0}`) and equips it in the pet's `w.<slot>`;
`--add-equip <type> <plus> <quality>` creates the instance but leaves it
unequipped in `X.R`. Both build the list from an empty field if needed
(`ensure_list`). Equip-slot note: a slot is **not** always the catalog `type` —
e.g. **Ear Muffs** (type 86, catalogued "Weapon") actually equips in the
**accessory** slot (`w.g`) in real saves, so crafters carry their hammer/pot in
`w.e` and Ear Muffs in `w.g` without conflict.

Output goes to a NEW file (never in place; the bin refuses `in == out`), is
self-verified (re-decoded and the edited paths re-read), and **must be named
`edited_*`** — that name is enforced by the bin and matches the
`**/edited_*.txt` gitignore rule (and the pre-commit hook content-checks it
too), because the output carries the save's **real, unredacted** data.
Named targets so far: `--gp` (p.j), `--stones` (X.y). **Best method for mapping
upgrades:** load the edited save into the **Kongregate web build** (InPrivate
tab) — its import leaves the Steam save untouched — and ideally diff against a
*fresh zero-purchase Kongregate save* (every field at default), so a single
in-game purchase isolates its field unambiguously.

When probing a *maxed* purchasable upgrade, set its field **down** (below the
cap) so it reappears on the purchase screen (as Camp Exp Boost did); setting it
*up* keeps it maxed and shows nothing. ChP / Overflow Points are **not yet
located**. A value-shape search came up empty — no scalar equals the ChP
total/used/left (781/650/131 in 06-16; the literal hits are unrelated ids), and
no leaf carries the 751→781 total-ChP delta from 06-09→06-16. That is
*consistent with* ChP being **derived** (total from challenge completions, used
from the stored "Chp X boost" levels, remaining = total − used — all of which
are stored), but it is not proof: the search only ruled out plain-integer
storage, and **Overflow Points** especially may well be a stored counter. The
clean way to settle both is a purpose-built before/after save — earn/spend a
known amount, then diff — which is now cheap with `save-edit` in hand.

**The committed `ManualSave_*.txt` here are REDACTED** (`save-dump --redact`):
the repo is public, so the root identity fields are replaced with placeholders
(`W`→god name, `s`→account login, `001`=Steam id, `002`/`004`=Steam
persona/display name, `003`=guest id). Structural data is byte-for-byte
unchanged, so all the cross-checks above still hold. The
`committed_saves_contain_no_identity` test guards this. For format work that
needs the original identity values, use your own local (un-redacted) save.

## Files here

- `ManualSave_2026-06-09.txt` — first raw save (input).
- `* Export.txt` — same-session in-game exports (ground truth).
- `second_save/` — 2026-06-10 save + re-exports + **full manual inventory
  transcription** (materials, gems, equipment counts) — the richest ground
  truth so far.
- `save_pet_stone_tbs/` — 2026-06-16 save + `notes.txt`: each Baal-Slayer
  component set to a distinct level (resolving `root.S`) and the final Max
  Crystal bought (the `p.001` 5→6 that validated the `root.p` upgrade block).
- `normal_stats_investigation.md` — the normal-stats formula work.
- `expand_save.ps1` — decoder/expander script (superseded by
  `save-dump --tree` but kept for history).
- `save_layer1.txt`, `decompressed.bin`, `save_expanded.txt` — derived
  artifacts (regenerable; the expanded tree is the one to read).
