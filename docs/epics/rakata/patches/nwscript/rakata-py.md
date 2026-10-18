---
title: "rakata-py: the Python module `rakata` with `rakata.nwscript` inside it, built with maturin"
kind: task
priority: p1
parent: m3-write
blocked_by: [authoring-api]
assignee_kind: agent
labels: [rakata, bindings, py]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, Architecture (`rakata-lua`, `rakata-py`): the Python module is `rakata` with `rakata.nwscript` inside it, and `nwscript-py` folds into it. `crates/nwscript-py/` is the precedent: PyO3 through maturin (`pyproject.toml`, module `nwscript`), consumed by modshop (`nwscript#42`). modshop runs a mod's script as `sys.executable -I script args` from its `Script` step (`~/src/github.com/ivy/modshop/src/modshop/kotor/steps.py`, `class Script`), so the module must be importable from an interpreter that has it installed and nothing else.

## Deliverables

- `crates/rakata-py/` with `pyproject.toml` (project name `rakata`, maturin, `module-name = "rakata"`), classes mirroring the façade: `Install`, `Resolution`, `Gff` with path get/set, `TwoDa`, `Tlk`, `Lip`, `Dialog` and `NodeKey`, `Mod` with the authoring operations, and diagnostics raised as an exception type carrying `.code` and the rendered message.
- `rakata.nwscript`: the `nwscript-py` API mounted as a submodule (the `nwscript-py` crate stays, so `import nwscript` keeps working for modshop until it switches; `rakata.nwscript` is the same code, not a copy).
- Type stubs (`rakata/__init__.pyi`, `rakata/nwscript.pyi`) shipped in the wheel.
- `rakata.__version__` equal to `rakata --version`'s build identity string (version, git describe, dirty-tree hash), from the same `build.rs` source; modshop keys on it when the module comes from a local checkout.
- Tests under `crates/rakata-py/tests/` run with pytest after `maturin develop` in a `uv` environment; a `mise` task or a documented command to run them.

## Acceptance criteria

- [ ] In a fresh `uv venv` with `maturin develop` run, `python -c "import rakata, rakata.nwscript"` succeeds.
- [ ] `rakata.Install(path).which("globalcat", "2da").hash` equals `rakata which globalcat.2da --format json`'s hash over `$RAKATA_GAME_K1`.
- [ ] An authoring error (a node key that matches nothing) raises the exception with the same `.code` the CLI prints.
- [ ] `pyright` over the test files passes with the stubs.
- [ ] `python -c "import rakata; print(rakata.__version__)"` prints the same string as `rakata --version` built from the same tree.
- [ ] `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Notes

- Keep the Python API a mirror of the Rust façade: the same names and the same operations, so the skill documents one surface. Do not add Python-only conveniences here.
- The wheel is built in CI by the release node; this node makes it build locally.
