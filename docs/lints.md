# Lints and modes

`nws check` and `nws compile` report two kinds of diagnostic. Errors (`E…`) are programs the language does not accept. Lints (`W…`) are programs it accepts that are probably not what was meant. Lints read the checked program and change nothing in it: a program compiles to the same bytes whatever lints, modes and directives are in force.

## Modes

| Mode | Lints | What nwnnsscomp accepted | What BioWare's compiler accepted |
| --- | --- | --- | --- |
| strict, the default | errors | errors | errors |
| `--compat nwnnsscomp` | warnings | warnings | errors |
| `--compat bioware` | warnings | warnings | warnings |

Strict is the default because a script written today is better off without any of these. `--compat nwnnsscomp` accepts what the Windows builds of nwnnsscomp that mods ship accept, so an existing mod checks and compiles unchanged: lints become warnings, and so do the errors in `codes::NWNNSSCOMP_ACCEPTED`, a struct named above its declaration (E0327) and two `case` labels with one value (E0328). `--compat bioware` also accepts what BioWare's own compiler accepted in the vanilla scripts: the codes in `codes::BIOWARE_ACCEPTED`, which are an engine routine called with the wrong number of arguments (E0308) or arguments of the wrong type (E0309), and a definition whose return type disagrees with its prototype (E0311), all from declarations that changed after BioWare compiled.

Within any mode, `--allow CODE` drops a lint, `--deny CODE` makes one an error, and `--deny warnings` makes every remaining warning an error. One line can allow lints with a comment, at its end or alone on the line before it:

```c
int nUnused = 1;   // nwscript: allow(W0001)
// nwscript: allow(W0004, W0001)
int nDroid = 0;
```

The directive's grammar is in [`grammar.md`](grammar.md#comment-directives). Errors cannot be allowed.

## The lints

| Code | What | Explanation |
| --- | --- | --- |
| W0001 | a local variable that is never read | `nws explain W0001` |
| W0003 | a function the entry point never calls | `nws explain W0003` |
| W0004 | a local that hides a parameter, an outer local or a global | `nws explain W0004` |
| W0005 | a statement after `return`, `break` or `continue` | `nws explain W0005` |
| W0006 | arguments whose result depends on their evaluation order | `nws explain W0006` |
| W0007 | a statement that computes a value and discards it | `nws explain W0007` |
| W0008 | an `int` converted to `float` in arithmetic | `nws explain W0008` |

Only functions defined in the file being checked are linted; an include is linted when it is checked on its own. A file without an entry point is a library, so W0003 does not apply to it.

## Calibration

Lints are calibrated against scripts that exist: a lint that fires on most of them would be noise, and would be demoted to a warning in strict mode (`codes::STRICT_WARNINGS`). None does, so the list is empty. The measurement, from `nws check` with `--compat nwnnsscomp` so that every lint is reported:

| Lint | Vanilla KotOR 1: 1,773 scripts | The modshop build's mods: 2,004 scripts |
| --- | --- | --- |
| W0001 | 100 in 37 files | 521 in 212 files |
| W0003 | 8 in 2 files | 1 in 1 file |
| W0004 | 3 in 3 files | 21 in 19 files |
| W0005 | 1 in 1 file | 5 in 5 files |
| W0006 | none | none |
| W0007 | 5 in 3 files | 27 in 7 files |
| W0008 | 3 in 3 files | 84 in 30 files |

The mods' scripts are the `.nss` files in the 192 archives of the modshop build, without the decompiler dumps one mod ships beside its source (`*_BYTECODE.nss`, `*_DOT_GRAPH.nss`, `*_REONE_TOOLS.nss`).

`NWNNSSCOMP_ACCEPTED` comes from the same scripts. Every one `nws check` rejected was compiled with nwnnsscomp as a black box, with the vanilla includes and the mod's own beside it; where nwnnsscomp compiled what `nws` rejected, the error was one of these two, and with them accepted, none of the 54 scripts `--compat nwnnsscomp` still rejects is one nwnnsscomp compiles. Those 54 have unfilled TSLPatcher tokens such as `#2DAMEMORY2#`, includes from other mods, or errors nwnnsscomp reports too.
