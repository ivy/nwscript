---
title: "rakata-index: facts, references and full text over a tree in SQLite, incremental by content hash"
kind: task
priority: p1
parent: m2-know
blocked_by: [dialog-model, corpus, rakata-2da]
assignee_kind: agent
labels: [rakata, index, cli]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, section "The index and references", and the Architecture row for `rakata-index`, the one crate declared to own a file. modshop's `games/kotor/docs/ideas/asset-index.md` lists the questions it answers and the shape: a row per dialogue node with its fields, per script call with routine and literal arguments (from `corpus`'s `calls.jsonl`), per GFF field of a blueprint or area instance, per 2DA cell, per TLK entry; references in every direction; full text over node text, comments, strings and script source. Keying by content hash is what makes it cheap over a build whose 1,609 checkpoints share most files: each checkpoint's `manifest.txt` is SHA-256 per path and `changed.txt` names what a new one touched (`~/src/github.com/ivy/modshop/games/kotor/data/game/ckpt/<key>/`). Two practices from DeusData's codebase-memory-mcp (MIT, `~/src/github.com/DeusData/codebase-memory-mcp`, `src/store/`) are adopted as design: a content digest of the index computed from what rows say rather than where they are stored, so an incremental build is proven equal to a cold one; and a coverage table recording what each file contributed (parsed, no reader, failed), so a negative answer can be qualified.

## Deliverables

- `crates/rakata-index/` on `rusqlite` with the `bundled` feature (no system dependency): tables `resources(hash, name, type, size)`, `locations(tree, path, member, hash)`, `trees(id, label, parent)`, `dialog_nodes`, `gff_fields`, `cells`, `strings`, `calls`, `refs(from_hash, kind: resref|strref|tag|global, value, site)`, `coverage(tree, path, status, reason)`, and FTS5 over text, comments, strings and script source; schema versioned in a `meta` table.
- Extractors as pure functions in the format crates (`rakata_dialog::facts`, `rakata_gff::facts`, `rakata_2da::facts`, `rakata_tlk::facts`; script calls from `calls.jsonl` or directly through `nwscript-ncs`'s decoded instructions), unit-tested there; only the store is here.
- `rakata index build --game DIR --db FILE [--tree LABEL] [--changed FILE] [--parent LABEL]`: cold, or incremental from a list of changed paths, parsing a hash once however many trees hold it; builds in an in-memory database and writes once.
- `rakata index query --db FILE SQL` (table or `--format json`), `rakata index digest --db FILE [--tree]`, `rakata index coverage --db FILE`.
- Timing and size printed by `build` and recorded in the result and in `docs/` where the proposal's Performance section says.

## Acceptance criteria

- [ ] Cold build over `$RAKATA_BUILD` completes; wall time and database size are in the result (target under a minute).
- [ ] Incremental build from a checkpoint's `changed.txt` over its parent's index parses only the changed hashes (logged count) and finishes in seconds; `rakata index digest` of the result equals the digest of a cold build of the same tree.
- [ ] With two checkpoints indexed in chain order, the Juhani dialogs' hashes have the `mod:juhani-clones` checkpoint as their first tree.
- [ ] `rakata index query` with a call-argument query for `k_pman_aioff` returns `k_pdan_juhani27`; an FTS query for `rakghoul` returns rows from the Taris transformation scripts' calls and dialogue nodes.
- [ ] `coverage` lists every file of the tree with a status, and files with no reader are counted in `build`'s output.
- [ ] `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Notes

- The database is derived from game bytes: it lives under the user's cache or where `--db` says, never in git.
- Do not add a graph query language; SQL over typed tables is the query surface, and `refs` (the next node) is the one traversal command.
- **Clean room** (`docs/proposals/rakata.md`, Clean room): codebase-memory-mcp is not a KotOR tool and may be read for design, but nothing is copied from it; PyKotor's `pykotor.tools.references` may not be opened. **No game bytes in the repository.**
