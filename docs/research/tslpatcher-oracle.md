# TSLPatcher under Wine, without its GUI

2026-10-17. The installer's acceptance ([`../proposals/rakata.md`](../proposals/rakata.md), "The installer") measures `rakata install` against TSLPatcher run as a black box on the same inputs. This records what the mod archives of modshop's build ship as `TSLPatcher.exe`, how it is driven with no window anyone looks at, what one install costs, what its log holds, and the first real comparison: repair 09's mod, installed by its own TSLPatcher onto the parent checkpoint's tree, against the build's checkpoint for that entry.

## Method

The archives are the 192 under modshop's `games/kotor/data/archives/<entry>/` (one folder per entry of the KotOR 1 Full Build, plus `mega/` and `k1cp/`), opened with 7-Zip (`7zz l -slt`, `7zz e`). Every `.exe` inside them was extracted, hashed with `sha256sum`, and its PE version resource read with `strings -el` (the UTF-16 strings `FileVersion`, `ProductVersion`, `ProductName`, `FileDescription`). What each binary calls itself was read from its own window: the caption of the main window and the version in its footer, under Wine on a virtual display, screenshot with ImageMagick's `import`. No binary was decompiled or disassembled.

The container is modshop's Wine image (`games/kotor/kmrp-build/wine/Containerfile`, image `modshop-wine`, Fedora 44, Wine, Xvfb, no network) plus xdotool, xwininfo, xprop, ImageMagick and openbox, from [`../../conformance/drivers/wine/Containerfile`](../../conformance/drivers/wine/Containerfile): `podman build -t rakata-oracle-wine conformance/drivers/wine` took 6.4 s on top of the already-built base. Runs use `podman run --rm --network none --security-opt label=disable`, the game tree mounted at `/game` (Wine's `Z:\game`), a copy of the mod folder at `/mod`, as modshop's KMRP step does.

Game trees are reflink copies of checkpoints (`cp -a --reflink=auto`, 0.61 s for the 21,180-file tree) under modshop's ignored `data/` area; the vanilla tree and the checkpoints were never written to. The parent of entry `170-blaster-adjustment` in the current chain is checkpoint `255d96be9c32590e` (entry 169); the build's checkpoint for 170 is `a0d0f8c2e9adc871`, whose `changed.txt` lists one path, `Override/baseitems.2da`.

## The installers the archives ship

100 `.exe` paths in 192 archives; 9 are macOS `._` resource forks and are dropped. Eight distinct binaries remain:

