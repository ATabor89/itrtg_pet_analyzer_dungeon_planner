# Handoff: finish the save model from the decompiled C#

**For a local Claude Code session on the Windows machine.** Goal: stop probing
save values in-game one at a time and instead read the ground truth straight out
of the game's own serialize/deserialize code, then close out the remaining
unknown fields in `crates/save-parser` and `FINDINGS.md`.

This is feasible and should be fast: the game is Unity with the **Mono** backend
(an `Assembly-CSharp.dll` exists and there's a `MonoBleedingEdge` runtime
folder), so the C# decompiles cleanly — no IL2CPP/Ghidra detour.

## Why this works even if symbols are obfuscated

The save keys are **string literals** (`"a"`, `"032"`, `"001"`–`"004"`, and the
`:` `;` `&` delimiters). String literals survive obfuscation, IL2CPP,
everything — they're data, not symbols. We already know ~70 key→meaning mappings
from in-game probing (see `FINDINGS.md`). That's a **Rosetta Stone**:

1. Find the serialize and/or deserialize method by searching the decompiled
   source for a distinctive known literal key.
2. Confirm the field assignments around the *known* keys match our table.
3. Once they do, the **same method body hands you every remaining key at once**,
   in serialization order (which equals our parse order).

Even in the worst case (fields literally renamed to `a`, `b`, …), you still gain:
field **order** (→ positional mapping of the unknown pet fields), each value's
**computation** (answers "stored vs. derived" questions), and **types**
(disambiguates id collisions). Realistically these custom idle-game serializers
are hand-written with short keys for save-size while the C# fields keep
meaningful names — likely a direct dictionary.

## Game file locations (reference in place — read only, never modify)

