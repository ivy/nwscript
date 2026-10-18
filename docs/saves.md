# Saves as fixtures

A save is the game's state at one moment. Loading one reaches a late scene without playing to it, and a save whose globals differ from a real one by a single value reproduces a symptom from a known state. This document is how to read a save, how to make such a fixture with `rakata save set`, and how to put it on the device the game runs on. The byte layout, with the evidence for each fact, is [`formats/save.md`](formats/save.md).

The machines are named by role: the build host, where the build and `rakata` run, and the handheld, where the game is played.

## How a save is laid out

A KotOR 1 save is a directory under the game's saves folder (`saves/` beside `chitin.key`), named `NNNNNN - GameN`, a six-digit number, a space, a hyphen, a space, and `Game` followed by a number. In every save measured the second number is the first less one: `000023 - Game22`, `000025 - Game24`. The directory holds five files:

| File | What it holds |
| --- | --- |
| `SAVEGAME.sav` | An archive of the party members' creatures (`AVAILNPC<n>.utc`, one per NPC met), the inventory, the factions, and one nested archive per visited module with that module's state. |
| `GLOBALVARS.res` | The campaign's global variables: the booleans, numbers, strings and locations the scripts read with `GetGlobalBoolean` and its kin. |
| `PARTYTABLE.res` | Who travels with the player, the gold, the journal, the message logs. |
| `savenfo.res` | The load menu's entry: the area's name, the module, the time played, the portraits. |
| `Screen.tga` | The load menu's screenshot. |

`GLOBALVARS.res` names each global in one catalogue per type, `CatBoolean`, `CatNumber`, `CatString` and `CatLocation`, and holds the values beside them in the same order. The booleans are packed eight to a byte, the first in the most significant bit; a number is one signed byte. The names are the save's own: they are the install's `globalcat.2da` at the time the save was made, and the game's scripts read only the globals that table declares.

To read a save:

```bash
rakata save show "saves/000025 - Game24"                    # the party, the area, the modules
rakata save show "saves/000025 - Game24" --globals OAJ_     # the globals whose names hold OAJ_
rakata save show "saves/000025 - Game24" --creature carth   # a creature's flags, effects and actions
```

## How to make a fixture

`rakata save set` writes a new save directory from an existing one. It rewrites `GLOBALVARS.res` with the globals named, and `PARTYTABLE.res` when `--party` is given, through `rakata-gff`, and copies every other file byte for byte. The original is never written to.

```bash
rakata --game BUILD save set "saves/000025 - Game24" --out "000026 - Game25" OAJ_DONE=false
```

`--game` names the install the save is for, the folder holding `chitin.key`: its `globalcat.2da` decides which globals exist. Each change is `NAME=VALUE`, or `NAME=TYPE:VALUE` where the save and the install both lack the name. A boolean takes `true`, `false`, `1` or `0`, and setting one leaves the other seven in its byte as they were; a number takes -128 to 127; a string takes any text; a location takes six numbers, `x,y,z,fx,fy,fz`. Names compare without regard to case.

The report names each global with its old and new value, and each field of the file that changed, down to the byte for the packed values:

```text
global   OAJ_DONE  boolean  true -> false
field    GLOBALVARS.res ValBoolean byte 14: 0x22 -> 0x02
wrote    000026 - Game25: rewrote GLOBALVARS.res; copied PARTYTABLE.res, SAVEGAME.sav, Screen.tga, savenfo.res
```

`--format json` gives the same as one document. A name the install's `globalcat.2da` does not declare is refused with `RE1205`, which names it; a name the install declares and the save lacks (a mod installed after the save was made) is added to the save's catalogue with the install's type.

### `--declare`

`--declare` writes a global the install does not declare. When the save lacks the name, it is appended to the end of the save's catalogue of its type, with its value written at the matching place (a boolean's bit, a number's byte, a location's 24 bytes, a string's list element); when the save holds it, its value is set. Either way the warning `RW1202` names it, because the game's scripts still read only the globals the install's `globalcat.2da` declares: for a script to see the global, the install has to declare it too, as the mod that uses it does.

### `--party`

`--party carth,mission` makes the party exactly the members listed, in order, by NPC number or by the tag of their `AVAILNPC<n>.utc`; `--party ''` leaves the player alone. Only `PARTYTABLE.res` changes: its member list and member count. A member already in the party keeps its entry as it was. A member the save holds no `AVAILNPC` for is refused (`RE1207`), and so is a party of more than two (`RE1208`). With `--party` and no `NAME=VALUE`, `GLOBALVARS.res` is copied unchanged.

### What stays the same

`savenfo.res` is copied unchanged, so the load menu shows the fixture with the original's area, time played and screenshot. Which entry is the fixture is known by its directory, and confirmed in play by the change it carries.

## How to put it on the handheld

1. On the handheld, list the game's saves folder (`saves/` in the game folder, beside `chitin.key`) and find the newest save, the highest number.
2. Name the fixture with the next number above it: `NNNNNN` one more than the newest, and `GameN` with N one less than that, as the game names its own (after `000025 - Game24`, `000026 - Game25`). A number no save uses overwrites nothing on the handheld.
3. On the build host, write the fixture under that name with `rakata save set --out`, then copy the whole directory into the handheld's saves folder while the game is not running, keeping the directory name.
4. Start the game, open the load menu, and load the fixture. Remove the directory from the saves folder afterwards to leave the handheld's saves as they were.
