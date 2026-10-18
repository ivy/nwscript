---
title: "Rakata M5 Prove: dialogue and tree lints, the semantic diff, lints in modshop's check"
kind: milestone
priority: p2
labels: [milestone, rakata]
---

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, section "Lint and diff".

Exit: `rakata dialog lint` and `rakata lint` run over the full build in a published time with findings traced to the entries that introduced them; `rakata diff` of the Juhani checkpoint against its parent lists exactly the mod's changes by field; modshop runs the lints over its mods.
