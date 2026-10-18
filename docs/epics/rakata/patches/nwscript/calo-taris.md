---
number: 99
title: "Calo Nord, Taris: the chapter's scenes written on the authoring API, built as a checkpoint"
kind: task
priority: p1
parent: m3-write
blocked_by: [calo-screenplay, calo-research, juhani-port]
assignee_kind: agent
labels: [rakata, calo, modshop, authoring]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "Authoring" and "The first mod it serves": the Taris scenes are written against the authoring API as the Juhani mod is ported onto it, so the API is proven on two mods before the second planet; one recipe entry per planet in story order makes Taris a checkpoint. The Juhani port (`juhani-port`) is the precedent for the mod's shape in modshop (`games/kotor/mods/juhani-clones/`: `make.py` as constants, a scene table and calls into `rakata`; how the `rakata` module reaches the step and how the entry is keyed). The research is `games/kotor/mods/calo-nord/research/taris.md`; the screenplay is where the `calo-screenplay` gate's result says.

## Deliverables

- `games/kotor/mods/calo-nord/` in modshop: `make.py` against `rakata` for the Taris chapter, `scenes.py` holding the screenplay's scenes as data, a README; voice and lip sync through `rakata`'s voice helpers when the gate says the lines are voiced.
- A recipe entry `mod:calo-nord-taris` after the Juhani entry, built into a checkpoint.
- A save fixture for the chapter's first new scene through `rakata save set`, with the in-game check written for the owner, as the save-set node did.

## Acceptance criteria

- [ ] The entry builds; `rakata dialog lint` over every dialogue it writes and `rakata lint --paths` over its `changed.txt` report nothing beyond the known lists.
- [ ] `rakata diff` of the entry's checkpoint against its parent lists exactly the chapter's changes, itemised in the README.
- [ ] Every scene in the screenplay maps to a scene in `scenes.py`, and every line to a node with a stable key.
- [ ] `make.py` holds no resolution, hashing or GFF plumbing; every operation it needed that the API lacked was added to `rakata::author` (with tests and the Python stubs) in nwscript.
- [ ] `mise run check` passes in modshop; the owner confirms the first scene on the handheld (a gate, filed when this node completes).

## Notes

- Repository rules: no mod authors' files or build outputs in git; no device identity; nothing edited in the game folder by hand. Commit messages are conventional; do not push.
- The owner's lines are the owner's; a line the screenplay lacks is a question for the owner, not an invention.
