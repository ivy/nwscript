---
title: "`rakata save set`: a save with changed globals or party, byte-identical elsewhere"
kind: task
priority: p2
parent: m4-saves
blocked_by: [rakata-save]
assignee_kind: agent
labels: [rakata, save, cli]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "Saves" (write a save whose globals or party differ from a read one, byte-identical elsewhere, for fixtures). The point is modshop's `games/kotor/docs/ideas/device-fixtures.md`: a late scene reached by loading a save rather than playing to it, and a symptom reproduced from a known state. The save model and reader are `rakata-save`; this node adds the write and the command. The gate that follows loads the result on the handheld, so this node also writes the instructions it needs.

## Deliverables

- `rakata save set SRC --out DST NAME=VALUE... [--party NPC,...]`: copies the save directory, rewrites `GLOBALVARS.res` (and `PARTYTABLE.res` when `--party` is given) with the named changes through `rakata-gff`, and copies every other member and file byte for byte; refuses a global name not declared in the install's `globalcat.2da` unless `--declare`; prints a report of the fields changed; `--format json`.
- `docs/saves.md`: how a save is laid out, how to make a fixture, and how to put it on a device: a new `<NNNNNN> - Game<N>` directory under the game's `Saves/` with a number above the newest, written with the role names "the handheld" and "the build host" and no device identity.
- A fixture save prepared from the Dantooine grove save with one global changed that is visible in play (the Juhani mod shows Carth's epilogue question "About what happened on Dantooine..." in his menus once `OAJ_DONE` is set), placed in the scratch directory for the gate, with the path and the exact check in the node's result.

## Acceptance criteria

- [ ] After `set`, `rakata save show` on DST shows the new value; `cmp` of every file and ERF member other than `GLOBALVARS.res` between SRC and DST reports no difference (test over a hand-made save fixture, and over the host's save when present).
- [ ] An undeclared global fails with a code naming it; `--declare` writes it.
- [ ] `--party` rewrites only `PARTYTABLE.res`.
- [ ] `docs/saves.md` exists and contains no hostname, user name or home path.
- [ ] `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Notes

- Boolean globals are packed by bit, most significant first, in `ValBoolean`; setting one must leave the others in the word unchanged.
- `savenfo.res` names the save in the game's load menu; leave it, or the owner cannot tell the fixture from the original. Say in the result which number the fixture should take on the device.
