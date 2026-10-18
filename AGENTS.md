# AGENTS.md

Onboarding for coding agents working on this workspace. The design and its reasons are in [`docs/prd.md`](docs/prd.md) for the scripts layer and [`docs/proposals/rakata.md`](docs/proposals/rakata.md) for everything else; read their Core principles and Clean room sections before any change to a crate.

## What this is

A Rust workspace: a toolset for the data of BioWare's Aurora-family games, KotOR 1 and 2 first. Crates under `crates/`, one responsibility each. The `nwscript-*` crates are the scripts toolchain, as the PRD's Architecture table lists them; the binaries are `nws` (the command line) and `nwnnsscomp` (a compatibility shim; the only place nwnnsscomp's flags exist). The `rakata-*` crates are the data layers, as the Rakata proposal's Architecture table lists them, with the `rakata` façade and the `rakata` binary; those not yet present are planned:

| Crate | Responsibility | State |
| --- | --- | --- |
| `rakata-res` | `ResRef`, the resource type table, content hashes | present |
| `rakata-archive` | KEY/BIF, ERF, RIM: read, write, add and replace members | present |
| `rakata-2da` | tables, every header variant the engine accepts, diff | present |
| `rakata-tlk` | `dialog.tlk` and `LocalizedString`, append | present |
| `rakata-lip` | lip-sync files: the duration and the keyframes, read, written bit for bit, made from cues | present |
| `rakata-game` | the install: search order, `which`, the `Resolver` for nwscript | present |
| `rakata-gff` | the GFF tree, lossless read, BioWare-layout write, paths, diff | present |
| `rakata-dialog` | DLG model, stable keys, transcript, lints | present |
| `rakata-save` | save games by name, and a copy with changed globals or party | present |
| `rakata-index` | facts, references and full text in SQLite, keyed by content hash; the schema is [`docs/index.md`](docs/index.md) | present |
| `rakata-patch` | `changes.ini` reader, plan, executor, report read back from the tree | present |
| `rakata` | the façade and the diagnostic codes | present |
| `rakata-lua` | the Lua binding of the façade, run hermetically by `rakata lua`; `api.md` is its reference | present |
| `rakata-py` | the Python module `rakata`: the façade's classes, `rakata.author`, and `rakata.nwscript` mounted from `nwscript-py` | present |
| `rakata-cli` | the `rakata` binary | present |

The `nwscript-*` library crates never depend on a `rakata-*` crate; `rakata-game` implements their resolver, and only the `nws` binary may take it.

## The clean-room rule

Do not open the source of nwnnsscomp (`~/src/github.com/niv/nwn-tools`, `~/src/github.com/meaglyn/nwn-tools`, any SourceForge tarball), Beamdog's compiler in `~/src/github.com/niv/neverwinter.nim`, PyKotor (all of it, in any checkout or installed copy, including modshop's `.venv`), HoloPatcher, kpatcher (`~/src/github.com/th3w1zard1/kpatcher`), TSLPatcher's reconstructed source, xoreos-tools' code (GPL), or nwnrs-nwscript (GPL), and do not query codebase-memory against those projects. Run compilers, patchers and other tools only as black boxes. Permitted: the user's own game files and mod archives, BioWare's `nwscript.nss`, written format documentation (xoreos's documents, the KotOR and NWN wikis' format pages, stoffe's TSLPatcher readme and `changes.ini` syntax pages), the engine's observable behaviour, and other tools' output.

Two roles share the room. The `format-scribe` agent (`.claude/agents/format-scribe.md`) is the one role that may read the references, and it writes only specifications under `docs/formats/`: facts about bytes and behaviour with a provenance section, never code or prose from the source. Every other agent is an implementer: it reads the public documents, `docs/formats/` and the game's files, and runs the reference tools as black boxes. The hooks in `.claude/settings.json` enforce the split (`scripts/clean-room.py` refuses tool calls naming a reference project; `scripts/scribe-write-guard.py` confines the scribe's writes). Ask for a specification only for a gap the public documents leave, and prefer measuring a tool to reading it. Spawn the scribe with its `subagent_type` and no `name`: the hooks recognise the role by the `agent_type` field, and a spawn name replaces that field. The purpose is independence as much as licence hygiene: an implementer who has seen how another project arranges its parsers carries that arrangement into the design, so the specifications describe bytes and behaviour and never a program.

## Game trees for tests

No game bytes are committed, ever. Tests that need a game install read the user's own, from two variables, and skip with a message naming the variable when it is unset: `$RAKATA_GAME_K1`, a vanilla KotOR 1 install (the folder holding `chitin.key`); `$RAKATA_BUILD`, a modded build in the same layout. `$RAKATA_SAVES` names a folder of save-game directories for the save tests. On the maintainer's build host these are modshop's `games/kotor/data/game/vanilla/tree`, `games/kotor/data/game/build` and `games/kotor/data/saves`, under `~/src/github.com/ivy/modshop`.

