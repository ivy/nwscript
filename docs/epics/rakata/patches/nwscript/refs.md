---
title: "`rakata refs`: every reference to a strref, resref, tag or global in every direction, and the Canderous report"
kind: task
priority: p1
parent: m2-know
blocked_by: [index-build]
assignee_kind: agent
labels: [rakata, index, cli]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "The index and references" (the `refs` paragraph and the acceptance cases). modshop's `games/kotor/docs/ideas/inspect-cli.md` records the question asked most and the chain that answered it by five throwaway scanners: the area-transition refusal message's strref 42724 is in `k_transabort.dlg`, whose placeable is `k_trans_abort.utp`, which the script `k_trg_transfail.ncs` spawns by name. One scan reported no script referred to the placeable because a strref is big-endian behind `CONSTI` in bytecode and little-endian in a GFF field: a false negative a tool with the reference kinds built in does not make. The Calo Nord mod's research asks, for every module, every dialogue node, condition and script call that names Canderous: blueprint `p_cand`, tag `Cand`, party slot 1 (`NPC_CANDEROUS` in the includes), his conversation `k_hcan_dialog`, and the TLK strings with his name.

## Deliverables

- `rakata refs THING [--kind strref|resref|tag|global|auto] [--to|--from] --db FILE`: each hit as the resource, where the game loads it from (from the index's locations), and the site: a GFF field path, a 2DA row and column, a TLK entry, or a script's routine name and instruction offset. `--format json`. On an empty answer, the coverage summary (how many resources had no reader) is printed so the absence is qualified.
- `docs/examples/canderous.sql`: the coverage report as one query over the index: dialogue nodes with speaker or listener `Cand`, nodes whose text names Canderous (FTS), script calls passing the literal 1 to the party routines, scripts and blueprints referring to the tag or the resref, grouped by module with counts. Names and counts only; no game text in the file.
- `docs/examples/README.md` saying how to run it with `rakata index query`.

## Acceptance criteria

- [ ] `rakata refs 42724 --kind strref` names `k_transabort.dlg`; `rakata refs k_transabort --kind resref` names `k_trans_abort.utp`; `rakata refs k_trans_abort --kind resref` names `k_trg_transfail.ncs` with the `CONSTS` site; each over an index of `$RAKATA_BUILD`.
- [ ] Each of those three answers in under 100 ms on the built index (timed in the test output).
- [ ] `rakata index query --db FILE "$(cat docs/examples/canderous.sql)"` runs and its per-module counts are recorded in the node's result.
- [ ] `rakata refs OAJ_DONE --kind global` names the Juhani mod's scripts and dialogue conditions that read or set it.
- [ ] `--format json` emits the envelope; `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Notes

- A tag is referenced by scripts as a `CONSTS` operand and by GFF fields such as `Tag` and `Speaker`; a strref by `CExoLocString` fields and 2DA `name`/`description` columns; a global by `CONSTS` arguments to the `GetGlobal*`/`SetGlobal*` routines and by `globalcat.2da` rows. The kinds are closed; `auto` tries all and says which matched.
- **No game bytes in the repository.** The example query file holds SQL, not results.
