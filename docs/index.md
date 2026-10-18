# The index

`rakata index` keeps, in one SQLite file, the facts of every resource a game tree holds: where each lies, what each says, every reference each makes to the game's symbols, and full text over what people read. It is the query surface the dialogue lints, `rakata refs`, the tree lint and the agent skill read; this document is their reference for the schema. The crate is `rakata-index`; the extractors that decide what a resource contributes are pure functions in the format crates (`rakata_dialog::facts`, `rakata_gff::facts`, `rakata_2da::facts`, `rakata_tlk::facts`, `nwscript_ncs::facts`).

## Commands

```bash
rakata index build --game DIR --tree LABEL [--db FILE] [--parent LABEL --changed FILE]
rakata index query [--db FILE] SQL              # rows as a table, or --format json
rakata index query [--db FILE] --file FILE      # a script: statements separated by ;
rakata index digest [--db FILE] [--tree LABEL]  # the content digest
rakata index coverage [--db FILE] [--tree LABEL] [--list]
rakata refs THING [--kind strref|resref|tag|global|auto] [--to|--from] [--direct] [--tree LABEL] [--db FILE]
```

The database is `--db FILE`, or `index.sqlite` under `$XDG_CACHE_HOME/rakata/` (`~/.cache/rakata/`). It is derived from the game's bytes, so it never goes in git and can always be built again. A build loads the file into memory, works there, and writes it back once (`VACUUM INTO` a file beside it, renamed over it). `query`, `digest` and `coverage` open it read-only.

A build walks the tree as the round-trip suite does (`rakata::conformance::walk`: `chitin.key` and its BIFs, `data/*.bif`, `Override/` at any depth, `modules/`, `rims/`, `lips/`, `patch.erf`, `TexturePacks/*.erf`, `streamwaves/**/*.lip`, `dialog.tlk`), lists every member of every archive, and hashes every file and member with SHA-256. Every other file of the tree is recorded as `outside` and not read. Script calls are read from each script's bytecode through `nwscript-ncs`, with routine names and parameter types from the tree's own `nwscript.nss`; the corpus's `calls.jsonl` is not consulted, so a modded tree's scripts are read the same way as vanilla's.

## Trees and incremental builds

A tree is a folder holding `chitin.key`, indexed under a label. Building a label the database already holds replaces that tree's rows. With `--parent LABEL`, every file the parent holds is carried over unread (its locations copied to the new tree) unless `--changed FILE` names it; files the parent does not hold are read whether or not the list names them. The list is one path per line relative to the tree, bare or marked as a modshop checkpoint's `changed.txt` marks it (`+ ` added, `~ ` changed, `- ` removed; removed paths are skipped). Paths compare without regard to case.

Every content is parsed once: a hash the database already holds, or another file or member of the same build has already met, is located and not parsed again. A build of a checkpoint from its parent therefore parses only the hashes its changed files introduce, and the build's output counts both (`hashes parsed`, `hashes reused`).

Indexing checkpoints in chain order, each from the one before it, makes `resources.first_tree` the first checkpoint in which a content appeared.

## Tables

Hashes are SHA-256 as sixty-four lowercase hex digits. A resource's name is its resref, lowercase; its type is its extension (`dlg`, `2da`, `ncs`), or for a loose file of no known type the file's own extension.

### `meta(key, value)`

`schema` holds the schema version (currently 3). There is no migration: a database of another version is refused (RE1502), by `build` as by every reader, and is built again into a new file (or after deleting the old one). The version changes with any table or column, or with what an extractor puts in one.

### `trees(id, label, parent, root)`

One row per tree. `parent` is the `id` of the tree it was built from, or null for a cold build; `root` is the folder as `--game` named it.

### `resources(hash, name, type, size, first_tree)`

One row per distinct name and type a content is known by: the same bytes as `a.utc` and `b.utc` are two rows. `size` is in bytes. `first_tree` is the `id` of the first tree whose build located it.

