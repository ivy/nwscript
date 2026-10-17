# Testing scripts

Tests are Lua files named `*_test.lua`, in the style of busted: `describe`, `it`, `assert`. `nws test` finds and runs them; `nws test --help` covers the command, and the nwscript repository's `docs/testing.md` is the full reference for worlds, objects, assertions and snapshots. Read it before writing more than a few lines.

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

`nws.script(NAME)` loads `NAME.ncs` from the test file's directory or `-I`, so compile first. `script:run(world)` runs an action script, `script:conditional(world)` a dialog conditional. A run's `trace` is every engine call.

- **Supply what the model lacks.** `w.engine.ROUTINE = function(...) return VALUE end` answers a routine the world model does not, and makes the test say what it assumed. `assert.modelled(run)` fails if any call went unanswered.
- **Test the change, not the game.** Assert the calls and results the change is about; a test that restates every call breaks on any recompile.
- **Snapshots** (`run --record FILE.snapshot.toml`) pin a script's whole trace for a fixed world; `nws test` replays them. Use one to show a refactor changed nothing.

`nws lua SCRIPT.lua` runs any Lua against the same library, for exploring outside a test.
