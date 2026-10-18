---
title: "The release covers the workspace: the `rakata` binary, the `rakata` wheel and the plugin at one version"
kind: task
priority: p2
parent: "#9"
blocked_by: [rakata-py, rename-repo, approve-release-setup, "#45"]
assignee_kind: agent
labels: [rakata, release, ci]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, "Release readiness" and "Name, licence and repository": one workspace version, the first tagged release carries the scripts toolchain and the data layers together, `cargo-semver-checks` for the library crates from the first release, the plugin at the workspace version. `docs/prd.md`'s "Release readiness" lists the mechanism the first graph's release node (`nwscript#45`) builds for `nws`: `cargo-dist` on five targets, cosign, attestations, SBOM, trusted publishing, `release-plz`. This node extends that to the second binary, the wheels and the renamed repository. The owner approves the setup at the gate this node waits on; its completion result may name choices (which wheel targets, whether PyPI is in the first release). Follow them.

## Deliverables

- `cargo-dist` configuration covering `rakata` beside `nws` and `nwnnsscomp` for the same targets.
- Wheels for `rakata-py` (and `nwscript-py`) built in CI with maturin for Linux x86_64 and aarch64, macOS and Windows, attached to the release and published to PyPI under trusted publishing (the PyPI side is the owner's, as `nwscript#46` is for crates.io; say so in the result).
- `cargo semver-checks` in CI for every library crate.
- `plugin/` version equal to the workspace version, checked by a test; `Cargo.toml`'s `repository` and the marketplace URL at the renamed address.
- README: the Results section gains the data layers' conformance numbers (from the `conformance` node's generated table).

## Acceptance criteria

- [ ] `cargo dist plan` lists `nws`, `nwnnsscomp` and `rakata` for every target.
- [ ] The CI wheel job produces an installable `rakata` wheel on Linux (`pip install dist/*.whl && python -c "import rakata.nwscript"`).
- [ ] `cargo semver-checks` runs in CI and passes.
- [ ] A test asserts every crate's version and the plugin's version are equal.
- [ ] `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`, `cargo test --workspace` pass.

## Notes

- Do not tag; `nwscript#51` is the owner's gate for the first release.
- The repository name must already be `ivy/rakata` (the `rename-repo` gate), or the release URLs are wrong on the first tag.
