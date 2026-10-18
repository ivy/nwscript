"""Globals: rows of `globalcat.2da`."""

from typing import Literal

from .. import TwoDa

def declare(table: TwoDa, name: str, type: Literal["Boolean", "Number"] = "Boolean") -> int:
    """The row's index; RE1404 when the table already declares `name` in any case."""
