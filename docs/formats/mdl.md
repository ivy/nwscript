# MDL and MDX

A KotOR model is two resources of the same resref: the MDL (resource type 2002), which holds the model's header, its tree of nodes and everything about them but the per-vertex data of its meshes, and the MDX (resource type 3008), which holds that per-vertex data, one interleaved block per mesh. KotOR 1 ships 3,072 binary models: 2,445 in `data/models.bif`, 305 in `items.bif`, 63 in `player.bif`, 19 in `party.bif`, 168 in `rims/global.rim`, 40 in `miniglobal.rim`, 30 in `chargen.rim`, one in `mainmenu.rim` and one in `patch.erf`. A RIM's MDX lives in its `dx` partner (`globaldx.rim` beside `global.rim`); a BIF or ERF holds both. Every MDL has its MDX. No module capsule holds a model.

This document covers what a renderer needs to draw and animate a model: the headers, the node tree, the trimesh part of every mesh node, controllers and their keys, skin nodes, animations, and supermodels. Dangly constraints, AABB trees, emitters, lights and saber blades are located but not decoded beyond their controllers.

All integers are little-endian; floats are IEEE 754 single precision. Coordinates are metres, Z up.

## File header

| Offset | Size | Content |
| --- | --- | --- |
| 0x00 | 4 | Zero in every binary model. An ASCII model begins with text, so a non-zero word marks a file that is not a binary model. |
| 0x04 | 4 | The model data's length: the file's length less 12, in all 3,072. |
| 0x08 | 4 | The MDX's length, in all 3,072. |

The model data follows at 0x0C. Every offset stored in the model data counts from its start (file offset less 12); every offset into the MDX counts from the MDX's first byte. An array in the model data is three u32s: the offset of its first element, the number used and the number allocated, equal in every array of every vanilla file.

## Model header

The model data begins with a header of 0xC4 bytes. Offsets below are from the start of the model data.

