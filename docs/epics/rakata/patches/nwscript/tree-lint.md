---
number: 85
title: "`rakata lint`: dangling references, strrefs past the TLK, case-variant duplicates and `.tpc` shadows over a tree"
kind: task
priority: p2
parent: m5-prove
blocked_by: [index-build, rakata-game]
assignee_kind: agent
labels: [rakata, lint, cli]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "Lint and diff" (the tree lints). modshop's `games/kotor/docs/ideas/chain-lint.md` lists the faults found by a crash or a patcher error that were visible in the files: references that resolve to nothing, case-variant duplicates in Override (`collapse_case` in `src/modshop/kotor/paths.py` found them), and a `.tpc` beside a same-named `.tga` or `.dds`, which crashes the game (modshop's `DedupeTpc` step in `steps.py` removes them). modshop runs a check per entry over what that entry changed (`changed.txt` in each checkpoint), so the command must take a list of paths. The rule about a `changes.ini` row naming a missing section needs the installer's reader and lands with milestone 6, not here.

## Deliverables

- `rakata lint --game DIR [--paths FILE] [--db FILE] [--known FILE] [--format json]`: rules with codes: a `ResRef` field in a GFF, or a `CONSTS` argument a script passes to a routine that takes a resref, that resolves to nothing in the install; a strref in a GFF or 2DA at or past the TLK's length; two Override files whose names differ only in case; a `.tpc` with a `.tga` or `.dds` of the same stem in Override. With `--paths`, only the listed paths are checked (resolution still sees the whole install); with `--db`, the index answers instead of a walk.
- Exit 1 on a finding not in the known list; findings carry path, field or site, and code.
- `docs/lints.md` section per rule.

## Acceptance criteria

- [ ] Over `$RAKATA_BUILD`, the case-variant duplicates reported equal `collapse_case`'s set (compare in the result; the script is not committed).
- [ ] A fixture Override with `a.tpc` and `a.tga` produces the shadow code; one with `Foo.utc` and `foo.utc` the duplicate code.
- [ ] `--paths` with a checkpoint's `changed.txt` runs in seconds over the full build and reports only findings in those paths.
- [ ] Whole-build wall time with `--db` is in the result.
- [ ] `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Notes

- Which routines take a resref is in the engine function table `nwscript-sema` already builds from `nwscript.nss` (`nwscript#24`); do not keep a second list.
- 2DA columns that hold resrefs are known per table from the wiki; start with the columns the game's own tables fill and keep the list in a data file with its source cited.
- **No game bytes in the repository.**
