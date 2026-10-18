---
title: "Port Oops! All Juhani onto the rakata Python module, with modshop's chain hash unchanged"
kind: task
priority: p2
parent: m3-write
blocked_by: [rakata-py, modshop-proposal]
assignee_kind: agent
labels: [rakata, modshop, authoring]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "Authoring" (acceptance) and "Integration with modshop". The work is in `~/src/github.com/ivy/modshop` (read its `CLAUDE.md` and `games/kotor/CLAUDE.md` first): the mod is `games/kotor/mods/juhani-clones/` (`make.py` 253 lines of plumbing against PyKotor, `dialog.py` a dialogue model, `voice.py` ffmpeg and Rhubarb into LIP files through PyKotor, `scenes.py` the scenes), installed by the recipe entry `mod:juhani-clones` (`src/modshop/kotor/recipe.py`, a `Script` step that runs `make.py` with the game dir and the tools). A step's key includes the files it reads and any mise tool it is given, keyed by version (`steps.py`, `Tool`); a change to the step runner's Python import closure or to `uv.lock`'s patcher packages rebuilds the whole chain (about 25 minutes), which is why the proposal pins Rakata outside that closure.

## Deliverables

- `make.py` written against `rakata` (the Python module): `Mod`, `loaded`, `guard_replaced`, the 2DA, global, blueprint and dialogue operations, `scripts.compile`, the writes. `dialog.py` deleted. `voice.py` keeps ffmpeg and Rhubarb and uses `rakata`'s spans, cues and cache key. `scenes.py` unchanged in what it says.
- The `rakata` module reaches the mod's interpreter the way modshop's `docs/proposals/rakata-in-modshop.md` decides (written by the `modshop-proposal` node; read it first): outside the step runner's import closure, with the entry keyed on `rakata.__version__`'s build identity, so a local `maturin develop` from the nwscript checkout rebuilds this entry and nothing else, and no push or release is needed to use a change. A `mise` task in modshop (`rakata:local`) builds and installs the binary and the module from a checkout path named in `local.toml`; documented in `games/kotor/README.md`.
- The mod's `README.md` updated; `grep -r pykotor games/kotor/mods/juhani-clones` finds nothing.
- The entry rebuilt: its checkpoint's `manifest.txt` compared with the previous checkpoint's for the same entry.

## Acceptance criteria

- [ ] `mise run check` passes in modshop.
- [ ] The rebuilt entry's `manifest.txt` is identical to the previous one, or every differing path is listed in the mod's README with the cause and a `rakata diff` (or per-format comparison) showing semantic equality.
- [ ] Only the Juhani entry rebuilt (`modshop kotor status` after the change shows the rest cached); after a one-line change in the nwscript checkout and `mise run rakata:local`, again only that entry's key changes.
- [ ] `make.py` contains no resolution, hashing or GFF plumbing of its own: its code is constants, the scene table and calls into `rakata` and `voice.py`.
- [ ] `games/kotor/README.md` says how the `rakata` module is installed and pinned.

## Notes

- Repository rules: no mod authors' files or build outputs in git; no device identity; nothing edited in the game folder by hand. Commit messages are conventional; do not push.
- `k_pdan_14juh_d` compiles in BioWare's mode (`--declarations bioware --compat bioware`); keep those flags in the compile call.
- If a byte difference comes from LIP or GFF layout (PyKotor's writer versus BioWare's), that is the expected "equal after re-read" case: explain it, do not chase identical bytes.
