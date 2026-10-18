---
name: rakata
description: Use when reading or changing the game data of Knights of the Old Republic (KotOR, K1/TSL) with the rakata command or its Python module — which copy of a resource the game loads, dialogues (.dlg) and their nodes, replies, links and conditions, globals and globalcat.2da, 2DA tables, GFF blueprints (.utc, .uti, .utp), dialog.tlk strings, what refers to a strref, script, tag or global, writing a mod's files into Override, linting a dialogue or an install, diffing two installs, diagnostic codes RE0xxx/RW1xxx.
when_to_use: Add a reply to this conversation, make this line only show when a global is set, which globalcat.2da does the game use, what refers to this script, who sets this object's plot flag, what writes each entry of this quest, who speaks strref 42724, does my changed dialogue have dangling links, what did my mod change, what does RW1106 mean, declare a new global, edit a creature's blueprint.
allowed-tools:
  - Read
---

# Game data with rakata

**Autonomy:** model-invocable · guidance only; the commands it leads to write only the outputs they are given

## Before any command

- `rakata --version` must work. If it does not, Rakata is not installed: ask the user where it is.
- Every command that reads the game needs `--game DIR`, the folder holding `chitin.key`; there is no default. A resource that lives in a module also needs `--module STEM` for `rakata which` and `rakata cat` (`rakata dialog show` and `rakata dialog lint` find the one module holding a vanilla dialogue on their own, but once the dialogue is in `Override/` they need `--module` too).
- `rakata <command> --help` is the manual for each command, with examples; `rakata explain CODE` explains every diagnostic code. Read them rather than guessing flags; this skill does not repeat them.
- Changes go through the Python module `rakata` (`references/author.md`); the command line reads, checks and compares.
- **Never write into the game folder the user plays from** unless they asked for that. Work in a copy (`references/author.md`, "A scratch install") and write only into its `Override/`.

## Open the reference for the task

Read only the file the task needs, from this skill's `references/` directory:

| Task | File |
|---|---|
| Which copy the game loads; read a resource, a table, a string, a dialogue, an item's properties, an area's instances, one script decompiled | `find.md` |
| What refers to a strref, script, tag or global; who sets and reads a flag or global; a quest's journal entries and their writers; search the install | `refs.md` |
| Make a change: add dialogue nodes and links, declare a global, compile a condition, edit a table or blueprint, write to Override | `author.md` |
| Check a change: lint a dialogue or the install, diff against the original | `check.md` |
| Working in the Rakata repository itself (tests, game files, the clean room) | `repository.md` |

The usual change runs through all four in order: find the copy, find what touches it, author, check.

## Traps that cross every area

- **One name, several copies.** The engine searches `Override/`, then the module's capsules, then `patch.erf`, then the BIFs. A file in `Override/` replaces every copy of that name in every module. `rakata which NAME.EXT --module STEM --game DIR` shows the winner and every loser; check it before writing a name to Override.
- **Address nodes by stable key, never by index alone.** `voice:RESREF` or `strref:N` survive other mods' insertions; `E3` and `R0` move and are marked fragile. Text keys often match several nodes.
- **Write what a dialogue refers to before the dialogue.** A dialogue write is checked against the install and refused (RE1407) if a script, condition or global it adds is missing: compile and write scripts, declare globals, then write the dialogue.
- **`--format json` writes exactly one document to stdout**; parse it instead of scraping text. Exit codes: 0 ok, 1 errors reported (a lint with a finding), 2 bad invocation, 3 a diff found a difference.
- **Text is compact for you.** With `CLAUDECODE=1` (or another agent's variable) and no `--format`, tables print unpadded and `index query` prints TSV; transcripts are unchanged. `--format text` gives the aligned form a person sees.
- **Vanilla has findings.** The shipped game already has dangling scripts and undeclared globals; a finding on a node you did not touch is not yours. Lint the original first so you know what was there.
- **Scripts are a separate toolchain.** Writing or reading NWScript beyond a short condition is the `nwscript` skill's (`nws`); the Python module compiles through the same compiler.

## Examples

- "Give this character a new reply that only shows until the player has asked about X" → `find.md` (show the dialogue, pick the entry's voice key), `author.md` (compile the condition, declare the global, add the reply, link it with the condition, write), `check.md` (lint, diff).
- "What uses the global `OAJ_DONE`?" → `refs.md`: `rakata refs OAJ_DONE --kind global --db FILE`.
- "Which choice sets the statue's flag, and which journal entry does each sword write?" → `refs.md`: `rakata flags k37_inv_statue --module korr_m37aa --db FILE`, `rakata quest kor37_ajuntapall --db FILE`.
- "× RW1106 … reads the global …, and globalcat.2da does not declare it" → `rakata explain RW1106`, then `author.md`, "Globals".
