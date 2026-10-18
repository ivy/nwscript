---
title: "Rakata M1 Formats: GFF, 2DA, TLK, LIP round-trip; format commands; nws resolves from an install"
kind: milestone
priority: p1
labels: [milestone, rakata]
---

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, sections "GFF, 2DA, TLK and LIP" and "Round-trip conformance".

Exit: `rakata-gff`, `rakata-2da`, `rakata-tlk` and `rakata-lip` exist; `rakata conformance` classifies every file of those formats in the vanilla install and in the full modded build, and the numbers are generated into the README; `rakata cat|gff|2da|tlk|lip` commands exist; `nws --game-dir` resolves includes from an install through `rakata-game`.
