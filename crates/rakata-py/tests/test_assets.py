"""Textures, layouts, walkmeshes and models: read from bytes and from the user's KotOR 1 install
(`$RAKATA_GAME_K1`), and written back as the bytes they were read from."""

import struct
from pathlib import Path

import pytest

import rakata

LYT = b"""beginlayout
   roomcount 2
      m01aa_01a 15.7058 22.2345 0.0
      m01aa_02a 30.777 20.707 0.0
   trackcount 0
   obstaclecount 0
   doorhookcount 1
      m01aa_01a door_16 0 21.1225 20.713 -1.31576 0.707107 0.0 0.0 -0.707107
donelayout
"""

VIS = b"""m01aa_01a 1
  m01aa_02a
m01aa_02a 1
  m01aa_01a
"""


def test_a_layout_reads_its_rooms_and_hooks() -> None:
    lyt = rakata.Lyt.parse(LYT)
    assert [r.model for r in lyt.rooms] == ["m01aa_01a", "m01aa_02a"]
    assert lyt.rooms[0].position == pytest.approx((15.7058, 22.2345, 0.0))
    room = lyt.room("M01AA_02A")
    assert room is not None and room.model == "m01aa_02a"
    (hook,) = lyt.door_hooks
    assert (hook.room, hook.name, hook.unknown) == ("m01aa_01a", "door_16", 0)
    assert hook.orientation == pytest.approx((0.707107, 0.0, 0.0, -0.707107))
    assert lyt.tracks == [] and lyt.obstacles == []
    assert lyt.to_bytes() == LYT
    assert lyt.semantic_eq(rakata.Lyt.parse(lyt.to_bytes()))
    with pytest.raises(rakata.RakataError) as e:
        rakata.Lyt.parse(b"roomcount 1\n")
    assert e.value.code == "RE2301"


def test_a_visibility_file_reads_its_rooms() -> None:
    vis = rakata.Vis.parse(VIS)
    assert vis.rooms == [("m01aa_01a", ["m01aa_02a"]), ("m01aa_02a", ["m01aa_01a"])]
    assert vis.visible("M01AA_01A") == ["m01aa_02a"]
    assert vis.visible("m01aa_03a") is None
    assert len(vis) == 2
    assert vis.to_bytes() == VIS


def test_a_txi_is_edited_and_written() -> None:
    txi = rakata.Txi.parse(b"envmaptexture CM_Baremetal\r\nnumchars 2\r\n")
    assert txi.entries == [("envmaptexture", "CM_Baremetal", []), ("numchars", "2", [])]
    assert txi.get("ENVMAPTEXTURE") == "CM_Baremetal" and txi.int("numchars") == 2
    assert txi.set("envmaptexture", "CM_dsk") == "CM_Baremetal"
    assert txi.set("blending", "additive") is None
    assert txi.to_bytes() == b"envmaptexture CM_dsk\r\nnumchars 2\r\nblending additive\r\n"
    assert len(txi) == 3
    txi.set("bad key", "x")
    with pytest.raises(rakata.RakataError) as e:
        txi.to_bytes()
    assert e.value.code == "RE2208"


def test_the_binary_formats_refuse_with_their_codes() -> None:
    cases = [
        (lambda: rakata.Tpc.parse(b"short"), "RE2201"),
        (lambda: rakata.Tga.parse(b"short"), "RE2204"),
        (lambda: rakata.Walkmesh.parse(b"not a walkmesh at all"), "RE2501"),
        (lambda: rakata.Model.parse(b"\x01" * 16), "RE2401"),
    ]
    for f, code in cases:
        with pytest.raises(rakata.RakataError) as e:
            f()
        assert e.value.code == code


def test_a_file_that_is_not_there_is_re0001(tmp_path: Path) -> None:
    with pytest.raises(rakata.RakataError) as e:
        rakata.Walkmesh.read(tmp_path / "none.wok")
    assert e.value.code == "RE0001"


@pytest.fixture(scope="module")
def install(game_k1: Path) -> rakata.Install:
    return rakata.Install(game_k1)


def test_end_m01aa_layout_and_visibility_round_trip(install: rakata.Install) -> None:
    raw = install.read("m01aa.lyt", module="end_m01aa")
    lyt = rakata.Lyt.load(install, "m01aa", module="end_m01aa")
    assert len(lyt.rooms) == 15 and len(lyt.door_hooks) == 15
    assert lyt.file_dependency == "M01aa.max"
    assert lyt.to_bytes() == raw
    vis = rakata.Vis.load(install, "m01aa", module="end_m01aa")
    assert {r.lower() for r, _ in vis.rooms} == {r.model.lower() for r in lyt.rooms}
    assert vis.to_bytes() == install.read("m01aa", "vis", module="end_m01aa")


