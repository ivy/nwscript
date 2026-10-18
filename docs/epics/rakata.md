---
date: 2026-10-17
type: epic
status: draft
project: "[[nwscript]]"
tags: [rakata, nwscript, modshop, formats, index, dialog, save, patch]
---

# Rakata: the data layers of the toolset, in the order the Calo mod needs them

Mini-PRD. The workspace at `~/src/github.com/ivy/nwscript` gains the data layers that [`../proposals/rakata.md`](../proposals/rakata.md) specifies, as `rakata-*` crates beside the `nwscript-*` ones and one `rakata` binary beside `nws`: resource identity and archives with a round-trip suite from the first week, the four formats a mod edits, an install model that answers "which copy loads", a corpus and an index that make "how does the game already do this" a query, an authoring API with stable keys that the Juhani mod is ported onto, a save reader, lints and a semantic diff, and last a `changes.ini` installer whose report is the tree. What that buys is the proposal's vision for the owner's next mod: the Taris chapter of the Calo Nord mod written against a library that answers in milliseconds, refuses to write what the game cannot load, and proves before deploy that nothing dangles.

## Outcome

By the end of this epic, Rakata reads and writes every data format a KotOR mod touches with proof it loses nothing, and answers any question about the game in milliseconds. A mod author with a coding agent writes a scene against a library that addresses the game by what things are, refuses to write what the game cannot load, and shows before deploy that nothing dangles. modshop builds the game on Rakata's installer with a report it can trust, consuming each change straight from the checkout. The measure is the Juhani mod rebuilt on the API to the same tree, and the Taris chapter of the Calo Nord mod written on it.

## Background

The proposal was written on 2026-10-17 and is uncommitted at `docs/proposals/rakata.md` (274 lines, `git status` shows `?? docs/proposals/rakata.md`). Its evidence is in modshop: the Juhani mod's authoring code is 253 lines of plumbing in `games/kotor/mods/juhani-clones/make.py` (its `loaded()` and `module_copies()` reimplement the engine's search order) plus `dialog.py` (129), `scenes.py` (342) and `voice.py` (105), every one of them against PyKotor's object model; the ideas index at `games/kotor/docs/ideas/README.md` ranks device-state, script-corpus and inspect as p0 by costs paid; `games/kotor/patches/README.md` lists eight PyKotor defects met in one build, two found by playing. The HoloPatcher repository the PyKotor monorepo points to and the reconstructed TSLPatcher source both returned 404 on 2026-10-17.

What exists in the workspace today: twelve crates under `crates/` (`nwscript-{ncs,vm,syntax,sema,codegen,decomp,diag,lua,py}`, `nwscript`, `nws`, `nwnnsscomp`), an `xtask` whose `archive.rs` (159 lines) reads KEY V1, BIFF V1, RIM V1.0 and ERF/MOD/SAV V1.0 well enough to extract 11,355 module scripts from 119 archives, a `Resolver` trait at `crates/nwscript/src/resolve.rs:19` with no install-backed implementation, a diagnostics crate with a code registry and JSON rendering, and a `nws` binary whose exit codes (0, 1, 2, 3, 101) and envelope every new command must share. The skift graph for the first PRD (`nwscript#1`–`#53`) has M0 to M6 done and M7 Release open with its gates (`#46`, `#48`, `#51`); nothing has been released.

The data the suites need is on the build host and never in the repository: a vanilla KotOR 1 tree at `modshop/games/kotor/data/game/vanilla/tree`, the full modded build at `modshop/games/kotor/data/game/build` (a symlink into `game/ckpt/<key>/tree`; 1,609 checkpoint directories, each with `manifest.txt` and `changed.txt`), 67 mod archives as zips under `data/archives/<entry>/` plus other formats, two saves under `data/saves/`, and the corpus at `~/.cache/nwscript/corpus/k1`.

**Why now.** The Calo mod's research starts with "every node, gate and call that names Canderous", a question asked hundreds of times across twenty-plus modules; every week without an index it is answered by scans, and every scene written before the authoring API exists is more hand-translated plumbing to port later. The installer can wait, because the current one works with patches; the knowing and writing layers cannot, because the mod is being written now.

## Goals

