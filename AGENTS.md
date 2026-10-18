# AGENTS.md

Onboarding for coding agents working on this workspace. The design and its reasons are in [`docs/prd.md`](docs/prd.md) for the scripts layer and [`docs/proposals/rakata.md`](docs/proposals/rakata.md) for everything else; read their Core principles and Clean room sections before any change to a crate.

## What this is

A Rust workspace: a toolset for the data of BioWare's Aurora-family games, KotOR 1 and 2 first. Crates under `crates/`, one responsibility each. The `nwscript-*` crates are the scripts toolchain, as the PRD's Architecture table lists them; the binaries are `nws` (the command line) and `nwnnsscomp` (a compatibility shim; the only place nwnnsscomp's flags exist). The `rakata-*` crates are the data layers, as the Rakata proposal's Architecture table lists them, with the `rakata` façade and the `rakata` binary; those not yet present are planned:

| Crate | Responsibility | State |
| --- | --- | --- |
| `rakata-res` | `ResRef`, the resource type table, content hashes | present |
| `rakata-archive` | KEY/BIF, ERF, RIM: read, write, add and replace members | planned |
| `rakata-gff` | the GFF tree, lossless read, BioWare-layout write, paths, diff | planned |
| `rakata-2da` | tables, every header variant the engine accepts, diff | planned |
| `rakata-tlk` | `dialog.tlk` and `LocalizedString`, append | planned |
| `rakata-lip` | lip-sync files | planned |
| `rakata-game` | the install: search order, `which`, the `Resolver` for nwscript | planned |
| `rakata-dialog` | DLG model, stable keys, transcript, lints | planned |
| `rakata-save` | save games by name | planned |
| `rakata-index` | facts, references and full text in SQLite | planned |
| `rakata-patch` | `changes.ini` reader, plan, executor, report read back from the tree | planned |
| `rakata` | the façade and the diagnostic codes | planned |
| `rakata-lua`, `rakata-py` | bindings; the Python module `rakata` carries `rakata.nwscript` | planned |
| `rakata-cli` | the `rakata` binary | planned |

The `nwscript-*` library crates never depend on a `rakata-*` crate; `rakata-game` implements their resolver, and only the `nws` binary may take it.

## The clean-room rule

Do not open the source of nwnnsscomp (`~/src/github.com/niv/nwn-tools`, `~/src/github.com/meaglyn/nwn-tools`, any SourceForge tarball), Beamdog's compiler in `~/src/github.com/niv/neverwinter.nim`, PyKotor (all of it, in any checkout or installed copy, including modshop's `.venv`), HoloPatcher, kpatcher (`~/src/github.com/th3w1zard1/kpatcher`), TSLPatcher's reconstructed source, xoreos-tools' code (GPL), or nwnrs-nwscript (GPL), and do not query codebase-memory against those projects. Run compilers, patchers and other tools only as black boxes. Permitted: the user's own game files and mod archives, BioWare's `nwscript.nss`, written format documentation (xoreos's documents, the KotOR and NWN wikis' format pages, stoffe's TSLPatcher readme and `changes.ini` syntax pages), the engine's observable behaviour, and other tools' output.

## Game trees for tests

No game bytes are committed, ever. Tests that need a game install read the user's own, from two variables, and skip with a message naming the variable when it is unset: `$RAKATA_GAME_K1`, a vanilla KotOR 1 install (the folder holding `chitin.key`); `$RAKATA_BUILD`, a modded build in the same layout. `$RAKATA_SAVES` names a folder of save-game directories for the save tests. On the maintainer's build host these are modshop's `games/kotor/data/game/vanilla/tree`, `games/kotor/data/game/build` and `games/kotor/data/saves`, under `~/src/github.com/ivy/modshop`.

## The corpus

No game bytes are committed, ever. Tests that need BioWare's scripts read a corpus extracted from the user's own game, from `$NWSCRIPT_CORPUS` (default `~/.cache/nwscript/corpus/k1`), and skip with a message naming the variable when it is absent.

```bash
cargo xtask corpus extract --game-dir PATH/TO/swkotor   # writes the corpus
cargo xtask corpus verify                              # checks it against conformance/k1.toml
```

[`docs/grammar.md`](docs/grammar.md) is the language as the corpus uses it, each construct cited with counts from `cargo xtask grammar-census`. After changing the census or the document, `cargo xtask grammar-census --check docs/grammar-census.txt --doc docs/grammar.md` confirms they agree.

The layout is `nss/`, `ncs/`, `nwscript.nss` as shipped, `oracle/` (the include directory for nwnnsscomp, with BioWare's line-5145 typo fixed), and `modules/STEM/` and `rims/STEM/` for the NCS in each capsule. On the maintainer's build host the vanilla install is modshop's `games/kotor/data/game/vanilla/tree`.

## Working here

- Build and check: `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`. A change is done when all four pass.
- Corpus-backed tests: `NWSCRIPT_CORPUS=… cargo test --workspace -- --include-ignored`.
- Fuzzing: targets live in `crates/<crate>/fuzz/`, outside the workspace, and need a nightly toolchain: from that directory, `mise x cargo:cargo-fuzz -- cargo +nightly fuzz run decode -- -max_total_time=600`.
- Tools come from mise (`mise.toml`); do not install with brew, apt or npx.
- Diagnostics are data (`nwscript-diag`): every error has a registered code with an explanation; never build one from rendered text.
- Markdown: one paragraph per line, no hard wrapping. Documents state current facts, not their history.
- Commits: small and coherent, conventional-commit subjects, no AI attribution.
- Prior work is described with respect: what it achieved and the constraints it was built under.
- Record what the corpus teaches as you find it: a quirk of BioWare's compiler, a defect in the shipped sources, a trap in the bytecode. It goes in `docs/research/bytecode-quirks.md` with the evidence (file, offset, the command that shows it); measurements of tools go in their own dated file under `docs/research/`.
