"""Offline cases for scripts/clean-room.py: `python3 -I scripts/clean-room-test.py` from the repository root. Reads the hook, never edits it."""
import json, subprocess, sys

cases = [
    ("DENY", {"tool_name": "Bash", "tool_input": {"command": "cat ~/src/github.com/OpenKotOR/PyKotor/README.md"}}),
    ("DENY", {"tool_name": "Bash", "tool_input": {"command": "cat target/hook-probe/pykotor-probe.txt"}}),
    ("DENY", {"tool_name": "Bash", "tool_input": {"command": "ls PyKotor/"}}),
    ("DENY", {"tool_name": "Bash", "tool_input": {"command": "python -c 'import pykotor'"}}),
    ("DENY", {"tool_name": "Bash", "tool_input": {"command": "uv run python -m pykotor.tools"}}),
    ("DENY", {"tool_name": "Bash", "tool_input": {"command": "cat .venv/lib/python3.12/site-packages/pykotor/resource/formats/rim.py"}}),
    ("DENY", {"tool_name": "Bash", "tool_input": {"command": "type C:\\\\tools\\\\TSLPatcher\\\\x.pas"}}),
    ("allow", {"tool_name": "Bash", "tool_input": {"command": "find ~/src -name x | grep -v pykotor"}}),
    # Known false positive: a sed delimiter looks like a path separator. Edit such a document with a heredoc or the Edit tool.
    ("DENY", {"tool_name": "Bash", "tool_input": {"command": "sed -i 's/PyKotor is measured/x/' docs/prd.md"}}),
    ("allow", {"tool_name": "Bash", "tool_input": {"command": "python3 - <<'EOF'\ns = s.replace('PyKotor is measured', 'x')\nEOF"}}),
    ("allow", {"tool_name": "Bash", "tool_input": {"command": "git commit -m 'docs: PyKotor, HoloPatcher and kpatcher are off limits'"}}),
    ("allow", {"tool_name": "Bash", "tool_input": {"command": "uv run holopatcher --help"}}),
    ("DENY", {"tool_name": "Read", "tool_input": {"file_path": "/home/user/src/github.com/th3w1zard1/kpatcher/src/main.rs"}}),
    ("allow", {"tool_name": "Read", "tool_input": {"file_path": "/home/user/src/github.com/ivy/nwscript/docs/formats/rim.md"}}),
    ("DENY", {"tool_name": "Glob", "tool_input": {"pattern": "**/pykotor/**/*.py"}}),
    ("DENY", {"tool_name": "mcp__codebase-memory__search_graph", "tool_input": {"project": "github-com-OpenKotOR-PyKotor", "query": "rim"}}),
    ("allow", {"tool_name": "mcp__codebase-memory__search_graph", "tool_input": {"project": "github-com-ivy-nwscript", "query": "pykotor mention"}}),
    ("allow", {"tool_name": "Read", "tool_input": {"file_path": "/x/pykotor.py"}, "agent_type": "format-scribe"}),
    ("allow", {"tool_name": "Read", "tool_input": {"file_path": "/home/user/src/github.com/ivy/nwscript/docs/epics/rakata/patches/nwscript/tslpatcher-oracle.md"}, "cwd": "/home/user/src/github.com/ivy/nwscript"}),
    ("allow", {"tool_name": "Bash", "tool_input": {"command": "sed -n 1,200p docs/epics/rakata/patches/nwscript/tslpatcher-oracle.md"}}),
    ("DENY", {"tool_name": "Read", "tool_input": {"file_path": "/home/user/src/github.com/OpenKotOR/PyKotor/docs/README.md"}}),
    ("DENY", {"tool_name": "Bash", "tool_input": {"command": "cat ~/src/github.com/th3w1zard1/kpatcher/docs/x.md"}}),
    ("allow", {"tool_name": "Bash", "tool_input": {"command": "podman run --rm -v $PWD:/mod wine wine /mod/TSLPatcher.exe"}}),
    ("allow", {"tool_name": "Bash", "tool_input": {"command": "sha256sum archives/*/TSLPatcher.exe; ls tslpatchdata/"}}),
    ("DENY", {"tool_name": "Bash", "tool_input": {"command": "cat ~/src/tslpatcher-src/Main.pas"}}),
    ("DENY", {"tool_name": "Read", "tool_input": {"file_path": "/home/user/src/TSLPatcher/Main.pas"}}),
    ("allow", {"tool_name": "Bash", "tool_input": {"command": "git commit -m 'the plan differs from TSLPatcher in three named cases'"}}),
    ("DENY", {"tool_name": "Bash", "tool_input": {"command": "python -c 'from pykotor.resource import gff'"}}),
    ("DENY", {"tool_name": "Bash", "tool_input": {"command": "uv run python -c 'import holopatcher'"}}),
    ("allow", {"tool_name": "Read", "tool_input": {"file_path": "/home/user/src/github.com/ivy/modshop/games/kotor/data/logs/holopatcher/006-k1cp.log"}}),
    ("allow", {"tool_name": "Bash", "tool_input": {"command": "tail -5 games/kotor/data/logs/holopatcher/006-k1cp.log"}}),
    ("DENY", {"tool_name": "Read", "tool_input": {"file_path": "/home/user/src/github.com/th3w1zard1/kpatcher/src/logs.rs"}}),
    ("allow", {"tool_name": "Write", "tool_input": {"file_path": "conformance/drivers/tslpatcher.sh", "content": "x"}}),
    ("allow", {"tool_name": "Write", "tool_input": {"file_path": "/home/user/src/github.com/ivy/nwscript/conformance/drivers/tslpatcher.sh", "content": "x"}, "cwd": "/home/user/src/github.com/ivy/nwscript"}),
    ("allow", {"tool_name": "Bash", "tool_input": {"command": "bash conformance/drivers/tslpatcher.sh data game 1"}}),
    ("DENY", {"tool_name": "Bash", "tool_input": {"command": "cat /other/conformance/../PyKotor/x.py; cat ~/x/pykotor/docs/y"}}),
    ("DENY", {"tool_name": "Read", "tool_input": {"file_path": "/home/user/src/github.com/OpenKotOR/PyKotor/conformance/x.py"}}),
]
bad = 0
for want, ev in cases:
    out = subprocess.run([sys.executable, "-I", "scripts/clean-room.py"], input=json.dumps(ev), capture_output=True, text=True).stdout
    got = "DENY" if "deny" in out else "allow"
    flag = "" if got == want else "  <-- WRONG"
    bad += got != want
    print(f"{got:5} want {want:5}{flag} :: {json.dumps(ev['tool_input'])[:80]}")
print("failures:", bad)
