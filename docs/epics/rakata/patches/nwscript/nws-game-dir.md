---
number: 88
title: "`nws --game-dir`: includes and nwscript.nss resolved from an install through rakata-game"
kind: task
priority: p3
parent: m1-formats
blocked_by: [rakata-game]
assignee_kind: agent
labels: [rakata, cli, game]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "Architecture", last paragraph: the `nwscript-*` library crates do not depend on any `rakata-*` crate; `rakata-game` implements their `Resolver` trait (`crates/nwscript/src/resolve.rs:19`), and `nws`, the command, may take it for a `--game-dir` flag. Today `nws` resolves includes from directories named on its command line; modshop's Juhani mod compiles against the build's own `nwscript.nss` and `k_inc_*` by extracting them first (`games/kotor/mods/juhani-clones/make.py`, `includes()`).

## Deliverables

- `nws compile|check|decompile|test --game-dir DIR [--module M]`: includes and `nwscript.nss` come from the install's loaded copies through `rakata-game`'s `Resolver` implementation; explicit include directories still take precedence when both are given.
- `crates/nws/Cargo.toml` depends on `rakata-game`; no `nwscript-*` library crate changes its dependencies.
- `nws --help` and `docs/` wherever the include flags are documented.

## Acceptance criteria

- [ ] `nws compile k_oaj_fight.nss --game-dir $RAKATA_BUILD --module danm14ac` (source from `~/src/github.com/ivy/modshop/games/kotor/mods/juhani-clones/`) produces bytes identical to compiling with include directories pointing at copies extracted by `rakata cat`.
- [ ] `cargo tree -p nwscript -e normal | grep rakata` and the same for `nwscript-sema`, `nwscript-codegen`, `nwscript-decomp` print nothing.
- [ ] A missing `nwscript.nss` in the named install is a diagnostic naming the flag, not a panic.
- [ ] `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass; the `nws` CLI tests still pass.

## Notes

- The Resolver's lookup is by resref and type; includes are `NSS` resources, which in a modded build may be in `Override/` as well as the BIFs. The install model decides; do not add a second search order here.
