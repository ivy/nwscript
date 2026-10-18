"""A mod's scripts compiled against the install: includes from the mod's sources first, then the
copy the game loads."""

from typing import ClassVar, Literal

from . import Mod

class Source:
    def __init__(self, name: str, text: bytes | str) -> None:
        """`name` without `.nss`; `str` text is encoded as Windows-1252. A name over sixteen bytes
        raises `RE0101`."""
    @property
    def name(self) -> str: ...
    @property
    def text(self) -> bytes: ...

class Flags:
    BIOWARE: ClassVar[Flags]
    """`--declarations bioware --compat bioware`."""

    def __init__(
        self,
        declarations: Literal["shipped", "bioware"] = "shipped",
        mode: Literal["strict", "nwnnsscomp", "bioware"] = "strict",
    ) -> None: ...

class Compiled:
    @property
    def name(self) -> str: ...
    @property
    def ncs(self) -> bytes: ...
    @property
    def warnings(self) -> str: ...
    @property
    def inputs(self) -> list[dict[str, str]]:
        """`{"role", "path", "sha256"}` for each include and `nwscript.nss` read."""

def compile(
    m: Mod, sources: list[Source], includes: list[Source] = ..., flags: Flags | None = None
) -> list[Compiled]:
    """Nothing is written; RE1408 on the first script with an error."""
