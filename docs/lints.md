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

The exit status is 1 when a finding is not in the `--known` list. [`conformance/dialog-known.toml`](../conformance/dialog-known.toml) is the vanilla game's own findings, as dialogue, module, place and code (a place is a node `E3`, a link `E3>R4`, a start `start>E0`, or the dialogue's end script: `end` for `EndConversation`, `abort` for `EndConverAbort`), and `--write-known FILE` writes such a list from a run.

### Where a dialogue is linted

A dialogue in a module's capsules is linted in that module: its scripts, voice-overs and lip sync resolve as the engine resolves them there, `Override/` first. A dialogue of the global layers (the BIFs, and `Override/` where no module's capsules also hold it) plays in whichever module starts it, so it is linted against the install alone, and a script or lip file no global layer holds counts as present when any module's capsules hold it. Linted without its module, a global dialogue would report every module script it names as missing. The lip sync of the global dialogues lies in `lips/localization.mod`, which the engine loads whatever the module; its members count as present everywhere.

### The rules

| Code | What | Vanilla KotOR 1 |
| --- | --- | --- |
| RW1101 | a link to a node the dialogue does not have | none |
| RW1102 | a script, condition or end script no layer holds | 55 |
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

A node's `Script`, a link's `Active`, or the dialogue's own `EndConversation` or `EndConverAbort` names a script no layer holds where the dialogue plays. The engine runs nothing, and a missing condition never passes. An end script lies on no node, so its finding carries no node key: the text names the field (`banter.dlg EndConversation names the script k_x, ...`), the JSON's `key` is null, and the known list's place is `end` or `abort`. The Juhani mod's conditions are scripts of its own, which must be in the install before the dialogues that name them. In vanilla: `dan13_cort.dlg` in `danm13`, E1, names `k_pdan_cort01`, which the game does not ship. Every one of the 195 end scripts vanilla dialogues name resolves (`dan13_elise.dlg` in `danm13` ends with `k_pdan_elise10`, in `danm13_s.rim`).

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

## The tree lint

`rakata lint` checks a game install for the faults a crash or a patcher error would otherwise find: what the files name that the install does not hold, and the `Override/` faults that make the game load the wrong file or none.

```bash
rakata lint --game DIR [--paths FILE] [--db FILE] [--known FILE] [--write-known FILE] [--format json]
```

| Code | Severity | What |
| --- | --- | --- |
| RW1901 | warning | a reference that resolves to nothing in the install |
| RW1902 | warning | a strref in a GFF or 2DA at or past the end of `dialog.tlk` |
| RW1903 | warning | two `Override/` files whose names differ only in case |
| RE1901 | error | a `.tpc` in `Override/` beside a `.tga` or `.dds` of the same name |

The `.tpc` shadow is an error because it has crashed the game; the others are warnings. A run exits 1 when it finds anything the known list does not hold, whatever the severity, and 0 otherwise. Each finding carries its code, its path relative to the tree (`PATH!MEMBER` for an archive member), its site and what it names; `--format json` gives them as `result.findings` beside the diagnostics.

### RW1901: a reference that resolves to nothing

Three kinds of site name a resource:

- A GFF's `CResRef` field, at any depth, in every GFF type the index reads: a creature's scripts and conversation, a placeable's inventory, an instance's template, an area's models. The site is the field's path (`Creature List[2].TemplateResRef`).
- A cell of a 2DA column that holds resrefs (`appearance.2da`'s `modela`, `spells.2da`'s `impactscript`). The site is `row N, column C`, the row by position. Numeric cells are not resrefs.
- A string literal a script's bytecode passes to an engine routine's resref parameter: `ExecuteScript`'s `sScript`, `CreateObject`'s `sTemplate`, `StartNewModule`'s `sModuleName` and `sMovie1`. Which parameters take a resref is read from the install's own `nwscript.nss`, by the words of each parameter's name, as the index reads them (`refs` in [`index.md`](index.md#refsfrom_hash-kind-value-site)). The site is `@OFFSET Routine.param`.

Each site names a type. A GFF field's and a 2DA column's come from the measured table below; a field the table does not type resolves as any type. A script parameter's follows its name: a script is an `ncs`, a dialogue a `dlg`, a sound a `wav`, a movie a `bik`, a module a module; a template is an item's `uti` when the name says item, else the blueprint the call's literal `OBJECT_TYPE_*` creates (constants from `nwscript.nss`), else any blueprint. A site typed `tpc` names a texture, which a `.tpc`, a `.tga` or a `.dds` of the name satisfies: the game reads a `.tga` or `.dds` in `Override/` where it has no `.tpc` (and crashes on both, RE1901).

A name resolves when the install holds a resource of that name and type where the site is loaded. A member of a module's capsule (`modules/STEM.mod`, `STEM.rim`, `STEM_s.rim`, `lips/STEM_loc.mod`) is loaded in that module: it sees the global layers and the module's own capsules. Anything else (`Override/`, a BIF, `patch.erf`, `rims/`) can be loaded in any module, so a name any module holds resolves. The global layers are `Override/` (its top level), `patch.erf`, the BIFs `chitin.key` indexes, the texture packs the install searches (the selected quality pack and `swpc_tex_gui.erf`), `rims/`, the `lips/` capsules no module owns, and the files under `streamwaves/`, `streammusic/`, `streamsounds/` and `movies/`; a module is any capsule stem under `modules/` or `rims/`.

