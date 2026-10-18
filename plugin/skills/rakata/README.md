# `/nwscript:rakata` — game data with rakata

Gives Claude the working loop for KotOR's data with `rakata` and its Python module, a reference at a time: find the copy the game loads, find what refers to it, make the change through the authoring API, and check it with the lints and a diff.

```text
/nwscript:rakata
/nwscript:rakata add a reply to this dialogue that shows only until a new global is set
/nwscript:rakata what refers to strref 42724?
```

## How it works

`SKILL.md` is a short index: what every command needs (`--game`, `--module`), a table from task to reference file, and the traps that cross every task. Claude reads only the reference the task needs:

| File | Covers |
| --- | --- |
| [`find.md`](references/find.md) | `which`, `cat`, `gff`, `2da` and `tlk show`, `dialog show` and the node keys it prints |
| [`refs.md`](references/refs.md) | `index build`, `refs`, `index query` and the index's tables |
| [`author.md`](references/author.md) | the Python module: a scratch install, the order of a change, dialogue edits by stable key, key failures, globals, scripts, tables, writes |
| [`check.md`](references/check.md) | `dialog lint`, `lint` with known lists, `diff` against the original |
| [`repository.md`](references/repository.md) | for an agent working in the repository: the clean room, no game bytes, the skill's own test |

The references name commands, flags and the module's calls, and leave their full description to `rakata <command> --help`, `rakata explain CODE` and the module's stubs, which change with the tool.

[`trial.md`](trial.md) is the fresh-agent acceptance trial for this skill: the prompt, the inputs and how to judge the result.
