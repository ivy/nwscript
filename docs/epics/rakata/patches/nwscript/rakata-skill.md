---
number: 92
title: "The skill teaches the data workflow (which, show, refs, author, lint); fresh-agent test with a data change"
kind: task
priority: p2
parent: "#9"
blocked_by: [format-commands, dialog-model, refs, authoring-api, rakata-py]
assignee_kind: agent
labels: [rakata, agent-surface, docs]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "Users" (the acceptance test is a fresh agent taking a described change to a working mod from the skill, `--help` and `explain`) and "Diagnostics, agent surface and command line" (a skill that teaches the workflow by progressive disclosure). The scripts skill is `plugin/skills/nwscript/SKILL.md`; a test checks every command and flag it cites against `--help` (`nwscript#43`), and `nwscript#50` is the fresh-agent acceptance test for scripts. modshop's `plugins/kotor/skills/pykotor/` is the skill this one replaces surface by surface.

## Deliverables

- `plugin/skills/rakata/SKILL.md` and its references: find the loaded copy (`which`, `cat`), read a dialogue (`dialog show`), find what refers to a thing (`refs`, `index build`), make a change through the Python module (the authoring operations with stable keys, and what a key failure means), check it (`dialog lint`, `lint`, `diff`), with the clean-room and no-game-bytes rules stated for agents working in the repository itself.
- The help-consistency test extended to the new skill.
- A fresh-agent trial: a new session with only the plugin, asked to add one reply with a condition on a new global to a vanilla dialogue in a scratch Override, from the skill alone; its findings fixed in the skill or the tool; a summary of the trial in the node's result.
- The plugin marketplace lists both skills.

## Acceptance criteria

- [ ] The help-consistency test passes for `plugin/skills/rakata/`.
- [ ] The trial completed the change with `rakata dialog lint` clean on the result, and the result records what the agent needed that the skill lacked.
- [ ] Every `rakata` command the skill names appears in `rakata --help`.
- [ ] `cargo test --workspace` passes.

## Notes

- Progressive disclosure: the SKILL.md front page is short; depth goes in reference files the agent opens when a step needs it.
- The skill never describes a flag the installed binary lacks; it ships at the workspace version for that reason.
