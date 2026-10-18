"""The authoring API: an install opened for a mod's writes (`Mod`), each write checked before it
lands, and the edits by submodule as in `rakata::author`."""

from .. import Dialog, Finding, Install, Key, NodeRef
from . import blueprint as blueprint
from . import dialog as dialog
from . import globals as globals
from . import lip as lip
from . import scripts as scripts
from . import twoda as twoda
from . import voice as voice

class Loaded:
    """A copy of a resource as the game would load it."""

    @property
    def bytes(self) -> bytes: ...
    @property
    def location(self) -> str:
        """Relative to the install: `Override/x.2da`, `modules/danm14ac_s.rim:x.ncs`."""
    @property
    def written(self) -> bool:
        """Whether the copy is one this mod wrote."""

class Written:
    """A file the mod wrote."""

    @property
    def path(self) -> str: ...
    @property
    def hash(self) -> str: ...
    @property
    def len(self) -> int: ...
    @property
    def warnings(self) -> list[Finding]:
        """For a dialogue, references its base copy already lacked."""

class Mod:
    """An install opened for a mod's writes, in one module."""

    @staticmethod
    def open(install: Install, module: str | None = None) -> Mod:
        """RE0403 when the install has no capsule of `module`."""
    @property
    def install(self) -> Install: ...
    @property
    def module(self) -> str | None: ...
    def loaded(self, name: str) -> Loaded:
        """`NAME.EXT` as the game loads it, this mod's writes first; RE0404 when no layer holds
        it."""
    def guard_replaced(self, name: str, sha256: str) -> None:
        """RE1401 (a module copy differs), RE1402 (Override holds one), RE1403 (none in the
        module)."""
    def find(self, dialog: Dialog, key: Key) -> NodeRef:
        """Text keys compared through the install's talk table too."""
    def check_dialog(self, dialog: Dialog) -> list[Finding]: ...
    def write_override(self, name: str, data: bytes) -> Written:
        """Read back as its type first; a dialogue whose edit names what the install lacks is
        refused with RE1407."""
    def write_streamwaves(self, resref: str, data: bytes) -> Written:
        """RE1413 for bytes that are not RIFF WAVE."""
