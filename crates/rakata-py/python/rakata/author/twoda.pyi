"""Rows added to a game table by copying another. A label is the `label` column's where the table
has one, the row label otherwise."""

from .. import Row, TwoDa

def find(table: TwoDa, key: Row) -> int:
    """RE0705 when no row matches, RE1405 when a label matches several."""

def add_row_copying(
    table: TwoDa, source: Row, new_label: str, cells: dict[str, str] | None = None
) -> int:
    """RE1405 when a row already carries `new_label`; RE0706 for a missing column."""
