---
number: 67
title: "rakata-lip: lip-sync files, the sixteen mouth shapes with their times"
kind: task
priority: p2
parent: m1-formats
blocked_by: [conformance]
assignee_kind: agent
labels: [rakata, lip]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "GFF, 2DA, TLK and LIP". LIP V1.0 (documented on the KotOR wiki) is a duration in seconds and a list of keyframes, each a time and one of sixteen mouth shapes. The game's own are members of `lips/*.mod`; modshop's Juhani mod writes 80 of them into `streamwaves/` from Rhubarb Lip Sync's cues (`games/kotor/mods/juhani-clones/voice.py`, through PyKotor today), and the authoring API will take cues and produce the file.

## Deliverables

- `crates/rakata-lip/`: `Lip { length: f32, keys: Vec<Key { time: f32, shape: Shape }> }`, `Shape` as an enum of the sixteen values, read, write, `from_cues(impl Iterator<Item = (f32, Shape)>, length)`.
- Registration with the conformance registry for `.lip`.
- `Cargo.toml` row; `AGENTS.md` row present.

## Acceptance criteria

- [ ] `rakata conformance --format lip` over `$RAKATA_GAME_K1`: every member of `lips/*.mod` identical; count in the README table.
- [ ] Over `$RAKATA_BUILD`: the `streamwaves/oaj_*.lip` files classify identical or under a named variant; zero "different".
- [ ] A hand-made file with three keys round-trips; a shape value above 15 is a diagnostic, not a panic.
- [ ] `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Notes

- Keep floats as read for write-back; `from_cues` is the only place a float is computed.
- **Clean room** (`docs/proposals/rakata.md`, Clean room). **No game bytes in the repository.**
