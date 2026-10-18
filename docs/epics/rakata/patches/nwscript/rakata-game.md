---
number: 64
title: "rakata-game: the install model, `rakata which`, and the `Resolver` for nwscript"
kind: task
priority: p1
parent: m0-workspace
blocked_by: [rakata-archive, rakata-skeleton]
assignee_kind: agent
labels: [rakata, game, cli]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "Resources and the install" and the principle "One model of the install". Which copy of a resource the game loads is answered three ways today: the Juhani mod's `loaded()` and `module_copies()` in modshop's `games/kotor/mods/juhani-clones/make.py` (Override, then the module's `.mod`, `.rim`, `_s.rim`, then the BIFs), modshop's `ci_find` and `collapse_case` in `src/modshop/kotor/paths.py` (case-insensitive lookup and the case-variant duplicates it found in Override), and `nwscript`'s `Resolver` trait (`crates/nwscript/src/resolve.rs:19`), which has no install-backed implementation. The engine's order and its module precedence are documented on the KotOR wiki and in xoreos's documents; the game runs under Wine on a case-sensitive file system, so paths compare without case on every platform.

## Deliverables

- `crates/rakata-game/`: `Install::open(dir, Profile)` (K1 first; K2 a profile with its own folders), `resolve(ResId, Option<&ModuleName>) -> Resolution { winner, candidates }` where each candidate names its location (Override path, capsule and member, or KEY/BIF), its content hash and why it lost; case-insensitive directory lookup over one cached listing per directory; module capsule enumeration in the engine's precedence.
- `impl nwscript::Resolver for Install` in this crate (the dependency runs from rakata to nwscript, never back).
- `rakata which NAME.EXT [--module M] --game DIR`: the winner, every candidate, and why; `--format json`.
- `Cargo.toml` row; `AGENTS.md` row present.

## Acceptance criteria

- [ ] For each resource the Juhani mod reads (`dan14_cutjuh.dlg`, `dan14_juhani.dlg`, `k_hcar_dialog.dlg`, `k_hmis_dialog.dlg`, `dan14_juhani.utc`, `k_pdan_juhani31.ncs`, `k_pdan_14juh_d.ncs`, `globalcat.2da`, `appearance.2da`, module `danm14ac`), `rakata which` over `$RAKATA_BUILD` names bytes whose SHA-256 equals what `make.py`'s `loaded()` returns when run in modshop's venv (`uv run python` from `~/src/github.com/ivy/modshop`); the comparison is a script in the node's result, not committed.
- [ ] Over `$RAKATA_BUILD`, the set of case-variant duplicates `rakata which` reports for Override equals what `collapse_case` reports.
- [ ] A test with a hand-made install (two files, two capsules) covers Override over module, `.mod` over `.rim`, and a BIF fallback.
- [ ] Timing in the result: open the full build cold, then resolve every name in Override and the modules; median per resolve printed (target under a millisecond after an open under 100 ms).
- [ ] `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Scenarios

```gherkin
Scenario: Two case variants in Override
  Given Override holds PLC_Invis.utp and plc_invis.utp with different hashes
  When plc_invis.utp is resolved
  Then both are candidates, the winner is marked ambiguous, and a diagnostic with a code names both paths
```

## Notes

- Do not read module IFOs or GFFs here; area lists wait for `rakata-gff`. Precedence among a module's capsules needs only their names.
- `loaded()` is ours and may be run; PyKotor's source may not be opened (**clean room**, `docs/proposals/rakata.md`). **No game bytes in the repository.** No home paths in committed files.
