# nwscript plugin

A [Claude Code plugin](https://code.claude.com/docs/en/plugins/overview) for writing and reading NWScript for Knights of the Old Republic with `nws`, this repository's toolchain: going from a described change to a checked, tested and compiled script; reading a diagnostic and acting on it; learning from the game's own scripts by decompiling and running them; and writing Lua tests. It teaches the workflow and defers to `nws <command> --help` and `nws explain CODE` for the details, so it stays right as the tool changes.

## Skills

| Skill | What it gives Claude |
| --- | --- |
| [`/nwscript:nwscript`](skills/nwscript/README.md) | The loop from a change to an installed script, and a reference per task: workflow, diagnostics, learning from vanilla scripts, testing |

Claude loads the skill on its own when a task involves NWScript, `.nss` or `.ncs` files, or `nws`; typing `/nwscript:nwscript` loads it explicitly.

## Requirements

- `nws` on the `PATH`. Until there is a release, build it from a checkout: `cargo build --release -p nws`, then put `target/release/nws` on the `PATH`.
- A copy of the game for its `nwscript.nss`, its includes and its scripts. The plugin contains no game data.

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

`crates/nws/tests/plugin.rs` checks that every `nws` command the skill names exists and that every flag it uses appears in that command's `--help`, so a CLI change that breaks the skill fails the tests.
