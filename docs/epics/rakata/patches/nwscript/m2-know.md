---
title: "Rakata M2 Know: corpus, dialogue transcripts, the index, refs, the Canderous report"
kind: milestone
priority: p1
labels: [milestone, rakata]
---

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, sections "Dialogues", "The index and references" and "The corpus".

Exit: `rakata corpus` decompiles every module script with headers and a call index; `rakata dialog show` renders `banter.dlg` with its conditions decompiled; the index builds cold and incrementally over the full build with published times; `rakata refs` reproduces the strref 42724 transition chain in three calls with no false negative; the Canderous coverage report runs as one query.
