# Walkmeshes: WOK, PWK and DWK

A walkmesh is the triangle mesh the engine walks creatures on and tests lines and clicks against. Each room model of an area has one, `ROOM.wok`; each placeable model may have one, `MODEL.pwk`; each door model has three, `MODEL0.dwk` to `MODEL2.dwk`. All three types share one binary layout, which begins `BWM V1.0`.

Every fact below is tagged **measured** (seen in the bytes of a vanilla file, or in a comparison between files) or **inferred**. The measurements are over every WOK, PWK and DWK a KotOR 1 install holds, read through `rakata-game` from `$RAKATA_GAME_K1`; the census that makes them is `crates/rakata-walkmesh/tests/vanilla.rs`, run as `RAKATA_GAME_K1=… cargo test -p rakata-walkmesh --test vanilla -- --nocapture`. Where a count below has exceptions, the census asserts the exact list.

## Where the files are

1,202 WOK, 196 PWK and 156 DWK, all in `data/models.bif`; no module capsule (`.rim`, `_s.rim`, `.mod`) and no file in `Override/` holds one (**measured**). The DWKs are 52 doors with three files each, the model name followed by `0`, `1` and `2` (`dor_lda010.dwk`, `dor_lda011.dwk`, `dor_lda012.dwk`) (**measured**); which door state each number is, is not established.

## Header

136 bytes, little-endian.

| Offset | Type | Field |
| --- | --- | --- |
| 0 | `char[8]` | `BWM V1.0` in all 1,554 files |
| 8 | `u32` | type: 1 in every WOK, 0 in every PWK and DWK |
| 12 | `f32[3]` | relative point 1 |
| 24 | `f32[3]` | relative point 2 |
| 36 | `f32[3]` | absolute point 1 |
| 48 | `f32[3]` | absolute point 2 |
| 60 | `f32[3]` | position |
| 72 | `u32` | vertex count |
| 76 | `u32` | vertex offset |
| 80 | `u32` | face count |
| 84 | `u32` | face offset |
| 88 | `u32` | material offset (face count entries) |
| 92 | `u32` | normal offset (face count entries) |
| 96 | `u32` | plane distance offset (face count entries) |
| 100 | `u32` | AABB node count |
| 104 | `u32` | AABB offset |
| 108 | `u32` | unknown |
| 112 | `u32` | adjacency count |
| 116 | `u32` | adjacency offset |
| 120 | `u32` | perimeter edge count |
| 124 | `u32` | perimeter edge offset |
| 128 | `u32` | perimeter count |
| 132 | `u32` | perimeter offset |

Offsets are from the start of the file (**measured**: each is the end of the array before it, below).

**The five points.** In every WOK all five are zero except the position, which is non-zero in 973 of the 1,202 (`m01aa_01a.wok`: −0.286, 0.606, 0.21); what a WOK's position is, is not established. In every DWK each absolute point is its relative point plus the position, within 1e-3 (`dor_lda010.dwk`: relative (0.0522, −0.5577, 0), position (−0.0522, −0.1369, 0), absolute (0, −0.6946, 0)). In every PWK both absolute points are zero; the relative points are non-zero in 185 and 187 of the 196 (**measured**). What the engine uses the points for is not established; their placement either side of a door's or placeable's centre suggests the points a creature walks to in order to use the object (**inferred**).

**The field at 108** is 0 in every PWK and DWK and in 1,173 WOKs; the other 29 WOKs hold values with no pattern found (`m01aa_07b.wok` 151957088, `m02ac_02t.wok` 1065353216, seven hold 4294967295) (**measured**). It is not established; a reader ignores it.

## Arrays

In every file the arrays lie in this order, each starting where the one before ends, and the file ends with the last (**measured**, 1,554 of 1,554):

