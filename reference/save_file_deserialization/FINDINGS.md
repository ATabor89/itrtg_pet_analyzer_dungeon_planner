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
   `V2`). Strip the first 2 characters before decoding. **This is a constant
   literal version tag `"V2"`, not salt** — verified against `Assembly-CSharp`
   serialization logic: the container writer (`HGAHFCFJLDN.CAPJIECENDO`) returns
   `"V2" + base64(gzip(...))` unconditionally.
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
| `h` | **Physical** conditioning (28 — `items::physical_training_name`, in-game Physical screen order: Running … Train on Dimension X; these raise the Physical stat, *not* "skills") | `a` = id/list position, `b` = **level**, `c` = **clones allocated**, `d` = 0 (unidentified). Player-confirmed 2026-06-18 by removing clones from some Physicals (leaving Skills synced) and watching `b`/`c` diverge as expected. Has **no** `e` sub-struct — its training **cap** is derived from the index-matched Skill's usage count (see `j`) |
| `i` | **creations** (29, Shadow Clone … Universe — `items::creation_name`) | `d` = current amount (Shadow Clone entry = the clone count ✓), `e` = clone cost, `i` = next-at ✓, `g` = **total created** — matches the in-game mouseovers (Earthlike Planet exactly 3,893; Shadow Clones 38.6M; Galaxy 980k vs ~1.1M read later). Divinity-bought copies do *not* increment `g` (hence the frozen Earthlike Planet count, and why Earth Eater's 7.35M eaten ≫ created). `f/h/j` = flags (auto-create settings?) |
| `j` | **Skills** (28 — `items::skill_name`: Double Punch … Time Manipulation; the actual Skills, raising the Mystic stat) | same outer shape as `h` (`a`/`b`/`c`/`d`), plus a Skills-only `e` sub-struct: `e.a` = skill id again, **`e.b` = "Special"-menu usage count** (the in-game "Usage Count" tooltip; auto-trains ~1/min, manual fights add more), `e.c` = small stable int (unidentified). The game derives the training **cap** from `e.b` and applies it to *both* this Skill and the index-matched Physical — confirmed 2026-06-18 by copying one save's `e.b` onto a fresh save and watching both caps drop to 1 clone. NB the byte-identical `b` between Physical[i] and Skill[i] in a fully-reduced Steam save is the in-game **"Sync"** toggle (keeps clone counts/levels equal), not a shared value |
| `k` | **Monsters** (34 — `items::monster_name`: Slimy … Monster Queen) fought to generate Battle/Divinity | `a` = id, `b` = **number defeated**, `c` = **clones allocated**, `d` = 0 (unidentified) |
| `D` | **monuments** (9, Mighty Statue … White Hole — `items::monument_name`) | `b` = **current level** (equals next-at once reached — clones spill onward; Black Hole mid-build: level 110 vs next-at 140 ✓ user expectation), `g` = next-at, `h` = spread ratio; `f`=building, `c`=clones allocated, `d`=progress. Each monument's **upgrade** lives in its own `e` sub-struct (7 fields): `e.b` = upgrade level, `e.f` = upgrade next-at, `e.g` = upgrade spread (player-tested 2026-06-17; `e.a/c/d/e` still unidentified) |
| `V` | **mights** (14 — `items::might_name`) | `b` = **current level** (specials all 64 = the user's "at level 64" ✓; Σ levels = 3,200 = the in-game White Tiger unlock progress ✓ — 25,000 needed, resets per rebirth), `m` = next-at, `n` = spread ratio. Specials 8–13: `e`=True, `g` = base unleash duration (+1 s/level: 30+64 = 94 s ✓), `i`/`j`/`k` = HP-recovery/Attack/Mystic % (Transformation Aura 200/200/200 ✓). "Total Might" (100,983) is the all-rebirths global at `x.129`/`p.F` |
| `O` | statistics mirror | `O.030` = light clones ✓ |
| `009` | **SpaceDim** (Light Dimension) | `009.a` = light clones (= the count allocated to the one active element); `009.b` = the 20-element list (see its section); `009.d` = **Light Clone Resets available** (player-tested 2026-06-17 — a reset rolls the escalating Light-Clone Baal-Power cost back to 1/clone: first 500 cost 1 BP each, next 500 cost 2 each, …); `009.e` = spacedim total. Per-element levels confirmed against the 2026-06-13 notes (Fusion Torch 18→70, Dyson 22→23, Quantum Genesis 2→6) |
| `K` | **Divinity Generator** | `K.g` = **capacity currently in use** (huge double — how much divinity is held *now*, NOT the total; the total is root `a`. The total *capacity* cap isn't stored nearby — likely computed). `K.c` = **Worker Clones** allocated to the generator (1,208,848 ✓). `K.n` = **Stone Storage** amount (huge double; its cap also not stored nearby). `K.l` = list of 3 upgrades — **0 = Capacity, 1 = Divinity Gain, 2 = Converting Speed** — `b` = level (all three 81→188 between the 2026-06-13 saves ✓), **`f` = next-at**, **`g` = spread (1/2/2)** (was mislabeled "multiplier"); `c`/`d`/`e`/`h` still unidentified. All the 06-18 reads player-confirmed |
| `P` | **current god fight** (≠ `X.P` active dungeon runs) | `P.c` = current god number = highest **P. Baal defeated + 1** (44→49 as the user went from "defeated 43" to "defeated 48" ✓); `P.b` = that god's HP-scale double (grows ~10× per few gods), `P.d` = `P.b`/10 |
| `T` | **Planet system** (`AIDFNOPNJGK`, marker **"Planet"**) — **corrected** (was mislabeled "Baal Slayer block"; marker + contents are definitively the Planet / Ultimate-Beings system; the 4 in-game planet screens likely map to its sub-blocks) | **`f` = the 5 Ultimate Beings** (`CEFAAPALBMD`, screen-1 "UBs"): each `c` = UB id (1 Planet Eater / 2 Godly Tribunal / 3 Living Sun / 4 God Above All / 5 ITRTG; spawn intervals 1–5 h), `b` = kill count, **`d` = spawn countdown ms** (counts DOWN; spawns at ≤0 — set 0 to force a spawn), `e` = a bool (`AHAEMEAEBPF`; reads True for all UBs on captured saves — earlier guessed "alive" but its in-class usages don't confirm that, so labeled neutrally), `f` = GP gained. Wired into the editor (titled by UB). **`h` = unspent Baal Power** (`KFLLILDEMGC`) — **player-confirmed** ("You have X Baal Power"; spent on Light Clones, which fight the UBs on your planet — hence the grouping; wired/editable). **`k` = the Ultimate Being V2 list** (`FPBMNCNKPHN`, one per UB — **corrected 2026-06-22**: player-reported, C#-confirmed it is **UBv2**, not a "per-UB multiplier" / "Powersurge" / "TBS track"). The C# gates this list on the `UBV2C` challenge (`AIDFNOPNJGK:560`), so these are the **Ultimate Being V2** bosses (same names + " V2"), each defeated **once per rebirth**: `c`=UB id, `b`=**cumulative defeats across rebirths** (incremented per defeat, `AIDFNOPNJGK:256-257`) — this drives the tooltip's **"Multi from Ultimate Beings"** (each UB adds a fixed % per defeat: Planet Eater 1% / Godly Tribunal 12% / Living Sun 21% / God Above All 32% / ITRTG 45%); `a`=per-UB state value reset to 100.0/0.0 in OfflineCalc (~100=full, exact role unconfirmed). The editor renders T.f and T.k as **two separate tables** ("Ultimate Beings" / "Ultimate Beings V2"); the V2 names append " V2". **Planet Multiplier is COMPUTED, not stored** — tooltip: `base 100% + Multi from Powersurge + Multi from Ultimate Beings (from T.k)` — directly multiplies Physical/Mystic/Battle/Creating. There is a **single Powersurge** (clone-allocation power generator) stored as a `T` scalar — TBD, needs in-game anchor (it is *not* `T.k`). **`d` = planet level** (`NJBCIIEMLFI`; player-confirmed 7; drives the planet-name tiers — `>=5` = "Almighty Planet in the Universe"; levels 1-5 from feeding planet/earthlike/sun/solar-system/universe, then +1 per **Ultimate Universe Challenge** [`OIDDHCOBPLG.UUC`]; the power-effective level adds the UUC count on top; wired/editable). Other scalars (first-pass, need anchors): `a` bool, `b`/`c`/`e`/`g`/`j`/`l` BigDouble (planet/clone power — `b` used as hours via `/3600000`), `i` int (=100). **Self-correct² (2026-06-20):** `T.h` = Baal Power was CORRECT all along (player-confirmed); my earlier "overturn" was wrong. `T.k` = the **Ultimate Being V2** list (earlier guesses "TBS tracks", "Powersurges", and "per-UB multiplier state" were all wrong — see the 2026-06-22 UBv2 correction above). The actual **Baal Slayer (TBS) is `root.S`** ("Baal Slayer Parts"), separate from this Planet block; Baal Power lives at `T.h` only because it buys the planet's Light Clones. The **single Powersurge** field is still TBD. |
| `024` | **Pet Village** (`PPKCEMJNHPA`) — **corrected** (was mislabeled "quests block"; the old "024.d = Quest Points 4553" was wrong — `024.d` is the Dojo struct, not a scalar) | 7 building sub-structs: **`b` = Tavern** (`IOBPPFGEBCD`; runs pet quests) — **field map corrected 2026-06-23** (decompile + player): **`v` = Tavern level** (upgraded/max level — NOT `b`), `m` = active level (selected quest difficulty, 0..v), **`e` = quest rank** (enum `GBFGHANMFII`: 0 F … 9 Ult; STORED, recomputed from quest points on quest/load events — load-time bumps to 4 at d≥8100, 5 at d≥24300; per-rank threshold ≈ `clamp(3^rank·100, 100, 450000)`), **`d` = Quest Points** [4453 ref], `i`=quests/day, `j`=max concurrent, **`n` = upgrade-elapsed timer ms** (the Tavern's upgrade timer is `n`, not `c`; set ≥ target to finish), `o`=upgrading flag, `u`=Tavern Keeper slot [999=empty], `x`=favorite quests `&`-list, `a`/`t`=active/pool quest lists. **`b` is a BeanStalk-quest counter (0–9 → WonderAxe), NOT the level.** `c`=usually-empty quest-id list; `g`/`l`/`p`(RNG seed)/`q`/`r`/`s`/`w` unconfirmed. Level/active-level/rank/points/quests-per-day/max-concurrent wired into the editor. **Active quests `024.b.a`** (`EBBFLHGBFCN` "Quest", a list that collapses to a lone struct for a single quest; player-confirmed + tested 2026-06-22): `a` = quest id (enum **`HGAJAIFLFFL`**, 0..=80, e.g. 10 AntQueen / 11 MagicTalk), `b` = **elapsed timer ms** (counts up to a per-quest-type target ≈12h/43,200,000, runtime-only `HPLOENDHFGE` — set `b` ≥ it to complete; overshoot tolerated), `c` = assigned pet type id (a `&`-list, scalar for the usual single pet), `d` = random reward roll 0–99 (consumed `d/20` for reward quality — NOT a quest id; an early "d=quest id" guess was a coincidence). Wired into the editor (Active Quests table: resolved pet+quest, editable elapsed timer + Force-complete, read-only roll)), **`d` = Dojo** (`JKDCFKCLCKH`; `b`=level — player-confirmed 8 in ref save, `c`=upgrade progress, the four `999` = 4 pet slots [2 Dojo Master + 2 pupil]), **`e` = Strategy Room** (`CJACGIIPNIG`; `b`=level, `c`=upgrade progress, **`e`/`f`/`g` = Physical/Mystic/Battle Multi%** [player-confirmed by tweaking], `h`=8 pet slots `&`-joined), **`f` = Museum** (`IOJHGILEEKA`; gives a crafting-speed "(museum)" mult; **statue list at `024.f.a`** — each `MCEIHMMCDNH` = `a`:level (20 maxed), `b`:statue id [enum `JBGNCMHGOFI` = event commemoratives Easter/Summer/Anni/Halloween/Valentine/Christmas by year, two ownable each]; wired with statue-name resolution. An *empty* museum serializes as `YTs=` = base64(`a;`), now decoded by the parser fix), **`g` = Material Factory** (`CHDGDEINMHO`) / **`h` = Alchemy Hut** (`GABIFCBBMPH`) — both extend the worker-building base `ANECMNGBLNI`: `a`=level (worker buildings + Divine Hut use `a` for level, unlike Dojo/Strategy Room's `b`), `e`=manager slot (pet type id, 999=empty), `d`=worker pet-slot list (each `FGKIILDKMEA`: `a`=pet type id [Lamb 138 confirmed in the Alchemy Hut fixture; 999=empty], `d`=work progress, `b`/`c`=in-progress-craft sub-structs); levels/managers/worker slots wired into the editor. **`i` = Divine Hut** (`HPACHDJMLLP`; `a`=level, `c`=progress, `e`=slotted pets — absent in ref save when unbuilt). **`a` = village building-state list** (9 × `AFELNLGMCAB`, marker "VillageBuilding"; `g` = building type via enum **`IMBOLMEHKCG`**: 1 Fishing, 2 Tavern, 3 VillageCenter, 4 Dojo, 5 MaterialFactory, 6 SnackBar, 7 Forge, 8 AlchemyHut, 9 DivineHut, 10 HuntersGuild, 11 CrystalTower, 12 StrategyRoom, 13 BattleTent, 14 Museum, 100+ = placeable cosmetic decorations [tiles/fences/walls/towers/pillars] — the village screen is a placement grid where you drop buildings, decorations, and pets. **Player-confirmed (2026-06-20): VillageCenter/SnackBar/Forge/HuntersGuild/CrystalTower/BattleTent are UNIMPLEMENTED** — placeable but their in-game mouse-over says "not implemented", so they have NO dedicated struct and nothing further to find (only their `IMBOLMEHKCG` enum entry). `c`=level, `f`=assigned pet, other fields unlock/flag state [mostly default in ref]; titled by building type in the editor). `c` = scalar. **Upgrade timer:** each building's `c` = `LDMJEPGEOME` = **elapsed** upgrade ms (the universal elapsed-timer field, same as a dungeon run's `b`); it accumulates until `c >= target` (`HPLOENDHFGE`), then the upgrade completes and `c` resets to 0 — so **set `c` large to force-complete** an in-progress upgrade (`JKDCFKCLCKH` lines 139-143). (**Exception: the Tavern's upgrade timer is `n`, not `c`** — its `c` is a quest-id list.) Containers + Tavern/Dojo/Strategy Room fields wired into the editor. (Quest Points resolved → Tavern `024.b.d`.) |
| `025` | **fishing block** (`KACINBICCNH`) | `a` = Fish Power (1,270,255 ≈ displayed 1.270e6 ✓); `b` = **current fishing exp** (resets to 0 on level-up — player-confirmed), `c` = **fishing level** (player-confirmed); `d`/`e` = selected **bait**/**rod** (material ids `NCPJFPLCPPK`), `f` = current **pond** (enum `BAMKFONNEMP`: 0 New … 4 Sad … 9 Final). Lists: `g` = **rods** (`a`=id 500-504 → Stick/Wooden/Bamboo/Voodoo/Titanium, `b`=owned), `h` = **bait** (`a`=id 520-524 → Feather Ball/Simple Worm/Big Worm/Caterpillar/Super Worm, `b`=count), `i` = **fish caught** (`a`=fish id 525-567, `c`=lifetime count). Rods/bait/fish are all `NCPJFPLCPPK` ids (rod/bait names were placeholders in `material_name`, now fixed). All wired into the editor. |
| `p` | god-power block, see its own section below | `j`=available GP, `v`=GP spent, `F`=total might, `002`=crystal power |
| `s` | **linked account login name** (Steam/Kongregate) — NOT the god name. C#: `DJJMJOHIHPO`, uploaded as `UserName`/`KongName`. | player-confirmed; redacted in committed saves |
| `r` | **linked Kongregate user id** (numeric `KongUserId`) — C#: `OBBCNEEELEN`, uploaded as `KongUserId` in the stats sync; persists in a Steam save. **PII.** | redacted in committed saves (`save-dump --redact` scrubs it → `0`); was missed by the original redaction set, fixed |
| `y`,`z` | achievements (168 each: flag + id) | |
| `W` | **in-game god (deity) name** | player-confirmed; redacted in committed saves |
| `X` | **the whole pet system** | see below |
| `Z` (bool), `001`=Steam id64, `002`=Steam persona name, `003`=account/guest id (`a_…`), `004`=Steam display name, `005`=save ms-timestamp, `006`=init log | identity in `001..004` redacted in committed saves; `005`/`006` kept | player-confirmed |

Root `a` = **Total Divinity** (the running divinity balance; player-confirmed
2026-06-18 by editing E+19 → E+29 and watching the in-game total change).
(Other unlabeled root keys not yet investigated: `b`,`d` big doubles, `A`,`B`,
`O`,`Q` blocks, etc.)

## `X` — pet system

| key | meaning | evidence |
|-----|---------|----------|
| `b` | list of 158 pets | count matches Pet Stats export exactly, same order |
| `c` | Puny Food count | 123,548 = inventory transcription exact (2nd save) |
| `d` | Strong Food count | 16,276 ✓ exact |
| `e` | Mighty Food count | 7,239 ✓ exact |
| `v` | Chocolate count | 9,989 ✓ exact (was the "10062 unknown" in save 1) |
| `002` | **gem inventory**: list of {`a`=element id, `b`=gem level, `c`=count} | all 7 stacks match the transcription exactly. Element ids are the `EMGELCMNFOL` enum (0=N,1=F,2=W,3=E,4=Wi, **plus 5=Dark, 6=Light, 50=Elemental, 99=All** — gems can be these even though pets/dungeons only use 0–4); `items::gem_element_name` / `GemStack::element_name()` name the full set |
| `x` | list of 8 **campaign slots** (`FMOLELEHAFD`) | `a` = **campaign type** (enum `AGGDKICFOAI`: Growth/Divinity/Food/Item/Level/Multiplier/GodPower/All/Event — *not* a slot index, **corrected**), `c` = **elapsed ms** (counts up to `e`; same `LDMJEPGEOME`/`HPLOENDHFGE` elapsed/target fields as a dungeon run — *not* a timestamp, **corrected**), `e` = **target duration** (43,200,000 ms = 12 h; set `c`=`e` to complete), `d` = `&`-joined pet ids, `f` = total bonus, `i` = RNG seed. Wired into the editor (slot titled by campaign type). |
| `y` | pet stones | 267,028 ✓ Main Stats |
| `z` | **cumulative pet stones spent** | fresh-save diff: `y` −750,000 / `z` +750,000 when buying 2 Dungeon Loot + 1 Dungeon Exp (= 2·275k + 200k) ✓ |
| `Y` (capital) | **free experience** | **Confirmed** 2026-06-16: editing `X.Y` to 1e9 showed the matching free-exp value in-game (30,855 advanced / 26 fresh). `SaveFile.free_experience`. (`X.Y` ≠ `X.y` = pet stones.) |
| `P` | list of 3 **active dungeon runs** (`MKDNAHGDLPI`) | `a` = **dungeon id** (enum `GFEKIABOPIH`; 2/3/5), `b` = **elapsed ms** (counts up toward `c`; player-confirmed via fresh runs at ~1.2M climbing), `c` = **target duration ms** (43,200,000 = 12 h), `d` = depth, `f` = **team index** (ties the run to its `X.S` team — player-confirmed), `e`/`j` = RNG seeds. **To force near-completion, set `b` just under `c`.** Wired into the editor (`X.P` = "Active Dungeon Runs", dungeon-id resolved). |
| `Q` | list of 69 — **material inventory** (`a`=item id, `b`=count) | export-confirmed: 117=Ant 192,164 ✓, 159=Strategy Book 2,840 ✓, 166=Honey 787 ✓, 174=Acorn 24,727 ✓. Full id→name table (incl. the prior project's identifications: Herb/Iron Ore/…/Soul of Sylph) lives in `crates/save-parser/src/items.rs` |
| `R` | list of 209 — **owned pet equipment** | see equipment struct |
| `S` | list of 3 — **dungeon teams** (`PCDCANGLENI`, static settings) | `a` = 6 pet ids (`&`-joined, matched via pet `k`), `b` = **dungeon id** (enum `GFEKIABOPIH`), `c` = **pending loot / team inventory** (`GCJMGGFGKBN`: `a`=item id → `material_name`, `b`=count; loot isn't rolled until the run completes), `d` = depth, **`e`/`f`/`g` = D1/D2/D3 difficulty** (player-confirmed in-game; `h` is a list not a difficulty int, so D4 isn't cleanly placed; `k` is a 2nd dungeon-enum, `l`/`m` ints — unconfirmed), `i` = dungeon name string. Wired into the editor (dungeon-id resolved, loot entries name-resolved). |
| `T` | list of 23 | ? (per-entry: id-ish `a`, float `c`, nested gear-like `g`) |
| `t` | **Afky God** (single struct; the AFK god that fires at clones for idle exp) | Player-confirmed + tested (2026-06-22): `a.a` = god power, `b.a` = firing speed, `c.a` = clone HP, `d.a` = clone count, `g` = experience, `h` = clones killed, `i` = exp multiplier. (`a/b/c/d` are base64-wrapped sub-structs each with a secondary `b` — unconfirmed; `e`/`f` scalars unconfirmed.) Labeled in the raw tree as "Afky God"; values are BigDoubles (text) except the two counts (uint). |
| `w` | **Crystal Factory** (single struct, `GKJLJMJLMIB`, marker "CrystalFactory") | `a` = Crystal Power (the factory's leveling resource; save 400000), `b` = the 6 per-grade **modules** (`IPNJFMOMAMB`: `a` = grade 0 Physical/1 Mystic/2 Battle/3 Creation/4 Ultimate/5 God [`KEMALIHPLCG` enum], `b` = level [save: grades 2/4/5 at 12], `e` = clones on the module, `c`/`d`/`f` BigDouble cost/timer/value — roles unconfirmed), `e` = factory-level **clone allocations** (`CNJMKHCJPGM`: `a` = grade, `b` = clone count, `c` value), `c`/`d` = BigDouble production counters (accumulated / progress — unconfirmed). Confirmed against the fixture (`X.w` = `{a,b=6 modules,c,d,e=3 clones}`). Wired into the editor (Crystal Factory section: Crystal Power + module levels editable; ambiguous BigDoubles read-only). |
| `Z` | **the Challenge team** (single `PCDCANGLENI`, same class as a dungeon team; C# `NMGIGAGPLCL`) | `a` = `&`-joined member pet ids, `c` = its own inventory, `b`/`d` = dungeon-id/depth (0/unused for challenges — no difficulty/depth/timer). Confirmed by C# **and** the member ids matching the player's challenge team (Egg/Squirrel/Armadillo/MistSphere/Sylph/Rabbit). Wired into the editor as "Challenge Team". |
| `001` | **challenge tries used** (`FEELNJKJJKP`) | available challenge dungeons = `10 − X.001` (clamped to the max). Lower it (0 or negative) to refill. Committed save: 10 (→ 0 available). C#: `CIJAEKANAPL` "Challenges (avail/max)" display. |
| `017` | **challenge max-tries upgrade** (`LHFIMMKICKK`, ChP upgrade, clamped 0–20) | max challenge tries = `10 + X.017` (so up to 30; at 20 the daily draw gives 4/day, at 30 → 5/day). Committed save: 0 (→ max 10). |
| `028` | **achievements / milestones catalog** — a single `AMEJKMHGAPD` object holding 737 entries (class `OCNIKNHPHHA`), each `{a = milestone id (the `FGGLKEBGPHJ` enum: PetOwner, Hyperion, MightyStatue10k, Clones1M, TyrantOverlordBaal, …), + 2 flags}`, partitioned into per-element sub-lists. The "catalog unlock flags" guess was right. | verified against `Assembly-CSharp` (`X.028`→`DIHGCIJMOAI`, enum `FGGLKEBGPHJ`) |

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
| `l` | unlocked flag (`JCJHMEDDJGJ`) | locked pets `False` ✓ export Unlocked column. **This is the *only* unlock gate** — there is no separate "secret"/"hidden" flag. **Secret pets** (Serow = type id 803, Owl/Book/Carno/Leviathan/…) are ordinary locked rows already present in `X.b`; the game checks each pet's unlock condition only at *unlock-attempt time* (C# `EKGGGJJPAIB`) and then sets `l=true` permanently — it never re-locks based on the condition. So flipping `l` to `True` unlocks any secret pet outright (the editor's existing per-pet/bulk Unlocked toggle already does this). Exceptions: **Fawn (900) / Herakles (901)** are event-spawned, *not* in the roster, so they can't be unlocked by toggling `l` (would need appending a full pet struct — out of scope). |
| `m` | timer ms: 86,400,000 (locked) / 34,976,500 (all unlocked pets) | shared countdown — next growth tick? |
| `v` | **dungeon team slot** (1–6, 0 = none) | matches all 18 team members ✓ |
| `w` | dungeon sub-struct, see below | |
| `E` | **growth** (float; exports round it). Stored *without* the Magic Egg ×1.3 — true base growth. | 66841.359… → "66,841" ✓ for every pet; Pandora's box `E` = 44334.321043064, the exact base the Magic Egg validation derived (export shows 57,635 = ×1.3 rounded) |
| `F` | **partner pet type id** (999 = none) | 158/158 match export Partner column; pairs are mutual (Cat↔Dog, Vampire↔Succubus, Gnome↔Dwarf) |
| `G` | partner-related counter (bond level?) | only nonzero when F≠999 |
| `H` | ? (only Cat: 10,920) | |
| `y` | **elemental-pet form counter** — the evolved-form/upgrade level you advance via the pet's "quest". `0` for every non-elemental pet; **+1 per form** for elemental pets (player-decoded 2026-06-18 by upgrading Gnome/Salamander/Sylph one form each and diffing — `y` ticked +1 and base growth `E` jumped). Offset per pet, so *not* directly the displayed "V" number: Gnome `y−10`, Salamander `y−15`, Sylph `y−20` (06-09 fixture Gnome 14 / Salamander 19 / Sylph 24 are all form **V4**). The export "Other" column carries the human label (`GnomeV2`). `SavePet.elemental_form_id`. | Gnome=14, Salamander=19, Sylph=24, all non-elemental=0 |
| `B` | **token-improved flag** (0/1) — the Pet-Token "Improvement" applied (export "Improvement" = Yes). Player-decoded 2026-06-19 by improving one pet (Aurelius) and diffing — only `B` flipped 0→1 (plus its recomputed Health). `SavePet.token_improved`. | 06-09 fixture: exactly the 20 export-improved pets have `B=1` ✓ (count match + Hedgehog/Sphinx=1, Mouse/Dog=0) |
| `t`,`u` | **`t` = Favorite Camp, `u` = Hate Camp** — the per-pet "Fav Camp"/"Hate Camp" settings that bias how pets are auto-assigned to campaigns (player-confirmed 2026-06-19). `0` = unset, else `items::campaign_type_name(value−1)` (the `AGGDKICFOAI` enum: Growth/Divinity/Food/Item/Level/Multiplier/GodPower/All/Event). `SavePet.favorite_campaign`/`hated_campaign`. | decoded from `DFLAKHONNPC.AIAOBIPOBFB`/`HDFIIPCPJCP`; Vampire t=1→Growth, Dog t=4→Item, Penguin t=7→GodPower ✓ |
| `d`,`e`,`f` | **additive stored growth components** — total growth = `E + d + e + f` (the game's `MILFAIOPDAF()`); `d`,`e` reset together (rebirth?), `f` persists. **NOT the Magic Egg bonus** — disproven 2026-06-19 by the Pandora probe: Pandora has the Magic Egg equipped yet `d=e=f=0`, and the export's ×1.3 (57,635 = 1.3·44,334) is a **runtime multiplier from the equipped item**, applied on top of `MILFAIOPDAF()`. `d/e/f` are 0 for ~every pet in the reference save; their actual source (event/item growth grants) is still TBD. | Pandora `d=e=f=0` with Magic Egg ⇒ Magic Egg is a display-time multiplier, not stored here |
| `n` | **growth pool/reserve** (AGJPDMBDHHG): the add-growth method deducts spent growth from `n` and caps a spend to it; the training tick accumulates it. Precise name TBD. | |
| `x` | **feeding setting** — per-pet auto-feed mode: 0 None, 1 Puny, 2 Strong, 3 Mighty, 4 Chocolate, 5 Free, 6 Starve. `SavePet.feeding_setting`. | decoded from `DFLAKHONNPC.CJMBBFKNFNF()` |
| `A` | **vaccinated flag** (bool) — set once the pet consumes a Vaccine item (Corona/Vaccina event). `SavePet.vaccinated`. | decoded from `CBNILFAJMAE()` |
| `s` | **recovery-cooldown timer** (long, ms) — while >0 the pet skips its update tick; its Health resets when it expires (a dungeon death/recovery cooldown). | decoded from the per-tick update method |
| `C` | **cosmetic skin/texture index** (int) — selects an alternate pet sprite (`0` = default). | property `PPEIMBAMGMJ` (Texture2D) |
| `z`,`D` | **vestigial flags** (bool) — serialized/deserialized but never read in the pet class (legacy/compat). | |

**Pet struct FULLY decoded against `Assembly-CSharp`** (class `DFLAKHONNPC`,
method `EBOFJJHOOLP`): every field is now identified — the field set and per-key
types are exactly the game's. The pet has **no** keys `b`, `c`, or `i`. Type ids
(`k`, `F`) are the `HFNFDKEMAIK` enum and the form (`y`) is the `ANHOKMNPAKI`
enum — both transcribed into `crates/save-parser/src/items.rs` (`pet_type_name` /
`elemental_form_name`); see the "Pet type & form enums" section below.

For the normal-stats formula work (display-side model, the Anni Cake
multiplier, open staircase questions), see `normal_stats_investigation.md`.

### Pet dungeon sub-struct (`w`)

| key | meaning | evidence |
|-----|---------|----------|
| `a` | element: 0=Neutral, 1=Fire, 2=Water, 3=Earth, 4=Wind | matches export Element for all checked |
| `b` | dungeon level | Salamander 101 ✓, Sylph 117 ✓ |
| `c` | dungeon exp — **current toward the next DL**, resets on level-up | matches the in-game "current / needed" display exactly (Salamander 147,749, Succubus 130,099); requirement = `(DL<10 ? DL² : DL^2.25)·10`, the exponent keyed on the **current DL** — verified against the game's `CIEAPBPBCLL.FGPGHDEAMLG`. (2026-06-19 this corrected a 1-step off-by-one in `formulas::dungeon_exp_to_next`, which had switched on `DL+1`, over-charging the single 9→10 step.) |
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
| `L` / `013` / `S` | **Daily-screen timers** (class `DFGCALKGABP`): `L` = free-draw (daily lucky-draw) countdown, `013` = bonus-pack countdown, `S` = daily-pack countdown. Stored as **remaining ms** (NOT wall-clock timestamps — they decrement each tick and reset to +24h on claim), so **set 0 = ready now** (`S` is a signed long, so `<0` also = ready; `L`/`013` clamp at 0). To shift *what time of day* it ticks over, write `desired_next_tick − now` ms. There is no separate stored daily anchor. | fixture ≈4.34e7 ms (~12h) each; reset adds 86,400,000 ms (`DFGCALKGABP` claim) — wired into Resources |
| `012` / `T` | **Bonus packs left** (`012`) / **Daily packs left** (`T`) — plain counts | fixture both 44; wired into Resources |
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
| `w` | TBS level-loss-on-miss / "Chance to lose progress" | PERM 20; editing `p.w`=17 showed "Chance to lose progress: 39%" (display looks derived, ≈ base − `w`). `p.e` is **not** its twin — it's **Ultimate Shadow Summon** (consumables diff). |
| `b`,`d`,`e`,`m`,`n` | **boost consumables** (`root.p`) | confirmed 2026-06-16 by a Steam save diff (`Steam/Consumables/`): `b`=**Godly Liquid** (151→141), `m`=**Godly Liquid V2** (50→51), `d`=**Chakra Pill** (156→146), `n`=**Chakra Pill V2** (47→48), `e`=**Ultimate Shadow Summon** (19→18). (Corrects earlier guesses: `e`≠TBS level-loss, `d`≠creation-count — both matched by value coincidence.) Promoted to `SaveFile`. |
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
| `p.V` | bool | **Refrigerator** (keep food on rebirth) | **Confirmed** (F→T diff) |
| `p.Y` | bool | **Pet Half Stats** (GP upgrade; prereq for Config Half Stats) | **Confirmed** (F→T) |
| `p.B`, `p.U` | bool | **Improved Next At** (base, GP) + **Improved Next At For Challenges** — buying the for-challenges upgrade also grants the base one (player confirmed base reads as already-purchased on the GP page), so both flip together; which key is base vs for-challenges is TBD | **Confirmed** (F→T) |
| `p.I` | currency | **Pet Tokens** (5→6 diff) | **Confirmed** |
| `p.023` | currency | **Class Change Tokens** (8→10 diff) | **Confirmed** |
| `p.K` | currency | **Lucky Draws** (unopened; 6→2→0 diff). *Opened* lifetime = `x.071`. | **Confirmed** |
| `p.b` | consumable | **Godly Liquid** (regular; 0→1 when a draw gave one) | **Confirmed** |
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
Aether boss kills (the Aether Ring "+28"; **C#-confirmed** — the pet evo gate
`DFLAKHONNPC` checks `root.x.186 >= 25` for Aether, and `NAKBHGFCICI` renders as
"Aether Ring +N"; also a factor in Aether's growth-campaign bonus. Labeled in the
save editor as Statistics → "Aether Ring lvl / Delirious Essence wins"), 216
Pignata bashes, 218 God Power
campaign hours, 234 Meteor campaign hours, 259 Caterpillar materials, 310
Mule quests, 311 Gold Dragon bonus growth, 324 Serow items saved, 336 Bag
bonus growth. Globals: 013 AFK clones killed, 049 day-pet-challenge multi,
071 lucky draws opened, 074 crystal power, 078/079/080 dungeon
bosses/enemies/rooms, 129 total might.

**Challenge completions — SOLVED + VALIDATED + WIRED (2026-06-20).** They are a
**single list at `root.x.242`** (`LLMCMCKAABP` reads `n242` → list of
`KPLPGPEOFNB`, field `AOGFMPHMNLI`), **not** one scalar field per challenge.
Each element is a `KPLPGPEOFNB` completion record:
`a` = challenge id (`OIDDHCOBPLG`, deser. line `KPLPGPEOFNB.cs:3684`),
`b` = lifetime completion count (the number the Challenges menu shows),
`c` = `HOLHIHDKBKA` difficulty (`0` Normal/None, `1` Hard, `2` Root, `3` Mixed→Hard on load),
`d` = ms epoch (last-completion time — inferred: per-challenge recency tracks the count),
`e` = a bool UI sort flag.
The enum `OIDDHCOBPLG` has 77 declared values (`None`=0 … `BCC`=76); full
id→name table (from the in-game name strings in `KPLPGPEOFNB`, matched by
initialism) is in `items::challenge_name`. Only the challenges the player has
completed appear in the list (12 entries in the captured save). **Validated** against an in-game
capture (Steam, `Steam/Challenges/`, 2026-06-20): `a`/`b` =
UUC(1)/2, BHC(2)/1, DRC(3)/8, UPC(4)/8, PMC(9)/1, AAC(10)/10, UBC(11)/1,
CPC(17)/2, MMC(25)/13, GPC(32)/2, PLC(48)/12 — all eleven match exactly. Wired
into the editor as the **Challenge** block (`x.242`, titled by challenge name).

**Self-correct (2026-06-20):** the earlier "one scalar field per challenge in
`root.x`, read by `HAKKBJKMHFN`, candidate map UUC=`x.H`/AAC=`x.002`/…" was
**WRONG** — those keys are all `0` in the captured save despite real completions.
`HAKKBJKMHFN` is not the completion-count store; the counts live in the
`x.242` list above. The old candidate map is retracted.

**Distinct** from the challenge *tries* (root **`X`** capital, pet system:
`X.001` used / `X.017` max-upgrade) — different block, do not confuse `x`/`X`.

**Challenge Points (ChP) + Overflow Points — mechanism SOLVED 2026-06-20.**
The ChP debug tooltip (`LLMCMCKAABP.cs:4063`) is the Rosetta stone for this
whole system. Key findings:
- **ChP total is DERIVED, not stored** (`LLMCMCKAABP.NFHDEHOCFMP`, lines
  4897–4902: reset to 0, summed over the challenge dict as `Σ item.Value.OIHGOPGKAJO`,
  then floored). So total ChP is a deterministic function of the
  `root.x.242` completion list — *computable*, no stored integer. This
  **confirms** (and explains) the earlier "no stored ChP integer" observation;
  it was right that nothing is stored, but the cause is that ChP is a sum.
  "Hard mode Challenge points" is a separate static (`MECCLHFLOIK.MOJDJOCCILM`).
  - **Per-challenge ChP values now tabulated** in `items::challenge_chp(id)`
    (transcribed from each challenge's wiki page, 2026-06-21): normal challenges
    pay a flat amount per completion (e.g. AAC 30, UBHC 30, UBC 25, UPC 8, MMC 4,
    PMC 10, PLC 1, GPC 1, BHC 2, NRC 40, NRCPC 60, LCNRC/NMNRC 50, …), so
    `total ≈ Σ(completions × challenge_chp)` over non-Day challenges. **Validated
    exactly** against an in-game capture (20 AAC + 4 UBHC + 4 UBC + 12 UPC +
    14 MMC + 4 PMC + 12 PLC + 8 GPC + 4 BHC = 1040 ChP). The **Day** challenges
    (`OIHGOPGKAJO` score formulas) contribute score-based ChP instead — not in
    the per-completion table (return `None`).
  - **Day-challenge score stats** (each Day challenge's ChP scales with a
    high-score statistic in `root.x`, per the `OIHGOPGKAJO` formulas): Day Baal
    (14) `x.045`, Day Universe (15) `x.047`, Day Pet (16) `x.049` (= "Highest
    Multiplier in DPC"; validated — fixture `x.049` = 3,664,035,884 = the
    player's 3.664 E+9), Day Might (21) `x.065`, Day No Divinity (22) `x.068`,
    Road to Infinity (30) `x.134` (highest P.Baal), Day Extreme Building (60)
    `x.304`. Day No Rebirth (41 `BHLJEBDIMBK`) and Day Multiverse (54
    `GLLHBNBFPJM`) use a non-standard parser (key TBD); Day God Power (52) reads
    a computed method `HPOOCBOEMGH()`, not a single field. These score stats are
    now labeled in the Statistics block (editable in the tree), and
    `items::challenge_is_score_based(id)` flags the 10 score-based ids.
- **Overflow Points "left" is a computed method** `LLMCMCKAABP.JHDPFECPEOM()`
  (root.x); "Ultimate Overflow Points left" derives from `LLMCMCKAABP.IGFIHCDIGOM`.
- **Overflow-Point UPGRADE levels are stored at `root.013`** (`HNFHEBJIPEL`,
  marker `"OverflowBoosts"`, real `EBOFJJHOOLP` keys `a`–`n`; root key settled
  empirically — the `098` assignment is a decoy, `013` holds the `a`–`n` struct
  in the fixture). Each field = bought amount; the effect getter
  (`HNFHEBJIPEL.cs:39–63`) adds a base. Field→label (getter→raw-field traced):
  `a` Black Hole, `b` Black Hole Upgrade, `c` Gem Cap, `d` Gem Gain,
  `e` V2 Auto Kill, `f` Hp Regen, `g` Crystal Power, `i` Creating Stat,
  `j` Powersurge, `k` Creation Count, `l` Might Speed, `m` Stats Multi,
  `n` Space Dim; `h` has no getter/label (vestigial here). **Wired** as the
  "Overflow Point Upgrades" block.
- **ChP UPGRADE levels — MAPPED + WIRED (2026-06-20).** They are scalar fields
  directly on `root.X` (`MLILKGIALMB`, the `FIHAENJIDAO` accessor), NOT a
  separate object. Keys from the real deserializer (`MLILKGIALMB.EBOFJJHOOLP`,
  line 10036; the `n19`/`n41`/`-36` copies elsewhere are decoys), labels from
  the tooltip: `X.E` Planet Level, `X.I` Divinity boost, `X.D` Damage Reduction
  UBs, `X.041` Faster UB spawn, `X.G` Crystal Upgrade boost, `X.H` Damage Boost
  V2s, `X.J` CP boost, `X.039` Crystal Sacrifice boost, `X.029` BS boost,
  `X.030` CS boost, `X.K` TBS Level Loss decrease, `X.L` Pet Stone Drop boost,
  `X.035` Stone Pet improvement (bool), `X.019` Adv EXP boost (clamped 0–20),
  `X.V` Dungeon Drop boost, `X.W` Dungeon Exp boost, `X.037` Dungeon Overtime,
  `X.038` Quest Overtime (bool), `X.034` D4 boss room (stored; shown as 60 − x),
  `X.X` Crafting boost, `X.014` SpaceDim boost, `X.040` Self Replicating AI
  boost. Wired as the "Challenge Point Upgrades" block. **ChP *spent/used*** is
  Σ(level × per-upgrade cost) (`MLILKGIALMB.cs:894`; e.g. Planet Level 100,
  Crystal Upgrade 200, Damage Reduction UBs 2, Faster UB spawn 250). A few more
  upgrades feed that sum but aren't in the display tooltip (`FJBKFKPMHGB`,
  `MPPMMMAOKIC`, `LEODLEACJPA`, plus `LHFIMMKICKK` = `X.017` challenge max-tries)
  — left unlabeled pending an anchor.

- **ChP cost table + complete field inventory (2026-06-21).** The full
  ChP-*used* sum (`MLILKGIALMB.cs:894`) enumerates every **costed** upgrade as
  `level × cost`. So `used = Σ(level × cost)`, `available = total − used`,
  `total = Σ challenge contributions` (player's Steam save: 781 / 650 / 131).
  The 24 costed fields, key = per-level ChP cost: `X.D`=2, `X.E`=100, `X.G`=200,
  `X.J`=100, `X.H`=100, `X.I`=100, `X.019`=100, `X.039`=100, `X.040`=100,
  `X.K`=200, `X.L`=100, `X.036`=100, `X.042`=15, `X.V`=100, `X.W`=100,
  `X.034`=100, `X.X`=100, `X.014`=100, `X.017`=50, `X.015`=500, `X.037`=15,
  `X.029`=150, `X.030`=150, `X.041`=250. All but three are named/wired; the
  **3 still-unidentified costed fields** (need a Kong-diff anchor — player will
  spend ChP on a fresh Kong save and diff):
  - `X.036` (`FJBKFKPMHGB`, cost 100) — candidate **UBV4 Fight Timer** (appears
    near a UBV4/TBS toggle; MED).
  - `X.042` (`MPPMMMAOKIC`, cost 15) — candidate **Overflow Challenge Overtime**
    (cost matches Dungeon Overtime `X.037`=15, same "overtime" category; MED-HIGH).
    **Not present in the committed fixture** (older save) → can't wire until a
    save that has it.
  - `X.015` (`LEODLEACJPA`, cost 500) — unidentified (highest cost; LOW).
- **Boolean ChP upgrades — NOT in the clean debug tooltip; need anchors.** The
  wiki lists ~13 non-%-boost ChP upgrades (Pet Exp Overflow, Auto Half Stats,
  Auto Refill, Early Spacedim, Crystal Sacrifice *button*, Stone Pet *unlock*,
  Pandora Bonus, Auto Adjust Clones, Easier Rebirth, Keep Gem Level, + Extra/
  Second Party Slot, Max auto-add clones). `root.X` has ~20 **bool** fields at
  `X.004`–`X.031` (`EPJELCEGMHD`=`n4`, `HLAEDLHOLFA`=`n5`, … `DDNFPFMJJLF`=`n31`;
  full list via `MLILKGIALMB.EBOFJJHOOLP` `PIPMKFFGFHO` reads) — these *contain*
  the boolean ChP upgrades mixed with general UI/setting flags, but **cannot be
  separated/named without anchors** (the shop UI strings are decompiler-scrambled,
  and there is **no ChP-upgrade enum**). Pin these by toggling each upgrade on a
  Kong save and diffing which `X.00N` bool flips. Until then they stay as raw
  tree keys.

**Self-correct (2026-06-20):** the roadmap had ChP/Overflow filed as
*user-gated* (needing a before/after save). That was based on my earlier wrong
read that ChP was opaque. It is fully **C#-tractable**: ChP total is a derived
sum, Overflow Points is a derived method, and the upgrade levels are plain
stored fields (OfP at `root.013` done; ChP upgrades on `root.X` next). A
before/after save is *not* required.

**Root-level coverage gap — inventory (2026-06-20).** A sweep of the fixture's
root keys against the editor registry found the root is **not** fully opened
(correcting the roadmap's earlier "no struct sub-blocks remain"). Unmapped
root sub-blocks, with their C# class + real-deserializer marker + inferred
purpose + confidence (HIGH = clean internal marker / known sibling; MED = clean
marker, purpose inferred; LOW = scrambled marker, class only):

| key | class | marker | inferred purpose | conf |
|-----|-------|--------|------------------|------|
| `029` | `CDNMNLIAPKA` | `UltimateOverflowBoosts` | **WIRED** — Ultimate Overflow upgrades; `.a` = list of `FDJCCPFCJAO` `{a: IDFOIHJPCHP type, b: level}`; 6 types (1 Dungeon Slot, 2 Multiverse Rebirth Multi, 3 Multiverse GP Increase, 4 Multiverse Growth %, 5 Multiverse Growth Levels, 6 Higher PBaal) | DONE |
| `y`/`z`/`A`/`B` | `OEFPEEEHKDB` | `AchievementId` | Achievement/milestone state lists (`{a:bool, b:int}`); sizes 168/168/136/84 | HIGH |
| `P` | `HAGJGEFFJMM` | `PBaal` | **CONFIRMED** = the current P.Baal god-fight (reconciles with the existing registry label "Current God Fight"). `c` = **Highest P.Baal Defeated** (shown +1; in "The last P.Baal you defeated is …" / Day Baal Challenge); `a`/`e` flags, `b` accumulator (receives root.`b` at load), `d` running-max. Fields mostly transient → **not wired** (container already labeled; low value) | IDENTIFIED |
| `Q` | `LLCEGHMEDHK` | `AvatarOptions` | **IDENTIFIED** — avatar/cosmetic options: `a`/`b`/`d`/`e` bools, `c` (empty) + `f` = 62-entry owned-avatar list `{a:id, b/c/d, e/f bools}`. Cosmetic → not wired (low value) | IDENTIFIED |
| `012` | `ANNEDIJDLIC` | `BattleUBV4` | **IDENTIFIED** — UBV4 battle state: `a`/`b`/`c` scalars + `d` = 18 `{a:id, b:val}` pairs (`a`'s enum untraced; some nonzero). Battle/per-run state → not wired pending an anchor | IDENTIFIED |
| `014` | `DIGFPPNEEOC` | `RTIBonuses` | **WIRED** — RTI (Road to Infinity) bonuses; `.a` = list of 10 `HEIPGLPOGEJ` (`RtiElement`), one per `BDAFIPJBPFN` stat type (1 Physical … 10 CreatingSpeed); entry `a`=type, `b`=Bonus Amount, `e`=elapsed timer (`LDMJEPGEOME`), `c`/`d`/`g`/`h`=per-type values (neutral). `.b` = `&`-list of `HFNFDKEMAIK` special pets (not wired) | DONE |
| `027` | `JCNIFKADIBN` | `Multiverse` | **IDENTIFIED** — Multiverse system: `a`/`b` BigDouble, `c`=elapsed timer (`LDMJEPGEOME`), `d` bool, `e` int, `f` = nested `PKONEHBAJKG` `{a,b,c,d}`, `g` = 3-entry `NHJPCFHIDML` list `{a..g}`, `h`/`i` BigDouble, `j` int. **All zero in the fixture** (Multiverse unused) → not wired (no data to anchor field roles) | IDENTIFIED |
| `o` | `IFBFOMJDBLH` | `---MightsStart---` (also "Salamander"/"Elemental Manipulation") | Might / elemental detail | LOW-MED |
| `O` | `ACDDNFHBJCD` | (decoy "Event will be removed in:") | Events (active event state) | LOW-MED |
| `015` | `DNIMPNBKPGO` | `AvatarNew` / "Multi from UBv1C" | Avatar v2 or UBv1 multi — unclear | LOW |
| `031` | `JGAHOCEHBFJ` | (decoy; "bgmusic"/"_front") | settings / cosmetic — unclear | LOW |

Method to map any of these (per the usual recipe): find the class's real
deserializer (letter/numeric keys, **not** the prose-key decoys), label fields
from a debug/tooltip anchor, confirm the root key + non-empty struct in the
fixture, wire as a block, validate via the coverage test. Each is its own PR.
**Status (2026-06-20): the high-value root mapping is complete.** The two
genuinely valuable unmapped subsystems were wired (`029` Ultimate Overflow,
`014` RTI bonuses). The rest of the tail is now **identified but deliberately
not wired** because it is low-value or unanchorable: `P` (P.Baal god-fight,
transient), `Q` (avatar/cosmetic), `012` (UBV4 battle state), `027` (Multiverse —
all zero in the fixture). Still genuinely open but low priority: the
`y`/`z`/`A`/`B` achievement-claim lists (`{a:bool, b:int id}` — would need the
achievement-id→name catalog resolved to be meaningful), and `o`/`O`/`015`/`031`
(scrambled markers; would need an in-game anchor or a save where they're
populated). None of these blocks the editor; wire opportunistically if a
populated save or in-game screen later anchors the field roles.

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

**Class = `EBIIBHGAOMA`** (root key `032`/`n32`, field `HHLPENMEPBL`; from the
real root deserializer `PKCECBJFIHD.HPNLHANNDAM`). 43 top-level keys (skeleton
mapped 2026-06-19 via the C#; meanings of the sub-blocks still to fill in):

- `b` = **the adventurer core** (`KPJFCPPKHDL`, marker "MVBattleStats"; the same
  struct is reused for adventure enemies). Decoded: `a` = **entity** (enum
  `NFKHCMANAKF`: Player=1, then the full enemy roster Slime=50, Akuma=51, … —
  supersedes the partial `adventure_enemy_name`); `b`=level, `c`=exp. `e` =
  **class** (enum `APJDLMDFIGI` → `items::adventure_class_name`: Newbie/Thief/
  Archer/Warrior/Mage/Cleric/Rogue/Assassin/Sniper/Pyromancer/…/Onion Knight).
  **Per-field meanings (decoded 2026-06-20, player-confirmed where noted):**
  - `n` = **current HP** (clamped to the max-HP method `INJMAMDMHFJ()`;
    player-confirmed, live 4385), `o` = **current MP** (clamped to max-MP
    `AKAIHHFEFMM()`; player-confirmed, live 73.9).
  - `p` = **recovery timer** — while >0 the entity shows "Recovering"; 0 = active
    (live 0).
  - `q`/`r` = the entity's **screen X / Y** (passed as `(x,y)` to UI-element
    constructors; default 110/150, live 110/150).
  - `g` = **battle skills** (`PGEICDFPINA` = AdvBattleSkill instances, live 32),
    `m` = **equipment** (`DDKDNIFCAJO`, the same adventure-gear class as `032.c`,
    live 8), `s` = **active pill** (`BEFDMHPNDHH` = AdvPill buff; feeds the Attack
    calc; empty when none), `t` = **skill loadout** (`OKOCFJJNMAK` = SetSkill
    assignments, live 4), `f` = **per-class progression** (`HGKLOMCJAIM`: one
    record per class the player has leveled — `a`=class id, `b`=level, `c`=exp,
    `d`=flag; class levels advance independently, so live 11 = 11 of the 20
    classes leveled). Wired as its own sub-list block (`["032","b","f"]`, entries
    titled by class).
  - `h`/`i` = **skill-id** lists (enum `ADCGDPGPBOI` → `items::adventure_skill_name`);
    `h` (a second slot) is **absent when empty**.
  - **Still unidentified:** `d` (BigDouble, feeds the Attack calc as `0.8·d/5`;
    live 1), `j`/`k` (stored BigDoubles with no in-class reads; live 136 /
    1,064,697), `l` (tracks a running max of something; live 1923).
  - **Note (obfuscation):** the displayed combat stats — HP, Attack, Defense,
    **Int** (= Intelligence, a real game stat), Resistance, Hit, Speed — are
    **computed methods** (`INJMAMDMHFJ`/`AOCPCCNNCLH`/`INCKGPBAMJO`/`GKIIMNNGKEN`/
    `HGFDNNMJMAK`/`HLBPJJNDGMJ`/`HNLGEDBOCAK`), not stored fields, so `d`/`j`/`k`/`l`
    are *not* those stats. (This is why the old type-placeholder label "Int" was
    misleading — it meant the int data type, not Intelligence.)
  Class + skill enums transcribed verbatim and **diff-verified** against the C#
  (34 classes / 158 skill ids, 0 mismatches). (`NFKHCMANAKF` enemy roster now
  fully in `items::adventure_enemy_name`.) **Live-confirmed on the 06-09 save:**
  `032.b` = `a`=1 (Player), `b`=116 (level), `c`=18,050,134 (exp), `e`=20
  (**Rogue** class), `i`=`19&6&48&5` (skills Dodge / Speed Boost / Dual Wield /
  Drops Boost) — matches the decoded enums exactly. Wired into the save-editor
  raw tree view (BlockSchema `["032","b"]`; `a`→Entity, `e`→Class resolve).
- `H` (`OKLONIELNEN`) → `H.a` = **researches** (below).
- `d` = **inventory** (below), `G` = **cores** (below).
- Sub-structs (markers from each class's `FromString`):
  - `k` (`MANFDMLBOMG`) = **AdvCrafting** (smithing): `a` = craftable gear (enum
    `LEIFLPFLEHJ` — ClothMantle/LeatherBelt/Blazing…/Aim… families), `b`/`g` =
    material (enum `BFNFKADNAKD`), `c` = exp/progress, `d`/`e`/`f` = levels.
  - `n` (`JADFDPJGJPA`) = **AdvAlchemy**: `a` = recipe (enum `DLCMNADKOJK`),
    `c` = exp/progress, `d`/`e` = levels.
  - `V` (`GIDEBPNNIDA`) = **Bestiary** (per-enemy kill records).
  - `z` (`NKAIFFEMNBO`, field `LMAELFDLLFN`) = the **currently-active recipe job**
    — the in-progress item the adventurer is working on, reassigned in the
    Crafting/Mining/Smithing/Alchemy action handlers
    (`LMAELFDLLFN = IMMBOLLGALI; HNOONEJHFDF(DCCKNPICIKG.Crafting, …)`).
    `NKAIFFEMNBO` *subclasses* the AdvCrafting base `MANFDMLBOMG` and adds the
    per-discipline ingredient-table builders (Smithing/Crafting/Battle/Alchemy/
    Mining). It **is** string-deserialized (constructor → the inherited
    `EBOFJJHOOLP`) and round-trips at key `z` via `KFIDJLHOBCO()`. (Corrects the
    earlier "TBD / not string-deserialized" note — it's the active-job mirror of
    the per-discipline states `k`/`n` above.)
  - The crafting **material** enum `BFNFKADNAKD` (107 ids: raw/refined/enhanced
    tiers + jewels/pills) is the same namespace as `032.d` inventory and is now
    complete in `items::adventure_item_name` (diff-verified). The craftable-gear
    `LEIFLPFLEHJ` (122 → `items::adventure_craft_gear_name`) and alchemy-recipe
    `DLCMNADKOJK` (27 → `items::adventure_recipe_name`) enums are now transcribed
    + diff-verified.
  - **List keys** (each `&`-joined; element class → purpose, from `FromString`
    markers): `c` = `DDKDNIFCAJO` adventure **equipment** (PetEquip-shaped),
    `d` = `PMHOCMNLNAE` **inventory** (the `032.d` materials), `j` = `OPPFHHDLOPE`
    **AdvSkill** (learned skills), `m` = `OLLAMHHGMAE` **AdvPotion** (potions),
    `G` = `IDDNEKHJMJB` **cores**, `L` = `BOGKBENOHDM` **MonsterCoreProgress**,
    `R` = `KPJFCPPKHDL` (a **list of MVBattleStats** — the enemy/bestiary
    battle-stat entities, same class as the single adventurer at `032.b`),
    `U` = `PPIEAILEOLF` **AdventureArea** (zones). The scalar `z` = `NKAIFFEMNBO`
    is the **active recipe job** (see the sub-structs list above — it *is*
    string-deserialized). Only a few bool flags remain unnamed. The whole
    `root.032` block is now structurally mapped.

`032.H.a` = **researches** (44 entries, id 0 is a placeholder): `a`=id,
`b`=level, `c`=in-progress flag, `d`=progress, `f`=max level. Ids 1–43
match the Main Stats export's "Researches" section in order, verified
43/43; exactly two entries had `c`=1, matching "Research Slots Level: 2".
Notably **id 28 = Pet Stats** ("multiplies the stats your pets gain from
growth, not dungeon stats", +1%/level) — at level 5 this is the ×1.05
factor in the normal-stats global multiplier.

### `032.d` — Adventure inventory & `032.G` — cores (player-decoded 2026-06-18)

`032.d` = **Adventure-mode inventory**: per entry `a` = item id, `b` = count
(`c`/`d` = 0). A **separate id namespace** from the main `X.Q` materials *and*
from the core enemy ids below (collisions: id 50 = Cloth here but Slime as a core,
53 = Common Herb but Astaroth, 64 = Small Bottle but Ghoul). The full 32-item set
is now identified (`items::adventure_item_name`, from a Steam export +
`Notes.txt`), clustering by material family: 1–4 raw (Sticky Fluid / Rough Hide /
Bag of Sand / Heat); 50–68 cloth/herbs/etc.; 100–121 metals; 150–151 planks;
199–201 Golden Chestnut / Refined Cloth / Refined Leather; 220–221 Refined Metal /
Iron; 240–241 Enhanced Pine / Beech; 260–261 Enhanced Paper / Fire Jewel; 265–266
Dark / Light Jewel. (Id 63 = Flask appears only in the Kongregate save.)

`032.G` = **cores** (the "<enemy> <quality>" drops): per entry `a` = enemy id,
`c` = count, **`d` = quality on the same 0–8 F→SSS ladder as equipment**
(verified entry-by-entry — Slime F=`d`0, E=1, D=2, B=4, A=5, S=6, SS=7, SSS=8);
`b` = 1 in every entry (unidentified). All seven enemy ids are now named
(`items::adventure_enemy_name`): 50 = Slime, 53 = Astaroth, 63 = Goblin, 64 =
Ghoul, 69 = Imp, 72 = Wraith, 87 = Shinigami. Decoded from a redacted Steam save
(`Steam/Adventure Mode Items and Cores/`) plus the earlier Kongregate save.
Promoted to `SaveFile.adventure_inventory` / `cores`.

## `root.009` — SpaceDim / Light Dimension (decoded 2026-06-13)

`009.a` = light clones (whatever is allocated to the active element). `009.b`
= the **20-element list**, in the in-game display order:

| key | meaning |
|-----|---------|
| `a` | element id = display order (1 = Controlled Entropy … 20 = Self Replicating AI) |
| `b` | light clones allocated to this element (only the active one is nonzero) |
| `c` | **level** |
| `d` | **next-at** (player-set target level — see the note below) |
| `e` | progress / accumulated clones toward next level |
| `f` | **spread** (player-set ratio for the "spread" button — not a fixed priority; see the note below) |

Verified end-to-end against the 2026-06-13 notes: save 1 Quantum Genesis
`c`=2, Fusion Torch `c`=18 (b=70,362 — all light clones), Dyson `c`=22; save 2
Fusion Torch `c`=70, Dyson 23, Quantum Genesis 6, etc. — and every `d`/`f`
matches the notes' Next-At/Spread columns. Levels reset per rebirth.

### "Next At" and "Spread" — the clone-spread menus (player-clarified 2026-06-18)

Both are **player-set**, not game-computed, and largely **independent** of each
other. The `next_at`/`spread` pair shows up in every menu where you allocate
Worker Clones across a list of things — **Monuments** (`D`), **Mights** (`V`),
**SpaceDim** (`009.b`), **Divinity Generator upgrades** (`K.l`), etc.

- **Spread** is a *ratio* used by the "spread" button: pressing it distributes
  the menu's clones in proportion to each item's spread value (an item with
  spread 2 gets twice the clones of one with spread 1). Any value is allowed —
  high spreads level a chosen item faster. It is **not** a fixed priority rank
  (the earlier "20…1 priority" reading was wrong; those numbers are just the
  player's chosen ratios).
- **Next At** is a *target level*. An item keeps leveling until it reaches its
  Next At, then clones move on to the next item — either by rolling down the list
  as you manually dump all clones into one thing, or via the spread button (which
  simply skips anything already at its target). Once every item is at its Next At,
  the menu's allocated clones go **idle**. (This is why `K.l[i].f` read a constant
  512 while the level climbed toward it — players rarely change these until
  pushing for higher values.)

**Creations** (`i`) reuse the **Next At** idea but not the clone allocation:
creating is passive, so Next At only sequences *what* gets created ("create Light
until its target, then Stone until its, …"), typically set to the per-rebirth
creation achievement breakpoints. Creations have no spread.

## `root.T` — Baal Slayer (decoded 2026-06-13)

The Planet block (`AIDFNOPNJGK`, marker "Planet"). Confirmed: `T.h` = **unspent
Baal Power** (0 → 334 between the two 2026-06-13 saves ✓). `T.f` = list of 5
regular **Ultimate Beings** (`CEFAAPALBMD`) with `d` = a staggered spawn
countdown (1 h / 3,600,000 ms apart; all decrement in real time), `b` = kill
count, `f` = GP gained.

`T.k` = list of 5 **Ultimate Being V2** records (`FPBMNCNKPHN`), **corrected
2026-06-22** (player-reported, C#-confirmed). These are *not* "TBS tracks" — the
C# gates this list on the `UBV2C` challenge (`AIDFNOPNJGK:560`, list type
`FPBMNCNKPHN`), so they are the UBv2 bosses (same names + " V2"), each defeated
**once per rebirth**. `c` = UB id (1..5), `b` = cumulative defeats across
rebirths (drives "Multi from Ultimate Beings": +1%/12%/21%/32%/45% per defeat),
`a` ≈ 100 (per-UB state, exact role unconfirmed). The ref save's `b`=
{39,27,25,21,16} is many rebirths' worth; a fresh test save reads {1,1,1,1,0}
(killed the first four V2s, never ITRTG V2 — matches in-game). The on-screen TBS
component levels live separately at `root.S` (see below).

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
| `a` | item type id — the `MBBDNNAMMHO` enum (`Assembly-CSharp`); ~110 types. Resolves the old {23,26,30,52,56} tie: **23=Flood Spear, 26=Tree Axe, 30=Hurricane Bow, 52=Iron Pot, 56=Leeching Sword**. Curated id→name+slot subset in `items::equipment_type_name` / `EQUIPMENT_TYPES`. | verified against `MBBDNNAMMHO` |
| `b` | plus level (+10/+20/+13); stat multiplier = 1 + 0.05×plus | ✓ export + wiki scaling rule |
| `c` | quality: A=5 baseline, ±1/step — full ladder **F=0 E=1 D=2 C=3 B=4 A=5 S=6 SS=7 SSS=8** (player-confirmed 2026-06-17; matches enum `GBFGHANMFII`, which also names a 10th tier **`Ult`=9** — but the equipment loader **clamps stored quality to 8**, so 9 never persists); stat multiplier = 1 + (c−5)×0.1 | ✓ export + wiki + `GBFGHANMFII` |
| `d`, `h` | **`d` = equip-reference id** (what pet slots `w.e/f/g` reference; `0` = unequipped/inventory) — unique among *equipped* items but **0 for every unequipped one**, so NOT a per-instance identity. **`h` = the permanent unique instance id**, assigned once from a global counter (`Assembly-CSharp` `ACDDNFHBJCD`: `if h==0 { h = counter+1 }`) — **never 0, never repeated**. Quantified 2026-06-19 on the 06-09 Steam save (209 instances): `h` 209/209 distinct, `d`=0 for the 30 unequipped and unique for the rest. Slots match by `d` — the game's `CIEAPBPBCLL` does `FirstOrDefault(x => x.d == slot)`. `d`/`h` usually equal, but **event gear diverges** (Vampire armor `d23`/`h136`=Merry Mantle; Bee armor `d21`/`h126`=Creators Vest; Christmas Boots), and the pet slot follows `d`. So **resolve slots by `d`**; use `h` to address a specific instance (esp. unequipped). `EquipmentItem.instance_id`=`d`, `EquipmentItem.unique_id`=`h` (+`equipment_by_unique_id`). **⚠ Bug fixed 2026-06-19:** the save-editor equipment table's "equipped on" lookup keyed by `h` while pet slots use `d`, so `d≠h` event gear wrongly showed as *Unequipped*; now resolved by `d`. | referenced by pet `w.e/f/g`; verified vs `ACDDNFHBJCD`/`CIEAPBPBCLL` |
| `e` | **Enchant level (0–20)** — reduces the item's elemental weakness 5%/level (max 100%); for neutral gear it instead adds elemental stats (+weapon_tier/2.2 per level, or tier/1.25 for Tier 4). Crafter/adventurer gear (sticks, blacksmith hammers, magic pots, ear-muffs) auto-starts at 20; dungeon gear starts 0. **Self-correct 2026-06-21:** was guessed "max-plus cap", but items with plus 20 keep `e`=0 (e.g. fixture `X.R[8]` type 301 plus 20 `e`=0) while type-79 items show `e`=20 — so it is NOT a plus cap. C# confirms ("Number of enchantments, max 20.", "Enchants will increase only the elemental stats", an "enchant level" sort). Model field renamed `plus_cap`→`enchant_level`; labeled "Enchant Level"; `add_equip_instance` now defaults `e`=0; editable 0–20 in the equipment editor. | C# enchant getter + save data |
| `f` | gem level (10) | ✓ |
| `g` | gem element — enum `EMGELCMNFOL`: 0 Neutral, 1 Fire, 2 Water, 3 Earth, 4 Wind, **5 Dark, 6 Light, 50 Elemental, 99 All** (richer than the pet 0–4 element set) | ✓ + `EMGELCMNFOL` |
| `i` | bool, false in every save so far (`PIPMKFFGFHO` reader) | ✓ type from C# |

### Equipment effect formula (from C#, `DOBKHNKLLLM`)

Every equip's effect magnitude is, generally:

```
value = base(effect) × (1 + quality_id) × (1 + plus) × eff_factor
```

- `quality_id`: F=0 … SSS=8 (so the "quality multiplier" is `1+id`, 1…9).
- `base(effect)` — set by **effect type** in `BEJDIJPMHPO`: default/CampaignBoost
  `0.088185`; CampaignBoost2 = that ×1.5 (`0.1322775`); SmithAlchCampBoost/
  OwlTeaching `0.0712261`; BuildingSpeed/CreationSpeed/DualMimic `0.004407`;
  CreationEvent/BuildingEvent `0.02204585` (+ flat `0.00881834`); DefBreak
  `0.0017635`; DefBreak2/PassiveGrowth `0.00088175`; SmallSmithing `0.05291005291`;
  LeechExp `0.0146975`. Special-cased: **BonusExp** = `min(2.5·plus,8)·quality_id`,
  **BonusGrowth** = `min(1.5·plus,8)·quality_id`.
- `eff_factor` = the per-item `NJDOCOGAJEM`, **except 4→6, 5→12** (SoulSword→3).

**Verified:** Magic Stick (CampaignBoost, factor 3) → `0.088185·3 · 9 · 21 =
50.0%` (its "up to 50%"). Candy Cane (CampaignBoost, factor 4→6) →
`0.088185·6 · 9 · 22 = 104.76%` at SSS+21; SSS has hardcoded overrides (only
+30-capable item): `+20→101`, `+25→125`, `+30→150`. (`items::campaign_boost_pct`.)

#### Event / special-item effect & factor table (`AOCFDHHLDDH`)

| item | effect | `NJDOCOGAJEM` (→eff_factor) |
|---|---|---|
| Magic Stick | CampaignBoost | 3 |
| Candy Cane | CampaignBoost | 4 (→6) |
| Merry Mantle, Christmas Boots | CampaignBoost2 | 4 (→6) |
| Creators Vest | CreationEvent | 4 (→6) |
| Godly Hammer | BuildingEvent | 4 (→6) |
| Learning Coat | BonusExp | 4 |
| Magic Egg | BonusGrowth | 4 |
| Growing Love Pendant | PassiveGrowth | 4 (→6) |
| Haposti | SmithAlchCampBoost | 4 (→6) |
| Spectrometers | AlchCostRed | 4 |
| Master Gloves | BSSpeed | 4 |
| Rune Patch | BlacksmithExtra | 4 |
| Celestial Bow | DodgeIgnore | 4 |
| Gram | PatreonBoss | 4 |
| Wonder Axe | DefensePierce10Perc | 5 (→12) |
| Enlightment Vest | CraftingSpeed50 | 5 |
| Ele Twin Dagger | DualMimic | 5 (→12) |
| Shroud of Enlightenment | OwlTeaching | 5 (→12) |
| Hungering Talon | OwlFeeding | 5 |

So every event item's effect value is computable at any quality/plus. **To verify
in-game** (cheap cross-checks the user can do): Merry Mantle / Christmas Boots
(CampaignBoost2 — is it a campaign boost or something else? tooltip text was a
decoy), and the per-tier values at S+10 (pet-stone) vs SSS+20 (cash). The
PassiveGrowth base for the Growing Love Pendant is a lead on the pendant's
per-hour rate (a separate open item). See `decode_roadmap.md`.

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

## Reading the format from `Assembly-CSharp` (the C# cross-check)

The game is Unity/Mono, so `Assembly-CSharp.dll` decompiles cleanly. Decompiled
to the gitignored scratch dir (`_cs_decomp/`, never committed — it's copyrighted
game source). The assembly is **obfuscated** (class/field names are random
letters) but the save **keys are not generated** — each (de)serializer passes
literal key strings, so the code is a direct Rosetta Stone. Method/field names
cited below are evidence pointers, **not** pasted source.

Key facts about the framework (class `OMHGFFEADBC`):

- Struct = `key:value;`, list = `&`-joined; the key constants are the literal
  `"a".."z","A".."Z"` and the numeric `"001"…` strings.
- The obfuscator emits **many decoy copies** of every (de)serializer, with junk
  prose/enum string keys in dead branches. The **real** deserializer is always
  the method named **`EBOFJJHOOLP(string)`** (clean single-char keys); the
  serializer is `KFIDJLHOBCO()`. The value getter is `BDEAAELBJKM(arr, key)`;
  typed readers wrap it (`…IJAHJNNEBEB`=int, `…LDNFLIFGNCH`=BigDouble,
  `…PIPMKFFGFHO`=bool, etc.).
- Class map confirmed: root = `PKCECBJFIHD`; root `X` (pet system) =
  `MLILKGIALMB`; its `b` list element (the pet) = `DFLAKHONNPC`; pet `w`
  (dungeon) = `CIEAPBPBCLL`, whose `d` (class) = `PJEGDBJIOAL`. X-list element
  classes: `Q` (materials) = `GCJMGGFGKBN` (item `a` = `NCPJFPLCPPK` enum),
  `R` (equipment) = `DOBKHNKLLLM` (type `a` = `MBBDNNAMMHO` enum), `S` (teams) =
  `PCDCANGLENI`.

The **real root deserializer** is `PKCECBJFIHD.HPNLHANNDAM` (not the short decoy
`LGCNFPEKBCI`). Its sub-block classes: `e` = `NBEFGPGOCOL` (shadow clones),
`p` = `DFGCALKGABP` (god-power/permanent upgrades), `K` = `OHLKALIOOKC` (divinity
generator), `O` = `ACDDNFHBJCD` (statistics — the 662-reader giant),
`P` = `HAGJGEFFJMM` (current god fight), `S` = `MONHKGNNJJK` (TBS component
levels), `T` = `AIDFNOPNJGK` (Baal slayer), `x` = `LLMCMCKAABP` (the `root.x`
tracker block), `X` = `MLILKGIALMB` (pet system). Still-unnamed root fields, with
types now pinned from `HPNLHANNDAM` (all **scalars**, not struct sub-blocks):
`b`/`d` = BigDouble; `l`/`m`/`n` = int, **default 1000** (save: 50/50/50) — the
**Shadow-clone stat divisors** (`Idle / All Shadow clones`; clone stat = your
stat ÷ divisor, so a **lower** value = stronger clones; set to `1000 − upgrades`
in `COMKEGGKPLD`). Pinned precisely from the clone tooltip
(`KFOECKINJAP.cs:15969`) + the property getters: **`l`** (`KKPOGIDPMPI`) =
**Attack** divisor (`IOHAICBLAOL = baseAtk / l`); **`m`** (`OFAEPCBJKGC`) =
**Defense** divisor (`EIFOLHNKCIH = baseDef / m`); **`n`** (`DGBEIOCFMLJ`) = **HP**
divisor (`LBFJPKGLPPM = baseHp / n`, the HP cap used in the pet training tick).
`q`/`t`/`u` = bool; `r`/`s` = string — **both PII account identifiers** (now
redacted): **`r`** (`OBBCNEEELEN`) = linked **Kongregate user id** (uploaded as
`KongUserId`), **`s`** (`DJJMJOHIHPO`) = the account **login/UserName** (uploaded
as `UserName`/`KongName`; `s` is also copied into the god-title field `g` under
one condition — likely a legacy/backup). **`U`** (`LIKCHFOLHKI`,
long ms) = the **"rested"/refreshed crafting-speed timer** — while >0, *"your
creating speed is tripled"* (Adventure crafting). On return from offline it banks
**1/6 of the offline time, capped at 10,800,000 ms = 3 h** (`OfflineCalc.cs:277-285`);
during active play it drains by elapsed time, floored at 0 (`UpdateStats.cs:5053-56`).
**`d`** (`DILEDALONFM`, BigDouble) is **vestigial** — it appears only in its
property declaration and the (de)serializer; it is never read anywhere in the
decompiled code (same category as the pet `z`/`D` flags). **`b`**
(`GDIIPNIODAD`, BigDouble) is a one-shot **pending value consumed at load** (if
`b`>0 on load it sets the recompute flag `ENLNNIPOEBB` and stashes `b` into a
root `CCIDPLFLDIA`, then zeroes `b`); its player-facing meaning is unclear and is
left unnamed. Meanings of `q`/`t`/`u` (bool) still need a per-field usage chase.
No plain
stored root integer for Challenge Points was found (consistent with ChP being
derived); Overflow Points, if stored, sits inside a named sub-block (e.g.
statistics `O`).

A small helper, `_cs_decomp/_extract_fields.py`, scopes to a class's
`EBOFJJHOOLP` (or a named method) and prints its key→(type, field) table (it
misses enum-cast reads like `(HFNFDKEMAIK)…(arr,"k")` — read those by eye).
Running progress notes live in `_cs_decomp/_PROGRESS.md` (also gitignored).

### Pet type & form enums (authoritative)

- **Pet type id** (pet `k`, partner `F`; 999 = None) = enum `HFNFDKEMAIK`,
  transcribed to `items::pet_type_name`. 152 pets (ids 0–151) plus specials at
  750–803 / 900–902 / 999. Every prior hand-derived anchor matches
  (2=Cat, 25=Reindeer, 32=Pandora, 89=Salamander, 123=Vampire, 803=Serow).
  Names are the **export-normalized** spellings (Reindeer, BHC, Firefox, …).
- **Elemental form** (pet `y`) = enum `ANHOKMNPAKI`, transcribed to
  `items::elemental_form_name`. This reveals the full **water/`Undine`** family
  (`FailedUndine`, `UndineV1`–`V4`, `UndineFinal` = ids 3–8) — the family
  `FINDINGS` previously guessed was "the unnamed count-0 block" — plus the
  `Gnome`/`Salamander`/`Sylph` ladders and the `LostArm`/`LostBody`/`GrayChild`
  specials. The fixture offsets all check out (Gnome `y`=14=GnomeFinal,
  Salamander `y`=19=Final, Sylph `y`=24=Final). Note Salamander/Sylph start at
  `V0` and Gnome at `V1`; each line ends in `…Final` (the displayed "V4").

Both enums are regression-tested in `tests/real_save.rs`
(`pet_type_ids_all_resolve_to_names`, `elemental_form_names_resolve`): every
type/partner id in the reference roster resolves, and the elemental forms match.

## Open questions / next steps

- Pet fields: **fully decoded** (every key identified — see the pet table).
  Beyond the long-known fields, the C# chase named: `t` (favorite camp) / `u`
  (hate camp), `d`/`e`/`f` (additive growth components, total = E+d+e+f), `n`
  (growth pool), `x` (feeding setting), `A` (vaccinated), `s` (recovery-cooldown
  timer), `C` (cosmetic skin index), `z`/`D` (vestigial). Promoted to the typed
  model: `favorite_campaign`/`hated_campaign`, `feeding_setting`, `vaccinated`.
- HP/Attack/Defense/Speed/elemental affinities from the Pet Stats export do
  not appear literally in the save → derived at runtime. If we ever need them,
  we either keep using the export or reverse the formulas.
- `X.v` = Chocolate count (resolved), `X.T` = crafting roster (decoded, see its
  section), `X.028` = achievements/milestones catalog (resolved, see the X table),
  pet `t`/`u` = favorite/hate campaign (resolved). Remaining root-struct
  unknowns are now just the bools `q`/`t`/`u` in the root deserializer (`d`
  BigDouble = vestigial/unused; `b` BigDouble = a load-time pending value;
  `l`/`m`/`n` = Shadow-clone Attack/Defense/HP stat divisors; `U` = the "rested"
  tripled-crafting-speed timer; `r`/`s` = Kong user id / account name — all
  pinned) and Overflow Points (likely inside a stats sub-block) — all low-value.
- Material id ↔ name: **`material_name` now transcribes the COMPLETE
  `NCPJFPLCPPK` enum** (`items.rs`) — base materials, keys/alloys/runes, the four
  elemental evolution-quest families (water/`Undine` 106–116, Gnome 126–129,
  Salamander 138–145, Sylph 146–150), T4 stones, sparks 350–352, and the fishing
  rods/baits/catches 500–567 — so every item names in the save editor.
  Export-confirmed display spellings (16/17 Health Potion X/S, 19 Antidote, etc.)
  are kept verbatim; foods 101–105 are the same enum but stored in `X.c/d/e/v`.
  The count-1 ids {160,164,167,168} are **resolved** from the `NCPJFPLCPPK` enum
  (the master material/item id table — X.Q item `a` casts to it): 160 = Not
  Nothing, 164 = Absolutely Nothing, 167 = Food Journal One, 168 = Food Journal
  Two. **⚠ Bug found & fixed in the same audit:** the three 10-count elemental
  bars were mis-assigned (indistinguishable by count → the prior project guessed
  the element order). Enum truth: 33 Inferno (fire), **34 Forest (earth)**,
  35 Hurricane (wind), **36 Titanium (neutral)**, **37 Tsunami (water)** — the
  old table had 34=Tsunami / 36=Forest / 37=Titanium. Corroborated by
  `dungeon_recommendations.yaml` (neutral Scrapyard event → Titanium Bar, Water
  Temple event → Tsunami Bar). The enum also names the whole namespace incl. the
  **`Undine`** water-quest family (106–116), fishing items (500–567) and sparks
  (350–352); `items.rs` keeps the X.Q subset at display spellings.
  **130 = Aether Ring** (player-confirmed 2026-06-18; base ring on a fresh save
  is id 130; the in-game "+N" boss-kill suffix is the same id 130 with a dynamic
  name, not consecutive ids since 131=Sun Stone) and **162 = Monster Blood**.
  The **T4 materials are resolved** (save-edit
  probe 2026-06-16, counts 41–45 read off in-game): 131=Sun Stone, 132=Jungle
  Stone, 133=Sky Stone, 134=Mythril, 135=Ocean Stone — now in `items.rs`.
  Player-confirmed 2026-06-18: dungeon consumables **30=Melting Bomb,
  48=Nanotrap, 49=Freezing Bomb**, plus **120=Cure**. The 126–149 cluster is
  the **elemental-pet evolution-quest materials** (you craft a family of items
  to give each elemental pet through its "quest" — *not* dungeon-boss drops):
  Gnome/earth (126 Core Shard of Gnome, 127 Magic Soil, …), Salamander/fire
  (138 Glowing Embers, **139 Igneous Bones**, **140 Pliable Magma**, 141 Living
  Flame), Sylph/wind (146 Whispers, 147 Secrets, **148 Mysteries of the Wind**,
  149 Soul of Sylph). The water family is the **`Undine`** line (now named from
  the C# `ANHOKMNPAKI` form enum — see "Pet type & form enums"); its quest
  materials are presumably the unnamed count-0 cluster.
- Equipment *type* id ↔ name: solved for everything equipped, derived by
  joining Pet Stats gear strings ↔ Pet Equips instance ids ↔ the save's `R`
  instance→type map (zero vote conflicts). 2026-06-13 the user equipped five
  of the previously-ambiguous types in-game and read them off save 2's
  instance→type map: **5 = Flame Armor, 8 = Flood Armor, 22 = Water Spear,
  41 = Tree Bracelet, 44 = Storm Ring** (resolving the 44 = {Magic Hammer |
  Storm Ring} tie). 2026-06-19, same method (Anteater/Salamander/Caterpillar):
  **48 = Magic Hammer** (the real one), **80 = Legendary Stick**, **81 =
  Legendary Pot** (the 79/80/81 Legendary crafting-weapon family). Now in
  `items.rs::equipment_type_name`. **{23,26,30,52,56} resolved** from the
  `MBBDNNAMMHO` enum (23=Flood Spear, 26=Tree Axe, 30=Hurricane Bow, 52=Iron Pot,
  56=Leeching Sword). **`equipment_type_name` now transcribes the COMPLETE enum**
  (~110 types, ids 1–86 base grid + 140–311 special/event), so every owned item
  names in the editor; `EQUIPMENT_TYPES` stays the curated slot-categorized subset
  for the builder, guarded by a test that its names match the full table.
- Challenge dungeons "available" (3/10 → 2/10 after using one attempt
  2026-06-13) is **not** a stored integer — no field went 3→2. It is computed
  (regen timer + used-counter), like an energy bar. Not yet located.
- The 2 leading junk chars: **resolved** — constant literal version tag `"V2"`
  (verified in `Assembly-CSharp`; see Container format).
- Re-serialization (writing a save) untested — only needed if we ever want to
  edit saves, which is out of scope for the planner.
- Token/evolution state (export "Other" column) — **partly located**: the
  elemental-pet **form** (`GnomeV2`/`SylphV1`/… in "Other") is pet field `y`
  (see the pet-struct table; `SavePet.elemental_form_id`, parsed from the
  export by `ExportPet::elemental_form`). The other "Other" contents (fight
  counts, day counters) are still unlocated among `t`,`u`,`x`,`z`,`A–D`,`H`.

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
