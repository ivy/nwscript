---
number: 97
title: "Calo Nord, Taris: the research report over the index, rerun on every build"
kind: task
priority: p1
parent: m3-write
blocked_by: [refs]
assignee_kind: agent
labels: [rakata, calo, modshop, research]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "The first mod it serves": before a scene of the Calo Nord mod is written, milestone 2 answers the research, every node, gate and script call that names Canderous or his party slot, as a report rerun on every build, which is also the continuity audit. The mod makes Calo Nord an optional party member in Canderous's place across the campaign, Taris first. The index (`rakata index build`, `docs/index.md`), `rakata refs`, `rakata dialog show` and the query `docs/examples/canderous.sql` exist; the `refs` node's result counted Canderous's footprint per module (49 modules; Taris among them).

## Deliverables

- In modshop (`~/src/github.com/ivy/modshop`, on a branch, not `main`): `games/kotor/mods/calo-nord/research/taris.md`, the Taris chapter's research as the scene writer needs it: for every Taris module (`tar_m*`), each dialogue node Canderous speaks or hears, each node that names him, each condition or script that reads his party slot or tag, each blueprint or GIT instance of him, and where he joins the party; for each, the dialogue and stable node key (`voice:`, `strref:` or `text:`), the condition or script by name, and what it does in one line; Calo Nord's own Taris footprint (his blueprint, tag, dialogues, the swoop and the cantina scenes) the same way. Names, keys and one-line descriptions only: no game text beyond a short line identifier, because the repository holds no game bytes.
- `games/kotor/mods/calo-nord/research/taris.sql` (or a `rakata index query` script beside it) that regenerates every table in the document from an index of the build, so the report reruns on every build.
- The joins: the scenes where Canderous enters the party on Taris, the conditions that would branch if Calo took his place, and every global the Taris scenes set or read about him, as the list a screenplay must account for.

## Acceptance criteria

- [ ] Every Taris module in the build appears in the document, with its counts, or is listed as having none.
- [ ] The regeneration query reproduces the document's tables from a fresh index of `$RAKATA_BUILD` (test it once and say so).
- [ ] Every node key in the document resolves with `rakata dialog show` (none fragile, or the fragile ones marked).
- [ ] The joins section names each party-entry scene and each global with the dialogue or script that sets it.
- [ ] No game text beyond line identifiers in the committed files; modshop's `mise run check` passes.

## Notes

- The screenplay is the owner's; this node does not invent scenes or lines. It states what the game does today around Canderous on Taris, so the screenplay can say what changes.
- Clean room (`docs/proposals/rakata.md`, Clean room). No game bytes in either repository.
