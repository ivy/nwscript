---
title: "`rakata conformance`: round-trip every resource of a tree and classify each file"
kind: task
priority: p1
parent: m0-workspace
blocked_by: [rakata-skeleton, rakata-archive]
assignee_kind: agent
labels: [rakata, suite, cli]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, section "Round-trip conformance": over a named tree, read every resource with the matching reader, write it with the matching writer, and classify the result per file as identical, equal after re-read (a layout another writer chose, named), or different (a bug). It is the counterpart of `nws conformance` (`crates/nws/src/cmd/conformance.rs`, manifest `conformance/k1.toml`, README rows between `<!-- conformance:begin -->` and `<!-- conformance:end -->`), and it is what each format crate must pass before it ships. The defect it exists for: a 2DA read as empty and written back empty, found in play (modshop `games/kotor/patches/README.md`, item 3). Only archives have a crate when this node starts; the format crates register later.

## Deliverables

- In `crates/rakata/`: a `Format` registry (name, extensions, `read`, `write`, `semantic_eq`) that `rakata-archive` implements now and each format crate implements when it lands.
- `rakata conformance --game DIR [--format NAME,...] [--report FILE] [--known FILE]`: walks `Override/`, `modules/`, `rims/`, `lips/`, the BIFs behind `chitin.key`, `data/*.bif` members, `streamwaves/*.lip` and the TLK; classifies each file with a registered format and counts files with none as "no reader"; writes a JSON report and a markdown summary; exit 3 if any file is "different".
- `conformance/rakata-known.toml`: named layout variants (writer, description, how recognised), no bytes.
- `cargo xtask readme` (or the existing README generation if there is one) inserts the summary table between `<!-- rakata-conformance:begin -->` and `<!-- rakata-conformance:end -->` in `README.md`, per format: files, identical, equal-after-reread, different, no reader.
- Timing in the report: wall time per format and total.

## Acceptance criteria

- [ ] `rakata conformance --game $RAKATA_GAME_K1 --format archive` classifies every archive and its identical count equals the `rakata-archive` node's.
- [ ] `rakata conformance --game $RAKATA_BUILD` completes over the full modded build and the result records its wall time and counts.
- [ ] A test with a deliberately wrong writer registered under a test format classifies its file "different" and exits 3.
- [ ] The README table is generated and the markers survive a second run unchanged (idempotent).
- [ ] `--format json` emits one envelope; `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Notes

- The report over the user's build describes their files; keep it under `target/` or the user's cache, never in git. Only the vanilla summary numbers go into the README.
- Walk archives recursively: a member of a `.mod` is classified by its own format, and the archive by the archive format.
- **Clean room** (`docs/proposals/rakata.md`, Clean room). **No game bytes in the repository.**
