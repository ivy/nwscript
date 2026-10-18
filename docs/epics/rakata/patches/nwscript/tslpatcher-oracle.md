---
title: "Spike: drive TSLPatcher under Wine without its GUI, as the installer's oracle"
kind: spike
priority: p3
parent: m6-install
assignee_kind: agent
labels: [rakata, patch, suite, spike]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "The installer" (acceptance: differences explained against TSLPatcher run under Wine on the same inputs) and "Clean room" (TSLPatcher may be run as a black box; its source and any reconstruction of it may not be read). modshop's `games/kotor/docs/ideas/patcher-parity.md` names the unknowns: whether TSLPatcher installs from a command line or needs synthetic input, which versions the mods ship (each archive carries its own `TSLPatcher.exe`), and where expected differences come from. modshop already runs a Windows installer in a Wine container with no network: `~/src/github.com/ivy/modshop/games/kotor/kmrp-build/wine/Containerfile`, used by its KMRP step. The mod archives are under `~/src/github.com/ivy/modshop/games/kotor/data/archives/<entry>/`; modshop's `Extractor` (7-Zip) opens them.

## Deliverables

- `docs/research/tslpatcher-oracle.md`: the `TSLPatcher.exe` versions found across the archives (version string and SHA-256, count of mods per version); whether and how each runs headless (command-line arguments, or `xvfb-run` with `xdotool`); the time one install takes in the container; what the log file it writes contains.
- `conformance/drivers/tslpatcher.sh` (beside the compiler drivers): takes a `tslpatchdata/` directory, a game directory and an option number, runs TSLPatcher in the container, and leaves the modified tree and TSLPatcher's log.
- One real run: the mod of modshop's repair 09 (`games/kotor/repairs/09-blaster-adjustment-orphan-rows/README.md` names it) installed by its own TSLPatcher onto a copy of the parent checkpoint's tree, compared with the build's checkpoint for that entry (`cmp -r` counts, differing paths listed in the research document, no bytes).

## Acceptance criteria

- [ ] The research document exists with the version table and the headless finding.
- [ ] The driver installs that mod with no person interacting with a window, twice, producing identical trees.
- [ ] The comparison with the build's checkpoint is in the document, with each differing path classified (repair 09's skipped row is the expected one).

## Notes

- Do not decompile or disassemble TSLPatcher; run it. kpatcher's notes (`~/src/github.com/th3w1zard1/kpatcher`) are off limits as source of implementation, and this spike does not need them.
- Mod authors' files stay under modshop's `data/`; the research document names versions and hashes only.
