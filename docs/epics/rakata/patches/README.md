# Rakata: the data layers, in the order the Calo mod needs them

The decomposition of [../../rakata.md](../../rakata.md). Forty-four nodes in one skift project, `nwscript` (the slug stays when the repository is renamed; only `vcs_url` changes): seven milestone grouping nodes, three gates, one spike and 33 tasks, with three nodes parented to the existing release milestone `#9`. Frontmatter is the op, the body is the work order. Filed on 2026-10-17 as `nwscript#53` through `nwscript#96`; each file's `number` key names its node.

Agents receive a node body and nothing else, so each body restates what its reader needs and cites files in the repositories: `docs/proposals/rakata.md` and `docs/prd.md` here, modshop's `CLAUDE.md`, `games/kotor/CLAUDE.md`, its ideas and patches READMEs, and `docs/proposals/rakata-in-modshop.md` once the `modshop-proposal` node writes it. No body names this epic.

## Before applying

What had to hold when the patch was applied; it did.

- **Commit the proposal.** `docs/proposals/rakata.md` is uncommitted; `accept-prd` is the node that commits it as accepted and is the root of the graph. Applying before it exists on `main` is fine; dispatching any other node before `accept-prd` is done is not, because every body cites that file.
- **The first graph is the ground.** `#9` (M7 Release), `#45` (release workflow), `#46`, `#48`, `#51` (its gates) are referenced by number and resolve to the filed nodes. `release-workspace`, `approve-release-setup` and `rakata-skill` take `parent: "#9"`, so that milestone now closes only when the data layers' release work does; a comment on `#9` saying so is worth leaving when this is applied. `#50` (fresh-agent test for scripts) is untouched; `rakata-skill` is its counterpart for data.
- **Three gates, each preceded by the node that makes it easy.** `rename-repo` (after `accept-prd`; blocks only the release); `approve-release-setup` (after `rakata-py`; blocks `release-workspace`); `save-loads` (after `save-set`; blocks nothing, and closes M4). The owner's rule: CI/CD and release publishing decisions are theirs; everything inside modshop, including switching its patcher once parity holds and how a local build reaches it, is implementation and is not gated. No modshop node waits on a release.
- **Nodes labelled `modshop` change `~/src/github.com/ivy/modshop`**, not this repository: `modshop-proposal`, `juhani-port`, `modshop-check`, `install-parity`, `modshop-install-step`. Their bodies name modshop's rules (no mod authors' files or build outputs in git, no device identity, nothing edited in a game folder by hand). Two of them (`juhani-port`, `modshop-install-step`) cause a chain rebuild by design; one commit each.
- **Game trees and saves are on this host only.** `accept-prd` documents `$RAKATA_GAME_K1` and `$RAKATA_BUILD` in `AGENTS.md`; every test needing them skips with a message when unset. `rakata-save` reads `$RAKATA_SAVES`. Nothing from them is committed.
- **Clean room applies to every node.** PyKotor (all of it), HoloPatcher, kpatcher (`~/src/github.com/th3w1zard1/kpatcher`, cloned on 2026-10-17 and still present) and TSLPatcher's source are off limits; TSLPatcher may be run. codebase-memory-mcp may be read for design; nothing is copied from it.
- **`unsafe` has one sanctioned exception**, in `rakata-archive`, earned by a measurement the node records; the workspace lint stays `forbid`.
- **Files several nodes touch, sequenced by the graph:** `Cargo.toml` and `AGENTS.md` (every crate node adds a row; they are independent edits to different rows, and conflicts are mechanical); `crates/rakata/` (the skeleton creates it, the conformance, authoring and index nodes add modules); `docs/lints.md` (`dialog-lint`, then `tree-lint`); `README.md` (`accept-prd`, `conformance`, `release-workspace`).

## The shape

