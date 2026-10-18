---
number: 93
title: "Rename github.com/ivy/nwscript to ivy/rakata and repoint what names it"
kind: gate
priority: p2
parent: m0-workspace
blocked_by: [accept-prd]
assignee_kind: user
labels: [rakata, foundation, gate]
---

## Context

The proposal (`docs/proposals/rakata.md`, "Name, licence and repository") decides the project is `github.com/ivy/rakata`; GitHub redirects the old name. Renaming is outward-facing and the owner's act, and it settles the open question on the lore name first (same section, "A lore name"). Nothing a crate does depends on the name; the release does: `workspace.package.repository` in `Cargo.toml`, `cargo-dist`'s URLs, the plugin marketplace path under `plugin/`, and modshop's pin `cargo:https://github.com/ivy/nwscript` in `~/src/github.com/ivy/modshop/mise.toml`.

## What to do

- Decide the name (Rakata, or the neutral fallback); the README keeps the theme either way.
- Rename the repository on GitHub.
- Update the four places above, or ask an agent to, and rerun `mise install` in modshop to confirm `nws` still installs from the redirected or new URL.

## Done when

`gh repo view ivy/rakata` succeeds, `Cargo.toml`'s `repository` and modshop's `mise.toml` name the new URL, and `main` is pushed.
