# Example queries

Queries over the index `rakata index build` writes, each one SQL statement in a file of its own. [`docs/index.md`](../index.md) describes the tables they read. The files hold SQL only: what they return describes the user's own game files and stays out of the repository.

## Running one

Build the index of the tree once, then pass the file's text to `rakata index query`:

```bash
rakata index build --game PATH/TO/swkotor --tree build --db idx.sqlite
rakata index query --db idx.sqlite "$(cat docs/examples/canderous.sql)"
rakata index query --db idx.sqlite --format json "$(cat docs/examples/canderous.sql)"
```

The database is opened read-only, so a query cannot change it. `--format json` gives the column names and the rows as arrays, for a script to read. A query reports on the tree built last unless its `tree` CTE names another by label.

## The queries

- [`canderous.sql`](canderous.sql): Canderous across the install, one row per module (a module's `.mod`, `.rim` and `_s.rim` together, `Override`, and `global` for the BIFs). The columns count the dialogue nodes he speaks or hears (tag `Cand`), the nodes whose text names him (full text over embedded text and the talk table), the script calls passing his party slot (`NPC_CANDEROUS`, 1) as the first argument of a party routine, and the scripts and other GFFs naming his tag, his blueprint `p_cand` or his conversation `k_hcan_dialog`. It runs in under a second over a full build.

For one symbol rather than a report, `rakata refs` answers directly: `rakata refs Cand --db idx.sqlite` lists every reference to the tag with where the game loads each resource from.
