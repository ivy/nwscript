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