Empty resrefs are not references. A dialogue node's `VO_ResRef` is not checked: BioWare's dialogues name a recording for lines whether or not one was made (24,755 of the 37,989 voice-over names in the vanilla dialogues resolve to nothing, every reply's among them), the engine shows an unvoiced line's text, and `rakata dialog lint` checks voices per dialogue (RW1104).

A GIT waypoint's `TemplateResRef` is not checked either. A waypoint instance holds the whole waypoint (its tag, names, map note and appearance), so the field records the blueprint it was made from, which BioWare's waypoint format describes as such; a creature, placeable, door, sound or trigger instance holds a position and the blueprint the engine loads, and its `TemplateResRef` is checked.

### RW1902: a strref past the talk table

A GFF's `CExoLocString` with a strref and every `StrRef` field, and every integer cell of a 2DA strref column, name a line of `dialog.tlk` by number. One at or past the table's length names nothing. `-1` and `4294967295` mean no line. The site is the field path or `row N, column C`.

### RW1903: case-variant duplicates in Override

Two or more files at the top level of `Override/` whose names are equal without regard to case, as `rakata which --duplicates` lists them ([`Install::override_case_variants`](../crates/rakata-game/src/install.rs)). Each file of a group is a finding, its path the file's, its value the others. Which of them the game opens under Wine is not established. With `--paths`, a group is reported when any of its files is listed.

### RE1901: a `.tpc` shadowed by a `.tga` or `.dds`

A `.tpc` at the top level of `Override/` with a `.tga` or `.dds` of the same stem, in any case. The finding's path is the `.tpc`, its value the files beside it. With `--paths`, it is reported when either is listed.

### The measured table

Which type a GFF field and a 2DA column name, and which 2DA columns hold strrefs, is [`crates/rakata/src/lint/sites.toml`](../crates/rakata/src/lint/sites.toml), measured over an index of the vanilla install by [`scripts/measure-resref-sites.py`](../scripts/measure-resref-sites.py); the file states the method. In short: a site (a GFF field label within a file type, a 2DA column within a table) is typed T when at least 80% of its vanilla values resolve as T and at least three distinct values do; a 2DA column whose values resolve as models only because a texture prefix the engine completes shares the model's name (`appearance.2da`'s `texa` to `texd`) is left out; a strref column is one whose name speaks of a string, whose cells are all integers, at least 60% of which name a line with text, and whose largest value is at least 100. The table holds 66 GFF fields, 129 2DA resref columns in 34 tables, and 75 2DA strref columns.

### Paths, the index, and the known list

`--paths FILE` checks only the files the list names, one per line relative to the tree, marked as a modshop checkpoint's `changed.txt` marks them (removed paths are skipped); a listed archive is checked member by member. Resolution still sees the whole install.

`--db FILE` reads the contents from an index of the tree ([`index.md`](index.md)): the tree whose recorded root is the `--game` folder, its `refs` for the GFFs, `cells` for the 2DAs and `calls` for the scripts, and its `failed` coverage for what did not read. It finds what the walk finds; a test over a built install holds the two equal, and over the vanilla install and the modshop build they agree finding for finding. The `Override/` rules and the names the install holds are read from the folder in either case.

`--known FILE` accepts the findings a list holds, by code, path and site: they are counted, not listed, and do not fail the run. `--write-known FILE` writes every finding of the run in that form:

```toml
[RW1901]
"modules/danm13.rim!module.ifo" = ["Mod_OnPlrDeath", "Mod_OnPlrDying", "Mod_OnSpawnBtnDn"]
```

[`conformance/tree-known.toml`](../conformance/tree-known.toml) is the vanilla KotOR 1 install's own list. A finding the vanilla game has reappears under a new path when a mod repackages its file (a module's `.rim` pair rebuilt as a `.mod`, a table copied into `Override/`), and is new to the list there.

### The vanilla game's findings

Over the vanilla install, 24,801 GFFs, 2DAs and scripts: 549 RW1901, and no RW1902, RW1903 or RE1901. A few of each kind were checked by hand against the index; none of the names resolves as any type anywhere in the install.

| Where | Findings | Example |
| --- | --- | --- |
| `module.ifo` event scripts | 189 | every module's `Mod_OnPlrDeath`, `Mod_OnPlrDying` and `Mod_OnSpawnBtnDn` name Neverwinter Nights' `nw_o0_death`, `nw_o0_dying` and `nw_o0_respawn` |
| 2DA cells | 165 | `genericdoors.2da` rows 0 to 10 name models `t_door01` to `t_door11`; `modulesave.2da` rows 116 to 122 name modules `m12ac` to `m45mg` |
| script literals | 64 | `k_pebn_exithawk.ncs` passes `lev_40ac` to `StartNewModule` (the module is `lev_m40ac`); `k_sup_galaxymap.ncs` passes the movie `5_9`; `k_ptar_zaaltalk.ncs` starts the conversation `tar05_zaalbar`, which no dialogue is |
| dialogues | 38 | `dan13_tareelok.dlg` conditions on `k_pdan_taree01`, `03` and `05` |
| GUIs | 25 | `partyselection.gui`'s `CONTROLS[1].BORDER.FILL` is `po_no` |
| blueprints | 68 | the toolset templates `*.btc` and `*.bic` name NWN's `nw_e0_default*` and `nw_c2_default*` scripts; `c_sebulba.utc`'s `Portrait` is `po_mark1_` |

### Measurements

On the build host (16 threads), with the files in the page cache: the vanilla install in 0.75 s walked and 0.93 s from its index; the modshop build (45,979 resources, 319 findings beyond the vanilla list) in 0.3 to 0.5 s walked and 0.9 to 2.3 s from its index; a checkpoint's `changed.txt` over its tree in 0.2 to 0.3 s (184 paths for `mod:juhani-clones`, 1,209 for `138-crashed-republic-cruiser`). The index answers no faster than the walk: reading every row of `refs`, `cells` and `calls` through SQLite costs more than parsing the GFFs, which are small.
