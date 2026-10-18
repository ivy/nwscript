---
number: 66
title: "rakata-2da: tables with every header variant the engine accepts, written in BioWare's layout, diffed by cell"
kind: task
priority: p1
parent: m1-formats
blocked_by: [conformance]
assignee_kind: agent
labels: [rakata, 2da]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "GFF, 2DA, TLK and LIP". The format is binary 2DA V2.b, documented on the KotOR wiki and in xoreos's documents. The defect this crate must not have is item 3 of modshop's `games/kotor/patches/README.md`: five of the build's mods ship tables whose header says 0 for the cell-data size, an editor quirk the game ignores because it reads cells by offset; a reader that trusted the header read every cell as empty and wrote the table back empty, blanking every feat name until character creation crashed. Nothing failed at build time. Row labels are strings and are not always the row's index.

## Deliverables

- `crates/rakata-2da/`: `TwoDa { columns, rows: Vec<Row { label, cells }> }`; read by offsets, never by the header's cell-data size; the empty cell (`****`) kept distinct from an empty string; write in BioWare's layout, including its sharing of identical cell strings; `add_row_copying(from_label, new_label, changes)`, `add_column`, `get`/`set` by row label or index and column name; `diff` by row label and column; `semantic_eq`.
- Registration with the conformance registry for `.2da`.
- Fixtures: a small table built with the API, and the same table with its header's cell-data size patched to 0, built in the test.
- `Cargo.toml` row; `AGENTS.md` row present.

## Acceptance criteria

- [ ] `rakata conformance --game $RAKATA_GAME_K1 --format 2da`: every table identical; count in the README table.
- [ ] `rakata conformance --game $RAKATA_BUILD --format 2da`: the zero-cell-data-size tables read with their cells intact and classify "equal after re-read" under a named variant; zero "different".
- [ ] The build's `feat.2da` (the loaded copy, via `rakata-game` when available or the Override path) has non-empty `label` cells in every row (test, skips without `$RAKATA_BUILD`).
- [ ] The two fixtures read to equal tables; writing the patched one produces the normal one's bytes.
- [ ] `diff` of a table and its copy with one changed cell lists that row label and column only.
- [ ] `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Notes

- The row label column exists in the binary layout separately from the data cells; keep it out of `columns`.
- Column names are case-sensitive in the file; the game looks them up as written. Do not lowercase.
- **Clean room** (`docs/proposals/rakata.md`, Clean room). **No game bytes in the repository.**
