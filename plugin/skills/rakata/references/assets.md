# Textures, layouts, walkmeshes and models

The four asset formats are read by their own commands (`find.md`, "Textures, layouts, walkmeshes and models"), compared by `rakata diff` (`check.md`), and their references are in the index and the tree lint (`refs.md`, `check.md`). This file is the rest: how a question that spans an area's rooms, models and textures is answered, and the bindings that answer it in code.

## What names what

- An area's layout, `AREA.lyt` (the area named as its module's `.are` is, `m01aa` in module `end_m01aa`), places its rooms: each room is a model, `ROOM.mdl` with its `ROOM.mdx`, and its walkmesh `ROOM.wok`, found as any resource is (`rakata which ROOM.mdl --module STEM --game DIR`; the game's room models are in `data/models.bif`). `AREA.vis` says which rooms are drawn from each.
- A model's meshes name a texture and, for a room, a lightmap. A walkmesh names nothing: its transitions are room numbers, positions in the layout's room list.
- A texture name is satisfied by a `.tpc`, a `.tga` or a `.dds` of that name, as RW1906 checks it (`rakata explain RW1906`). The game's textures are TPCs in the texture packs (`TexturePacks/swpc_tex_tpa.erf`); its lightmaps (`ROOM_lmN`) are TGAs in `data/lightmaps.bif`. So `rakata which NAME.tpc` alone calls every lightmap missing: try `.tga` too before calling a name unresolved.
- A room may have no textures (a model holding only its walkmesh node) or no lightmaps. `NULL` and an empty name are no reference.
- Model and texture names are written in mixed case (`M01aa_04a`); every command and binding compares them without regard to case.

## Worked example: an area's rooms and their textures

The question: for the Endar Spire's first area (module `end_m01aa`, area `m01aa`), its rooms, each room's model, the textures and lightmaps those models use, and whether each resolves.

By hand, one command per step:

```bash
rakata lyt show m01aa.lyt --module end_m01aa --game DIR          # the rooms: each model and its position
rakata mdl show m01aa_01a.mdl --module end_m01aa --game DIR      # one room: each mesh's texture and lightmap
rakata which lhr_flr01.tpc --game DIR                            # a texture: the copy the game loads, or RE0404
rakata which m01aa_01a_lm0.tga --game DIR                        # a lightmap
```

That is one `mdl show` per room and one `which` per name, hundreds of calls for an area. Three ways do it at once:

1. **The tree lint names only what does not resolve.** `rakata --format json lint --game DIR` puts every finding in `result.findings`, each `{code, path, site, value, want}` (`path` the model's location, `data/models.bif!m01aa_04a.mdl`); keep the RW1906 findings whose `path` ends in one of the rooms' `.mdl` (or is the area's `.lyt`). Every other name the models use resolves. Vanilla KotOR has thousands of findings of its own, and the command exits 1 when it has any. It takes about a second over a KotOR 1 install.
2. **The index holds every name with where it lies.** Build it once (`refs.md`), then `rakata refs m01aa_01a.mdl --from --db FILE` lists one model's textures and lightmaps by site (`NODE.texture`, `NODE.lightmap`). One `index query --file` report gives the whole area, joining the layout's rooms to their models' references and each name to where a texture of it lies:

   ```sql
   CREATE TEMP VIEW rooms AS
     SELECT DISTINCT r.value AS room FROM locations l JOIN refs r ON r.from_hash = l.hash
     WHERE l.name = 'm01aa' AND l.type = 'lyt' AND r.site LIKE 'room[%';
   CREATE TEMP VIEW uses AS
     SELECT DISTINCT rooms.room, r.value AS name,
            CASE WHEN r.site LIKE '%.lightmap' THEN 'lightmap' ELSE 'texture' END AS role
     FROM rooms JOIN locations m ON m.name = rooms.room COLLATE NOCASE AND m.type = 'mdl'
     JOIN refs r ON r.from_hash = m.hash
     WHERE r.site LIKE '%.texture' OR r.site LIKE '%.lightmap';
   SELECT u.room, u.role, u.name,
          (SELECT min(t.path || CASE t.member WHEN '' THEN '' ELSE '!' || t.member END) FROM locations t
           WHERE t.name = u.name COLLATE NOCASE AND t.type IN ('tpc', 'tga', 'dds')) AS lies_at
   FROM uses u ORDER BY u.room, u.role DESC, u.name;
   ```

   `lies_at` empty is a name nothing holds. A room with no textures has no rows, so compare the rooms listed with the layout's.
3. **A script walks the formats.** `rakata lua SCRIPT --game DIR` runs Lua with the bindings and needs nothing installed beyond `rakata`:

   ```lua
   local install = rakata.Install.open()
   local function lies_at(name)
     for _, ext in ipairs({ "tpc", "tga", "dds" }) do
       local copy = install:read(name .. "." .. ext, "end_m01aa")
       if copy then return copy.location end
     end
   end
   local lyt = rakata.lyt.load(install, "m01aa", "end_m01aa")
   for _, room in ipairs(lyt:rooms()) do
     local model = rakata.mdl.load(install, room.model, "end_m01aa")
     for _, node in ipairs(model:nodes()) do
       local mesh = node.mesh
       if mesh then
         for _, name in ipairs({ mesh.texture or "", mesh.lightmap or "" }) do
           if name ~= "" and name:upper() ~= "NULL" then
             print(room.model, node.name, name, lies_at(name) or "MISSING")
           end
         end
       end
     end
   end
   ```

Whichever way is used, check the answer against another: the lint's unresolved names should be exactly the report's or the script's `MISSING` rows. A name the vanilla game itself leaves unresolved is the game's, not a mod's (`check.md`, "Lint the tree").

## The bindings

The same formats in Python and Lua, read through the install (the copy the game loads, texture packs included) or from bytes. Their full reference is the stubs (`author.md`, "Before you start") and `rakata lua --reference`; read the class you need there.

- **Python**: `rakata.Lyt`, `rakata.Vis`, `rakata.Walkmesh`, `rakata.Model`, `rakata.Tpc`, `rakata.Txi`, `rakata.Tga`, each with `parse(bytes)`, `read(path)` and `load(install, name, module=None)`, a bare name taking the format's extension (`Walkmesh` a room's `.wok`). `Model.load` reads the MDX with the MDL. `install.which(name, ext, module=...)` resolves a name and raises `RakataError` with code `RE0404` when no layer holds it; `install.read` returns the bytes.
- **Lua**: `rakata.lyt`, `rakata.vis`, `rakata.walkmesh`, `rakata.mdl`, `rakata.tpc`, `rakata.txi`, `rakata.tga`, each with `parse` and `load(install, name, [module])`; lists are methods (`lyt:rooms()`, `model:nodes()`, `w:faces()`), indices into the file's lists count from 0, and `install:read(name, [module])` returns nil when no layer holds the name, else `{bytes, location}`; a BIF's location is `data/NAME.bif[N]`, which carries no extension, so the type found is the one asked for.

The area's layout in Python, as the Lua above reads it:

```python
import rakata

install = rakata.Install("DIR")
lyt = rakata.Lyt.load(install, "m01aa", module="end_m01aa")
for room in lyt.rooms:
    model = rakata.Model.load(install, room.model, module="end_m01aa")
    for node in model.nodes:
        if node.mesh is not None:
            print(room.model, node.name, node.mesh.texture, node.mesh.lightmap)
```

Edits are few: `Txi.set`, a TPC's `txi`, a TGA's `rgba` and `Walkmesh.set_material`, each written back with `to_bytes()` and `m.write_override` (`author.md`, "Writes"); layouts and models are read-only. Each format's file read and written back unchanged is the bytes it was read from.
