"""Lip-sync files made from a tool's mouth cues."""

from .. import Lip

def from_cues(cues: list[tuple[float, str]], length: float) -> Lip: ...
def rhubarb_shape(letter: str) -> str | None:
    """The game's shape label for Rhubarb's `A`-`H` or `X`; `None` for another letter."""
