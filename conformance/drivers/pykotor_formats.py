"""pykotor's readers and writers as a benchmark comparator: MODE FORMAT LIST OUT.

The protocol is the one `cargo xtask bench` speaks to every batch driver (conformance/bench.toml):
`start` imports the format's module and exits, 3 with the reason on stderr when the format has no
documented entry point here; `read` reads every file in LIST; `write` reads and writes each again in
memory; `check` does both and saves a written file that differs from the original as OUT.d/I.EXT. OUT
gets one JSON line per file.

Only the entry points named in docs/proposals/rakata.md's appendix are called, as
`pykotor.resource.formats.MODULE.read_X(bytes)` and `bytes_X(model)`, the shape
conformance/drivers/pykotor_compile.py uses for NCS. A format whose entry points are not documented
there is reported unsupported with that reason.
"""

import importlib
import json
import logging
import os
import sys

logging.disable(logging.CRITICAL)

# FORMAT: (module, reader, writer or None)
ENTRY_POINTS = {
    "gff": ("gff", "read_gff", "bytes_gff"),
    "2da": ("twoda", "read_2da", "bytes_2da"),
    "tlk": ("tlk", "read_tlk", "bytes_tlk"),
    "lip": ("lip", "read_lip", "bytes_lip"),
    "erf": ("erf", "read_erf", None),
    "rim": ("rim", "read_rim", None),
}
UNSUPPORTED = 3


def first_line(e: BaseException) -> str:
    text = f"{type(e).__name__}: {e}"
    return text.splitlines()[0] if text else type(e).__name__


def main() -> int:
    mode, fmt, list_path, out_path = sys.argv[1:5]
    if fmt not in ENTRY_POINTS:
        print(f"unsupported: no documented reader for {fmt}", file=sys.stderr)
        return UNSUPPORTED
    module_name, reader_name, writer_name = ENTRY_POINTS[fmt]
    try:
        module = importlib.import_module(f"pykotor.resource.formats.{module_name}")
        reader = getattr(module, reader_name)
        writer = getattr(module, writer_name) if writer_name else None
    except (ImportError, AttributeError) as e:
        print(f"unsupported: {first_line(e)}", file=sys.stderr)
        return UNSUPPORTED
    if mode == "write" and writer is None:
        print(f"unsupported: no documented writer for {fmt}", file=sys.stderr)
        return UNSUPPORTED
    with open(out_path, "w") as out:
        if mode == "start":
            return 0
        with open(list_path) as f:
            paths = [line.rstrip("\n") for line in f if line.strip()]
        saved = os.path.splitext(out_path)[0] + ".d"
        if mode == "check":
            os.makedirs(saved, exist_ok=True)
        for i, path in enumerate(paths):
            out.write(json.dumps(one(mode, reader, writer, path, i, saved)) + "\n")
    return 0


def one(mode, reader, writer, path, i, saved):
    with open(path, "rb") as f:
        data = f.read()
    try:
        model = reader(data)
    except Exception as e:  # every reader failure is a result
        return {"i": i, "error": first_line(e)}
    if mode == "read" or writer is None:
        return {"i": i, "ok": True}
    try:
        written = bytes(writer(model))
    except Exception as e:
        if mode == "write":
            return {"i": i, "error": first_line(e)}
        return {"i": i, "ok": True, "write_error": first_line(e)}
    if mode == "write":
        return {"i": i, "ok": True}
    if written == data:
        return {"i": i, "ok": True, "write": "identical"}
    target = os.path.join(saved, f"{i}{os.path.splitext(path)[1]}")
    with open(target, "wb") as f:
        f.write(written)
    return {"i": i, "ok": True, "write": "different", "out": target}


if __name__ == "__main__":
    sys.exit(main())
