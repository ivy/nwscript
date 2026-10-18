"""The scripts toolchain: `nwscript-py`'s functions (`import nwscript`), mounted here unchanged.
Results are plain dicts, lists, bytes and strings; a bad argument raises `ValueError`."""

import os
from typing import Literal, TypeAlias

__version__: str

StrPath: TypeAlias = str | os.PathLike[str]
Game: TypeAlias = Literal["k1", "k2"]
Mode: TypeAlias = Literal["strict", "nwnnsscomp", "bioware"]
Declarations: TypeAlias = Literal["shipped", "bioware"]

def check(
    source: bytes,
    name: str = "script.nss",
    include_dirs: list[StrPath] = ...,
    nwscript: StrPath | None = None,
    game: Game = "k1",
    mode: Mode = "strict",
    declarations: Declarations = "shipped",
) -> dict[str, object]:
    """`{"ok", "diagnostics", "inputs"}`."""

def compile(
    source: bytes,
    name: str = "script.nss",
    include_dirs: list[StrPath] = ...,
    nwscript: StrPath | None = None,
    game: Game = "k1",
    mode: Mode = "strict",
    declarations: Declarations = "shipped",
) -> dict[str, object]:
    """`{"ok", "ncs" (bytes or None), "diagnostics", "inputs"}`."""

def disassemble(ncs: bytes, nwscript: StrPath | None = None) -> str: ...
def assemble(text: str) -> bytes: ...
def run(
    ncs: bytes,
    nwscript: StrPath,
    engine: Literal["recording", "world"] = "recording",
    seed: int = 0,
) -> dict[str, object]:
    """`{"result", "trace", "unmodelled", "fault"}`."""

def diff(a: bytes, b: bytes, nwscript: StrPath | None = None) -> dict[str, object]:
    """`{"bytes", "instructions"}`, and `"trace"` with `nwscript`."""
