# Results

What the toolset is measured against, in full. The README's feature matrix summarises these. Every number here comes from a command over the user's own copy of the game, named beside it; no game bytes are in the repository.

## The compiler reproduces BioWare's bytecode, and accounts for every script it doesn't

KotOR 1 ships 1,754 scripts as both source and BioWare's compiled bytecode (`nws conformance`, over the corpus `rakata corpus extract` writes; [`conformance/report.md`](../conformance/report.md) is its report). Compiled with the declarations each was built against, **1,665 of them (94.9%) come out of `nws` byte for byte identical to BioWare's**, more than any other compiler measured:

<!-- conformance:begin -->
| Compiler | Byte-identical to BioWare |
| --- | ---: |
| `nws`, with the declarations each script was compiled against | **1,665** (94.9%) |
| `nws --declarations bioware`, one setting for all | 1,652 (94.2%) |
| `nws`, the shipped declarations | 1,603 (91.4%) |
| nwnnsscomp `-v1.00` | 1,598 (91.1%) |
| nwnnsscomp, default version | 1,204 (68.6%) |
| pykotor (HoloPatcher) | 262 (14.9%) |
| nwnrs | 0 |
<!-- conformance:end -->

The other 89 are not compiler failures. No compiler can reproduce them from the sources and declarations that shipped with KotOR, and each has an established cause in [`conformance/known.toml`](../conformance/known.toml), re-derived by the tests rather than asserted:

- **5 are include files**, with no entry point and nothing to compile.
- **75 were compiled against older versions of their include files.** Their bytecode lacks globals and functions the shipped includes have, such as `k_inc_utility`'s `SW_CONSTANT_*` globals in 60 of them.
- **8 were compiled from an older version of the script itself**, such as `k_qa`, whose bytecode holds a subroutine its shipped source no longer has.
- **1 is Neverwinter Nights code** left in the game's archive, compiled against NWN's engine table.

Getting there meant reconstructing BioWare's build environment from its output. The engine's declarations changed while the game was written, and the bytecode carries each era: of the 55 scripts that call `ActionStartConversation`, 51 were compiled when it took eleven arguments, three against the twelve-argument declaration that shipped, and one against five; ten call the party-member routines at numbers and with arguments the shipped table no longer has. `--declarations bioware` restores the eleven-argument era, and [`conformance/overlays/`](../conformance/overlays/) holds the earlier declarations inferred from the bytecode, each file citing the bytes it rests on. The findings are written up in [`docs/research/bytecode-quirks.md`](research/bytecode-quirks.md); the full report, with every differing script, is [`conformance/report.md`](../conformance/report.md).

## Every file of KotOR 1 a format reads writes back byte for byte

`rakata conformance` is the suite for the game's data: over an install it reads every file a format crate handles, writes it again with that crate's writer, and classifies the result per file. Over a vanilla KotOR 1 install, into every archive's members (`rakata conformance --game DIR --report FILE`, then `cargo xtask readme --report FILE` writes this table):

<!-- rakata-conformance:begin -->
| Format | Files | Identical | Equal after re-read | Different | No reader |
| --- | ---: | ---: | ---: | ---: | ---: |
| `archive` | 401 | 401 | 0 | 0 | |
| `tlk` | 1 | 1 | 0 | 0 | |
| `lip` | 18206 | 18206 | 0 | 0 | |
| `lyt` | 124 | 124 | 0 | 0 | |
| `vis` | 112 | 112 | 0 | 0 | |
| `tpc` | 11536 | 11536 | 0 | 0 | |
| `txi` | 5616 | 5616 | 0 | 0 | |
| `tga` | 5601 | 5601 | 0 | 0 | |
| `walkmesh` | 1554 | 1554 | 0 | 0 | |
| `2da` | 519 | 519 | 0 | 0 | |
| `gff` | 11143 | 11143 | 0 | 0 | |
| `mdl` | 6157 | 6157 | 0 | 0 | |
| no format | 17535 | | | | 17535 |
<!-- rakata-conformance:end -->

A file is **identical** when the writer reproduces its bytes; **equal after re-read** when the bytes differ but read back to the same data, a layout another tool chose and [`conformance/rakata-known.toml`](../conformance/rakata-known.toml) names; **different** when they do not, which is a defect and exits 3. `walkmesh` is the WOK, PWK and DWK files; `mdl` counts each MDL and MDX. Files with no reader are formats no crate reads yet, the sounds, scripts and sound sets among them.

Over the modded build (the vanilla game with the mods modshop installs), the same suite reads 76,578 files with a registered format: none different, and 243 equal after re-read (2 tables, 54 GFFs and 187 models) under the layouts other tools wrote, each named in [`conformance/rakata-known.toml`](../conformance/rakata-known.toml).

## It builds a real modded game identically

modshop, a companion project, builds a modded KotOR 1 by script from a pristine game and the community's mod archives; four of its mods compile scripts at install time. Built with `nws`'s nwnnsscomp shim natively instead of nwnnsscomp under Wine, the game is identical: the same tree hash over 21,300 files, and none of its 24,277 compiled scripts differs.

## The interpreter runs the game's scripts without the game

`nws run` and `nws test` execute bytecode on a model of the script machine, with an engine that records every call it is asked to make. Over the KotOR 1 corpus, **12,967 of 13,139 compiled scripts run to completion** (`NWSCRIPT_CORPUS=DIR cargo test --release -p nwscript-vm --test corpus corpus_runs -- --ignored --nocapture`). The other 172 stop where the game's own scripts do something the machine refuses: 158 divide by zero when every engine routine answers with its type's default (`k_pdan_attack` divides by the maximum hit points, which is then 0), and 14 pass an object where the shipped `nwscript.nss` declares an int, having been compiled against an older table ([`research/bytecode-quirks.md`](research/bytecode-quirks.md)).

## It installs TSLPatcher mods as the game's community does

`rakata install` reads a mod's `tslpatchdata` (its `changes.ini` and files) and applies it, reading back every write. modshop's patcher step builds its modded KotOR 1 with it; compared over 72 install entries against TSLPatcher, run as a black box under Wine, and HoloPatcher, no Rakata defect is open. The method, and the one difference class TSLPatcher's own 2DA writer introduces, are in [`research/tslpatcher-oracle.md`](research/tslpatcher-oracle.md). One behaviour differs by choice: TSLPatcher appends every entry of a mod's `append.tlk`, while `rakata install` and HoloPatcher append only the entries a token names.

## The decompiler proves what it recovers

`nws decompile` recompiles the source it recovers and marks a function verified only when that reproduces the original instructions exactly. **4,023 of 4,029 functions (99.85%) in the vanilla scripts verify, and 46,475 of 46,511 (99.92%) across the 11,355 scripts that ship in KotOR 1's modules as bytecode only.** None is left unrecovered, and every function that does not verify calls an engine routine with arguments from another era of the table, or from Neverwinter Nights'. The fraction for every module is in [`docs/decompiler.md`](decompiler.md).

These are KotOR 1 results. KotOR 2 has not been measured yet.
