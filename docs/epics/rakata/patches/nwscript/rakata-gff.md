---
title: "rakata-gff: lossless GFF read, BioWare-layout write, field paths, diff"
kind: task
priority: p1
parent: m1-formats
blocked_by: [rakata-res, conformance]
assignee_kind: agent
labels: [rakata, gff]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "GFF, 2DA, TLK and LIP" and "Round-trip conformance". GFF is the container for dialogues, blueprints (UTC, UTI, UTP, UTD, UTT, UTE, UTM, UTS, UTW), area files (ARE, GIT, IFO), journals, factions, GUI files and the `.res`/`.sav` members of a save; it was the second most used PyKotor surface in the sessions surveyed (proposal, Appendix). The format (V3.2 for KotOR) is documented in BioWare's GFF specification as reproduced on the KotOR and NWN wikis and in xoreos's documents. The suite's requirement: BioWare's files write back byte-identical; files other tools wrote may differ in layout and are then "equal after re-read" with the variant named.

## Deliverables

- `crates/rakata-gff/`: `Gff { file_type, version, root }`, `Struct { id, fields }`, `Field` with every type the engine uses (Byte, Char, Word, Short, Dword, Int, Dword64, Int64, Float, Double, CExoString, ResRef, CExoLocString with strref and substrings by language and gender, Void, Struct, List, Orientation, Vector, StrRef), reading with struct order, field order and the label table order preserved; writing BioWare's layout from the preserved order so a read file writes back identically; a `Path` syntax (`EntryList[3].RepliesList[0].Index`) with parse, get and set; `diff(&Gff, &Gff) -> Vec<Change { path, before, after }>`; `semantic_eq`.
- Registration with the conformance `Format` registry under the GFF extensions and for `.res`/`.sav` members by their file-type field.
- Hand-made fixtures built with the API; the test over `$RAKATA_GAME_K1` and `$RAKATA_BUILD` runs through `rakata conformance`.
- `Cargo.toml` row; `AGENTS.md` row present.

## Acceptance criteria

- [ ] `rakata conformance --game $RAKATA_GAME_K1 --format gff`: every GFF identical; the count per file type is in the README table.
- [ ] `rakata conformance --game $RAKATA_BUILD --format gff`: zero "different"; every "equal after re-read" variant is named in `conformance/rakata-known.toml` with how it is recognised.
- [ ] Path test: set `Tag` on a hand-made UTC through the path API, write, read, get returns the new value, and the rest diffs empty.
- [ ] `diff` of two hand-made blueprints differing in two fields lists exactly those two paths.
- [ ] `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Notes

- BioWare shares one label table entry across every field with the same label and orders the data block by first use; keep the label list and the order in the model rather than recomputing them, or identical write-back fails on files with repeated labels.
- A `CExoLocString` strref of `0xFFFFFFFF` means none; do not normalise it to -1 or 0 on write. Floats are written from the bytes read, never reformatted.
- `Void` fields appear in area GITs and saves; carry them as bytes.
- **Clean room** (`docs/proposals/rakata.md`, Clean room): PyKotor's `resource/formats/gff` may not be opened; the wikis' and xoreos's documents may. **No game bytes in the repository.**