## The corpus

No game bytes are committed, ever. Tests that need BioWare's scripts read a corpus extracted from the user's own game, from `$NWSCRIPT_CORPUS` (default `~/.cache/nwscript/corpus/k1`), and skip with a message naming the variable when it is absent.

```bash
rakata corpus extract --game PATH/TO/swkotor     # writes the corpus (cargo xtask corpus extract --game-dir is the same)
rakata corpus verify                             # checks it against conformance/k1.toml
rakata corpus decompile --game PATH/TO/swkotor   # decompiled/: every bytecode-only script, with headers and calls.jsonl
```

[`docs/grammar.md`](docs/grammar.md) is the language as the corpus uses it, each construct cited with counts from `cargo xtask grammar-census`. After changing the census or the document, `cargo xtask grammar-census --check docs/grammar-census.txt --doc docs/grammar.md` confirms they agree.

The layout is `nss/`, `ncs/`, `nwscript.nss` as shipped, `oracle/` (the include directory for nwnnsscomp, with BioWare's line-5145 typo fixed), `modules/STEM/` and `rims/STEM/` for the NCS in each capsule, and `decompiled/CAPSULE/NAME.nss` with `decompiled/calls.jsonl` from `rakata corpus decompile` ([`docs/decompiler.md`](docs/decompiler.md)). On the maintainer's build host the vanilla install is modshop's `games/kotor/data/game/vanilla/tree`.

## Working here

- Build and check: `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`. A change is done when all four pass.
- Corpus-backed tests: `NWSCRIPT_CORPUS=… cargo test --workspace -- --include-ignored`.
- Round-trip conformance for data: `rakata conformance --game $RAKATA_GAME_K1 --report target/rakata-conformance.json`, then `cargo xtask readme --report target/rakata-conformance.json` refreshes the README's table. Reports describe the user's files and stay out of git; only the vanilla table goes into the README. Every format crate registers in `crates/rakata/src/formats/` and must pass before it ships.
- Fuzzing: targets live in `crates/<crate>/fuzz/`, outside the workspace, and need a nightly toolchain: from that directory, `mise x cargo:cargo-fuzz -- cargo +nightly fuzz run decode -- -max_total_time=600`.
- The Python module: `mise run test:py` makes a uv environment under `target/rakata-py/`, builds `crates/rakata-py` into it with `maturin develop`, and runs its pytest suite and pyright over the tests; the game-backed tests read `$RAKATA_GAME_K1`. Its stubs (`crates/rakata-py/python/rakata/*.pyi`) are the Python API and change with it.
- Tools come from mise (`mise.toml`); do not install with brew, apt or npx.
- Diagnostics are data (`nwscript-diag`): every error has a registered code with an explanation; never build one from rendered text.
- Rakata codes come in blocks of a hundred, one per crate or command family, so parallel branches never collide: `rakata-res` RE01xx, `rakata-archive` RE02xx, `rakata-patch` RE03xx/RW03xx, `rakata-game` RE04xx/RW04xx, conformance RE05xx/RW05xx, `rakata-gff` RE06xx/RW06xx, `rakata-2da` RE07xx/RW07xx, `rakata-tlk` RE08xx/RW08xx, `rakata-lip` RE09xx/RW09xx, `rakata corpus` RE10xx/RW10xx, `rakata-dialog` RE11xx/RW11xx, `rakata-save` RE12xx/RW12xx, the inspection commands (`rakata cat`, `gff`, `2da`, `tlk`, `lip`) RE13xx/RW13xx, the authoring API (`rakata::author`) RE14xx/RW14xx, `rakata-index` RE15xx/RW15xx, the semantic diff (`rakata diff`) RE16xx/RW16xx, the Lua binding (`rakata-lua`, `rakata lua`) RE18xx, the tree lint (`rakata lint`) RE19xx/RW19xx, `rakata decompile` and `rakata show` (`rakata::show`) RE20xx/RW20xx; the next crate takes the next free block and records it here.
- Markdown: one paragraph per line, no hard wrapping. Documents state current facts, not their history.
- Commits: small and coherent, conventional-commit subjects, no AI attribution.
- Prior work is described with respect: what it achieved and the constraints it was built under.
- Record what the corpus teaches as you find it: a quirk of BioWare's compiler, a defect in the shipped sources, a trap in the bytecode. It goes in `docs/research/bytecode-quirks.md` with the evidence (file, offset, the command that shows it); measurements of tools go in their own dated file under `docs/research/`.
