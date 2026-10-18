# Handoff: the Rakata epic, paused 2026-10-18

This picks up the Rakata epic (`docs/epics/rakata.md`, filed in skift as `nwscript#53` onward) where it was paused so the owner's Odyssey-compatible Unreal Engine runtime could use the toolset. Read it with `skift node show '#N' -p nwscript` for any node it names: each node's result records what was produced, measured and decided.

## Where the epic stands

The epic's goal: Rakata reads and writes every data format a KotOR mod touches with proof it loses nothing, answers questions about the game in milliseconds, lets a mod author write scenes against a validating library, and modshop builds on Rakata's installer; measured by the Juhani mod rebuilt on the API to the same tree, and the Taris chapter of the Calo Nord mod written on it.

| Milestone | State |
| --- | --- |
| M0 workspace (#53) | done except the owner's rename gate (#93) |
| M1 formats (#57) | closed: archives, GFF, 2DA, TLK and LIP round-trip every vanilla file byte for byte (30,270 files in 0.43 s; README table) |
| M2 know (#56) | closed: corpus, dialogue transcripts, the index, `refs` |
| M3 write (#55) | the authoring API, Python and Lua bindings and the Juhani port done; the Calo Nord Taris chapter in progress |
| M4 saves (#84) | closed, confirmed on the handheld |
| M5 prove (#72) | closed: dialogue and tree lints, `rakata diff`, `modshop kotor lint` |
| M6 install (#76) | closed: modshop's patcher step runs `rakata install`; parity over 72 entries with no Rakata defect open |
| M7 release (#9) | waits on the owner's gates #70 and #93 |

The first measure is met (#83): the Juhani mod, ported onto the `rakata` Python module, rebuilds to the same tree (178 of 184 changed paths byte-identical, 5 dialogues equal in data, one Rhubarb keyframe 10 ms off and explained), reproducibly (tree `c3b7cb610403` on every Rakata build since). The second measure, the Calo Nord Taris chapter, is a test of the system the owner may rewrite later; its nine design questions were decided by the lead at the owner's direction (#107's result).

## Work done for OdysseyUE after the pause

The owner's Unreal Engine runtime (the OdysseyUE session, `~/src/github.com/ivy/odyssey-ue`) asked for the 3D asset crates, which landed on `main` as their own nodes, each closed with its API and census: #124 `rakata-texture` (TPC with TXI, TGA; mip data borrowed), #125 `rakata-lyt` (LYT and VIS; door-hook quaternions w x y z, measured), #126 `rakata-mdl` (static geometry; vertex attributes as stride-and-offset views into the MDX), #127 `rakata-walkmesh` (WOK, PWK, DWK; vertices already in area coordinates), #128 skins, animations with compressed quaternions, and supermodel chains in `rakata-mdl`. Each parses every vanilla instance with zero failures; the specs are in `docs/formats/`. OdysseyUE merges `main` into its own vendored branch; keep `main` stable for it, and build with at most four cargo jobs while it shares the host. Questions only the engine can answer (cube face directions, uncompressed TPC channel and row order, whether animation position keys are offsets from rest, three Endar Spire doors half a turn off their hooks) are in the nodes' results and the specs.

## The asset crates to the standard of the first formats (#129)

After the 3D asset crates landed, the owner asked for them to meet the engineering standard of 2DA, GFF, TLK and LIP, with conformance and benchmarks against other tools. That is skift group #129; each node's result records what it produced and measured. On `main`: writers proven byte for byte by `rakata conformance` for TPC, TXI, TGA, LYT, VIS, WOK, PWK, DWK and MDL with its MDX, over vanilla and the modded build (#130 to #133); the texture packs in `rakata-game`'s search order (#145); semantic diff (#141); the references inside models and layouts in the index and `rakata lint` RW1906 (#142); the Python and Lua bindings (#140); the inspection commands `rakata texture|lyt|walkmesh|mdl show` (#139); the benchmark harness, `cargo xtask bench` with `conformance/bench.toml` (#134). Fuzz targets for every rakata reader, `decode` and `roundtrip` in each crate's `fuzz/` (#143, which found and fixed ten defects); the agent skill's asset reference, trial and name check (#144). Two owner gates remain: #135 (the host is quiet, so #136 can take the measured run) and #137 (review the results before #138 publishes them); nothing is published in the README or `docs/research/` before #137.

## State of the repositories

- nwscript `main`: the 3D asset crates on top of e7a83e8 (see above). Clean. Not pushed (`origin/main` is c90437e; no history rewrite has been done, and four early commits of `scripts/clean-room-test.py` still carry the build host's home path in sample paths; the owner said it is not a big deal; rewrite before a first push only if they ask).
- modshop `main`: b65b6b2. Clean. Not pushed. The chain is up to date on Rakata 2fce656 (`mise run rakata:local` from `local.toml`'s `[rakata] checkout`); its final tree is `c3b7cb610403`.

## Unmerged branches, by node

Each is committed and builds; none is merged. `calo-a-fixes` and #122 are fully verified (the four checks with the game variables set); #121's and #123's full `cargo test --workspace` runs with the game variables were interrupted by the pause. To resume: rebase each onto `main`, run the four checks with `RAKATA_GAME_K1`, `RAKATA_BUILD` and `RAKATA_SAVES` set, merge, and complete the node with the agent's report (summarised here).

| Node | Branch (nwscript unless noted) | What it holds |
| --- | --- | --- |
| #121 decompiler naming | `worktree-agent-ab5c27b1b6beaeef0` (83fa95f, a0c7d18, b74a6b3) | folds pick the right library when several are byte-identical (`k35_trg_shaaswrd` now folds to `k_inc_utility`); include constant tables fold alone (`k37_ud_freedon` 195 to 41 lines; scripts with unnamed globals 1,588 to 483); the combat AI cannot fold (an older `k_inc_generic`); corpus verified fraction unchanged; fold CPU time up about 33% |
| #122 refs, show, output | `worktree-agent-ae53a1c6208581278` (ee9112b to fbba726) | `refs` and `show` read `resolved_calls` (written as schema 4, but `main` took schema 4 for the asset references of #142: renumber it to 5 when rebasing); `refs` grouped by resource (27 lines for the sword, down from 91); compact JSON for agents and `--pretty`; one notation for archive members (`ARCHIVE!MEMBER`, BIF members by KEY name); short hashes in text; a GIT-placed global blueprint's events count in its module. Also closes #119. This branch also contains the two `calo-a-fixes` commits (4a3f26a, bae497d): merge those once |
| #123 dialogue sites, quest | `worktree-agent-a5fe6576ddbe91ca8` (028e36d, 70d16b0, 67c0295, 26ab647) | dialogue sites written `E0->R33 [KEY]` everywhere through `rakata_dialog::site` and `rakata_index::Sites`; `quest` traces gates along the chain to each writer and a script write's guards; `--brief` keeps quest writes; `--node` decompiles only its nodes; `decompile A B C`; `quest TAG --scripts`. Its script-guard reading parses the decompiler's printed conditions, so merge it after #121 and rerun the sword quest test |
| Calo pieces' Rakata fix | `calo-a-fixes` (4a3f26a, bae497d), worktree `~/src/github.com/ivy/nwscript-calo-a` | `Gff.append(list_path, struct_id)` and `Gff.set(path, [], "List")` in Rust and Python, which journal entries in `tar_escape` need; verified by the lead before the pause (rebased on e7a83e8, 711 tests with `RAKATA_GAME_K1`, clippy and fmt clean), ready to merge |
| Calo pieces 4 to 6 (#111 to #113) | modshop `calo-a`, worktree `~/src/github.com/ivy/modshop-calo-a` | piece 4 (the unlock) built and committed (3bdfd79, checkpoint 9bddb8fd2c329e22, lints clean, diff exactly the piece); piece 5 (Calo's first two scenes, a2c3adf and README 2d68bab) and piece 6 (T3-M4, 9920dcd) committed and dialogue-linted clean against the branch's wheel over a scratch copy of the build, not yet built as checkpoints; piece 6's play test is written in the mod's README but has no save fixture, because there is no Taris save on this host and `rakata save set` cannot change the journal or the inventory (a tool gap worth a node); one modshop test, `test_deploy_refuses_while_game_runs`, fails with or without these changes because it expects a process named `sh`. Building a branch's entry from the main checkout repoints the shared `game/build` link; run `modshop kotor build` on `main` afterwards to point it back (done at the pause, chain end `1d641bb2961ec692`), because building needs `calo-a-fixes` merged and `mise run rakata:local` (which rebuilds the chain once) |

Merge order that avoids rework: `calo-a-fixes`, then #122 (which already contains it), then #121, then #123; then `mise run rakata:local` in modshop and one chain rebuild; then the Calo branch, building pieces 5 and 6. After each merge, `cargo clean --release -p nwscript-ncs` if a release build sees an older `nwscript-ncs` (a stale artifact seen once after fast-forward merges).

## What remains of the epic

1. Merge the branches above and complete #119, #121, #122, #123, #111, #112, #113.
2. The Calo chapter: #114 (codes and Bastila; its first step is the scene table's field for `Mod.write_module`, noted on the node), #115 (the join; it must set `G_Banter5` and `G_Banter6`), #116 (estate), #117 (hangar), #118 (aboard the Ebon Hawk), then #99 (the four play-throughs on save fixtures; the owner plays them on the handheld). The decisions are in #107's result; the plan, with every anchor checked, is modshop's `games/kotor/mods/calo-nord/plan/taris.md`.
3. A third pass of the Ajunta Pall research (`docs/research/2026-10-17-field-report-ajunta-pall.md`, `docs/research/2026-10-18-field-report-ajunta-pall-second-pass.md`) once the three branches are merged, as a fresh session with only the installed tools.
4. The owner's gates: #70 (release targets, PyPI, semver policy), #93 (the repository's name), and one open question from install parity: TSLPatcher appends every entry of a mod's `append.tlk`, Rakata and HoloPatcher only the entries a token names (`tlk-append-listed`, kept as Rakata's behaviour until the owner says otherwise).
5. The release (#94, #9) after the gates.

## Ways of working that held

- One worktree per agent; the lead rebases, runs the four checks with the game variables, merges by fast-forward, and completes the node with the agent's report as its result. Leases on this skift server lapse within the hour, so nodes are completed without a lease.
- Codes come in blocks of a hundred per crate or command family (`AGENTS.md`); the next free block is RE26xx.
- No host details in committed files: no absolute home paths, no `/tmp` session paths, no host names, no vault paths.
- The clean-room hook (`scripts/clean-room.py`, cases in `scripts/clean-room-test.py`) refuses tool calls naming a reference project; the format-scribe agent is the one exempt role, spawned without a `name`. A tool's logs, paths inside this repository and `docs/`/`conformance/` paths are not reads of a reference.
- Feedback from agents that use the tools (the Ajunta Pall reports) has been the best source of defects; file each report under `docs/research/` and its items as nodes.
