---
number: 82
title: "modshop's side of the roadmap: `docs/proposals/rakata-in-modshop.md`, and how a local Rakata build enters a key"
kind: task
priority: p1
parent: m0-workspace
blocked_by: [accept-prd]
assignee_kind: agent
labels: [rakata, modshop, docs]
---

## Context

The proposal (`docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "Integration with modshop") says which of modshop's ideas become `rakata` commands and which stay in modshop, and that Rakata is pinned outside the step runner's Python import closure so a tool change does not rebuild the 190-entry chain. modshop has no document saying this from its side, and its ideas index (`~/src/github.com/ivy/modshop/games/kotor/docs/ideas/README.md`) still ranks the library questions as its own work. Two decisions the owner made in conversation on 2026-10-17 have no file yet, and later nodes (`juhani-port`, `modshop-check`, `modshop-install-step`) must cite one: using a change in Rakata from modshop must never require a push or a release; and what enters an entry's key is the build identity Rakata prints (`rakata --version` and `rakata.__version__`: workspace version, git describe, a hash of the dirty tree), not where the binary came from. modshop's design document is `docs/proposals/modshop-cli.md`; its conventions for proposals and for keys are in `docs/conventions.md` and `src/modshop/core/key.py` (`code_fingerprint`, `env_fingerprint`, `override_env_fingerprint`); a step takes a mise tool keyed by its version (`src/modshop/kotor/steps.py`, `Tool`; commit `a0360cd`).

## Deliverables

- `~/src/github.com/ivy/modshop/docs/proposals/rakata-in-modshop.md`: which idea files Rakata serves (inspect's `refs` and `which`, asset index, script corpus, device state's `save`, round-trip tests, semantic diff, chain lint's rules, patcher audit, patcher parity) and which stay (try, fixtures, recordings, live probes, virtual pad, crash capture, bisection, driver ergonomics, engine symbols); the keying rule: an entry that uses Rakata keys on the build identity string, obtained by running the tool, never on a path or a lock file; the two delivery modes: `mise run rakata:local` (builds `rakata` and the `rakata` wheel from a checkout path in `local.toml` into the build host's tool directory and the venv, with `maturin develop` or an equivalent) and the `mise.toml` cargo-backend pin for a release; the `Patcher` protocol as the seam for `rakata install`.
- The affected idea files gain one line under their title pointing at the proposal and the command that serves them; their priorities are left for the owner.
- `games/kotor/docs/ideas/README.md`: a sentence under "How they are ranked" saying which ideas moved.
- Nothing is implemented here; the tasks and settings are specified for the nodes that build them.

## Acceptance criteria

- [ ] The proposal exists, links `docs/proposals/rakata.md` in nwscript by URL, and states the keying rule and both delivery modes.
- [ ] Every idea file the proposal names has the pointer line; `grep -l rakata-in-modshop games/kotor/docs/ideas/*.md` lists exactly them.
- [ ] `mise run check` passes in modshop (documents only).
- [ ] No device identity (hostnames, user names, home paths) in the new text; the checkout path is described as a `local.toml` key.

## Notes

- Do not delete or re-rank idea files; that is the owner's.
- Documents state current facts: no "previously" or "now" narration; the commit message carries the history.
