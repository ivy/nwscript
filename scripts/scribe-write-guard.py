#!/usr/bin/env python3
"""PreToolUse hook for the `format-scribe` agent: it may write Markdown under docs/formats/ and nothing else.

The scribe reads the reference projects; this is the wall in the other direction. Reads the hook's JSON on
stdin and denies any Write or Edit whose path is not docs/formats/<name>.md inside the project.
"""

import json
import os
import sys


def main() -> None:
    try:
        event = json.load(sys.stdin)
    except json.JSONDecodeError:
        return
    if event.get("tool_name") not in {"Write", "Edit", "NotebookEdit"}:
        return
    path = str((event.get("tool_input") or {}).get("file_path") or (event.get("tool_input") or {}).get("notebook_path") or "")
    root = os.path.realpath(event.get("cwd") or os.getcwd())
    target = os.path.realpath(os.path.join(root, path))
    allowed_dir = os.path.join(root, "docs", "formats")
    inside = os.path.commonpath([target, allowed_dir]) == allowed_dir
    if inside and target.endswith(".md") and os.path.dirname(target) == allowed_dir:
        return
    print(
        json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "deny",
                    "permissionDecisionReason": "format-scribe writes only docs/formats/<name>.md: a description of a format or "
                    "behaviour, never code. Nothing else may be written from this role.",
                }
            }
        )
    )


if __name__ == "__main__":
    main()
