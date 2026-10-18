# Make a change through the Python module

Changes are made in Python with the module `rakata`, which reads the install as the game does, checks every write against it, and writes into `Override/`. The command line only reads, checks and compares.

## Before you start

- The module must import in the interpreter you use, which is not necessarily `python3`: the user names it (a virtual environment's `bin/python`, or a wrapper that sets `PYTHONPATH` to an unpacked wheel), and every `python3` below means that interpreter. `PYTHON -c "import rakata; print(rakata.__version__)"` prints the same build as `rakata --version`. If it does not import, ask the user which environment holds it. `-I` (isolated mode) ignores `PYTHONPATH`, so it breaks a wrapper that relies on it; with a virtual environment, `-I` is fine.
- The module's documentation is its type stubs, which ship beside it. Print where they are and read the ones the task needs:

  ```bash
  python3 -c "import rakata, os; print(os.path.dirname(rakata.__file__))"
  ```

  `__init__.pyi` has the install, the formats, dialogues and node keys; `author/__init__.pyi` has `Mod`; `author/dialog.pyi`, `author/globals.pyi`, `author/scripts.pyi`, `author/twoda.pyi` and `author/blueprint.pyi` have the edits. `help(rakata.author.dialog.add_reply)` prints the same text.
- Every refusal raises `rakata.RakataError`, whose `.code` is a registered code: `rakata explain CODE` says what it means and what to do.

## A scratch install

Never write into the install the user plays from unless asked. Make a copy and write into the copy's `Override/`. On a filesystem with reflinks (Btrfs, XFS) a copy costs no space:

```bash
cp -a --reflink=auto VANILLA/ SCRATCH/
```

Keep the copy on the same disk as the original, not under a small `/tmp`. Afterwards `rakata diff VANILLA SCRATCH` shows exactly what the change did (`check.md`).

A mod's script reads the copies the game loads, and after a first run those are its own outputs in `Override/`. Run it again over the same copy and it builds on them: its globals are already declared (RE1404) and its dialogue nodes are added a second time. Before running it again, delete the files it wrote from `SCRATCH/Override/` (the original copy's `Override/` is the baseline), or make a fresh copy.

## The order of a change

A dialogue write is checked against the install, including what this mod already wrote, and refused with RE1407 when a script, condition or global it adds does not exist. So write in this order:

1. Compile the scripts and write each `.ncs`.
2. Declare the globals and write `globalcat.2da`.
3. Edit the dialogue and write it.

```python
import rakata
from rakata.author import dialog, globals, scripts

install = rakata.Install("SCRATCH")          # the folder holding chitin.key; profile="k2" for TSL
m = rakata.Mod.open(install, "STEM")         # the module the dialogue lives in; None for a global one

# 1. A condition script: StartingConditional returns TRUE when the node may show.
src = scripts.Source("cond_resref", """
int StartingConditional()
{
    return !GetGlobalBoolean("MYMOD_ASKED");
}
""")
(compiled,) = scripts.compile(m, [src])
m.write_override("cond_resref.ncs", compiled.ncs)

# 2. The global, as a new row of the globalcat.2da the game loads.
table = rakata.TwoDa.parse(m.loaded("globalcat.2da").bytes)
globals.declare(table, "MYMOD_ASKED", "Boolean")
m.write_override("globalcat.2da", table.to_bytes())

# 3. The dialogue: a new reply, linked from an existing entry under the condition.
name = "NAME.dlg"
d = rakata.Dialog.parse(name, m.loaded(name).bytes)
reply = dialog.add_reply(d, "The mod's own line.")
dialog.link(d, "voice:RESREF", reply, condition="cond_resref")
written = m.write_override(name, d.to_bytes())
print(written.path, written.hash, [f.message(name) for f in written.warnings])
```

Resrefs (script and file names) are at most 16 bytes (RE0101 at the write; the compiler does not check the name), lower case by convention. Give the mod's scripts and globals a prefix of its own, so they cannot collide with the game's or another mod's.

## Dialogue edits

`rakata.author.dialog` appends and never moves an existing node, so every index and every other mod's key stays valid:

- `add_entry(d, text, speaker=…, script=…)` and `add_reply(d, text, script=…)` append a node and return its `NodeRef`. Text is `None` (no line), an `int` (the game's line by strref) or a `str` (the mod's own line, in English; Windows-1252 only, RE1108 otherwise).
- `link(d, source, to, condition=None, at=None)` adds a link from `source` to `to`, last among the source's links unless `at` gives a position; it returns the position. An entry links to replies and a reply to entries (RE1106 otherwise). A reply with no links ends the conversation.
- `start(d, to, condition=None, at=None)` adds a starting line, first unless `at` says otherwise.
- `unlink_all` and `reorder` change existing structure, so use them only when the task asks for it.

A node's action script is the `script=` argument; a link's condition is `condition=`. Both name a compiled script by resref.

## Node keys, and what a key failure means

Anywhere the API takes a node it takes a key: the text `voice:RESREF`, `strref:N`, `text:TEXT`, `E3`, `R0`, a `rakata.NodeKey`, or a `NodeRef` returned by an `add_` call. `rakata dialog show` prints each node's voice resref and strref (`find.md`).

- **RE1102**, no node matches: the key is wrong, or the dialogue is another copy than the one you looked at. Check `rakata which NAME.dlg --module STEM --game SCRATCH` and the transcript.
- **RE1103**, several nodes match: usually a text key on a line the dialogue repeats. Use the voice resref or strref.
- **RE1104**, the key does not parse.
- An index key (`E3`, `R0`) always resolves but moves when another mod inserts nodes before it; `NodeKey.parse("E3").is_fragile()` is true. Prefer `voice:` or `strref:`.

`m.find(d, key)` resolves a key to its `NodeRef`, comparing text keys through the install's talk table, so you can check a key before editing.

## Globals

`globals.declare(table, name, "Boolean" | "Number")` adds a row to `globalcat.2da` and returns its index; RE1404 when the table already declares the name in any case (it is the game's or another mod's, or this mod already ran on this tree). A condition that reads a global the table lacks is RW1106 in the lints and refuses the dialogue write.

## Scripts

`scripts.compile(m, sources, includes=[], flags=None)` compiles against the install's `nwscript.nss` and includes (the mod's own sources first, then the copy the game loads) and returns one `Compiled` per source; it writes nothing. RE1408 carries the compiler's diagnostics; `nws explain CODE` explains each. The default is the strict mode; `scripts.Flags.BIOWARE` compiles a decompiled game script unchanged. For anything longer than a condition, write and test the script with the `nwscript` skill first.

## Tables and blueprints

- `rakata.author.twoda.add_row_copying(table, source_row, new_label, cells)` copies a row with some cells changed and returns the new index.
- `rakata.author.blueprint.get` and `set` read and change a GFF field by path, keeping its type (RE1406 for a value of another type); `clone(gff, template)` copies a blueprint under a new `TemplateResRef`.
- `rakata.Gff.parse(m.loaded("NAME.utc").bytes)` and `.to_bytes()` round-trip any GFF; write it back with `m.write_override`.

## Writes

`m.write_override(name, data)` reads the bytes back as their type before writing atomically into `Override/`, and returns `Written` (`path`, `hash`, `len`, `warnings`). A dialogue's `warnings` are findings its original copy already had, which are the game's, not the edit's. `m.loaded(name)` returns the copy the game loads with this mod's writes first, and `.written` says whether it is the mod's own. `m.guard_replaced(name, sha256)` stops a mod that replaces a resource when the copy the game loads is not the one it was written against.
