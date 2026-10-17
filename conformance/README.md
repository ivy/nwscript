# Conformance

How closely each compiler reproduces BioWare's bytecode for the vanilla KotOR scripts that ship as both source and bytecode. The current numbers are in [`report.md`](report.md), generated with [`report.json`](report.json) by the command below.

| File | What |
| --- | --- |
| `k1.toml`, `k1-modules.toml` | the corpus manifests: script names and sha256 hashes, no game bytes |
| `compilers.toml` | the compilers the suite can run, as argv templates |
| `known.toml` | the established causes of failures no compiler can avoid, and which inferred overlay reproduces which scripts |
| `overlays/` | declarations inferred from BioWare's bytecode, for `nws --overlay FILE`; each says what it rests on |
| `drivers/` | small wrappers that give a library compiler a command line |
| `report.json`, `report.md` | the last full measurement |

## Running it

Extract the corpus from your own game once (see `AGENTS.md`), then:

```bash
export NWNNSSCOMP=/path/to/nwnnsscomp   # a native build: modshop's `mise run nwnnsscomp:build`
export MODSHOP=/path/to/modshop         # for the pykotor row: a checkout after `uv sync`
export NWNRS=$(mise x cargo:nwnrs@0.0.1 -- which nwnrs)   # for the nwnrs rows
nws conformance --compiler nwnnsscomp-v100 --compiler nwnnsscomp --compiler pykotor \
  --compiler nwnrs-o0 --compiler nwnrs-o1 --compiler nwnrs-o2 --compiler nwnrs-o3 \
  --timeout 120 --report conformance/report.json --markdown conformance/report.md
```

Each script is compiled in its own temporary directory with stdin closed and a timeout, and its output is classed as identical, the same length with different bytes, a different length, or failed. A failure whose cause is established in `known.toml` is shown with it. The report records each compiler's command as written, with `{env:…}` unexpanded, and the sha256 of its binary, so a committed report names no machine. `--baseline OLD.json` exits 3 when a compiler reproduces a script it reproduced before.

This project's compiler is two rows: `nws`, against the shipped declarations, and `nws-bioware`, with `--declarations bioware`, which restores the routines BioWare's own scripts were compiled against (the eleven-argument `ActionStartConversation`). Build it first with `cargo build --release -p nws`. Between them they reproduce every script whose bytecode was compiled from the sources that shipped; `known.toml` names the cause of every other, and the report shows it beside each failure and each different output.

nwnnsscomp hangs forever on the `nwscript.nss` BioWare shipped, which has a typo at line 5145; its templates use the corpus's `oracle/` directory, where that line is corrected.
