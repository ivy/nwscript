---
number: 73
title: "`rakata dialog lint`: the dialogue faults play has found, as stable codes"
kind: task
priority: p2
parent: m5-prove
blocked_by: [authoring-api, corpus]
assignee_kind: agent
labels: [rakata, dialog, lint, cli]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "Dialogues" (the Lints paragraph) and "Lint and diff". The reference checks already exist in `rakata_dialog::check` (built by the authoring node for validation at write time); this node adds the rules that need the whole dialogue or the install, the command, and the published list of the game's own faults. The faults are the ones modshop's Juhani mod met in play (`games/kotor/mods/juhani-clones/README.md`): a condition or script that resolves to nothing, a voiced node without its file, a node nothing reaches, a global a condition reads that `globalcat.2da` does not declare, and the `SoundExists` flag disagreeing with the voice fields. `docs/lints.md` documents the scripts toolchain's lints; the dialogue lints join it.

## Deliverables

- Rules in `rakata-dialog` with codes: script or condition resolves to no resource; strref at or past the TLK's length; voiced node (`VO_ResRef` set) with no `streamwaves/<vo>.wav` (and no `.lip`, as a separate lower-severity code); node reached by no start and no link; condition reading a global (from its decompiled source through `nwscript-decomp`, or from the corpus's `calls.jsonl`) not declared in `globalcat.2da`; `SoundExists` disagreeing with `Sound`/`VO_ResRef`.
- `rakata dialog lint NAME.dlg|--all --game DIR [--module M] [--known FILE] [--format json]`; exit 1 on a finding not in the known list.
- `conformance/dialog-known.toml`: the vanilla game's own findings, as dialogue name, node index and code, no text.
- `docs/lints.md`: a section per rule, what it catches and the case it comes from.

## Acceptance criteria

- [ ] `rakata dialog lint --all --game $RAKATA_GAME_K1 --known conformance/dialog-known.toml` exits 0; without `--known` its findings are exactly the file's entries.
- [ ] `rakata dialog lint --game $RAKATA_BUILD --module danm14ac` over the Juhani mod's five dialogues (`dan14_cutjuh`, `dan14_juhani`, `k_hcar_dialog`, `k_hmis_dialog`, `oaj_banter`) reports nothing.
- [ ] Fixtures: a dialogue with a dangling script, one with an orphan entry, one with an undeclared global in a condition each produce their code and nothing else.
- [ ] Wall time over all vanilla dialogues is in the result.
- [ ] `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Notes

- Reachability here is structural (starts and links), not conditional; the conditional version is in the proposal's Later and is not this node.
- A global read through a variable rather than a literal cannot be checked; report it as "undecidable" under its own code, never as clean.
- **No game bytes in the repository**: the known list is names, indices and codes.
