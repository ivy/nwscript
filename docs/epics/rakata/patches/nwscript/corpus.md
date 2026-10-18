---
number: 71
title: "`rakata corpus`: every module script decompiled, with origin headers, folded includes and a call index"
kind: task
priority: p1
parent: m2-know
blocked_by: [rakata-game]
assignee_kind: agent
labels: [rakata, corpus, cli]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, section "The corpus". `cargo xtask corpus extract` (`xtask/src/corpus.rs`) writes the vanilla scripts to `$NWSCRIPT_CORPUS` (default `~/.cache/nwscript/corpus/k1`): `nss/`, `ncs/`, `modules/<stem>/*.ncs`, `rims/<stem>/*.ncs`, `nwscript.nss`, `oracle/`. Module scripts exist only as bytecode, and `nws decompile` recovers them with 99.92% of functions verified (`docs/decompiler.md`). modshop's `games/kotor/docs/ideas/script-corpus.md` records what that cost without a decompiled corpus: the cause of the Juhani mod's companions ignoring the player was in `k_pdan_juhani27`, which runs `k_pman_aioff` on the party, found by `strings` over every `.ncs` and decompiling the hits one by one; about thirty hand decompiles in one session, the long ones printing their whole included library.

## Deliverables

- `rakata corpus extract --game DIR [--out DIR]`: the xtask's extraction moved into the façade and the command; `cargo xtask corpus` becomes an alias or is removed, and `cargo xtask corpus verify` against `conformance/k1.toml` keeps working under the new name.
- `rakata corpus decompile [--out DIR] --game DIR`: for every `.ncs` in every module capsule, `rims/` and `Override/`, writes `decompiled/<capsule>/<script>.nss` with a header comment naming the capsules that hold it, which copy the game loads (from `rakata-game`), and the verified function count; functions that match a function in an include the script could have included (matched by recompiled bytes through `nwscript-decomp`, never by name) are replaced by the `#include` line so a grep hit is in the script's own logic.
- `decompiled/calls.jsonl`: one line per engine call per script: script, capsule, routine name, literal arguments where the operands are constants, instruction offset. This is what the index takes as a script's references.
- Wall time printed at the end and recorded in `docs/decompiler.md`.

## Acceptance criteria

- [ ] `grep -rl k_pman_aioff $NWSCRIPT_CORPUS/decompiled/` lists `danm14ac_s/k_pdan_juhani27.nss` and no file outside `decompiled/`'s `.nss` files.
- [ ] Every file under `decompiled/` starts with the header; a script present in `danm14ac.rim` and `Override/` names Override as the loaded copy.
- [ ] `calls.jsonl` has a row for `k_pdan_juhani27` calling `ExecuteScript` with first argument `k_pman_aioff`.
- [ ] The verified fraction over all modules equals `docs/decompiler.md`'s figure (no regression), and the total wall time is recorded there.
- [ ] Running the command twice produces an identical tree (idempotent); `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Notes

- Include folding must not drop a function the script changed: `k_pdan_14juh_d` in modshop's Juhani mod differs from the game's by one call; a byte match is the only safe criterion.
- The output is derived from game bytes: it lives under the user's cache and is never committed.
- **Clean room** (`docs/proposals/rakata.md`, Clean room). **No game bytes in the repository.**
