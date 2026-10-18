---
number: 63
title: "rakata-tlk: dialog.tlk read, `LocalizedString` resolution, append with the returned strref, write"
kind: task
priority: p1
parent: m1-formats
blocked_by: [rakata-res, conformance]
assignee_kind: agent
labels: [rakata, tlk]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "GFF, 2DA, TLK and LIP". TLK V3.0 (documented on the KotOR and NWN wikis and in xoreos's documents) holds every string the game shows by strref, each entry with flags (text present, sound present, sound length present), a sound resref, volume and pitch variance and a sound length. Installing a mod appends strings and needs the strref back; the voice pipeline reads the sound resref per entry (modshop's `games/kotor/mods/juhani-clones/voice.py` borrows four of the game's lines by their voice names). A `CExoLocString` in a GFF resolves to text through the TLK when its strref is set, and through its own substrings otherwise.

## Deliverables

- `crates/rakata-tlk/`: `Tlk { language, entries }`, `Entry { text, sound: Option<ResRef>, sound_length, flags, volume_variance, pitch_variance }`, `get(StrRef)`, `append(Entry) -> StrRef`, `write` in BioWare's layout; `LocalizedString` (shared with `rakata-gff`'s `CExoLocString`, placed in `rakata-res` or here so both depend one way) with `resolve(&Tlk, language, gender) -> Option<&str>`.
- Registration with the conformance registry for `.tlk`.
- `Cargo.toml` row; `AGENTS.md` row present.

## Acceptance criteria

- [ ] `rakata conformance --format tlk` over `$RAKATA_GAME_K1`: `dialog.tlk` identical; over `$RAKATA_BUILD`: identical or a named variant.
- [ ] Strref 42724 in the build's TLK resolves to non-empty text (the area-transition refusal message modshop's `games/kotor/docs/ideas/inspect-cli.md` traced); the test prints it but commits nothing.
- [ ] `append` on a hand-made TLK returns the previous entry count; the written file re-reads with the new entry at that strref and the earlier entries unchanged.
- [ ] A `CExoLocString` with strref set resolves through the TLK; with strref unset and a substring for the language, resolves to the substring.
- [ ] `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Notes

- Text is Windows-1252 in KotOR's TLK, not UTF-8; decode and re-encode exactly, and keep bytes that are not valid in the code page as they were.
- The sound length and the string offsets are what break identical write-back; compute offsets from the preserved entry order.
- **Clean room** (`docs/proposals/rakata.md`, Clean room). **No game bytes in the repository.**
