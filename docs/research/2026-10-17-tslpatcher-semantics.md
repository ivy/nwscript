# TSLPatcher's semantics, measured on hand-made mods

2026-10-17. What TSLPatcher does with the inputs `docs/formats/changes-ini.md` lists as not established, and where each list reads the file it changes. Every fact here comes from running TSLPatcher as a black box under Wine through [`../../conformance/drivers/tslpatcher.sh`](../../conformance/drivers/tslpatcher.sh) on a mod written for the question, then reading its log and the bytes it left.

## Method

The patcher is TSLPatcher 1.2.10b1, SHA-256 `013655cd871a71e0f5b50f80ed0d32308912916ba7e12e2a366f42d992a71b16` (the build `docs/research/tslpatcher-oracle.md` lists, taken from entry 180's archive). Three runs repeat a question with the build whose version resource is blank, `1a267454…` (footer "TSLPatcher 1.2.9b", 41 entries of the build), and say so.

Each mod is a `tslpatchdata/` with a `changes.ini` (the `[Settings]` ChangeEdit writes, with `LogLevel=4`, `PlaintextLog=1`, `LookupGameFolder=0`), an `info.rtf`, and small GFF, 2DA, TLK, SSF and NCS files built by a script from the public layouts (BioWare's GFF, 2DA, TalkTable, KeyBIF and ERF documents in xoreos-docs). `PlaintextLog=1` makes the patcher write `installlog.txt` beside its executable; the lines below are quoted from it. The game is a fresh copy, per run, of a vanilla KotOR 1 install's `chitin.key`, `dialog.tlk`, `swkotor.exe`, `swkotor.ini`, `data/templates.bif`, `data/2da.bif`, `data/scripts.bif` and the `modules/danm1*` capsules, with an empty `Override/`; where a question needs a file already in place (a `.mod`, an Override copy) the script puts it there before the run. Results were read with the same script. Each run takes 13 to 15 s through the driver. No mod author's file is used, and nothing from these runs is in the repository.

## Where each list reads its target

| List and destination | Target already there | Not there |
| --- | --- | --- |
| `[2DAList]` | `Override/X.2da` is edited: `Modifying file "appearance.2da" found in Override folder...` | the copy in `tslpatchdata` is copied to Override and edited (`Copying file spells.2da to Override folder...`), even when the BIFs hold the table; with no copy in `tslpatchdata` either, `No file blueprint found in tslpatchdata folder, fallback to manual source...`, a "File not found! Please locate…" dialog, and `[Error] Cannot open file "Z:\mod\feat.2da"`: the install stops there |
| `[GFFList]`, `override` | the Override copy is edited (`Modifying file "ovr_keep.uti" found in Override folder...`); with `!ReplaceFile=1` the `tslpatchdata` copy overwrites it first (`Updating and replacing file ovr_repl.uti in Override folder...`) | the `tslpatchdata` copy is copied and edited; a file only in the BIFs is not found, as for 2DA, and the install stops |
| `[GFFList]`, `modules\X.mod` | the member is edited (`File "dan15_guarda.utc" already exists in archive "danm15.mod", modifying existing file...`); with `!ReplaceFile=1` the `tslpatchdata` copy replaces it first | the `tslpatchdata` copy is added (`Adding file "dan15_mem.utc" to archive "danm15.mod"...`); the module's `.rim` and `_s.rim` are not read; no copy in `tslpatchdata` is `[Error] Cannot open file …` and the install stops |
| `[GFFList]`, a `.mod` that does not exist | | `[Error] Unable to locate archive "Z:\game\modules\danm13.mod" to modify or insert file "dan13_dorak.utc" into, skipping...`, then the next file; no `.mod` is made from the `.rim` |
| `[InstallList]`, `modules\X.mod` that does not exist | | `[Error] Destination file "modules\danm16.mod" does not exist at the specified location! Skipping section...` |
| `[SSFList]`, `[HACKList]` | not measured with a file in place | the `tslpatchdata` copy is copied to Override and edited; a sound set only in the BIFs is not found and stops the install |

So TSLPatcher never reads the game's BIFs or a module's RIMs: a file it changes comes from the destination or from `tslpatchdata`, and a missing one is a dialog asking the user for it. The 1.2.9b-footer build behaves the same for a 2DA only in the BIFs and a GFF into a `.mod` that does not exist.

When a GFF goes into a capsule and Override also holds a file of that name, the patcher edits the capsule's member and logs `Warning: A file named dan15_guarda.utc already exists in the override folder. It will override the one in the ERF/RIM archive in-game.` at debug level; it is not counted as a warning.

`!Filename` and `!SaveAs` differ between the builds. 1.2.10b1 does not know `!Filename`: it treats the key as a field path (`[Warning] Unable to find a field label matching "!Filename" in src_name.uti, skipping...`) and edits the file under the `File#` name, in Override or in the capsule alike. The 1.2.9b-footer build reads `!Filename` as the file's name both in `tslpatchdata` and in the destination (`GFF FILE: src_name.uti and dst_name.uti.`, then `Copying file dst_name.uti to Override folder...`; without a `dst_name.uti` in `tslpatchdata`, the locate dialog). Both builds read `!SaveAs` as the name the `tslpatchdata` file is installed under: `Copying file "src2.uti" as "dst2.uti" to Override folder...`.

## `[InstallList]`

`File#` of a name the destination holds is skipped with `[Warning] A file named i_exist.ncs already exists in the Override folder. Skipping file...`, in a folder and in a capsule (`... already exists in the modules\danm15.mod archive. Skipping file...`). `Replace#` overwrites (`Replacing file i_repl.ncs in the Override folder...`); into a `.mod` holding the member, the member afterwards holds the `tslpatchdata` bytes (`Replacing file dan15_guarda.utc in the modules\danm15.mod archive...`), so this TSLPatcher does not have HoloPatcher's sixth defect. A new member is added (`Adding file zz_inst.ncs to the modules\danm15.mod archive...`).

## `[TLKList]`

- An `append.tlk` entry equal to an existing `dialog.tlk` entry in every field (flags, sound resref, volume and pitch variance, sound length, text) is not appended; its token takes the existing index: `Identical string for append StrRef 0 found in dialog.tlk StrRef 36850, reusing it instead.`, and the summary reads `dialog.tlk file updated with 2 new entries, 1 entries already existed.` With two equal entries in `dialog.tlk` (83 and 84), the first, 83, is reused.
- Equal text is not enough. The same text with flags 1 or 3 against the entry's 7, or with a sound length one second longer, is appended as a new entry; "Yes." without a sound was appended although many entries read "Yes." with one.
- Two tokens naming the same `append.tlk` entry (`StrRef2=2`, `StrRef3=2`) get the same new index; the entry is appended once.
- `Replace0=rep.tlk` with `[rep.tlk]` holding `100=0` and `StrRef101=1` is ignored without a log line; `dialog.tlk` entries 100 and 101 are unchanged. Replacing talk-table entries is not a feature of 1.2.10b1, whichever spelling the key takes.

## `[2DAList]`

The table `t.2da` had columns `label`, `Name`, `num` and rows labelled `0`, `1`, `2`.

- `AddRow` without `RowLabel`: the new row's label is its index (`3`).
- `RowLabel=high()`: one more than the highest numeric row label (`Setting row label to next HIGHEST value 4.`); `num=high()` in an added row: one more than the column's highest number (`... column num to next HIGHEST value 6.`, the column holding 1, 5 and 3).
- `2DAMEMORY#=RowLabel` stores the row's label; `2DAMEMORY#=RowIndex` after an `AddRow` the new row's index.
- `AddRow` with `ExclusiveColumn=Name` and `Name=beta`, a value row 1 holds: `New Exclusive row matched line 1 in 2DA file t.2da, modifying existing line instead.`, the row's other cells take the new values, and `2DAMEMORY3=RowIndex` stores 1, the matched row.
- A column named in another case (`name=` for `Name`) is matched: column names are compared without regard to case.
- `RowLabel=2` finds the row labelled `2`.
- `LabelIndex=gamma` is not read: `[Error] No RowIndex/RowLabel identifier for row to modify found at top of modifier list! Unable to apply modifier "t_c1".`, and the patcher goes on to the next modifier.
- `ChangeRow` with `RowIndex=50` in a three-row table: the same error, the modifier skipped, the install goes on and ends `Done. Some changes may have been applied, but 2 errors were encountered!`.
- `CopyRow` with `RowIndex=0`, `NewRowLabel=cp`: the copy is appended labelled `cp`, the given cells changed, `2DAMEMORY#=RowIndex` storing its index.
- `AddColumn` with `DefaultValue=dv`, `I0=x0`, `L2=x2`: every row takes `dv`, row index 0 takes `x0`, the row labelled `2` takes `x2`; `2DAMEMORY4=I0` stores the cell's value, `x0`, not an index.
- A cell set to `****` is written as the empty string.
- Cells an `AddRow` does not name are empty.

## `[GFFList]`

- Modifiers run in the order the section lists them, edits and `AddField`s interleaved: a `2DAMEMORY2=9` after the `AddField` that stores slot 2 sets the added field.
- `FVal=1,5` sets the float to 1.5, as `FVal2=2.5` sets 2.5; `Value=1,5|2,25|3` for a Position and `1,5|0|0|1` for an Orientation are read the same way, the components in the order written. Under Wine's default locale a decimal comma and a decimal point both parse.
- A label in another case (`tag=` for `Tag`) is not found: `[Warning] Unable to find a field label matching "tag" in g.utc, skipping...`. Labels are compared with case. A field that does not exist is the same warning, and the patcher goes on.
- `AddField` of a struct with `Path=MyList` into a list of two: `Added Struct, index 2, at position "MyList"`, with the given `TypeId` as the struct's id; `2DAMEMORY1=ListIndex` stores 2, which `Idx=2DAMEMORY1` in a later GFF file reads. `2DAMEMORY2=!FieldPath` on a field added inside it stores `MyList\3\C`.
- `LocName(strref)=5` sets the strref. `LocName(lang0)=changed0` on a string holding substring 0 replaces its text. `LocName2(lang2)=added2` on a string without substring 2 logs `Modified value "added2" to field "LocName2(lang2)"` and writes nothing: the field afterwards holds only its old substring. A substring the string lacks is not added, although the log says it was.

## `[HACKList]`

`OFFSET=VALUE` writes `VALUE` as a 32-bit unsigned big-endian integer at byte `OFFSET`, both decimal: `16=300` wrote `00 00 01 2C` at bytes 16 to 19, `0=5` wrote `00 00 00 05`, `24=65536` wrote `00 01 00 00`, `40=70000` wrote `00 01 11 70`. A hexadecimal offset (`0x20=1`) or a negative value (`32=-1`) is refused with `[Warning] Invalid offset(0x20) or value(1) modifier for file zz_hackb.ncs. Skipping...`. The file comes from `tslpatchdata` when Override lacks it (`Copying file zz_hacka.ncs to Override folder...`).

## `[SSFList]`

Each name writes one slot of the `SSF V1.1` file, in the order of `docs/formats/ssf.md`'s table: given 28 distinct strrefs 1000 to 1027 in that order, slot `i` held `1000 + i` afterwards and slots 28 to 39 were unchanged (`Finished updating 28 entries in file "zz_map.ssf".`). The file kept its 172 bytes.

## What TSLPatcher writes that the game would read differently from the author's intent

- A `(lang#)` edit on a missing substring is logged as done and not written.
- A file the destination lacks and `tslpatchdata` does not ship stops the install at a dialog, though the game holds the file in its BIFs or the module's RIMs, and a `.mod` that does not exist yet is skipped rather than made.
- `[TLKList]` replacement entries, `LabelIndex` and (in 1.2.10b1) `!Filename` are not read at all; mods that use them were written for HoloPatcher or a later build.

## What was not settled

How the patcher parses a decimal comma under a locale whose separator is the comma (the container's Wine runs the default locale). Whether `2DAMEMORY#` stored under a GFF section's own keys can name a path in other builds. `!OverrideType` (no mod in the build uses it). `[SSFList]` and `[HACKList]` into a capsule.
