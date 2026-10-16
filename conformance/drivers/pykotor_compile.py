"""Compile one NSS file with pykotor's built-in compiler: IN OUT.

The conformance suite runs this as a black box, the way the research
measurement called it: compile_nss(source, Game.K1) with pykotor's bundled
include library, written with bytes_ncs. Exit 1 with the error on stderr
when compilation fails.
"""

import logging
import sys

logging.disable(logging.CRITICAL)

from pykotor.common.misc import Game  # noqa: E402
from pykotor.resource.formats.ncs import bytes_ncs, compile_nss  # noqa: E402


def main() -> int:
    src_path, out_path = sys.argv[1], sys.argv[2]
    with open(src_path, encoding="windows-1252") as f:
        source = f.read()
    try:
        ncs = compile_nss(source, Game.K1)
    except Exception as e:  # every compiler failure is a result, not a crash
        print(f"error: {type(e).__name__}: {e}".splitlines()[0], file=sys.stderr)
        return 1
    with open(out_path, "wb") as f:
        f.write(bytes(bytes_ncs(ncs)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
