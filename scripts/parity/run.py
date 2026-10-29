"""Parity harness: run the Python and the Rust collector once on copies of the same home and compare.

    cd collector && uv run python ../scripts/parity/run.py [--only NAME ...] [--keep] [--rust-bin PATH]

For every fixture home (``fixtures.py``) it runs ``python -m tirith collect --once`` and
``tirith-rs collect --once`` on two copies of the home, then compares the two ``state.json`` files with
``compare.py``: sections the Rust state does not have yet print as "not ported yet" (not a failure);
anything else that differs is printed as a diff and fails the run.  Exit codes: 0 parity, 1 differences,
2 a collector failed or the Rust binary is missing.

It only ever touches temporary folders and passes ``--home`` explicitly: never the real ``~/.tirith``.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import compare  # noqa: E402
import fixtures  # noqa: E402

ROOT = fixtures.ROOT
TIMEOUT_S = 180


def rust_binary() -> Path:
    """``tirith-rs`` of the current cargo target folder (``CARGO_TARGET_DIR`` wins)."""
    target = os.environ.get("CARGO_TARGET_DIR")
    if not target:
        out = subprocess.run(["cargo", "metadata", "--format-version", "1", "--no-deps"], cwd=ROOT,
                             capture_output=True, text=True, check=True)
        target = json.loads(out.stdout)["target_directory"]
    return Path(target) / "debug" / "tirith-rs"


def run_collector(argv: list[str], home: Path, env_extra: dict[str, str]) -> tuple[dict | None, str]:
    """Run one collector on ``home``; ``(state, "")`` or ``(None, why)``."""
    env = {**os.environ, "TIRITH_HOME": str(home), **env_extra}
    env.pop("TIRITH_SESSION", None)
    try:
        done = subprocess.run(argv, cwd=ROOT / "collector", env=env, capture_output=True, text=True,
                              timeout=TIMEOUT_S)
    except subprocess.TimeoutExpired:
        return None, f"timed out after {TIMEOUT_S}s"
    if done.returncode != 0:
        return None, f"exit {done.returncode}: {(done.stderr or done.stdout).strip()[-600:]}"
    try:
        return json.loads((home / "state.json").read_text(encoding="utf-8")), ""
    except (OSError, ValueError) as exc:
        return None, f"no readable state.json ({exc})"


def run_fixture(name: str, rust_bin: Path, keep: bool) -> int:
    """Compare one fixture; returns 0 (parity), 1 (differences) or 2 (collector failure)."""
    scratch = Path(tempfile.mkdtemp(prefix=f"tirith-parity-{name}-"))
    try:
        source = fixtures.FIXTURES[name](scratch / "src")
        homes = {side: scratch / side for side in ("python", "rust")}
        for home in homes.values():
            shutil.copytree(source, home)
        py, py_err = run_collector([sys.executable, "-m", "tirith", "collect", "--once", "--home",
                                    str(homes["python"])], homes["python"], {})
        rs, rs_err = run_collector([str(rust_bin), "--home", str(homes["rust"]), "collect", "--once"],
                                   homes["rust"], {})
        if py is None or rs is None:
            print(f"[FAIL] {name}")
            for side, err in (("python", py_err), ("rust", rs_err)):
                if err:
                    print(f"       {side} collector: {err}")
            return 2
        ok, not_ported, diffs = compare.compare_states(py, rs)
        print(f"[{'DIFF' if diffs else ' ok '}] {name}: {len(ok)} sections equal, {len(not_ported)} not ported yet")
        if diffs:
            for line in diffs:
                print(f"       {line}")
        return 1 if diffs else 0
    finally:
        if keep:
            print(f"       kept {scratch}")
        else:
            shutil.rmtree(scratch, ignore_errors=True)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--only", nargs="+", metavar="NAME", help="run just these fixtures")
    parser.add_argument("--list", action="store_true", help="list the fixtures and exit")
    parser.add_argument("--keep", action="store_true", help="keep the temporary homes")
    parser.add_argument("--rust-bin", type=Path, help="the tirith-rs binary (default: cargo target folder)")
    parser.add_argument("--show-not-ported", action="store_true", help="name every section not ported yet")
    args = parser.parse_args(argv)
    if args.list:
        print("\n".join(fixtures.FIXTURES))
        return 0
    names = args.only or list(fixtures.FIXTURES)
    unknown = [n for n in names if n not in fixtures.FIXTURES]
    if unknown:
        print(f"unknown fixture(s): {', '.join(unknown)}; try --list", file=sys.stderr)
        return 2
    rust_bin = args.rust_bin or rust_binary()
    if not rust_bin.is_file():
        print(f"missing {rust_bin}: build it first (cargo build -p tirith-cli)", file=sys.stderr)
        return 2
    results = [run_fixture(name, rust_bin, args.keep) for name in names]
    if args.show_not_ported:
        print_not_ported(names, rust_bin)
    worst = max(results)
    verdict = {0: "parity", 1: "DIFFERENCES", 2: "FAILED"}[worst]
    print(f"parity: {verdict} ({results.count(0)}/{len(results)} fixtures equal)")
    return worst


def print_not_ported(names: list[str], rust_bin: Path) -> None:
    """The union of the not-yet-ported sections over the first fixture that runs (they share one schema)."""
    scratch = Path(tempfile.mkdtemp(prefix="tirith-parity-sections-"))
    try:
        source = fixtures.FIXTURES[names[0]](scratch / "src")
        py_home, rs_home = scratch / "python", scratch / "rust"
        shutil.copytree(source, py_home)
        shutil.copytree(source, rs_home)
        py, _ = run_collector([sys.executable, "-m", "tirith", "collect", "--once", "--home", str(py_home)], py_home, {})
        rs, _ = run_collector([str(rust_bin), "--home", str(rs_home), "collect", "--once"], rs_home, {})
        if py is not None and rs is not None:
            _, missing, _ = compare.compare_states(py, rs)
            for section in missing:
                print(f"not ported yet: {section}")
    finally:
        shutil.rmtree(scratch, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
