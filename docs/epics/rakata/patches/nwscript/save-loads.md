---
title: "Load a save written by `rakata save set` on the handheld and confirm the changed global"
kind: gate
priority: p2
parent: m4-saves
blocked_by: [save-set]
assignee_kind: user
labels: [rakata, save, gate]
---

## Context

The proposal's exit for saves (`docs/proposals/rakata.md`, Milestones, row 4) is a written save that loads on the handheld and shows the changed global. Only a person at the device can see that. The `save-set` node's result names the fixture save's directory, the global it set, the save number to give it, and what to look for in play; `docs/saves.md` has the copying steps.

## What to do

- Copy the fixture to the handheld's `Saves/` folder under the number the result names (the deploy's drift check should be run before and after, as `games/kotor/README.md` in modshop says for any change on the device).
- Load it, and do the check the result describes (for `OAJ_DONE`: talk to Carth and look for the epilogue question in his menu).
- Complete this node with what happened: the save loaded or did not, the line appeared or did not, and anything else seen.

## Done when

The completion result says whether the save loaded and whether the change was visible.
