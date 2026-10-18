---
number: 53
title: "Rakata M0 Workspace: façade, binary, res, archive, game, conformance over archives"
kind: milestone
priority: p1
labels: [milestone, rakata]
---

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, extending `docs/prd.md` (the scripts toolchain) to the whole of a game's data.

Exit: the proposal is accepted and the README's first paragraph describes the toolset; the `rakata` façade crate and `rakata` binary exist with the same envelope and exit codes as `nws`; `rakata-res`, `rakata-archive` (grown from `xtask/src/archive.rs`) and `rakata-game` exist; `rakata which` agrees with what the game loads over the full modded build; `rakata conformance` runs over archives and its numbers are in the README.
