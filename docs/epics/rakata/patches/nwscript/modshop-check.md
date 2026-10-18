---
number: 86
title: "modshop runs the rakata lints over its mods: `modshop kotor lint`, in `mise run check` and per entry"
kind: task
priority: p3
parent: m5-prove
blocked_by: [dialog-lint, tree-lint, juhani-port]
assignee_kind: agent
labels: [rakata, modshop, lint]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, Milestones row 5 ("the lints in modshop's `mise run check` for its mods") and "Integration with modshop". The work is in `~/src/github.com/ivy/modshop` (read `CLAUDE.md`, `games/kotor/CLAUDE.md`, `docs/proposals/modshop-cli.md`). The build is a chain of checkpoints under `games/kotor/data/game/ckpt/<key>/`, each with `changed.txt`; `modshop kotor status` shows the chain; `mise run check` runs ruff, pyright strict and pytest. The idea this implements is `games/kotor/docs/ideas/chain-lint.md`: findings pinned to the entry that introduced them, found before anything is deployed.

## Deliverables

- `rakata` on the build host's PATH as modshop's `docs/proposals/rakata-in-modshop.md` says: the `rakata:local` task from a checkout during development, the `mise.toml` cargo-backend pin by revision otherwise; the lint runs beside the chain and enters no key.
- `modshop kotor lint [KEY] [--all]`: runs `rakata lint --paths <changed.txt>` and `rakata dialog lint` for the dialogues an entry changed, over that checkpoint's tree, and writes `lint.json` beside `changed.txt`; `--all` walks the chain; `status` shows entries with findings.
- `mise run check` runs `modshop kotor lint` over the chain end when `data/game/build` exists and says it skipped otherwise.
- `games/kotor/README.md` documents the command and what a finding means for an entry; the idea file `chain-lint.md` is updated or retired as the owner decides (ask in the result, do not delete).

## Acceptance criteria

- [ ] `modshop kotor lint` over the chain end reports nothing for the `mod:juhani-clones` entry.
- [ ] Over the whole chain, every finding names its entry; the count per entry is in the result.
- [ ] `mise run check` passes, with and without a build present (synthetic game under `tests/integration/`).
- [ ] `lint.json` is not part of the checkpoint's key (a test on the synthetic game: linting does not change `manifest.txt` or the key).

## Notes

- Repository rules: no build outputs in git; no device identity; `recipe.json` is regenerated and committed only if a step changed, which this should not require.
- Tools that enter a step's key rebuild the chain; the lint runs beside the chain, not in it.
