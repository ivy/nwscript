# Proposal: Rakata, a toolset for the Aurora games in Rust

Status: accepted on 2026-10-17; decomposed in [`../epics/rakata.md`](../epics/rakata.md). It extends [`../prd.md`](../prd.md), the accepted proposal for the NWScript toolchain, from one layer (scripts) to the whole of a game's data, and renames the repository. Nothing in that document is withdrawn; where this one says less about scripts, that one is the reference. The evidence it rests on is the making of [Oops! All Juhani](https://github.com/ivy/modshop/tree/main/games/kotor/mods/juhani-clones) in modshop on 2026-10-16/17, the costs recorded in modshop's [ideas index](https://github.com/ivy/modshop/tree/main/games/kotor/docs/ideas), and a survey of every PyKotor call made from a coding-agent session on this machine ([Appendix](#appendix-the-pykotor-surface-in-use-today)).

## Summary

A clean-room toolset for the data of BioWare's Aurora-derived games, KotOR 1 and 2 first, written in Rust: the archives, the resource formats, the install's loading rules, dialogues, saves, an index that joins them, an authoring API over them, and a TSLPatcher-compatible installer, each verified by round-tripping the bytes the games ship and the bytes a real modded build holds. The scripts toolchain already in this repository is one layer of it. Everything is a library first, with a command line, Lua and Python over the same façade, so that a coding agent, a build system and, later, a desktop application all drive the same code. Every command reports as structured data, and every number the project claims about itself is measured by its own suite and published.

The decisions this proposal makes:

1. **One repository, one version.** `ivy/nwscript` becomes `ivy/rakata`; the `nwscript-*` crates keep their names, new crates are `rakata-*`, and everything shares the workspace version until 1.0 ([Name, licence and repository](#name-licence-and-repository)).
2. **The game's bytes are the specification, for data as for bytecode.** A format is read losslessly and written back identically for every file the games ship; where that cannot hold, the suite says so per file ([Round-trip conformance](#round-trip-conformance)).
3. **Clean room, for every crate.** PyKotor and TSLPatcher are measured, not read ([Clean room](#clean-room)).
4. **Built in the order a campaign-length mod needs it**: know the game, then write to it, then prove the result; the installer after those ([Milestones](#milestones)).
5. **Fast is a feature with a number.** Each milestone publishes its timings over a full modded build, not only its correctness ([Performance](#performance)).

## Vision

The vision of [`../prd.md`](../prd.md) stands: anyone can reshape these games, with a coding agent doing the work, without first becoming an expert in a twenty-year-old modding scene. Scripts were its first layer because everything else depends on them. This proposal is the rest of the life cycle.

A mod is a story told in the game's own data: dialogue nodes with their speakers, conditions and voice; blueprints; table rows; scripts that join them. Writing one today means holding that data in a Python toolkit's object model, one throwaway script at a time, and finding out in play what a query would have settled on the build host. The bet is the same as before: the thing standing between a newcomer, or an owner with a campaign to write, and a working mod is tooling robust enough for an agent to drive. For data that means: ask the game anything about itself in milliseconds and get an answer that cites the resource; change it through an API that refuses to write what the game cannot load; prove, before anything is deployed, that every reference resolves, every line has a voice, every branch is reachable, and the install did what it said. The mod is the script; the toolset makes the script run.

## Why

**The toolkit the work depends on is a single Python library, and the project around it is shedding repositories.** modshop's build installs mods with HoloPatcher, whose patching engine is PyKotor's `tslpatcher` package, and its one mod of our own is written against PyKotor's object model. On 2026-10-17 the HoloPatcher repository the PyKotor monorepo points to as a submodule (`OldRepublicDevs/HoloPatcher`) and the reconstructed TSLPatcher source that kpatcher points to (`OpenKotOR/TSLPatcher`) both returned "Repository not found". The code survives in PyPI's 1.5.3 wheel, in PyKotor's history before the split, and in our fork; the design does not survive anywhere we control. PyKotor is LGPL-3.0-or-later, so a port is not an option and a clean-room implementation is.

**The patcher's defects were silent, and the fixes are ours to carry.** Eight patches in modshop's series fix PyKotor defects met in one build: a 2DA read as empty and written back empty, blanking every feat name; module replacements logged as done and not done, dropping most of the Community Patch's module fixes; a compiler whose argument order was wrong, found in play. Two were found by playing. Each was found one at a time, by a failure, and each costs a rebase when upstream moves.

**The cost of knowing the game is paid again on every question.** Making the Juhani mod asked, over and over, "how does the game already do this": which script turns the party's AI off, which dialogue gets a prone character up and with what number, how a party member starts a conversation. Each answer was a scan of the build's 43,000 resources by a script written for the occasion, and the questions that went unanswered (which dialogue animation kneels, what a placed camera frames) cost a play session each. Three of the mod's causes were read from a save in minutes after hours of guessing, by four more one-off scripts.

**The next mod does not fit the current tooling.** The owner's next mod makes Calo Nord an optional party member in Canderous's place, across the whole campaign: changed scenes, new scenes, dialogue edits and a party swap in twenty-plus modules, with two arcs that must converge on the same game state. The Juhani mod's authoring code is 253 lines of plumbing hardcoded to one module plus a 342-line hand translation of a screenplay into dialog edits; the corpus has no dialogues and no index; nothing checks continuity. The research for Calo asks "every line, gate and speaker that names Canderous" hundreds of times. That is an index, not a scan.

**Rust is where this project already is.** The scripts toolchain exists, is measured (1,665 of 1,754 vanilla scripts byte-identical; 99.92% of module functions verified by recompilation; a real modded build identical when built with its shim), and is a Cargo workspace with shared lints and Lua and Python bindings. Its `xtask` already reads KEY/BIF and ERF/RIM/MOD to extract the corpus. The remaining formats are smaller than a compiler.

## Users

In priority order:

- **A mod author with a coding agent, writing something long.** The owner, first. They have a screenplay, a research file and a game install, and they want each scene to go from text to a tested change without hand-translating it, to know before deploying that nothing dangles, and to reach a late-planet scene by loading a save, not by playing to it. Success is the Taris chapter of the Calo mod shipped with the toolset carrying the plumbing.
- **A newcomer with a coding agent.** As in [`../prd.md`](../prd.md): everything arrives through progressive disclosure, and the acceptance test is a fresh agent taking a described change to a working mod from the skill, `--help` and `explain`.
- **The modshop build**, which needs an installer whose report is true, readers that do not drop data, and all of it pinned by version outside its Python import closure, so a tool change does not rebuild 190 entries.
- **An existing mod author or installer.** Their `tslpatchdata/` installs unchanged, and faster, on Linux, macOS and Windows.
- **A desktop application, later.** The crates do no I/O and own no process state, so an editor over them is a view, not a port.

## Goals and non-goals

Goals:

- Read every resource in a vanilla KotOR 1 install and in modshop's full modded build, and write each back byte-identical, or classify the exception per file.
- Answer "which copy of this resource does the game load" exactly as the engine does: Override, then the module's capsules in their order, then the BIFs, case-insensitively.
- Index every dialogue node, script call with literal arguments, blueprint field, table cell and string in a tree, so that a reference in any direction is a query, with provenance to the mod that introduced it.
- Render a dialogue as a transcript a person reads, with each condition decompiled to source and each node's speaker, listener, voice, animations and camera named.
- Give a mod an authoring API in which an existing node is addressed by what it is (its voice resref, its text), not where it is, and the build fails when the base moved.
- Read a save game by name: globals, party, each creature's tag, flags, effects, local variables and action queue; write one whose globals differ, for fixtures.
- Lint a mod's output statically for the faults play has found: dangling scripts and resrefs, strrefs past the TLK, lines with no voice, unreachable nodes, undeclared globals.
- Install a `tslpatchdata/` mod with the semantics TSLPatcher has, and report every change by what the tree now holds, not by what the installer intended.
- Expose all of it as one façade with Lua and Python bindings, a `rakata` command with the same JSON envelope and exit codes as `nws`, and a skill that teaches the workflow.
- Keep the game-independent layers free of KotOR assumptions, so KotOR 2 and the other Aurora games are profiles.

Non-goals:

- A graphical tool, in this proposal. The crates are shaped so one can follow; none is built here.
- Models, textures, walkmeshes and audio decoding, in the first milestones. They are read for provenance and hashing only until a cost is paid that needs their contents ([Later](#milestones)).
- Simulating the engine beyond what the scripts toolchain's world model already does. A reachability check over dialogue conditions is static analysis over scripts and globals, and it says what it could not decide.
- Reproducing TSLPatcher's defects. Its behaviour is the specification for `changes.ini`; where it is wrong about the game's formats, the suite records the difference and the installer does what the game needs.
- Reading PyKotor's or any patcher's source.

## Core principles

The eight principles of [`../prd.md`](../prd.md) apply to every crate: drop-in first; standing on the community's shoulders and saying so; written for the agent that will use it; the game's bytes are the specification; diagnostics are data; library first; hermetic by default; honest about unknowns. Three are sharpened for data:

**The game's bytes are the specification, and other writers' bytes are evidence.** BioWare's own files fix a format's layout, and the writers reproduce it byte for byte. Files written by community tools (KotOR Tool, TSLPatcher, PyKotor) show which variants the engine tolerates; the readers accept every variant the engine does, the suite records which variant each file in a real build has, and the writers emit BioWare's.

**One model of the install.** Which resource the game loads, from where, under what name, is decided in one crate and nowhere else. Every command, the index and the authoring API ask it. The Juhani mod's `loaded()` and `module_copies()` and modshop's `ci_find` are three answers to the same question today.

**Fast is a feature with a number.** A question an agent asks hundreds of times is answered in milliseconds or it is not answered. Every milestone publishes its timings over the full modded build: a cold index, an incremental one, a resolve, a transcript. Memory-mapped archives and zero-copy parsing are how; the published number is what.

## Clean room

The rule of [`../prd.md`](../prd.md) extends to every crate. Permitted: the files in the user's own copy of the game and in their mod archives; the engine's observable behaviour, including its decompiled code where a question is about the engine; the community's published format documentation (the xoreos documents, the KotOR and Neverwinter Nights wikis' format pages, stoffe's TSLPatcher readme and `changes.ini` syntax pages, which describe files and an installer's contract, not an implementation); and the output of other tools run as black boxes. Not permitted: PyKotor's source (LGPL-3.0-or-later, and the design must be independent of it), HoloPatcher's, kpatcher's, TSLPatcher's reconstructed Delphi tree should it reappear, and the compilers already excluded.

The room has two roles, and an agent holds one or the other. The format scribe may read the reference projects and writes only Markdown specifications under `docs/formats/`: the layout of a file or the behaviour of an operation as facts a reader can check against bytes, with a provenance section naming what was read, and never identifiers, code or prose from the source. Implementers never open the references; they read the public documents, `docs/formats/`, and the game's own files, and they measure the reference tools as black boxes. A specification is written only for a gap the public documents leave, and a measured fact (run the tool, describe the bytes) is preferred to a read one because it needs no wall at all. The repository enforces the split: a hook refuses an implementer's tool call that names a reference project, and the scribe's hook refuses its writes anywhere but `docs/formats/`.

The cost is the same as before and so is what it buys. One cost is specific to data: PyKotor's `tslpatcher` encodes fifteen years of what mods actually put in `changes.ini`, and that knowledge has to be re-derived from the archives of the 192 mods in modshop's build and from TSLPatcher itself, run under Wine as the oracle. The build is the corpus.

## Prior work

To the names in [`../prd.md`](../prd.md), which the README's Acknowledgments carry, this proposal adds: **stoffe**, whose TSLPatcher and ChangeEdit defined how a KotOR mod is distributed and installed, a contract mods have followed for two decades; **the xoreos project** again, for documenting the archive and resource formats of every Aurora game, not only the bytecode; **the KotOR modding wiki's editors**, whose format pages are the primary documentation for GFF, 2DA, TLK, LIP and the save game; **Fred Tetra's KotOR Tool**, which first opened the archives; **PyKotor and the Holocron Toolset**, which brought all of this to Python with a GUI, and whose installer's behaviour over a real build is the measure this project's installer starts from; **th3w1zard1's kpatcher**, whose Ghidra notes on TSLPatcher's pipeline order are cited where the suite tests it; and **Daniel Wolf's Rhubarb Lip Sync**, which the voice pipeline runs. The permitted inputs from each are documentation and measured behaviour.

## Architecture

New crates join the workspace beside the `nwscript-*` crates, under one prefix:

| Crate | Responsibility | Depends on |
| --- | --- | --- |
| `rakata-res` | Resource identity: `ResRef` (16 bytes, case-insensitive), the resource type table (ids, extensions), content hashing. | nothing |
| `rakata-archive` | KEY/BIF, ERF (and MOD, SAV, HAK), RIM: read with memory mapping, write, add and replace members. Grows from `xtask/src/archive.rs`. | `res` |
| `rakata-gff` | The GFF tree: every field type, lists and structs, lossless read, BioWare-layout write, a path syntax (`EntryList[3].RepliesList[0].Index`), a diff. | `res` |
| `rakata-2da` | 2DA: rows, columns, cells, row labels, the header variants the engine accepts, write, diff. | nothing |
| `rakata-tlk` | TLK and `LocalizedString`: strrefs, languages, genders, the sound resref per entry, append. | `res` |
| `rakata-lip` | LIP: the sixteen mouth shapes with their times; read and write. | nothing |
| `rakata-game` | The install: the search order, module capsule precedence, case-insensitive paths, `which` with provenance; the resolver implementation for `nwscript`'s includes. | `res`, `archive`, `gff`, `2da`, `tlk` |
| `rakata-dialog` | DLG as a model over `gff`: nodes, links, starts, conditions, animations, cameras; stable-key addressing; the transcript; the dialogue lints. Decompiles conditions through `nwscript-decomp`. | `gff`, `tlk`, `game`, `nwscript` |
| `rakata-save` | Save games: the outer ERF, `GLOBALVARS`, `PARTYTABLE`, `AVAILNPC*`, each module's `.sav` and its GIT; creatures' flags, effects, variables and action queues by name; write. | `archive`, `gff`, `game` |
| `rakata-index` | The index: facts, references and full text over a tree, keyed by content hash, in SQLite. The one crate that owns a file, declared as such. | all format crates, `nwscript-ncs` |
| `rakata-patch` | `changes.ini` and `namespaces.ini`: the reader, the install plan, the executor, the report checked against the tree. | all format crates, `game`, `nwscript` |
| `rakata` | The façade: one API for resolve, read, write, index, show, lint, diff, install; the diagnostic codes. | all |
| `rakata-lua`, `rakata-py` | Bindings of the façade, the same shape as `nwscript-lua`; the Python module is `rakata` with `rakata.nwscript` inside it, and `nwscript-py` folds into it. | `rakata`, `nwscript-lua` |
| `rakata-cli` | The `rakata` binary. `nws` stays as it is. | `rakata` |

The `nwscript-*` library crates do not depend on any `rakata-*` crate; `rakata-game` implements their resolver trait, and `nws`, the command, may take it for a `--game-dir` flag. The dependency runs one way so the scripts library stays usable alone.

`rakata-index` is the declared exception to "the crates do no I/O": an index is a file by nature. Its extractors (what facts a dialogue, a script or a blueprint contributes) are pure functions in the format crates and are tested there; only the store is in `rakata-index`.

A **game profile**, as in the scripts toolchain, names the game: KotOR 1 and 2 are the first two, and they differ in resource types, GFF fields, the save's layout and the install's folders.

## Requirements

### Resources and the install

- Resolve a resource name and type to the bytes the engine would load, following its order: `Override/` (with the engine's case rules), then the module's capsules in the engine's precedence, then the KEY's BIFs. Report every candidate the install holds, which one wins and why.
- Paths compare case-insensitively on every platform, since mods and TSLPatcher write names in every case and the game runs under Wine on a case-sensitive file system.
- Hash every resource by content, so an index, a cache and a build key agree on identity.
- Acceptance: `rakata which` over every resource name in the full modded build agrees with what the game loads, checked against the Juhani mod's `loaded()` on its inputs and against the case-variant duplicates modshop's `collapse_case` found.

### Archives

- Read KEY/BIF, ERF in its variants, and RIM, from a memory map, without copying a member until it is asked for.
- Write each, and replace or add a member in place, producing an archive the engine loads; the replacement defect in PyKotor's series is the first regression test.
- Acceptance: every archive in the vanilla install and the full build lists and extracts; every archive written from a read one is byte-identical where the source was BioWare's.

### GFF, 2DA, TLK and LIP

- GFF: every field type the engine uses, including `CExoLocString` with its substrings, `Void`, `Orientation` and `Vector`; nested structs and lists; the file's struct and field ordering preserved on read so a write reproduces it; a path syntax for naming one field from the command line and from a diff.
- 2DA: the header variants seen in the wild (the zero cell-data size five mods ship), row labels that are not indices, column add, row add and copy.
- TLK: append with the returned strref, since that is what installing a mod needs; the sound resref per entry, which the voice pipeline uses.
- LIP: read and write with the sixteen shapes the game knows.
- Acceptance: the round-trip numbers of [Round-trip conformance](#round-trip-conformance), per format, over the vanilla install and the full build.

### Dialogues

- A model over the GFF: entries, replies, starting links, each link's condition script and its `IsChild`, each node's speaker, listener, text or strref, voice, sound, script, delay, animations with participants, camera angle and id, quest and comment.
- **Stable keys.** A node is addressed by its voice resref, by its strref, or by its text, and the API fails with a diagnostic naming the key when the base has no such node or more than one. Index addressing exists for the cases keys cannot cover and is marked as fragile in the output.
- **Append-only edits by default.** New nodes go at the ends of the lists so every existing index stays, as the Juhani mod does; a reordering edit is a separate, named operation.
- **`rakata dialog show`** renders a conversation as a transcript: who says what to whom, each branch indented, each condition decompiled to source (verified or not, per `nws decompile`), each script named, each animation's number and participant, each camera's id, and each voice line's duration where the audio is present.
- **Lints**: a script or condition that resolves to no resource; a strref past the TLK; a voiced node whose voice resref has no file; a node no start or link reaches; a condition that reads a global `globalcat.2da` does not declare; a `SoundExists` that disagrees with the voice field.
- Acceptance: the transcript of `banter.dlg` shows the prone script and the get-up animation 10055 on consecutive nodes; the lints run clean on the vanilla dialogues except for a published list of the game's own faults, and on the Juhani mod's output.

### The index and references

- Facts: a row per dialogue node with the fields above, per script call with its routine name and literal arguments (from `nwscript-ncs`'s decoded instructions, so a party-slot number compiled to a constant is found), per GFF field of a blueprint or area instance, per 2DA cell, per TLK entry.
- References: the game's symbols are resrefs, tags, strrefs and global names, and each format refers to them in its own way (`CONSTI`/`CONSTS` operands, `ResRef` and `CExoLocString` fields, 2DA cells, `globalcat` rows). `rakata refs THING` answers in every direction and names, for each hit, the resource, where the game loads it from, and the field or instruction.
- Text: full-text search over node text, comments, TLK strings and script source, decompiled where there is none.
- Keyed by content hash so a resource shared by every checkpoint of a build is parsed once; incremental from a list of changed paths; provenance to the first tree a hash appeared in.
- Acceptance: the Canderous coverage report (every node, gate and call naming Canderous or his party slot, across the install) as one query; the transition chain (strref 42724 to `k_trg_transfail`) as three `refs` calls with no false negative; the `k_pman_aioff` finding as a query; a cold index of the full build and an incremental one, timed and published.

### The corpus

- `rakata corpus` extracts every script of an install, as the `xtask` does today, and decompiles every module script into a tree beside the sources, with a header naming its origin and which copy the game loads, whether every function verified, and with include libraries folded out so a grep hit is in the script's own logic. The call index is written alongside for the index to take.
- Acceptance: `grep k_pman_aioff` over the corpus finds `k_pdan_juhani27` and nothing spurious; the whole of KotOR 1's modules decompile in a published time.

### Authoring

- The operations the Juhani mod performs, as library calls: find the loaded copy; guard a replaced resource by hash; add a row to a 2DA by copying another with changed cells; declare globals in `globalcat.2da`; edit a blueprint's fields by path; add entries, replies, links and starts to a dialogue by stable key; make a LIP from timed shapes; compile scripts against the install's includes through `nwscript`; write the results to `Override/` and `streamwaves/`.
- Every write goes through the same validation the lints apply, so a mod cannot write a node whose script does not exist without saying so.
- The voice pipeline's external steps (cutting with ffmpeg, Rhubarb's cues) stay outside the library; the library gives them the spans and takes the cues, and the cache is keyed by take hash and span so a line is cut once.
- Available from Python (modshop's `Script` step runs a mod's script in the tool's interpreter) and from Lua (`rakata lua SCRIPT`, hermetic, no interpreter to install).
- Acceptance: the Juhani mod ported to the API, with its own code reduced to its scenes, producing the same bytes: modshop's chain hash unchanged, or every difference explained.

### Saves

- Read `SAVEGAME.sav` and its contents by name: globals from `GLOBALVARS` with the boolean bit order, the party table, each `AVAILNPC` with its action list, each module's state with its creatures' tags as the world has them, `Commandable`, faction, animation, effects (type, subtype, creator, named when the creator is in the save), `SWVarTable` booleans and numbers by index and by `SW_FLAG_*` name where the scripts' includes name them, and the pending event queue.
- Numbers whose meaning lives only in the engine (effect types, dialogue animation numbers) are printed as numbers with a note that the table is not known, until an engine-tables file supplies it.
- Write a save whose globals or party differ from a read one, byte-identical elsewhere, for fixtures.
- Acceptance: `rakata save show` reproduces the three Juhani findings (AI off on two creatures by bit 62, the world's lowercase tags, the banter queued behind action 61 on Mission) from its output alone; a written save loads on the handheld and shows the changed global.

### Lint and diff

- The dialogue lints above, plus the tree lints modshop's ideas name: a resref that resolves to nothing, case-variant duplicates and `.tpc` shadows in Override, a `changes.ini` row naming a missing section.
- A semantic diff between two trees or two resources: fields by path in GFFs, cells in 2DAs, strings in TLKs, members in archives, nodes in dialogues as a transcript diff; models and textures by hash and header only until their crates exist.
- Later, a reachability check: from a set of global values, which dialogue nodes and starts can be reached across the install, by static analysis of the condition scripts through the scripts toolchain's interpreter, with the undecidable conditions listed.
- Acceptance: the diff of the Juhani entry's checkpoint against its parent lists exactly the mod's changes, by field; the lints over the full build run in a published time with findings traced to entries.

### The installer

- Read `changes.ini` and `namespaces.ini` with TSLPatcher's semantics: every section (`TLKList`, `GFFList`, `2DAList`, `InstallList`, `HACKList`, `CompileList`, `SSFList`), its memory tokens, its pipeline order, its treatment of a missing section (skip, as TSLPatcher does, where PyKotor raised), its `Required` without `RequiredMsg`.
- Compile a `CompileList` through `nwscript`, never through a shipped `nwnnsscomp.exe`.
- **The report is the tree.** After every operation the installer re-reads what it wrote and reports the hash it finds, so a claim that did not land is an error from the installer itself, not from an audit after it.
- A dry run that prints the plan; a JSON report a build system keys on.
- Acceptance: over every HoloPatcher step in modshop's full build, the same tree as the current patcher, or every difference explained against TSLPatcher run under Wine on the same inputs. The eight patches in modshop's series are regression tests.

### Diagnostics, agent surface and command line

As in [`../prd.md`](../prd.md), without exception: stable codes with `rakata explain`, miette for people and one JSON document per run for programs with the same envelope and exit codes as `nws`, `--help` complete enough to need no other document, nothing read that was not named, and a skill in the repository's plugin that teaches the workflow by progressive disclosure. The commands, each one library call: `which`, `cat`, `gff show|set|diff`, `2da show|diff`, `tlk show|append`, `dialog show|lint|add`, `refs`, `index build|query`, `corpus`, `save show|set`, `lint`, `diff`, `install`, `lua`, `explain`. `--game` has no default, for the same reason as before.

## Round-trip conformance

The suite for data is `rakata conformance`, the counterpart of `nws conformance`: over a tree the user names, read every resource with the matching reader, write it with the matching writer, and classify the result per file as identical, equal after re-read (a layout another writer chose, named), or different (a bug). It runs over the vanilla install and over modshop's full build, whose 190 entries hold what mods wrote with every tool the community has used. The repository holds no game bytes: the manifest is names and hashes, and the report's headline numbers are generated from it into the README beside the compiler's.

This is the project's test suite for formats, and the first thing each format crate must pass. A reader that drops data silently is the defect class the build paid most for, and the suite is what makes it impossible to ship one.

## Performance

Targets, to be replaced by measurements as each milestone lands and published in the README:

- Resolve any resource of the full build in under a millisecond after a cold start under a hundred milliseconds.
- Index the full build cold in under a minute on the build host; incrementally, from a checkpoint's changed paths, in seconds.
- Render any dialogue's transcript, conditions decompiled, in under a second.
- Install a `tslpatchdata/` mod no slower than HoloPatcher does today, measured per entry over the build.

A target that is missed is published as missed, with the cause.

## Release readiness

Everything under [`../prd.md`](../prd.md)'s Release readiness applies to the whole workspace: five targets, signed and attested releases through `cargo-dist`, trusted publishing, Renovate, `cargo-deny`, the fixtures, and `release-plz` on conventional commits. The first tagged release of Rakata is the first release of the scripts toolchain as well; neither has been released, and one release carrying both is the point of one version. The Claude Code plugin carries the workspace version so a skill never describes a flag the installed binary lacks. `cargo-semver-checks` runs for the library crates from the first release, and the JSON envelope is versioned in the output.

## Milestones

Each milestone has a measurable exit, and the Calo mod's Taris chapter is the order's reason: know the game first, then write to it, then prove the result, then save-based fixtures, then the installer that makes the build's foundation ours.

| Milestone | Exit |
| --- | --- |
| 0. Workspace | The repository is `ivy/rakata`; the README's first paragraph describes the toolset; `rakata-res`, `rakata-archive` (from `xtask/src/archive.rs`) and `rakata-game` exist; `rakata which` agrees with what the game loads over the full build; `rakata conformance` runs over archives. |
| 1. Formats | GFF, 2DA, TLK and LIP crates; the round-trip numbers published for the vanilla install and the full build; `nws` resolves includes from an install through `rakata-game`. |
| 2. Know | `rakata corpus` decompiles every module script with headers and a call index; `rakata dialog show` renders `banter.dlg`; the index builds cold and incrementally with published times; `refs` reproduces the transition chain; the Canderous coverage report runs as a query. |
| 3. Write | The authoring API with stable keys and validated writes; Python and Lua bindings; the Juhani mod ported with its code reduced to its scenes and modshop's chain hash unchanged or every difference explained; the voice cache keyed by take and span. |
| 4. Saves | `rakata save show` reproduces the three Juhani findings; `rakata save set` writes a save the handheld loads; a named save library is possible from the build's own modules. |
| 5. Prove | The dialogue and tree lints over the full build with findings traced to entries; the semantic diff of a checkpoint against its parent; the lints in modshop's `mise run check` for its mods. |
| 6. Install | `rakata install` over every HoloPatcher step of the full build matches the current tree or explains each difference against TSLPatcher under Wine; modshop's `holopatcher` step runs it; the PyKotor patch series is retired, each patch having become a regression test. |
| 7. Release | Every item under Release readiness in place for the whole workspace; the first signed, attested release; the skill passes its fresh-agent test with a data change, not only a script. |
| 8. Later | Reachability over both of the Calo mod's arcs; MDL and TPC crates once a lint needs their contents (repair 07's power-of-two textures, repair 08's emitter controllers); engine tables from the decompiled executable; KotOR 2 measured; WebAssembly; a desktop application over the façade. |

## The first mod it serves

The Calo Nord mod is the order's test, chapter by chapter, Taris first. Before a scene is written, milestone 2 answers the research: every node, gate and script call that names Canderous or his party slot, as a report rerun on every build, which is also the continuity audit. Under milestone 3 the Taris scenes are written against the authoring API as the Juhani mod is ported onto it, so the API is proven on two mods before the second planet; one recipe entry per planet in story order makes Taris a checkpoint. Milestone 4 makes a late scene a load, not a replay. Milestone 5 runs the lints and the diff on every build before deploy. A screenplay source format compiled to scenes is deliberately after all of these: the Python API over two mods shows which operations recur, and that is what a format should name.

## Integration with modshop

Rakata is pinned in `mise.toml` by version like `nws`, outside the step runner's Python import closure, so a tool change rebuilds only the entries whose steps use it, which is how `nws`'s shim already enters a key. The `Script` step gives a mod the `rakata` Python module or the `rakata` binary; the `holopatcher` step gains a second implementation of its `Patcher` protocol, `rakata install`, run beside the current one until milestone 6's exit is met. The ideas in modshop's index that are library questions (inspect's `refs` and `which`, the asset index, the script corpus, device state's `save`, round-trip tests, semantic diff, chain lint's rules, patcher audit and parity) become `rakata` commands and modshop's own commands over them; the ideas that are about the build, the device and play (try, fixtures, recordings, live probes, the virtual pad) stay in modshop. The `pykotor` skill in modshop's plugin is replaced by Rakata's as each surface lands, with PyKotor kept for what Rakata does not yet read.

## Risks and open questions

- **Scope.** Eight milestones over twelve crates is a lot of project, again. The order is chosen so each is useful alone and the first three pay for themselves in the Calo mod's first chapter; the installer is last of the substantive milestones because the current one works, with patches, and the mod does not wait on it.
- **`changes.ini` in the wild.** TSLPatcher's semantics are documented by its author and reconstructed by others, but what mods actually write is known only from the archives. 192 archives and TSLPatcher under Wine are the corpus and the oracle; the risk is a construct none of them uses.
- **Formats other writers laid out differently.** A GFF written by KotOR Tool may not round-trip byte-identical through BioWare's layout, and the suite's "equal after re-read" class exists for it; the risk is a variant the engine accepts that the reader rejects, which only the full build can reveal.
- **The index's size and the targets.** Not measured yet; the targets above are targets.
- **Engine-only numbers.** Dialogue animation numbers, effect types and camera framing are in the executable, not the data. The save and dialogue tools print them honestly as numbers until the engine-tables work supplies names.
- **The authoring language.** Python first because modshop's mods run as Python today and the Juhani port is the proof; Lua is the same façade for a mod that wants no interpreter. Whether a declarative scene format follows is decided after two mods, not before.
- **A lore name.** Rakata is a word from the games' setting, as HoloPatcher and Holocron Toolset are. An invented in-universe noun is closer to a trademark question than a generic word; the community's precedent is long, and the fallback is a neutral name with the theme kept in the README.

## Name, licence and repository

The repository is `github.com/ivy/rakata`, renamed from `ivy/nwscript`; GitHub redirects the old name, so the plugin marketplace and existing clones keep working. An organisation follows a second maintainer, not the other way round: the repositories lost this year were in organisations, and what protects code is one repository with pinned dependents, not where it lives. `rakata` is free on crates.io as of 2026-10-17 (as are `rakata-*` by extension; `odyssey`, `aurora` and `starforge` are taken); the GitHub user `rakata` is a dormant account from 2012, and `rakata-dev` is free to hold.

The crates are `nwscript-*` for the language and `rakata-*` for everything else; the binaries are `nws` and `rakata`; the Python module is `rakata`. Every crate takes the workspace version, `0.0.0` today, until a format crate has users who want it stable, at which point that crate alone sets its own version. The licence is ISC; the project copies nothing, so nothing constrains it. The repository holds no game bytes: manifests are names and hashes, and the user's game and archives supply the content.

## Appendix: the PyKotor surface in use today

Every PyKotor module and name imported in a coding-agent session on this machine, from the session index on 2026-10-17 (seven sessions, two projects; counts are tool calls that mention the name, so they rank rather than measure), joined with what modshop's committed code imports. Where each goes:

| PyKotor surface | Calls / sessions | Rakata |
| --- | --- | --- |
| `ResourceType`, `ResRef` | 133/7, 8/2 | `rakata-res`, milestone 0 |
| `Installation`, `SearchLocation`, `Capsule`, `Chitin`, `Module`, `Game` | 111/6, 17/3, 34/5, 6/1, 6/2, 31/4 | `rakata-game`, `rakata-archive`, milestone 0 |
| `read_gff`, `bytes_gff`, `GFFStruct`, `GFFList`, `GFFFieldType`, generics (`utc`, `uti`, `dlg`, `git`, `are`, `ifo`, `jrl`) | 81/4, 18/2, 11/3, 10/3, 7/2, 3 to 7 each | `rakata-gff`, `rakata-dialog`, milestones 1 and 2 |
| `read_2da`, `bytes_2da`, `TwoDA` | 47/4, 10/2, 9/2 | `rakata-2da`, milestone 1 |
| `read_tlk`, `bytes_tlk`, `LocalizedString`, `Language`, `Gender` | 18/4, 4/1, 10/2, 7/2, 5/2 | `rakata-tlk`, milestone 1 |
| `read_erf`, `read_rim`, `ERF`, `RIM`, `ERFType` | 33/3, 13/2 | `rakata-archive`, milestone 0 |
| `LIP`, `LIPShape`, `bytes_lip`, `read_lip` | committed in `voice.py`; 4/2 | `rakata-lip`, milestone 1 |
| `read_ncs`, `compile_nss`, `bytes_ncs`, `NCSInstructionType`, `KOTOR_FUNCTIONS`, `dencs` | 26/3, 23/4, 16/4, 11/2, 8/2, 4/1 | already `nwscript-*`; PyKotor's were used as the comparison |
| `find_strref_references`, `find_script_references` (`find_tag_references` appears only in the skill's docs) | 3/1, 4/1 | `refs` and the index, milestone 2 |
| `pykotor.extract.savedata` | 4/3 | `rakata-save`, milestone 4 |
| `tslpatcher.reader`, `.logger`, `.patcher`; HoloPatcher's `core` | 6/2, 6/1, 1/1; the whole build | `rakata-patch`, milestone 6 |
| `diff_tool`, `pykotor.cli` | 1/1, 2/1 | `rakata diff`, milestone 5 |
| `read_mdl`, `write_mdl`, `bytes_mdl` | 16/2, 6/2, 5/2 | later; by hash and header until then |
| `tpc`, `bwm`, `lyt`, `vis`, `txi`, `wav`, `ltr`, `ssf` | 1 to 7, 1 to 2 sessions | later, by need |
