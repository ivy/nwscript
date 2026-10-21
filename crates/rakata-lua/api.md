# The rakata Lua API

`rakata lua SCRIPT [ARGS...] --game DIR` runs a Lua 5.4 script with the global `rakata` loaded: the install, the formats, dialogues by stable key, and the authoring operations of the `rakata` façade, under the façade's own names. A script reads as the Rust it stands for: `rakata::author::twoda::add_row_copying(&mut t, ...)` is `rakata.author.twoda.add_row_copying(t, ...)`, and `Mod::write_override` is `m:write_override(...)`. `rakata lua --reference` prints this page.

```lua
local m = rakata.Mod.open(rakata.Install.open(), "danm14ac")
local cat = rakata.twoda.parse(m:loaded("globalcat.2da").bytes)
rakata.author.globals.declare(cat, "OAJ_DONE", "Boolean")
m:write_override("globalcat.2da", cat:to_bytes())
```

## Conventions

- Bytes are Lua strings, which hold any bytes: a file read, a resource loaded, a table's `to_bytes()`.
- Indices are the file's, from 0: a 2DA row, a dialogue node (`E0` is the first entry), a link's position. A sequence the binding returns (columns, findings, candidates) is a Lua sequence from 1.
- Options are a table as the last argument, every key optional; a key the function does not take is refused (`RE1805`).
- Errors are raised, never returned. See [Errors](#errors).
- A resource is named `NAME.ext` (`appearance.2da`, `dan14_juhani.utc`), case not mattering.

## Errors

Every error the façade raises carries its registered code. `pcall` and `xpcall` hand the handler a `rakata.Error`, a table `{code = "RE1404", message = "..."}` whose `tostring` is `RE1404: message`; `error(e)` on one raises it again with its code. An error the script does not catch ends the run with exit 1 and a diagnostic under its code (`rakata explain CODE`). An error of the script's own (`error("...")`, a call on `nil`, a syntax error) has the code `RE1804`.

```lua
local ok, e = pcall(rakata.author.globals.declare, cat, "OAJ_DONE", "Boolean")
if not ok and e.code == "RE1404" then print("already declared: " .. e.message) end
```

- `rakata.Error.is(v)`: whether `v` is a `rakata.Error`.
- `rakata.explain(code)`: `{code, summary, explanation}` for a registered code, case not mattering, or nil.

The binding's own codes are `RE1801` (a path that is not one of the script's inputs), `RE1802` (a module not in the script's directory), `RE1803` (the install opened without `--game`), `RE1804` (a script's own error), `RE1805` (a value of the wrong kind), and `RE1806` (a file that could not be written). Reading a file that does not exist is `RE0001`.

## Hermeticity

A script's inputs are its arguments and `--game`. The state has the `string`, `table`, `math`, `utf8` and `coroutine` libraries and the base functions, without `dofile` and `loadfile`; there is no `io`, `os`, `package` or `debug`. Files are reached only through `rakata.fs`, `require`, and the install:

- Reads (`rakata.fs.read`, `exists`, `list`, a session's `load` of a path) are allowed below `--game`, the script's directory, and each path an argument names.
- Writes (`rakata.fs.write`, and `Mod`'s writes into the install) are allowed below `--game` and each path an argument names.
- A path is compared after `.`, `..` and symbolic links are resolved; anything else is `RE1801`.

## Globals

- `require(name)`: the module `name` from the script's directory, dots as directory separators (`require "lib.scenes"` loads `lib/scenes.lua`), loaded once and its value kept; `require "rakata"` is the `rakata` table. A missing module or a name with `..`, a slash or an empty part is `RE1802`.
- `print(...)`: the values, tab-separated, a line to standard output; with `--format json`, kept for the envelope's `result.output`.
- `pcall`, `xpcall`, `error`: Lua's, with `rakata.Error` kept as described under [Errors](#errors).

## rakata

- `rakata.version`: the build identity, as `rakata --version` prints it.
- `rakata.args`: the arguments after the script's name, a sequence of strings.
- `rakata.game`: the install `--game` names, as given, or nil.
- `rakata.script`: the script's path, as given.

## rakata.fs

- `rakata.fs.read(path)`: the file's bytes.
- `rakata.fs.write(path, bytes)`: write the file whole, creating its directory.
- `rakata.fs.exists(path)`: whether the path exists.
- `rakata.fs.list(path)`: the names in a directory, sorted.
- `rakata.fs.join(...)`: the parts joined as a path.

## rakata.res

- `rakata.res.hash(bytes)`: the SHA-256 of the bytes, in hex, as the install and `guard_replaced` compare them.
- `rakata.res.id(name)`: the resource name as the façade reads it, or `RE0101`/`RE0102`/`RE0103` for one it does not.

## rakata.Install

The install `--game` names, read in the engine's order: `Override/`, the module's capsules, `patch.erf`, the texture packs (the selected quality pack, then `swpc_tex_gui.erf`), the BIFs.

- `rakata.Install.open([{profile = "k1", texture_pack = "tpa"}])`: open it, the profile `k1` or `k2` (default: `rakata lua --profile`, itself `k1` by default), and the quality pack the texture layer searches, `tpa`, `tpb` or `tpc` (default `tpa`: which pack the game's `Texture Quality` selects is not established); another pack name is `RE0405`. Without `--game`, `RE1803`.
- `install:root()`: the install's directory.
- `install:profile()`: `k1` or `k2`.
- `install:texture_pack()`: `tpa`, `tpb` or `tpc`.
- `install:modules()`: every module's name (`danm14ac`), a sequence.
- `install:resolve(name, [module])`: which copy the engine loads, `{id, module, ambiguous, candidates}`, each candidate `{layer, location, len, hash, verdict}` with `layer` one of `override`, `module`, `patch`, `texturepack`, `chitin`, and `verdict` `wins`, `ambiguous` or `shadowed` (then also `by`, the winner's index from 0, and `reason`). The first candidate wins.
- `install:read(name, [module])`: `{bytes, location}` of the copy the engine loads, or nil when no layer holds it. `location` is relative to the install: `Override/x.2da`, `modules/danm14ac_s.rim:k_pdan_juhani31.ncs`, `TexturePacks/swpc_tex_gui.erf:fnt_d16x16.tpc`, `data/2da.bif[12]`.

## rakata.Mod

An install opened for a mod's writes, in one module. Every write is read back as its type first, and a dialogue is checked against the install and the mod's earlier writes ([`rakata::author`](../rakata/src/author/mod.rs)).

- `rakata.Mod.open(install, [module])`: open `install` for a mod working in `module`, or in the global layers alone. A module the install lacks is `RE0403`.
- `m:install()`: the install.
- `m:module()`: the module's name, or nil.
- `m:set_module([module])`: work in `module` from now on, or in the global layers alone with nil, keeping every write made so far. A module the install lacks is `RE0403`, the module unchanged.
- `m:guard_other_modules(name)`: refuse to go on when another module holds its own copy of `name`, different from the one this module loads, which a write to `Override/` would replace there too (`RE1415`).
- `m:loaded(name)`: `{bytes, location, written}`, the copy the game loads, the mod's own write first; `written` says whether the mod wrote it. No copy is `RE0404`.
- `m:guard_replaced(name, sha256)`: refuse to go on unless every copy in the module's capsules has this SHA-256 (`RE1401`), `Override/` holds none (`RE1402`), and the module holds one (`RE1403`).
- `m:write_override(name, bytes)`: write to `Override/` as `name`; returns `{path, hash, len, warnings}`, `warnings` the findings a dialogue's loaded copy already had. A dialogue whose edit names what the install lacks is refused with the file unchanged (`RE1407`).
- `m:write_module(name, bytes)`: write into the module's `STEM.mod`, for an edit this module alone loads, another module's own copy left as it is; a `.mod` the module lacks is made from its `.rim` and `_s.rim` first. Checked as `write_override` is, and returns as it does, `path` the `.mod`. A copy in `Override/` that would be loaded over it is `RE1418`, a mod in no module `RE1419`.
- `m:write_streamwaves(resref, bytes)`: write a voice-over to `streamwaves/RESREF.wav`; bytes that are not RIFF WAVE are `RE1413`. Returns as `write_override`.
- `m:find(dialog, key)`: the `NodeRef` a key names, text keys compared through the install's talk table.
- `m:check_dialog(dialog)`: the reference checks `write_override` runs, a sequence of findings `{code, place, key, subject, via, message}`, `key` nil for an end script (`place` `EndConversation` or `EndConverAbort`).

## rakata.twoda

A 2DA table. A row is named by its index from 0 (an integer) or its row label (a string, the first row carrying it). A cell is a string; a number given for one is written as Lua prints it.

- `rakata.twoda.parse(bytes)`: read a table.
- `rakata.twoda.new(columns)`: an empty table with these column names.
- `t:to_bytes()`: the table in BioWare's layout.
- `t:columns()`: the column names, a sequence.
- `t:len()`, `#t`: the number of rows.
- `t:label(row)`: the row's label.
- `t:row(row)`: `{index, label, cells}`, `cells` by column name.
- `t:get(row, column)`: the cell.
- `t:set(row, column, value)`: set the cell.
- `t:row_index(row)`: the row's index, or nil.
- `t:column_index(name)`: the column's index from 0, or nil.
- `t:add_row(label, [cells])`: append a row of blank cells with `cells` (`{column = value}`) set; returns its index.
- `t:add_row_copying(from, label, [cells])`: append a copy of the row `from` labelled `label` with `cells` changed; returns its index. This is the table's own operation: the label is the row label. `rakata.author.twoda.add_row_copying` is the authoring one.
- `t:add_column(name, [fill])`: append a column, every row taking `fill` (default empty); returns its index.
- `t:semantic_eq(other)`: whether the two hold the same columns, labels and cells.
- `t:diff(other)`: how `other` differs, a sequence of lines.
- `t:clone()`: a copy.

A missing row is `RE0705`, a missing column `RE0706`, a column added twice `RE0707`.

## rakata.gff

A GFF file: a creature, item, placeable, dialogue or any other `V3.2` file. A field is named by path (`Str`, `ItemList[0].Dropable`, `EntryList[3].RepliesList[0].Index`).

A value read is plain Lua: an integer, a number, a string (`CExoString`, `CResRef` and `VOID` as the file's bytes), a locstring `{strref = N, strings = {[id] = text}}` (`strref` nil when it has none; `id` is the language times two, plus one for the feminine form), a struct as `{Label = value}`, a list as a sequence of those, a vector or orientation as a sequence of numbers. A value set is either plain, taking the type the field holds now (a number that does not fit is `RE1805`), or typed, made by one of the constructors below, which keeps its own type; a field that does not exist yet needs a typed value.

- `rakata.gff.parse(bytes)`: read a file.
- `rakata.gff.new(file_type)`: an empty file of this type (`UTC`, `DLG`).
- `rakata.gff.byte(n)`, `char(n)`, `word(n)`, `short(n)`, `dword(n)`, `int(n)`, `dword64(n)`, `int64(n)`, `float(x)`, `double(x)`, `string(s)`, `resref(s)`, `strref(n)`, `void(bytes)`, `vector({x, y, z})`, `orientation({a, b, c, d})`, `locstring(v)`: a typed value; `locstring` takes a strref, an English string, or the locstring table. A resref longer than sixteen bytes is `RE0608`.
- `value.type`, `value.value`: a typed value's type name (`BYTE`, `CExoString`, `CResRef`, `CExoLocString`, `List`, ...) and its plain value; typed values compare with `==`.
- `g:to_bytes()`: the file in BioWare's layout.
- `g:file_type()`: the four-character type, trailing blanks trimmed.
- `g:get(path)`: the plain value.
- `g:get_value(path)`: the typed value, structs and lists included, to set elsewhere.
- `g:type(path)`: the field's type name.
- `g:has(path)`: whether the path names a field or a struct.
- `g:len(path)`: the number of elements in a list.
- `g:fields([path])`: the labels of the struct at `path` (the root by default), in file order.
- `g:set(path, value)`: replace an existing field's value, of any type; returns the old plain value.
- `g:clone()`: a copy.
- `g:semantic_eq(other)`: whether the two hold the same tree.

A path that does not parse is `RE0606`; one naming nothing is `RE0607`.

## rakata.lip

A lip-sync track. A shape is a label (`rest ee eh ah oh oo y s f n th m t sh l k`) or its byte, 0 to 15.

- `rakata.lip.parse(bytes)`: read a file.
- `rakata.lip.from_cues(cues, length)`: a track from cues `{{time, shape}, ...}` (each also `{time = t, shape = s}`), in time order, with a rest at `length`.
- `lip.length`: the line's length in seconds.
- `lip:keys()`: the keyframes, `{time, shape}` with the shape's label.
- `lip:to_bytes()`: the file.

## rakata.tlk

A talk table.

- `rakata.tlk.parse(bytes)`: read a table.
- `tlk:len()`, `#tlk`: the number of entries.
- `tlk:text(strref)`: the text the engine shows for the strref, or nil.
- `tlk:append(text)`: add an entry and return its strref.
- `tlk:to_bytes()`: the table in BioWare's layout.

## The assets

Textures, layouts, walkmeshes and models. Each table has `parse(bytes)` and `load(install, name, [module])`, which reads the copy the install loads; `name` without an extension takes the format's own (`m01aa` is `m01aa.lyt` to `rakata.lyt.load`), and no copy is `RE0404`. A vector is a sequence `{x, y, z}` in metres, Z up, and a rotation `{w, x, y, z}`. Bulk data is a sequence of vectors, and where the file stores it as one array, also the bytes as stored, a Lua string: a level's pixels, a walkmesh's vertices, a mesh's MDX block. A file read and written back with `to_bytes()` unchanged is the bytes it was read from.

### rakata.txi

A texture's instructions, from a `.txi` or a TPC: keywords, their values, and the numeric rows that follow some.

- `rakata.txi.parse(bytes)`: read the text; any bytes read, as Latin-1. `rakata.txi.new()`: an empty one.
- `txi:entries()`: `{key, value, rows}` each, in file order.
- `txi:preamble()`: the numeric lines before any keyword.
- `txi:get(key)`, `txi:int(key)`: the first value of `key`, in any case, and its first word as an integer; nil without one.
- `txi:set(key, value)`: set the first instruction with `key`, or append one; returns the value it had, or nil.
- `txi:len()`, `#txi`: the number of instructions.
- `txi:to_bytes()`: the text. One that would not read back as itself (a key with a space, a line break in a value) is `RE2208`.

### rakata.tpc

A TPC texture, its pixel format kept as stored: DXT1 and DXT5 levels are the blocks, the others the pixels in the file's channel order. `load` searches the texture pack the install selects.

- `tpc.width`, `tpc.height`: the header's size; a cube map's height is six faces'.
- `tpc.format`: `grey`, `rgb`, `rgba`, `dxt1` or `dxt5`. `tpc.has_alpha`.
- `tpc.mip_count`, `tpc.data_size`, `tpc.alpha_mean`: the header's fields.
- `tpc.layout`: `single`, `cube` or `flipbook`; `tpc.frames`, a flipbook's `{numx, numy}`, else nil.
- `tpc:images()`: each image (one, six faces, or the frames in row order) as a sequence of its levels, largest first, each `{width, height, offset, data}`, `offset` where it begins in the file.
- `tpc:pixels()`: every level of every image, back to back.
- `tpc:rgba([level], [image])`: level `level` (0 when nil) as RGBA8, four bytes a pixel, rows top first: the whole texture at the header's size (a cube's faces stacked top to bottom, a sheet's frames across then down), or the one face or frame `image` names, from 0. DXT1 and DXT5 are decoded; grey, RGB and RGBA bytes widened as stored. A level the texture lacks is `RE2210`, an image `RE2211`.
- `tpc:to_png([level], [image])`: what `rgba` gives, as an 8-bit RGBA PNG file.
- `tpc:txi()`: a copy of the TXI, or nil. `tpc:set_txi(txi)`: replace it, nil for none.
- `tpc:to_bytes()`: the header and pixels as read, then the TXI.

A file shorter than its header is `RE2201`, an encoding that names no format `RE2202`, pixel data that does not divide into its levels `RE2203`.

### rakata.tga

A TARGA image (the game's lightmaps), decoded to RGBA8, rows top first.

- `tga.width`, `tga.height`, `tga.bits`, `tga.alpha_bits`: as stored.
- `tga.image_type`: `truecolor`, `grey`, `truecolor_rle` or `grey_rle`; `tga.origin`: `bottom_left`, `bottom_right`, `top_left` or `top_right`.
- `tga:rgba()`: the pixels, four bytes each. `tga:set_rgba(bytes)`: replace them, `width * height * 4` bytes (`RE1805` otherwise).
- `tga:to_png()`: the pixels as an 8-bit RGBA PNG file.
- `tga:to_bytes()`: the image in its stored depth, order and packing.

A truncated file is `RE2204`, an image type or pixel depth it does not decode `RE2205`.

### rakata.lyt

An area's room layout, named as the area is (`m01aa`).

- `lyt.file_dependency`: the `filedependancy` line's value, or nil.
- `lyt:rooms()`, `lyt:tracks()`, `lyt:obstacles()`: `{model, position}` each.
- `lyt:door_hooks()`: `{room, name, unknown, position, orientation}` each.
- `lyt:room(model)`: the room whose model is `model`, in any case, or nil.
- `lyt:semantic_eq(other)`: whether the two hold the same rooms, tracks, obstacles and hooks, floats by their bits.
- `lyt:to_bytes()`: the text.

Text without `beginlayout` is `RE2301`; a line out of place `RE2302`, a number that does not read `RE2303`.

### rakata.vis

An area's room visibility: for each room, the rooms drawn while the camera is in it.

- `vis:rooms()`: `{room, visible}` each, `visible` a sequence of room names.
- `vis:visible(room)`: the rooms seen from `room`, in any case, or nil.
- `vis:len()`, `#vis`, `vis:semantic_eq(other)`, `vis:to_bytes()`.

### rakata.walkmesh

A room's WOK, a placeable's PWK or a door's DWK. `load` takes a bare name as the room's `.wok`; name the others with their extension (`plc_footlker.pwk`). Face, vertex and edge indices are the file's, from 0.

- `w.kind`: `area` (a WOK) or `object` (a PWK or DWK).
- `w.position`, `w.relative_points`, `w.absolute_points`, `w.unknown`: the header's fields, the points a sequence of two vectors.
- `w:vertices()`: every vertex. `w:vertex_bytes()`: the same as stored, twelve bytes each.
- `w:faces()`: `{vertices, material, normal, distance}` each, `material` the face's row of `surfacemat.2da`.
- `w:adjacency()`: for each walkable face, a table whose keys 1 to 3 are the `{face, edge}` across its edges, nil where no walkable face is.
- `w:perimeter_edges()`: `{face, edge, transition}` each, `transition` the index in the area's LYT room list of the room the edge leads into, or nil. `w:perimeters()`: where each loop of them ends.
- `w:aabb()`: the AABB tree, root first: `{min, max, face, unknown, plane, left, right}` each, `face` nil at an inner node and `left` and `right` nil at a leaf.
- `w:walkable_faces(surfacemat)`: the faces whose material `surfacemat.2da`'s table (a `rakata.twoda`) says is walkable; a table without a `walk` column is `RE2505`.
- `w:set_material(face, material)`: change a face's material; returns the one it had. A face past the last is `RE1805`.
- `w:semantic_eq(other)`, `w:to_bytes()`.

Bytes that do not begin `BWM V1.0` are `RE2501`; a truncated file `RE2502`.

### rakata.mdl

A binary model and its MDX. Nodes are named from 0 by their place in the sequence they are in, as `parent` and `children` give them.

- `rakata.mdl.parse(mdl, [mdx])`: read a model; the MDX may be left out for one whose meshes have none. `rakata.mdl.load(install, name, [module])` reads both.
- `model.name`, `model.supermodel` (nil for none), `model.subclassification`, `model.fog`, `model.radius`, `model.animation_scale`, `model.bounding_box` (`{min, max}`).
- `model.classification`: `other` (rooms), `effect`, `tile`, `character`, `door`, `lightsaber`, `placeable` or `unknown`.
- `model:nodes()`: every node of the geometry, the root first. A node is `{name, kind, flags, number, supernode, parent, children, position, orientation, controllers, mesh, skin, light, emitter, saber}`, `kind` one of `dummy`, `light`, `emitter`, `reference`, `trimesh`, `skin`, `animmesh`, `dangly`, `aabb`, `saber`, `other`; `position` and `orientation` relative to the parent; `mesh`, `skin`, `light`, `emitter` and `saber` nil on a node without them.
- `model:node(name)`: the first node named `name`, in any case, or nil.
- `model:animations()`, `model:animation(name)`: the model's own animations, `{name, length, transition, root, events, nodes}`, `events` `{time, name}` each and `nodes` as `model:nodes()` gives them.
- `model:to_bytes()`: two strings, the MDL and the MDX, laid out as BioWare's compiler lays them out.

A mesh is `{texture, lightmap, texture_count, lightmapped, render, shadow, transparency_hint, diffuse, ambient, bounding_box, radius, average, vertex_count, mdx_flags, stride, vertex_buffer, offsets, positions, normals, uv1, uv2, faces, indices}`: `vertex_buffer` the MDX block as stored and `offsets` each attribute read from it (`position`, `normal`, `uv1`, `uv2`) by its offset within `stride`; `positions` and `normals` sequences of vectors and `uv1` and `uv2` of `{u, v}`, the last three nil where the mesh has none; `faces` `{normal, distance, surface, adjacent, vertices}` each (`adjacent` -1 where no face is); `indices` the triangle list, or nil.

A skin is `{bones, weights, bone_indices, bind}`: `bones` the node numbers of the bones by local index; `weights` and `bone_indices` four numbers for each vertex (-1 where a slot names no bone); `bind[number]` the bone's bind transform, `{rotation, translation}`, by node number.

A light is `{flare_radius, flares, priority, ambient_only, dynamic_type, affect_dynamic, shadow, generate_flare, fading}`, `flares` `{texture, size, position, color_shift}` each. An emitter is `{dead_space, blast_radius, blast_length, lightning_words, x_grid, y_grid, space_type, update, render, blend, texture, chunk, two_sided, looping, render_order, name_187, flags}`: `update` `Single`, `Fountain`, `Explosion` or `Lightning` in vanilla, `texture` and `chunk` (the model each particle is) nil when the field names none, `name_187` the name at 187 as stored. A saber is `{positions, uv, normals, words}`, its vertices; its triangles are its mesh's faces. `docs/formats/mdl.md` ("Light header", "Emitter header", "Saber header") says what each field is and which meanings the files leave open.

A controller is `{kind, name, field, columns, is_bezier, is_compressed, keys}`, `kind` the controller type (8 position, 20 orientation, 36 scale), `name` its name on the kind of node that carries it (`radius` on a light, `birth_rate` on an emitter; nil when not established), and `keys` `{time, value}` each: an orientation's value its rotation, a compressed one decoded; any other controller's its row of numbers as stored (a bezier key's value, then its two control values).

A file that is not a binary model is `RE2401`, a truncated one `RE2402`, an MDX shorter than its meshes say `RE2403`.

## rakata.dialog

Dialogues, their nodes by `NodeRef` (list and index) and by `NodeKey` (what the node says, which survives edits). A function taking a key takes a `NodeKey`, a `NodeRef`, or the key's text: `voice:RESREF`, `strref:N`, `text:TEXT`, `E3`, `R0`, `entry:3`, `reply:0`. A key matching no node is `RE1102`, several `RE1103`, one that does not parse `RE1104`.

- `rakata.dialog.parse(name, bytes)`: read a DLG named `name`; a GFF that is not a dialogue is `RE1101`.
- `d:name()`: its name.
- `d:to_bytes()`: the file.
- `d:len(kind)`: the number of nodes of `entry` or `reply`.
- `d:node(at)`: `{at, key, speaker, listener, strref, text, voice, sound, script, delay, camera_angle, camera_id, quest, quest_entry, comment, animations, links}` for a node, or nil; `text` is the embedded English, `key` the node's stable key, `animations` `{participant, animation}` each, `links` `{target, condition, is_child}` each with `target` a `NodeRef`.
- `d:starts()`: the starting list, as `links`.
- `d:key_of(at)`: the stable key that names the node and no other.
- `d:gff()`: the dialogue's GFF, a copy.
- `d:clone()`: a copy.

### rakata.dialog.NodeRef

- `rakata.dialog.NodeRef.new(kind, index)`, `NodeRef.entry(index)`, `NodeRef.reply(index)`: a node by list and index.
- `ref.kind`, `ref.index`: `entry` or `reply`, and the index from 0. `tostring(ref)` is `E3`; refs compare with `==`.

### rakata.dialog.NodeKey

- `rakata.dialog.NodeKey.parse(text)`, `NodeKey.voice(resref)`, `NodeKey.strref(n)`, `NodeKey.text(text)`, `NodeKey.index(kind, i)`: a key.
- `rakata.dialog.NodeKey.of(dialog, at)`: the stable key naming the node at `at`: its voice resref, else its strref, else its text, whichever names it alone; an index key otherwise.
- `key:is_fragile()`: whether the key names a position, which moves when a list is reordered.
- `key:resolve(dialog)`: the `NodeRef`, text compared with the text a node embeds.
- `tostring(key)` is the key's text, which `parse` reads back; keys compare with `==`.

### rakata.dialog.Session

Dialogues loaded from the install, with transcripts whose conditions are decompiled.

- `rakata.dialog.Session.open([{profile = "k1", module = "danm14ac"}])`: a session over the install `--game` names (`RE1803` without it).
- `s:load(what)`: `{dialog, location, module}`. `what` is `NAME.dlg`, resolved through the session's module (with none set and no global copy, or one only in `Override/`, the one module holding it is found and set; several are `RE1105`), or a path with a directory in it (`./banter.dlg`), read when it is one of the script's inputs.
- `s:set_module(module)`, `s:module()`: the module whose capsules are searched.
- `s:transcript(dialog, [{scripts = false}])`: the transcript's fields as `rakata --format json dialog show` gives them under `result.transcript` (`dialog`, `info`, `entries`, `replies`, `starts`, `nodes`, `scripts`, `tree`, `warnings`), and `text`, the transcript as text. `scripts = true` decompiles the action scripts too.
- `s:resolve(dialog, key)`: the `NodeRef`, text keys compared through the talk table.

## rakata.author

The authoring operations ([`rakata::author`](../rakata/src/author/mod.rs)), each on a value a format table holds; nothing is written until `Mod:write_override`.

### rakata.author.twoda

- `rakata.author.twoda.find(t, key)`: the index of the one row `key` names: an index, or a label as the game's tables mean one (the `label` column where the table has one, the row label otherwise). No row is `RE0705`; a label matching several is `RE1405`.
- `rakata.author.twoda.add_row_copying(t, from, new_label, [cells])`: append a copy of the row `from` labelled `new_label` with `cells` (`{column = value}`) changed, and return its index. Where the table has a `label` column the label goes there and the row label is the new index; otherwise `new_label` is the row label. A label already carried is `RE1405`.

### rakata.author.globals

- `rakata.author.globals.declare(t, name, type)`: declare the global `name`, of type `Boolean` or `Number`, in `globalcat.2da`'s table; returns the row's index. A name already declared, in any case, is `RE1404`.

### rakata.author.blueprint

- `rakata.author.blueprint.get(g, path)`: the plain value at `path`.
- `rakata.author.blueprint.get_value(g, path)`: the typed value.
- `rakata.author.blueprint.set(g, path, value)`: set the field, adding it when its struct lacks it; returns the old plain value or nil. A typed value of another type than the field holds is `RE1406`.
- `rakata.author.blueprint.set_each(g, list, label, value)`: set `label` in every element of the list at `list`, returning how many; a path naming no list changes nothing and returns 0.
- `rakata.author.blueprint.clone(g, template)`: a copy whose `TemplateResRef` is `template`.

### rakata.author.dialog

A new node is a table `{text, speaker, listener, voice, script, animations, quest, camera, comment}`, every field optional: `text` a string (English, embedded) or an integer (the game's strref); `voice` and `script` resrefs; `animations` `{{participant, animation}, ...}`; `quest` `{category, entry}`; `camera` a camera id, placing a static camera. A reply ignores `speaker`, `animations`, `quest` and `camera`.

- `rakata.author.dialog.add_entry(d, node)`: append an entry; returns its `NodeRef`.
- `rakata.author.dialog.add_reply(d, node)`: append a reply; returns its `NodeRef`.
- `rakata.author.dialog.link(d, from, to, [{condition = "k_c", at = 0}])`: link `from` to `to`, of the other kind, offered when the condition script returns true, at `at` among `from`'s links or after the last; returns the link's position. Two nodes of one kind or a position past the end is `RE1106`.
- `rakata.author.dialog.unlink_all(d, from)`: remove every link out of `from`.
- `rakata.author.dialog.start(d, to, [{condition = "k_c", at = 0}])`: a start into the entry `to`, at `at` in the starting list, first by default; returns its position.
- `rakata.author.dialog.reorder(d, kind, order)`: reorder the `entry` or `reply` list, `order[i]` (from 1, as a Lua sequence) the old index from 0 of the node that goes to position `i - 1`; links and starts follow. Not a permutation is `RE1107`.
- `rakata.author.dialog.empty_like(d, name)`: a dialogue named `name` with `d`'s settings and no lines.
- `rakata.author.dialog.CAMERA_STATIC`, `NO_DELAY`: the camera angle a placed camera takes, and the delay that means none.

### rakata.author.lip

- `rakata.author.lip.from_cues(cues, length)`: as `rakata.lip.from_cues`.
- `rakata.author.lip.rhubarb_shape(letter)`: the game's shape label for one of Rhubarb Lip Sync's mouth shapes (`A` to `H`, `X`), or nil.

### rakata.author.scripts

- `rakata.author.scripts.compile(m, sources, [{includes = {...}, flags = "default"}])`: compile each source against the install's includes, `nwscript.nss` and includes found first among `sources` and `includes`, then through `m`. `sources` and `includes` are `{{name = "k_x", text = "..."}, ...}` or `{k_x = "..."}`; `flags` is `default` or `bioware` (BioWare's declarations and compatibility, so a decompiled BioWare script compiles to BioWare's bytes). Returns a sequence of `{name, ncs, warnings, inputs}`, `inputs` each `{role, path, sha256}`. Nothing is written; a name over sixteen bytes is `RE0101` before anything compiles, and a script that does not compile is `RE1408`.

### rakata.author.voice

The part of a voice pipeline that is not ffmpeg or Rhubarb.

- `rakata.author.voice.take(name, audio, transcript)`: a take, `{name, hash, segments}`, from its audio's bytes and its AWS Transcribe JSON; `segments` `{start, end}` in seconds. A transcript without segments is `RE1412`.
- `rakata.author.voice.spans(lines, takes, [lead])`: the span of each line, `lead` seconds kept each side. A line is `{resref, take, text, first, last}`, `first` and `last` the take's segments from 0, inclusive (nil for the whole take or to its end). Returns `{resref, take, text, start, ["end"], start_text, end_text, key}` each, `key` the cache key of the cut. Two lines of one resref are `RE1411`; a span the take does not hold is `RE1410`.
- `rakata.author.voice.take_cues(json)`: a LIP from Rhubarb's JSON cues.
- `rakata.author.voice.streamwave(mp3)`: the MP3 behind the 58-byte header the game's `streamwaves/` files carry.
