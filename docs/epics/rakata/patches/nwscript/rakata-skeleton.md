---
title: "The `rakata` façade crate and `rakata` binary: envelope, exit codes, `explain`, `--game`"
kind: task
priority: p1
parent: m0-workspace
blocked_by: [accept-prd]
assignee_kind: agent
labels: [rakata, foundation, cli]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, sections "Architecture" (the `rakata` façade and `rakata-cli` rows) and "Diagnostics, agent surface and command line", which requires the same JSON envelope, exit codes and `explain` as `nws`. Today those live in the `nws` binary: `crates/nws/src/output.rs` (`Output::emit`, one JSON document or rendered text), `crates/nws/src/cli.rs` (shared flags), `crates/nws/src/main.rs` (exit codes 0 Ok, 1 Errors, 2 Usage, 3 Different, 101 Internal), with the envelope type `nwscript::Envelope` and the code registry and renderers in `crates/nwscript-diag/`. The first graph built its CLI skeleton (`nwscript#13`) before any compiler crate so every later command shared one shape; this node does the same for data.

## Deliverables

- `crates/rakata/`: the façade crate, with the `rakata` diagnostic code registry (a prefix distinct from the nwscript codes, registered through `nwscript-diag`'s mechanism; if that registry only accepts one crate's codes, open it in `nwscript-diag` with a test) and the `Envelope` shared with `nws`. If the envelope type cannot be shared without `nwscript` depending on `rakata`, move it into `nwscript-diag`; the `nwscript-*` library crates must not depend on any `rakata-*` crate.
- `crates/rakata-cli/`: the `rakata` binary with `--format text|json`, `--color`, `--quiet`, `--game DIR` (no default; a command that needs it and lacks it exits 2 naming the flag), `explain CODE`, `--version`.
- Build identity: `rakata --version` prints the workspace version, the git describe of the checkout it was built from, and a short hash of the working tree when it was dirty (a `build.rs` in `rakata`, so the Python module can print the same string). This is what modshop keys its entries on when it uses a local build, so a change in the checkout changes the key without a push or a release. Shared flag and output code lifted out of `nws` into a module both binaries use, or duplicated minimally with a test that keeps the two exit-code tables equal.
- `Cargo.toml`: `[workspace.dependencies]` rows for `rakata` and `rakata-cli`.
- `AGENTS.md`: the two rows change from "planned" to present.

## Acceptance criteria

- [ ] `cargo run -p rakata-cli -- explain <one registered rakata code>` prints its explanation; with `--format json` it prints one JSON document whose top-level keys equal those of `nws --format json explain <nwscript code>`.
- [ ] A test asserts the `rakata` exit-code enum has the same values as `nws`'s.
- [ ] `cargo run -p rakata-cli -- which x.y` (or any command requiring `--game`) exits 2 and names `--game`.
- [ ] `cargo tree -p nwscript -e normal | grep rakata` prints nothing; the same for every `nwscript-*` library crate.
- [ ] `rakata --version` from a clean checkout prints `0.0.0` and the commit; from a checkout with one file changed it prints a different string (test by building twice in a temporary clone).
- [ ] The `nws` CLI tests still pass unchanged; `cargo clippy --workspace -- -D warnings` and `cargo fmt --check` pass.

## Notes

- Register no commands beyond `explain` here; each crate node adds its own. Do not list planned commands in `--help`.
- The workspace forbids `unsafe_code`; keep it that way.
