"""Fixtures: the user's game install from `$RAKATA_GAME_K1`, a writable install over it, and the
`rakata` binary built from the same checkout as the module."""

import os
import shutil
import subprocess
from pathlib import Path

import pytest

WORKSPACE = Path(__file__).resolve().parents[3]


@pytest.fixture(scope="session")
def game_k1() -> Path:
    """A vanilla KotOR 1 install; the tests that need one skip without it."""
    value = os.environ.get("RAKATA_GAME_K1")
    if not value:
        pytest.skip("RAKATA_GAME_K1 is unset: set it to a vanilla KotOR 1 install (the folder holding chitin.key)")
    return Path(value)


@pytest.fixture
def scratch_install(game_k1: Path, tmp_path: Path) -> Path:
    """An install a test may write to: every entry of the vanilla one linked, except Override/
    and streamwaves/, which are empty folders of its own, and modules/, a folder of its own
    whose files are linked one by one."""
    root = tmp_path / "game"
    root.mkdir()
    for entry in game_k1.iterdir():
        if entry.name.lower() in ("override", "streamwaves"):
            continue
        if entry.name.lower() == "modules":
            (root / entry.name).mkdir()
            for f in entry.iterdir():
                (root / entry.name / f.name).symlink_to(f)
            continue
        (root / entry.name).symlink_to(entry)
    (root / "Override").mkdir()
    (root / "streamwaves").mkdir()
    return root


@pytest.fixture(scope="session")
def rakata_bin() -> Path:
    """The `rakata` binary, built by cargo from this checkout."""
    cargo = shutil.which("cargo")
    if cargo is None:
        pytest.skip("cargo is not on PATH, so the rakata binary cannot be built to compare with")
    subprocess.run([cargo, "build", "--quiet", "-p", "rakata-cli"], cwd=WORKSPACE, check=True)
    target = Path(os.environ.get("CARGO_TARGET_DIR", WORKSPACE / "target"))
    return target / "debug" / "rakata"