### `contents(hash, type, status, reason)`

One row per content: what the index made of it. `status` is one of the coverage statuses below, never `outside`; `reason` is the reader's error for `failed`, and why for a `no_reader` that has one.

### `locations(tree, path, member, name, type, hash)`

One row per file of a tree's walk and per member of each archive in it. `path` is relative to the tree, with `/` separators, spelled as the file system has it (the walk prefers the folder that is not a symlink). `member` is empty for a loose file; for an archive member it is `name.ext`, and for a BIF member it is the name `chitin.key` gives it, or `#N.ext` (its position) when the KEY names none.

### `coverage(tree, path, member, status, reason)`

One row per file of the tree, walked or not, and per archive member: what each contributed. The statuses:

- `parsed`: a reader read it, and its facts are in the tables below. Readers exist for every GFF type (dialogues, blueprints, areas, instances, journals, factions, GUIs, paths), 2DA, TLK, NCS and NSS.
- `container`: an archive (KEY, BIF, ERF, MOD, SAV, HAK, RIM); its members have rows of their own. An archive inside an archive is `no_reader`.
- `no_reader`: no reader for its type (textures, models, walkmeshes, sounds, lip-sync files): located and hashed only. `rakata index coverage` and the build's output count these by type.
- `failed`: a reader rejected it, or the file could not be read; `reason` holds the error with its code. A file that could not be read has a coverage row and no location.
- `outside`: a file of the tree outside the folders the game loads resources from (the executable, configuration, movies, `streamwaves/` audio): recorded, not read, not hashed.

A query that finds nothing is qualified by coverage: a reference in a `no_reader` or `failed` resource cannot be found.

### `dialog_nodes(hash, node, kind, idx, speaker, listener, strref, text, voice, sound, script, delay, camera_angle, camera_id, camera_animation, quest, quest_entry, comment, animations)`

One row per entry and reply of a dialogue. `node` is `E3` for `EntryList[3]` and `R0` for `ReplyList[0]`; `kind` is `entry` or `reply` and `idx` the index. `speaker`, `listener` and `quest` are tags, null when empty (an entry with no speaker is the owner's). `strref` is the text's strref, null when it has none; `text` is the English text the node embeds, null when it embeds none, so a node's shown text is `text` or the talk table's line for `strref` (the view `node_text` does the join). `voice`, `sound` and `script` are lowercase resrefs. `delay`, the camera fields and `quest_entry` are the stored numbers. `animations` is JSON, `[{"participant": TAG, "animation": N}]`, or null.

### `dialog_links(hash, source, position, target, condition, is_child)`

One row per link: `source` is `start` for the `StartingList` or the node the link leaves (`E3`), `position` its index in that list, `target` the node it leads to, `condition` the lowercase resref of its `Active` script or null, `is_child` 1 when it is a link to a node another link owns.

### `gff_fields(hash, path, label, type, value, strref)`

One row per field, at any depth, of every GFF that is not a dialogue. `path` is the field's path as `rakata gff show --path` takes it (`Creature List[0].TemplateResRef`). `type` is the format's name for the field type (`INT`, `CExoString`, `CResRef`, `CExoLocString`, `List`, …). `value` is an integer for every integer type, a real for `FLOAT` and `DOUBLE`, text for `CExoString` (decoded from Windows-1252) and `CResRef` (as stored), the English substring for a `CExoLocString` (null without one), space-separated numbers for `Vector` and `Orientation`, the length for a `List`, the struct id for a `Struct`, and null for `VOID`. `strref` is set for a `CExoLocString` with a strref and for a `StrRef` field.

### `cells(hash, row, label, column, value)`

One row per cell of a 2DA that is not blank: `row` is the row's position, `label` its label, `column` the column's name as written. `column` compares without regard to case, so `column = 'Name'` finds a column the file spells `name`.

### `strings(hash, strref, text, sound, sound_length, flags)`

One row per entry of a talk table that has text or a voice-over: the strref, the text, the voice-over's lowercase resref or null, its length in seconds and the presence flags as stored.

### `calls(hash, offset, routine, routine_id, args)`

One row per `ACTION` in a script's bytecode: the instruction's file offset, the routine's name (`routine_N` when the tree has no `nwscript.nss`) and number, and `args`, a JSON array with one element per argument: the literal where a single `CONST*` instruction pushes it (a number, a string, `{"object": "OBJECT_SELF"}` or `{"object": "OBJECT_INVALID"}` or `{"object": N}`, `[x, y, z]` for a vector), and `null` where it is computed. Each argument is read on its own, from the stack slot it occupies when the `ACTION` runs (the stack depths of `nwscript_ncs::cfg`, with each routine's parameter and return sizes from `nwscript.nss`): `GetItemPossessedBy(oPC, "k37_itm_freedont")` with `oPC` copied from a local is `[null,"k37_itm_freedont"]`. A slot is a literal only when one `CONST*` instruction pushes it and nothing pops, changes or overwrites it before the call; a string built by `+`, a negated number or a value passed through a variable is `null`. Where a depth cannot be determined (a tree without `nwscript.nss`, a routine the table does not know), only the literals directly before the `ACTION` are read, up to the first computed argument. The literal conventions are those of the corpus's `calls.jsonl`. Read an argument with `json_extract(args, '$[1]')`.