| SHA-256 | Size | Version resource | What it says it is | Entries | Copies |
| --- | --- | --- | --- | --- | --- |
| `1a267454b2efcd61232332be83f4da4ffe88bd6770c5a0df517880062b1a75e8` | 686,080 | `FileVersion 1.0.0.0`, no product name | footer "TSLPatcher 1.2.9b" | 41 | 42 (+ `mega/Install.exe`) |
| `013655cd871a71e0f5b50f80ed0d32308912916ba7e12e2a366f42d992a71b16` | 681,984 | `TSLPatcher`, "Mod Installer for KotOR1 & KotOR2.", `FileVersion 1.2.10.1`, `ProductVersion 1.2.10b1` | footer "TSLPatcher 1.2.9b" | 17 | 18 (+ `mega/TSLPatcher.exe`) |
| `638aaeea4e40ffb6c592f68787f2140318a0dec7f2cde85908806ce3b5bc5131` | 685,056 | `TSLPatcher`, same strings, `1.2.10.1` / `1.2.10b1` | footer "TSLPatcher 1.2.9b" | 6 | 6 |
| `2a108d0ba866867da4726dedf6b17efdb23db4cbf3809856adbadf4a366cecb6` | 9,518,185 | none | window "HoloPatcher" | 10 | 17 (entries 061 and 152 ship it under several names) |
| `5cc9fc7aeafefb8b18cd584de0ad530f5c7a057e4b0a1308ed7bc6a9e4bc3fbe` | 7,653,859 | none | window "HoloPatcher" (K1CP's `INSTALL.exe`) | 1 | 2 (+ `k1cp/`) |
| `7a1cc0fb30c5f3653fac36b9d8283d04c5a3db308e55c247cc1593c6ef45690f` | 9,523,783 | none | window "HoloPatcher v1.5.2" | 1 | 1 |
| `a7007ec081344f416972493879c1f57239897a4fde094c4d79c3585eb9146247` | 11,319,611 | none | window "HoloPatcher v1.6.0b4" | 2 | 2 |
| `539eb689d2e0d3751aeed273385865278bef6696c46bc0cab116b40c3b2fe820` | 212,992 | none | `nwnnsscomp.exe`, shipped beside TSLPatcher for a `CompileList` | 3 | 3 |

So three TSLPatcher builds cover 64 numbered entries, and 41 of them, the majority, carry the build whose version resource was blanked (`1.0.0.0`) and which goes by `INSTALL.exe`, `Install.exe` or the mod's own name as often as by `TSLPatcher.exe`. All three builds print "TSLPatcher 1.2.9b" in the footer of their main window, so the footer does not distinguish them; the resource and the hash do. The filename is no guide either: `TSLPatcher.exe` names copies of the first build in six entries (141, 161, 167, 176, 177, 184). Fourteen entries ship HoloPatcher rather than TSLPatcher; for those the oracle is the version of HoloPatcher the author tested with, not TSLPatcher, and the parity question is a different one.

Across the 64 TSLPatcher-installed entries, the `.ini` files declare: 129 ini files in all; 17 with both a `2DAList` and a `GFFList`; one (141, Bastila Has Battle Meditation v1.2) with every list but `SSFList` non-empty, including a `TLKList` and a `CompileList` of 7 scripts.

## Headless: no command line, synthetic input works

TSLPatcher takes nothing from its command line. Started with no argument, with `Z:\game`, or with `/?`, it writes nothing to stdout or stderr and opens the same first window; the argument changes nothing visible. It also has no silent mode in its `Settings`: `LookupGameFolder=1` (mod 141) reads the game folder from the registry, and in a prefix without that key it falls back to the same Browse for Folder dialog. Driving the windows is the only way, and it works.

What the windows are, in order, for a mod with `namespaces.ini`:

1. "TSLPatcher - Select installation type": a combo box with focus, listing the namespaces in file order, a Select button (mnemonic S) and Exit. The main window behind it updates live as the combo selection changes.
2. The main window, captioned with the ini's `WindowCaption` (or "TSLPatcher (TLK / 2DA / GFF Patcher)" without one): the mod's info text, Exit bottom left, "Install Mod" bottom right, "TSLPatcher 1.2.9b" in the footer, "Game folder: User selected." in the status bar.
3. "Browse for Folder" (the shell's folder tree, no edit box: Desktop, `/`, Documents, My Computer, Trash), opened by Install Mod. The main window starts its progress log ("Installation started ...") before the folder is chosen.
4. The run, logged into the main window, then a finish box: "Information" ("The Installer is finished. Please check the progress log...") when the log counts no warnings, "Warning" ("The Installer is finished, but N warnings were encountered! The Mod may or may not be properly installed...") when it does.
5. Exit closes the window and writes `installlog.rtf` beside the `.exe`. Before Exit the file does not exist: the mod folder listed while the finish box was up held `backup/` and no log.

What had to be learned to drive it:

- A window manager is required. Under bare Xvfb the Select button's press left the dialog mapped and the main window never took the Install Mod click; under openbox every step works. The image adds openbox for that reason.
- Keys sent with `xdotool key --window ID` (XSendEvent) reach the combo box but not the buttons; keys after `xdotool windowactivate` (XTEST) reach both. The combo takes Down N times for option N; Select takes `alt+s`.
- Install Mod ignores its mnemonic (`alt+i` did nothing) and is clicked at its position, 62 px from the right and 34 px from the bottom of the main window; Exit at 59 px from the left and the same height. The window's size is fixed by the binary, so the offsets hold across mods.
- The folder tree is navigated by keyboard: click `/` (67, 90 within the dialog), Right to expand it, type `game` (the tree's type-ahead jumps to the entry), Return. The tree lies at `/game` because the driver mounts it there; the Windows path TSLPatcher logs is `Z:\game\`.
- Case: TSLPatcher logs `Z:\game\override\baseitems.2da` while the tree has `Override/` (and modshop's `override` symlink to it); Wine resolves the case for it.

The driver is [`../../conformance/drivers/tslpatcher.sh`](../../conformance/drivers/tslpatcher.sh): `tslpatcher.sh TSLPATCHDATA GAME OPTION [OUT]`. It copies the mod folder (TSLPatcher writes `backup/` and the log beside its `.exe`), finds the one `.exe` beside `tslpatchdata/` (or `$TSLPATCHER_EXE`), runs the steps above inside the container, and leaves in OUT the log, the backup folder, a screenshot of each step and `driver.log` with the main window's caption, the finish box's title and any other box it dismissed. The exit status is 0 when the finish box appeared.

It drove every build without a person: entry 170 (build `1a2674…`, with namespaces) twice plus its full option, 168 (same build, namespaces by `DataPath`, GFF edits inside a `.mod`), 180 (build `013655…`, with namespaces), 130 (build `638aae…`, no namespaces) and 141 (build `013655…`, `LookupGameFolder=1`, every list, a `CompileList` through the shipped `nwnnsscomp.exe`).

## Time

One install of entry 170 (option 1, one 2DA edit) through the driver: 15.9 s and 15.9 s wall for the two runs; 15.7 s for 168, 15.7 s for 180, 13.8 s for 130, 14.8 s for 141 (`/usr/bin/time` around `podman run`, as the driver prints). TSLPatcher's own work is a small part of that: from Return in the folder dialog to the finish box took 0 s to 2 s on every mod measured (the driver's half-second poll). The rest is fixed: Xvfb and openbox start, Wine's start, the driver's sleeps between steps, and `wineserver --wait` at the end. A whole-build parity run over 64 entries is therefore about 16 minutes of wall time at this pace, most of it removable by tightening the waits.

## The log

`installlog.rtf`, RTF written by Wine's riched20 ("Wine Riched20 2.0" as generator), Courier New, green for the start and end lines, grey for the rest, one `\par` per line. Rendered as text, entry 170's run reads, in full:

```text
* Installation started 10/18/2026 12:54:50 AM...
* Modifying file "baseitems.2da" found in Override folder...
* Saving unaltered backup copy of baseitems.2da in Z:\mod\backup\baseitems.2da
* Updated 2DA file Z:\game\override\baseitems.2da.
* Done. All changes have been applied.
```

The timestamp is the container's clock in US format; it is the only line that differs between two runs. The warning lines take the form `Warning: A file named X already exists in the override folder. Skipping file...` (`InstallList` with `!ReplaceFile` unset, for a file an earlier entry already put there) and are what turns the finish box into "Warning"; the end line then reads `Done. Changes have been applied, but N warnings were encountered.`. A GFF edit logs `Modifying GFF format files...`, `Modifying file "X" found in Override folder...` or `File "X" already exists in archive "Y.mod", modifying existing file...`, then `Finished updating GFF file "X"...` and, for an archive, `Saving modified file "X" in archive "Y.mod".`; at `LogLevel=3` the GFF lines come with `GFF Check: N_...` and `GFF FILE: ...` diagnostic lines (40 of the 72 lines of entry 168's log). Scripts log `Modifying and compiling scripts...`, `Compiling modified script X.nss...`, `Adding script "X.ncs" to archive "Y.mod"...`. TLK: `Appending strings to TLK file "Z:\game\dialog.tlk"`, `dialog.tlk file updated with 1 new entries.`

Entry 141's log, the one with every list, fixes the pipeline order TSLPatcher runs: `TLKList`, `InstallList`, `2DAList`, `GFFList`, `CompileList` (its `SSFList` is empty). Entry 170's full option (one 2DA, two GFF) shows the same 2DA-before-GFF order. That is the order stoffe's readme gives.

Repair 09's premise, from the same log: `pistol_rifle.ini` names `ChangeRow12=repeating_blaster` and `ChangeRow13=heavy_repeating_blaster`, sections the file does not have. TSLPatcher 1.2.9b writes nothing about them: no warning, the end line is "All changes have been applied", the finish box is "Information", and the twelve rows the sections exist for are applied. It skips an orphan row silently.

## The comparison with the build

Two driver runs of entry 170, option 1, each onto a fresh reflink copy of checkpoint `255d96be9c32590e`'s tree, then `diff -rq` between the two results: no difference (35 s for the 21,180 files). The driver is deterministic on this input.

`diff -rq` of one result against the build's checkpoint `a0d0f8c2e9adc871/tree`: one path differs, reported twice because `override` is a symlink to `Override`:

| Path | Classification |
| --- | --- |
| `Override/baseitems.2da` | expected: the same table written by two writers; see below |

Cell by cell (a reader of the documented `2DA V2.b` layout: column names, row count, row labels, a `uint16` offset per cell, a `uint16` string-block size, the strings), TSLPatcher's file and the build's hold 92 rows by 61 columns and 0 cells differ. Against the parent's file both changed the same seven cells: rows 12 and 13 `dietoroll` 6→8 and 8→10, row 15 `dietoroll` 4→6 and `crithitmult` 2→3, row 16 `dietoroll` 4→6 and `critthreat` 1→3, row 20 `critthreat` 2→1, the pistol and rifle changes of the option, and nothing from the two orphan rows: TSLPatcher's skipping and the repair's removal of the two lines come to the same table. The build's checkpoint is right.

The bytes differ because TSLPatcher 1.2.9b rewrites the string block differently: its file is 16,166 bytes against 16,076 for both the parent and the build's, with 467 distinct cell offsets against the parent's 433, the new values appended as new strings rather than shared with equal strings already present, and the string-block size field left at the parent's 3,894 while the block it wrote is 3,984 bytes long (the 90 bytes past the declared size are the last rows' strings, `Revan_Armor` … `Squad_Recovery_kit`, pushed there by the insertion). A reader that trusts the size field cuts the last rows' strings off; one that reads to the end of the file, as a cell-by-cell comparison must, sees the same table. Every Windows player of this mod has installed that file, so the engine reads past the field. This is the shape an "explained difference" takes: the parity comparison for 2DA has to be semantic, and `rakata-2da` must read the string block to end of file and should write the size field correctly.

## What was not done

- The driver's waits are fixed sleeps, tuned on this machine; a slower one may need `TSLPATCH_TIMEOUT` and longer pauses. The click offsets are measured on these three builds and openbox's decorations; a fourth build with a different window would need its own.
- A mod with a `ConfirmMessage` was not run; the driver answers any box before the folder dialog with Enter, which is untested.
- Only the first difference class (2DA writer bytes) is classified. GFF, TLK and `.mod` writes (entries 141 and 168 produced them) await a semantic diff before their byte differences can be sorted into expected and not.
- The HoloPatcher-installed entries have no TSLPatcher to serve as oracle; the version shipped is the author's reference.
