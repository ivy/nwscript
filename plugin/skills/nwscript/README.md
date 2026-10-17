# `/nwscript:nwscript` — NWScript with nws

Gives Claude the working loop for KotOR scripts with `nws`, a reference at a time: what to run to go from a described change to an installed script, how to read a diagnostic, how to learn from the game's scripts, and how to test one without starting the game.

```text
/nwscript:nwscript
/nwscript:nwscript make k_con_atdant true only after the Dantooine enclave is done
/nwscript:nwscript what does modules/danm13_s/k_pdan_13_area.ncs do?
```

## How it works

`SKILL.md` is a short index: what every command needs (`--game`, `-I`), a table from task to reference file, and the traps that cross every task. Claude reads only the reference the task needs:

| File | Covers |
| --- | --- |
| [`workflow.md`](references/workflow.md) | find the script, change and check, test, compile, install |
| [`diagnostics.md`](references/diagnostics.md) | reading a diagnostic, `nws explain`, lint levels and `--compat`, declarations and overlays |
| [`learning.md`](references/learning.md) | finding vanilla examples, decompiling, disassembling, running with a trace |
| [`testing.md`](references/testing.md) | Lua tests: worlds, answering unmodelled routines, assertions, snapshots |

The references name commands and flags and leave their full description to `nws <command> --help` and `nws explain CODE`, which are complete and change with the tool.
