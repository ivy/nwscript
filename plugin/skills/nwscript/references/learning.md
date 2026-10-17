# Learning from the game's scripts

The fastest way to write a script the game accepts is to read how BioWare wrote one that does something similar.

## Find examples

- KotOR 1's global scripts have source (`scripts.bif`): search it by the routine or constant you need. Prefixes say what a script is: `k_con_` dialog conditionals and `k_act_` dialog actions shared across the game, `k_def_` default creature events, `k_inc_` includes, `k_ai_master` the combat AI. Most of the rest are `k_p` and a planet (`k_pman_` Manaan, `k_pdan_` Dantooine, `k_ptat_` Tatooine) followed by a short subject.
- Module scripts have bytecode only: decompile them (below). A whole module at once: give `decompile` the directory, which reports how many functions verified per directory; then decompile the files of interest one by one.
- `nws -I DIR fn NAME` prints an engine routine's declaration and BioWare's comment on it; `nws fn --grep WORD` searches names and comments.

## Decompile

```bash
nws --game k1 --declarations bioware -I DIR decompile FILE.ncs
```

Each function carries a comment: `verified` (recompiles to the original bytes, so the source is exactly right), `unverified` (recovered, but recompiles differently from the first offset named), or `not recovered` (with the reason). Recovered names are invented (`nLocal1`, `sub_1a3`); rename them as you understand them. `--strict` exits 3 unless every function verified.

`nws disasm FILE.ncs` prints the bytecode as assembly with engine routine names, and `disasm --cfg` the control-flow graph; use them when a function is unverified or to see exactly what a compiler emitted.

## Run it

```bash
nws --game k1 -I DIR run --trace FILE.ncs
```

runs the script under the recording engine and lists every engine call with its arguments and the reply it got. `--engine world` runs against a small world model (objects, globals, party) instead; `--world FILE.toml` gives it a starting state. Calls neither engine models are marked unmodelled and get default replies: what the script does after one may not be what the game does.
