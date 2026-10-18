#!/usr/bin/env python3
"""Measure which 2DA tables and columns resolve an item's properties.

Reads an index of a vanilla KotOR 1 install (`rakata index build --game
"$RAKATA_GAME_K1" --tree vanilla --db vanilla.sqlite`) and writes the table
`rakata show` reads, `crates/rakata/src/show/items.toml`, to stdout:

    python3 scripts/measure-item-properties.py vanilla.sqlite > crates/rakata/src/show/items.toml

Every property of every vanilla .uti (`PropertiesList[i]`: PropertyName,
Subtype, CostTable, CostValue, Param1, Param1Value) is tried against the
tables, and each column is chosen as the one that resolves the most of them;
the counts go in the file's header. The tables named up front
(`itempropdef`, `iprp_costtable`, `iprp_paramtable`, `baseitems`) are the
install's item-property 2DAs by name; the counts confirm each.
"""

import collections
import re
import sys
import textwrap

import sqlite3

PROPERTIES = "itempropdef"
COSTS = "iprp_costtable"
PARAMS = "iprp_paramtable"
BASE_ITEMS = "baseitems"
NONE = 255  # Param1 that names no parameter table, measured below
LIST = re.compile(r"PropertiesList\[(\d+)\]\.(\w+)$")


def main(db: str) -> None:
    con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    (tree,) = con.execute("SELECT id FROM trees ORDER BY id LIMIT 1").fetchone()
    text = {s: t for s, t in con.execute("SELECT strref, text FROM strings")}

    # Each 2DA by lowercase name, the BIF's copy first (the engine's last
    # layer, the one every vanilla capsule repeats).
    tables: dict[str, dict[int, dict[str, str]]] = {}
    for name, h in con.execute(
        """SELECT DISTINCT name, hash FROM locations WHERE tree = ? AND type = '2da'
           ORDER BY name, path NOT LIKE 'data/%', path""",
        (tree,),
    ):
        name = name.lower()
        if name in tables:
            continue
        rows: dict[int, dict[str, str]] = collections.defaultdict(dict)
        for row, column, value in con.execute("SELECT row, column, value FROM cells WHERE hash = ?", (h,)):
            rows[row][column] = value
        tables[name] = dict(rows)

    def cell(table: str, row: int, column: str):
        v = tables.get(table.lower(), {}).get(row, {}).get(column)
        return None if v in (None, "****") else v

    def has_row(table, row) -> bool:
        return table is not None and row in tables.get(table.lower(), {})

    props = collections.defaultdict(dict)
    items = set()
    for h, path, value in con.execute(
        """SELECT DISTINCT g.hash, g.path, g.value FROM gff_fields g
           JOIN locations l ON l.hash = g.hash
           WHERE l.tree = ? AND l.type = 'uti' AND g.path LIKE 'PropertiesList[%'""",
        (tree,),
    ):
        m = LIST.match(path)
        if m and value is not None:
            props[(h, int(m[1]))][m[2]] = int(value)
            items.add(h)
    props = [p for p in props.values() if "PropertyName" in p]
    n = len(props)

    defined = [p for p in props if has_row(PROPERTIES, p["PropertyName"])]
    columns = sorted({c for r in tables[PROPERTIES].values() for c in r})

    def best(scores: dict[str, tuple[int, int]]) -> str:
        return max(sorted(scores), key=lambda c: (scores[c][0], -scores[c][1]))

    # The subtype table: the itempropdef column naming the 2DA whose row the
    # property's Subtype is.
    sub = {}
    for c in columns:
        named = [p for p in defined if cell(PROPERTIES, p["PropertyName"], c)]
        hit = sum(has_row(cell(PROPERTIES, p["PropertyName"], c), p["Subtype"]) for p in named)
        sub[c] = (hit, len(named))
    subtype = best(sub)

    # The cost table: iprp_costtable's column naming the 2DA whose row the
    # CostValue is, for the property's own CostTable; and the itempropdef
    # column that agrees with that CostTable.
    cost_cols = sorted({c for r in tables[COSTS].values() for c in r})
    cost = {}
    for c in cost_cols:
        hit = sum(has_row(cell(COSTS, p["CostTable"], c), p["CostValue"]) for p in defined)
        cost[c] = (hit, len(defined))
    cost_resref = best(cost)
    # A cost table without rows (iprp_base1) is a property with no value.
    rowless = [p for p in defined if not tables.get((cell(COSTS, p["CostTable"], cost_resref) or "").lower())]
    rowless_tables = sorted({cell(COSTS, p["CostTable"], cost_resref).lower() for p in rowless})
    agree = {}
    for c in columns:
        agree[c] = (sum(cell(PROPERTIES, p["PropertyName"], c) == str(p["CostTable"]) for p in defined), len(defined))
    cost_column = best(agree)

    # The parameter table, for a Param1 other than NONE.
    with_param = [p for p in defined if p.get("Param1", NONE) != NONE]
    param_cols = sorted({c for r in tables[PARAMS].values() for c in r})
    param = {}
    for c in param_cols:
        hit = sum(has_row(cell(PARAMS, p["Param1"], c), p.get("Param1Value", 0)) for p in with_param)
        param[c] = (hit, len(with_param))
    param_resref = best(param)

    # A value's name: the column of the value tables whose cell is a strref
    # with text in the most rows used; the fallback, the most often filled
    # column that is not.
    used = []  # (table, row)
    for p in defined:
        t = cell(PROPERTIES, p["PropertyName"], subtype)
        if has_row(t, p["Subtype"]):
            used.append((t, p["Subtype"]))
        t = cell(COSTS, p["CostTable"], cost_resref)
        if has_row(t, p["CostValue"]):
            used.append((t, p["CostValue"]))
    named = collections.Counter()
    filled = collections.Counter()
    for t, r in used:
        for c, v in tables[t.lower()][r].items():
            if v == "****":
                continue
            filled[c] += 1
            if re.fullmatch(r"\d+", v) and text.get(int(v), "").strip():
                named[c] += 1
    value_name = max(sorted(named), key=lambda c: named[c])
    # The fallback: the column filled in the most rows used whose cells
    # are not strrefs.
    value_label = max(sorted(c for c in filled if c != value_name), key=lambda c: filled[c] - named[c])

    prop_named = sum(bool(text.get(int(cell(PROPERTIES, p["PropertyName"], value_name) or -1), "").strip()) for p in defined)

    base = collections.Counter()
    for (b,) in con.execute(
        """SELECT DISTINCT g.hash || ':' || g.value FROM gff_fields g JOIN locations l ON l.hash = g.hash
           WHERE l.tree = ? AND l.type = 'uti' AND g.path = 'BaseItem'""",
        (tree,),
    ):
        row = int(b.split(":")[1])
        base["items"] += 1
        base["rows"] += has_row(BASE_ITEMS, row)
        v = cell(BASE_ITEMS, row, value_name)
        base["named"] += bool(v and re.fullmatch(r"\d+", v) and text.get(int(v), "").strip())

    nones = sum(p.get("Param1", NONE) == NONE for p in defined)

    print(
        comment(HEADER.format(
            items=len(items),
            n=n,
            defined=len(defined),
            prop_named=prop_named,
            subtype=subtype,
            sub_hit=sub[subtype][0],
            sub_of=sub[subtype][1],
            cost_resref=cost_resref,
            cost_hit=cost[cost_resref][0],
            rowless=len(rowless),
            rowless_tables=", ".join(rowless_tables),
            with_cost=len(defined) - len(rowless),
            cost_column=cost_column,
            agree=agree[cost_column][0],
            nones=nones,
            param_resref=param_resref,
            param_hit=param[param_resref][0],
            param_of=param[param_resref][1],
            used=len(used),
            value_name=value_name,
            named=named[value_name],
            value_label=value_label,
            labelled=filled[value_label],
            base_items=base["items"],
            base_rows=base["rows"],
            base_named=base["named"],
        ))
    )
    print()
    print("[properties]")
    print(f'table = "{PROPERTIES}"')
    print(f'subtype = "{subtype}"')
    print(f'cost_table = "{cost_column}"')
    print()
    print("[costs]")
    print(f'table = "{COSTS}"')
    print(f'resref = "{cost_resref}"')
    print()
    print("[params]")
    print(f'table = "{PARAMS}"')
    print(f'resref = "{param_resref}"')
    print(f"none = {NONE}")
    print()
    print("[values]")
    print(f'name = "{value_name}"')
    print(f'label = "{value_label}"')
    print()
    print("[base_items]")
    print(f'table = "{BASE_ITEMS}"')


