---
number: 60
title: "rakata-archive: KEY/BIF, ERF and RIM read, written back identically, members added and replaced"
kind: task
priority: p1
parent: m0-workspace
blocked_by: [rakata-res]
assignee_kind: agent
labels: [rakata, archive]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "Architecture" (`rakata-archive`) and "Archives". `xtask/src/archive.rs` (159 lines) reads KEY V1, BIFF V1, RIM V1.0 and ERF/MOD/SAV V1.0 well enough to have extracted 11,355 module scripts from 119 archives; it copies members and cannot write. The regression this crate exists to make impossible is item 6 of modshop's `games/kotor/patches/README.md`: a replace into a `.mod` that wrote the old bytes back while the log said it replaced the file, silently dropping hundreds of module fixes per build.

## Deliverables

- `crates/rakata-archive/`: `Key` + `Bif`, `Erf` (ERF, MOD, SAV, HAK variants, V1.0, with the localized description strings and build date fields preserved), `Rim`; a `Source` abstraction over the file's bytes so members are borrowed slices, never copied until asked; `write` reproducing BioWare's layout; `add(ResId, bytes)` and `replace(ResId, bytes)` that produce an archive holding the new bytes under that name with every other member unchanged.
- `xtask` uses the crate and `xtask/src/archive.rs` is deleted.
- A test over `$RAKATA_GAME_K1`: every archive (`data/*.bif` with `chitin.key`, `modules/*.rim|*.mod`, `rims/*.rim`, `lips/*.mod`, `patch.erf`, `texturepacks/*.erf`) lists; each one read and written is compared to its source, and the counts of identical and different are printed. Hand-made fixtures (built with the API, a few bytes each) for the add and replace cases.
- `Cargo.toml` workspace row; `AGENTS.md` row present.

## Acceptance criteria

- [ ] `cargo xtask corpus verify` passes against `conformance/k1.toml` after the xtask switch, and `cargo xtask corpus extract` produces the same tree as before.
- [ ] Every vanilla archive lists; every one written from a read copy is byte-identical, or the node's result lists each exception with its cause.
- [ ] Replace fixture: an ERF with two members, one replaced; re-read gives the new bytes for it and the old for the other, and writing the result again is byte-identical to building the same archive fresh.
- [ ] Reading a 100 MB BIF and listing it allocates no per-member copies (a test or a documented measurement with `--release` timing in the result).
- [ ] `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Scenarios

```gherkin
Scenario: A replaced member is what the archive holds
  Given a .mod holding k_pdan_juhani31.ncs
  When replace(k_pdan_juhani31.ncs, new_bytes) is written and the archive re-read
  Then the member's bytes equal new_bytes and every other member's hash is unchanged
```

## Notes

- The workspace forbids `unsafe_code`, and `memmap2`'s map call is `unsafe`. The owner allows one exception, earned by a number: implement `Source` over whole-file reads first, then a mapped `Source` in one module of this crate with `#![allow(unsafe_code)]` on that module alone and the single call documented; measure cold open plus a full listing of every vanilla archive both ways and put both figures in the result. Keep the mapped source only if it is faster by a margin worth the exception; otherwise delete it and say so. The workspace lint stays `forbid`.
- Resref case: archives store names in mixed case; compare case-insensitively (`rakata-res`) and write back the spelling read.
- **Clean room** (`docs/proposals/rakata.md`, Clean room): do not open PyKotor's, HoloPatcher's, kpatcher's or TSLPatcher's source or query codebase-memory against them; permitted are the game's files, the xoreos documents and the wikis' format pages. **No game bytes in the repository.**
