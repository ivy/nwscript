---
title: "rakata-patch executor: TSLPatcher's pipeline, memory tokens, compile through nwscript, and a report read back from the tree"
kind: task
priority: p2
parent: m6-install
blocked_by: [patch-reader, rakata-gff, rakata-2da, rakata-tlk, rakata-archive, rakata-game]
assignee_kind: agent
labels: [rakata, patch, cli]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "The installer": the sections run in TSLPatcher's order, a `[CompileList]` compiles through `nwscript` and never through a shipped `nwnnsscomp.exe`, and the report is the tree: after every operation the installer re-reads what it wrote and reports the hash it finds, so a claim that did not land is an error from the installer itself. The defects this must never have are in modshop's `games/kotor/patches/README.md`: a `Replace#` into a `.mod` that wrote the old bytes back while logging success (6); a `[CompileList]` compiled with a different compiler on Linux than on Windows (8); and the audit modshop wanted after the fact (`games/kotor/docs/ideas/patcher-audit.md`) is what the read-back makes unnecessary. The reader and the plan are `rakata-patch`'s from the previous node.

## Deliverables

- The executor over a `Plan` against an install (`rakata-game` resolves targets): `[TLKList]` appends with `StrRef#` tokens filled; `[GFFList]` edits by path with tokens, including struct and list additions; `[2DAList]` operations with `2DAMEMORY#` tokens; `[InstallList]` into folders and into `.mod`, `.rim` and `.erf` members through `rakata-archive`'s replace; `[HACKList]` byte writes; `[CompileList]` through the `nwscript` façade against the install's includes; `[SSFList]` with a small SSF reader and writer inside this crate.
- Read-back: after each operation the target file or member is re-read, its hash recorded, and a mismatch is an error naming the operation.
- `rakata install TSLPATCHDATA --game DIR [--option N] [--report FILE] [--format json]`: the report lists every operation, its target and the hash found; exit 1 on any failed read-back.
- Fixtures for defects 1, 2, 4, 6 and 8 and for each token kind, hand-made, under `crates/rakata-patch/tests/`.

## Acceptance criteria

- [ ] Replace fixture: an `InstallList` `Replace#` into a hand-made `.mod` that already holds the resource; re-reading the member gives the new bytes.
- [ ] A `[CompileList]` fixture compiles through `nwscript` and the bytes equal `nws compile`'s; a `nwnnsscomp.exe` placed in `tslpatchdata/` that fails if executed is never executed.
- [ ] Read-back test: a target made unwritable after planning produces an installer error naming the operation, not a success report.
- [ ] Token test: a `[2DAList]` `AddRow` whose `2DAMEMORY1` is used by a `[GFFList]` field and a `StrRef1` from `[TLKList]` used in a 2DA cell both resolve in the written files.
- [ ] One real mod from the archives installs onto a copy of `$RAKATA_GAME_K1` and the resulting tree's differences from the build's checkpoint for that entry are counted in the result.
- [ ] `--format json`; `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Scenarios

```gherkin
Scenario: The report is the tree
  Given a plan with one GFF edit and one InstallList copy
  When the install runs and the copy's destination is replaced by another process before read-back
  Then the report marks the copy failed with both hashes and the command exits 1
```

## Notes

- Where TSLPatcher is wrong about a format (the proposal's non-goal "Reproducing TSLPatcher's defects"), do what the game needs and record the difference as a named case for the parity node.
- **Clean room** (`docs/proposals/rakata.md`, Clean room). **No game bytes or mod authors' files in the repository.**