# One paragraph or item per line, wrapped as the file is written.
HEADER = """
The 2DA tables and columns that resolve an item's properties and base item, for `rakata show` (crates/rakata/src/show.rs).

Generated by scripts/measure-item-properties.py over an index of a vanilla KotOR 1 install; no game bytes are recorded, only table and column names and the counts below. Regenerate rather than edit.

Method. Every property of every vanilla .uti ({n} properties on the {items} distinct .uti that have one) is tried against the tables; each column is the one that resolves the most of them.

- PropertyName is a row of itempropdef for {defined} of {n}; the row's `{value_name}` names text for {prop_named}.
- The subtype table is the itempropdef column `{subtype}`: of the {sub_of} properties whose row names one, the Subtype is a row of it for {sub_hit}.
- The cost table is the 2DA that iprp_costtable's column `{cost_resref}` names in the property's own CostTable row. For {rowless} properties that 2DA has no rows ({rowless_tables}): the property has no value. For the other {with_cost}, the CostValue is a row of it for {cost_hit}. itempropdef's `{cost_column}` equals the CostTable for {agree} of {defined}; the property's own field is read.
- Param1 is {none_value} (no parameter) for {nones} of {defined}; for the other {param_of}, the parameter table is the 2DA that iprp_paramtable's column `{param_resref}` names, and Param1Value is a row of it for {param_hit}.
- A value's name is the column `{value_name}`, a strref with text in {named} of the {used} subtype and cost rows used; `{value_label}` (filled in {labelled} of them) is the fallback where it has none.
- BaseItem is a row of baseitems for {base_rows} of the {base_items} distinct .uti; the row's `{value_name}` names text for {base_named}.
"""


def comment(text: str) -> str:
    out = []
    for line in text.strip().split("\n"):
        if not line:
            out.append("#")
            continue
        bullet = line.startswith("- ")
        out.extend(
            textwrap.wrap(line, 76, initial_indent="# ", subsequent_indent="#   " if bullet else "# ")
        )
    return "\n".join(out)


if __name__ == "__main__":
    HEADER = HEADER.replace("{none_value}", str(NONE))
    main(sys.argv[1])
