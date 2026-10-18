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
rakata dialog show NAME.dlg --game DIR --brief
```

The transcript lists the starts in the order the engine tries them, each with its condition's source, then each branch indented. `E3` is `EntryList[3]` (the owner's or another speaker's line), `R0` is `ReplyList[0]` (the player's). Each node shows its strref, its voice resref (`voice nm14acjuha07024_`), its sound, its action script, and every condition decompiled to source and marked `[verified]` when the source recompiles to the original bytes. Include libraries are folded out, so a condition calls `k_inc_utility`'s functions and passes its constants by name (`SW_PLOT_BOOLEAN_01`, not 0); only a function that does not verify carries a comment, `// E0602: unverified: …`. `(a link)` marks a link that reuses a node shown elsewhere; `-> E3 (shown above)` points back to it. `--scripts` decompiles the action scripts too. `--brief` prints only each node's speaker and text and the names of its conditions and script, decompiling nothing: the quick way to read the shape of a long conversation.

The engine opens a conversation with the first start whose condition passes, trying them in the order listed, so a start without a condition is the fallback and every start after it is unreachable. An entry that appears among the starts is itself a starting line: a reply added under it is offered whenever the conversation opens there. A voice line appears only for a line the game would voice or whose file exists: player replies and entries whose `SoundExists` is 0 name placeholders and print none. A voiced line whose file is missing says "no file under streamwaves/", and `dialog lint` reports it.

Each node's line carries its stable key in brackets, the key an edit names it by: `E85 [voice:nglobebant00000_] Carth to Bastila: "…"`. It is `voice:RESREF` when the voice resref names that node alone, else `strref:N`, else `text:"…"` for text the node embeds, else `index:E3 (fragile)`: an index moves when another mod inserts a node, so prefer a node's content key. The JSON gives each node's `key` and `fragile`.

To read one node rather than the whole tree, name it with `--node KEY` (repeatable; a stable key or an index such as `E3`):

```bash
rakata dialog show NAME.dlg --module STEM --game DIR --node voice:nglobebant00000_ --node R0
```

It prints the node and each of its links: the link's condition, decompiled, then `-> R95 [KEY]`, the node it leads to with that node's key. A key that matches no node fails with `RE1102`, one that matches several with `RE1103` (common for `text:` keys, and for a `strref:` two nodes share).

A module's dialogue copied into `Override/` is shown with the one module whose capsules hold the same name, so its scripts resolve: the first line reads `Override/NAME.dlg (module STEM)`. When several modules hold it, `RE1105` names them; pass `--module STEM`.

`rakata --format json dialog show NAME.dlg --game DIR` gives the same as structured nodes.
