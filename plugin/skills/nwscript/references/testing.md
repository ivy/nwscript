# Testing scripts

Tests are Lua files named `*_test.lua`, with `describe` and `it` as in busted. `nws test` finds and runs them; `nws test --help` covers the command, and `nws test --reference` prints the full reference: world fields, object fields and methods, run fields, every assertion, snapshots. Read it before writing more than a few lines; the API is small and not busted's (`assert.equal`, not `assert.are.equal`).

```lua
describe("k_my_cond: true once the player has the keycard", function()
  local script = nws.script("k_my_cond")

  it("is false without it", function()
    assert.is_false(script:conditional(nws.world()).result)
  end)

  it("is true with it", function()
    local w = nws.world()
    local pc = w:create("creature", "player", { is_pc = true })
    w.party = { pc }
    pc:give("keycard")
    local run = script:conditional(w)
    assert.is_true(run.result)
    assert.called(run, "GetItemPossessedBy")
  end)
end)
```

```bash
nws --game k1 -I DIR test tests/
```

`nws.script(NAME)` loads `NAME.ncs` from the test file's directory, then the `-I` directories, so compile first and keep the `.ncs` beside the test or pass its directory with `-I`. The same holds under `nws lua`, which searches the Lua file's directory, not the current one. `script:run(world)` runs an action script, `script:conditional(world)` a dialog conditional. A run's `trace` is every engine call.

- **Supply what the model lacks.** `nws -I DIR fn NAME` ends with whether the world model answers the routine. One it does not returns its type's default (gold is 0, for instance, and objects have no gold field); `w.engine.NAME = function(...) return VALUE end` answers it, receiving the routine's arguments in parameter order, objects as values that compare equal to the world's objects. `assert.modelled(run)` fails if any call went unanswered, which catches a routine you did not know to supply.
- **Test the change, not the game.** Assert the calls and results the change is about; a test that restates every call breaks on any recompile.
- **Snapshots** (`run --record FILE.snapshot.toml`) pin a script's whole trace for a fixed world; `nws test` replays them. Use one to show a refactor changed nothing.

`nws lua SCRIPT.lua` runs any Lua against the same library, for exploring outside a test.
