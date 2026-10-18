#!/usr/bin/env python3
"""Measure which resource type each resref site of a vanilla install names.

Reads an index of a vanilla KotOR 1 install (`rakata index build --game
"$RAKATA_GAME_K1" --tree vanilla --db vanilla.sqlite`) and writes the table
`rakata lint` reads, `crates/rakata/src/lint/sites.toml`, to stdout:

    python3 scripts/measure-resref-sites.py vanilla.sqlite > crates/rakata/src/lint/sites.toml

A name "resolves as" a type when the tree holds a resource of that name and
type: a location of the index, or a file outside the folders it reads
(`streamwaves/`, `streammusic/`, `streamsounds/`, `movies/`) by its name.
"""

import collections
import os
import re
import sqlite3
import sys

RATIO = 0.8  # of a site's values that must resolve as its type
DISTINCT = 3  # distinct values that must resolve as it
SKIP = {"mdx", "nss"}  # the engine loads a model's mdl and a script's ncs
CAPSULE = {"rim", "mod"}  # a capsule named as a value is a module
STRING_COLUMN = re.compile(r"strref|name|desc|tip|title|text|tlk|string", re.I)
STRREF_TEXT = 0.6  # of a string column's cells (other than -1) that name a line with text
STRREF_MAX = 100  # the largest value at least this, so a small enumeration is not taken for strrefs


def main(db: str) -> None:
    con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    (tree,) = con.execute("SELECT id FROM trees ORDER BY id LIMIT 1").fetchone()
    names: dict[str, set[str]] = collections.defaultdict(set)
    for n, t in con.execute("SELECT DISTINCT name, type FROM locations WHERE tree = ?", (tree,)):
        names[n].add(t)
    for (p,) in con.execute("SELECT path FROM coverage WHERE tree = ? AND status = 'outside'", (tree,)):
        base = os.path.basename(p).lower()
        if "." in base:
            n, t = base.rsplit(".", 1)
            names[n].add(t)

    def composed(vs, t):
        # A texture prefix the engine completes with a variation number
        # (appearance.2da's texa..texd: `N_CommFC` names N_CommFC01.tpc),
        # which resolves as a model only where a model shares the name: the
        # values that do not resolve as the type resolve with `01` as tpc.
        rest = {v for v in vs if t not in names.get(v, ())}
        return len(rest) >= DISTINCT and sum("tpc" in names.get(v + "01", ()) for v in rest) / len(rest) >= RATIO

    values: dict = collections.defaultdict(list)

    def typed(rows):
        st = collections.defaultdict(lambda: [0, collections.defaultdict(set), collections.Counter()])
        for key, v in rows:
            values[key].append(v)
            s = st[key]
            s[0] += 1
            for t in names.get(v, ()):
                s[1][t].add(v)
                s[2][t] += 1
        out = {}
        for key, (n, distinct, count) in sorted(st.items()):
            for t, c in sorted(count.items(), key=lambda tc: (-tc[1], tc[0])):
                if t in SKIP:
                    continue
                if c / n >= RATIO and len(distinct[t]) >= DISTINCT and not composed(values[key], t):
                    out[key] = "module" if t in CAPSULE else t
                break
        return out

    gff_rows = (
        ((ty, site.rsplit(".", 1)[-1]), v)
        for ty, site, v in con.execute(
            """SELECT DISTINCT l.type, r.site, r.value FROM refs r
               JOIN (SELECT DISTINCT hash, type FROM locations WHERE tree = ?) l ON l.hash = r.from_hash
               WHERE r.kind = 'resref' AND r.site NOT LIKE '@%' AND l.type NOT IN ('tlk', 'ncs')""",
            (tree,),
        )
    )
    gff = typed(gff_rows)

    cells = list(
        con.execute(
            """SELECT l.name, c.column, c.value FROM cells c
               JOIN (SELECT DISTINCT hash, name FROM locations WHERE tree = ? AND type = '2da') l
               ON l.hash = c.hash WHERE c.value != '****'""",
            (tree,),
        )
    )
    twoda = typed(((t, c), v.lower()) for t, c, v in cells if not re.fullmatch(r"-?[\d.]+", v))

    text = {s: t for s, t in con.execute("SELECT strref, text FROM strings")}
    cols = collections.defaultdict(lambda: [0, 0, 0, -1])  # cells, integers, with text, max
    for t, c, v in cells:
        s = cols[(t, c)]
        s[0] += 1
        if re.fullmatch(r"-?\d+", v):
            s[1] += 1
            i = int(v)
            s[3] = max(s[3], i)
            if i == -1:
                s[0] -= 1
                s[1] -= 1
            elif text.get(i, "").strip():
                s[2] += 1
    strref = sorted(
        k
        for k, (n, ints, with_text, top) in cols.items()
        if STRING_COLUMN.search(k[1]) and n and ints == n and with_text / n >= STRREF_TEXT and top >= STRREF_MAX
    )

    print(HEADER.strip())
    print()
    print("[gff]")
    for (ty, label), t in sorted(gff.items()):
        print(f'"{ty}.{label}" = "{t}"')
    print()
    print("[twoda]")
    for (table, column), t in sorted(twoda.items()):
        print(f'"{table}.{column}" = "{t}"')
    print()
    print("[strref]")
    print("columns = [")
    for table, column in strref:
        print(f'  "{table}.{column}",')
    print("]")


HEADER = """
# Which resource type each resref site names, and which 2DA columns hold
# strrefs, for `rakata lint` (docs/lints.md, "The tree lint").
#
# Generated by scripts/measure-resref-sites.py over an index of a vanilla
# KotOR 1 install; no game bytes are recorded, only field labels, table and
# column names and type names. Regenerate rather than edit.
#
# Method. A value resolves as a type when the install holds a resource of
# that name and type. A site is a GFF field label within a file type
# ("utc.Conversation") or a 2DA column within a table ("appearance.modela",
# numeric cells left out). A site is typed T when at least 80% of its vanilla
# values resolve as T and at least three distinct values do; mdx and nss are
# never chosen (the engine loads the mdl and the ncs), and a capsule (rim,
# mod) is recorded as "module", a module's name. A column is left out when
# at least three of its values do not resolve as its type and 80% of those
# name a texture with `01` appended: a texture prefix the engine completes
# (appearance.2da's texa to texd). A 2DA column not listed is
# not a resref column; a GFF CResRef field not listed resolves as any type.
#
# A strref column is a 2DA column whose name speaks of a string (strref,
# name, desc, tip, title, text, tlk, string), whose every cell is an
# integer, at least 60% of whose cells other than -1 name a talk-table line
# with text, and whose largest value is at least 100.
"""

if __name__ == "__main__":
    main(sys.argv[1])
