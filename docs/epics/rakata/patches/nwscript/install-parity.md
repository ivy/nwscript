---
number: 81
title: "Parity: `rakata install` over every HoloPatcher step of the full build, each difference explained against TSLPatcher"
kind: task
priority: p3
parent: m6-install
blocked_by: [patch-exec, semantic-diff, tslpatcher-oracle]
assignee_kind: agent
labels: [rakata, patch, modshop, suite]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "The installer" (acceptance) and "Integration with modshop". modshop's build (`~/src/github.com/ivy/modshop`; read `games/kotor/CLAUDE.md`) is a chain of checkpoints under `games/kotor/data/game/ckpt/<key>/` whose `meta.json` names the parent; `build/recipe.json` lists every entry and its steps; the HoloPatcher steps' logs are under `data/logs/holopatcher/<entry>-<step>.log`, with their timings. `games/kotor/docs/ideas/patcher-parity.md` describes the run: restore the parent twice, install with each patcher, diff. The TSLPatcher driver is `conformance/drivers/tslpatcher.sh` from the spike; the diff is `rakata diff`.

## Deliverables

- `modshop kotor parity [ENTRY...]` in modshop (`src/modshop/cli/`, a command over the recipe): for each HoloPatcher step, a reflink copy of the parent's tree, `rakata install` with the entry's `tslpatchdata` and option, `rakata diff` against the entry's checkpoint; where they differ, TSLPatcher through the driver on another copy, and a classification per differing path: rakata agrees with TSLPatcher (HoloPatcher's difference), HoloPatcher agrees with TSLPatcher (a rakata defect), or all three differ.
- `games/kotor/docs/research/patcher-parity.md`: the table per entry (result, differing paths, classification, cause where known), rakata's wall time beside HoloPatcher's from its log, dated and reproducible; no mod bytes.
- Each rakata defect found becomes a fixture and a fix in `rakata-patch` (separate commits in this repository), and the table is rerun.

## Acceptance criteria

- [ ] Every HoloPatcher step in `build/recipe.json` appears in the table.
- [ ] Zero rows classified "all three differ" are unexplained; every "rakata defect" row has a fixture in `crates/rakata-patch/tests/` and is fixed or listed as open with its cause.
- [ ] rakata's wall time per entry is no greater than HoloPatcher's logged time, or the exceptions are named with the cause.
- [ ] `mise run check` passes in modshop; the research document names no device identity.

## Notes

- Expected differences include repair 09's skipped row and the cases the proposal lists under "Reproducing TSLPatcher's defects"; classify, do not paper over.
- Mod authors' files and the resulting trees stay under modshop's `data/`.
