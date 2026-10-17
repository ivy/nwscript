# Testing scripts

`nws test` runs tests written in Lua against compiled NWScript. A test builds a small world, runs a script in it under the interpreter, and checks what the script returned and which engine routines it called with what. No game runs; a test takes milliseconds.

```bash
nws --game k1 -I "$NWSCRIPT_CORPUS" -I "$NWSCRIPT_CORPUS/ncs" test docs/examples
```

`nws test` finds every `*_test.lua` under the paths it is given (the current directory by default). A script named in a test is found as `NAME.ncs` in the test file's directory, then in each `-I` directory; the game's `nwscript.nss` comes from `--nwscript` or an `-I` directory. It exits 0 when every test passes and 3 when any fails, printing each failure's message and the trace of the last script it ran. `--filter TEXT` runs only tests whose name contains the text.

The examples in [`examples/`](examples/) test ten of KotOR 1's scripts and are the quickest way to learn the API. Editors with lua-language-server complete the API from [`../lua/nws.lua`](../lua/nws.lua).

## A test

```lua
describe("k_pman_misc59: the PC has less than 500 credits", function()
  local script = nws.script("k_pman_misc59")

  it("asks the engine for the PC's gold, which a test supplies", function()
    local w = nws.world()
    local pc = w:create("creature", "player", { is_pc = true })
    w.party = { pc }
    w.engine.GetGold = function(who) return who == pc and 120 or 0 end
    local run = script:conditional(w)
    assert.is_true(run.result)
    assert.modelled(run)
  end)
end)
```

`describe` groups tests and may nest; `it` declares one; `before_each` runs a function before each test of the `describe` it is in. A test fails when it raises an error, which every `assert` does on a mismatch.

## Worlds

`nws.world({ seed = N })` makes an empty world: a module object, no area, nobody in the party, and a random generator seeded with N (0 by default). Scripts run against it change it, and a test reads it back.