| Offset | Size | Content |
| --- | --- | --- |
| 0x00 | 8 | Two u32s, 0x00413670 and 0x00405520 in every file (the torlack document's routine addresses). |
| 0x08 | 32 | The model's name, ASCII, NUL-ended. Equals the resref but for case in all models but `dor_lma022`, named `DOR_LMA02`. |
| 0x28 | 4 | Offset of the root node. |
| 0x2C | 4 | Node count. Equal to the number of nodes the tree reaches in every model without a supermodel; larger in 406 of the 413 with one. |
| 0x30 | 24 | Two arrays, all zero. |
| 0x48 | 4 | Zero. |
| 0x4C | 1 | 2 in every model (an animation's header has 5 here). |
| 0x4D | 3 | Not zero in 2,364 models; uninitialised padding. |
| 0x50 | 1 | Classification (below). |
| 0x51 | 1 | 0 in 2,777 models, 4 in 214 (placeables), 2 in 81; meaning not established. |
| 0x52 | 1 | Zero. |
| 0x53 | 1 | 1 in 2,983 models, 0 in 89 (83 room models, four `stuntroom` models, `skyboxbase` and `rep_soldier`). The torlack NWN document puts the fog flag in this byte of its layout; the KotOR files agree with that reading but do not prove it. |
| 0x54 | 4 | Zero. |
| 0x58 | 12 | Array of offsets of the animation headers; 1,209 models have animations. |
| 0x64 | 4 | Zero in 2,666 models; not zero (a value that is no offset into the file) in exactly the 406 with a supermodel and a larger node count. |
| 0x68 | 24 | Bounding box, min x, y, z then max x, y, z. The default (-5, -5, -1) to (5, 5, 10) in every model but `gidy_intlight` (±99999). It is not the model's extent; a reader computes that from the meshes. |
| 0x80 | 4 | Radius: 7.0 (the default) in 2,405 models, other values in 667. |
| 0x84 | 4 | Animation scale: 1.0 in 2,905 models. |
| 0x88 | 32 | Supermodel name, ASCII, NUL-ended; `NULL` when there is none (2,659 models). |
| 0xA8 | 4 | Offset of a node: the root in 2,936 models, the `neck_g` node in the 136 head models. |
| 0xAC | 4 | Zero. |
| 0xB0 | 4 | The MDX's length again, in all 3,072. |
| 0xB4 | 4 | Zero. |
| 0xB8 | 12 | The name array. |

### Classification

| Value | Models | What they are |
| --- | --- | --- |
| 0 | 1,306 | Rooms (1,270 of them named like rooms, `mNNxx_…`), and models of no class. |
| 1 | 561 | Effects (`v_*`, `fx_*`), and 224 models named like rooms, mostly minigame pieces (`m03mg_mgt02`) and the Endar Spire's `m01aa_07b`, a view of space. |
| 2 | 23 | Force-power effects (`v_fpush_*`, `v_fwave_*`); the torlack document's "tile". |
| 4 | 679 | Creatures, heads and bodies (`c_*`, `n_*`, `p*`), cutscene actors and cameras, some items. |
| 8 | 56 | Doors, every one a `dor_*` model. |
| 16 | 23 | Lightsabers, every `w_*sbr_*` model. |
| 32 | 421 | Placeables (`plc_*`), item and weapon models. |
| 64 | 3 | `c_brith`, `m03mg_camera`, `m45mg_camera`; not established. |

### Names

The name array's elements are u32 offsets, one per name, and the names follow the array back to back, each ASCII and NUL-ended, in every file. A node's number (below) indexes this array. The array may hold more names than the tree has nodes (162 models, mostly doors and characters): the extra names belong to nodes only animations have. Every name in vanilla is ASCII and under 32 bytes.

## Nodes

The tree is reached from the root offset at 0x28: each node lists its children's offsets, and every node of a vanilla model is reached exactly once. A node is an 80-byte header, then, by its flags, the headers of what it is.

| Offset | Size | Content |
| --- | --- | --- |
| 0x00 | 2 | Flags (below). |
| 0x02 | 2 | Equal to the node number in every model without a supermodel; it differs in 20,349 nodes, all in models with one. The torlack document's part numbers suggest the supermodel's number for the node; that is not checked here. |
| 0x04 | 2 | Node number: the index of the node's name. Distinct within each model. |
| 0x06 | 2 | Zero. |
| 0x08 | 4 | Zero. |
| 0x0C | 4 | Offset of the parent node; zero for the root. Correct in every node. |
| 0x10 | 12 | Position x, y, z, relative to the parent. |
| 0x1C | 16 | Orientation as a quaternion `w, x, y, z`, relative to the parent. A unit quaternion in every node. |
| 0x2C | 12 | Array of the children's offsets. |
| 0x38 | 12 | Array of controllers, 16 bytes each. |
| 0x44 | 12 | Array of controller data, floats. |

The position and orientation repeat the node's position (type 8) and orientation (type 20) controllers: in all 114,874 nodes with each, the position controller's value equals the header's, and the orientation controller's `x, y, z, w` equals the header's `w, x, y, z` reordered (in 129 the negation, the same rotation, to within 0.00001). The 3,057 nodes with neither, roots mostly, have position zero and the identity orientation. Controllers are described under [Controllers](#controllers).

### Flags

The flags word names the structures after the node header. The values in vanilla, with node counts across all models:

| Flags | Kind | Nodes | After the node header |
| --- | --- | --- | --- |
| 0x001 | dummy | 22,457 | nothing |
| 0x003 | light | 5,599 | a light header |
| 0x005 | emitter | 9,442 | an emitter header |
| 0x011 | reference | 2,012 | a 32-byte model name and a u32 (1 in 65) |
| 0x021 | trimesh | 73,577 | the mesh header |
| 0x061 | skin | 1,033 | the mesh header, then a skin header |
| 0x121 | dangly | 2,641 | the mesh header, then a dangly header |
| 0x221 | AABB | 1,102 | the mesh header, then a u32 offset of the AABB tree's root |
| 0x821 | saber | 68 | the mesh header, then a saber header |

Bit 0x020 marks the mesh header: 78,421 nodes have one. The NWN animmesh (0x0A1) does not occur. Every model's root is a dummy named as the model.

## Mesh header

The mesh header is 332 bytes, following the node header. That length is confirmed by what follows it: the dangly header's first array counts exactly the mesh's vertices in all 2,641 dangly nodes, and the AABB node's root offset lies inside the file in all 1,102. Offsets below are from the start of the mesh header.

| Offset | Size | Content |
| --- | --- | --- |
| 0 | 8 | Two u32s, constant for each kind: 0x00405750 and 0x00405760 for trimesh, AABB and saber nodes, 0x00405740 and 0x00405730 for dangly, 0x00405710 and 0x00405720 for skin. |
| 8 | 12 | Array of faces, 32 bytes each. |
| 20 | 24 | Bounding box in the node's coordinates. It contains every vertex of the mesh in all meshes, and fits them exactly in 38,581 of 78,406 with vertices. |
| 44 | 4 | Radius. |
| 48 | 12 | Average of the vertices (to 0.01 in 40,609). |
| 60 | 12 | Diffuse colour. |
| 72 | 12 | Ambient colour. |
| 84 | 4 | Transparency hint: 0 in 77,741 meshes; 1 (486), 7 (111), 2 (40), 3 (21), 5 (14), 4 (6), 8 and 13 (one each). Taken as the transparency hint because the first twenty of the 680 non-zero meshes read are window, glass, dust, star-field, drip and planet meshes (textures `LTS_glass01`, `LHR_dust01`, `LSP_stars02`, `LHR_planet`, `LTS_drip01`). |
| 88 | 32 | Texture name, ASCII, NUL-ended; bytes after the NUL are often not zero. `NULL` (19,640 meshes) when there is none. |
| 120 | 32 | Second texture name: on every lightmapped mesh, the lightmap (`m01aa_01a_lm0`); empty (36,866) when there is none. |
| 152 | 24 | Zero. |
| 176 | 12 | Array of index counts: one u32 in every mesh with faces, equal to three times the face count. |
| 188 | 12 | Array of index-list offsets: one u32, the offset in the model data of a u16 triangle list. |
| 200 | 12 | Array of one u32, non-zero in all but 178 meshes; not established. |
| 212 | 4 | -1 in 78,406 meshes. |
| 216 | 4 | -1. |
| 220 | 4 | Zero. |
| 224 | 4 | 3 in 78,339 meshes. |
| 228 | 4 | Zero in 78,353 meshes. |
| 232 | 4 | Zero, or 1 in 214 meshes. |
| 236 | 16 | Four floats of various values; not established. |
| 252 | 4 | MDX stride: the bytes per vertex in the mesh's MDX block. |
| 256 | 4 | MDX flags: the attributes each vertex of the block holds (below). |
| 260 | 4 | Offset of the position within a vertex, or -1. |
| 264 | 4 | Offset of the normal, or -1. |
| 268 | 4 | -1 in every mesh (the NWN layout's vertex colours). |
| 272 | 4 | Offset of the first texture coordinates (the texture's), or -1. |
| 276 | 4 | Offset of the second texture coordinates (the lightmap's), or -1. |
| 280 | 8 | -1, -1: the third and fourth texture coordinates, never present. |
| 288 | 4 | Offset of the tangent space, nine floats, or -1. |
| 292 | 12 | -1. |
| 304 | 2 | Vertex count. |
| 306 | 2 | Texture count: 2 on 41,541 of the 41,556 lightmapped meshes, else 1 or 0; it does not always agree with the names (5,519 meshes count 1 and have no texture name). |
| 308 | 1 | Lightmapped: 1 exactly when the mesh has a second texture and second texture coordinates, in all meshes but one (which has neither). |
| 309 | 2 | Zero. |
| 311 | 1 | Shadow: 1 in 11,991 meshes, nearly all on characters' meshes (10,954) and none on an AABB node. |
| 312 | 1 | Zero. |
| 313 | 1 | Render: 1 in 59,294 meshes, 0 in 19,127, including 822 of the 1,102 AABB nodes (walkmeshes). |
| 314 | 2 | Various values; not established. |
| 316 | 4 | A float, often near 0.00015; not established. |
| 320 | 4 | Zero. |
| 324 | 4 | Offset of the mesh's block in the MDX. |
| 328 | 4 | Offset in the model data of the positions again, three floats per vertex. |

What the flag bytes 308, 311 and 313 are rests on their correlations above and on the position of the kotor_mdl document's shadow and render flags (u16s of 256 at 310 and 312, so the 1 in their high bytes); the render flag reads as the torlack document says NWN's defaults (off for AABB meshes, on otherwise).

### Faces

A face is 32 bytes: the plane normal (three floats; a unit vector in all but 816 of 5,779,369 faces), the plane distance (a float), a u32 surface id, three i16 adjacent face indices (-1 for none), and three u16 vertex indices. Every vertex index of every vanilla face is less than the mesh's vertex count.

### Triangle list

The u16 list the index-offset array points to holds three times the face count indices, and in all 78,338 meshes that have one it is the faces' vertex indices in face order. It lies in the model data (the torlack document places NWN's in the raw data). The 83 meshes without the arrays are the 68 saber nodes and 15 trimeshes with no vertices.

## Skin header

A skin node's mesh header is followed by a 100-byte skin header. That length is confirmed by the bone map, which begins exactly 100 bytes after the mesh header ends in every one of the 1,033 skin nodes. Offsets below are from the start of the skin header.

| Offset | Size | Content |
| --- | --- | --- |
| 0 | 12 | An array, empty in every skin. |
| 12 | 4 | Offset within each MDX vertex of the four bone weights: 32 in 957 skins (stride 64), 68 in 76 (stride 100). |
| 16 | 4 | Offset within each MDX vertex of the four bone indices: the weights' offset plus 16. |
| 20 | 4 | Offset of the bone map. |
| 24 | 4 | Its count: the model's name count in all skins but one. |
| 28 | 12 | Array of qbones: one quaternion `w, x, y, z` per bone map entry. |
| 40 | 12 | Array of tbones: three floats per bone map entry. |
| 52 | 12 | Array of one u32 per bone map entry; values like memory addresses, not zero in any skin; not established. |
| 64 | 34 | Seventeen u16s: the node numbers of the skin's bones by local index, as far as the skin has bones; after them, bytes of no evident meaning. |
| 98 | 2 | Not established. |

The bone map holds one float per node number: the node's local bone index, or -1 when the node is no bone of this skin. In every skin the indices that occur are 0 to one less than the bone count, each once, and the u16s at 64 are the node numbers in local-index order (as many as the skin has bones, up to 17). Skins have 1 to 17 bones: 205 have 16, 247 have 14, 22 have the most.

A vertex's four bone indices are floats in the MDX, each a local bone index or -1 for an empty slot; its four weights sum to one in every vertex of every skin, and every index that is not -1 names a bone of the bone map.

The qbone and tbone of a bone are the bind transform: the rotation and then the translation that take the skin node's coordinates to the bone's at the rest pose the geometry stores. With `(Qb, Tb)` the bone's transform to the model's coordinates and `(Qs, Ts)` the skin node's, the qbone is `Qb⁻¹·Qs` and the tbone `Qb⁻¹(Ts − Tb)`, to 0.001, in 11,848 of the 11,955 bones of all skins. The other 107: the 75 of `mainmenu_model`, whose rest pose is posed away from its bind; and eight bones in each of `pfbcl`, `pfbcm` and `pfbcs` (the mesh `LArm`) and `p_bastilabb` (the mesh `torso`), five of each eight holding the transform that fits another bone of the same mesh.

## MDX

A mesh's MDX block is `vertex count × stride` bytes at the offset at 324, one vertex after another, each vertex the attributes the flags name at the offsets the header gives. After each block comes one more vertex's worth of bytes whose first float is 10,000,000 (1,000,000 after each of the 1,033 skin meshes' blocks), a terminator. The positions in the block equal the positions at offset 328 of the mesh header in all 78,338 meshes with a block.

| Bit | Attribute | Floats |
| --- | --- | --- |
| 0x01 | position | 3 |
| 0x02 | first texture coordinates | 2 |
| 0x04 | second texture coordinates | 2 |
| 0x20 | normal | 3 |
| 0x80 | tangent space | 9 |

The bits and the offsets agree in every mesh: an offset is -1 exactly when its bit is clear. The layouts in vanilla, by flags and stride, with mesh counts:

| Flags | Stride | Attributes, at offsets | Meshes |
| --- | --- | --- | --- |
| 0x21 | 24 | position 0, normal 12 | 14,392 |
| 0x23 | 32 | position 0, normal 12, UV1 24 | 21,049 |
| 0x23 | 64 | the same, and 32 bytes of skin data (skin meshes) | 957 |
| 0x25 | 32 | position 0, normal 12, UV2 24 | 14 |
| 0x27 | 40 | position 0, normal 12, UV1 24, UV2 32 | 37,012 |
| 0xA1 | 60 | position 0, normal 12, tangent space 24 | 23 |
| 0xA3 | 68 | position 0, normal 12, UV1 24, tangent space 32 | 286 |
| 0xA3 | 100 | the same, and 32 bytes of skin data (skin meshes) | 76 |
| 0xA7 | 76 | position 0, normal 12, UV1 24, UV2 32, tangent space 40 | 4,529 |
| 0 | 0 or -1 | none: the 68 saber nodes (stride 0), whose positions are only those at offset 328, with further arrays in the saber header, and 15 meshes with no vertices (stride -1) | 83 |

Every skin mesh's stride exceeds its attributes by 32 bytes: the four weights and then the four bone indices, at the offsets the skin header gives (above).

## Controllers

A node's controllers are two arrays of the node header: at 0x38 the controller records, 16 bytes each, and at 0x44 the controller data, floats. A controller record:

| Offset | Size | Content |
| --- | --- | --- |
| 0 | 4 | Type (below). |
| 4 | 2 | 0xFFFF in all 838,334 controllers of the models' geometry; in an animation, 16 on every position controller and 28 on every orientation controller (the offsets of those fields in the node header), 0xFFFF on the rest. |
| 6 | 2 | Row count: the number of keys. |
| 8 | 2 | Index of the first time key, counted in floats of the data array. |
| 10 | 2 | Index of the first value, counted in floats of the data array. |
| 12 | 1 | Column count: the floats of one value, with 0x10 set when the controller is bezier keyed. |
| 13 | 3 | Padding; not zero in most. |

The time keys are `rows` consecutive floats; the values follow them directly in every controller of every model. A row of values holds as many floats as the columns say, except in two cases: a bezier-keyed controller (columns with 0x10) holds three values' worth per row, the value and then two control values (taken as the value because it is the triple that equals the node's rest position most often and moves least from row to row), and an orientation controller with two columns holds one 32-bit word per row, a compressed quaternion (below). In a model's geometry every controller has one row at time zero; no key of any animation lies past its animation's length.

The types are those the torlack document lists for NWN, meaning by the kind of node that carries them (88 is a light's radius and an emitter's birth rate). By where they occur, with counts across all models:

| Type | Columns | Geometry | Animations | What |
| --- | --- | --- | --- | --- |
| 8 | 3 | 114,874 | 95,343 | position |
| 8 | 19 | | 1,441 | position, bezier |
| 20 | 4 | 114,874 | 101,755 | orientation, `x, y, z, w` |
| 20 | 2 | | 104,083 | orientation, compressed |
| 36 | 1 | 75,056 | 751 | scale |
| 100 | 3 | 74,969 | 459 | a mesh's self-illumination colour |
| 100 | 19 | | 51 | the same, bezier |
| 132 | 1 | 84,410 | 1,395 | on meshes; on emitters one of their own |
| others | 1, 3 or 19 | on lights and emitters | 7,652 in all | lights' and emitters' own |

### Compressed orientation

An orientation controller of two columns stores each key's rotation in one u32: `x` in bits 0 to 10, `y` in bits 11 to 21, `z` in bits 22 to 31, each read as `field / max × 2 − 1` with `max` 2047 for the 11-bit fields and 1023 for `z`; `w` is not stored, and is the non-negative `√(1 − x² − y² − z²)`. There are 581,376 such keys in 104,083 controllers, all in animations.

The files establish it four ways:

- The partition. Taking three fields of 8 to 13 bits from the low bits of every key of 3,000 controllers chosen at random, and reading them as above, the widths 11, 11, 10 put `x² + y² + z²` within the unit sphere for 99.7% of keys and give the smallest steps between consecutive keys; the next best partition (12, 10, 10) puts 91.6% within it, with steps nearly eight times as large.
- Uncompressed copies. Where another model holds an animation of the same name with a node of the same name, the same key times and four-column keys, the decoded compressed keys agree with the stored `x, y, z, w` (or their negation, as `w` is never negative when decoded): in the 2,980 such controllers that agree to 0.01 (17,758 keys), the median difference is 0.00049 in `x` and `y` and 0.00098 in `z`, one step of each field's scale.
- The scale. Against those copies the stored field is `⌊(v + 1) / 2 × max⌋` for nearly all components, the difference between the exact and the stored field falling between 0 and 1.25; with `max` taken as 2048 and 1024 instead it spreads from 0 to 2.25.
- The rest pose. The first key of 38,923 of the 104,068 compressed controllers whose node the model has agrees to 0.003 with that node's rest orientation, as an animation that begins at rest would.

Decoded, 3,573 keys have `x² + y² + z²` slightly over one (at most 1.0024), within a step of the fields; `w` is zero for them.

## Animations

The model header's array at 0x58 points to the animation headers: 7,143 animations in 1,209 models. An animation header is 0x88 bytes:

| Offset | Size | Content |
| --- | --- | --- |
| 0x00 | 8 | Two u32s, 0x004134F0 and 0x0043ECE0. |
| 0x08 | 32 | The animation's name. |
| 0x28 | 4 | Offset of the animation's root node. |
| 0x2C | 4 | A node count; not the number of nodes the animation's tree reaches in most. |
| 0x30 | 28 | Zero, but for 5 at 0x4C and the three bytes after it (uninitialised, as in the model header). |
| 0x50 | 4 | Length in seconds. |
| 0x54 | 4 | Transition time in seconds. |
| 0x58 | 32 | The animation root's name: the model's own name in 7,077 of the 7,143. |
| 0x78 | 12 | Array of events, 36 bytes each: a float time, then a 32-byte name. |
| 0x84 | 4 | Zero. |

An animation's nodes are a tree like the geometry's, reached from its root, with the same 80-byte node header; every one of the 232,309 is a dummy (flags 0x001) whose number indexes the model's own name array, and whose controllers hold the keys. The animation's root node is named as the model in all 7,143; 146 nodes name no node of their own model's geometry. There are 3,884 events of 21 names, `snd_footstep` (1,228) and `detonate` (1,050) the most.

An animation's position keys do not read as the node's position: of the 92,996 position controllers in animations whose node rests away from the origin (by more than 0.01 on an axis), the first key lies nearer the origin than the rest position in 90,351, as offsets from the rest position would. Its orientation keys do read as the orientation itself (above).

## Supermodels

A model names its supermodel at 0x88 of the model header; 413 do. The chains, followed through the install's resolution of each name, pass up to seven supermodels (`s_female02` names `S_Female01`, which names `S_Male02`, which names `S_Male01`, which names none). Three models name a supermodel the game does not ship: `h_m_hi01` (`CSpar2`), `n_djedi_h` and `p_carthbbh` (both `S_Male02H`). A supermodel holds animations its descendants play: `s_male02` holds 166, `walk` among them, and a body such as `pmbam`, whose chain is `S_Female02`, `S_Female01`, `S_Male02`, `S_Male01`, has none of its own. An animation's nodes act on the playing model's nodes of the same names; a body model lacks the head's bones, which its head model has.

## In the Endar Spire

The first area of the demo module, `end_m01aa` (area `m01aa`), draws 15 room models from its layout (`m01aa_01a` to `m01aa_10`, all class 0 but `m01aa_07b`, class 1), two door models through `genericdoors.2da` (`dor_lhr01`, `dor_lhr02`) and 12 placeable models through `placeables.2da` (one of them, `plc_invis`, has no rendered mesh). Two of their meshes name a lightmap the install does not ship: `M01aa_04a_a0002t` in `m01aa_04a` and `DOR_LHR02_a00004` in `dor_lhr02`.

## What the public documents leave out or get wrong

The xoreos-docs `kotor_mdl.html` gives the headers' shape; the files say more, and in places otherwise:

- The names are not only "stored one after the other": an array of offsets to them comes first.
- The controller's u16s at 8 and 10 index floats of the data array; they are not byte offsets. The u16 at 4 is always 0xFFFF.
- The node header's orientation is `w, x, y, z`; the orientation controller's value is `x, y, z, w`.
- In the mesh header, offset 84 (unknown there) is the transparency hint; 152 to 176 is zero; the arrays at 176 and 188 are the index count and the triangle list's offset, not vertex locations; 256 is the MDX flags word; 260 is the position's MDX offset, 0 when present, not "always 0"; the seven words from 276 hold the second texture coordinates' offset and, at 288, the tangent space's; byte 308 is the lightmapped flag. Fields called "always" a value there are that value in most meshes but not all (212, 224, 232).
- The model header's first byte is the classification, and the KotOR values 16 and 32 are not in the torlack NWN list. The bounding box there is the default in all but one model.
- The "names array header" field the document calls the root node's location is a different node in the head models.
- Neither document describes the MDX: the per-mesh blocks, the flags word's bits, or the terminator rows.
- Neither describes compressed orientation keys (two columns, one word per key).
- The controller's u16 at 4 is not always unknown: in animations it is the node header offset of the field the controller drives.
- In the skin header, the words at 12 and 16 are the MDX offsets of the weights and bone indices; the bone map's count follows its offset directly (no allocated count); the arrays at 28 and 40 are the qbones and tbones; the node list at 64 is seventeen u16s, not fifteen; and the bone map's entries are floats, local bone indices by node number, as the example in `kotor_mdl.html` has it.

## What could not be established

The meaning of the model header's byte 0x51, the mesh header's arrays at 200 and fields at 212 to 248 and 314 to 320, and the classification value 64; whether byte 0x53 is the fog flag; the skin header's array at 52 and bytes 98 to 100; which of a bezier key's two control values is which; whether the engine adds an animation's position keys to the rest position or puts them in its place, and whether it uses the stored qbones and tbones or the rest pose's (the files differ in the five models above). Each needs the engine's behaviour, not the files alone.

## Provenance

Public documents read: xoreos-docs `specs/kotor_mdl.html` (KotOR's binary MDL), `specs/torlack/binmdl.html` (NWN's binary MDL: the file header, arrays, node flags, the face record and the mesh header's NWN layout), `specs/gmax_nwn_mdl_0.3b.ms` (the ASCII field names) and `specs/torlack/basics.html` (resource types).

Reference projects read: none.

Measured: on 2026-10-18, every count above comes from the vanilla KotOR 1 install under `$RAKATA_GAME_K1`, first with short Python scripts reading the headers, nodes, mesh headers, faces, index lists, controllers, skin headers, animation headers and MDX blocks of every model the KEY's BIFs, `rims/`, `modules/` and `patch.erf` hold, then with `crates/rakata-mdl/tests/vanilla.rs`, which parses every one of them, prints the node kinds, classifications, MDX layouts, flag counts, controllers by type, compressed keys, skins and their bind transforms, resolves every supermodel chain, and reads every model `end_m01aa` uses (`RAKATA_GAME_K1=… cargo test -p rakata-mdl --test vanilla -- --nocapture`). The compressed orientation's partition, its agreement with uncompressed copies and its scale were measured by the Python scripts over the 2,832 models of distinct names (one copy of each name the BIFs and RIMs hold).

Date: 2026-10-18.

Written by the implementer from measurement; no implementation's source was read.