def test_each_room_of_end_m01aa_has_a_model_and_a_walkmesh(install: rakata.Install) -> None:
    lyt = rakata.Lyt.load(install, "m01aa", module="end_m01aa")
    classes: dict[str, str] = {}
    for room in lyt.rooms:
        model = rakata.Model.load(install, room.model, module="end_m01aa")
        assert model.name.lower() == room.model.lower()
        classes[room.model.lower()] = model.classification
        assert any(n.mesh is not None and n.mesh.render for n in model.nodes)
        wok = rakata.Walkmesh.load(install, room.model, module="end_m01aa")
        assert wok.kind == "area"
        assert wok.to_bytes() == install.read(room.model, "wok", module="end_m01aa")
    # m01aa_07b, the view of space, is classed an effect (docs/formats/mdl.md)
    assert {r for r, c in classes.items() if c != "other"} == {"m01aa_07b"}


def test_a_room_model_reads_its_meshes_and_writes_back(install: rakata.Install) -> None:
    model = rakata.Model.load(install, "m01aa_01a")
    assert model.nodes[0].parent is None and model.nodes[0].name == model.name
    for i, n in enumerate(model.nodes):
        for c in n.children:
            assert model.nodes[c].parent == i
    node = model.node("OBJECT3232")
    assert node is not None and node.kind == "trimesh"
    mesh = node.mesh
    assert mesh is not None
    assert (mesh.texture, mesh.lightmap) == ("LHR_flr02", "M01aa_01a_lm1")
    assert mesh.offsets == {"position": 0, "normal": 12, "uv1": 24, "uv2": 32}
    assert len(mesh.vertex_buffer) == mesh.vertex_count * mesh.stride
    positions = [
        struct.unpack_from("<3f", mesh.vertex_buffer, i * mesh.stride + mesh.offsets["position"])
        for i in range(mesh.vertex_count)
    ]
    assert positions == mesh.positions
    uv2 = mesh.uv2
    assert uv2 is not None and len(uv2) == mesh.vertex_count
    assert mesh.indices == [v for f in mesh.faces for v in f.vertices]
    assert [k.kind for k in node.controllers][:2] == [8, 20]
    mdl, mdx = model.to_bytes()
    assert mdl == install.read("m01aa_01a.mdl") and mdx == install.read("m01aa_01a.mdx")


def test_a_character_model_reads_its_skin_and_animations(install: rakata.Install) -> None:
    model = rakata.Model.load(install, "c_bantha")
    assert model.classification == "character"
    skinned = [n for n in model.nodes if n.skin is not None]
    assert skinned
    skin = skinned[0].skin
    assert skin is not None
    for w, ix in zip(skin.weights, skin.bone_indices):
        assert sum(w) == pytest.approx(1.0, abs=1e-5)
        assert all(i == -1 or 0 <= i < len(skin.bones) for i in ix)
    for b, node in enumerate(skin.bones):
        assert skin.bone_index(node) == b
        assert skin.qbone(node) is not None and skin.tbone(node) is not None
    walk = model.animation("CWALK")
    assert walk is not None and walk.length > 0
    keyed = [c for n in walk.nodes for c in n.controllers if c.kind == 20]
    assert keyed
    for c in keyed:
        for _, q in c.keys:
            # a compressed key quantizes x, y and z, so it is unit length only nearly
            assert sum(x * x for x in q) == pytest.approx(1.0, abs=1e-2)
    mdl, mdx = model.to_bytes()
    assert (mdl, mdx) == (install.read("c_bantha.mdl"), install.read("c_bantha.mdx"))


def test_lights_emitters_and_sabers_read_as_typed_values(install: rakata.Install) -> None:
    muzzle = rakata.Model.load(install, "v_muzflash_01")
    node = muzzle.node("muzflash01")
    assert node is not None and node.kind == "emitter"
    e = node.emitter
    assert e is not None
    assert (e.update, e.render, e.blend) == ("Explosion", "Billboard_to_Local_Z", "Lighten")
    assert e.texture == "fx_muzflash" and e.chunk is None
    assert e.two_sided and e.name_187 == "NULL"
    assert "birth_rate" in [c.name for c in node.controllers]
    sun = rakata.Model.load(install, "gidy_sun")
    amb, diff = sun.node("gidy_sun_amb"), sun.node("gidy_sun_diff")
    assert amb is not None and diff is not None
    assert amb.light is not None and diff.light is not None
    assert amb.light.ambient_only and not diff.light.ambient_only
    assert diff.light.shadow and not amb.light.shadow
    assert [c.name for c in amb.controllers if c.kind == 88] == ["radius"]
    blades = rakata.Model.load(install, "w_dblsbr_001")
    blade = blades.node("plane269")
    assert blade is not None and blade.saber is not None and blade.mesh is not None
    assert blade.saber.positions == blade.mesh.positions
    assert len(blade.saber.uv) == len(blade.saber.normals) == 176
    assert blade.saber.words == (98, 97)
    planet = rakata.Model.load(install, "planet_01").node("Planet")
    assert planet is not None and planet.mesh is not None
    assert planet.mesh.texture_motion is not None and planet.mesh.texture_motion[1] == 0.0
    assert planet.mesh.area > 0
    assert len(blade.saber.triangles) == 12
    assert {v for t in blade.saber.triangles for v in t} >= {0, 4, 88, 92}