1. **Every resource the games ship round-trips, or its exception is named.** The counterpart of `nws conformance` runs over the vanilla install and the full build and classifies every file.
2. **"Which copy does the game load" has one answer in one crate**, and every command, the index and the authoring API ask it.
3. **The game's own precedent is a query, not a scan.** A decompiled corpus and an index over dialogues, scripts' calls, blueprints, tables and strings answer references in every direction in milliseconds.
4. **A mod addresses what it changes by what it is**, and a build fails with a named key when the base moved.
5. **A save answers a diagnosis in one command**, and a written save turns a late scene into a load.
6. **Faults play has found are found before deploy**: dangling references, voiceless lines, unreachable nodes, undeclared globals, case-variant duplicates.
7. **The installer's report is true**, because it is read back from the tree, and every defect in modshop's patch series is a regression test here.
8. **All of it is one façade** with a command line, Python and Lua, with the same envelope and exit codes as `nws`, and a skill that teaches the workflow.

## Non-goals

- **A graphical tool.** The crates do no I/O and own no process state so one can follow; none is built here.
- **Models, textures, walkmeshes, audio contents.** Hashed and header-read for provenance and diff only. The lints that would need their contents (power-of-two textures, emitter controllers) wait for a cost paid after this epic.
- **Simulating the engine.** A reachability check over dialogue conditions is static analysis through the scripts toolchain's interpreter and says what it could not decide; it is in Later, not here.
- **Reproducing TSLPatcher's defects.** Its behaviour defines `changes.ini`; where it is wrong about the game's formats, the suite records the difference and the installer does what the game needs.
- **Reading PyKotor's, HoloPatcher's, kpatcher's or TSLPatcher's source.** Clean room for every crate, as the proposal says; the cost is re-deriving fifteen years of `changes.ini` practice from the archives and from TSLPatcher run as a black box.
- **A declarative scene format.** Decided after two mods have used the Python API, not before.
- **KotOR 2.** A profile, measured later; the game-independent layers carry no KotOR 1 assumption, which the first epic's profile mechanism already provides.

## Requirements

Numbered so a node body can cite one. Each is one testable sentence; the proposal's sections carry the reasoning.

### Workspace and surface

- **R1** The proposal is the accepted design, with its status line saying so, and the README's first paragraph describes the toolset, not only the scripts toolchain.
- **R2** New crates are `rakata-*` and share the workspace version; `nwscript-*` library crates depend on no `rakata-*` crate.
- **R3** The `rakata` binary emits the same JSON envelope, exit codes and `explain` behaviour as `nws`, through the same diagnostics crate.
- **R4** `--game` has no default and nothing is read that was not named.
- **R5** The repository holds no game bytes: manifests are names and hashes, fixtures are hand-assembled.

### Resources and the install

- **R6** `ResRef` compares case-insensitively and holds at most sixteen bytes; the resource type table maps ids to extensions for every type KotOR 1 ships.
- **R7** Every resource is hashed by content so an index, a cache and a build key agree on identity.
- **R8** KEY/BIF, ERF in its variants and RIM read from a memory map without copying a member until asked, and write back byte-identical where the source is BioWare's.
- **R9** Adding or replacing a member in an archive produces an archive that holds the new bytes under that name and nothing else changed.
- **R10** Resolution follows the engine: `Override/` with its case rules, then the module's capsules in the engine's precedence, then the KEY's BIFs; every candidate is reported with the winner and why.

### Formats

- **R11** GFF reads every field type the engine uses, preserves struct and field order so a write reproduces the file, names a field by a path syntax, and diffs two trees by path.
- **R12** 2DA reads the header variants the engine accepts, including a zero cell-data size, and writes BioWare's layout.
- **R13** TLK appends with the returned strref and exposes each entry's sound resref.
- **R14** LIP reads and writes the sixteen shapes with their times.
- **R15** `rakata conformance` over a named tree classifies every resource as identical, equal after re-read (variant named) or different, and the headline numbers are generated into the README.

### Know

- **R16** `rakata corpus` decompiles every module script with a header naming its origin and loaded copy, folds include libraries out, and writes a call index of routine names with literal arguments.
- **R17** A dialogue is a model over GFF with every node's speaker, listener, text or strref, voice, scripts, conditions with `IsChild`, delay, animations with participants, camera, quest and comment, addressable by voice resref, strref or text.
- **R18** `rakata dialog show` renders a transcript with each condition decompiled to source and marked verified or not.
- **R19** The index holds facts, references and full text over a tree in SQLite, keyed by content hash, built incrementally from a list of changed paths, with provenance to the first tree a hash appeared in.
- **R20** `rakata refs THING` answers for a strref, resref, tag or global in every direction and names the resource, where it loads from, and the field or instruction.

