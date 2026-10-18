---
title: "Accept the Rakata proposal: status line, README, AGENTS.md, the test data variables"
kind: task
priority: p0
parent: m0-workspace
assignee_kind: agent
labels: [rakata, foundation, docs]
---

## Context

`docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript` proposes the data layers of the toolset (archives, formats, the install model, dialogues, saves, an index, an authoring API, an installer) as `rakata-*` crates beside the `nwscript-*` ones, and renames the project. It is uncommitted (`git status` shows `?? docs/proposals/rakata.md`) and its status line says "proposed on 2026-10-17". Every later node cites that file by section, so it must be in the repository as the accepted design before any of them starts. The README's first paragraph and `AGENTS.md`'s "What this is" describe only the scripts toolchain, and `AGENTS.md`'s clean-room list names only compilers.

## Deliverables

- `docs/proposals/rakata.md`: the status line reads "accepted on <today's date>; extends `../prd.md`". Nothing else in it changes except a fact found wrong, each with the evidence in the commit message. The file stays where it is.
- `README.md`: the first paragraph describes the whole toolset (the scripts toolchain as one layer; archives, formats, the install, dialogues, saves, index, authoring, installer as the rest, in progress) and links the proposal. The Results section is unchanged.
- `AGENTS.md`: "What this is" gains the `rakata-*` crate table from the proposal's Architecture section, each row marked "planned" until its crate exists, and the `rakata` binary; the clean-room paragraph gains PyKotor (all of it, not only its compiler; any checkout and modshop's `.venv` copy), HoloPatcher, kpatcher (`~/src/github.com/th3w1zard1/kpatcher`) and TSLPatcher, with the permitted sources from the proposal's "Clean room"; a new section "Game trees for tests" decides and documents two variables: `$RAKATA_GAME_K1`, a vanilla KotOR 1 install (on this host `~/src/github.com/ivy/modshop/games/kotor/data/game/vanilla/tree`), and `$RAKATA_BUILD`, a modded build (`~/src/github.com/ivy/modshop/games/kotor/data/game/build`), with the rule that a test needing one skips with a message naming the variable when it is unset. Home paths go in the example as `~/...`, never as an absolute path with a user name.
- One commit on `main` with a conventional subject.

## Acceptance criteria

- [ ] `git log --oneline -1 -- docs/proposals/rakata.md` shows the commit and `head -3 docs/proposals/rakata.md` contains "accepted on".
- [ ] `head -5 README.md` names the data layers and links `docs/proposals/rakata.md`.
- [ ] `grep -n 'RAKATA_GAME_K1\|RAKATA_BUILD\|PyKotor\|kpatcher' AGENTS.md` finds all four.
- [ ] `cargo test --workspace` and `cargo clippy --workspace -- -D warnings` pass (nothing but documents changed).
- [ ] `git grep -n '/home/'` finds nothing new.

## Notes

- Do not rename the repository, the workspace, or any crate: the GitHub rename is a separate gate for the owner, and crates are added by the nodes that build them.
- Do not fold the proposal into `docs/prd.md`; the two are read together by design, and the first graph's nodes cite `docs/prd.md` by its current section names.