| Array | Element | Size |
| --- | --- | --- |
| vertices | `f32[3]` X, Y, Z | 12 |
| faces | `u32[3]` vertex indices | 12 |
| materials | `u32`, a row of `surfacemat.2da` | 4 |
| normals | `f32[3]` | 12 |
| plane distances | `f32` | 4 |
| AABB nodes | below | 44 |
| adjacency | `i32[3]` | 12 |
| perimeter edges | `u32` edge, `i32` transition | 8 |
| perimeters | `u32` | 4 |

An empty array's offset is where it would start in every WOK (in a WOK with no faces every offset is 136). In a PWK or DWK the vertex to plane-distance offsets follow the same rule (136 in the 74 PWKs that are a bare header), and the offsets of the AABB, adjacency, perimeter edge and perimeter arrays, which are empty in every PWK and DWK, are 0 (**measured**). A reader takes an array of count 0 as empty whatever its offset.

### Vertices and the frame

Metres, Z up. A WOK's vertices are in the area's frame, the room's LYT origin already applied: each of the 14 non-empty rooms of `m01aa` has its `m01aa.lyt` origin inside its vertices' X–Y extent (`M01aa_01a` at (15.7058, 22.2345) spans X 10.1–20.7, Y 15.2–30.4), and neighbouring rooms meet in area coordinates (`M01aa_01a` and `M01aa_02a` share two vertices bit for bit) (**measured**). A runtime does not add the LYT origin to a WOK. A PWK's or DWK's vertices lie within 18.1 m (PWK) and 12.4 m (DWK) of the origin on every axis (**measured**), in the object's own frame, placed by the object's position and bearing (**inferred**).

### Faces, normals and plane distances

Three `u32` indices into the vertex array; every index is in range (**measured**). Seen from the side the normal points to, the vertices run counter-clockwise: the stored normal is the unit vector of (v1 − v0) × (v2 − v0) within 1e-3 for 189,476 of the 189,588 WOK faces and all 976 DWK faces. The other 112 WOK faces, in 45 files (`m02ae_09a.wok`, `m02ae_10a.wok`), have zero area and store the normal (1, 0, 0) (**measured**).

The plane distance is −(normal · v0), within 1e-2, for every WOK and DWK face, the zero-area ones included (**measured**).

In every PWK the 765 normals are (0, 0, 0) and the plane distances are values unrelated to the face (`plc_lever.pwk`: 1.7e-33, 2.1e17) (**measured**). A reader computes a PWK's normals from its vertices.

### Materials and walkability

A face's material is a row of `surfacemat.2da`; the `walk` column says whether the face is walkable. The vanilla table (in `data/2da.bif`, and identical copies in `global.rim` and `miniglobal.rim`) has 31 rows, labelled `0` to `30`, `walk` 1 for rows 1, 3, 4, 5, 6, 9, 10, 11, 12, 13, 14, 18 and 30 (**measured**). The materials the walkmeshes use (**measured**):

| Material | `label` | `walk` | WOK faces | PWK faces | DWK faces |
| --- | --- | --- | --- | --- | --- |
| 1 | Dirt | 1 | 57,142 | 3 | 56 |
| 2 | Obscuring | 0 | 15,869 | | 8 |
| 3 | Grass | 1 | 12,143 | | |
| 4 | Stone | 1 | 6,501 | | |
| 5 | Wood | 1 | 882 | | |
| 6 | Water | 1 | 47 | | |
| 7 | Nonwalk | 0 | 74,009 | 762 | 912 |
| 8 | Transparent | 0 | 103 | | |
| 9 | Carpet | 1 | 20 | | |
| 10 | Metal | 1 | 18,534 | | |
| 11 | Puddles | 1 | 16 | | |
| 13 | Mud | 1 | 327 | | |
| 17 | DeepWater | 0 | 29 | | |
| 19 | NonWalkGrass | 0 | 3,966 | | |

95,612 of the 189,588 WOK faces (50.4%) are walkable, 3 of 765 PWK faces and 56 of 976 DWK faces (**measured**).

