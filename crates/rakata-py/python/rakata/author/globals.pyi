"""Globals: rows of `globalcat.2da`."""

from typing import Literal

from .. import TwoDa

def declare(table: TwoDa, name: str, type: Literal["Boolean", "Number"] = "Boolean") -> int:
    """The row's index; RE1404 when the table already declares `name` in any case."""

def type_of(table: TwoDa, name: str) -> Literal["Boolean", "Number"]:
    """The type the table declares `name` with, in any case; RE1417 when no row declares it."""
