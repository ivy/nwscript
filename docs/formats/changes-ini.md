# TSLPatcher's `changes.ini` and `namespaces.ini`

The files a KotOR mod ships in its `tslpatchdata` folder to tell TSLPatcher what to change, as the mods in modshop's build write them and as stoffe's TSLPatcher documentation and TSLPatcher's own logs describe their meaning. `crates/rakata-patch` is this document as code: its `Ini` reader is the dialect, its `Changes` model the sections, its `Plan` the order, its executor the semantics. Counts below come from `cargo xtask ini-census` over the 80 mods of the build that carry an ini (151 changes files and 34 `namespaces.ini`), run on 2026-10-17; the name of a mod beside a fact is where the fact was seen.

## The dialect

Windows INI, as the Windows `GetPrivateProfile*` functions read it and nothing more:

- A section header is `[` to `]` on its own line; surrounding whitespace is dropped. An entry is `Key=Value`; whitespace around the key and the value is dropped, and the first `=` splits (`WindowCaption=a=b` has the value `a=b`). A comment line starts with `;`. Blank lines mean nothing. No other line form occurs in the 185 files: no `//` or `#` comments, no inline comments after a value, no quoting (a value such as `RequiredMsg="K2 Force Powers for K1" must be installed first` keeps its quotes as text).
- Section names and keys are matched without regard to case: the lists are written `[2DAList]` and `[2dalist]`, folders `Override` and `override`, flags `File0` and `file0`.
- When a name repeats, the first section of that name and the first key of that name within a section are what the INI functions answer with; the second is unread. Repeats occur (11 in the corpus: a `File5` written twice in a `[GFFList]`, Swoop Bike Upgrades; a `dietoroll` written twice in a row section, JC's Blaster Adjustment; a struct's sections written twice, Alignment Affects Force Powers). The reader keeps the first and reports the repeat under `RW0303`. That TSLPatcher itself answers with the first is the documented behaviour of the Windows functions, not a measurement of the patcher.
- Bytes are Windows-1252; three files carry bytes above 0x7F in a `Description` (Dodonna's Transmission). A file that is valid UTF-8 with such bytes is read as UTF-8, since a modern editor wrote it. A UTF-8 byte-order mark is dropped. Line endings are CRLF in all but a few files; LF is accepted.
- An entry's value may be empty (2,987 `Label=` under struct additions, `Path=`, `Value=`): empty means "none", not an error.
- Keys of a list are numbered (`File0`, `Replace3`, `Table0`, `install_folder0`, `ChangeRow12`, `AddField7`, `StrRef0`, `Namespace1`) but processed in file order, as the documentation says ("everything is done in the order it is listed"); the numbers are labels, need not be dense (`ChangeRow12`, `ChangeRow13` after `ChangeRow0`..`ChangeRow11` with a gap), and start at 0 or 1 (`Namespace0`, Blaster Adjustment; `Namespace1` everywhere else).

## The sections

A `changes.ini` has `[Settings]` and up to seven lists. Each list entry names, in its value, a section elsewhere in the file holding that entry's keys, except `[InstallList]`, whose folder sections are named by the entry's key. An entry naming a section the file does not have is skipped by TSLPatcher without a word in its log (the two orphan `ChangeRow`s of JC's Blaster Adjustment's `pistol_rifle.ini`, measured under Wine: "Done. All changes have been applied"); the reader reports `RW0301` and plans without it. A key a section's vocabulary does not have is kept on the model's `unknown` list and reported under `RW0302`; the corpus has one, an `install_folder0=Override` written inside `[install_folder0]` (Ixgil the Bith).

### `[Settings]`

All 151 files write `FileExists`, `LogLevel`, `InstallerMode`, `BackupFiles`, `PlaintextLog`, `LookupGameFolder`, `LookupGameNumber`, `SaveProcessedScripts` (the set ChangeEdit writes), 141 `ConfirmMessage`, 125 `WindowCaption`. The flags take 0 or 1, `LogLevel` 0 to 4, `LookupGameNumber` 1 (KotOR) or 2 (TSL). `Required=FILE` names a file that must exist in `override` before the install proceeds and `RequiredMsg=TEXT` the message when it does not; the readme documents them as a pair, the mods write twelve `Required` and eleven `RequiredMsg`: three `Required` without a message (Republic Soldier's New Shade), and modshop's series records a lone `RequiredMsg=0` and a lone empty `Required=` that TSLPatcher accepts. The reader pairs them by rank, a file without a message gets the patcher's own text, a message without a file requires nothing, an empty file name requires nothing.

### `[TLKList]`

- `StrRef#=N`: entry `N` of `append.tlk` is appended to `dialog.tlk`, and the token `StrRef#` means the index it received, everywhere a token may stand (161 in the corpus; K1CP has 161 of them alone). An entry equal to an existing `dialog.tlk` entry in every field (flags, sound, volume and pitch variance, sound length, text) is not appended and its token takes the first such index, as the readme says and TSLPatcher 1.2.10b1 does ("Identical string for append StrRef 0 found in dialog.tlk StrRef 36850, reusing it instead."); equal text with other flags or another sound length is appended; two tokens naming one entry share its index ([`../research/2026-10-17-tslpatcher-semantics.md`](../research/2026-10-17-tslpatcher-semantics.md)).
- `Replace#=FILE` (K1CP, `append.tlk`) and `ReplaceFile#=FILE` (Multifire and Autofire, `append.tlk`; Rebalanced Grenades, `replace.tlk`) name a TLK whose entries overwrite existing `dialog.tlk` entries, as `[FILE]` maps them: `25859=27` (K1CP) and `StrRef223=0` (Rebalanced Grenades) both mean "`dialog.tlk` entry on the left takes the file's entry on the right". TSLPatcher 1.2.10b1 ignores such sections in either spelling, without a log line: replacing entries is a feature of other installers (K1CP ships HoloPatcher). The reader accepts both spellings and `rakata install` applies them.
- `!SourceFile` and `!SourceFileF` (changelog 1.2.8b) name the file appended instead of `append.tlk`, the second for the female TLK; neither occurs in the corpus.

### `[InstallList]`

`install_folder#=DEST` (416): `DEST` is a folder under the game, `Override` (100), `modules` (63), `StreamWaves`, `StreamSounds`, `StreamMusic`, `StreamVoice\m12aa` (a sub-folder, 25), or a capsule, `modules\danm13.mod` (219, K1CP). The files are listed in `[install_folder#]`, the section named by the key: `File#=NAME` copies unless a file of that name exists, which the log warns about and skips (699); `Replace#=NAME` overwrites (6,805). A file may have a section of its own, `[NAME]`, with `!SaveAs=` (the name to install under, 6, Party Conversations on the Ebon Hawk: `EBO_k_pebn_pophawk.ncs` and `TAR_k_pebn_pophawk.ncs` both installed as `k_pebn_pophawk.ncs` into different capsules) and `!SourceFile=` (the file to copy instead); a section of that name may also belong to a 2DA or GFF the mod both installs and edits, so other keys there are that list's.

TSLPatcher's own log (Taris Rapid Transit, Sith Uniform Reformation Revised, shipped as `installlog.rtf`) writes "Installing unmodified files..." before "Modifying GFF format files...".

### `[2DAList]`

`Table#=FILE.2da` (104); `[FILE.2da]` lists the modifiers, `ChangeRow#` (758), `AddRow#` (219), `CopyRow#` and `AddColumn#` (neither in the corpus; from the readme), each naming its section.

- A row to change or copy is found by `RowIndex=N` (750), `RowLabel=L` (8, the far-left column), or `LabelIndex=L` (the row whose `label` column is `L`; readme; TSLPatcher 1.2.10b1 does not read it). A row that does not exist (`RowIndex` past the end) is an error in TSLPatcher's log, the modifier is skipped and the install goes on. Column names match without regard to case.
- Every other key of a row section is a column assignment, `COLUMN=VALUE` (5,533), except `2DAMEMORY#=SOURCE`, which stores into memory slot `#`: `RowIndex` (147, the row's index, after an `AddRow` the new row's), `RowLabel`, or a column's name (that column's value in the row).
- `AddRow`: `ExclusiveColumn=COLUMN` (138) adds only when no row has the new row's value in that column, and changes that row instead when one does. `RowLabel=` sets the new row's label; absent, the label is the row's index.
- `CopyRow` takes the row selectors, `ExclusiveColumn`, and `NewRowLabel=` for the copy's label.
- `AddColumn`: `ColumnLabel=` (at most 16 characters), `DefaultValue=` for every row, `I#=VALUE` for the row at index `#`, `L#=VALUE` for the row labelled `#`, and `2DAMEMORY#=I#` or `L#` to store the new column's value in that row.
- Values: text as written, `****` (257, the engine's empty cell), `high()` (2; in a cell, one more than the column's highest number; as a label, the next free label), `2DAMEMORY#` (112) and `StrRef#` (217) tokens.

### `[GFFList]`

`File#=NAME` (2,087; `Replace#` is accepted the same way); `[NAME]` has the file's settings and its changes.

- `!Destination=` (1,931) a folder or capsule, `override` when absent; written with `\` and, in four entries, `/` (Party Conversations). `!Filename=` (548) the name in the destination (the build of TSLPatcher with a blank version resource reads it so, in `tslpatchdata` as in the destination; 1.2.10b1 does not know the key and takes it for a field path); `!SaveAs=` (17) the same for a file installed under another name; `!SourceFile=` the file in `tslpatchdata` to start from (changelog); `!ReplaceFile=1` (7; 131 write `=0`) copies the fresh file from `tslpatchdata` over the destination before editing, where the default edits the file already there; `!OverrideType=` `ignore`, `warn` or `rename` for a copy of the same name in `override` when the destination is a capsule (changelog; none in the corpus).
- `PATH=VALUE` (12,629) sets an existing field; `PATH` is labels and list indices joined by `\` (`Door List\0\TransitionDestin`, `PropertiesList\0\Subtype`), case-sensitive. `PATH(strref)=` (150) sets an ExoLocString's strref; `PATH(lang#)=` (19) sets its substring for language-and-gender id `#` (`FirstName(lang0)=Senni Vek`). `2DAMEMORY#=VALUE` at this level sets the field whose path the slot holds (readme: it stores nothing here; measured: a `2DAMEMORY2=9` after the `AddField` that stored slot 2's `!FieldPath` sets the added field). Edits and additions run in the order the section lists them. A label in another case, or a field that does not exist, is a warning in TSLPatcher's log and the edit is skipped. A decimal comma parses as a decimal point in a Float, a Position and an Orientation (`1,5` is 1.5, under Wine's default locale).
- `AddField#=SECTION` (9,199) adds a field described by `[SECTION]`: `FieldType=` by name (`Byte`, `Char`, `Word`, `Short`, `Dword`, `Int`, `Dword64`, `Int64`, `Float`, `Double`, `ExoString`, `ResRef`, `ExoLocString`, `Void`, `Struct`, `List`, `Orientation`, `Position`) or by the GFF type id 0 to 17 in that order; `Label=` (at most 16 characters, empty for a struct in a list); `Path=` where to add it, the root when empty, never set on a nested field, which goes into its parent; `Value=`; for an ExoLocString `StrRef=` (`-1` for none; 291) and `lang#=` substrings; for a struct `TypeId=` (1,462); a struct's or list's own `AddField#=` for its children (6,762 nested); `2DAMEMORY#=ListIndex` (4) stores the index the struct was added to its list at, `2DAMEMORY#=!FieldPath` the added field's full path. `Orientation` is four and `Position` three decimals joined by `|`, written with the author's decimal separator (`92,1600036621094|62,5499992370605|13,7505683898926`, Czerka Guard; a comma, as the German or French Windows the author used writes decimals).
- 152 edits and 107 additions take a token as their value (`Appearance_Type=2DAMEMORY1`, `Value=StrRef0`).

### `[HACKList]`

`File#=NAME` or `Replace#=NAME`; `[NAME]`: `!Destination=`, `ReplaceFile=0|1`, then `OFFSET=VALUE` writes `VALUE` as a 32-bit unsigned big-endian integer at byte `OFFSET`, both decimal (`16=300` writes `00 00 01 2C` at bytes 16 to 19), the byte order of NCS bytecode; a hexadecimal offset or a negative value is refused with a warning (measured on TSLPatcher 1.2.10b1). Five files write the header and leave it empty; no mod in the build hacks bytes.

### `[CompileList]`

`Replace#=NAME.nss` (40, all four compiling mods) or `File#=`: compile the script, with `#2DAMEMORY#` and `#StrRef#` in its source replaced by the tokens' values, to `NAME.ncs`; `File#` skips an existing `.ncs` with a warning, `Replace#` overwrites. `!DefaultDestination=` at the list level and `!Destination=` in the script's own `[NAME.nss]` (6, Bastila has TSL Battle Meditation) send the result into a capsule; `override` otherwise. The flag is the first of the eight defects in modshop's series.

### `[SSFList]`

`Replace#=NAME.ssf` (2, K1CP) or `File#=`; `[NAME.ssf]`: `SOUND=VALUE` with the sound named as ChangeEdit spells it (`Battlecry 1`..`6`, `Attack 1`..`3`, `Critical hit`, `Low health`, `Pain 1`, `Pain 2`, `Death`, and the rest of the SSF's 28 slots, each writing one slot of [`ssf.md`](ssf.md)'s table) and a `StrRef#` token or a number as the value.

## `namespaces.ini`

An installer offering options has `[Namespaces]` with `Namespace#=SECTION` (103 options over 34 files, one to six per file) and, per option, `[SECTION]` with `Name=` (103), `Description=` (98), `IniName=` (100), `InfoName=` (100) and `DataPath=` (88). `IniName` and `InfoName` default to `changes.ini` and `info.rtf` when absent or empty (JC's Mandalorian Armor writes neither in any of its three options; TSLPatcher installs it); `DataPath` is a folder under `tslpatchdata` holding that option's files, `tslpatchdata` itself when absent. The fourth of the eight defects.

## The order of the phases

TSLPatcher runs the lists as phases in a fixed order, each in the order listed: TLK appending, InstallList, 2DAList, GFFList, HACKList, CompileList, SSFList. The source is the readme's changelog for 1.2.8b0 ("The different install phases now run in this order: TLK Appending, Install List, 2DA changes, GFF Changes, ... Script compilation, SSF Editing"), with the byte hacks placed after the GFF changes from stoffe's forum description of an earlier order (TLK, 2DA, GFF, hacks, install list). TSLPatcher 1.2.10b1 run under Wine on Bastila Has Battle Meditation, a mod with every list non-empty, logs the phases in this order, TLK, Install, 2DA, GFF, Compile (`docs/research/tslpatcher-oracle.md`); its shipped logs confirm Install before GFF (above), and the data confirms 2DA before GFF: 137 GFF values in the corpus read `2DAMEMORY` tokens that only 2DA operations set (`Appearance_Type=2DAMEMORY1` after an `AddRow` with `2DAMEMORY1=RowIndex`). A token read before any earlier operation sets it is `RW0304`; the readme: "If you try to read a token before it is set, bad things will happen."

## Where each list reads and writes

Measured on TSLPatcher 1.2.10b1 and the build with a blank version resource, with hand-made mods (`docs/research/2026-10-17-tslpatcher-semantics.md`):

- `[TLKList]` appends to and replaces in the game folder's `dialog.tlk`; the source TLK is in the option's folder.
- `[InstallList]` copies from the option's folder into the destination folder or capsule. `File#` skips a name the destination holds, with a warning, in a folder and in a capsule; `Replace#` overwrites, and a `Replace#` into a `.mod` that holds the member leaves the new bytes there. A destination capsule that does not exist is an error and the section is skipped.
- `[2DAList]`, `[GFFList]` to a folder, `[SSFList]` and `[HACKList]` edit the destination's copy (`Override`) when it has one; otherwise they copy the option's file there and edit that. `!ReplaceFile=1` copies the option's file over the destination's first. A file in neither is a "locate file" dialog, and the install stops: TSLPatcher never reads the BIFs or a module's RIMs.
- `[GFFList]` into a capsule edits the member when the capsule holds it and adds the option's file otherwise; the module's `.rim` and `_s.rim` are not read. A capsule that does not exist is an error and the file is skipped.
- `[CompileList]` writes `NAME.ncs` to `override` or `!Destination`, with `File#` skipping an existing one.

`rakata install` reads and writes the same places in the same order, with the differences the next section names, and after every write reads the file or member back (`crates/rakata-patch`, `exec`).

## Where `rakata install` differs from TSLPatcher

Each difference is deliberate, does what the game needs where TSLPatcher would stop or write what the game misreads, and has a name for the parity comparison:

- `loaded-copy`: a file to edit that neither the destination nor the option's folder holds is taken from the copy the game loads, through `rakata-game` (the BIFs, `patch.erf`), where TSLPatcher asks the user for it and stops.
- `mod-from-rims`: a `.mod` that does not exist is made from the module's `.rim` and `_s.rim` (and KotOR 2's `_dlg.erf`), the first copy of a name winning, and the member goes into it; TSLPatcher skips the entry. A `.mod` replaces the module's RIMs in the engine (`docs/formats/install.md`), so one made with a single member would empty the module. A member of a `.mod` not made yet is read from those RIMs.
- `continue-on-error`: an operation that cannot apply is an error in the report and the install goes on; TSLPatcher stops at a missing file.
- `gff-missing-field-error`: a GFF edit naming a field the file lacks (a label in another case included) is an error, `RE0309`, where TSLPatcher logs a warning and skips it.
- `substring-added`: `PATH(lang#)=TEXT` on a string without that substring adds it; TSLPatcher logs the change and writes nothing.
- `tlk-replace`: `Replace#`/`ReplaceFile#` sections of `[TLKList]` are applied; TSLPatcher 1.2.10b1 ignores them.
- `label-index`: `LabelIndex=` finds a row by its `label` column; TSLPatcher 1.2.10b1 does not read the key and skips the modifier.
- `filename-key`: `!Filename=` names the file in the destination and, when no file of the `File#` name is in the option's folder, there too, as the blank-version build reads it; 1.2.10b1 takes it for a field path.
- `hack-hex-offset`: a `0x`-prefixed HACKList offset is read as hexadecimal; TSLPatcher refuses it.
- `compiler`: a `[CompileList]` compiles through `nwscript`, as `nws compile --game-dir GAME` does, with includes and `nwscript.nss` from the option's folder and then the install; TSLPatcher runs the `nwnnsscomp.exe` the mod ships.

## What could not be established

How TSLPatcher parses a decimal comma under a Windows locale whose separator is the comma (the container's Wine runs the default locale, where both parse). `!OverrideType` (no mod in the build uses it). `[SSFList]` and `[HACKList]` into a capsule. Whether `2DAMEMORY#` under a GFF section's own keys behaves the same in builds other than 1.2.10b1. Each needs TSLPatcher run under Wine on a prepared input; `docs/research/` is where such a measurement goes.

## Provenance

Public documents read: stoffe's TSLPatcher v1.2.10b1 readme (the ChangeEdit guide and its changelog, through a public mirror of the document), and stoffe's release thread for TSLPatcher on the LucasForums archive (the processing order and the token rules in posts 14, 27 and 32). Neither the KotOR wiki nor deadlystream reproduces the readme's syntax reference.

Mods' files read: every `.ini` under `tslpatchdata` in the 192 archives of modshop's `games/kotor/data/archives/`, unpacked with 7-Zip into a scratch directory (80 mods carry inis; the rest are textures and movies), and the three `installlog.rtf`/`.txt` files mod authors shipped beside them, two of them TSLPatcher's own logs and one HoloPatcher's. Nothing from them is in the repository; the fixtures under `crates/rakata-patch/tests/fixtures/` are hand-written in their shapes.

Reference projects read: none. The clean-room hook refused the only attempt, a listing of modshop's `patches/pykotor/` series; the four reader defects it fixes are taken from the prose of modshop's `patches/README.md` and `repairs/09-blaster-adjustment-orphan-rows/README.md`.

Measured: TSLPatcher 1.2.10b1 and the build with a blank version resource, run under Wine as black boxes on hand-made mods, for the read-and-write rules and the semantics above ([`../research/2026-10-17-tslpatcher-semantics.md`](../research/2026-10-17-tslpatcher-semantics.md)); the counts above, by `cargo xtask ini-census` over the unpacked archives on 2026-10-17: 185 files parse with no error and 14 warnings (2 `RW0301`, 1 `RW0302`, 11 `RW0303`), every one a fact about the mod and not a construct the reader lacks.

Written by an implementer; no reference source was opened.