- Managed assemblies:
  `F:\SteamLibrary\steamapps\common\Idling to Rule the Gods\Idling to Rule the Gods_Data\Managed\`
  - Main target: `Assembly-CSharp.dll`
  - Also worth a glance: `Assembly-CSharp-firstpass.dll`, any non-Unity DLLs.
- Mono runtime (confirms backend, not needed for decompile):
  `F:\SteamLibrary\steamapps\common\Idling to Rule the Gods\MonoBleedingEdge\`

You may read these files directly. **Do not modify anything under the game's
install directory.**

## Setup / prerequisites

1. .NET SDK 8+ (`winget install Microsoft.DotNet.SDK.8` or dotnet.microsoft.com).
2. `dotnet tool install --global ilspycmd` — headless decompiler.
3. (Optional GUI for hand-spelunking) **dnSpyEx** — standalone zip, the
   maintained dnSpy fork. Good for "Analyze → Used By", string search, and
   stepping through deserialize if a value's provenance is unclear.

Decompile the whole assembly to a **gitignored** scratch folder (already in
`.gitignore` as `_cs_decomp/`):

```
ilspycmd "F:\SteamLibrary\steamapps\common\Idling to Rule the Gods\Idling to Rule the Gods_Data\Managed\Assembly-CSharp.dll" -p -o reference/save_file_deserialization/_cs_decomp
```

`-p` = project layout (one file per type → far easier to grep than one blob).

> **Repo hygiene — important.** This repo is **public**. The decompiled `.cs` is
> copyrighted game source: it stays in `_cs_decomp/` (gitignored) and is **never
> committed**. Commit only *derived* knowledge — the key→field mappings written
> into `FINDINGS.md` and promoted into the Rust model. Do not paste large
> verbatim decompiled method bodies into committed files. (Same spirit as the
> no-raw-saves rule.) Work on the branch `claude/sharp-franklin-anxs8f`.

## How to locate the serializer

Search the decompiled tree for anchors. The deserializer (parse/load) is usually
the clearest — look for a `switch`/`if` chain over the short keys, or a split on
`';'` then `':'`. Good distinctive literals to grep for:

- `"032"` (adventure block), `"009"` (SpaceDim), the timestamp key handling.
- The container/delimiter logic: a split on `';'`, `':'`, and `'&'` together.
- Compression: `GZipStream` / a length-prefix write, and the inner/outer
  Base64 (`Convert.ToBase64String` / `FromBase64String`). This pins the
  **container** code (see `FINDINGS.md` "Container format") and confirms the
  Steam-gzip vs. Kongregate-LZF split — search for an LZF implementation too.
- The pet class: find the type whose serialized fields are
  `a,g,h,j,k,l,m,o,p,q,r,v,w,E,F,G,H,…` — its field **declaration order** maps
  our unknown pet fields positionally.

## Validation protocol (do this before trusting any new mapping)

Pick ~10 already-known mappings spanning structs and confirm the C# agrees
before believing the unknowns. High-confidence anchors from `FINDINGS.md`:

Root: `c`=save unix time, `a`=total divinity, `p.j`=available GP,
`p.i`=building speed %, `p.h`=creating speed %, `p.r/s/t/u`=stat allocation
phys/mystic/battle/creating, `p.002`=crystal power, `p.C`=statistics multi (2^50).

Pet (`X.b[i]`): `a`=name, `g`=normal level, `h`=current exp, `k`=type id,
`l`=unlocked, `E`=base growth (pre Magic-Egg ×1.3), `F`=partner type id
(999=none), `v`=team slot, `o/p/q/r`=training-clone stat snapshot, `H`=village
working-exp ms. Pet `w`: `a`=element(0–4), `b`=dungeon level, `c`=dungeon exp,
`d.a/b/c`=class id/level/exp, `e/f/g`=weapon/armor/accessory instance id.

If these line up, the rest of the method is trustworthy.

## Priority gaps to close (the payoff)

From `FINDINGS.md` "Open questions / next steps":

1. **Unknown pet fields** `d,e,f,n,s,t,u,x,y,z,A–D,H` — top prize. Suspected
   evolution/token state (export "Other": GnomeFinal/SylphFinal, fight counts,
   day counters). Sample hints: pet `t`: Vampire=1, Dog=4, Penguin=7; `y`:
   Gnome=14, Salamander=19, Sylph=24. The C# field names/types should name these
   outright.
2. **Challenge Points / Overflow Points** — confirm whether stored or computed
   (the "available" challenge counter is computed from a regen timer + used
   counter; no field went 3→2). Read it off the code rather than guessing.
3. **Equipment type ids** still ambiguous: `{23,26,30,52,56}` ↔ {Iron Pot,
   Flood Spear, Leeching Sword, Tree Axe, Hurricane Bow}. Look for the
   id→name/type table or enum in code.
4. **Material ids** `{160,164,167,168}` ↔ {Not Nothing, Absolutely Nothing,
   Food Journal One/Two} — same: find the table/enum.
5. **Misc root unknowns**: `X.v` (10,062), `X.T` (23-entry crafting roster),
   `X.028` (737 ids), the 2 leading "junk" chars on the Steam container
   (constant vs. version tag?).

## Deliverables

- Update `FINDINGS.md`: fill the key→meaning tables, mark each newly-confirmed
  field with evidence = "verified against `Assembly-CSharp` serialization logic"
  (cite the class/method name, **not** pasted source).
- Promote newly-identified fields from the raw `Node` tree into the typed
  `SaveFile`/`SavePet` in `crates/save-parser/src/model.rs`, with tests in
  `crates/save-parser/tests/real_save.rs` cross-checking against the existing
  reference saves where possible.
- Resolve the id-table ambiguities in `crates/save-parser/src/items.rs` /
  `labels.rs`.
- Run `cargo test --workspace` and `cargo clippy --workspace --all-targets`
  (no new warnings; 3 pre-existing planner warnings are expected — see
  `CLAUDE.md`).
- Follow the normal workflow in `CLAUDE.md` (branch, fresh-eyes review for
  substantive logic, PR as the review checkpoint).

## Don'ts

- Don't modify game files.
- Don't commit anything under `_cs_decomp/` or paste large verbatim decompiled
  source into committed files.
- Don't commit raw/un-redacted saves (pre-commit hook + `save-dump --check`).
