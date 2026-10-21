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

## Textures, layouts, walkmeshes and models

```bash
rakata texture show lhr_flr01.tpc --game DIR
rakata texture export lhr_flr01.tpc --game DIR --out lhr_flr01.png
rakata lyt show m01aa.lyt --game DIR
rakata walkmesh show m01aa_01a.wok --game DIR
rakata mdl show m01aa_01a.mdl --game DIR
rakata mdl show v_muzflash_01.mdl --controllers --game DIR
```

Each reads one resource in the same three forms as `gff show`, and tells the format by its extension; a resource of another format fails with `RE1307`, whose help names the command that reads it.

- `texture show` reads a `.tpc` (size, pixel format `dxt1`/`dxt5`/`rgb`/`rgba`/`grey`, one image, a cube's six faces or an animation sheet's frames, each level's size and offset, then the TXI it carries), a `.txi` (one `KEY VALUE` line per instruction) or a `.tga` (size, image type, depth, origin; the lightmaps are TGAs, `ROOM_lm0.tga`). A texture name resolves through the texture packs too: `rakata which` shows the pack it came from.
- `texture export` writes a `.tpc` or `.tga` as an RGBA PNG (`--out PATH`, else `NAME.png` here) and its TXI beside it as `STEM.txi`: the text after a TPC's pixels, else the `NAME.txi` the install holds. Level 0 by default; `--level N` one level, `--all-levels` each as `STEM.N.png`. A cube's six faces are stacked top to bottom, an animation sheet's frames laid out as the header's size; `--image N` writes one alone. A level or image the texture lacks is `RE2210` or `RE2211`. DXT1 and DXT5 are decoded; RGB and RGBA bytes are written in the order stored, since their channel order is not established.
- `lyt show` reads a `.lyt` (each room model and its position, swoop tracks and obstacles, each door hook with its room, position and orientation) or a `.vis` (`ROOM  sees N: ROOM ...`, the rooms drawn from each).
- `walkmesh show` reads a `.wok`, `.pwk` or `.dwk`: counts, the bounds of its vertices, the faces of each surface material, and `transitions`, the perimeter edges leading into each room by its index in the area's layout. With `--game` the materials are labelled and walkable faces counted through `surfacemat.2da`; `RW1301` says the install has none that reads. A WOK's vertices are already in the area's frame, so its bounds sit where its room stands in the LYT; a DWK's or PWK's are the object's own.
- `mdl show` reads a model with its MDX (found as the MDL was: through the install, beside a path, in the same archive; `RW1302` when it is missing): the classification and supermodel, the node tree with each node's kind (`dummy`, `trimesh`, `skin`, `dangly`, `aabb` for a room's walkmesh, `light`, `emitter`, `reference`, `saber`), each mesh's texture, lightmap and vertex and face counts, and the animations. A light, an emitter and a saber blade each get a second line: a light's colour, radius, multiplier, priority, flags and flare textures; an emitter's update (how its particles are born: `Single`, `Fountain`, `Explosion`, `Lightning`), render, blend, texture or chunk model, frame grid, birth rate, life and frames; a blade's vertex count, extent along Z and its two words. `--controllers` prints every controller's keys by name on the node's kind (`birth_rate` on an emitter, `radius` on a light, `type N` where the meaning is not established), under each node and under each animation. With `--format json` each node carries `controllers` (type, name and keys), `light`, `emitter` and `saber` in full; `--controllers` adds each animation's `keyed_nodes`. `docs/formats/mdl.md` says what each field is. The supermodel chain is resolved by name through the install, each model listed with where it lay, then every animation the model inherits and from which; `RW1303` names a supermodel that is not found.

`--format json` gives each node's number, parent, transform and controller count, and each mesh's flags, stride and attributes.

## A blueprint or an area, read

```bash
rakata show NAME.uti --module STEM --game DIR
rakata show AREA.git --module STEM --game DIR --db FILE
```

`rakata show` reads a GFF with what it names resolved, so the 2DAs and the talk table need not be queried by hand. An item (`.uti`) shows its name, tag, base item and each property as one line, `Damage Bonus (Fire): 2 [itempropdef 11, iprp_damagetype 8, iprp_damagecost 2]`, the rows it came from in brackets. A creature, placeable or door (`.utc`, `.utp`, `.utd`) shows its name, tag, conversation, event scripts and inventory. An area's instances (`.git`) show each instance with its template's name, tag and scripts; with `--db`, an index (`refs.md`), it also lists what the module's scripts spawn there with `CreateObject` and which script does it, `kor37_freedon (creature) … by k37_opensarc`. A module's resources need `--module`. Any other GFF prints as `gff show` does.

## One script, decompiled

```bash
rakata decompile NAME.ncs --module STEM --game DIR > NAME.nss
```

The copy the engine loads, decompiled as `dialog show` decompiles conditions: the install's `nwscript.nss`, include libraries folded back to `#include`, every function verified by recompiling. The first lines are `//` comments giving where the copy lies and `[verified, N of N functions]`; a function that did not verify is reported (RW2001) and should be read against `nws disasm`. Without `--module`, the one module holding the script is used.

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
