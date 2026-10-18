---
number: 58
title: "rakata-res: `ResRef`, the resource type table, content hashes"
kind: task
priority: p1
parent: m0-workspace
blocked_by: [accept-prd]
assignee_kind: agent
labels: [rakata, res]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "Architecture" (`rakata-res`) and "Resources and the install". Resource identity today is strings: `xtask/src/archive.rs` lowercases resrefs and knows two type ids (`NSS = 2009`, `NCS = 2010`). Every other crate needs one type for a name, one table from type id to extension, and one content hash so an index, a cache and modshop's build keys agree on identity. modshop's checkpoint manifests (`games/kotor/data/game/ckpt/<key>/manifest.txt`) are SHA-256 per path, which is why the hash is SHA-256.

## Deliverables

- `crates/rakata-res/`: `ResRef` (at most sixteen bytes; `Eq`, `Hash`, `Ord` case-insensitive; keeps the spelling it was read with for writing back; `Display` as read), `ResType` (id to extension and back for every type KotOR 1 ships; unknown ids kept as `ResType::Unknown(u16)`, never rejected), `ResId { name: ResRef, ty: ResType }` with parsing of `name.ext`, and `ContentHash` (SHA-256, hex lowercase, `Display`).
- The type table from the KotOR wiki's and xoreos's resource type documentation, with a test over `$RAKATA_GAME_K1` enumerating every type id present in `chitin.key` and every module archive and asserting each maps to an extension (skips, naming the variable, when unset).
- `Cargo.toml` workspace row; `AGENTS.md` row present.

## Acceptance criteria

- [ ] `ResRef::new("Carth") == ResRef::new("carth")` and both hash equal; `to_string()` of the first is `Carth`.
- [ ] A seventeen-byte name is an error carrying a registered diagnostic code.
- [ ] The vanilla type census test passes with zero `Unknown` ids, and its output lists the ids found.
- [ ] `ContentHash::of(bytes)` equals `sha256sum` for a fixture file (test).
- [ ] No `unsafe`, no I/O in the crate; `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Notes

- **Clean room** (`docs/proposals/rakata.md`, Clean room): do not open PyKotor's source (any checkout, modshop's `.venv`), HoloPatcher's, kpatcher's (`~/src/github.com/th3w1zard1/kpatcher`) or TSLPatcher's, and do not query codebase-memory against them. Permitted: the game's own files, the xoreos format documents, the KotOR and NWN wikis' format pages.
- **No game bytes in the repository**: the census is a test over the user's tree, not a committed list of resources.
