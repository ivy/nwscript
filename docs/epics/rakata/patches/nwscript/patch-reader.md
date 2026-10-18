---
number: 77
title: "rakata-patch: `changes.ini` and `namespaces.ini` read with TSLPatcher's semantics, the plan, `install --dry-run`"
kind: task
priority: p2
parent: m6-install
blocked_by: [rakata-skeleton]
assignee_kind: agent
labels: [rakata, patch, cli]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "The installer" and "Clean room". The syntax and semantics come from stoffe's TSLPatcher readme and `changes.ini` documentation (permitted) and from what mods actually write: every `tslpatchdata/*.ini` in the archives under `~/src/github.com/ivy/modshop/games/kotor/data/archives/<entry>/` (extract with 7-Zip into a scratch directory; they are mod authors' files and stay out of git) and modshop's own change files under `games/kotor/fixes/`. Four of the eight defects in modshop's `games/kotor/patches/README.md` are reader semantics: a `[CompileList]` entry's replace flag (1), a lone `RequiredMsg=0` or lone `Required=` (2), a `namespaces.ini` section without `IniName` or `InfoName`, which default to `changes.ini` and `info.rtf` (4), and a row naming a section that does not exist, which TSLPatcher skips where the other reader raised (repair 09).

## Deliverables

- `crates/rakata-patch/`: a parser for `namespaces.ini` and `changes.ini` into a typed `Changes` model: `[Settings]`, `[TLKList]`, `[GFFList]` (with nested struct and list additions), `[2DAList]` (`ChangeRow`, `AddRow`, `CopyRow`, `AddColumn`, row selection by index, label or label index, `high()`, exclusive columns), `[InstallList]` (folders, `File#` and `Replace#`), `[HACKList]`, `[CompileList]`, `[SSFList]`, and the memory tokens (`2DAMEMORY#`, `StrRef#`) wherever they may appear; unknown keys kept and reported with a code, never silently dropped.
- A `Plan`: the operations in TSLPatcher's order (TLK, GFF, 2DA, InstallList, HACK, CompileList, SSF) with token placeholders, printable.
- `rakata install TSLPATCHDATA --game DIR [--option N] --dry-run [--format json]` printing the plan.
- `cargo xtask ini-census DIR`: parses every ini under a directory and prints the constructs it could not, with counts (a dev tool over the user's extracted archives; nothing committed).
- Hand-written fixture inis, one per section and one per defect above, under `crates/rakata-patch/tests/`.

## Acceptance criteria

- [ ] `cargo xtask ini-census` over the extracted archives parses every `changes.ini` and `namespaces.ini`, or names each unparsed construct with its file; the counts (mods, files, constructs) are in the node's result.
- [ ] Fixtures: lone `RequiredMsg=0` and lone `Required=` parse; a namespace without `IniName` defaults to `changes.ini`; a `[2DAList]` row naming a missing section yields a warning code and a plan that skips it.
- [ ] `--dry-run` on a fixture prints the plan in TSLPatcher's order; `--format json` wraps it.
- [ ] Every new code has an `explain` entry.
- [ ] `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Notes

- **Clean room** (`docs/proposals/rakata.md`, Clean room): PyKotor's `tslpatcher` package and HoloPatcher may not be opened; TSLPatcher's documentation and the mods' inis may be read; TSLPatcher itself may be run.
- The INI dialect is TSLPatcher's: case-insensitive keys, `;` comments, duplicate keys in `[InstallList]` folders numbered by suffix. Write the dialect down in the crate's docs as you find it.
- **No game bytes and no mod authors' files in the repository.**
