---
title: "Approve the release and CI setup for the data layers: cargo-dist targets, wheels, PyPI, semver checks"
kind: gate
priority: p2
parent: "#9"
blocked_by: [rakata-py]
assignee_kind: user
labels: [rakata, release, gate]
---

## Context

The owner wants oversight of build-system decisions: the release node (`release-workspace`) adds the `rakata` binary to `cargo-dist`, builds wheels for `rakata-py` and `nwscript-py` in CI, adds `cargo-semver-checks`, and may publish to PyPI under trusted publishing. `docs/prd.md` "Release readiness" and `docs/proposals/rakata.md` "Release readiness" describe the mechanism; `nwscript#45` builds it for `nws`; `nwscript#46` and `#48` are the gates for crates.io and for where conformance runs.

## What to do

- Read the two Release readiness sections and `nwscript#45`'s result.
- Decide: which wheel targets the first release carries; whether PyPI publishing is in the first release or later; whether `cargo-semver-checks` blocks CI or reports.
- Complete this node with those choices as the result; the release node reads them as its inputs.

## Done when

The completion result names the targets, the PyPI decision and the semver policy.
