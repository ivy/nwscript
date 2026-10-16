# AGENTS.md

Onboarding for coding agents working on nwscript. The design and its reasons are in [`docs/prd.md`](docs/prd.md); read its Core principles and Clean room sections before any change to a compiler crate.

## What this is

A Rust workspace: a toolchain for NWScript, the scripting language of BioWare's Aurora-family games, KotOR 1 and 2 first. Crates under `crates/`, one responsibility each, as the PRD's Architecture table lists them. The binaries are `nws` (the command line) and `nwnnsscomp` (a compatibility shim; the only place nwnnsscomp's flags exist).

## The clean-room rule

Do not open the source of nwnnsscomp (`~/src/github.com/niv/nwn-tools`, `~/src/github.com/meaglyn/nwn-tools`, any SourceForge tarball), Beamdog's compiler in `~/src/github.com/niv/neverwinter.nim`, pykotor's compiler (`pykotor/resource/formats/ncs/`), xoreos-tools' code (GPL), or nwnrs-nwscript (GPL), and do not query codebase-memory against those projects. Run compilers only as black boxes. Permitted: the corpus, BioWare's `nwscript.nss`, written format documentation (xoreos's docs, the KotOR and NWN wikis), and compilers' output.

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
- Tools come from mise (`mise.toml`); do not install with brew, apt or npx.
- Diagnostics are data (`nwscript-diag`): every error has a registered code with an explanation; never build one from rendered text.
- Markdown: one paragraph per line, no hard wrapping. Documents state current facts, not their history.
- Commits: small and coherent, conventional-commit subjects, no AI attribution.
- Prior work is described with respect: what it achieved and the constraints it was built under.
