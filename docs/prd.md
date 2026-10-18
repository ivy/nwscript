# Proposal: an NWScript toolchain in Rust

Status: accepted on 2026-10-16. The evidence it rests on is [`research/nwnnsscomp-conformance.md`](research/nwnnsscomp-conformance.md). The toolchain lives at `github.com/ivy/nwscript` under the ISC licence. [modshop](https://github.com/ivy/modshop), the KotOR build that motivated it, consumes it as a pinned tool.

## Summary

A clean-room toolchain for NWScript, the scripting language of BioWare's Aurora-derived engines, written in Rust: a bytecode library, an interpreter, a compiler, a decompiler and a command line, built so that each layer is verified against the bytecode BioWare shipped. The compiler's target is byte-identical output to BioWare's compiler for KotOR, measured against the 1,754 vanilla scripts that ship as both source and bytecode. The interpreter makes scripts testable without the game. The decompiler is checked by recompiling its output. Every tool reports diagnostics as structured data with stable codes, rendered for people with miette and for programs as JSON, and every command is one call into a library that does no I/O of its own.

The decisions this proposal makes:

1. **BioWare's bytecode is the specification.** A construct compiles the way BioWare compiled it, byte for byte, even where another encoding would be shorter or clearer ([Principles](#principles)).
2. **Clean room.** The design is derived from the file formats, the corpus and the language as BioWare's scripts use it, not from any existing compiler's source. nwnnsscomp and pykotor are measured, not read ([Clean room](#clean-room)).
3. **Library first.** The core crates take strings and bytes and return values and diagnostics; the CLI, the Python bindings and any editor integration are thin layers over them ([Architecture](#architecture)).
4. **Strictness never changes bytes.** Lints and compatibility modes decide what is accepted; code generation is one function of the accepted program ([Strict by default](#strict-by-default)).
5. **The conformance suite is the project's test suite and its scoreboard**, and it ranks every compiler that can be run, including this one ([Conformance](#conformance)).

## Vision

Anyone can reshape these games, with a coding agent doing the work, without first becoming an expert in a twenty-year-old modding scene.

The bet: the thing standing between a newcomer and a working mod is not knowledge of the game but tooling robust enough for an agent to drive. The community's reverse engineering is thorough and its format documentation is good, and the tools built on it have carried two decades of mods. They were built for people, by volunteers, in the eras they came from: a compiler written for a different game before KotOR shipped, toolkits that made the archives readable for the first time, decompilers that worked out module scripts by hand. What none of them were built for is a reader with no context driving them at speed. An agent given those tools gets stuck the way a newcomer does, slowly and without knowing why. An agent given a compiler that explains its errors, an interpreter that runs the result in milliseconds, a decompiler that says which functions it is sure of, and a skill that teaches it the workflow, can take "make the Taris vault open for anyone carrying the keycard" from a sentence to a tested change.

Scripts are the lowest layer of that life cycle, and the one every other layer depends on: dialog conditions, triggers, quest state, combat AI and cutscenes are all scripts. Getting this layer right, and getting it right in a way that is drop-in for the people and tools already here, is the foundation the rest can be built on.

## Why

KotOR shipped no script compiler. Every mod's scripts are compiled by nwnnsscomp, Edward T. Smith's Neverwinter Nights compiler from 2002, which the community adopted for KotOR by feeding it the game's `nwscript.nss` and which exists for KotOR as Windows builds, or by pykotor's compiler, an independent reimplementation in Python that the modshop build uses with the seven patches in its series. Measured against the vanilla corpus with KotOR's own `nwscript.nss`:

| Compiler | Byte-identical to BioWare | Of 1,754 pairs |
| --- | --- | --- |
| nwnnsscomp, native build, `-v1.00` | 1,598 | 91% |
| nwnnsscomp, native build, default flags | 1,204 | 69% |
| pykotor 2.3.12 with the seven patches | 262 | 15% |

The 156 scripts nwnnsscomp does not reproduce are 132 whose output differs in length, one whose output differs in content, and 23 it rejects, 18 of them because BioWare's shipped `nwscript.nss` declares those functions differently from the `nwscript.nss` BioWare compiled against. No compiler yet reproduces the whole corpus, and no decompiler yet verifies its own output, because until a compiler matches BioWare's bytes there is nothing to verify against. The 91% is the measure of how far the community got without a corpus harness, and it is a long way.

The costs the modshop build pays today, which the toolchain removes:

- **A compiled script is unverified until someone plays it.** pykotor's argument-order defect was found in game. A compiler measured against the corpus, and an interpreter that runs the result, catch that class of defect on the build host.
- **A script in a module ships as bytecode only.** About 10,800 module scripts have no source. Fixing one means reading a disassembly by hand or trusting a decompiler's reading. DeNCS, xoreos-tools and pykotor's disassembler each recover a great deal, and each was built before there was a compiler that could recompile their output for comparison, so none can say which of its readings are exact.
- **The compiler was built for a person at a terminal.** nwnnsscomp reports errors as text for a reader, and on BioWare's own `nwscript.nss`, which carries a typo at line 5145, it waits rather than erroring, a case its author never had reason to meet. A tool the build and a coding agent drive needs stable codes, spans, exit codes and JSON.
- **The tools grew separately.** Compiling, disassembling, comparing and running a script are four programs in three languages, each written for its own need, with no shared model of the bytecode to compose them.

## Users

In priority order:

- **A newcomer with a coding agent.** They want to change the game and have never modded it. They have not read the community's documentation, which was written for people who already knew the games, and their agent's training data on this language is thin. Everything they need arrives through progressive disclosure: a skill that teaches the agent the workflow, a `--help` that is the manual, an `explain` for every error, and JSON for everything the agent does by hand today. Success is that their first script compiles, runs under the interpreter, and installs, without a question to a forum.
- **An existing mod author or installer.** They have a `tslpatchdata/` folder with `nwnnsscomp.exe` beside it and no appetite for rearchitecting. The shim means they replace one binary and get Linux and macOS, a compiler measured against BioWare's, and errors that explain themselves. Nothing else about their setup changes.
- **The modshop build**, which compiles mods' scripts at install time through pykotor's external-compiler seam and needs a compiler that is right, fast, hermetic and native on Linux.
- **Reverse engineering and repair work**, which needs to read module bytecode as source, change it, and prove the change did only what was intended.

## Goals and non-goals

Goals:

- Compile every vanilla KotOR script that BioWare's `nwscript.nss` can declare to bytes identical to BioWare's.
- Round-trip every vanilla NCS file, from the game's data and from its modules, through decode and encode without a byte of difference.
- Run any NCS file against a mock engine and record every engine call with its arguments, so two compilations can be compared by behaviour where their bytes differ and a script's logic can be tested without the game.
- Decompile module bytecode to source that recompiles to the same bytes, and say which functions it could not.
- Report every error and warning with a stable code, a span, a short message, a long explanation, and a machine-readable form.
- Be a drop-in replacement for nwnnsscomp at the command line the build and mods' installers already use.
- Support KotOR 1 and 2 from the first release, and keep the bytecode and language layers free of KotOR-specific assumptions so other Aurora games are a profile, not a fork.
- Let a newcomer's coding agent take a described change to a compiled, tested script using only the shipped skill, the tool's own help and a game install.
- Ship signed, attested binaries for Linux, macOS and Windows from a declared release workflow, with the fixtures of a mature open-source project in place before the first release.

Non-goals:

- Reproducing nwnnsscomp's output where it differs from BioWare's. nwnnsscomp is a compiler to measure, not a target.
- Simulating the game engine. The interpreter models the virtual machine exactly and the engine's functions only as far as a test needs; it says which results depend on functions it does not model.
- An editor or a GUI. A language server is a later layer over the same library; a graphical tool is out of scope.
- Reading any existing compiler's source to learn how a construct is compiled. The corpus answers that question.

## Core principles

**Drop-in first.** Every existing workflow keeps working before any new one is offered. The shim accepts the old command line. Output files land where the old tools put them. A mod compiled with the old tool and with this one is compared, not assumed equal. Adoption in a small community with established habits comes from removing a step, never from adding one.

**Standing on the community's shoulders, and saying so.** The file formats, the opcode semantics and the engine function tables were recovered by other people over two decades, mostly as volunteers, and every number in this document was measured with their tools. The project credits that work by name in the README and the format documentation, cites the source for each fact it relies on, contributes corrections back where it finds them, and describes the tools it measures in terms of what they achieved and the constraints they were built under. A clean room keeps compiler source out; it keeps nothing else out, least of all acknowledgment.

**Written for the agent that will use it.** Every surface is designed for a reader with no context: a newcomer, or their agent. Help text is complete, errors carry their own explanation, output is parseable, and the skill shipped with the tool teaches the workflow rather than assuming it. A feature an agent cannot discover from `--help` and `explain` is not finished.

**BioWare's bytecode is the specification.** The language has no standard. What exists is 1,754 scripts with their compiled form, and an engine that runs that form. Where the vanilla corpus shows how a construct is encoded, the compiler encodes it that way. Where the corpus is silent, the compiler chooses the encoding nwnnsscomp's `-v1.00` output uses, measured not read, and records the choice in the grammar document as unconfirmed. This is what makes the decompiler verifiable: decompiled source that recompiles to the original bytes is right by construction.

**Diagnostics are data.** One diagnostic type carries code, severity, labelled spans, message and help. miette renders it for people, serde for programs, a conversion for the language server protocol. The explanation text for a code lives beside the code's definition, so `explain` and the rendered help cannot drift apart. No diagnostic is built as rendered text.

**Library first.** The crates do no I/O. Includes and `nwscript.nss` reach the compiler through a resolver trait the caller implements: a test passes strings, the CLI passes directories, pykotor passes an installation. Every CLI command is one library call plus argument parsing and output formatting.

**Hermetic by default.** A compilation's inputs are the source, the files the resolver returned and the game profile. The same inputs produce the same bytes on every machine. The CLI reads nothing it was not pointed at, and its output names every input it used.

**Honest about unknowns.** The interpreter marks results that depend on unmodelled engine functions. The decompiler marks functions it could not recover. The conformance report marks scripts whose bytes cannot be matched and why. A tool that guesses silently is worse than one that stops.

## Clean room

The toolchain is written from the file formats, the corpus and the language as BioWare's scripts use it. The sources permitted are: the NSS and NCS files in the user's own copy of the game, BioWare's `nwscript.nss`, the engine's observable behaviour, and the output of other compilers run as black boxes. The sources not permitted are the code of nwnnsscomp (BSD-style licence, readable in principle but excluded so that the design is independent), pykotor's compiler, and nwnrs-nwscript (GPL-3, excluded by licence). The community's published format documentation for NCS is permitted; it describes the file, not an implementation.

The restriction costs something: a construct nwnnsscomp compiles one way for a reason its source would explain has to be understood from the corpus instead. That cost buys a licence with no questions attached and a design free to make different choices for a different reader, while nwnnsscomp keeps the place it earned.

## Prior work

Everything the project knows about these games was worked out by other people, most of them volunteers, over twenty years. The README's Acknowledgments section credits that work by name, above the usage so it is the first thing a reader sees after the one-line description; the format documentation cites the source for each fact; and this list is the seed of both, not the whole of it. Adding a name that is missing is always a welcome pull request. The permitted inputs, under the clean-room rule, are documentation and measured behaviour; the excluded inputs are compiler source.

- **The NCS format and opcode semantics**: the xoreos project's documentation and its `ncsdis` and `ncsdecomp` tools, which established the instruction set, the stack discipline and the `STORE_STATE` closure convention for every Aurora game.
- **The engine function tables**: BioWare's own `nwscript.nss`, shipped with the games, and the community's annotations of its functions on the KotOR and Neverwinter Nights wikis.
- **The compilers**: Edward T. Smith's `nwnnsscomp`, which has compiled nearly every KotOR mod script ever released and reproduces BioWare's bytes for 91% of the vanilla corpus without ever having been written for KotOR; Beamdog's `nwnsc` in neverwinter.nim; pykotor's compiler, the first cross-platform one for KotOR; `nwnrs-nwscript`. Under the clean-room rule the project measures them rather than reading them, and each is a row in the conformance report with the respect that implies.
- **The decompilers**: DeNCS, whose output taught the community what module scripts do, and pykotor's disassembler.
- **The toolkits**: KotOR Tool, which made the game's archives readable; the Holocron Toolset and PyKotor; TSLPatcher and HoloPatcher, whose installer conventions the shim exists to honour.
- **The corpus**: BioWare, whose decision to ship 1,754 scripts as both source and bytecode is the only reason any of this can be verified.

## Architecture

Crates in a workspace, each with a single responsibility, named under one prefix:

| Crate | Responsibility | Depends on |
| --- | --- | --- |
| `nwscript-ncs` | The NCS file: header, instruction decoding and encoding, the textual assembly form and its parser, a control-flow view. | nothing |
| `nwscript-vm` | The interpreter: stack, frames, STORE_STATE closures, an engine trait, a recording mock engine, the snapshot format. | `ncs` |
| `nwscript-syntax` | Lexer, parser with error recovery, lossless syntax tree, the grammar document the parser is checked against. `nwscript.nss` is parsed by this crate as data. | nothing |
| `nwscript-sema` | Name resolution, types, constant folding, the engine function table from `nwscript.nss`, lints. | `syntax` |
| `nwscript-codegen` | Bytecode from a checked program, per game profile. | `sema`, `ncs` |
| `nwscript-decomp` | Source from bytecode, verified by `codegen`. | `ncs`, `sema`, `codegen` |
| `nwscript-diag` | The diagnostic type, codes, explanations, miette and JSON rendering. | nothing; used by all |
| `nwscript` | The façade crate: one API for compile, decompile, assemble, run, check, with the resolver trait. | all |
| `nwscript-lua` | The Lua binding of the façade: the test and scripting language, with the recording engine's trait exposed as a Lua table. | `nwscript` |
| `nws` | The command line. `nws test` runs Lua tests through `nwscript-lua`. | `nwscript`, `nwscript-lua` |
| `nwnnsscomp` | The compatibility shim: nwnnsscomp's command line mapped onto one library call. Nothing else in the workspace knows its flags. | `nwscript` |

The Lua binding is the model for every other binding: a thin layer over the façade with the same shape of API, so documentation for one is mostly documentation for the next. Later layers follow it: Python bindings through PyO3 so pykotor and modshop call the library directly; a WebAssembly build for a browser playground, which carries the Lua binding with it; a language server and a formatter over the lossless tree.

A **game profile** names the game and version: which `nwscript.nss`, which codegen forms, which engine functions have modelled semantics. KotOR 1 and KotOR 2 are the first two.

## Requirements

### Bytecode

- Decode every opcode and operand form the KotOR engines accept, and encode each back to the same bytes. Decoding an unknown byte sequence is an error with the offset, never a guess.
- Provide a textual assembly form that round-trips: disassemble, assemble, identical bytes. Labels replace absolute offsets so a disassembly reads as a program.
- Provide a control-flow view: basic blocks, edges, subroutine boundaries, the stack depth at each instruction where it is statically determinable. This is the input to the decompiler and to trace comparison.
- Acceptance: every NCS file in a vanilla KotOR 1 install (1,784 in `scripts.bif`, about 10,800 in modules) round-trips through decode and encode, and through disassemble and assemble.

### Interpreter

- Execute NCS exactly as the engine's virtual machine does for every opcode, including STORE_STATE closures and the `ACTION` calling convention.
- The engine is a trait. The default implementation records every engine call with the function id, the arguments popped and the value returned, and returns a declared default per return type. A test supplies its own implementation or a world model for the functions it cares about.
- Ship a small world model behind the engine functions scripts use most: local and global variables, object tags, the party, a seedable random source. Everything outside the model records the call and marks the run as depending on an unmodelled function.
- Tests are Lua. The binding exposes the world, the script, the run and its trace as values, and the engine trait as a table, so a test that depends on a function the world model does not cover defines that function in place, scoped to the test. Recorded runs are data: `nws run --record` writes the world, the entry and the trace as a snapshot file, and `nws test` replays snapshots as tests with no logic in them. Rust tests use the trait directly.
- Define trace equivalence: two runs are equivalent when their recorded engine calls, in order, with arguments, and their results, are equal. This is the second oracle, for scripts whose bytes legitimately differ.
- Acceptance: every vanilla NCS runs under the recording engine without a virtual machine fault; every pair of byte-identical scripts is trace-equivalent, as a check on the interpreter itself.

### Front end

- A hand-written lexer and recursive-descent parser with error recovery, so one mistake produces one diagnostic and the rest of the file is still checked.
- A lossless syntax tree: every byte of the source, including comments and whitespace, is in the tree, and the tree prints back to the source. The formatter and the language server need this; the compiler ignores the trivia.
- A written grammar, in the repository, that the parser is tested against. Constructs the corpus uses that the grammar does not describe are grammar bugs.
- `nwscript.nss` is parsed as data by the same parser. BioWare's typo at line 5145 of KotOR 1's file is a diagnostic with a span, and the rest of the file still loads.
- Acceptance: every vanilla NSS parses, and every one prints back byte-identical from its tree.

### Semantic analysis and code generation

- Types, scopes, constants, struct and vector operations, `action` arguments and the closure they compile to, the engine function table, and the version-gated encodings BioWare's compiler used, all as the corpus shows them.
- Codegen is a pure function of the checked program and the game profile. No lint, flag or mode changes its output.
- Acceptance, staged: first, byte-identical output for at least the 1,598 scripts nwnnsscomp reproduces; then, every script in the corpus whose shipped `nwscript.nss` can declare it; then, for the 18 whose declarations are stale, byte-identical output given the corrected declarations the decompiler recovers from their bytecode.

### Decompiler

- Recover source from bytecode: subroutines, control flow, expressions, types from stack usage and engine function signatures, locals and globals.
- Verify every recovered function by recompiling it and comparing bytes. Output marks each function as verified, unverified with the difference, or not recovered.
- Acceptance: for the vanilla pairs, the recovered source recompiles to the original bytes for a measured and published fraction, and that fraction is the project's headline decompiler number. For module scripts, the verified fraction is published per module.

### Diagnostics

- Every diagnostic has a stable code, a severity, one or more labelled spans, a one-line message, and an explanation reachable from `nws explain CODE`.
- Human output renders through miette. Machine output is one JSON document per run with every diagnostic, the inputs used and their hashes, and the outputs written.
- Exit codes are documented: success, diagnostics at error severity, bad invocation, internal error. A warning never changes the exit code unless asked.

### Agent surface

- **A skill ships with the tool.** A Claude Code plugin, and the equivalent for other agents as they settle on a format, in the repository and installable from it. It teaches the workflow by progressive disclosure: when to compile, check, run, test and decompile; how to read a diagnostic and act on it; how to go from a desired change to a tested script; where the game's own scripts are and how to learn from them. It points at `--help` and `explain` rather than restating them, so it does not rot.
- **`--help` is the manual.** Every command's help text is complete enough to use the command without any other document, with an example. The reference documentation is generated from the same text.
- **Every error is actionable without a person.** The explanation for each code includes the fix, and the diagnostic's help line suggests the specific one where the compiler can tell.
- **The learning loop is closed in milliseconds.** An agent iterating on a script runs `check`, then `test`, then `compile`, and never waits on the game to know whether it is right. The game is for the engine behaviour the interpreter does not model, and the output says when that is the case.
- **Progressive disclosure is measured.** The acceptance test for the skill is a fresh agent, with no conversation history, taking a described change to a compiled and tested script using only the skill, the tool and a game install.

### Command line

One binary, `nws`, with one verb per command. The shape follows the conventions coding agents and build systems already know from cargo and rustc: inputs are positional, outputs are `-o`, `-` means stdin or stdout, every command has `--format json`, and nothing is read from the current directory unless named.

#### Shared flags

| Flag | Meaning |
| --- | --- |
| `--game k1\|k2` | The game profile: which codegen forms, which engine semantics. Required by every command that compiles or runs; it has no default, because a wrong default produces bytes that look right. |
| `-I DIR` | An include directory, repeatable, searched in order after the directory of the file being compiled. `nwscript.nss` is found the same way. |
| `--nwscript FILE` | Use this `nwscript.nss` instead of searching. The project ships no `nwscript.nss`, since it is game data; the error when none is found names the flag. |
| `--game-dir DIR`, `--module STEM` | A game install, the folder holding `chitin.key`. Includes and `nwscript.nss` the explicit directories lack are read from the copies the game loads, in its search order: `Override/`, the capsules of the module `--module` names, then the game's own archives. The order is `rakata-game`'s, the one model of the install; `nws` adds none of its own. Needs `--game`; an install without `nwscript.nss` is `NE0001`, the command's own code. |
| `--format text\|json` | Output format. `text` renders diagnostics through miette and results as tables; `json` writes one document to stdout and nothing else there. Diagnostics go to stderr in text mode and inside the document in JSON mode. |
| `--allow CODE`, `--deny CODE`, `--compat nwnnsscomp\|bioware`, `--deny warnings` | Lint control, as in [Strict by default](#strict-by-default). |
| `-q`, `--color always\|never\|auto` | Quiet suppresses warnings in text mode. `NO_COLOR` is honoured. |

#### Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Succeeded; warnings may have been emitted. |
| 1 | Diagnostics at error severity; no output file was written. |
| 2 | The invocation was wrong: unknown flag, missing input, no `--game`. |
| 3 | Comparison or verification found a difference (`diff`, `decompile --strict`, `test`, `conformance`). |
| 101 | Internal error. The message asks for a bug report and names the input that caused it. |

A warning never changes the exit code unless `--deny warnings` promotes it.

#### The JSON document

Every command's `--format json` output is one object with the same envelope and a command-specific `result`:

```json
{
  "tool": "nws",
  "version": "0.3.0",
  "command": "compile",
  "game": "k1",
  "inputs": [
    {"role": "source", "path": "k_act_com1.nss", "sha256": "…"},
    {"role": "include", "path": "inc/k_inc_generic.nss", "sha256": "…"},
    {"role": "nwscript", "path": "inc/nwscript.nss", "sha256": "…"}
  ],
  "outputs": [{"path": "k_act_com1.ncs", "sha256": "…", "bytes": 3812}],
  "diagnostics": [
    {
      "code": "E0201",
      "severity": "error",
      "message": "no function named `GetFirstPCx`",
      "spans": [{"path": "k_act_com1.nss", "start": {"line": 12, "col": 9, "offset": 311}, "end": {"line": 12, "col": 20, "offset": 322}, "label": "not declared in nwscript.nss or any include"}],
      "help": "did you mean `GetFirstPC`?",
      "explain": "nws explain E0201"
    }
  ],
  "result": {}
}
```

`inputs` lists every file the resolver returned with its hash, so a build system can key a step on them without a separate dependency scan. In text mode the same diagnostic renders as:

```text
error[E0201]: no function named `GetFirstPCx`
  --> k_act_com1.nss:12:9
   |
12 |     o = GetFirstPCx();
   |         ^^^^^^^^^^^ not declared in nwscript.nss or any include
   |
   = help: did you mean `GetFirstPC`?
   = note: run `nws explain E0201` for the full explanation
```

#### Commands

`nws compile FILE... [-o FILE | --out-dir DIR] [--emit ncs,asm]`

Compiles each file independently, sharing one parse of `nwscript.nss` and of each include. The output is `FILE.ncs` beside the input, or `-o` for a single input, or `--out-dir` for many. `--emit asm` writes the assembly beside the bytecode. A file with errors writes nothing and the command continues with the next file, exiting 1 at the end. In JSON mode `result.files[]` carries each file's status, outputs and diagnostics.

`nws check FILE...`

The front end, semantic analysis and lints with no output file. The fast path for an editor or an agent iterating on a script. Exit codes as for `compile`.

`nws disasm FILE.ncs [-o FILE.asm] [--raw] [--cfg]`

The textual assembly form, with labels in place of absolute offsets and the engine function's name on each `ACTION`. `--raw` keeps offsets and numbers. `--cfg` writes the control-flow graph as Graphviz instead.

`nws asm FILE.asm [-o FILE.ncs]`

The inverse. `nws disasm X | nws asm - -o Y` yields `Y` identical to `X`, and the test suite holds that for every vanilla script.

`nws decompile FILE.ncs [-o FILE.nss] [--no-verify] [--strict]`

Source from bytecode. Each recovered function is recompiled and compared, and the output carries one comment per function stating `verified`, `unverified` with the first differing offset, or `not recovered` with the reason. `--strict` exits 3 if any function is not verified. `--no-verify` skips recompilation for speed. In JSON mode `result.functions[]` carries the per-function status, and `result.verified` is the count.

`nws run FILE [--entry NAME] [--arg VALUE]... [--world FILE] [--trace] [--record FILE]`

Runs a script under the recording engine. `FILE` is `.ncs`, or `.nss` to compile first. `--entry` is `main` or `StartingConditional` by default, whichever the script defines. `--world` loads a world snapshot. The result is the script's return value when it has one, and `--trace` prints every engine call in order with its arguments. The output marks the run `depends on unmodelled: GetReputation, …` when any recorded call had no model behind it. `--record` writes the world, the entry, the result and the trace as a snapshot file that `nws test` replays.

`nws test [PATH]... [--filter PATTERN]`

Discovers `*_test.lua` and `*.snapshot.toml` under each path (the current directory when none is given), runs each, and reports like `cargo test`: one line per test, a summary with the failing test's recorded trace, exit 3 on any failure. A snapshot is a recorded run replayed and compared, with no logic. A Lua test:

```lua
local nws = require("nws")
local vault = nws.script("k_tar_openvault", { game = "k1" })

describe("k_tar_openvault", function()
  it("opens after quest step 2", function()
    local w = nws.world()
    w.globals.K_TAR_VAULT = 2
    local run = vault:conditional(w)
    assert.is_true(run.result)
    assert.called(run, "GetGlobalNumber", "K_TAR_VAULT")
    assert.not_called(run, "GetItemPossessedBy")
  end)

  it("opens with the keycard", function()
    local w = nws.world()
    local pc = w:create("creature", "pc")
    w.speaker = pc
    pc:give("tar_vaultcard")
    local run = vault:conditional(w)
    assert.is_true(run.result)
    assert.called(run, "GetItemPossessedBy", nws.any, "tar_vaultcard")
  end)

  it("checks the faction when the quest is stuck", function()
    local w = nws.world()
    w.engine.GetReputation = function(target, source) return 100 end
    assert.is_true(vault:conditional(w).result)
  end)
end)
```

The third test is why the language is Lua and not a data format: a function the world model does not cover is defined in the test, scoped to it, and the run is no longer marked as depending on an unmodelled call. Lua 5.4 is vendored through `mlua`, so there is no system Lua to find, and a type-annotation file for the API ships with the binary for editor completion. The same API, run from `nws lua SCRIPT`, is the scripting layer for batch work over the library: a custom conformance run, a walk over every module asking one question.

`nws diff A B [--level bytes|asm|trace] [--world FILE]`

Compares two scripts, each `.ncs` or `.nss`. Without `--level` it reports all three: whether the bytes are identical, whether the instructions are identical once labels are resolved, and whether the traces under the recording engine are identical. Exit 0 when identical at every requested level, 3 otherwise. The text output shows the first differing instruction with both encodings, and the first differing engine call. This is the command behind the conformance report and the one a repair uses to prove a change did only what it meant.

`nws fn [NAME | --grep PATTERN]`

Prints an engine function's signature and declaration comment from the resolved `nwscript.nss`, or lists matches. Agents call it instead of grepping a 20,000-line file.

`nws explain CODE`

The long explanation for a diagnostic code, with an example that triggers it and one that fixes it.

`nws conformance --corpus DIR --manifest FILE [--compiler NAME=COMMAND]... [--report FILE]`

Runs each compiler over the corpus and writes the report described in [Conformance](#conformance). `DIR` holds `nss/`, `ncs/` and `nwscript.nss` as the user extracted them. `COMMAND` is a template with `{in}`, `{out}` and `{include}` placeholders, so any compiler with a command line can be a row. `nws` itself is always a row.

`nws fmt`, `nws lsp`

Later, over the lossless tree. Reserved so the names do not drift.

#### The nwnnsscomp shim

The installers that mods ship call `nwnnsscomp.exe` with a fixed set of flags, and pykotor's external-compiler seam does the same. The project ships a second binary, `nwnnsscomp`, from its own crate: it parses that command line, makes one library call, writes the output where the original put it, and exits zero or nonzero as the original did. When stdout is not a terminal it prints diagnostics in the original's `FILE(LINE): Error: MESSAGE` shape, since that is what the installers parse. The game profile comes from `NWS_GAME`, defaulting to `k1`, because the original had no such notion.

The shim is the only place the old flags exist. `nws` has no compatibility mode, and the library has no notion of them. The mapping:

| nwnnsscomp | Meaning | Library call |
| --- | --- | --- |
| `-c FILE.nss` | compile | compile |
| `-d FILE.ncs` | disassemble | disassemble |
| `-i DIR` | include directory | include path |
| `-o FILE` | output path | output path |
| `-g` | KotOR output | implied by the profile |
| `-v1.00` | pre-1.30 codegen forms | implied by the profile; `-v1.30` is an error, since no KotOR build wants it |
| `-q` | quiet | quiet |
| `-e`, `-l`, `-r`, `-s`, `-x`, `-t#`, `-p DIR` | optimisation and NWN-specific behaviour | accepted and ignored, with a warning naming the flag |

Dropping the shim binary in place of the Windows one is the whole integration for an existing installer, and for modshop's build.

## Strict by default

Strictness is a lint layer in `nwscript-sema`, separate from code generation. A mode is a set of lints enabled at error severity. The modes:

- **strict**, the default: everything the project has learned to flag, including unused variables and functions, implicit int-to-float conversion, shadowing, unreachable code, and calls whose arguments depend on evaluation order.
- **`--compat nwnnsscomp`**: accepts what the Windows builds of nwnnsscomp accept, so an existing mod compiles unchanged.
- **`--compat bioware`**: accepts what the vanilla corpus needed, including the stale declarations.

Escape hatches at two grains: a per-lint flag, `--allow CODE` and `--deny CODE`, and a per-line comment directive with its own stable grammar, of the form `// nwscript: allow(CODE)`, since the language has no attribute syntax. The vanilla scripts and the modshop build's mods' scripts are the calibration set: a lint that fires on most of them is noise and is demoted before release.

## Conformance

The conformance suite is a command, `nws conformance`, that takes a corpus directory the user extracted from their own game and a list of compilers to run, and produces a report. The repository holds no game bytes: the suite's manifest lists script names and the hashes of their expected bytecode, and the user's game supplies the content.

Each script is classified per compiler as identical, same length, different length, or failed. Scripts that are not identical are further compared by trace, so the report distinguishes a different encoding of the same behaviour from a different behaviour. The report is a table with one row per compiler: BioWare's bytecode as the reference, nwnnsscomp, pykotor, nwnrs-nwscript, and this project. It runs in continuous integration on a machine that has a game installed, and the headline numbers in the README are generated from it.

The existing shell harness in the research document is the suite's first version and is replaced by the command.

The corpus is extracted by `cargo xtask corpus extract --game-dir DIR`, a development task that reads the game's KEY/BIF archives and its module RIM and ERF capsules directly. It writes `nss/` and `ncs/` (every script `chitin.key` lists), `nwscript.nss` as shipped, `oracle/` (the scripts with BioWare's line-5145 typo in `nwscript.nss` corrected, the include directory for nwnnsscomp, which hangs on the shipped file), and `modules/` and `rims/` (each capsule's NCS by archive). It writes to `$NWSCRIPT_CORPUS`, by default `~/.cache/nwscript/corpus/k1`, which is where every corpus-backed test and command looks. `conformance/k1.toml` and `conformance/k1-modules.toml` are the committed manifests, names and sha256 hashes only, and `cargo xtask corpus verify` checks a corpus against them.

## Release readiness

A release is the compiled binaries for every supported platform, signed and attested, with the fixtures a person expects before trusting a tool from a stranger. The first tagged release ships when milestone 4 is reached and everything in this section is in place, which is milestone 7.

**Platforms.** Linux x86-64 and aarch64, macOS arm64 and x86-64, Windows x86-64. Windows is where most existing mod authors and installers are, and it is the platform the shim exists for, so it is a first-class target with its own CI job, not a cross-compile hoped to work. Line endings, path case and backslash separators are tested there.

**Release workflow.** GitHub Actions on a tag: build the matrix, run the full test suite on every platform, assemble archives per platform and an installer script, sign each with cosign through Sigstore keyless signing using the workflow's OIDC identity, attach SLSA build provenance through GitHub's artifact attestations, publish checksums, and publish the crates to crates.io through trusted publishing with no long-lived token. `cargo-dist` generates the matrix and installers so the workflow is declared, not hand-written. A user verifies a download with `cosign verify-blob` and `gh attestation verify`, and the README shows both commands.

**Supply chain.** Dependencies kept current by Renovate: `config:recommended`, the cargo and GitHub Actions managers, actions pinned to digests, lockfile maintenance weekly, grouped minor and patch updates, and automerge for patch updates that pass CI. `cargo-deny` in CI for advisories, licence allow-list and duplicate crates. `cargo-auditable` so a binary carries its dependency list. A software bill of materials attached to each release.

**Continuous integration.** On every pull request: `cargo fmt --check`, `clippy` with warnings as errors, the test suite on all five targets, the documentation build, `cargo-semver-checks` against the last release for the library crates, and the minimum supported Rust version build. The conformance suite needs a game install and therefore cannot run on a public runner: it runs on a self-hosted runner, or by the maintainer before a release, and its report is committed with the input hashes so the published numbers are reproducible by anyone with the game.

**Fixtures.** `LICENSE` (ISC); a `README.md` with an Acknowledgments section near the top, above the usage, naming the people and projects in [Prior work](#prior-work) and what each contributed, and whose conformance table is generated from the suite's report; `CONTRIBUTING.md` with the clean-room rule, the corpus setup and the conformance expectations for a compiler change; `CODE_OF_CONDUCT.md`; `SECURITY.md` with a private reporting channel; `CHANGELOG.md` kept by release-plz from conventional commits; issue templates that ask for the `--format json` output; a pull request template that asks for the conformance delta; `CODEOWNERS`; branch protection requiring CI and a review; documentation on docs.rs for the crates and a published site for the CLI and the skill.

**Versioning.** Semantic versioning per crate, with the façade crate's version as the project's. The bytecode and diagnostic JSON schemas are versioned in the output and documented as stable from 1.0. Before 1.0, the changelog says what broke.

## Milestones

Each milestone has a measurable exit. None is done until its number is published by the suite.

| Milestone | Exit |
| --- | --- |
| 0. Suite | `nws conformance` reproduces the research document's table for nwnnsscomp and pykotor from a manifest and a corpus directory. |
| 1. Bytecode | Every vanilla NCS, data and modules, round-trips through decode and encode and through disassemble and assemble. |
| 2. Interpreter | Every vanilla NCS runs under the recording engine; the Lua API is documented with ten tests over vanilla scripts and `run --record` round-trips through `test`. |
| 3. Front end | Every vanilla NSS parses and prints back byte-identical; `nwscript.nss` loads with the typo reported as a diagnostic. |
| 4. Compiler | At least 1,598 byte-identical; then every script the shipped `nwscript.nss` can declare; the modshop build compiles its mods' scripts with `nws` and the chain hash is unchanged or every difference is explained by trace equivalence. |
| 5. Decompiler | A published verified fraction for the vanilla pairs and for each module of KotOR 1. |
| 6. Surface | The `nwnnsscomp` shim, JSON mode, `explain`, `diff`, Python bindings consumed by modshop. |
| 7. Release | Every item under [Release readiness](#release-readiness) in place; the first signed, attested release on all five targets; the skill passes its fresh-agent acceptance test. |
| 8. Later | Language server, formatter, WebAssembly playground, KotOR 2 profile measured against its corpus. |

## Integration with modshop

The build's compile step calls the toolchain through pykotor's external-compiler seam first, pointed at the `nwnnsscomp` shim, and through the Python bindings once they exist. The toolchain is pinned by version in `mise.toml` like every other tool. The seven pykotor compiler patches are retired when milestone 4 is reached and the chain's hash is accounted for. The research document's harness is retired when milestone 0 is reached.

## Risks and open questions

- **The 18 stale declarations.** BioWare compiled those scripts against a `nwscript.nss` that was not shipped. The decompiler can recover the signatures from the bytecode, but the exact declarations are an inference and the suite marks them so.
- **Encodings the corpus does not show.** A construct no vanilla script uses has no reference bytes. The grammar document carries a list of such constructs and the encoding chosen for each; the engine, run on the handheld, is the only oracle for them.
- **The engine's semantics.** About 800 engine functions, with semantics known only from use. The world model grows by need and the interpreter never claims more than it models.
- **A neighbouring project's licence.** nwnrs-nwscript is GPL-3 and recent, and it is aiming at the same gap. Under the clean-room rule the project cannot read it, so it is measured as a row in the table and credited there; where the two projects can share documentation or test corpora without sharing code, they should.
- **Scope.** Seven milestones is a lot of project. The order is chosen so that each one is useful on its own and the first four pay for themselves in modshop's build.

## Name, licence and repository

The repository is `github.com/ivy/nwscript`, the crates are `nwscript-*`, the binary is `nws`. The names describe the language, which belongs to several games, rather than one game. They are free on crates.io as of 2026-10-16, and they do not collide with `nwnnsscomp` or `nwnsc`. The licence is ISC; the project copies nothing, so nothing constrains it. The repository holds no game bytes: the suite's manifest is names and hashes, and the corpus is extracted by whoever runs it.
