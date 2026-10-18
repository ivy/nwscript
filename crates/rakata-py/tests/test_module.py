"""The module without a game install: import, build identity, the formats and the authoring calls
that work on values."""

import json
import subprocess
import sys
from pathlib import Path

import pytest

import rakata
import rakata.nwscript
from rakata.author import blueprint, dialog, globals, lip, scripts, twoda, voice


def test_imports_from_an_isolated_interpreter() -> None:
    r = subprocess.run(
        [sys.executable, "-I", "-c", "import rakata, rakata.nwscript, rakata.author.voice"],
        capture_output=True,
        text=True,
    )
    assert r.returncode == 0, r.stderr


def test_version_is_the_binarys_build_identity(rakata_bin: Path) -> None:
    out = subprocess.run([rakata_bin, "--version"], capture_output=True, text=True, check=True).stdout
    assert out.strip() == f"rakata {rakata.__version__}"
    assert rakata.VERSION == rakata.__version__


def test_nwscript_is_mounted() -> None:
    out = rakata.nwscript.check(b"void main() {}")
    assert set(out) == {"ok", "diagnostics", "inputs"}
    assert rakata.nwscript.__version__


def test_gff_get_and_set_keep_types() -> None:
    gff = rakata.Gff.new("UTC")
    assert gff.file_type == "UTC"
    assert gff.set("Str", 16, type="BYTE") is None
    assert gff.set("Str", 8) == 16
    assert gff.get("Str") == 8
    assert gff.field_type("Str") == "BYTE"
    gff.set("Tag", "oaj_juhani1", type="CExoString")
    copy = blueprint.clone(gff, "oaj_juhani1")
    assert copy.get("TemplateResRef") == "oaj_juhani1"
    back = rakata.Gff.parse(copy.to_bytes())
    assert back.get("Tag") == "oaj_juhani1"
    with pytest.raises(rakata.RakataError) as e:
        gff.set("Str", 3, type="WORD")
    assert e.value.code == "RE1406"
    assert str(e.value) == f"RE1406: {e.value.message}"
    with pytest.raises(rakata.RakataError) as e:
        gff.get("NoSuchField")
    assert e.value.code == "RE0607"


def test_blueprint_set_each_takes_the_elements_type() -> None:
    gff = rakata.Gff.new("UTC")
    assert blueprint.set_each(gff, "ItemList", "Dropable", 0) == 0


def test_twoda_rows_and_globals() -> None:
    table = rakata.TwoDa.new(["name", "type"])
    assert globals.declare(table, "OAJ_DONE", "Boolean") == 0
    with pytest.raises(rakata.RakataError) as e:
        globals.declare(table, "oaj_done")
    assert e.value.code == "RE1404"
    appearance = rakata.TwoDa.new(["label", "race", "modeltype"])
    appearance.add_row("0", {"label": "Creature_Brith", "race": "c_brith", "modeltype": "S"})
    i = twoda.add_row_copying(appearance, "Creature_Brith", "Juhani_Clones_Hidden", {"race": "PLC_Invis"})
    assert i == 1
    assert appearance.get(i, "race") == "PLC_Invis"
    assert appearance.get(i, "modeltype") == "S"
    assert twoda.find(appearance, "Juhani_Clones_Hidden") == 1
    back = rakata.TwoDa.parse(appearance.to_bytes())
    assert back.rows == appearance.rows
    with pytest.raises(rakata.RakataError) as e:
        twoda.add_row_copying(appearance, "Creature_Brith", "Juhani_Clones_Hidden")
    assert e.value.code == "RE1405"


def test_lip_from_cues() -> None:
    a = lip.from_cues([(0.0, "m"), (0.2, "ah")], 0.5)
    assert a.keys[-1] == (0.5, "rest")
    assert rakata.Lip.parse(a.to_bytes()).keys == a.keys
    assert lip.rhubarb_shape("A") == "m"
    assert lip.rhubarb_shape("Z") is None


def test_voice_spans_and_cues() -> None:
    transcript = json.dumps(
        {"results": {"audio_segments": [{"start_time": "0.5", "end_time": "1.0"}, {"start_time": "1.2", "end_time": "2.0"}]}}
    )
    take = voice.Take.from_transcript("juhani_a", b"audio", transcript)
    lines = [voice.Line("oaj_ju01", "juhani_a", "Hello.", 1)]
    (span,) = voice.spans(lines, [take])
    assert span.start_text() == "1.050"
    assert span.end_text() == "2.150"
    assert span.resref == "oaj_ju01" and len(span.key) == 64
    with pytest.raises(rakata.RakataError) as e:
        voice.spans(lines + lines, [take])
    assert e.value.code == "RE1411"
    cues = voice.take_cues(json.dumps({"metadata": {"duration": 1.0}, "mouthCues": [{"start": 0.0, "value": "X"}]}))
    assert cues.length == 1.0
    wav = voice.streamwave(b"mp3")
    assert wav[:4] == b"RIFF" and wav.endswith(b"mp3") and len(voice.WAV_HEADER) == 58


def test_dialog_additions_by_key(rakata_bin: Path) -> None:
    d = rakata.Dialog.new("oaj_test.dlg", rakata.Gff.new("DLG"))
    hello = dialog.add_entry(d, "Hello.", speaker="Juhani", voice="oaj_ju01")
    bye = dialog.add_reply(d)
    assert str(hello) == "E0" and str(bye) == "R0"
    assert dialog.link(d, hello, bye) == 0
    assert dialog.start(d, rakata.NodeKey.voice("OAJ_JU01")) == 0
    node = d.node(hello)
    assert node is not None and node.voice == "oaj_ju01" and node.links[0].target == bye
    assert rakata.NodeKey.of(d, hello).resolve(d) == hello
    with pytest.raises(rakata.RakataError) as e:
        dialog.link(d, rakata.NodeKey.text("No such line."), bye)
    assert e.value.code == "RE1102"
    # The code is the CLI's: its registry explains it.
    out = subprocess.run([rakata_bin, "--format", "json", "explain", e.value.code], capture_output=True, text=True)
    assert out.returncode == 0 and e.value.code in out.stdout
    with pytest.raises(rakata.RakataError) as e:
        rakata.NodeKey.parse("Q7")
    assert e.value.code == "RE1104"
    empty = dialog.empty_like(d, "oaj_banter.dlg")
    assert empty.len("entry") == 0 and d.len("entry") == 1


def test_a_script_name_over_sixteen_bytes_is_refused_at_construction() -> None:
    source = scripts.Source("k_oaj_sixteen_01", "void main() { }")
    assert source.name == "k_oaj_sixteen_01"
    assert source.text == b"void main() { }"
    with pytest.raises(rakata.RakataError) as e:
        scripts.Source("k_oaj_seventeen01", "void main() { }")
    assert e.value.code == "RE0101"
    assert e.value.message == "resource name `k_oaj_seventeen01` is 17 bytes; a resref holds at most 16"
