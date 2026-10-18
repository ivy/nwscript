---
number: 89
title: "rakata-lua and `rakata lua SCRIPT`: the façade in Lua, hermetic"
kind: task
priority: p3
parent: m3-write
blocked_by: [authoring-api]
assignee_kind: agent
labels: [rakata, bindings, lua]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, Architecture (`rakata-lua`) and "Authoring" (available from Lua, hermetic, no interpreter to install). `crates/nwscript-lua/` is the precedent (`mlua`, `nws lua`, `nws test`, `crates/nwscript-lua/testing.md`); modshop's Juhani mod has a Lua test (`games/kotor/mods/juhani-clones/juhani_clones_test.lua`) run through `nws test`.

## Deliverables

- `crates/rakata-lua/`: tables mirroring the façade (install, formats, dialog with node keys, the authoring operations, diagnostics as error values with a code), the same shape and naming as the Python module.
- `rakata lua SCRIPT [ARGS...] --game DIR`: runs the script with the façade loaded and `--game` as the install; reads nothing not named.
- `crates/rakata-lua/api.md` documenting every table and function.

## Acceptance criteria

- [ ] A Lua script that opens the install, copies a `globalcat.2da` row with a changed label and writes the table to a temporary directory produces bytes identical to the same operations through the Rust API (test).
- [ ] An authoring error surfaces as a Lua error carrying the diagnostic code.
- [ ] `nws test` still runs the Juhani Lua test unchanged.
- [ ] `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Notes

- No `require` of files outside the script's directory and no network; hermetic means the script's inputs are its arguments and `--game`.
