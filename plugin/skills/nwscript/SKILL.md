---
name: nwscript
description: Use when writing, changing, checking, compiling, testing or reading NWScript for Knights of the Old Republic (KotOR, K1/TSL) with the nws command — .nss source, .ncs bytecode, nwscript.nss and engine routines, compiler errors and warnings (E0xxx/W0xxx codes), Lua tests of scripts, decompiling or disassembling the game's scripts to see how they work.
when_to_use: Make this script do X, why won't this script compile, what does E0308 mean, what does this .ncs do, how does the game script Y, write a test for this conditional, did my change alter behaviour, compile a mod's scripts on Linux, which engine routine does Z.
allowed-tools:
  - Read
---

# NWScript with nws

**Autonomy:** model-invocable · guidance only; the commands it leads to write only the outputs they are given

## Before any command

- `nws --version` must work. If it does not, `nws` is not installed: build it from a checkout (`cargo build --release -p nws`) or ask the user where it is.
- Every command that compiles or runs needs `--game k1` (or `k2`); there is no default.
- `nwscript.nss` and includes are found in the script's own directory, then each `-I DIR` in order. Point `-I` at a directory holding the game's `nwscript.nss` and the includes the script uses (`k_inc_*`), or pass `--game-dir DIR` (the folder holding `chitin.key`, with `--module STEM` for a module's own scripts) and they are read from the install as the game loads them; `-I` directories still win.
- `nws fn` needs `nwscript.nss` too: `nws -I DIR fn NAME`.
- `nws <command> --help` is the manual for each command, with examples. Read it rather than guessing flags; this skill does not repeat it.
- Never write into the game folder unless the user asked for the install step.

## Open the reference for the task

Read only the file the task needs, from this skill's `references/` directory:

| Task | File |
|---|---|
| Change what a script does, from a description to an installed script | `workflow.md` |
| A compile error or warning, a code like E0308, lint levels, `--compat` | `diagnostics.md` |
| What a vanilla or mod script does; how the game does something; an engine routine | `learning.md` |
| Prove a script behaves as intended before anyone runs the game | `testing.md` |

## Traps that cross every area

- **Strict is the default.** Lints are errors unless `--compat nwnnsscomp` (sources written for the compilers mods ship) or `--compat bioware` (the game's own scripts). A vanilla script that fails strict is not broken.
- **The game's own scripts need `--declarations bioware`** to compile to BioWare's bytes: most were compiled against an older `ActionStartConversation`, and without it they compile, but six bytes longer per call.
- **`--format json` writes exactly one document to stdout**; parse it instead of scraping text. Exit codes: 0 ok, 1 errors, 2 bad invocation, 3 a comparison or test found a difference.
- **A trace is the recording engine's view.** Calls no model answers are marked unmodelled; their results are defaults, not what the game would return. Say so when a conclusion rests on one.
- **One resref, several scripts.** The same script name can be different scripts in the BIF and in modules; a loose file in `Override` replaces every one. Check before installing (`workflow.md`, step 5).
- **The module scripts ship without source.** Decompile them; the comment above each function says whether it was verified by recompiling to the original bytes.

## Examples

- "Make Carth's join script also give the player 100 credits" → `workflow.md`: find the script, decompile it, change it, check, test, compile, install.
- "× no function named `GetGlobalNumbr`" → `diagnostics.md`: apply the help line (`did you mean GetGlobalNumber?`), or `nws explain E0304`.
- "How does the game decide Bastila's dialogue after Leviathan?" → `learning.md`: find the dialog's conditional scripts, decompile, run with `--trace`.
