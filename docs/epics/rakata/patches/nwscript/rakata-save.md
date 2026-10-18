---
title: "rakata-save and `rakata save show`: a save's globals, party and creatures by name"
kind: task
priority: p1
parent: m4-saves
blocked_by: [rakata-archive, rakata-gff, rakata-game]
assignee_kind: agent
labels: [rakata, save, cli]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, section "Saves". A KotOR 1 save is a directory (`<game>/Saves/<NNNNNN> - Game<N>/`) with `SAVEGAME.sav` (an ERF holding `GLOBALVARS.res`, `PARTYTABLE.res`, one `AVAILNPC<n>.utc` per party member with their `ActionList`, and one nested `<module>.sav` ERF per visited module, each holding that module's `.git` with its `Creature List` as the world has them), plus `savenfo.res` and a screenshot. The layout is on the KotOR wiki's save-game pages. modshop's `games/kotor/docs/ideas/device-state.md` records the three findings a save gave in minutes after hours of guessing, each read by a script written for the occasion: two creatures with `SWVarTable.BitArray`'s second word holding `0x40000000`, local boolean 62, `SW_FLAG_AI_OFF`; the world's tags `carth` and `mission` where the blueprints say `Carth` and `Mission`; Mission's `ActionList` holding a standing action (61) with the banter's `ActionStartConversation` queued behind it. Two saves are on this host under `~/src/github.com/ivy/modshop/games/kotor/data/saves/`; the Dantooine grove save is the one whose visited modules include `danm14ac`. The `SW_FLAG_*` names are constants in the game's include scripts in the corpus (`$NWSCRIPT_CORPUS/nss/`).

## Deliverables

- `crates/rakata-save/`: `Save::open(dir)`; `globals()` by name and type (booleans by bit, most significant first, numbers, strings, locations); `party()`; `creatures(module)` from the module's GIT and the `AVAILNPC` members: tag as saved, `Commandable`, faction, animation, effects (type, subtype, creator, named by tag when the creator is an object in the save), local booleans and numbers by index and by `SW_FLAG_*` name, the action queue with parameters; the module's event queue. Every GFF inside is read through `rakata-gff`, so write-back is lossless.
- `rakata save show DIR [--module M] [--creature TAG] [--globals PATTERN] [--format json]`.
- Numbers whose names live only in the engine (effect types, animations) printed as numbers with "(table unknown)".

## Acceptance criteria

- [ ] Over the Dantooine save, `rakata save show DIR --module danm14ac --creature carth` and `--creature mission` each show `SW_FLAG_AI_OFF` set, tags `carth` and `mission` as saved, and Mission's queue with action 61 followed by `ActionStartConversation`.
- [ ] `--globals OAJ_` lists the Juhani mod's booleans with values.
- [ ] Every GFF member of both saves on this host round-trips byte-identical through `rakata-gff` (test, skipped without the saves).
- [ ] A creature in a module and a party member in `AVAILNPC` are both found by `--creature TAG`, case-insensitively.
- [ ] `--format json`; `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Notes

- Saves hold the player's state and are theirs; nothing from them is committed, and the test paths are read from `$RAKATA_SAVES` or skipped.
- The nested `.sav` members are ERFs inside an ERF; `rakata-archive` handles both, and the module name is the member's stem.
- **Clean room** (`docs/proposals/rakata.md`, Clean room): PyKotor's `extract/savedata` may not be opened; the wiki's save pages may.
