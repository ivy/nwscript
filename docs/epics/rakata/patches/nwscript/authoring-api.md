---
number: 68
title: "The authoring API: the Juhani mod's operations as validated library calls on the façade"
kind: task
priority: p1
parent: m3-write
blocked_by: [dialog-model, rakata-2da, rakata-tlk, rakata-lip, rakata-game]
assignee_kind: agent
labels: [rakata, authoring, dialog]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, section "Authoring". The operations are those of modshop's Juhani mod, `~/src/github.com/ivy/modshop/games/kotor/mods/juhani-clones/make.py` (read it): find the copy the game loads; guard a replaced resource by its SHA-256 and stop if any copy differs or Override already holds one; clone a blueprint with fields changed; add a row to `appearance.2da` copying another row with changed cells; declare globals in `globalcat.2da`, stopping if one exists; add entries, replies, links and starts to a dialogue at the ends of its lists so no existing index moves; make LIP files from Rhubarb's cues; compile scripts with `nws` against the build's includes; write to `Override/` and `streamwaves/`. The proposal requires every write validated by the same reference checks the lints will apply, so a mod cannot write a node whose script does not exist without being told.

## Deliverables

- `crates/rakata/src/author/`: `Mod::open(&Install, module)`; `loaded(ResId)`; `guard_replaced(ResId, ContentHash)`; `twoda::add_row_copying(table, from_label, new_label, cells)`; `globals::declare(name, GlobalType)`; `blueprint::set(gff, path, value)`; `dialog::{add_entry, add_reply, link, start}` taking `NodeKey`s, append-only, and `dialog::reorder` as a separate named operation; `lip::from_cues`; `scripts::compile(sources, flags)` through the `nwscript` façade with the install as resolver; `write_override`, `write_streamwaves`.
- `rakata_dialog::check`: the reference checks (a script or condition resolves through the install; a voice resref has its WAV; a strref is below the TLK's length; a global read by a condition is declared) returning diagnostics with codes; `author` runs them before every write and refuses on an error.
- Voice: `spans(lines) -> Vec<Span>` and `take_cues(cues) -> Lip`, with a cache key of take hash and span, so the caller cuts audio with ffmpeg and runs Rhubarb once per line; the two external programs stay outside the library.
- `rakata explain` entries for every new code; `cargo doc` for the module.

## Acceptance criteria

- [ ] A hand-made fixture install exercises each operation; each has a test asserting the written bytes.
- [ ] Writing a dialogue node whose `Script` does not resolve fails with a code naming the script and the node key; the file on disk is unchanged.
- [ ] A `NodeKey` with no match or two matches fails naming the key (the dialog crate's diagnostic, surfaced unchanged).
- [ ] After `add_entry`, every pre-existing node keeps its index (test over a fixture with ten nodes).
- [ ] `guard_replaced` fails when one of two capsule copies differs from the given hash, naming the capsule.
- [ ] `cargo doc -p rakata --no-deps` has no warnings; `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Scenarios

```gherkin
Scenario: Declaring a global that exists
  Given a globalcat.2da already declaring OAJ_DONE
  When globals::declare("OAJ_DONE", Boolean) runs
  Then it fails with a code naming OAJ_DONE and the table is unchanged
```

## Notes

- Do not port the Juhani mod here; that is the `juhani-port` node, and it is the proof this API is complete.
- `make.py`'s `clone_blueprint` and `check_replaced` are the specification of the semantics; its PyKotor calls are not to be mirrored, only their effect.
- **Clean room** (`docs/proposals/rakata.md`, Clean room). **No game bytes in the repository.**