### Write

- **R21** The authoring API performs the Juhani mod's operations as library calls, addresses dialogue nodes by stable key, appends by default, and fails with a diagnostic naming the key when the base has no such node or more than one.
- **R22** Every write is checked for references that resolve to nothing before it lands, with the same checks the lints apply.
- **R23** The façade is available as the Python module `rakata` with `rakata.nwscript` inside it, and as Lua through `rakata lua`.

### Saves

- **R24** `rakata save show` prints a save's globals, party, and each creature's tag, `Commandable`, faction, effects, local variables (by `SW_FLAG_*` name where the includes name them) and action queue, by name.
- **R25** `rakata save set` writes a save whose named globals or party differ from a read one and is byte-identical elsewhere.

### Prove

- **R26** `rakata dialog lint` and `rakata lint` report dangling scripts and resrefs, strrefs past the TLK, voiceless voiced nodes, unreachable nodes, undeclared globals, `SoundExists` disagreements, case-variant duplicates and `.tpc` shadows, each with a stable code.
- **R27** `rakata diff` compares two trees or two resources by field, cell, string, member or dialogue node, and by hash and header for formats without a crate.

### Install

- **R28** `changes.ini` and `namespaces.ini` are read with TSLPatcher's semantics, including a missing section skipped and `Required` without `RequiredMsg`, and every ini in the build's archives parses or names the construct it cannot.
- **R29** The executor runs the sections in TSLPatcher's order, compiles through `nwscript`, and after every operation re-reads what it wrote and reports the hash found.
- **R30** Over every HoloPatcher step of the full build, `rakata install` produces the current tree or explains each difference against TSLPatcher run under Wine on the same inputs.

### Release

- **R31** The first signed release carries both binaries, the Python wheel and the plugin at one version, and the skill passes a fresh-agent test with a data change.

## Design choices

**The façade crate and the `rakata` binary exist from milestone 0, and each crate node adds its API to them.** The first graph did the same (`#13`, the CLI skeleton, landed before any compiler crate) and every later command shared one envelope. Rejected: build the format crates first and write the façade in milestone 3, when the authoring API needs it. That order means `rakata which` and `rakata conformance` are written against crates directly and rewritten against the façade later, and the JSON envelope is two shapes for a quarter.

**The round-trip harness lands before any format crate, and every format node's acceptance is a number from it.** The defect class the build paid most for is a reader that drops data silently (`patches/README.md` item 3: a 2DA read as empty and written back empty). Rejected: per-crate unit tests on hand-made fixtures, with the suite later. Fixtures hold the cases their author thought of; the full build holds what every community writer produced, and only a walk over it finds the variant nobody thought of.

**The GitHub rename is a gate that blocks only the release.** Nothing a crate does depends on the repository's name; the release does (`workspace.package.repository`, `cargo-dist`'s URLs, modshop's mise pin). Rejected: rename first. A person's act at the root of the graph stalls every agent node behind it. Rejected: not renaming. The proposal decides the name, and a toolset for the whole of a game's data named after its scripting language misleads the newcomer the vision is for.

**Reference checks live in the format and dialogue crates and are used by the authoring API in milestone 3, before the lint commands of milestone 5.** The proposal requires every write validated (R22) and the Calo scenes are written under milestone 3. Rejected: lints first, then authoring. The mod waits a milestone for a command it does not need yet. Rejected: authoring without validation, lints later. The Juhani mod's first play sessions were spent on exactly the faults a check at write time catches.

**The Juhani port is the authoring API's acceptance, filed in this project with the `modshop` label.** The first graph's `#37` ("modshop compiles its mods' scripts with nws via the shim") is the precedent: the work changes modshop's tree, the acceptance is modshop's chain hash, and the node lives where the tool does. Rejected: a modshop skift project. None exists, and an edge across projects for one node's worth of work buys nothing.

**The installer is one reader node and one executor node, with TSLPatcher under Wine a spike that is ready on day one.** The sections share the memory tokens, the pipeline order and the report; one node per section is seven nodes arguing over shared state. Rejected: parity first. There is nothing to compare until the executor exists; but whether TSLPatcher can be driven headless is a question with no dependency, so it is a spike, not a late node.

