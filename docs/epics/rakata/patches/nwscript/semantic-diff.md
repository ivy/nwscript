---
title: "`rakata diff`: two trees or two resources by field, cell, string, member and dialogue node"
kind: task
priority: p2
parent: m5-prove
blocked_by: [rakata-gff, rakata-2da, rakata-tlk, rakata-archive, dialog-model]
assignee_kind: agent
labels: [rakata, diff, cli]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "Lint and diff" (the semantic diff). modshop's `games/kotor/docs/ideas/semantic-diff.md`: a checkpoint's `changed.txt` names paths, and what changed inside them takes a one-off script; a change to one member of a `.mod` shows as the whole archive. Checkpoints are `~/src/github.com/ivy/modshop/games/kotor/data/game/ckpt/<key>/` with `tree/`, `manifest.txt`, `changed.txt` and `meta.json` (which names the parent). The format crates each have a `diff`; this node composes them over trees and archives and adds the dialogue view.

## Deliverables

- `rakata diff A B [--only PATH...] [--format json]` where A and B are two files, two archives, or two trees: files paired by path (case-insensitively), archives recursed to members, each pair compared by its format: GFF by path, 2DA by row label and column, TLK by strref, DLG as a transcript diff (nodes added, removed, changed, by stable key where one exists), and every format without a crate by hash with its header fields where known (size and format for TPC, nothing beyond length for MDL), marked "contents not compared". Exit 3 on any difference.
- Output grouped by path, then by site, readable by a person and as JSON.

## Acceptance criteria

- [ ] `rakata diff <parent>/tree <juhani>/tree` over the `mod:juhani-clones` checkpoint and its parent lists exactly the mod's changes: the nodes added to its four dialogues, the `OAJ_*` rows in `globalcat.2da`, the `Juhani_Clones_Hidden` row in `appearance.2da`, and the added `.ncs`, `.utc`, `.dlg`, `.wav` and `.lip` files, with nothing else.
- [ ] A `.mod` and a copy with one member replaced name the member and its field-level difference.
- [ ] Wall time over two full checkpoint trees is in the result.
- [ ] `--format json`; `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Notes

- Pair Override files case-insensitively but report a case-only rename as a change of its own.
- The transcript diff keys nodes by voice resref, then strref, then text; nodes with none fall back to index and say so.
- **No game bytes in the repository.**