### `resolved_calls(hash, site, offset, routine, routine_id, args, path)`

One row per call whose arguments following the script resolves further than `calls` reads them (`nwscript_ncs::flow`): through the script's own functions, so `UT_SetPlotBooleanFlag(OBJECT_SELF, SW_PLOT_BOOLEAN_01, TRUE)` gives the `SetLocalBoolean` inside `k_inc_utility`'s wrapper the arguments `[{"object":"OBJECT_SELF"},0,1]`; through locals; through globals that keep the value their initialiser pushes (K1's `SW_PLOT_BOOLEAN_01` is a global `int`, not a constant); and through what an earlier call returned, as `{"call": "GetObjectByTag", "args": ["k37_inv_statue", 0]}`. `offset` is the `ACTION`'s, and `routine`, `routine_id` and the literals are as in `calls`. `path` is a JSON array of offsets from the instruction where the arguments are known to the `ACTION`: each `JSR` (or a deferred action's `STORE_STATE`) on the way, then the `ACTION`; `site` is its first. A call whose arguments `calls` already reads in full is not repeated here, and an `ACTION` reached with the same arguments along several paths has one row, for the first path. `OBJECT_SELF` in the body of an action passed to a routine taking an object before it (`AssignCommand`) is that object; in one passed to `DelayCommand` or `ActionDoCommand`, and in a function called from either, it is the script's own. A value the analysis cannot follow (a loop's variable, a parameter of the script's entry, a value from two branches that differ) is `null`.

### `script_strings(hash, offset, value)`

One row per `CONSTS` in a script's bytecode: the instruction's file offset and the string, decoded from Windows-1252. `value` compares without regard to case. It holds every string a script names, whether or not a call the index resolved takes it, so a tag that reaches its call through a variable or a script's own function is still found here; `rakata refs` reads it for its weaker matches.

### `refs(from_hash, kind, value, site)`

One row per naming of a symbol by a content. The kinds:

- `resref`: a resource, by lowercase resref. From every non-empty `CResRef` field of a GFF (a dialogue's `Script`, `Active`, `VO_ResRef`, `Sound`; a blueprint's `Conversation`, `TemplateResRef`, event scripts); from a talk-table entry's voice-over; and from a script's string literal passed to a parameter whose name speaks of a script, template, resref, dialogue, sound, movie or module (`ExecuteScript`'s `sScript`, `CreateObject`'s `sTemplate`).
- `strref`: a line of the talk table, in decimal. From every `CExoLocString` with a strref and every `StrRef` field, from each 2DA cell holding a decimal number in a column of strrefs (`name`, `description`, `spelldesc`, `gameplayhint`, `storyhint`, `trapname`, and any column whose name contains `strref` or `stringref`), and from a script's integer literal passed to a parameter named for a strref (`nStrRef`).
- `tag`: an object's tag, as written. From a `CExoString` field labelled `Tag`, `LinkedTo`, `KeyName`, `Speaker`, `Listener`, `Participant` or `Quest`, and from a script's string literal passed to a parameter whose name contains `Tag`.
- `global`: a global variable, as written. From each row of `globalcat.2da` (its `name` cell), and from a script's string literal passed to `GetGlobal*` or `SetGlobal*`.

`value` compares without regard to case. `site` says where in the content: a GFF field path for a GFF or dialogue (`EntryList[3].RepliesList[0].Active`), `@OFFSET Routine.param` for a script (`@4793 ExecuteScript.sScript`), `row N` for `globalcat.2da` and `row N COLUMN` for a 2DA's strref cell, `strref N` for a talk table.

### `search(body, kind, hash, site)`

Full text, FTS5 with the `unicode61` tokenizer (diacritics removed) and a prefix index; query it with `MATCH`. `kind` and `site`:

- `node`: a dialogue node's embedded text; `site` is the node (`E3`).
- `comment`: a dialogue node's comment; `site` is the node.
- `string`: a talk-table line; `site` is the strref in decimal, so `CAST(site AS INTEGER)` joins `dialog_nodes.strref`.
- `call`: a script call with at least one string literal, as the routine's name and the strings (`CreateObject tar04_rakghoulr`); `site` is `@OFFSET`.
- `source`: a script's NSS source, whole; `site` is empty. Bytecode-only scripts have no source row; their calls are searchable.

The tokenizer splits on punctuation, `_` included, and matches whole tokens: `rakghoul` finds "rakghouls" only as `rakghoul*`, and `tar04_rakghoulr` is the tokens `tar04` and `rakghoulr`.

### Views

- `tree_resources(tree, path, member, name, type, hash, status)`: every location with its tree's label and its content's status.
- `node_text(tree, path, member, hash, node, kind, idx, strref, text)`: every dialogue node at every location, with its shown text: the embedded text, or the line of the `dialog.tlk` of the same tree.

## The digest

`rakata index digest` prints a SHA-256 over what the rows say, never over row ids or the order rows are stored in, so two databases that hold the same facts about a tree have the same digest however they were built; it is how an incremental build is proven equal to a cold one.

The input starts with `rakata-index/v<schema>` and a newline, then `tree` or `all` and a newline, then a fixed list of sections. Each section is the byte `s`, the section's name as an eight-byte big-endian length and its bytes, and its rows, sorted by every column. Each row is the byte `r` and its values; each value is a tag and its bytes: `N` for null, `I` and eight big-endian bytes for an integer, `R` and the eight big-endian bytes of a real's IEEE bits, `T` (text) or `B` (blob), an eight-byte big-endian length and the bytes.

With `--tree LABEL`, the sections are: the tree's `locations` (path, member, name, type, hash) and `coverage` (path, member, status, reason); the `resources` (hash, name, type, size) and `contents` its locations hold; and, for each of `dialog_nodes`, `dialog_links`, `gff_fields`, `cells`, `strings`, `calls`, `resolved_calls`, `script_strings`, `refs` and `search`, every row whose hash its locations hold. The tree's label, parent and root and the resources' `first_tree` are left out: they record how the database was built, not what the tree is. Without `--tree`, the sections are every tree (label, parent's label), every location and coverage row with its tree's label, every resource with its first tree's label, every content, and every row of each fact table.

## Queries

The `k_pman_aioff` call, by argument:

```sql
SELECT DISTINCT l.path, l.member, c.routine, c.args
FROM calls c JOIN locations l ON l.hash = c.hash
WHERE json_extract(c.args, '$[0]') = 'k_pman_aioff';
```

Every script call and dialogue node mentioning rakghouls:

```sql
SELECT DISTINCT l.member, s.site, s.body
FROM search s JOIN locations l ON l.hash = s.hash
WHERE search MATCH 'rakghoul*' AND s.kind = 'call';

SELECT DISTINCT COALESCE(NULLIF(l.member, ''), l.path) AS dialog, n.node, s.body
FROM search s
JOIN dialog_nodes n ON n.strref = CAST(s.site AS INTEGER)
JOIN locations l ON l.hash = n.hash
WHERE search MATCH 'rakghoul*' AND s.kind = 'string';
```

Everything that names a resref, and where:

```sql
SELECT r.kind, r.site, t.label, l.path, l.member
FROM refs r JOIN locations l ON l.hash = r.from_hash JOIN trees t ON t.id = l.tree
WHERE r.kind = 'resref' AND r.value = 'k_pdan_juhani27';
```

The checkpoint in which each of a tree's dialogues first appeared:

```sql
SELECT l.path, l.member, t.label AS first_tree
FROM locations l
JOIN resources r ON r.hash = l.hash AND r.name = l.name AND r.type = l.type
JOIN trees t ON t.id = r.first_tree
WHERE l.tree = (SELECT id FROM trees WHERE label = 'ckpt:ba55d9e0db548e33') AND l.type = 'dlg';
```

## `rakata refs`

`rakata refs THING` reads the `refs` table. With `--to`, the default, it lists what refers to THING; with `--from`, what the resource THING (`name` or `name.ext`) refers to. `--kind auto`, the default, tries every kind THING can be (a strref only when it is a number) and the answer says which matched. Each hit is the resource holding the reference, the site, and every location of that resource, each marked with the copy the game loads instead when there is one: an `Override/` file of the same name and type holding other bytes, or for a member of `modules/STEM.rim` or `STEM_s.rim`, the `STEM.mod` that replaces both. A script's site carries the instruction that pushes the literal, `CONSTS` for a string and `CONSTI` for a strref.

Toward a symbol, indirect hits follow the direct ones, each naming what it goes through, unless `--direct` is given: a resref that names a blueprint (`utc`, `utd`, `ute`, `uti`, `utm`, `utp`, `uts`, `utt`, `utw`) also finds the references to that blueprint's `Tag`, other than objects' own `Tag` fields, since a script finds an object by its tag; and every script among the hits is followed to the references that run it (a dialogue's `Script` and `Active`, an object's `On…` and `Script…` events, `ExecuteScript`). Last come the weaker matches, kept with `--direct` too: every script that holds THING as a string constant (`script_strings`, without regard to case) and has no `refs` row naming it, as when the tag reaches the call through a variable or a function of the script's own. Each is kind `string`, with the offsets of its `CONSTS` instructions as the site and their count (`unresolved` in JSON), and the text lists them under "weaker: a string constant, call not resolved". An empty answer carries RW1505 with the count of resources the index has no reader for or failed to read, since a reference in one of them cannot be found. `--format json` gives the hits as the `result` of the envelope, with `tried`, `matched`, `coverage` (on an empty answer) and `seconds`.

## Diagnostics

The index reports under `RE15xx` and `RW15xx`: RE1501 the database could not be read or written, RE1502 another schema version, RE1503 a tree the database does not hold, RE1504 a query SQLite rejects (the database is opened read-only, so a write is rejected too), RE1505 a changed-paths file that could not be read, RE1506 a `refs` symbol that cannot be of the kind asked for, RE1507 a `refs --from` resource the index does not hold; RW1501 a changed path the tree does not hold, RW1502 a file that could not be read, RW1503 a tree without `nwscript.nss`, RW1504 files new to the tree the changed list does not name, RW1505 a `refs` query that found nothing, with what the index could not search. `rakata explain CODE` gives each in full.
