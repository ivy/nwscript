# Check a change

Three checks, each a command: the dialogue lints, the tree lints, and a diff against the original. Run them against the scratch install after every change.

## Lint the dialogue

```bash
rakata dialog lint NAME.dlg --game SCRATCH
rakata dialog lint NAME.dlg --module STEM --game SCRATCH
```

The copy linted is the one the game loads, so a dialogue the mod wrote to `Override/` is the one checked. Name the module the dialogue plays in with `--module`, so its scripts and conditions are looked up where the game finds them. Each finding names its code, the node's stable key and what is missing: RW1101 a link to a missing node, RW1102 a script, condition or end script (`EndConversation`, `EndConverAbort`) no layer holds, RW1104 a voiced node without its voice-over, RW1105 a strref past the talk table, RW1106 a condition reading a global `globalcat.2da` does not declare, RW1107 a node no start reaches, RW1108 a voiced node without its lip sync, RW1109 a global whose name is computed, RW1110 a `SoundExists` that disagrees with the node's audio. `rakata explain CODE` says what to do about each. Exit 1 when there is a finding.

Lint the original first (`--game VANILLA`): what it already reports is the game's. `--write-known FILE` records a run's findings and `--known FILE` accepts them in later runs, so only new findings are reported and the exit status is 0 when there are none. `--all` lints every dialogue of the install, or of `--module`.

## Lint the tree

```bash
rakata lint --game SCRATCH
```

References that resolve to nothing in blueprints, 2DAs and scripts (RW1901), and in models and layouts (RW1906: a mesh's texture or lightmap, a supermodel, a layout's room), strrefs past `dialog.tlk` (RW1902), Override files whose names differ only in case (RW1903), and a `.tpc` in Override beside a `.tga` or `.dds` of the same name, which crashes the game (RE1901). The vanilla game has hundreds of findings of its own: run it once on the original with `--write-known FILE` (it prints every finding, so send the output to a file), then on the scratch install with `--known FILE`, and only the change's findings remain.

## Diff against the original

```bash
rakata diff VANILLA SCRATCH
rakata diff VANILLA SCRATCH --only Override/
```

A tree diff pairs files by path; an Override file the original lacks is compared with the copy it replaces, so a changed dialogue shows its nodes, not a new file. `+` is added, `-` removed, `~` changed, with the sites (fields, rows, nodes and links by stable key; a texture's levels and TXI instructions, a layout's rooms, a walkmesh's materials and transitions, a model's nodes, textures and animations) beneath each path. A model's MDL and MDX are read as one: when both changed, the MDL's entry describes the model. A file whose bytes differ while its data is the same (another tool's layout) is marked `the same data, laid out differently`. Exit 3 when the trees differ, 0 when they hold the same data.

Read it as the statement of the change: every line should be something the task asked for. For a new reply under a condition, expect the dialogue with one added reply and one added link carrying the condition, the condition's `.ncs` added (bytecode is compared by hash), and one added row in `globalcat.2da`. Anything else is a side effect to explain or undo.

Two resources compare the same way: `rakata diff NAME.dlg ./edited.dlg --game VANILLA --module STEM`.
