# Reading diagnostics

Every diagnostic has a code. `E` codes are errors, `W` codes are lints (warnings unless a level says otherwise).

```text
E0304

  × no function named `GetGlobalNumbr`
   ╭─[k_my_script.nss:3:18]
 2 │ {
 3 │     int nState = GetGlobalNumbr("K_STATE");
   ·                  ───────┬──────
   ·                         ╰── not declared in this script, its includes, or nwscript.nss
 4 │ }
   ╰────
  help: did you mean `GetGlobalNumber`?
  = note: run `nws explain E0304` for the full explanation
```

- The `help` line is the specific fix when the compiler can tell what it is. Apply it, check again.
- `nws explain CODE` prints what the code means, why it matters, and how to fix it, with examples. Read it before improvising a fix for an unfamiliar code.
- `--format json` gives each diagnostic with `code`, `severity`, `message`, `spans`, `help` and `explain`, for acting on many at once.

## Levels and modes

- Strict is the default: lints are errors, so a file with a never-read variable does not compile.
- `--compat nwnnsscomp` accepts what the nwnnsscomp builds mods ship accepted, lints as warnings. Use it for a mod's existing sources.
- `--compat bioware` accepts what BioWare's own scripts need. Use it for the game's scripts.
- `--allow CODE` and `--deny CODE` (or `--deny warnings`) adjust single codes. A comment `// nwscript: allow(W0001)` at the end of a line, or alone on the line above, allows that lint there; errors cannot be allowed.

A mode never changes the bytes produced, only which diagnostics stop a compile.

## When the declarations are the problem

`nwscript.nss` changed during KotOR's development, and some scripts were compiled against older declarations:

- A vanilla script compiles but differs from BioWare's bytes by six bytes per `ActionStartConversation` call: compile with `--declarations bioware`, the eleven-argument declaration BioWare compiled against.
- A vanilla script calls a routine with arguments the shipped `nwscript.nss` does not take (E0308, or E0309 for a type): `--overlay FILE` applies a file of prototypes after the declarations. The nwscript repository's `conformance/overlays/` has the ones inferred from BioWare's bytecode, and `nws --help` describes the format.

For a new script, write against the shipped declarations; do not reach for an overlay to make a call compile.
