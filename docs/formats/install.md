# The install

A KotOR install is the folder holding `chitin.key`. The engine loads a resource, named by a resref of at most sixteen bytes and a type, from the first of several places that holds it; `rakata-game` models those places as layers, searches them in the engine's order, and reports every copy it finds with the reason each lost. This document records what the layers are, how names are compared, and what the two measured installs (the vanilla Steam copy and modshop's modded build) hold.

## Layout

The folders that matter to resolution, as the vanilla install spells them:

| Path | Holds |
| --- | --- |
| `chitin.key` | the index over the BIFs: 26 BIFs under `data/` in the vanilla KEY, each named by a path with backslashes (`data\2da.bif`) |
| `data/*.bif` | the base game's resources, nameless in the BIF and named by the KEY |
| `Override/` | loose files, `name.ext`, that replace any resource of the same name and type; empty in the vanilla install, 5,769 files in the modded build |
| `modules/` | each module's capsules: `STEM.rim` (the area: IFO, ARE, GIT and similar) and `STEM_s.rim` (blueprints, scripts, dialogues); a `STEM.mod` is written by mod installers and replaces the pair. 234 files in the vanilla install, 341 in the build (124 modules, every one with a `.mod` beside its RIMs) |
| `lips/STEM_loc.mod` | the module's lip-sync files, a MOD archive per module |
| `patch.erf` | KotOR 1 only: an ERF beside `chitin.key` whose members patch the BIFs' |
| `rims/` | the global and menu-screen archives (`global.rim`, `chargen.rim`, `mainmenu.rim`, their `dx` variants); loaded by the engine for those screens, not searched by name |
| `TexturePacks/` | the textures, as ERF archives: three quality packs (`swpc_tex_tpa.erf`, `swpc_tex_tpb.erf`, `swpc_tex_tpc.erf`), of which the game reads one, and `swpc_tex_gui.erf`; see [The texture packs](#the-texture-packs) |
| `streamsounds/`, `streammusic/`, `streamwaves/`, `movies/` | media by file; not searched by name |

The modded build also holds `Modules -> modules` and `override -> Override` as symbolic links, so that an installer which spells either folder differently finds it on a case-sensitive file system.

## The search order

A resolution asks for a name and type, optionally in a module, and walks:

1. `Override/`: the loose file `name.ext`, in any case.
2. The module's capsules, when a module is named: `STEM.mod`, then `STEM.rim`, then `STEM_s.rim` (KotOR 2 adds `STEM_dlg.erf`), then `lips/STEM_loc.mod`. A `.mod` written over a module replaces the whole pair of RIMs in the engine, so a member of `STEM.rim` that the `.mod` also holds is never loaded.
3. `patch.erf`, in KotOR 1.
4. The texture packs: the selected quality pack (`swpc_tex_tpa.erf` unless the caller selects another), then `swpc_tex_gui.erf`.
5. The BIF that `chitin.key` names for the id, at the index it gives.

The first copy found wins; every later copy is a candidate that lost, with the rule that beat it named (`Override outranks the module's capsules`, `danm14ac.mod outranks danm14ac.rim`). The order is the one the proposal states and the one the Juhani mod's `loaded()` applies; over the modded build the two agree on every input the mod reads (see Provenance). The relative position of `patch.erf` and a module's capsules is not measured: no resource in either install lies in both, so no resolution depends on it. The texture packs' place is set out in the next section.

## The texture packs

`TexturePacks/` (the folder `swkotor.ini`'s `[Alias]` names, `TEXTUREPACKS=.\texturepacks`) holds four ERF V1.0 archives, the same bytes in the vanilla install and the modded build:

| Pack | Members | Types | Size |
| --- | --- | --- | --- |
| `swpc_tex_tpa.erf` | 3,294 | TPC | 400 MB |
| `swpc_tex_tpb.erf` | 3,294 | TPC | 116 MB |
| `swpc_tex_tpc.erf` | 3,294 | TPC | 45 MB |
| `swpc_tex_gui.erf` | 1,571 | 1,570 TPC, 1 TXI (`lbl_menudarr.txi`) | 112 MB |

The three quality packs hold the same 3,294 names. Of them, 2,712 are half as wide in `tpb` as in `tpa` and half again in `tpc` (`c_bantha01`: 512, 256 and 128 pixels, 10, 9 and 8 mip levels); 570 are the same bytes in all three (`3dgui_a00001`, `blood`); 12 are halved from `tpa` to `tpb` and not again. The game reads one quality pack, by the `Texture Quality` option under `[Graphics Options]` in `swkotor.ini` (2 in the vanilla install's file).

`swpc_tex_gui.erf` shares no name with the quality packs: it holds the interface (`lbl_*`, `fnt_*`, the menu art), the quality packs the world (rooms, creatures, placeables, effects). A texture font's TXI is not a member of its own: `fnt_d16x16.tpc` ends with its TXI text (the glyph coordinates), and no layer holds `fnt_d16x16.txi`.

No member of any pack is in the BIFs (which hold 5,601 TGA and 5,615 TXI, no TPC; one BIF TXI shares its name with a quality pack's TPC) or in a module's capsules. `patch.erf` holds 84 TPCs, 75 of them player heads (`pfha01`, `pfha01d`, …) the quality packs hold too; every one of the 75 is other bytes than all three packs' copies, at the size of `tpa`'s (256 pixels where `tpb` has 128).

Over the modded build, `Override/` holds 1,470 TPC, 382 TGA and 89 TXI files named like a quality pack's TPC, and 156 TPC and 238 TGA named like the GUI pack's: mods replace pack textures by writing the name to Override, in either format.

`rakata-game` places the layer, and `rakata which` names the pack a copy came from (`TexturePacks/swpc_tex_gui.erf:fnt_d16x16.tpc`):

- The selected quality pack, then the GUI pack. The two share no name, so their order decides nothing in these installs.
- Below `Override/`, as every layer is; a module's capsules and `patch.erf` above the packs, the BIFs below. Nothing in either install lies in both a pack and a BIF or a module capsule, so only `patch.erf`'s place decides a resolution: its 75 heads win over the packs'. That is a choice, made because the copies a patch shipped at `tpa`'s size would otherwise never be loaded; it is not an observation of the engine.
- The quality pack is `tpa` unless the caller selects another (`rakata which --texture-pack tpb`, `rakata.Install(path, texture_pack="tpb")` in Python, `rakata.Install.open{texture_pack = "tpb"}` in Lua). `tpa` is the pack the asset census and OdysseyUE's reader use; it is a stated choice, and `swkotor.ini` is not read. The unselected packs are not searched: a name only one of them holds resolves nowhere.
- Every type a pack holds is searched for (TPC and TXI in these packs).
- When `TexturePacks/` lacks the selected pack, only the GUI pack is searched and `rakata which` warns (`RW0402`).
- A name is resolved for the type asked: `rakata which lhr_flr01.tpc` does not report an `Override/lhr_flr01.tga`.

## Names and case

Every comparison folds ASCII case: resource names inside archives (`PFHA01` in `rims/chargen.rim` beside the lowercase majority), file names under `Override/`, the capsule file names under `modules/`, and the folder names themselves. The game ran on Windows, where the file system does the same; under Wine on a case-sensitive file system an installer can leave two files whose names differ only in case, and which the engine then opens is not established. `rakata-game` reports such a pair rather than choosing: both are candidates, the first is marked ambiguous, and `RW0401` names them.

Each directory is listed once when the install is opened and every lookup runs over that listing, so a resolution does no directory reads. The listing keeps every spelling it saw.

### Case variants in the modded build

The build's `Override/` holds no case-variant pairs: modshop's build removes them after each installer run (`collapse_case`, which keeps the newest). The build log records ten groups removed across its 192 entries, every one a texture or its `.txi` written in two spellings by different mods: `CM_WK_YAVHGR.tpc`/`CM_WK_YavHgr.tpc`, `n_xorh.tpc`/`N_XorH.tpc`, `n_zharh.tpc`/`N_ZharH.tpc`, `pfbh04.tpc`/`PFBH04.tpc`, `pfbh07.tpc`/`PFBH07.tpc`, `pmbh04.tpc`/`PMBH04.tpc`, `pmbh07.tpc`/`PMBH07.tpc`, `w_blstrpstl_020.tga`/`w_BlstrPstl_020.tga`, `w_blstrpstl_020.txi`/`w_BlstrPstl_020.txi`, `x_vbroswrd_003.tga`/`x_Vbroswrd_003.tga`. `rakata which --duplicates` over the build reports none, which agrees.

## Duplicates within a layer

No vanilla archive repeats a (name, type) pair, so the engine's tie-break is unmeasured; `rakata-archive`'s lookups and this crate answer with the first member in table order. The KEY likewise is read first-wins. A module in the build may hold the same resource in its `.mod` and its `_s.rim` (`dan14_juhani.utc` is in both `danm14ac.mod` and `danm14ac_s.rim`); that is the normal case of a `.mod` written over the RIMs, and the `.mod` wins.

## Measurements

Over the modded build (`$RAKATA_BUILD`, 124 modules, 5,769 Override files), release build, `cargo test -p rakata-game --release --test build -- --nocapture`:

- Open: 18 ms (read the KEY, list the root, `Override/`, `modules/`, `lips/` and `data/`; no archive opened).
- Resolve: 43,043 names (5,768 from Override, 37,275 from the modules' capsules), 64,600 candidates; median 852 ns per resolve, maximum 115 ms (the first touch of a large archive's map and the SHA-256 of a large member), total 11.1 s, almost all of it hashing every candidate's bytes.

The proposal's targets (a resolve under a millisecond after an open under a hundred milliseconds) are met.

## What could not be established

Where `patch.erf` stands relative to a module's capsules; how the engine breaks a tie between two Override files differing only in case, or between two members of one archive with one name; whether the engine searches `lips/STEM_loc.mod` as a module capsule or opens it only for lip-sync, which affects nothing but a LIP placed elsewhere. Each needs the engine, not the files.

For the texture packs: which pack each `Texture Quality` value selects (OdysseyUE's runtime always reads `tpa` and does not read `swkotor.ini`, so it has no observation either); whether `patch.erf` outranks the packs, as modelled, or the reverse; where the packs stand relative to a module's capsules and the BIFs, which no resource in either install tests; whether the engine reads a type other than TPC and TXI from a pack; and which of a TGA in `Override/` and a TPC of the same name in a pack the engine draws (mods write both, and the lint counts either as resolving a texture name). Each needs the engine.

## Provenance

Public documents read: xoreos-docs `specs/torlack/basics.html` (the NWN folder layout and the role of `override`) and the ERF, KEY/BIF and RIM documents in this repository's `docs/formats/`. The KotOR wiki was unreachable from this host.

Reference projects read: none. The search order is the one `docs/proposals/rakata.md` states and the one modshop's own `games/kotor/mods/juhani-clones/make.py` (`loaded()`, `module_copies()`) applies; the two symbolic links and the ten case-variant groups come from listing the build and reading its build log.

Measured: on 2026-10-18, the texture packs' members, types, sizes and TPC headers, their overlaps with each other, `patch.erf`, the BIFs the KEY names, the modules' capsules and the build's `Override/`, read from the vanilla install and the build by a script over the ERF and KEY layouts in this repository's `docs/formats/`; the counts above are its output. The texture layer's tests (`crates/rakata-game/tests/vanilla.rs`) hold `fnt_d16x16.tpc` to the GUI pack, `lhr_flr01.tpc` (a texture of the Endar Spire's `m01aa_01a` room) to each selected quality pack, and `pfha01.tpc` to `patch.erf` over `tpa`.

On 2026-10-17, `rakata which` over `$RAKATA_BUILD` named bytes whose SHA-256 equals what `loaded()` returns for each of the nine resources the Juhani mod reads (`dan14_cutjuh.dlg`, `dan14_juhani.dlg`, `k_hcar_dialog.dlg`, `k_hmis_dialog.dlg`, `dan14_juhani.utc`, `k_pdan_juhani31.ncs`, `k_pdan_14juh_d.ncs`, `globalcat.2da`, `appearance.2da`; every one in Override in that build), for the module-only `module.ifo` and the BIF-only `k_inc_generic.nss` and `classes.2da` in module `danm14ac`, and the set of `danm14ac` capsules holding `dan14_juhani.utc` equals `module_copies()`'s. `loaded()` ran as a black box through modshop's `uv run python`; the comparison script is in the node's result, not committed. The timings above are from the same day on the build host.

Date: 2026-10-17; the texture packs 2026-10-18.
