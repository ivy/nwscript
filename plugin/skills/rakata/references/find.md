# Find the loaded copy and read it

Every read names the install with `--game DIR`. Names compare without regard to case.

## Which copy the game loads

```bash
rakata which globalcat.2da --game DIR
rakata which NAME.dlg --module STEM --game DIR
```

The first line is the winner, `wins LOCATION … sha256 HASH`; the rest are every other copy and why it lost. A module's own resource is invisible without `--module`: `RE0404 … a module's copy needs its module named` means name the module. To find the module, `rakata dialog show NAME.dlg --game DIR` prints the copy it loaded on its first line, `modules/STEM_s.rim:NAME.dlg (module STEM)`, or ask the index (`refs.md`).

`rakata which --duplicates --game DIR` lists Override files whose names differ only in case, which the engine treats as one.

## The bytes, and the formats

- `rakata cat NAME.EXT --game DIR --module STEM -o FILE` writes the winning copy's bytes; `ARCHIVE!MEMBER` reads a member directly (`modules/STEM_s.rim!NAME.dlg`).
- `rakata gff show RESOURCE --game DIR --module STEM --path FIELD` prints a GFF (blueprints `.utc`, `.uti`, `.utp`, areas, `.dlg` too) or one field by path (`Tag`, `ItemList[0].InventoryRes`).
- `rakata 2da show globalcat.2da --game DIR --row LABEL --column NAME` prints a table, a row, a column or a cell.
- `rakata tlk show STRREF --game DIR` prints a string of `dialog.tlk`; `--text PATTERN` finds strings containing a text.

Each takes a file path as well (`./edited.dlg`), so the same commands read what a mod wrote.

## A dialogue

```bash
rakata dialog show NAME.dlg --game DIR
```

The transcript lists the starts in the order the engine tries them, each with its condition, then each branch indented. `E3` is `EntryList[3]` (the owner's or another speaker's line), `R0` is `ReplyList[0]` (the player's). Each node shows its strref, its voice resref (`voice nm14acjuha07024_`), its sound, its action script, and every condition decompiled to source and marked `[verified]` when the source recompiles to the original bytes. `(a link)` marks a link that reuses a node shown elsewhere; `-> E3 (shown above)` points back to it. `--scripts` decompiles the action scripts too.

The engine opens a conversation with the first start whose condition passes, trying them in the order listed, so a start without a condition is the fallback and every start after it is unreachable. An entry that appears among the starts is itself a starting line: a reply added under it is offered whenever the conversation opens there. A voice resref shown with "no file under streamwaves/" is a note, not a finding: many vanilla lines name a voice-over that was never recorded, the game shows them unvoiced, and `dialog lint` reports a missing voice-over only for nodes the game would try to voice.

To pick a node to edit, note its **voice resref** or **strref** from the transcript: those are its stable keys (`voice:nm14acjuha07024_`, `strref:38576`). A node with neither has only its index, which is fragile.

A module's dialogue copied into `Override/` is shown with the one module whose capsules hold the same name, so its scripts resolve: the first line reads `Override/NAME.dlg (module STEM)`. When several modules hold it, `RE1105` names them; pass `--module STEM`.

`rakata --format json dialog show NAME.dlg --game DIR` gives the same as structured nodes.
