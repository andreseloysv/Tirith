"""Fixture homes for the parity harness, synthesized the way the Python tests build theirs.

Each builder takes an empty folder and returns the Tirith home to hand to both collectors.  Project folders
live next to the home and are referenced from ``config.toml`` by absolute path, so both runs read the same
files.  Add a fixture by writing a builder and listing it in ``FIXTURES``; later lanes extend this as they
port more sections (lanes, events, notes ... go into the home).
"""

from __future__ import annotations

import shutil
from datetime import date, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

SAMPLE = """\
# TODO: Demo plan

Deadline: 2026-12-01T10:00:00Z

Intro text that is ignored.

## 1. First step

- [x] a finished item
- [ ] an open item that continues
      on the next line
  - [ ] nested items count too
* [X] star bullet, done
+ [ ] plus bullet, open

## Order
Headings without a number are not steps.
- [ ] so this one is not counted

## 2. Second step ##
- [x] only done items

## 3. Empty step

## 4. Tagged
- [ ] owner thing (dueño: payment pay the invoice)
- [ ] owner without reason (owner)
- [ ] out of scope (fuera de este plan)
- [ ] english out (out of this plan: later)
"""


def _project(root: Path, name: str, files: dict[str, str | bytes]) -> Path:
    folder = root / name
    for rel, content in files.items():
        path = folder / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(content, bytes):
            path.write_bytes(content)
        else:
            path.write_text(content, encoding="utf-8")
    folder.mkdir(parents=True, exist_ok=True)
    return folder


def _home(root: Path, config: str) -> Path:
    home = root / "home"
    home.mkdir(parents=True, exist_ok=True)
    (home / "config.toml").write_text(config, encoding="utf-8")
    return home


def _row(pid: str, folder: Path | None, **extra: str) -> str:
    lines = ["[[projects]]", f'id = "{pid}"']
    if folder is not None:
        lines.append(f'repo = "{folder}"')
    lines += [f'{k} = "{v}"' for k, v in extra.items()]
    return "\n".join(lines) + "\n"


def empty_home(root: Path) -> Path:
    return _home(root, "")


def basic_roadmap(root: Path) -> Path:
    folder = _project(root, "basic", {"docs/TODO.md": SAMPLE})
    return _home(root, _row("basic", folder, name="Basic", color="#2f7d5b"))


def date_tags(root: Path) -> Path:
    today = date.today()
    day = lambda n: (today + timedelta(days=n)).isoformat()  # noqa: E731
    text = (
        "## 1. Waiting on dates\n"
        f"- [ ] tomorrow (desde {day(1)})\n"
        f"- [ ] next week (from {day(7)})\n\n"
        "## 2. Arrived\n"
        f"- [ ] today (desde {day(0)})\n"
        f"- [ ] yesterday (from {day(-1)})\n\n"
        "## 3. Mixed\n"
        f"- [x] done\n- [ ] later (desde {day(3)})\n- [ ] gated (dueño: credentials log in)\n\n"
        "## 4. Bad date\n- [ ] not a day (desde 2026-13-45)\n"
    )
    folder = _project(root, "dates", {"TODO.md": text})
    return _home(root, _row("dates", folder))


def manifest_plans(root: Path) -> Path:
    folder = _project(root, "plans", {
        "tirith.toml": 'roadmap = "plan/ROAD.md"\ndeadline = "2026-11-04T21:00:00-05:00"\n'
                       'plans = ["plan/ROAD.md", "plan/beta.md", "plan/missing.md", "./plan//beta.md"]\n',
        "plan/ROAD.md": "# Plan: Main road\n\n## 1. Go\n- [x] a\n- [ ] b\n",
        "plan/beta.md": "# Beta\nDeadline: 2027-01-01\n\n## 1. One\n- [ ] x\n## 2. Two\n- [x] y\n",
        "docs/TODO.md": "## 1. Ignored when roadmap is set\n- [ ] z\n",
    })
    return _home(root, _row("plans", folder))


def missing_and_odd(root: Path) -> Path:
    no_plan = _project(root, "noplan", {"README.md": "hello\n"})
    big = _project(root, "big", {"TODO.md": "## 1. Big\n- [ ] " + "x" * (256 * 1024) + "\n"})
    bad_toml = _project(root, "badtoml", {"tirith.toml": "roadmap = [", "TODO.md": "## 1. Falls back\n- [ ] a\n"})
    cfg_missing = _project(root, "cfgmissing", {"tirith.toml": 'roadmap = "nope.md"\n', "TODO.md": "## 1. Root\n- [ ] a\n"})
    return _home(root, "".join([
        _row("noplan", no_plan), _row("big", big), _row("badtoml", bad_toml), _row("cfgmissing", cfg_missing),
        _row("norepo", None, name="No folder"), _row("gone", root / "does-not-exist"),
    ]))


def text_edge_cases(root: Path) -> Path:
    crlf = "## 1. Windows\r\n- [x] done\r\n- [ ] open\r\n      wrapped\r\n"
    unicode_text = (
        "# TODO: Año nuevo — señal\n\n## 1. Ñandú \U0001F600\n- [ ] (DUEÑO: personal-account (Apple ID) paga) café\n"
        "- [ ] (Desde 2099-01-01) futuro\n- [ ] línea separada por u2028\n"
    )
    folder_a = _project(root, "crlf", {"TODO.md": crlf.encode("utf-8")})
    folder_b = _project(root, "unicode", {"docs/TODO.md": unicode_text})
    folder_c = _project(root, "latin", {"TODO.md": b"## 1. Bad \xff bytes\n- [ ] x\n"})
    return _home(root, _row("crlf", folder_a) + _row("unicode", folder_b, name="Ünï") + _row("latin", folder_c))


def projects_forms(root: Path) -> Path:
    a = _project(root, "alpha", {"TODO.md": "## 1. A\n- [ ] a\n"})
    b = _project(root, "beta", {"TODO.md": "## 1. B\n- [x] b\n"})
    config = (
        "[projects.zeta]\n" f'folder = "{a}"\nname = "Zeta"\n\n'
        "[projects.beta]\n" f'repo = "{b}"\ncolor = "#111111"\nguard = false\n\n'
        "[brain]\nenabled = false\n"
    )
    return _home(root, config)


def tirith_self(root: Path) -> Path:
    """This repository's own plan and manifest: real data, real tags."""
    files = {name: (ROOT / name).read_text(encoding="utf-8") for name in ("tirith.toml", "docs/TODO.md")}
    folder = _project(root, "tirith", files)
    return _home(root, _row("tirith", folder, name="Tirith"))


FIXTURES = {
    "empty-home": empty_home,
    "basic-roadmap": basic_roadmap,
    "date-tags": date_tags,
    "manifest-plans": manifest_plans,
    "missing-and-odd": missing_and_odd,
    "text-edge-cases": text_edge_cases,
    "projects-forms": projects_forms,
    "tirith-self": tirith_self,
}


def clean(root: Path) -> None:
    shutil.rmtree(root, ignore_errors=True)
