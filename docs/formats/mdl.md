# MDL and MDX

A KotOR model is two resources of the same resref: the MDL (resource type 2002), which holds the model's header, its tree of nodes and everything about them but the per-vertex data of its meshes, and the MDX (resource type 3008), which holds that per-vertex data, one interleaved block per mesh. KotOR 1 ships 3,072 binary models: 2,445 in `data/models.bif`, 305 in `items.bif`, 63 in `player.bif`, 19 in `party.bif`, 168 in `rims/global.rim`, 40 in `miniglobal.rim`, 30 in `chargen.rim`, one in `mainmenu.rim` and one in `patch.erf`. A RIM's MDX lives in its `dx` partner (`globaldx.rim` beside `global.rim`); a BIF or ERF holds both. Every MDL has its MDX. No module capsule holds a model.

This document covers what a renderer needs to draw and animate a model: the headers, the node tree, the trimesh part of every mesh node, controllers and their keys, skin nodes, lights, emitters, saber blades, animations, and supermodels. Dangly constraints and AABB trees are located but not decoded. [Layout](#layout) gives the order in which BioWare's compiler lays every structure out, which a writer follows to reproduce the files byte for byte.

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

What the flag bytes 308, 311 and 313 are rests on their correlations above and on the position of the kotor_mdl document's shadow and render flags (u16s of 256 at 310 and 312, so the 1 in their high bytes); the render flag reads as the torlack document says NWN's defaults (off for AABB meshes, on otherwise). The walkmesh an AABB node holds is invisible in the game even when its render flag is 1, though 92 of the 280 such nodes name a texture (and one of the 822 with the flag 0); whether the engine loads that texture is not established.

### Faces

A face is 32 bytes: the plane normal (three floats; a unit vector in all but 816 of 5,779,369 faces), the plane distance (a float), a u32 surface id, three i16 adjacent face indices (-1 for none), and three u16 vertex indices. Every vertex index of every vanilla face is less than the mesh's vertex count. The stored winding is front-facing in a left-handed frame once Y is negated (KotOR's right-handed Z-up to Unreal's left-handed Z-up), so a renderer that negates Y keeps the order as stored and does not reverse it.

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

## Light header

A light node's header of 92 bytes follows the node header. That length is confirmed by what follows it: in all 5,599 lights the first array the light points to, or its child array when it points to none, begins exactly 92 bytes after the node header ends. Offsets below are from the start of the light header.

| Offset | Size | Content |
| --- | --- | --- |
| 0 | 4 | Flare radius, a float: non-zero exactly in the 61 lights with flares (5,000 in 30, 5,625 in 8, 1,000 in 5), zero in the other 5,538. |
| 4 | 12 | An array, four bytes per element; empty in every light. |
| 16 | 12 | Array of flare sizes, a float each. |
| 28 | 12 | Array of flare positions, a float each. |
| 40 | 12 | Array of flare colour shifts, three floats each. |
| 52 | 12 | Array of offsets of the flare textures' names, each NUL-ended. The four flare arrays have one count in every light: 1 in 49, 2 in 8, 4 in one, 6 in three. |
| 64 | 4 | Priority: 5 in 1,723 lights, 4 in 1,671, 3 in 1,621, 2 in 330, 1 in 253; -7 in `plc_endcorps`'s `SuperNova`. |
| 68 | 4 | Ambient only: 1 in 178 lights, 0 in 5,421. Every light named as ambient has it (`gidy_sun_amb`, `gidy_intlight_amb`, `AuroraAmb`, `AuroraAmbient`) and their diffuse partners (`gidy_sun_diff`, `gidy_intlight_diff`) do not. |
| 72 | 4 | 2 in 5,388 lights, 1 in 207, 0 in four (`fx_flame01`'s light, in both its copies, and the two `gidy_intlight` lights); the torlack document's dynamic type. |
| 76 | 4 | 1 in 34 lights, 0 in 5,565; the torlack document's affect-dynamic flag. |
| 80 | 4 | Shadow: 1 in 3,028 lights, 0 in 2,571. Every light named for shadow has it (`AuroraShadow`, `AuroraShadow01`, `AurMainShadw`, `ShadowAurora`), and of the `gidy_sun` pair the diffuse light has it and the ambient not. |
| 84 | 4 | Zero in every light, the 61 with flares too. The torlack document puts the generate-flare flag here. |
| 88 | 4 | 1 in 5,438 lights, 0 in 161; the torlack document's fading flag. |

The words at 64 to 88 are where the torlack document lists NWN's light priority, ambient-only flag, dynamic type, affect-dynamic flag, shadow flag, generate-flare flag and fading flag, in that order, and its light header is 92 bytes too. The ambient and shadow readings rest on the names above; the others rest on position alone. 5,429 of the lights are in class 0 models (rooms).

A light's own controllers are under [Controllers](#controllers): colour (76), radius (88) and multiplier (140) on nearly every light.

## Emitter header

An emitter node's header of 224 bytes follows the node header. That length is confirmed by what follows it: an emitter points to nothing, and in all 9,442 emitters its child array begins exactly 224 bytes after the node header ends. Offsets below are from the start of the emitter header. A name field holds ASCII up to a NUL; the bytes after the NUL are often not zero (in 8,561 update fields, 8,860 render fields, 8,944 blend fields and 8,493 texture fields), uninitialised memory like the model header's bytes at 0x4D; in the chunk field and the field at 187 they are zero.

| Offset | Size | Content |
| --- | --- | --- |
| 0 | 4 | A float, the torlack document's dead space: zero in 9,322 emitters; 12 (43), 10 (37), 0.20944 (28), 1.5708 (8) and 0.087266 (4) in the rest, all `Fountain` emitters but seven `Explosion`. |
| 4 | 4 | Blast radius, a float: non-zero in 11 emitters (5, 1, 0.2), every one an `Explosion`. |
| 8 | 4 | Blast length, a float: 1 in 8 emitters, every one an `Explosion`. |
| 12 | 4 | Zero but in 48 `Lightning` emitters (1 to 5); not established. |
| 16 | 4 | Zero but in 40 `Lightning` emitters (1); not established. |
| 20 | 4 | X grid, as the torlack document names the word at its place: with the Y grid, the cells of the texture's frames (below). 16 in 4,926 emitters, 4 in 2,741, 2 in 691, 1 in 520, 0 in 555, 5 in 6, 8 in 3. |
| 24 | 4 | Y grid. 4 in 2,739, 16 in 2,539, 8 in 2,415, 2 in 688, 1 in 500, 0 in 555, 5 in 6. |
| 28 | 4 | 1 in 74 `Fountain` emitters, else zero; the torlack document's space type. |
| 32 | 32 | Update: `Single` (6,539), `Fountain` (2,171), `Explosion` (655), `Lightning` (77). |
| 64 | 32 | Render: `Billboard_to_Local_Z` (5,099), `Normal` (3,227), `Billboard_to_World_Z` (459), `Motion_Blur` (440), `Linked` (144), `Aligned_to_Particle_Dir` (50), `Aligned_to_World_Z` (23). |
| 96 | 32 | Blend: `Normal` (7,934), `Lighten` (1,485), `Punch-Through` (19), `PunchThrough` (4). |
| 128 | 32 | Texture name: 102 names, at most 16 characters; `NULL` in 60 emitters. |
| 160 | 16 | Chunk name: a model, in 251 emitters (46 names, at most 15 characters); empty in 9,191. Every emitter with a chunk renders `Normal` (248) or `Motion_Blur` (3). |
| 176 | 4 | 1 in 342 emitters, else zero; the torlack document's two-sided texture flag. |
| 180 | 4 | 1 in 8 `Single` emitters, else zero; the torlack document's loop flag. |
| 184 | 2 | Render order: 0 in 7,432 emitters, 1 in 1,764, 2 to 8 in the other 246. |
| 186 | 1 | Zero. |
| 187 | 32 | A name: `NULL` in 4,643 emitters, `fx_depth` in one (`Fx_PuffySand`), empty in 4,798. It is set in exactly the 4,644 emitters that carry the controllers of types 216 to 284 (below), and empty in exactly the others. What it names is not established; nothing in the install is named `fx_depth`. |
| 219 | 1 | Not zero in 1,614 emitters (63, 17, 255 and 191 most); not established. |
| 220 | 4 | Flags (below). |

The torlack document's NWN emitter header is 216 bytes: the same fields, but for two words fewer before the grid (KotOR's 12 and 16), a texture of 64 bytes rather than 32, and two bytes of padding between the render order and the flags where KotOR has 34 (186 to 220).

The update names how particles are born. `Single` emitters have no velocity or mass (types 168 and 124 are zero in every one); the blast fields are only on `Explosion` emitters; the lightning controllers only on `Lightning` emitters, but for `plc_smk01`'s three `Fountain` emitters, and every `Lightning` emitter renders `Linked`.

A frame index (types 108 and 112, below) counts the cells of the X × Y grid. The frame end is less than the grid's cell count in 6,601 emitters; in 2,841 it is not, among them 1,202 of the 4,754 crowd emitters of `m31aa_00a`, `m31ab_00a` and `m31ad_00a`, whose frames run to 207 with a grid of 16 × 8. How the engine draws a frame past the grid is not established.

The flags word's bits, with emitter counts, named as the torlack document names NWN's:

| Bit | Emitters | Torlack's name | What the files say |
| --- | --- | --- | --- |
| 0x0001 | 156 | P2P | Only on `Fountain` emitters; the P2P bezier controllers (128 and 132) are non-zero only on emitters with it. |
| 0x0002 | 9,210 | P2P Sel | On every emitter with a chunk; not established. |
| 0x0004 | 154 | Affected by Wind | Only on `Fountain` emitters. |
| 0x0008 | 719 | Is Tinted | |
| 0x0010 | 518 | Bounce | 509 of the 848 emitters whose bounce coefficient (92) is not zero have it. |
| 0x0020 | 6,699 | Random | |
| 0x0040 | 2,005 | Inherit | |
| 0x0080 | 13 | Inherit Vel | |
| 0x0100 | 6,266 | Inherit Local | |
| 0x0200 | 4 | Splat | Only on `Explosion` emitters. |
| 0x0400 | 55 | Inherit Part | |
| 0x1000 | 9 | none | Only on `Fountain` emitters, all with a bounce coefficient. |

Bits 11 and 13 to 31 are clear in every emitter.

## Saber header

A saber node's mesh header is followed by a saber header of 20 bytes. That length is confirmed by its first array, which begins exactly 20 bytes after the saber header in all 68 saber nodes. The 68 are in the 23 class-16 models (`w_*sbr_*`; 22 names, one of them in two places), every one of which has saber nodes. Offsets below are from the start of the saber header.

| Offset | Size | Content |
| --- | --- | --- |
| 0 | 4 | Offset of the positions: three floats per vertex, equal to the positions at 328 of the mesh header in every saber. |
| 4 | 4 | Offset of the texture coordinates: two floats per vertex, all between 0 and 1. |
| 8 | 4 | Offset of the normals: three floats per vertex, a unit vector in every vertex. |
| 12 | 4 | A word from 86 to 98. |
| 16 | 4 | A word from 86 to 98, different from the word at 12 in every saber. Neither word is a node number of its model; what they are is not established. |

Every saber node has 176 vertices and 12 faces, an MDX stride of 0 and no MDX block (above): the three arrays are its vertices. They form 44 columns of four, each column one vertex per row along the blade's length (Z), the rows rising in every column: in `w_dblsbr_001`'s `plane269` the first column lies at Z −0.043, 0.005, 0.932 and 0.981 along X 0, the second at X 0.101, with texture V running 0 to 1 up each column. In all 68 saber nodes:

- The columns at vertices 0 and 88 are the blade's centre line (texture U 0.5); the column at 4 is its right edge (X about +0.1, U 1) and the column at 92 its left edge (X about −0.1, U 0). The other forty columns repeat the centre line exactly; what the engine does with them is not established.
- Every vertex has the same normal, (0, −1, 0): the blade is flat, in the node's XZ plane.
- The 12 stored faces are the sequential triples of vertices 0 to 32, then (33, 32, 31): they are not the blade's surface, and they leave the left half (U 0 to 0.5) undrawn.

The blade's surface is the two strips between the centre line and each edge, three quads each, twelve triangles: `rakata_mdl::Saber::blade_triangles` gives them, wound as a mesh's faces are (counter-clockwise about the stored normal, which every trimesh face of the saber models' other meshes is, 3,847 of 3,847). That the engine draws these strips is inferred from the vertices, not observed.

## MDX

A mesh's MDX block is `vertex count × stride` bytes at the offset at 324, one vertex after another, each vertex the attributes the flags name at the offsets the header gives. After each block comes one more vertex's worth of bytes whose first float is 10,000,000 (1,000,000 after each of the 1,033 skin meshes' blocks), a terminator. The positions in the block equal the positions at offset 328 of the mesh header in all 78,338 meshes with a block.

| Bit | Attribute | Floats |
| --- | --- | --- |
| 0x01 | position | 3 |
| 0x02 | first texture coordinates | 2 |
| 0x04 | second texture coordinates | 2 |
| 0x20 | normal | 3 |
| 0x80 | tangent space | 9 |

The bits and the offsets agree in every mesh: an offset is -1 exactly when its bit is clear. A skin mesh's first texture coordinates run the other way in V from a trimesh's: drawn with a texture uploaded top row first, a trimesh's V is flipped and a skin's is used as stored, which draws `pmhc01`'s face upright where the trimesh's flip inverts it (observed in a renderer; why the two kinds differ is not established). The layouts in vanilla, by flags and stride, with mesh counts:

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

A type's meaning depends on the kind of node that carries it (88 is a light's radius and an emitter's birth rate). The types every node and every mesh carries, by where they occur, with counts across all models:

| Type | Columns | Geometry | Animations | What |
| --- | --- | --- | --- | --- |
| 8 | 3 | 114,874 | 95,343 | position |
| 8 | 19 | | 1,441 | position, bezier |
| 20 | 4 | 114,874 | 101,755 | orientation, `x, y, z, w` |
| 20 | 2 | | 104,083 | orientation, compressed |
| 36 | 1 | 75,056 | 751 | scale |
| 100 | 3 | 74,969 | 459 | a mesh's self-illumination colour |
| 100 | 19 | | 51 | the same, bezier |
| 132 | 1 | 74,968 | 1,395 | a mesh's alpha: 1 in 74,156 meshes; less in water (`FountainWater03` 0.7, `puddle1` 0.2), beams, ice and hologram screens (`plc_btlscreen01` 0.5), and keyed to fade glows in animations (`c_drdsentry`'s `eye_glow` in `cdead`); from -0.4 to 6.01 in all. The torlack document numbers NWN's mesh alpha 128 |

The counts are of the controllers on every kind of node; a light's or an emitter's types 100 and 132 are its own (below), and not counted here. 36 is on meshes and on 88 dummies; 100 and 132 on meshes, the sabers among them.

The torlack document's NWN numbers hold for lights unchanged. On lights, with counts across all models:

| Type | Columns | Geometry | Animations | What |
| --- | --- | --- | --- | --- |
| 76 | 3 | 5,599 | 402 | colour; components from -1 (the negative lights `AuroraNegLight*`) to 1 |
| 88 | 1 | 5,599 | 33 | radius, 0 to 140,000 |
| 96 | 1 | 4 | | shadow radius: two on each copy of `fx_flame01`'s light, 5 and 15 |
| 100 | 1 | 2 | | vertical displacement: `fx_flame01`'s light |
| 140 | 1 | 5,589 | | multiplier, 0 to 50; on every light but `fx_flame01`'s and the `gidy_*` lights |

An emitter's types are NWN's, but that the two colours NWN numbers 96 and 108 (twelve bytes each) are elsewhere and every NWN type after them is 24 less: NWN's frame end (132) is KotOR's 108, its lightning scale (216) KotOR's 192. The files bear the reading out: the frame start (112) is at most the frame end (108) in 9,441 of 9,442 emitters; the lightning types (184 to 192) are non-zero only on `Lightning` emitters but for `plc_smk01`'s three; the P2P bezier types (128, 132) only on emitters with the P2P flag; velocity and mass never on a `Single` emitter; the spread (160) holds angles in radians (6.2832, 2.0944, 0.3491 most). Types 196 to 284 are not in the torlack list as numbered here; they are on the 4,644 emitters whose name at 187 is set (above), 4,564 to 4,644 of them by type. On emitters, with counts across all models:

| Type | Columns | Geometry | Animations | What |
| --- | --- | --- | --- | --- |
| 80 | 1 | 9,442 | 407 | alpha at the end of a particle's life, 0 to 1 |
| 84 | 1 | 9,442 | 433 | alpha at the start, 0 to 1 |
| 88 | 1 | 9,442 | 2,715 | birth rate |
| 92 | 1 | 9,442 | | bounce coefficient |
| 96 | 1 | 9,442 | | combine time; non-zero in one emitter |
| 100 | 1 | 9,442 | | drag |
| 104 | 1 | 9,442 | | frames per second, 0 to 32 |
| 108 | 1 | 9,442 | | frame end, an integer |
| 112 | 1 | 9,442 | | frame start, an integer |
| 116 | 1 | 9,442 | | gravity |
| 120 | 1 | 9,442 | 351 | life expectancy in seconds; -1 in 6,507 |
| 124 | 1 | 9,442 | 59 | mass |
| 128 | 1 | 9,442 | | P2P bezier 2 |
| 132 | 1 | 9,442 | | P2P bezier 3 |
| 136 | 1 | 9,442 | | particle rotation |
| 140 | 1 | 9,442 | 3 | random velocity |
| 144 | 1 | 9,442 | 94 | size at the start |
| 148 | 1 | 9,442 | 37 | size at the end |
| 152 | 1 | 9,442 | | Y size at the start |
| 156 | 1 | 9,442 | | Y size at the end |
| 160 | 1 | 9,442 | 12 | spread, radians |
| 164 | 1 | 9,442 | | threshold |
| 168 | 1 | 9,442 | 117 | velocity |
| 172 | 1 | 9,442 | 36 | X size |
| 176 | 1 | 9,442 | 60 | Y size |
| 180 | 1 | 9,442 | | blur length; 10 in 8,812 |
| 184 | 1 | 9,442 | | lightning delay |
| 188 | 1 | 9,442 | | lightning radius |
| 192 | 1 | 9,442 | | lightning scale |
| 196 | 1 | 4,642 | | 0 to 8; non-zero on 77 `Lightning` emitters and 7 `Fountain`; not established |
| 200 | 1 | 4,564 | | 0 to 25; non-zero only on 60 `Lightning` emitters; not established |
| 216 | 1 | 4,644 | 526 | alpha at mid-life: between the start and end alphas in 1,072 of the 1,178 emitters where they differ |
| 220 | 1 | 4,644 | | percent of life at the start; 0 but in 13 (1) |
| 224 | 1 | 4,644 | 205 | percent of life at mid-life, 0 to 1: at least the start's and at most the end's in every emitter |
| 228 | 1 | 4,644 | | percent of life at the end; 1 in 4,635 |
| 232 | 1 | 4,644 | 300 | size at mid-life: between the start and end sizes in 1,728 of the 2,036 where they differ |
| 236 | 1 | 4,644 | 205 | Y size at mid-life: non-zero in exactly the 117 emitters whose Y sizes are, and between them in 53 of 55 |
| 240 | 1 | 4,642 | 738 | -40 to 100, non-zero in 1,251, nearly all `Fountain` and `Explosion`; not established |
| 252 to 272 | 1 | 4,592 to 4,630 each | | six types (252, 256, 260, 264, 268, 272), non-zero mostly on `Lightning` emitters; not established |
| 284 | 3 | 4,644 | | colour at mid-life: between the start and end colours in each component in 1,248 of the 1,691 emitters where they differ |
| 284 | 19 | | 422 | the same, bezier |
| 380 | 3 | 9,442 | 30 | colour at the end |
| 380 | 19 | | 232 | the same, bezier |
| 392 | 3 | 9,442 | 30 | colour at the start |
| 392 | 19 | | 205 | the same, bezier |

The mid-life types take the torlack document's names for NWN's (alpha mid, percent start, mid and end, size mid, size mid Y, colour mid) by the fit the table gives. 380 is the end colour and 392 the start because sparks fade to black: of the 353 spark, fire, flame and explosion emitters whose two colours differ in total brightness, 380 is the darker in 273. Emitters and lights carry position (8) and orientation (20) controllers too, as every node does; an animation keys an emitter's position in 160 controllers and its orientation in 197.

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

## What a model names

A model names other resources in seven places, all in the geometry's tree or the model header (an animation's nodes are dummies in every vanilla model): the supermodel; each mesh's texture and second texture (the lightmap); each light's flare textures (the light header's array at 52); a reference node's model (the first 32 bytes of its header, a NUL-ended name); and each emitter's texture (at 128) and chunk model (at 160). Counted over the 2,833 distinct vanilla MDL contents, one naming per node and site, each name matched against every layer of the install without regard to case:

| Site | Namings | Names | Held |
| --- | --- | --- | --- |
| texture | 58,257 | 1,665 | 58,033 as TPC, 5 as TGA only; 193 by nothing (35 names) |
| lightmap | 41,538 | 8,170 | 309 as TPC, 38,873 as TGA only (the BIFs' lightmaps); 2,356 by nothing |
| light flare | 80 | 10 | 63 as TPC; 17 by nothing (5 names) |
| reference node | 1,952 | 42 | all as models (`fx_ref`, `OR_Tatshrub01`, `C_Brith`) |
| supermodel | 375 | 45 | 372 as models; the three above by nothing |
| emitter texture | 8,823 | 102 | 4,016 as TPC; 4,807 by nothing (14 names) |
| emitter chunk | 207 | 46 | all as models (`fx_part_01`, `fx_mrock`, `fx_droid02`) |

The 2,356 lightmaps held by nothing are each named once, by 51 models: 2,001 named `MODEL_aNNNNN` with a base-36 counter (611 in `m31ad_00a`, 528 in `m31ab_00a`, 420 in `m31aa_00a`; `M01aa_04a_a0002t` in the Endar Spire's `m01aa_04a`, `DOR_LHR02_a00004` in `dor_lhr02`), 354 named `MODEL_lmNNN` in seven models nothing the index reads names (`crossgob`, `hal01gob` to `hal04gob`, `hcrates`, `v_ehawk`), and `plc_knrthvin01_a`. The textures held by nothing are mostly on character and cutscene models (`pmbf`, `pmbmv_01` and `pmblv` on the body models `n_commkidm` and `pmbc`, whose `appearance.2da` rows name a texture of their own in `texa` and `texb`; `h_f_lo01headtest` and `p_bastillah01` on cutscene characters), and on toolset markers (`toolcolors` on `gi_waypoint` and `gi_sound_*`). Of the emitter textures held by nothing, 4,754 namings are the four crowd textures of the Taris upper city's rooms (`FX_crowdhead01`, `FX_crowdtorso01`, `FX_crowdlarm01`, `FX_crowdrarm01` in `m31aa_00a`, `m31ab_00a` and `m31ad_00a`), whose `LTS_crowd*` counterparts the install holds and `m02ae_07a` names; the other ten names are `fx_flare04` (15), `fxp_asteroids01` (12), `fxpa_smoke01` (10), `fx_lsabre01`, `fxp_aurabesh01`, `fxpa_sparks` and `lkofx_ripple2` (three each), and `fxpa_cloud02`, `fxpa_flame00` and `fxpa_flame05` (two each). An emitter whose texture is `NULL` (60) names none. The emitter's name at 187 (`fx_depth`, once) is held by nothing. One mesh texture held by nothing has a longer name held beside it, `pmbf` with `pmbf01` and `pmbf02`; whether the engine completes a mesh's texture name as it completes `appearance.2da`'s texture prefixes is not established, so a name is matched as written.

## Layout

Every byte of the model data of all 3,072 vanilla models belongs to a structure the headers reach: no gaps, no structure shared. The structures beyond those above, as the writer holds them:

- A light's header is 92 bytes ([Light header](#light-header)): a float, five arrays, then seven words.
- An emitter's header is 224 bytes ([Emitter header](#emitter-header)) and a reference's 36 (a model name and a word); neither points anywhere.
- A dangly header is 28 bytes: an array of four-byte constraints (one per vertex), three floats, and at 24 the offset of twelve bytes per vertex.
- An AABB header is the offset of the tree's root; each entry is 40 bytes, a box, the offsets of its two children at 24 and 28 (zero for none), and two words.
- A saber header is 20 bytes ([Saber header](#saber-header)): the offsets of three per-vertex arrays (twelve bytes per vertex at 0, eight at 4, twelve at 8) and two words.

BioWare's compiler lays the model data out in one order, the same in all 3,072 models, so a writer that follows it reproduces every offset:

1. The model header, the name array, the names back to back, the animation array.
2. Each animation in turn: its header, its events, its tree.
3. The geometry's tree.

A node is written as its header; the header of its kind; that kind's arrays (a skin's bone map, qbones, tbones and the array at 52; a dangly mesh's constraints and positions; an AABB tree in pre-order, left before right; a saber's arrays at 0, 8 and 4, in that order; a light's array at 4, its texture name offsets and the names, its sizes, positions and colours); then a mesh's faces, index counts, positions (the offset at 328), index offsets, the array at 200 and the index lists; then its child array; then each child, recursively; and last its controllers and their data, so a node's controllers come after all its descendants. An empty array points where the next structure begins, except the controllers, the controller data, the events, the faces and a light's arrays, which hold offset zero, and a mesh's index offset array and array at 200, which hold zero in a mesh with no vertices and point on in one with vertices (the sabers). Array counts are used and allocated alike.

The MDX holds the geometry's blocks in node number order, every mesh but the skins first and then the skins: in 271 models this differs from the order the tree reaches the meshes. After each block's terminator but the last come as many bytes as the MDX's length then lies past a multiple of 16 (8 bytes after a block ending at 8 past a multiple of 16, 4 after one at 4, 12 after one at 12, none after one on the multiple); these bytes are often not zero, uninitialised memory like the model header's bytes at 0x4D. Nothing follows the last block. Thirteen MDXs in `rims/globaldx.rim` and `rims/mainmenudx.rim` (`fxc_droid01` and ten more, `fxmuzzle`, `cgmain_light`) have no MDL in their partner RIM; the MDL of each name is in `models.bif`, and the RIM's copy equals the BIF's.

Other tools lay models out otherwise; `conformance/rakata-known.toml` names the layouts the modded build's models show.

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
- The torlack document's emitter header is NWN's: KotOR's has two more words before the grid (at 12 and 16), a texture field of 32 bytes rather than 64, and 34 bytes between the render order and the flags where NWN has two (a name at 187 among them); KotOR's emitter controllers are NWN's numbers less 24 after the bounce coefficient, with the start, end and mid-life colours at 392, 380 and 284 and the other mid-life values at 216 to 236. Its light header and light controllers match KotOR's, but that the word it calls the generate-flare flag is zero in every KotOR light, those with flares too.
- Neither document describes the saber header or its per-vertex arrays.
- In the skin header, the words at 12 and 16 are the MDX offsets of the weights and bone indices; the bone map's count follows its offset directly (no allocated count); the arrays at 28 and 40 are the qbones and tbones; the node list at 64 is seventeen u16s, not fifteen; and the bone map's entries are floats, local bone indices by node number, as the example in `kotor_mdl.html` has it.

## What could not be established

The meaning of the model header's byte 0x51, the mesh header's arrays at 200 and fields at 212 to 248 and 314 to 320, and the classification value 64; whether byte 0x53 is the fog flag; the skin header's array at 52 and bytes 98 to 100; which of a bezier key's two control values is which; the light header's words at 72, 76, 84 and 88 beyond their position in the torlack document's layout; the emitter header's dead space, its words at 12, 16 and 28, its name at 187 and byte at 219, the meaning of flag bits 1 to 3 and 5 to 12 beyond the torlack document's names, and emitter types 196, 200, 240 and 252 to 272; how a frame index past the grid is drawn; the saber header's two words and the forty columns that repeat the blade's centre line; whether the engine adds an animation's position keys to the rest position or puts them in its place, and whether it uses the stored qbones and tbones or the rest pose's (the files differ in the five models above). Each needs the engine's behaviour, not the files alone.

## Provenance

Public documents read: xoreos-docs `specs/kotor_mdl.html` (KotOR's binary MDL), `specs/torlack/binmdl.html` (NWN's binary MDL: the file header, arrays, node flags, the face record and the mesh header's NWN layout), `specs/gmax_nwn_mdl_0.3b.ms` (the ASCII field names) and `specs/torlack/basics.html` (resource types).

Reference projects read: none.

Measured: on 2026-10-18, every count above but those of lights, emitters and sabers comes from the vanilla KotOR 1 install under `$RAKATA_GAME_K1`, first with short Python scripts reading the headers, nodes, mesh headers, faces, index lists, controllers, skin headers, animation headers and MDX blocks of every model the KEY's BIFs, `rims/`, `modules/` and `patch.erf` hold, then with `crates/rakata-mdl/tests/vanilla.rs`, which parses every one of them, prints the node kinds, classifications, MDX layouts, flag counts, controllers by type, compressed keys, skins and their bind transforms, resolves every supermodel chain, and reads every model `end_m01aa` uses (`RAKATA_GAME_K1=… cargo test -p rakata-mdl --test vanilla -- --nocapture`). The compressed orientation's partition, its agreement with uncompressed copies and its scale were measured by the Python scripts over the 2,832 models of distinct names (one copy of each name the BIFs and RIMs hold). The layout was measured by Python scripts that list every structure each model's headers reach and check that they tile the model data and fall in the order above, and is confirmed by `crates/rakata-mdl/tests/vanilla.rs`, which writes every vanilla model back from `rakata_mdl::ModelFile` and compares bytes, and by `rakata conformance --only mdl`.

Measured: on 2026-10-20, the light, emitter and saber headers, their controllers and the names emitters hold, over the same install, with `crates/rakata-mdl/tests/vanilla.rs`'s `the_light_emitter_and_saber_headers` (`RAKATA_GAME_K1=… cargo test -p rakata-mdl --test vanilla the_light_emitter_and_saber_headers -- --nocapture`): it confirms each header's length from where the next structure begins in the raw model data, counts every field's values by kind of node and update, counts each controller type by the kind of node that carries it in the geometry and in animations, checks the frame, lightning, P2P and colour readings, and resolves each emitter's texture and chunk names through every layer of the install, modules included, over the 2,833 distinct MDL contents. The fits of the mid-life and colour types, and the spark, fire and explosion emitters' colours, were measured by short Python scripts over a dump of the same headers and controllers.

Observed in a renderer: the winding, the undrawn AABB nodes and the skin meshes' texture coordinates, from the OdysseyUE runtime drawing the Endar Spire (`end_m01aa`) from these structures and comparing with the game.

Date: 2026-10-18; lights, emitters and sabers 2026-10-20.

Written by the implementer from measurement; no implementation's source was read.
