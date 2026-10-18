---
title: "`rakata cat`, `gff show|set|diff`, `2da show|diff`, `tlk show|append`, `lip show`"
kind: task
priority: p2
parent: m1-formats
blocked_by: [rakata-gff, rakata-2da, rakata-tlk, rakata-lip, rakata-game]
assignee_kind: agent
labels: [rakata, cli]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "Diagnostics, agent surface and command line", which lists these commands and requires each to be one library call with the shared envelope and exit codes (see `crates/rakata-cli/`). modshop's `games/kotor/docs/ideas/inspect-cli.md` records the questions these answer today by one-off script: a module's resources, a field of a blueprint, two tables by cell.

## Deliverables

- `rakata cat NAME.EXT --game DIR [--module M]`: the loaded copy's bytes to stdout (`rakata which` decides which); `--from FILE` for a file or archive member.
- `rakata gff show FILE|NAME.EXT [--path P]` (tree as text, or JSON), `gff set FILE PATH=VALUE... -o OUT`, `gff diff A B`.
- `rakata 2da show FILE|NAME.EXT [--row LABEL] [--column C]`, `2da diff A B`.
- `rakata tlk show STRREF|--text PATTERN [--game]`, `tlk append FILE --text T [--sound R] -o OUT` printing the strref.
- `rakata lip show FILE`.
- Each with `--format json`; diffs exit 3 when different; `--help` complete enough that the skill can cite every flag.
- CLI tests with hand-made fixtures under `crates/rakata-cli/tests/`.

## Acceptance criteria

- [ ] `rakata gff show dan14_juhani.utc --game $RAKATA_BUILD --module danm14ac --path Tag` prints the blueprint's tag.
- [ ] `rakata 2da diff` between vanilla's and the build's loaded `globalcat.2da` lists added rows including `OAJ_SPECIAL` and `OAJ_DONE` (the Juhani mod's globals) and exits 3.
- [ ] `rakata tlk show 42724 --game $RAKATA_BUILD` prints the text; `--format json` wraps it in the envelope.
- [ ] `gff set` then `gff show --path` round-trips a value on a fixture; the written file is otherwise identical to the input (checked with `gff diff`).
- [ ] Every command's `--help` is covered by a test that runs it; `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Notes

- Values on `gff set` need a type: take `PATH=TYPE:VALUE` or infer from the existing field and refuse to create a field without a type.
- Nothing here walks a tree or builds an index; that is `rakata refs`.
- **Clean room** (`docs/proposals/rakata.md`, Clean room). **No game bytes in the repository.**
