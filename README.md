# nwscript

A toolchain for NWScript, the scripting language of BioWare's Aurora-family games: a compiler measured byte for byte against BioWare's own, an interpreter that makes scripts testable without the game, a decompiler that verifies its output by recompiling it, and a drop-in replacement for `nwnnsscomp`. Built for people new to modding who work with coding agents, and for the installers and mods that already exist.

## Results

### The compiler reproduces BioWare's bytecode, and accounts for every script it doesn't

KotOR 1 ships 1,754 scripts as both source and BioWare's compiled bytecode. Compiled with the declarations each was built against, **1,665 of them (94.9%) come out of `nws` byte for byte identical to BioWare's**, more than any other compiler measured:

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

The other 89 are not compiler failures. No compiler can reproduce them from the sources and declarations that shipped with KotOR, and each has an established cause in [`conformance/known.toml`](conformance/known.toml), re-derived by the tests rather than asserted:

- **5 are include files**, with no entry point and nothing to compile.
- **75 were compiled against older versions of their include files.** Their bytecode lacks globals and functions the shipped includes have, such as `k_inc_utility`'s `SW_CONSTANT_*` globals in 60 of them.
- **8 were compiled from an older version of the script itself**, such as `k_qa`, whose bytecode holds a subroutine its shipped source no longer has.
- **1 is Neverwinter Nights code** left in the game's archive, compiled against NWN's engine table.

Getting there meant reconstructing BioWare's build environment from its output. The engine's declarations changed while the game was written, and the bytecode carries each era: of the 55 scripts that call `ActionStartConversation`, 51 were compiled when it took eleven arguments, three against the twelve-argument declaration that shipped, and one against five; ten call the party-member routines at numbers and with arguments the shipped table no longer has. `--declarations bioware` restores the eleven-argument era, and [`conformance/overlays/`](conformance/overlays/) holds the earlier declarations inferred from the bytecode, each file citing the bytes it rests on. The findings are written up in [`docs/research/bytecode-quirks.md`](docs/research/bytecode-quirks.md); the full report, with every differing script, is [`conformance/report.md`](conformance/report.md).

### It builds a real modded game identically

modshop, a companion project, builds a modded KotOR 1 by script from a pristine game and the community's mod archives; four of its mods compile scripts at install time. Built with `nws`'s nwnnsscomp shim natively instead of nwnnsscomp under Wine, the game is identical: the same tree hash over 21,300 files, and none of its 24,277 compiled scripts differs.

### The decompiler proves what it recovers

`nws decompile` recompiles the source it recovers and marks a function verified only when that reproduces the original instructions exactly. **4,023 of 4,029 functions (99.85%) in the vanilla scripts verify, and 46,475 of 46,511 (99.92%) across the 11,355 scripts that ship in KotOR 1's modules as bytecode only.** None is left unrecovered, and every function that does not verify calls an engine routine with arguments from another era of the table, or from Neverwinter Nights'. The fraction for every module is in [`docs/decompiler.md`](docs/decompiler.md).

These are KotOR 1 results. KotOR 2 has not been measured yet.

## Usage

Nothing is released yet. The design is in [`docs/prd.md`](docs/prd.md).

From a checkout, `cargo build --release -p nws -p nwnnsscomp` builds the command and the nwnnsscomp-compatible shim, and `maturin build --release -m crates/nwscript-py/Cargo.toml` builds the Python module (`import nwscript`; its functions are documented in [`crates/nwscript-py/src/lib.rs`](crates/nwscript-py/src/lib.rs)).

## Acknowledgments

Everything this project knows about these games was worked out by other people, most of them volunteers, over more than twenty years. It stands on their shoulders, and this list is a start, not the whole of it. If a name is missing, a pull request adding it is always welcome.

- **BioWare**, for shipping the KotOR scripts as both source and bytecode. Those 1,754 pairs are the only reason a compiler for this language can be verified at all.
- **Edward T. Smith (Torlack)**, whose `nwnnsscomp` has compiled nearly every KotOR mod script ever released. It was written for Neverwinter Nights in 2002 and 2003, and without ever being written for KotOR it reproduces BioWare's bytecode for 91% of the game's scripts. That number is the bar this project measures itself against.
- **The xoreos project**, whose documentation of the NCS format, its stack discipline and its `STORE_STATE` closures describes the bytecode for every Aurora game.
- **The KotOR and Neverwinter Nights wikis and their editors**, for the annotated engine function tables and the file format documentation.
- **KotOR Tool**, which first made the game's archives readable for modders.
- **TSLPatcher**, whose installer conventions every KotOR mod follows, and which the `nwnnsscomp` shim exists to honour.
- **PyKotor, the Holocron Toolset and HoloPatcher**, which brought KotOR's formats, an editor, and a cross-platform installer and compiler to Python.
- **DeNCS**, the decompiler whose output taught the community what the game's module scripts do.
- **Beamdog's `nwnsc`** in neverwinter.nim, the compiler of Neverwinter Nights: Enhanced Edition.
- **nwnrs**, a Rust toolchain for Neverwinter Nights whose compiler, written for a different game, already accepts KotOR's `nwscript.nss` and compiles every vanilla KotOR script that nwnnsscomp does.

This project is a clean-room implementation: it reads no other compiler's source. It measures the compilers above as black boxes, and each appears as a row in the conformance report.

## Licence

ISC. See [`LICENSE`](LICENSE).