In every WOK the walkable faces come first: faces 0 to _n_ − 1 are walkable and the rest are not, where _n_ is the adjacency count (**measured**, 1,202 of 1,202). The 11 WOKs with faces and no adjacency have no walkable face. Four DWKs have walkable faces after unwalkable ones (`dor_lts051.dwk`, `dor_lts052.dwk`, `dor_lts101.dwk`, `dor_lts102.dwk`) (**measured**).

### Adjacency

One row per walkable face, in face order: for each of the face's three edges, the edge of the walkable face across it, or −1 (**measured**). Edge _e_ of a face runs from its vertex _e_ to its vertex (_e_ + 1) mod 3, and an edge is stored as `face × 3 + edge` (**measured**: across every pairing in every WOK, the edge named runs between the same two positions, in the opposite direction). Every value is −1 or below 3 × the adjacency count, so adjacency never names an unwalkable face (**measured**).

The pairing is mutual (if edge _a_ names _b_, _b_ names _a_) in 968 of the 969 WOKs with adjacency. In `m38aa_03.wok` four walkable faces meet along one edge, from (79.5, 72.375, 0.75) to (79.5, 77.625, 0.75), and face 25's edge 1 names face 27's edge 0, which names face 34's (**measured**).

No PWK or DWK has adjacency, including the 2 PWKs and 14 DWKs with walkable faces (**measured**).

### Perimeter edges and perimeters

The perimeter edges are exactly the walkable faces' edges with −1 in the adjacency, each once (**measured**, every WOK). Each is a `u32` edge (`face × 3 + edge`) and an `i32` transition.

The perimeters array cuts the edges into loops: each entry is the index one past its loop's last edge, the first loop starting at 0, and the last entry equals the perimeter edge count (**measured**). The entries increase strictly. Each loop is closed and in order: each edge ends at the position the next starts at, and the last ends where the first starts (**measured**, all 1,789 loops). The 969 WOKs with adjacency hold 39,366 perimeter edges.

