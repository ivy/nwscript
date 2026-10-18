# What refers to a thing

The references live in an index, a SQLite file built once per install and reused. Build it before the first `refs` or `index query`:

```bash
rakata index build --game DIR --tree vanilla --db FILE
```

A few seconds for KotOR 1, a few hundred MB on disk. Without `--db` it goes to `~/.cache/rakata/index.sqlite`; pass `--db` to every later command to keep installs apart. `--tree LABEL` names the tree inside the file; building again under the same label replaces it, so rebuild after changing the tree.

The index records each tree's root, so `--db FILE` stands in for `--game DIR` on any command: without `--game`, the install is the root of the one tree the index holds, or of the tree `--tree LABEL` names when it holds several (`rakata --db FILE which NAME.EXT`).

## refs

```bash
rakata refs OAJ_DONE --kind global --db FILE
rakata refs 42724 --kind strref --db FILE
rakata refs k_transabort --kind resref --db FILE
rakata refs k_trg_transfail.ncs --from --db FILE
```

Each hit is the resource holding the reference, the site in it (a GFF field path, `row N COLUMN` of a 2DA, `@OFFSET Routine.param` of a script) and every place the install holds that resource. Indirect hits follow: a script that names the thing leads to the dialogue nodes, links and events that run it. `--direct` leaves those out. `--from` lists what one resource refers to. Without `--kind`, every kind the word can be is tried. An empty answer says how many resources the index could not read, since a reference inside one of them cannot have been found.

## index query

`rakata index query --db FILE "SQL"` runs SQL read-only: one statement, or a script of several separated by `;`. `rakata index query --help` lists the tables with their key columns; every column:

```bash
rakata index query --db FILE "SELECT name, sql FROM sqlite_master WHERE type = 'table'"
```

The ones a mod author uses most: `locations` (every file and archive member: `path`, `member`, `name`, `type`, `hash`), `dialog_nodes` and `dialog_links` (by the dialogue's `hash`), `cells` (2DA cells), `strings` (the talk table), `calls` (each script's engine calls with their literal arguments), `refs`, and `search`, a full-text table (`WHERE search MATCH 'word*'`). Join through `hash` to `locations` for where a thing lies. For example, the modules that hold a dialogue:

```bash
rakata index query --db FILE "SELECT path FROM locations WHERE name = 'NAME' AND type = 'dlg'"
```

A dialogue name in more than one module means a copy in Override replaces all of them.

A report with a shared prelude is one script: the `CREATE TEMP VIEW` statements run once, on one connection, and every `SELECT` after them reads the views. `--file FILE` reads the script from a file (`-` for standard input) instead of the command line:

```bash
rakata index query --db FILE --file report.sql
```

Each statement that returns rows prints as its own table, a blank line between them. Run by an agent (`CLAUDECODE=1` and the like) with no `--format`, each table is TSV: a header line, then one line per row, tab-separated and never cut; `--format text` gives the aligned table a person sees, which cuts a cell of more than 60 characters in any column but the last. With `--format json` one such statement gives `{columns, rows}` and several give an array of them, in order. Statements that would change the database (`INSERT`, `DELETE`, `CREATE TABLE`, …) fail with `RE1504`, and nothing after the failure runs.
