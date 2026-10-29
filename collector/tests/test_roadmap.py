"""Roadmap parsing, step status, lane --step, collector output and tirith.toml keys."""

from __future__ import annotations

import json
import os
from pathlib import Path

import jsonschema

from tirith import manifest as mf
from tirith import roadmap as rm
from tirith.cli import main

SAMPLE = """# TODO: plan

Intro paragraph with - [ ] nothing.

## 0. Done step

- [x] One.
- [X] Two.

## 1. Mixed `step`

- [x] Finished.
- [ ] Open item that wraps
      onto a second line.
  - [ ] Nested open.
    - [x] Deeply nested done.

## 2. Nothing yet

- [ ] A.

## 3. No items

Just prose.

## Orden

- [ ] not a step item
"""


def _write_project(folder: Path, toml: str = "", text: str = SAMPLE, name: str = "docs/TODO.md") -> Path:
    path = folder / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    if toml:
        (folder / "tirith.toml").write_text(toml, encoding="utf-8")
    return path


def test_parse_steps_and_items() -> None:
    steps = rm.parse(SAMPLE)
    assert [s["id"] for s in steps] == ["0", "1", "2", "3"]
    assert steps[1]["title"] == "Mixed `step`"
    assert [i["done"] for i in steps[0]["items"]] == [True, True]
    texts = [i["text"] for i in steps[1]["items"]]
    assert texts == ["Finished.", "Open item that wraps onto a second line.",
                     "Nested open.", "Deeply nested done."]
    assert [i["done"] for i in steps[1]["items"]] == [True, False, False, True]
    assert steps[3]["items"] == []


def test_unnumbered_heading_is_not_a_step() -> None:
    steps = rm.parse(SAMPLE)
    assert all(s["title"] != "Orden" for s in steps)
    assert "not a step item" not in json.dumps(steps)


def test_step_status_rules() -> None:
    assert rm.step_status(0, 0, True) == "empty"
    assert rm.step_status(2, 2, False) == "done"
    assert rm.step_status(1, 3, False) == "active"
    assert rm.step_status(0, 3, True) == "active"
    assert rm.step_status(0, 3, False) == "todo"


def test_build_counts_and_status(tmp_path: Path) -> None:
    _write_project(tmp_path)
    road = rm.build(tmp_path, "p", [])
    assert [s["status"] for s in road["steps"]] == ["done", "active", "todo", "empty"]
    assert (road["done"], road["total"]) == (4, 7)
    assert road["deadline"] is None
    assert road["file"].endswith("docs/TODO.md")


def test_working_agent_makes_step_active(tmp_path: Path) -> None:
    _write_project(tmp_path)
    agents = [{"id": "a", "project": "p", "status": "working", "step": "2"},
              {"id": "b", "project": "p", "status": "done", "step": "2"},
              {"id": "c", "project": "other", "status": "working", "step": "2"}]
    step = rm.build(tmp_path, "p", agents)["steps"][2]
    assert step["status"] == "active"
    assert step["agents"] == ["a"]


def test_default_files_and_missing(tmp_path: Path) -> None:
    assert rm.build(tmp_path, "p", []) is None
    assert rm.build(None, "p", []) is None
    _write_project(tmp_path, name="TODO.md")
    assert rm.build(tmp_path, "p", [])["file"].endswith("/TODO.md")
    _write_project(tmp_path, text="## 9. Docs step\n- [ ] x\n")
    assert rm.build(tmp_path, "p", [])["steps"][0]["id"] == "9"


def test_toml_roadmap_and_deadline(tmp_path: Path) -> None:
    _write_project(tmp_path, 'roadmap = "plan/ROAD.md"\ndeadline = "2026-11-04T21:00:00-05:00"\n',
                   text="## 1. Only\n- [x] a\n", name="plan/ROAD.md")
    _write_project(tmp_path)
    manifest = mf.load(tmp_path)
    assert (manifest.roadmap, manifest.deadline) == ("plan/ROAD.md", "2026-11-04T21:00:00-05:00")
    road = rm.build(tmp_path, "p", [])
    assert road["file"].endswith("plan/ROAD.md")
    assert road["deadline"] == "2026-11-04T21:00:00-05:00"
    assert [s["id"] for s in road["steps"]] == ["1"]


def test_cache_follows_mtime_and_size_limit(tmp_path: Path) -> None:
    path = _write_project(tmp_path, text="## 1. A\n- [ ] x\n")
    assert rm.build(tmp_path, "p", [])["done"] == 0
    path.write_text("## 1. A\n- [x] x\n", encoding="utf-8")
    os.utime(path, ns=(1, 2 * 10 ** 18))
    assert rm.build(tmp_path, "p", [])["done"] == 1
    path.write_text("## 1. A\n" + "- [x] y\n" * (rm.MAX_BYTES // 4), encoding="utf-8")
    assert rm.build(tmp_path, "p", []) is None


def test_lane_step_flows_into_state(home: Path, tmp_path: Path) -> None:
    from conftest import make_repo
    repo = make_repo(tmp_path / "proj")
    _write_project(repo, 'deadline = "2026-11-04T21:00:00-05:00"\n')
    (home / "config.toml").write_text(
        f'[[projects]]\nid = "demo"\nname = "Demo"\nrepo = "{repo}"\n', encoding="utf-8")
    main(["lane", "start", "--project", "demo", "--class", "herald", "--feature", "Plan work",
          "--id", "h-1", "--pid", str(os.getpid()), "--step", "2", "--workspace", str(repo)])
    lane = json.loads((home / "lanes" / "h-1.json").read_text(encoding="utf-8"))
    assert lane["step"] == "2"
    main(["collect", "--once"])
    state = json.loads((home / "state.json").read_text(encoding="utf-8"))
    agent = [a for a in state["agents"] if a["id"] == "h-1"][0]
    assert agent["step"] == "2"
    project = [p for p in state["projects"] if p["id"] == "demo"][0]
    road = project["roadmap"]
    assert road["deadline"] == "2026-11-04T21:00:00-05:00"
    assert road["steps"][2]["status"] == "active" and road["steps"][2]["agents"] == ["h-1"]
    schema = json.loads((Path(__file__).resolve().parents[1] / "state.schema.json").read_text())
    jsonschema.validate(state, schema)


def test_lane_without_step_is_null(home: Path) -> None:
    main(["lane", "start", "--project", "demo", "--class", "herald", "--feature", "x", "--id", "h-2"])
    lane = json.loads((home / "lanes" / "h-2.json").read_text(encoding="utf-8"))
    assert lane["step"] is None
