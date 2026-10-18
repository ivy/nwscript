"""Additions to a dialogue by stable key, keeping every index. Text is `None` (no line), an `int`
(the game's line by strref) or a `str` (the mod's own, in English)."""

from .. import Dialog, Key, Kind, NodeRef

NO_DELAY: int
CAMERA_STATIC: int

def add_entry(
    dialog: Dialog,
    text: int | str | None = None,
    *,
    speaker: str = "",
    listener: str = "",
    voice: str | None = None,
    script: str | None = None,
    animations: list[tuple[str, int]] = ...,
    quest: tuple[str, int] | None = None,
    camera: int | None = None,
    comment: str = "",
) -> NodeRef: ...
def add_reply(
    dialog: Dialog,
    text: int | str | None = None,
    *,
    listener: str = "",
    voice: str | None = None,
    script: str | None = None,
    comment: str = "",
) -> NodeRef: ...
def link(
    dialog: Dialog, source: Key, to: Key, condition: str | None = None, at: int | None = None
) -> int:
    """The link's position; RE1102, RE1103 for a key, RE1106 for a link that cannot be made."""

def replace_script(dialog: Dialog, at: Key, old: str | None, new: str | None) -> None:
    """The node `at` runs `new` (`None` for no script) instead of `old`, names compared without
    case; RE1416, with the dialogue unchanged, when it does not run `old`."""

def unlink_all(dialog: Dialog, source: Key) -> None: ...
def start(dialog: Dialog, to: Key, condition: str | None = None, at: int | None = None) -> int:
    """At `at` in the starting list, or first."""

def reorder(dialog: Dialog, kind: Kind, order: list[int]) -> None:
    """RE1107 unless `order` is a permutation of the list."""

def empty_like(like: Dialog, name: str) -> Dialog: ...
