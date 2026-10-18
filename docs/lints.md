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

## Dialogue lints

`rakata dialog lint` reports what is wrong with a dialogue against the install it plays in. The rules are `rakata_dialog::lint`; the install answers through `rakata::dialog::lint::Linter`. Every finding is a warning (`RW…`) or a note, and none changes a byte.

```bash
rakata dialog lint --all --game $RAKATA_GAME_K1 --known conformance/dialog-known.toml
rakata dialog lint dan14_cutjuh.dlg dan14_juhani.dlg --module danm14ac --game $RAKATA_BUILD
```

The exit status is 1 when a finding is not in the `--known` list. [`conformance/dialog-known.toml`](../conformance/dialog-known.toml) is the vanilla game's own findings, as dialogue, module, place and code, and `--write-known FILE` writes such a list from a run.

### Where a dialogue is linted

A dialogue in a module's capsules is linted in that module: its scripts, voice-overs and lip sync resolve as the engine resolves them there, `Override/` first. A dialogue of the global layers (the BIFs, and `Override/` where no module's capsules also hold it) plays in whichever module starts it, so it is linted against the install alone, and a script or lip file no global layer holds counts as present when any module's capsules hold it. Linted without its module, a global dialogue would report every module script it names as missing. The lip sync of the global dialogues lies in `lips/localization.mod`, which the engine loads whatever the module; its members count as present everywhere.

### The rules

| Code | What | Vanilla KotOR 1 |
| --- | --- | --- |
| RW1101 | a link to a node the dialogue does not have | none |
| RW1102 | a script or condition no layer holds | 55 |
| RW1104 | a voiced node without its voice-over | none |
| RW1105 | a strref at or past the end of the talk table | none |
| RW1106 | a condition reading a global `globalcat.2da` does not declare | 39 |
| RW1107 | a node no start reaches | none |
| RW1108 | a voiced node without its lip sync (a note) | 2 |
| RW1109 | a condition computing the name of the global it reads | none |
| RW1110 | a `SoundExists` that disagrees with the node's audio | 36 |

The counts are findings over the 1,167 dialogues `--all` lints in the vanilla install; the known list holds 115 entries, as several findings share a place (a condition reading several undeclared globals). The run takes 0.6 s.

#### RW1101: a dangling link

A start or link whose `Index` lies past the end of the list it points into: the engine has no node to show. Appending nodes, as the Juhani mod does, keeps every index; a tool that removes nodes without renumbering leaves these. Vanilla has none.

#### RW1102: a script or condition that resolves to nothing

A node's `Script` or a link's `Active` names a script no layer holds where the dialogue plays. The engine runs nothing, and a missing condition never passes. The Juhani mod's conditions are scripts of its own, which must be in the install before the dialogues that name them. In vanilla: `dan13_cort.dlg` in `danm13`, E1, names `k_pdan_cort01`, which the game does not ship.

#### RW1104 and RW1108: a voiced node without its files

A node is voiced when it names a `VO_ResRef`, no `Sound`, and a `SoundExists` other than 0. BioWare's tool writes 0 on the lines it has no recording for (every player reply names a placeholder such as `_m13aabast01001_`), and a node with a `Sound` plays that instead (Zaalbar's Shyriiwook, `n_gwwook_comm1`); counting either as voiced would report some 20,000 vanilla nodes. The voice-over is `streamwaves/` (any subfolder) or a resource layer; the lip sync, a lower-severity note, is any layer or `lips/localization.mod`. The Juhani mod's 80 lines are made at build time, and a line whose file was not written plays silent. In vanilla: no voice-over is missing, and `k_hdroid1_dialog.dlg` E0 (`nm35aadroi01000_`) has no lip file.

#### RW1105: a strref past the talk table

A node's text is a strref at or past the length of `dialog.tlk`, which shows nothing. Vanilla has none, and the Juhani mod's new lines embed their text.

#### RW1107: a node nothing reaches

Reachability is structural: from every start along every link, whatever the conditions answer. A node outside every chain, or reached only from such nodes, is never shown. The Juhani mod appends its nodes and links them from existing ones; a node appended and not linked is lost. Vanilla has none; the modshop build has one, `tar02_janice021.dlg` R69, from another mod.

#### RW1106 and RW1109: the globals a condition reads

The globals are read from the condition's bytecode as `calls.jsonl` reads arguments: the string literal pushed directly before a one-argument `GetGlobal*` call. A name built on a literal (`"OAJ_DOWN_" + IntToString(n)`, as the Juhani mod's `IsDown` builds it) is checked as a family: some declared global must begin with the literal, else RW1106. Any other computed name cannot be checked and is RW1109, never clean. The engine answers 0 for an undeclared global, so a condition on one never changes; the Juhani mod declares its globals in `globalcat.2da` for this reason. In vanilla: `dan13_crattis.dlg` in `danm13`, the start into E19, reads `Kas_ChuunderDead` through `k_pdan_com05`; no vanilla read is undecidable (`k_pman_flow05` reads the family `MAN_WATER_`).

#### RW1110: `SoundExists` against the audio

`SoundExists` 0 on a node whose voice-over is present, or nonzero on a node that names no audio. The Juhani mod's builder writes `SoundExists` 1 on every voiced line it adds. In vanilla: 36 nodes on the Endar Spire, 27 of them in `end_trask01.dlg` (`end_m01aa`), and `end_carth001.dlg` in `end_m01ab` E10 (`n_m1bncart01030_`). Whether the engine plays such a line has not been measured.
