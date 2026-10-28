"""Write ``<name>.js`` (``window.TIRITH_STATE = <json>;``) next to each given JSON fixture.

Usage: ``uv run python fixture_js.py ../fixtures/state.example.json ...``
With no arguments it regenerates every ``state*.json`` fixture in ../fixtures.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

from tirith.store import state_js, write_text_atomic


def convert(path: Path) -> Path:
    target = path.with_suffix(".js")
    write_text_atomic(target, state_js(json.loads(path.read_text(encoding="utf-8"))))
    return target


def main(argv: list[str]) -> int:
    paths = [Path(a) for a in argv] or sorted(
        (Path(__file__).resolve().parent.parent / "fixtures").glob("state*.json"))
    for path in paths:
        print(convert(path))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