The world answers the engine routines `nws fn NAME` says it models (the repository's `docs/engine-model.md` lists them all), about nine in ten of the calls KotOR 1's scripts make. Every other routine returns its type's default (0, 0.0, `""`, `OBJECT_INVALID`, a null structure) and the run records it as unmodelled. A test can answer any routine itself: `w.engine.NAME = function(args...) return value end` replaces `NAME` in that world, and its replies count as modelled. Arguments arrive as Lua values (below) in the routine's parameter order; the return is converted to the routine's return type.

| Field | What |
| --- | --- |
| `w.globals.NAME` | a global variable: an integer is a global number, a boolean a global boolean, a string a global string; `nil` removes it |
| `w.engine.NAME` | a Lua function answering the engine routine `NAME` |
| `w.self` | `OBJECT_SELF` for the script |
| `w.speaker` | `GetPCSpeaker()` |
| `w.entering`, `w.exiting`, `w.opened_by`, `w.used_by`, `w.clicking`, `w.perceived` | the objects the matching `Get…` routines return |
| `w.user_event` | `GetUserDefinedEventNumber()` |
| `w.party` | the party as a list of objects, the leader first (`GetFirstPC`, `GetPartyMemberByIndex`) |
| `w.npc_party` | the `NPC_*` numbers in the party (`IsNPCPartyMember`) |
| `w.module` | the module object |

| Method | What |
| --- | --- |
| `w:create(kind, tag, fields)` | a new object; `kind` is `"creature"`, `"item"`, `"trigger"`, `"door"`, `"waypoint"`, `"placeable"`, `"store"`, `"area"` or an `OBJECT_TYPE_*` number; `fields` sets any object field |
| `w:object(tag)` | the first object with the tag, or `nil` |
| `w:enemies(a, b)` | makes two `STANDARD_FACTION_*` numbers hostile (`GetIsEnemy`) |

## Objects

Objects compare equal when they are the same object, and every field below can be read and set.

| Field | What |
| --- | --- |
| `id` | the object's id (read only) |
| `tag`, `name`, `resref` | strings |
| `hp`, `max_hp`, `race`, `gender`, `faction`, `ai_style`, `base_item` | integers |
| `dead`, `plot`, `in_combat`, `is_pc` | booleans |
| `area` | the object's area, an object; distance and perception count only within one area |
| `position` | `{ x = …, y = …, z = … }` |
| `facing` | degrees |
| `destroyed` | true after `DestroyObject` (read only) |
| `inventory` | a list of item objects (read only; use `give`) |

| Method | What |
| --- | --- |
| `o:give(tag)` | a new item with that tag in the inventory |
| `o:equip(slot, tag)` | a new item in an `INVENTORY_SLOT_*` |
| `o:learn(spell)`, `o:feat(feat)`, `o:affect(spell)` | the creature knows a spell, has a feat, or is under a spell's effect |
| `o:talent(category, type, id)` | a talent for `GetCreatureTalentRandom` and `GetCreatureTalentBest` |
| `o:set_local(index, value)` | a local boolean or number at the index |
| `o:valid()` | whether it exists and has not been destroyed |

## Scripts and runs

`nws.script(name, { game = "k1" })` loads a compiled script; `game`, when given, must be the profile `nws` runs with. `script:run(w)` runs `main`, then the deferred actions the script created (`DelayCommand`, `AssignCommand`, `ActionDoCommand`) in order, each as the object it was assigned to. `script:conditional(w)` runs a `StartingConditional` and turns its result into a boolean. Without a world each makes a fresh one.

| Field of a run | What |
| --- | --- |
| `run.result` | the entry's return value: a boolean for `conditional`, otherwise the value or `nil` |
| `run.trace` | every engine call: `{ name, routine, args, ret, modelled, deferred }` |
| `run.unmodelled` | the routines that answered with defaults, sorted |
| `run.fault` | why the run stopped early, or `nil` |
| `run:calls(name)` | the calls to one routine |
| `run:text()` | the trace as text, as a failure prints it |

Values cross into Lua as integers, numbers, strings (bytes, as the game stores them), objects, engine structures (userdata compared by kind and handle), vectors as `{ x, y, z }`, and deferred actions as the text `"<deferred N>"`. `OBJECT_SELF` in a trace is shown as the object it stood for.

## Assertions

| Assertion | Passes when |
| --- | --- |
| `assert(v, message)` | `v` is truthy, as in plain Lua |
| `assert.is_true(v)`, `assert.is_false(v)`, `assert.is_nil(v)` | `v` is exactly that |
| `assert.equal(want, got, message)` | the two are equal |
| `assert.called(run, name, args...)` | some call to `name` has these leading arguments; `nws.any` matches any one |
| `assert.not_called(run, name, args...)` | no call to `name` has these leading arguments |
| `assert.called_before(run, first, second)` | the first call to `first` precedes the first call to `second` |
| `assert.modelled(run)` | no reply in the run came from defaults |

Numbers match by value, objects by identity, strings by bytes.

## Snapshots

A snapshot is a recorded run kept as data: the script's name, the engine and world it ran in, its result, and every engine call. `nws test` replays each `*.snapshot.toml` it finds and fails at the first call that differs, which makes snapshots the cheapest guard for a script that should not change behaviour.

```bash
nws --game k1 -I corpus run --world world.toml --record k_pman_comp35.snapshot.toml k_pman_comp35.ncs
nws --game k1 -I corpus test .
```

A failing replay names the call:

```text
call 1 differs:
  recorded: GetGlobalBoolean("MAN_PTRAP_DONE") -> 1
  now:      GetGlobalBoolean("MAN_PTRAP_GONE") -> 0
(the script's bytecode differs from the recording's)
```

The world file, given to `nws run --world` and kept as a snapshot's `[world]`, names objects by tag:

```toml
seed = 7
npc_party = [1]
party = ["player"]
context = { speaker = "player" }
globals_number = { DAN_JON_STATE = 1 }
globals_boolean = { MAN_PTRAP_DONE = true }

[[objects]]
kind = 1            # OBJECT_TYPE_CREATURE
tag = "player"
is_pc = true
inventory = ["ptar_sithpapers"]
```

Objects take `kind`, `tag`, `name`, `hp`, `max_hp`, `is_pc`, `dead`, `faction`, `position` and `inventory` (item tags); `context` takes `self`, `speaker`, `entering`, `exiting`, `opened_by`, `used_by`, `clicking` and `perceived`; `user_event` sets `GetUserDefinedEventNumber`. Without `--world` a snapshot records the recording engine, which answers every routine with its default.

## Constants and scripting

`nws.const.NAME` is any constant `nwscript.nss` declares, such as `nws.const.NPC_HK_47`. `nws lua FILE ARGS...` runs a Lua file with the same API and `nws.args`, for work over many scripts:

```lua
for _, name in ipairs(nws.args) do
  local run = nws.script(name):conditional(nws.world())
  print(name, run.result, #run.trace .. " calls")
end
```
