# nwscript

A toolchain for NWScript, the scripting language of BioWare's Aurora-family games: a compiler measured byte for byte against BioWare's own, an interpreter that makes scripts testable without the game, a decompiler that verifies its output by recompiling it, and a drop-in replacement for `nwnnsscomp`. Built for people new to modding who work with coding agents, and for the installers and mods that already exist.

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
- **Beamdog's `nwnsc`** in neverwinter.nim, and **nwnrs**, the toolchains of the Neverwinter Nights community.

This project is a clean-room implementation: it reads no other compiler's source. It measures the compilers above as black boxes, and each appears as a row in the conformance report.

## Conformance

<!-- conformance:begin -->
Not yet measured. The table is generated from the conformance report.
<!-- conformance:end -->

## Usage

Nothing is released yet. The design is in [`docs/prd.md`](docs/prd.md).

## Licence

ISC. See [`LICENSE`](LICENSE).
