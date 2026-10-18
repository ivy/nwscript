---
title: "modshop's patcher step runs `rakata install`; the PyKotor patch series retires into regression tests"
kind: task
priority: p3
parent: m6-install
blocked_by: [install-parity]
assignee_kind: agent
labels: [rakata, modshop, patch]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "Integration with modshop" and Milestones row 6. In `~/src/github.com/ivy/modshop` (read `CLAUDE.md` and `games/kotor/CLAUDE.md`): `src/modshop/kotor/patcher.py` defines the `Patcher` protocol (`install(game, tslpatchdata, *, option, log_to, timeout, launcher) -> PatchReport`) with one implementation, `HoloPatcherCli`; `src/modshop/kotor/toolchain.py` resolves the patcher and computes the environment fingerprint that enters every key (`env_fingerprint` over `uv.lock`, or `override_env_fingerprint` when `MODSHOP_PATCHER` names another executable); `steps.py`'s `HoloPatcher` step calls it. `games/kotor/patches/README.md` lists the eight PyKotor defects the fork's series fixes and how the pin works. The parity node's table says where `rakata install` and HoloPatcher differ. Switch only when every row is classified and no "rakata defect" row is open; if one is, fix it there first. Where a row is an explained TSLPatcher-side difference, keep that entry's result as the table says and note it in `games/kotor/README.md`.

## Deliverables

- `RakataCli` implementing `Patcher`, producing a `PatchReport` from `rakata install --format json --report`; `Toolchain.resolve` selects it when `local.toml` or a setting says so, defaulting to it once parity holds, with the environment fingerprint being `rakata --version` from the `mise.toml` pin (cargo backend, revision-pinned), so a rakata bump rebuilds the chain once and a PyKotor change no longer does.
- The `HoloPatcher` step unchanged in shape; `case_aliases` kept only if the parity table shows a case rakata does not handle.
- `games/kotor/README.md`: the patcher section; `games/kotor/patches/README.md`: the PyKotor series marked retired for installing, each of its eight items pointing at the regression fixture in `crates/rakata-patch/tests/` (items 5 and 7 are the compiler's and already covered by `nws`); PyKotor stays installed for what Rakata does not yet read (`games/kotor/CLAUDE.md` says which skills use it).
- `build/recipe.json` regenerated and committed if the step's canonical form changed.

## Acceptance criteria

- [ ] A full rebuild with `RakataCli` reproduces every entry's tree hash, or each difference is in the parity table with its classification.
- [ ] `mise run check` passes; the synthetic game tests cover the new `Patcher`.
- [ ] Each of items 1, 2, 3, 4, 6, 8 in `patches/README.md` names a passing test in this repository.
- [ ] `modshop kotor status` after the switch shows the chain rebuilt once; a second `modshop kotor build` does nothing.

## Notes

- Repository rules: no build outputs or mod authors' files in git; no device identity; a step change rebuilds the chain by design, so make the switch in one commit.
- Do not delete the PyKotor fork or its series; retiring means it no longer installs anything, and the README says so.