The transition is −1 (35,440 edges) or the index, in the area's LYT room list, of the room the edge leads into (3,926 edges) (**measured**: in `m01aa`, `M01aa_01a`'s transitions are all 7, the layout's eighth room `M01aa_02a`, whose own are 6 and 8, `M01aa_03a` and `M01aa_01a`). Over the 124 layouts, 1,922 room-to-room transitions are found; 1,890 lead to a room whose WOK has a transition back; the other 32 are all in cutscene layouts (`stunt_*`), which place rooms from another area whose WOKs number that area's rooms (31 lead to a room without a WOK in the cutscene's layout and one, in `stunt_unkhall`, past its last room) (**measured**).

### AABB tree

44 bytes a node:

| Offset | Type | Field |
| --- | --- | --- |
| 0 | `f32[3]` | box minimum |
| 12 | `f32[3]` | box maximum |
| 24 | `i32` | face, at a leaf; −1 at an inner node |
| 28 | `u32` | unknown: 4 in all 378,136 vanilla nodes |
| 32 | `u32` | plane: at an inner node 1, 2, 4, 8, 16 or 32; 0 at a leaf |
| 36 | `i32` | left child's index; −1 at a leaf |
| 40 | `i32` | right child's index; −1 at a leaf |

Torlack's binary MDL notes (xoreos-docs, `torlack/binmdl.html`) describe NWN's AABB entry with the same box, children and leaf face fields and a "most significant plane" field of the same six values, with the children as pointers; in a walkmesh they are indices into the node array (**measured**).

The tree is in pre-order: node 0 is the root, and an inner node's left child is the next node (**measured**, all 188,578 inner nodes). Every WOK with faces has a tree (980) and no PWK or DWK has one. An inner node's box is exactly its children's boxes together (**measured**). A leaf's box is its face's extent grown on each side by between 0.005 and 0.02 for 189,168 of the 189,558 leaves, by 0.01 within 0.0005 for 173,240 of them; 303 more hold their face with other margins, and 87 leaves in 24 WOKs (`m02aa_09a`, `m02af_01a`, `m14ab_02a`, `m38aa_03`, `m39aa_05`, among others the census lists) have a box that misses part of the face they name (**measured**). A runtime that relies on the tree to find every face a ray or point may touch builds its own for those rooms.

The tree has one leaf per face, so 2 × faces − 1 nodes, in 979 of the 980 trees. `m02af_01a.wok` has 140 faces and 219 nodes: the tree holds faces 0 to 109, and faces 110 to 139, all Nonwalk, have no leaf (**measured**).

**The plane.** At an inner node the field names an axis and a direction, 1 +X, 2 +Y, 4 +Z, 8 −X, 16 −Y, 32 −Z: the centre of the right child's box lies that way from the centre of the left's in 183,627 of the 188,578 inner nodes, level with it in 4,950, and the other way in 1 (**measured**). Torlack's notes name the field "most significant plane". How the tree's builder chose the axis is not established; it is the axis of the node box's longest side in 125,793 of the 188,578 inner nodes only (**measured**).

## The Endar Spire's first area

Module `end_m01aa`, area `m01aa`: its 15 rooms' WOKs all read, and 14 have faces; `m01aa_07b.wok` is a bare header (**measured**). Every non-empty room has its AABB tree, 2 × faces − 1 nodes, and one loop of perimeter at least.

| Room | Vertices | Faces | Walkable | AABB nodes | Perimeter edges | Loops | Transitions to rooms |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `m01aa_01a` | 114 | 195 | 79 | 389 | 73 | 3 | 7 |
| `m01aa_02a` | 38 | 60 | 22 | 119 | 24 | 1 | 6, 8 |
| `m01aa_03a` | 68 | 105 | 25 | 209 | 27 | 1 | 5, 7 |
| `m01aa_04a` | 115 | 193 | 99 | 385 | 71 | 5 | 6, 14 |
| `m01aa_05a` | 42 | 56 | 12 | 111 | 14 | 1 | 4, 13, 14 |
| `m01aa_06a` | 63 | 96 | 23 | 191 | 25 | 1 | 1, 4 |
| `m01aa_06b` | 16 | 18 | 4 | 35 | 6 | 1 | 2, 11 |
| `m01aa_07a` | 180 | 320 | 142 | 639 | 136 | 5 | 0, 2 |
| `m01aa_07b` | 0 | 0 | 0 | 0 | 0 | 0 | |
| `m01aa_08a` | 70 | 109 | 60 | 217 | 36 | 2 | 0, 3, 12 |
| `m01aa_08b` | 33 | 56 | 37 | 111 | 7 | 1 | 9 |
| `m01aa_08c` | 29 | 39 | 11 | 77 | 13 | 1 | 1, 9 |
| `m01aa_08d` | 4 | 2 | 2 | 3 | 4 | 1 | 9 |
| `m01aa_09` | 37 | 56 | 11 | 111 | 13 | 1 | 11 |
| `m01aa_10` | 41 | 60 | 13 | 119 | 15 | 1 | 5, 11 |

## What could not be established

- What a WOK's position (offset 60) is, and what the engine does with a PWK's or DWK's points.
- The field at offset 108, and the AABB node field at 28 (always 4).
- Which door state each of a door's three DWKs is.
- How the engine treats the 87 AABB leaves whose box misses their face, the 30 faces of `m02af_01a.wok` with no leaf, and the edge `m38aa_03.wok` shares among four faces.
- Whether the engine reads a PWK's normals and plane distances at all, since every vanilla PWK stores none.
- KotOR 2's walkmeshes, which were not measured.

## Provenance

No public document describes the binary walkmesh. Torlack's binary MDL notes (xoreos-docs, `specs/torlack/binmdl.html`) describe NWN's AABB entry inside a model, which named the node's fields to look for; `gmax_nwn_mdl_0.3b.ms` (xoreos-docs) names the ASCII `aabb` node and the surface material list, and was read for those names only. Everything else here was measured from the vanilla KotOR 1 files, `surfacemat.2da` and the area layouts of the same install; no other tool was read or run.
