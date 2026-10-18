# KotOR 1 saved games

A KotOR 1 save is a directory under the game's `Saves/` folder, named `NNNNNN - GameN` (`000023 - Game22`). Everything in it is an ERF or a GFF V3.2 file except the screenshot, and every GFF is the ordinary format of [`gff.md`](gff.md): the save adds no container of its own. This document is what `rakata-save` reads, measured on two saves of one modded KotOR 1 build; each fact below names the evidence. Facts the two saves cannot settle are listed at the end.

## The directory

| File | Content |
| --- | --- |
| `SAVEGAME.sav` | An ERF: the party members, the inventory, the factions, and one nested ERF per visited module. |
| `GLOBALVARS.res` | A GFF of type `GVT `: the campaign's global variables. |
| `PARTYTABLE.res` | A GFF of type `PT  `: the party, gold, the journal, the message logs. |
| `savenfo.res` | A GFF of type `NFO `: what the load screen shows. |
| `Screen.tga` | The screenshot, 196,626 bytes in both saves. |

`GLOBALVARS.res` and `PARTYTABLE.res` are loose files beside `SAVEGAME.sav` in both saves, not members of it. A reader looks beside the archive first and inside it second; `rakata-save` does both, and names compare without regard to case.

## `SAVEGAME.sav`

The archive is ERF V1.0 as [`erf.md`](erf.md) describes, but its signature is `MOD V1.0`, not `SAV V1.0`, in both saves; the build year and day are 126 and 289 (2026, the 290th day, the day the saves were made) and the description strref is 0. Its members, in key-list order:

| Member | Type | Content |
| --- | --- | --- |
| `AVAILNPC<n>` | `utc` (2027) | One per NPC the player has met, `n` the NPC number (0, 1, 2, 6, 7 and 8 in both saves): the creature as it travels with the player, with the same fields as a world creature (below) but no `ObjectId`. The root struct has the usual id `0xFFFFFFFF`. |
| `<module>` | `sav` (2057) | One per visited module, named by the module's stem (`danm14ac`): a nested ERF, also signed `MOD V1.0`, holding `<area>.git` (2023), `<area>.are` (2012) and `Module.ifo` (2014), in that order. |
| `INVENTORY` | `res` (0) | A GFF of type `INV `. |
| `REPUTE` | `fac` (2038) | A GFF of type `FAC `: `FactionList`, whose element `i` is faction `i` (`FactionName` `Player`, `Hostile_1`, `Friendly_1`, `Hostile_2`, `Friendly_2`, `Neutral`, ...), and `RepList`. |

The module name is the nested member's stem; the area inside it has its own stem (`m14ac` in `danm14ac`).

## `savenfo.res`

