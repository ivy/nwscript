"""Blueprints edited by field path; a field keeps its type."""

from .. import Gff, GffValue

def get(gff: Gff, path: str) -> GffValue: ...
def set(gff: Gff, path: str, value: object, type: str | None = None) -> GffValue | None:
    """What the field held, or `None` for a field added; RE1406 for another type."""

def set_each(gff: Gff, list: str, label: str, value: object, type: str | None = None) -> int:
    """`label` set in every element of the list at `list`; how many there are (0 when `list`
    names no list)."""

def clone(source: Gff, template: str) -> Gff:
    """A copy whose `TemplateResRef` is `template`."""
