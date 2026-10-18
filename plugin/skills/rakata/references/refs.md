# What refers to a thing

The references live in an index, a SQLite file built once per install and reused. Build it before the first `refs` or `index query`:

```bash
rakata index build --game DIR --tree vanilla --db FILE
```

A few seconds for KotOR 1, a few hundred MB on disk. Without `--db` it goes to `~/.cache/rakata/index.sqlite`; pass `--db` to every later command to keep installs apart. `--tree LABEL` names the tree inside the file; building again under the same label replaces it, so rebuild after changing the tree.

## refs

```bash
rakata refs OAJ_DONE --kind global --db FILE
rakata refs 42724 --kind strref --db FILE
rakata refs k_transabort --kind resref --db FILE
rakata refs k_trg_transfail.ncs --from --db FILE
```

Each hit is the resource holding the reference, the site in it (a GFF field path, `row N COLUMN` of a 2DA, `@OFFSET Routine.param` of a script) and every place the install holds that resource. Indirect hits follow: a script that names the thing leads to the dialogue nodes, links and events that run it. `--direct` leaves those out. `--from` lists what one resource refers to. Without `--kind`, every kind the word can be is tried. An empty answer says how many resources the index could not read, since a reference inside one of them cannot have been found.

## index query

`rakata index query --db FILE "SQL"` runs one read-only statement. The tables and their columns:

```bash
rakata index query --db FILE "SELECT name, sql FROM sqlite_master WHERE type = 'table'"
```

The ones a mod author uses most: `locations` (every file and archive member: `path`, `member`, `name`, `type`, `hash`), `dialog_nodes` and `dialog_links` (by the dialogue's `hash`), `cells` (2DA cells), `strings` (the talk table), `calls` (each script's engine calls with their literal arguments), `refs`, and `search`, a full-text table (`WHERE search MATCH 'word*'`). Join through `hash` to `locations` for where a thing lies. For example, the modules that hold a dialogue:

```bash
rakata index query --db FILE "SELECT path FROM locations WHERE name = 'NAME' AND type = 'dlg'"
```

A dialogue name in more than one module means a copy in Override replaces all of them.