`AREANAME` (the area's name, `Dantooine - Grove`), `LASTMODULE` (the stem of the module the player was in, one of the nested members), `TIMEPLAYED` (seconds), `CHEATUSED`, `SAVEGAMENAME` (empty for a quick save), `GAMEPLAYHINT`, `STORYHINT`, `LIVE1` to `LIVE6`, `LIVECONTENT`, and `PORTRAIT0` to `PORTRAIT2`, the resrefs of the party's portraits.

## Globals: `GLOBALVARS.res`

Four catalogue lists name the variables, each element a struct with one field `Name`: `CatBoolean`, `CatNumber`, `CatLocation`, `CatString` (823, 370, 5 and 2 entries in both saves). The values are parallel to them:

- `ValBoolean`, `VOID`: boolean `i` is bit `7 - i % 8` of byte `i / 8`, most significant bit first; 823 booleans in 103 bytes. Evidence: in `000023` the Juhani mod's `OAJ_DOWN_2` and `OAJ_DOWN_6` read set, the NPC numbers of Carth and Mission, the two companions down in that save, and `OAJ_DONE` clear; in `000025`, after the fight, `OAJ_DONE`, `OAJ_BANTER` and `OAJ_SPECIAL` read set and every `OAJ_DOWN_*` clear. Read least significant bit first, every `OAJ_*` boolean in both saves is clear.
- `ValNumber`, `VOID`: number `i` is byte `i`, signed; 370 bytes for 370 numbers. Evidence: `K_PARTY_STORE1` and `K_PARTY_STORE2` hold `0xFE`, and `k_inc_utility.nss` sets `K_PARTY_STORE1` to -2; `K_TURRET_SKYBOX` holds `0xFF`, and `k_inc_stunt.nss` sets it to -1.
- `ValLocation`, `VOID`: location `i` is 24 bytes at `24 * i`, six little-endian floats, the position x, y, z and the facing x, y, z. The field is 2,400 bytes in both saves for five locations, the bytes past the fifth all zero. Evidence: each facing read this way is a unit vector in the plane (`-0.556, -0.831, 0`), and the one unset location reads position zero facing `1, 0, 0`.
- `ValString`, a list parallel to `CatString`, each element a struct with one field `String`.

## Creatures

A module's creatures are the elements of its GIT's `Creature List` (struct id 4); the player character is element 0 of its IFO's `Mod_PlayerList`, with `ObjectId` `0x7FFFFFFF` and an empty `Tag`. A party member is in `AVAILNPC<n>.utc` whether or not it is also in the module: in `000023` Carth and Mission are in both `danm14ac`'s `Creature List` and their `AVAILNPC`, with the party table empty; in `000025` they are only in their `AVAILNPC`, with the party table naming NPCs 6 and 2. The fields `rakata save show` reads:

| Field | Type | Content |
| --- | --- | --- |
| `ObjectId` | `DWORD` | The object's id within the module's state. Absent from `AVAILNPC`. |
| `Tag` | `CExoString` | The tag as the world has it: `carth` and `mission`, lower case, in both saves' records of both. |
| `FirstName` | `CExoLocString` | The name, with an embedded substring. |
| `Commandable` | `BYTE` | 1 for every creature in both saves. |
| `FactionID` | `WORD` | An index into `REPUTE.fac`'s `FactionList` (5, `Neutral`, for Carth and Mission in both). |
| `Animation` | `INT` | The engine's animation number (10000, 10139). |
| `EffectList` | list | The effects (below). |
| `SWVarTable` | struct | The local booleans and numbers (below). |
| `ActionList` | list | The action queue (below). |

An object id of `0x7F000000` stands for no object: `CombatInfo.LeftEquip` holds it on a creature with nothing in the left hand, and every unused `ObjectList` slot of an effect holds it.

### Local variables: `SWVarTable`

`BitArray` is a list of three structs, each with one `DWORD` field `Variable`: 96 local booleans, boolean `i` being bit `i % 32` (value `1 << (i % 32)`) of word `i / 32`, least significant bit first, unlike the globals. Evidence: the game's `k_inc_generic.nss` defines `int SW_FLAG_AI_OFF = 62;`, and Carth and Mission in `000023`, whose AI the mod had turned off, carry `0x40000000` (`1 << 30`) in word 1 and nothing else, 32 + 30 = 62. `ByteArray` is a list of eight structs, each with one `BYTE` field `Variable`: local numbers 0 to 7. The scripts name the indices in their include files: `SW_FLAG_*` for booleans (`k_inc_generic.nss`, `k_inc_gensupport.nss`, `k_inc_walkways.nss`), `SW_NUMBER_*` (3 `SW_NUMBER_COMBO_ROUTINE`, 4 `SW_NUMBER_COMBO_INDEX`, 5 `SW_NUMBER_LAST_COMBO`, 6 `SW_NUMBER_ROUND_COUNTER`, 7 `SW_NUMBER_COMBAT_ZONE`) and `WALKWAYS_*` (0 to 2) for numbers. `rakata-save` reads those names from the user's own corpus at run time and carries none.

### Effects: `EffectList`

Each element (struct id 2): `Id` (`DWORD64`, a serial; `Module.ifo`'s `Mod_Effect_NxtId` is the next), `Type` and `SubType` (`WORD`), `Duration` (`FLOAT`), `SkipOnLoad`, `ExpireDay`, `ExpireTime`, `CreatorId` (`DWORD`, an object id: the creature itself, or the item that grants the effect, such as `g_w_blstrpstl010`), `SpellId` (`DWORD`, `0xFFFFFFFF` for none), `IsExposed`, `NumIntegers` (`INT`, 8), and four lists of parameters: `IntList` (struct id 3, `Value` `INT`), `FloatList` (id 4, `FLOAT`), `StringList` (id 5, `CExoString`), `ObjectList` (id 6, `DWORD`). The effect type numbers are the engine's own; no file of the game names them.

### Action queues: `ActionList`

Each element: `ActionId` (`DWORD`), `GroupActionId` (`WORD`; the actions one script call queues share it), `NumParams` (`WORD`), and, when there are parameters, a list spelled `Paramaters` whose elements (struct id 1) hold `Type` (`DWORD`) and `Value`. Across every action list of both saves, `Type` decides `Value`'s field type: 1 `INT`, 2 `FLOAT`, 3 `DWORD` (an object id), 4 `CExoString`, 5 a struct, the stored script situation of a `DelayCommand` or `ActionDoCommand` body (struct id 2: `CodeSize`, `Code`, `Name` and the rest of a script situation, as in the event queue).

The action ids are the engine's own. One is established by a save: **24 is `ActionStartConversation`**. The Juhani mod's `k_oaj_up.nss` queues `AssignCommand(oMission, ActionStartConversation(GetFirstPC(), "oaj_banter"))`, and `000025`'s `AVAILNPC6.utc` (Mission) holds action 24 with the parameters object `0x7FFFFFFF` (the player character), string `oaj_banter`, int 0, int 1, int 1, object `0x7F000000`, queued (group 2) behind action 61 (group 1, no parameters), which the conversation waited behind. What action 61 is, is not established. `rakata-save`'s `action_name` names 24 alone and prints every other id as a number.

## The event queue

A module's pending events are its IFO's `EventQueue` list (struct id `0xABCD`): `Day`, `Time` (`DWORD`, milliseconds), `ObjectId` and `CallerId` (`DWORD`, object ids), `EventId` (`DWORD`, the engine's own number; 1 for every event in both saves), and `EventData`, a script situation (struct id `0x7777`) with `CodeSize`, `Code`, `CRC`, `InstructionPtr`, `SecondaryPtr`, `Name` (the script, `atosht_heart`, `k_oaj_fight`), `StackSize` and `Stack`. An object id in the queue may name a party member who is not in the module's `Creature List`, and so has no tag there.

## Narrow fields stored sign-extended

The game's saves store a negative `CHAR` or `SHORT` sign-extended through the whole 32-bit data word, where BioWare's blueprints zero the bits above the value: `WillSaveThrow` -1 as `0xFFFFFFFF`, `CurrentForce` -42 as `0xFFFFFFD6`, `CurrentHitPoints` -13 as `0xFFFFFFF3`, in 71 field entries of eleven GITs and IFOs across the two saves. A writer that keeps only the value writes those words zero-extended; `rakata-gff` keeps the word as read while the value is unchanged, and with that every one of the 26 GFF members of each save (3 beside `SAVEGAME.sav`, 8 in it, 15 in its five modules) writes back byte for byte.

## What could not be established

Which archive signature the engine requires of `SAVEGAME.sav` and of the nested module archives, and whether it ever writes `SAV V1.0`; whether `GLOBALVARS.res` and `PARTYTABLE.res` are ever members of `SAVEGAME.sav` rather than beside it; whether `ValLocation` is always 2,400 bytes; the names of the engine's action ids (other than 24), effect types, event ids and animation numbers; and what action 61 is. Each needs more saves or the engine.

## Provenance

Public documents read: none for this layout. The KotOR wiki's save-game pages were unreachable from this host, and no other tool's save-game code was opened.

Measured: on 2026-10-17, from two KotOR 1 saves of the maintainer's modded build, both in the Dantooine grove (`LASTMODULE` `danm14ac`): `000023 - Game22`, made while Carth and Mission lay down after the Juhani mod's fight, and `000025 - Game24`, made after it. The archive tables and every GFF were read with a short Python script written for the occasion, and the readings checked again through `rakata save show` (`rakata save show "000023 - Game22" --module danm14ac --creature carth`, `--creature mission` over `000025`, `--globals OAJ_`). The include scripts cited are the game's own, from the corpus extracted from the same install (`nss/k_inc_generic.nss`, `nss/k_inc_utility.nss`, `nss/k_inc_stunt.nss`), and the mod's `k_oaj_up.nss` is in modshop's `games/kotor/mods/juhani-clones/`. The round trip is `RAKATA_SAVES=… cargo test -p rakata-save --test saves -- --nocapture`. No save bytes are committed.

Date: 2026-10-17.