```
accept-prd ─┬─ rakata-skeleton ─┬─────────────────────────────────────── conformance ─┬─ rakata-gff ──┬─ dialog-model ─┬─ index-build ── refs
            ├─ rakata-res ──────┴─ rakata-archive ──┬─ (conformance)                  ├─ rakata-2da ──┤                 │
            ├─ modshop-proposal                      └─ rakata-game ──┬─ nws-game-dir  ├─ rakata-tlk ──┤   corpus ───────┘
            └─ rename-repo (gate)                                     ├─ corpus         └─ rakata-lip ──┤
                                                                      └─ (dialog-model, authoring-api, tree-lint, patch-exec, rakata-save)

dialog-model, 2da, tlk, lip, game ── authoring-api ─┬─ rakata-py ─┬─ juhani-port (+ modshop-proposal) ── modshop-check (+ dialog-lint, tree-lint)
                                                    ├─ rakata-lua  ├─ approve-release-setup (gate) ── release-workspace (+ rename-repo, #45)
                                                    │              └─ rakata-skill (+ format-commands, dialog-model, refs)
                                                    └─ dialog-lint (+ corpus)
gff, 2da, tlk, lip, game ── format-commands
index-build, game ── tree-lint
gff, 2da, tlk, archive, dialog-model ── semantic-diff
archive, gff, game ── rakata-save ── save-set ── save-loads (gate)
tslpatcher-oracle (spike, no dependencies)
rakata-skeleton ── patch-reader ── patch-exec (+ gff, 2da, tlk, archive, game) ── install-parity (+ semantic-diff, tslpatcher-oracle) ── modshop-install-step
```

Every task has `parent` set to its milestone (`m0-workspace` … `m6-install`) or to `#9`, so the grouping nodes close when their children do.

| Kind | Count |
|---|---|
| `milestone` grouping nodes | 7 (`m0-workspace`, `m1-formats`, `m2-know`, `m3-write`, `m4-saves`, `m5-prove`, `m6-install`) |
| `gate` (`assignee_kind: user`) | 3 (`rename-repo`, `approve-release-setup`, `save-loads`) |
| `spike` | 1 (`tslpatcher-oracle`) |
| `task` | 33 |
| edges to nodes already filed | 2 (`#45` → `release-workspace`; `parent: "#9"` on three nodes) |

**Ready on the first day**: `accept-prd` and `tslpatcher-oracle`. Two nodes, and that is right: every body cites the accepted proposal, so nothing else can start before it is in the tree, and the spike has no dependency at all. The day after, four more: `rakata-skeleton`, `rakata-res`, `modshop-proposal`, and the gate `rename-repo`.

The long pole is `accept-prd → rakata-res → rakata-archive → conformance → rakata-gff → dialog-model → authoring-api → rakata-py → juhani-port`: nine nodes to the proof that the authoring API is complete. `rakata-game`, `corpus`, `patch-reader` and `rakata-save` branch off early and run beside it. The installer's chain (`patch-reader → patch-exec → install-parity → modshop-install-step`) is the longest in wall-clock terms and the least urgent; it is why the spike is ready on day one.

## Sizing

One crate or one command cluster each, under eight acceptance criteria and six scenarios. The largest are `patch-exec` (seven sections sharing tokens, order and the read-back report; split further only if the reader node's census shows a section none of the archives use), `index-build` (the schema, the extractors' wiring and the incremental proof) and `authoring-api` (the Juhani mod's whole operation set, each with a fixture). `accept-prd` and `modshop-proposal` are documents nodes and the ones that must land first, because every later body cites what they write. The gates are small by design: each reads one result and records one decision.

Session cost: none of these nodes spends tokens in the work itself. Several spend wall time on the build host: the conformance and index runs over the full build, the corpus decompile of 11,355 scripts, the parity run over every patcher step (each a reflink copy of a checkpoint and an install), and the two chain rebuilds in modshop (about 25 minutes each).

## What this proves when it lands

The proposal's vision for the Calo mod's first chapter: an agent asks the game anything about itself in milliseconds and gets an answer that cites the resource (`which`, `dialog show`, `refs`, the Canderous report as one query); it changes the game through an API that refuses to write what the game cannot load, and the Juhani mod, ported onto that API with its code reduced to its scenes, rebuilds to the same tree; it proves before deploy that every reference resolves, every voiced line has a file and every node is reachable, and shows a checkpoint's change by field; a late scene is a load, not a replay; and the build's installer is ours, its report read back from the tree, with every defect the patch series carried now a test. Every number the project claims about itself is in the README, generated by the suite that measured it.
