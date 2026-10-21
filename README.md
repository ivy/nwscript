# nwscript

Mod Star Wars: Knights of the Old Republic (KotOR) with a coding agent. Unit-test scripts without launching the game, compile them to BioWare's exact bytecode more often than any other compiler measured, decompile with proof, install TSLPatcher mods, and read and write the game's files byte for byte, from a command line with JSON output and from Python. Every claim is measured against the files the game ships, and the measurements rerun on your own copy.

It stands on two decades of the community's work, named in [Acknowledgments](#acknowledgments).

## Getting started

There are no releases yet: clone the repository and build it. The Rust toolchain and uv come from [mise](https://mise.jdx.dev) (`mise install` reads [`mise.toml`](mise.toml)); the Python modules build with [maturin](https://www.maturin.rs).

```bash
git clone https://github.com/ivy/nwscript && cd nwscript
mise install
cargo build --release -p nws -p nwnnsscomp -p rakata-cli   # target/release/nws, nwnnsscomp, rakata
maturin build --release -m crates/rakata-py/Cargo.toml      # the Python module `rakata`
maturin build --release -m crates/nwscript-py/Cargo.toml    # the Python module `nwscript`
```

Put `target/release` on your `PATH` and install the wheels from `target/wheels/` into your Python environment. `nws` handles scripts, `rakata` the game's data, and `nwnnsscomp` stands in for the compiler of that name wherever a mod installer expects it.

With Claude Code, the repository is also a plugin that teaches an agent the workflows. From a session in any project:

```text
/plugin marketplace add /path/to/nwscript
/plugin install nwscript@nwscript
```

[`plugin/README.md`](plugin/README.md) says what its two skills cover and how to check that it loaded. You need your own copy of the game: nothing here contains game data.

## What it does

| Capability | State | Verified by | What to use instead |
| --- | --- | --- | --- |
| Compile NWScript to KotOR bytecode (`nws compile`) | shipped | [1,665 of 1,754 vanilla scripts byte-identical to BioWare's](docs/results.md#the-compiler-reproduces-biowares-bytecode-and-accounts-for-every-script-it-doesnt); each of the other 89 has an established cause | nwnnsscomp, the compiler nearly every KotOR mod has been built with |
| Stand in for `nwnnsscomp` in installers | shipped | [a modded KotOR 1 built with it is identical over 21,300 files](docs/results.md#it-builds-a-real-modded-game-identically) | |
| Test scripts without the game (`nws test`, `nws run`) | shipped | [12,967 of 13,139 compiled scripts run to completion](docs/results.md#the-interpreter-runs-the-games-scripts-without-the-game) | playing the game, for what depends on the engine itself |
| Decompile, each function proved by recompiling (`nws decompile`, `rakata decompile`) | shipped | [4,023 of 4,029 functions verify in the vanilla scripts](docs/results.md#the-decompiler-proves-what-it-recovers) | DeNCS, the decompiler the community learned the module scripts from |
| Read and write archives: KEY/BIF, ERF, MOD, RIM | shipped | [401 of 401 archives byte-identical](docs/results.md#every-file-of-kotor-1-a-format-reads-writes-back-byte-for-byte) | KotOR Tool and the Holocron Toolset, to browse them in a window |
| Read and write GFF, 2DA, talk tables and lip-sync files | shipped | [11,143 GFFs, 519 2DAs, the talk table and 18,206 lip-sync files byte-identical](docs/results.md#every-file-of-kotor-1-a-format-reads-writes-back-byte-for-byte) | the Holocron Toolset's editors, to edit them visually |
| Read and write textures (TPC, TGA, TXI); export to PNG (`rakata texture export`) | shipped | [11,536 TPCs, 5,601 TGAs and 5,616 TXIs byte-identical](docs/results.md#every-file-of-kotor-1-a-format-reads-writes-back-byte-for-byte) | an image editor on the exported PNG; turning an edited image back into a TPC is not here yet |
| Read and write models, walkmeshes and room layouts (`rakata mdl`, `walkmesh`, `lyt`) | shipped | [6,157 MDL and MDX files, 1,554 walkmeshes and 124 layouts byte-identical](docs/results.md#every-file-of-kotor-1-a-format-reads-writes-back-byte-for-byte) | KotorBlender, to edit models in Blender |
| Find the copy the game loads, and everything that refers to a resource (`rakata which`, `index`, `refs`) | shipped | the command line's tests over the install (`crates/rakata-cli/tests/`) | |
| Change dialogues, globals, tables and blueprints from Python | shipped | a published mod rebuilt on it, 178 of 184 changed files byte-identical (modshop's port of the Juhani mod) | the Holocron Toolset's dialogue editor, and DLGEditor |
| Read saved games; copy one with changed globals or party (`rakata save`) | shipped | the save tests over the user's saves (`crates/rakata-save/tests/`) | KotOR Savegame Editor, to edit a save in a window |
| Install TSLPatcher mods (`rakata install`) | shipped | [72 install entries compared with TSLPatcher and HoloPatcher](docs/results.md#it-installs-tslpatcher-mods-as-the-games-community-does) | the TSLPatcher or HoloPatcher each mod ships with |
| Lint a game tree, and diff two (`rakata lint`, `rakata diff`) | shipped | the lint and diff tests (`crates/rakata/src/lint/`, `crates/rakata/src/diff/`) | |
| A window to work in | not yet | | the Holocron Toolset and KotOR Tool |
| KotOR II: The Sith Lords | not yet | nothing has been measured against it | the tools above, most of which support both games |

The numbers are KotOR 1's. [`docs/results.md`](docs/results.md) has every table in full, with the command that reproduces each.

## Roadmap

KotOR runs on Odyssey, BioWare's branch of its Aurora engine, so the formats the games share carry across from one game to the next. In order, without dates:

- **Next:** KotOR II: The Sith Lords, on the same Odyssey engine. The tools already take a KotOR II profile (`nws --game k2`, `rakata which --profile k2`); nothing has been verified against the game yet.
- **Later:** Jade Empire, the last Odyssey game; then Neverwinter Nights: Enhanced Edition, NWScript's home on Aurora, which has mature, maintained tools of its own, Beamdog's `nwnsc` and the neverwinter.nim utilities among them.
- **Considering:** Neverwinter Nights 2 and The Witcher, whose formats drift further from Aurora's, and Dragon Age, a newer generation of formats on the Eclipse engine.

## Examples

Find the copy of a resource the game loads, what refers to it, what a conversation says, and what a script does:

```bash
rakata which appearance.2da --game ~/games/swkotor
rakata index build --game ~/games/swkotor --db idx.sqlite
rakata refs Cand --db idx.sqlite                     # every reference to Canderous's tag
rakata dialog show k_hcan_dialog.dlg --game ~/games/swkotor
rakata decompile k_pdan_attack.ncs --module danm14ab --game ~/games/swkotor
rakata install MyMod/tslpatchdata --game ~/games/swkotor --dry-run
```

Every command takes `--format json` and then writes exactly one JSON document, and `rakata explain CODE` explains any diagnostic it prints. The same data from Python:

```python
import rakata

install = rakata.Install("/path/to/swkotor")        # the folder holding chitin.key
model = rakata.Model.load(install, "v_muzflash_01")
for node in model.nodes:
    if node.emitter is not None:
        print(node.name, node.emitter.update, node.emitter.texture)
```

A script tested without the game. The condition, `k_my_cond.nss`:

```c
int StartingConditional()
{
    return GetIsObjectValid(GetItemPossessedBy(GetFirstPC(), "keycard"));
}
```

Its test, in Lua, with `describe` and `it` as in busted. The world model answers the engine's calls, and each run records every call the script made:

```lua
-- tests/k_my_cond_test.lua
describe("k_my_cond: true once the player has the keycard", function()
  local script = nws.script("k_my_cond")

  it("is false without it", function()
    assert.is_false(script:conditional(nws.world()).result)
  end)

  it("is true with it", function()
    local w = nws.world()
    local pc = w:create("creature", "player", { is_pc = true })
    w.party = { pc }
    pc:give("keycard")
    local run = script:conditional(w)
    assert.is_true(run.result)
    assert.called(run, "GetItemPossessedBy")
  end)
end)
```

```bash
nws --game k1 --game-dir ~/games/swkotor compile k_my_cond.nss -o tests/k_my_cond.ncs
nws --game k1 --game-dir ~/games/swkotor -I tests test tests/
```

`--game-dir` supplies the game's `nwscript.nss` and include files, from your copy of the game.

`nws test --reference` prints the whole testing API.

## How it's built

This is a clean-room implementation. It reads no other project's source: other tools are run only as black boxes and measured, and the file formats are learned from public documentation and from the game's own bytes, every fact recorded in [`docs/formats/`](docs/formats/) with how it was established. That keeps the code free of other projects' licences and its design its own. The rule and how it is enforced are in [`AGENTS.md`](AGENTS.md); the designs are [`docs/prd.md`](docs/prd.md) for the scripts and [`docs/proposals/rakata.md`](docs/proposals/rakata.md) for the data.

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
