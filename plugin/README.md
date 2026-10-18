# nwscript plugin

A [Claude Code plugin](https://code.claude.com/docs/en/plugins/overview) for modding Knights of the Old Republic with this repository's tools. With `nws`: going from a described change to a checked, tested and compiled script; reading a diagnostic and acting on it; learning from the game's own scripts by decompiling and running them; and writing Lua tests. With `rakata`: finding the copy of a resource the game loads, finding what refers to it, changing dialogues, globals, tables and blueprints through the Python module, and checking the change with the lints and a diff. It teaches the workflow and defers to `--help` and `explain CODE` for the details, so it stays right as the tools change.

## Skills

| Skill | What it gives Claude |
| --- | --- |
| [`/nwscript:nwscript`](skills/nwscript/README.md) | The loop from a change to an installed script, and a reference per task: workflow, diagnostics, learning from vanilla scripts, testing |
| [`/nwscript:rakata`](skills/rakata/README.md) | The loop from a change to the game's data to a checked file in Override, and a reference per step: find, refs, author, check |

Claude loads a skill on its own when a task involves it (NWScript, `.nss` or `.ncs` files and `nws` for the first; dialogues, globals, 2DA tables, blueprints and `rakata` for the second); typing `/nwscript:nwscript` or `/nwscript:rakata` loads one explicitly.

## Requirements

- `nws` on the `PATH`. Until there is a release, build it from a checkout: `cargo build --release -p nws`, then put `target/release/nws` on the `PATH`.
- For the data skill, `rakata` on the `PATH` (`cargo build --release -p rakata-cli`, then `target/release/rakata`) and the Python module `rakata` in the interpreter the agent uses (`mise run test:py` builds it into `target/rakata-py/venv`; `maturin build -m crates/rakata-py/Cargo.toml` makes a wheel).
- A copy of the game for its `nwscript.nss`, its includes, its scripts and its data. The plugin contains no game data.

## Install

The repository root is a plugin marketplace named `nwscript` ([`.claude-plugin/marketplace.json`](../.claude-plugin/marketplace.json)). From a clone, in a Claude Code session:

```text
/plugin marketplace add /path/to/nwscript
/plugin install nwscript@nwscript
```

A marketplace added from a local path loads the plugin from the clone in place, so a `git pull` takes effect at the next session start or after `/reload-plugins`. `claude plugin install nwscript@nwscript --scope project` enables it for one project instead of every session.

## Check that it loaded

Run `/plugin` and look for `nwscript` in the **Installed** tab, or type `/nwscript:` and see `nwscript` offered. If it is missing, the **Errors** tab says why.

## Keeping it right

`crates/nws/tests/plugin.rs` checks that every `nws` command the plugin names exists and that every flag it uses appears in that command's `--help`; `crates/rakata-cli/tests/plugin.rs` does the same for every `rakata` command and subcommand, and checks the Python names the data skill cites against the module's stubs. A change that breaks a skill fails the tests.