def test_a_room_walkmesh_reads_and_changes_a_material(install: rakata.Install) -> None:
    raw = install.read("m01aa_01a.wok")
    wok = rakata.Walkmesh.parse(raw)
    assert len(wok.vertex_bytes) == 12 * len(wok.vertices)
    assert list(struct.iter_unpack("<3f", wok.vertex_bytes)) == wok.vertices
    surfacemat = rakata.TwoDa.parse(install.read("surfacemat.2da"))
    walkable = wok.walkable_faces(surfacemat)
    assert walkable == list(range(len(wok.adjacency)))
    for (face, edge), _ in wok.perimeter_edges:
        assert face in walkable and edge in (0, 1, 2)
    assert wok.perimeters[-1] == len(wok.perimeter_edges)
    leaves = [n.face for n in wok.aabb if n.face is not None]
    assert sorted(leaves) == list(range(len(wok.faces)))
    old = wok.set_material(0, 7)
    edited = rakata.Walkmesh.parse(wok.to_bytes())
    assert edited.faces[0].material == 7 and not edited.semantic_eq(rakata.Walkmesh.parse(raw))
    edited.set_material(0, old)
    assert edited.to_bytes() == raw
    with pytest.raises(IndexError):
        wok.set_material(len(wok.faces), 0)


@pytest.mark.parametrize("name", ["plc_footlker.pwk", "dor_lhr010.dwk"])
def test_an_object_walkmesh_writes_back(install: rakata.Install, name: str) -> None:
    w = rakata.Walkmesh.load(install, name)
    assert w.kind == "object" and w.aabb == [] and w.adjacency == []
    assert w.to_bytes() == install.read(name)


def test_a_texture_by_name_through_the_texture_pack(install: rakata.Install) -> None:
    raw = install.read("LHR_flr02.tpc")
    tpc = rakata.Tpc.load(install, "LHR_flr02")
    assert tpc.format in ("dxt1", "dxt5") and tpc.layout == "single" and tpc.frames is None
    (image,) = tpc.images
    assert len(image) == tpc.level_count == tpc.mip_count
    assert (image[0].width, image[0].height) == tpc.image_size == (tpc.width, tpc.height)
    assert b"".join(level.data for level in image) == tpc.pixels
    assert raw[image[1].offset : image[1].offset + len(image[1].data)] == image[1].data
    assert tpc.to_bytes() == raw
    txi = tpc.txi
    assert txi is not None
    txi.set("bumpmaptexture", "LHR_flr02B")
    tpc.txi = txi
    edited = rakata.Tpc.parse(tpc.to_bytes())
    again = edited.txi
    assert again is not None and again.get("bumpmaptexture") == "LHR_flr02B"
    assert edited.pixels == tpc.pixels
    tpc.txi = None
    assert rakata.Tpc.parse(tpc.to_bytes()).txi is None


def test_a_standalone_txi_writes_back(install: rakata.Install) -> None:
    raw = install.read("lbl_menudarr.txi")
    txi = rakata.Txi.load(install, "lbl_menudarr")
    assert txi.entries and txi.to_bytes() == raw


def test_a_lightmap_decodes_and_writes_back(install: rakata.Install) -> None:
    raw = install.read("M01aa_01a_lm1.tga")
    tga = rakata.Tga.load(install, "M01aa_01a_lm1")
    assert len(tga.rgba) == tga.width * tga.height * 4
    assert tga.to_bytes() == raw
    tga.rgba = bytes(len(tga.rgba))
    black = rakata.Tga.parse(tga.to_bytes())
    assert set(black.rgba[i] for i in range(0, len(black.rgba), 4)) == {0}
    with pytest.raises(ValueError):
        tga.rgba = b"\x00"


def test_a_texture_decodes_to_rgba_and_png(install: rakata.Install) -> None:
    tpc = rakata.Tpc.load(install, "LHR_flr02")
    rgba = tpc.rgba()
    assert len(rgba) == tpc.width * tpc.height * 4
    assert len(tpc.rgba(level=1)) == len(rgba) // 4
    png = tpc.to_png()
    assert png.startswith(b"\x89PNG\r\n\x1a\n")
    assert struct.unpack(">II", png[16:24]) == (tpc.width, tpc.height)
    with pytest.raises(rakata.RakataError) as e:
        tpc.rgba(level=tpc.level_count)
    assert e.value.code == "RE2210"
    with pytest.raises(rakata.RakataError) as e:
        tpc.rgba(image=1)
    assert e.value.code == "RE2211"
    tga = rakata.Tga.load(install, "M01aa_01a_lm1")
    assert struct.unpack(">II", tga.to_png()[16:24]) == (tga.width, tga.height)
