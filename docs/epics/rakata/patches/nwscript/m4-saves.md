---
title: "Rakata M4 Saves: `save show` reproduces the Juhani findings; `save set` writes a save the handheld loads"
kind: milestone
priority: p2
labels: [milestone, rakata]
---

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, section "Saves".

Exit: `rakata save show` reproduces the three Juhani findings (AI off by local boolean 62 on two creatures, the world's lowercase tags, the banter queued behind action 61) from its output alone; `rakata save set` writes a save that differs only in the named globals or party and loads on the handheld showing the change.
