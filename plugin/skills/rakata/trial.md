# Fresh-agent trial of the rakata skill

The acceptance test for this skill: a new Claude Code session that has only the plugin, the installed tools and their own help takes a described change to a vanilla dialogue to a lint-clean file in a scratch Override. Whoever runs it records what the agent needed that the skill did not give, and fixes it in the skill or the tool.

## Setup

Done by the person running the trial, before the session starts. `VANILLA` is a vanilla KotOR 1 install (`$RAKATA_GAME_K1`); `SCRATCH` is a directory on the same disk, outside this repository and not on a small `/tmp`.

1. Build the tools from this checkout: `cargo build --release -p rakata-cli -p nws`, and put `target/release` first on the `PATH` of the session.
2. Build the Python module: `mise run test:py` makes `target/rakata-py/venv`, whose `bin/python` imports `rakata`. That environment loads the module from `crates/rakata-py/python/`, so the agent's permission to read the stubs there is stated in the prompt. A wheel avoids that: `target/rakata-py/venv/bin/maturin build --release -m crates/rakata-py/Cargo.toml`, then unpack it into `SCRATCH/py` with `python3 -m zipfile -e WHEEL SCRATCH/py` and use `PYTHONPATH=SCRATCH/py` with a Python of the wheel's version. `PYTHON` below is whichever command imports the module.
3. Make the scratch install: `cp -a --reflink=always VANILLA SCRATCH/tree`. Its `Override/` is empty in a vanilla install; check that it is.
4. Copy the plugin without this file, which holds the answer: `cp -r plugin SCRATCH/plugin && rm SCRATCH/plugin/skills/rakata/trial.md`.
5. Start the session from `SCRATCH`, not from inside this repository, so the repository's `AGENTS.md` and `CLAUDE.md` are not loaded: `claude --plugin-dir SCRATCH/plugin`. Confirm with `/plugin` that the `nwscript` plugin is loaded and that `/nwscript:rakata` is offered.

## The prompt

Give the agent this text, with `VANILLA`, `SCRATCH` and `PYTHON` replaced by the real paths, and nothing else:

```text
KotOR 1 is installed at VANILLA (unmodified; do not write there). SCRATCH/tree is a copy of it for you to change.

The task: in the dialogue tar02_twitch.dlg (module tar_m02ae), give the entry whose voice resref is nm02aetwit33016_ one new player reply, with a line of your own, that is offered only while a new Boolean global of your own is false. Declare the global in globalcat.2da and compile the condition script yourself. The reply ends the conversation. Everything you write goes into SCRATCH/tree/Override. When you are done, rakata dialog lint must report no findings for the dialogue in SCRATCH/tree, in its module, and you report the output of rakata diff between VANILLA and SCRATCH/tree.

Tools: rakata and nws are on the PATH. The Python module rakata imports in PYTHON. Use the rakata skill of the nwscript plugin, rakata --help and each command's --help, rakata explain CODE, and the Python module's own stubs and help(). You may read the .pyi stubs in the module's directory and nothing else of the source tree they come from: no other file of the nwscript repository, no work orders or issue trackers, no notes of earlier attempts, and no other modding tool's source. Do not edit the plugin.

At the end, list everything you needed that the skill did not tell you, and every step where you guessed.
```

## Judging the result

All of these, run by the person judging, not taken from the agent's report:

1. `rakata dialog lint tar02_twitch.dlg --module tar_m02ae --game SCRATCH/tree` prints `0 findings` and exits 0. The loaded copy is `Override/tar02_twitch.dlg` (`rakata which tar02_twitch.dlg --module tar_m02ae --game SCRATCH/tree` shows it winning over the module's).
2. `rakata diff VANILLA SCRATCH/tree` exits 3 and lists exactly three added paths, and nothing else:
   - `Override/globalcat.2da`, over the BIF's copy, with one row added: the agent's global, type `Boolean`.
   - `Override/NAME.ncs`, the condition script.
   - `Override/tar02_twitch.dlg`, over `modules/tar_m02ae_s.rim!tar02_twitch.dlg`: the entry `voice:nm02aetwit33016_` changed by one link added to the new reply under the condition `NAME`, and one reply added. No other node changed, and no existing link moved.
3. `rakata dialog show tar02_twitch.dlg --module tar_m02ae --game SCRATCH/tree` shows the new reply last among that entry's replies, its condition decompiled and `[verified]`, reading the agent's global through `GetGlobalBoolean` and true only when the global is false; the reply has no links.
4. `VANILLA/Override` is still empty, and nothing outside `SCRATCH/tree/Override` changed (the diff in 2 shows it).
5. The agent's closing list, and any point in the transcript where it read something outside its inputs, opened a file it was told not to, or retried a refused call, are recorded with the trial's result. Each item is fixed in the skill or, when the tool itself lacked it, in the tool, before the trial counts as passed.
