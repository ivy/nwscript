"""The module over the user's KotOR 1 install (`$RAKATA_GAME_K1`): resolution against the binary,
and the authoring API writing into a scratch install over it."""

import json
import subprocess
from pathlib import Path

import pytest

import rakata
from rakata.author import blueprint, dialog, scripts

# k_pdan_juhani31.ncs as danm14ac_s.rim ships it
JUHANI31_SHA256 = "f684480ac7b9ace19d012f2107eb1f28f916d2f7d349965fe0185559ec44ef28"


def test_which_matches_the_binary(game_k1: Path, rakata_bin: Path) -> None:
    ours = rakata.Install(game_k1).which("globalcat", "2da")
    out = subprocess.run(
        [rakata_bin, "--format", "json", "which", "globalcat.2da", "--game", game_k1],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    theirs = json.loads(out)["result"]
    assert ours.hash == theirs["winner"]["sha256"]
    assert [c.hash for c in ours.candidates] == [c["sha256"] for c in theirs["candidates"]]
    assert ours.winner.layer == theirs["winner"]["layer"]


def test_which_names_what_no_layer_holds(game_k1: Path) -> None:
    with pytest.raises(rakata.RakataError) as e:
        rakata.Install(game_k1).which("no_such_thing.2da")
    assert e.value.code == "RE0404"


def test_mod_guards_reads_and_refuses(scratch_install: Path) -> None:
    m = rakata.Mod.open(rakata.Install(scratch_install), "danm14ac")
    assert m.module == "danm14ac"
    m.guard_replaced("k_pdan_juhani31.ncs", JUHANI31_SHA256)
    with pytest.raises(rakata.RakataError) as e:
        m.guard_replaced("k_pdan_juhani31.ncs", "0" * 64)
    assert e.value.code == "RE1401"

    juhani = rakata.Gff.parse(m.loaded("dan14_juhani.utc").bytes)
    clone = blueprint.clone(juhani, "oaj_juhani1")
    clone.set("Tag", "oaj_juhani1")
    hp = juhani.get("HitPoints")
    assert isinstance(hp, int)
    clone.set("HitPoints", max(1, hp // 2))
    written = m.write_override("oaj_juhani1.utc", clone.to_bytes())
    assert Path(written.path).parent.name == "Override"
    again = m.loaded("oaj_juhani1.utc")
    assert again.written and rakata.Gff.parse(again.bytes).get("Tag") == "oaj_juhani1"

    name = "dan14_juhani.dlg"
    d = rakata.Dialog.parse(name, m.loaded(name).bytes)
    line = dialog.add_entry(d, "Oops.", script="k_oaj_missing")
    dialog.start(d, line)
    with pytest.raises(rakata.RakataError) as e:
        m.write_override(name, d.to_bytes())
    assert e.value.code == "RE1407"
    assert not (scratch_install / "Override" / name).exists()
    with pytest.raises(rakata.RakataError) as e:
        m.find(d, rakata.NodeKey.text("No line of Juhani's says this."))
    assert e.value.code == "RE1102"


def test_scripts_compile_against_the_install(scratch_install: Path) -> None:
    m = rakata.Mod.open(rakata.Install(scratch_install), "danm14ac")
    src = scripts.Source("k_oaj_test", 'void main() { PrintString("oops"); }')
    (out,) = scripts.compile(m, [src])
    assert out.name == "k_oaj_test" and out.ncs[:8] == b"NCS V1.0"
    assert any(i["role"] == "nwscript" for i in out.inputs)
    m.write_override("k_oaj_test.ncs", out.ncs)
    with pytest.raises(rakata.RakataError) as e:
        scripts.compile(m, [scripts.Source("k_oaj_bad", "void main() { Nope(); }")])
    assert e.value.code == "RE1408"