**The index is SQLite with FTS5, in the one crate declared to own a file, with typed fact tables and one reference table.** modshop's asset-index idea weighed DuckDB and SQLite and chose SQLite for two-to-three-join questions plus full text; `rusqlite` with the bundled feature adds no system dependency, which the hermetic principle requires. DeusData's codebase-memory-mcp (MIT, C, a generic node and edge schema over SQLite with FTS5 and a Cypher subset, built for source code) is the design precedent for three things the index adopts: a content digest computed from what rows say so an incremental build is proven equal to a cold one, a coverage table so an empty answer is qualified, and building in memory with one write at the end. Rejected: DuckDB, for a question depth that does not need columnar scans and a dependency that does. Rejected: an in-memory graph rebuilt per run, because a cold parse of 43,000 resources is the cost the index exists to pay once. Rejected: a generic property graph with a query language, because the questions are joins over typed facts (a dialogue node's speaker, a call's literal argument) and `refs` is the one traversal; a 6,700-line Cypher executor buys nothing here, and tree-sitter, that project's parser, has nothing to parse in GFF or 2DA.

**Milestones are grouping nodes of kind `milestone`, as in the first graph, and Release folds into the existing `#9`.** Rejected: a second release milestone. One workspace, one version, one first release; a Rakata release that is not the scripts toolchain's release contradicts the proposal's decision 1.

**Saves go to the handheld through a gate.** "A written save loads on the handheld and shows the changed global" is something only a person at the device can see. The agent node that writes the save and the instructions is separate, so the gate arrives with its inputs.

**Gates are for the rename, the release and CI setup, and the device check; nothing in modshop waits on them.** The owner keeps authority over CI/CD and release publishing and wants implementation and testing autonomous. So the rename, approving the release and CI setup, and loading a save on the handheld (which only a person at the device can do) are gates, each preceded by the agent node that produces its inputs. Rejected: gating the switch of modshop's patcher, its mise pins or the corpus refactor. They are how modshop uses the tools, which is the owner's explicit non-concern, and the rule that using a change in Rakata from modshop must never require a push or a release means no modshop node may depend on a release node.

**A Rakata build enters a modshop key by what it is, not where it came from.** `rakata --version` and `rakata.__version__` print one build identity: workspace version, git describe, a hash of the dirty tree. An entry that uses Rakata keys on that string, so a local build from the checkout rebuilds the entries that use it and nothing else, and a pinned release is the same mechanism with a cleaner string. Rejected: pinning the wheel in `uv.lock`, because the lock enters every key and a one-line change in Rakata would rebuild the 190-entry chain. Rejected: a path dependency, because the lock records the path and not the content, so a change would be invisible to the key, which is worse than slow.

**The `unsafe_code` forbid stays at the workspace, and `rakata-archive` may earn one exception with a number.** The owner's reading of the rule is that it stops unsafe being used where a safe way is as good; squeezing the hardware for the build pipeline is worth an exception. So the archive node measures a whole-file `Source` against a memory-mapped one and keeps the mapped one, in one module with the lint allowed there alone, only if the margin justifies it. Rejected: lifting the lint up front, which decides without the number; rejected: never mapping, which the PRD's performance targets may not survive.

**modshop's side is a proposal in modshop, written first.** `docs/proposals/rakata-in-modshop.md` names which ideas Rakata takes and which stay, the keying rule and the two delivery modes, so the port, lint and patcher nodes cite a file. Rejected: rewriting each idea file in place, which scatters one decision over ten documents; rejected: writing nothing until the tools land, which leaves the nodes citing a conversation.

## Spikes

1. **Drive TSLPatcher under Wine without its GUI** (`tslpatcher-oracle`). The parity requirement (R30) needs TSLPatcher as a black-box oracle over every patcher step. modshop already runs a Windows installer in a Wine container with no network (`games/kotor/kmrp-build/wine/Containerfile`). Unknown: whether TSLPatcher takes a destination on its command line or needs synthetic input under `xvfb-run`; which versions of `TSLPatcher.exe` the archives ship. Ends in a written finding under `docs/research/` and a script that installs one mod. Ready on day one.

No other spike. The index's size and the timings are published measurements in the index node's acceptance rather than a spike, because the answer changes nothing about the shape; the GFF layout variants other writers produce are what the harness exists to enumerate.

## Acceptance criteria for the epic

