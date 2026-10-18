#!/usr/bin/env python3
"""PreToolUse hook: the clean-room rule, enforced.

Denies any tool call that would read the reference projects AGENTS.md names (PyKotor, HoloPatcher, kpatcher,
TSLPatcher's reconstructed source, nwnnsscomp and the other compilers), for every agent except `format-scribe`,
the one role allowed to read them so it can write a format description under docs/formats/. Reads the hook's JSON
on stdin; prints a JSON deny on stdout when a path or command matches; prints nothing otherwise.
"""

import json
import re
import sys

SCRIBE = "format-scribe"

# Path fragments and command words that name a reference project, matched case-insensitively.
FORBIDDEN = [
    r"pykotor",
    r"holopatcher",
    r"kpatcher",
    r"tslpatcher",
    r"nwn-tools",
    r"neverwinter\.nim",
    r"xoreos-tools",
    r"nwnrs",
    r"holocron",
]
PATTERN = re.compile("|".join(FORBIDDEN), re.IGNORECASE)

# What each tool reads, as the fields of tool_input to inspect.
FIELDS = {
    "Read": ["file_path"],
    "Grep": ["path"],
    "Glob": ["path", "pattern"],
    "Edit": ["file_path"],
    "Write": ["file_path"],
    "Bash": ["command"],
    "NotebookEdit": ["notebook_path"],
}


def deny(reason: str) -> None:
    print(
        json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "deny",
                    "permissionDecisionReason": reason,
                }
            }
        )
    )


def main() -> None:
    try:
        event = json.load(sys.stdin)
    except json.JSONDecodeError:
        return
    if event.get("agent_type") == SCRIBE:
        return
    tool = event.get("tool_name", "")
    tool_input = event.get("tool_input") or {}
    texts: list[str] = []
    if tool.startswith("mcp__codebase-memory__"):
        texts = [json.dumps(tool_input)]
    else:
        texts = [str(tool_input.get(f, "")) for f in FIELDS.get(tool, [])]
    for text in texts:
        hit = PATTERN.search(text)
        if hit:
            deny(
                f"clean room (AGENTS.md): `{hit.group(0)}` names a reference project whose source may not be read. "
                "Permitted: the game's own files, the xoreos documents, the wikis' format pages, docs/formats/, and running "
                "the tool as a black box. Only the `format-scribe` agent may read it, and only to write docs/formats/."
            )
            return


if __name__ == "__main__":
    main()