- `rakata conformance --game <vanilla>` and `--game <full build>` run to completion and the README carries their numbers for archives, GFF, 2DA, TLK and LIP, generated from the report.
- `rakata which <resref>.<ext> --game <full build>` agrees with the Juhani mod's `loaded()` on every input it reads and reports every case-variant duplicate `collapse_case` found.
- `grep -rl k_pman_aioff $NWSCRIPT_CORPUS/decompiled/` finds `k_pdan_juhani27` and no spurious file.
- `rakata dialog show banter.dlg --game <full build>` shows the prone script and animation 10055 on consecutive nodes with the conditions decompiled.
- The Canderous coverage report is one query file against an index of the full build, and the strref 42724 chain is three `rakata refs` calls with no false negative.
- modshop's `mod:juhani-clones` entry runs on the `rakata` Python module and the chain hash is unchanged, or every difference is listed and explained in the entry's README.
- `rakata save show` over the Dantooine save reproduces the three findings (bit 62 on two creatures, lowercase tags, action 61 ahead of the banter), and a save written by `rakata save set` has loaded on the handheld.
- `rakata install` over every HoloPatcher step of the full build matches the current tree or has each difference explained against TSLPatcher, and modshop's `holopatcher` step can run it.

## Open questions

- **The lore name.** Rakata is a word from the games' setting; an invented noun is nearer a trademark question than a generic word. Settled by the owner before the rename gate; the fallback is a neutral name, and nothing in the crates' code depends on it.
- **Engine-only numbers.** Dialogue animation numbers, effect types and camera framing live in the executable. The save and dialogue tools print them as numbers until an engine-tables file exists; whether that file is generated from REA output in modshop or committed here is open, since it is derived from game bytes.
- **The archive corpus for `changes.ini`.** 67 zips are under `data/archives/`; the rest of the build's mods are other archive formats, extracted by modshop's `Extractor` with 7-Zip. The reader node counts what it parsed; whether the count reaches every mod in `build/k1full.json` is checked there.

## Later

- Reachability over both of the Calo mod's arcs, by static analysis of condition scripts through `nwscript-vm`, with the undecidable conditions listed. Waits for the index and the dialogue lints.
- MDL and TPC crates, when a lint needs their contents (repair 07's power-of-two textures, repair 08's emitter controllers). Waits for a cost paid after the hash-and-header diff ships.
- Engine tables from the decompiled executable, naming animation numbers, effect types and cameras. Waits for the open question above.
- A KotOR 2 profile measured against its own corpus and install.
- WebAssembly and a desktop application over the façade.
- A declarative scene format, after the Python API has carried two mods.

These belong to the held `nwscript#52` (Later), to be decomposed when the proposal's milestone 7 ships, as that node's hold says.

## Work orders

Decomposed as 44 nodes under [`rakata/patches/nwscript/`](rakata/patches/nwscript/), not yet applied: seven milestone grouping nodes, three gates, one spike, and 33 tasks, with three nodes parented to the existing `#9`. The [README there](rakata/patches/README.md) has the shape, the sizing and what must be true before applying.

## Provenance

Drafted 2026-10-17 from `docs/proposals/rakata.md` and `docs/prd.md` in this repository; `Cargo.toml`, `xtask/src/archive.rs`, `crates/nws/src/output.rs`, `crates/nwscript/src/resolve.rs`, `AGENTS.md`, `conformance/`; the skift graph `nwscript#1`–`#53` (`node_list`, and `show_node` on `#8`, `#9`, `#11`, `#16` for the body conventions); modshop's `CLAUDE.md`, `games/kotor/CLAUDE.md`, `src/modshop/kotor/{patcher,toolchain,steps}.py`, `games/kotor/mods/juhani-clones/{make.py,README.md}`, `games/kotor/patches/README.md`, `games/kotor/docs/ideas/{README,device-state,inspect-cli,asset-index,script-corpus,roundtrip-tests,chain-lint,semantic-diff,patcher-audit,patcher-parity}.md`, and the data layout under `games/kotor/data/`; the steading epic `the-floor-reads-the-wire` as the format precedent; and DeusData's codebase-memory-mcp (`~/src/github.com/DeusData/codebase-memory-mcp`: `README.md`, the schema in `src/store/store.c`, `src/store/store_content_digest.c`), read for the index's design. The conversation weighed renaming first, format crates before the harness, lints before authoring, one node per ini section, DuckDB, a generic graph store with Cypher, pinning the wheel in `uv.lock`, lifting the `unsafe` lint up front, and a second release milestone, and rejected each for the reasons under [[#Design choices]]. The owner set the gating rule (rename and build systems gated, implementation and testing autonomous, wiring never gated) and the no-push rule in conversation on 2026-10-17. The cross-session handoff from the Calo mod's session supplied the Know → Write → Prove → Saves order and the acceptance cases.
