"""Projects as folders (add/list/remove, auto-discovery) and sessions."""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

import pytest

from tirith.cli import main
from tirith.collector import Collector
from tirith.projects import current_session, load_auto, match, entries
from tirith.home import load_config
from tirith.store import read_json, read_jsonl

from conftest import make_repo, write_config

jsonschema = pytest.importorskip("jsonschema")
ROOT = Path(__file__).resolve().parents[2]
SCHEMA = json.loads((ROOT / "collector" / "state.schema.json").read_text(encoding="utf-8"))


@pytest.fixture(autouse=True)
def _no_session_env(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("TIRITH_SESSION", raising=False)


def collect(home: Path) -> dict:
    assert main(["collect", "--once"]) == 0
    state = json.loads((home / "state.json").read_text(encoding="utf-8"))
    jsonschema.validate(state, SCHEMA)
    return state


# -- project add / list / remove ----------------------------------------------------------


def test_project_add_list_remove_keeps_config(home: Path, tmp_path: Path, capsys) -> None:
    config = home / "config.toml"
    config.write_text('# my notes\n[brain]\nenabled = false  # offline\n\n'
                      '[[rooms]]\nid = "r1"\n', encoding="utf-8")
    folder = tmp_path / "My App"
    folder.mkdir()
    assert main(["project", "add", str(folder), "--color", "#112233",
                 "--guard-words", "foo, bar"]) == 0
    assert capsys.readouterr().out.startswith(f"my-app: {folder.resolve()}")
    text = config.read_text(encoding="utf-8")
    assert text.startswith("# my notes\n[brain]\nenabled = false  # offline\n")
    assert '[[rooms]]\nid = "r1"' in text
    cfg = load_config(home)
    proj = cfg.project("my-app")
    assert proj and proj.repo == folder.resolve() and proj.color == "#112233"
    assert proj.guard_words == ["foo", "bar"] and proj.name == "My App"
    assert cfg.brain_enabled is False and cfg.room("r1") is not None
    # Same folder again: nothing changes; same name, other folder: unique id.
    assert main(["project", "add", str(folder)]) == 0
    assert "already registered as my-app" in capsys.readouterr().out
    other = tmp_path / "other"
    other.mkdir()
    assert main(["project", "add", str(other), "--name", "My App"]) == 0
    assert load_config(home).project("my-app-2") is not None
    assert main(["project", "list"]) == 0
    listing = capsys.readouterr().out
    assert "my-app " in listing and "my-app-2 " in listing and str(folder.resolve()) in listing
    assert main(["project", "remove", "my-app"]) == 0
    text = config.read_text(encoding="utf-8")
    assert text.startswith("# my notes\n") and 'id = "my-app"\n' not in text
    assert load_config(home).project("my-app-2") is not None
    assert main(["project", "remove", "ghost"]) == 1
    assert not list(home.glob("config.toml.*.tmp"))


def test_project_add_dot_and_validation(home: Path, tmp_path: Path, monkeypatch, capsys) -> None:
    folder = tmp_path / "dotted"
    folder.mkdir()
    monkeypatch.chdir(folder)
    assert main(["project", "add", "."]) == 0
    assert load_config(home).project("dotted").repo == folder.resolve()
    with pytest.raises(SystemExit):
        main(["project", "add", str(tmp_path / "missing")])
    with pytest.raises(SystemExit):
        main(["project", "add", str(folder), "--name", "x", "--color", "red"])


def test_longest_prefix_match(home: Path, tmp_path: Path) -> None:
    outer, inner = tmp_path / "mono", tmp_path / "mono" / "apps" / "web"
    inner.mkdir(parents=True)
    main(["project", "add", str(outer), "--name", "mono"])
    main(["project", "add", str(inner), "--name", "web"])
    items = entries(load_config(home), load_auto(home))
    assert match(inner / "src", items).id == "web"
    assert match(outer / "docs", items).id == "mono"
    assert match(tmp_path / "elsewhere", items) is None


# -- auto-discovery -------------------------------------------------------------------------


def test_auto_discovery_from_lane_run_and_room(home: Path, tmp_path: Path, monkeypatch) -> None:
    repo = make_repo(tmp_path / "gitrepo")
    (repo / "sub" / "deep").mkdir(parents=True)
    plain = tmp_path / "plainfolder"
    plain.mkdir()
    roomdir = tmp_path / "roomfolder"
    roomdir.mkdir()
    registered = tmp_path / "reg"
    (registered / "pkg").mkdir(parents=True)
    write_config(home, [{"id": "reg", "name": "Reg", "repo": str(registered)}],
                 extra=f'[[rooms]]\nid = "room1"\nproject = "{roomdir}"\n')
    main(["lane", "start", "--project", "whatever", "--class", "warden", "--feature", "Look",
          "--id", "l1", "--workspace", str(repo / "sub" / "deep"), "--pid", str(os.getpid())])
    main(["lane", "start", "--project", "reg", "--class", "warden", "--feature", "In reg",
          "--id", "l2", "--workspace", str(registered / "pkg"), "--pid", str(os.getpid())])
    monkeypatch.chdir(plain)
    main(["run", "--project", "ci", "--pipeline", "p", "--stage", "s", "--",
          sys.executable, "-c", "print(1)"])
    events = read_jsonl(home / "events.jsonl")
    assert events[0]["cwd"] == str(plain.resolve()) == events[1]["cwd"]
    state = collect(home)
    by_id = {p["id"]: p for p in state["projects"]}
    assert by_id["gitrepo"]["auto"] is True and by_id["gitrepo"]["folder"] == str(repo.resolve())
    assert by_id["plainfolder"]["auto"] is True
    assert by_id["roomfolder"]["auto"] is True
    assert by_id["reg"]["auto"] is False and by_id["reg"]["folder"] == str(registered.resolve())
    assert "pkg" not in by_id and "deep" not in by_id
    agents = {a["id"]: a for a in state["agents"]}
    assert agents["l1"]["project"] == "gitrepo" and agents["l2"]["project"] == "reg"
    # "ci" is not a known project, so the stage's cwd decides.
    assert state["pipelines"][0]["project"] == "plainfolder"
    assert by_id["gitrepo"]["counts"] == {"agents_working": 1, "agents_total": 1,
                                          "pipelines_running": 0, "flows_24h": 1}
    assert by_id["reg"]["counts"]["agents_total"] == 1
    # Persisted and stable: the file exists and a second cycle does not duplicate.
    stored = {p["id"]: p["folder"] for p in load_auto(home)["projects"]}
    assert stored["gitrepo"] == str(repo.resolve())
    again = collect(home)
    assert [p["id"] for p in again["projects"]] == [p["id"] for p in state["projects"]]
    assert {p["id"]: p["folder"] for p in load_auto(home)["projects"]} == stored


def test_auto_project_remove_sticks_and_add_promotes(home: Path, tmp_path: Path, capsys) -> None:
    work = tmp_path / "scratch"
    work.mkdir()
    main(["lane", "start", "--project", "x", "--class", "warden", "--feature", "F", "--id", "a1",
          "--workspace", str(work), "--pid", str(os.getpid())])
    collect(home)
    assert main(["project", "list"]) == 0
    listing = capsys.readouterr().out
    assert "scratch" in listing and "[auto]" in listing
    assert main(["project", "add", str(work), "--name", "Scratch Pad"]) == 0
    assert load_config(home).project("scratch-pad") is not None
    assert load_auto(home)["projects"] == []
    state = collect(home)
    scratch = next(p for p in state["projects"] if p["id"] == "scratch-pad")
    assert scratch["auto"] is False and scratch["name"] == "Scratch Pad"
    assert next(a for a in state["agents"] if a["id"] == "a1")["project"] == "scratch-pad"
    # Remove it again, then an unknown folder's auto project that is removed stays removed.
    assert main(["project", "remove", "scratch-pad"]) == 0
    state = collect(home)
    assert any(p["id"] == "scratch" and p["auto"] for p in state["projects"])
    assert main(["project", "remove", "scratch"]) == 0
    state = collect(home)
    assert not any(p.get("folder") == str(work.resolve()) for p in state["projects"])


# -- sessions ---------------------------------------------------------------------------------


def test_session_start_end_list(home: Path, capsys) -> None:
    assert main(["session", "start", "Tirith web", "--id", "s-web"]) == 0
    out = capsys.readouterr().out.splitlines()
    assert out == ["s-web", "export TIRITH_SESSION=s-web"]
    assert (home / "session").read_text(encoding="utf-8").strip() == "s-web"
    assert current_session(home) == "s-web"
    assert main(["session", "start", "Another"]) == 0
    auto_id = capsys.readouterr().out.splitlines()[0]
    assert auto_id.endswith("-another") and auto_id[:4].isdigit()
    assert current_session(home) == auto_id
    with pytest.raises(SystemExit):
        main(["session", "start", "dup", "--id", "s-web"])
    assert main(["session", "list"]) == 0
    listing = capsys.readouterr().out
    assert f"* {auto_id}" in listing and "  s-web" in listing and "open" in listing
    assert main(["session", "end"]) == 0
    assert current_session(home) is None and not (home / "session").exists()
    assert main(["session", "end", "s-web"]) == 0
    assert main(["session", "end", "nope"]) == 1
    assert main(["session", "end"]) == 1
    rows = read_json(home / "sessions.json", [])
    assert all(r["ended_at"] for r in rows)


def test_env_wins_over_file(home: Path, monkeypatch) -> None:
    main(["session", "start", "file one", "--id", "from-file"])
    assert current_session(home) == "from-file"
    monkeypatch.setenv("TIRITH_SESSION", "from-env")
    assert current_session(home) == "from-env"


def test_events_record_session_and_state_exposes_it(home: Path, tmp_path: Path, capsys,
                                                    monkeypatch) -> None:
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(tmp_path)}])
    # No session: everything is null.
    main(["note", "before", "--project", "demo"])
    assert read_jsonl(home / "notes.jsonl")[0]["session"] is None
    main(["session", "start", "Release day", "--id", "rel"])
    capsys.readouterr()
    main(["lane", "start", "--project", "demo", "--class", "herald", "--feature", "Ship",
          "--id", "h1", "--pid", str(os.getpid())])
    main(["pipeline", "declare", "rel", "--project", "demo", "--stages", "build"])
    main(["run", "--project", "demo", "--pipeline", "rel", "--stage", "build", "--",
          sys.executable, "-c", "pass"])
    main(["note", "during", "--project", "demo"])
    main(["wait", "add", "Press go", "--project", "demo", "--option", "Go|go"])
    wait_id = capsys.readouterr().out.strip().splitlines()[-1]
    main(["flow", "--from", "manager", "--to", "owner", "--text", "hello"])
    main(["lane", "end", "h1", "--result", "done", "--report", "ok"])
    assert read_json(home / "lanes" / "h1.json", {})["session"] == "rel"
    assert all(e["session"] == "rel" for e in read_jsonl(home / "events.jsonl"))
    assert read_jsonl(home / "notes.jsonl")[1]["session"] == "rel"
    assert read_jsonl(home / "waits.jsonl")[0]["session"] == "rel"
    assert {f["session"] for f in read_jsonl(home / "flows.jsonl")} == {"rel"}
    # A session set only through the env var is also recorded (and listed).
    main(["session", "end"])
    monkeypatch.setenv("TIRITH_SESSION", "env-only")
    main(["note", "via env", "--project", "demo"])
    assert read_jsonl(home / "notes.jsonl")[-1]["session"] == "env-only"
    monkeypatch.delenv("TIRITH_SESSION")
    main(["wait", "done", wait_id])
    state = collect(home)
    agent = state["agents"][0]
    assert agent["session"] == "rel" and agent["project"] == "demo"
    assert state["pipelines"][0]["session"] == "rel"
    notes = {n["text"]: n["session"] for n in state["manager"]["notes"]}
    assert notes == {"before": None, "during": "rel", "via env": "env-only"}
    assert all(f["session"] in ("rel", None) and "project" in f for f in state["flows"])
    assert {f["project"] for f in state["flows"] if f["kind"] == "brief"} == {"demo"}
    sessions = {s["id"]: s for s in state["sessions"]}
    assert sessions["rel"]["title"] == "Release day" and sessions["rel"]["ended_at"]
    assert sessions["rel"]["projects"] == ["demo"]
    # flows: brief, request, flow, report (lane end), pipeline report, answer (wait done)
    assert sessions["rel"]["counts"] == {"agents": 1, "pipelines": 1, "flows": 6, "notes": 1}
    assert sessions["env-only"]["counts"]["notes"] == 1


def test_open_wait_exposes_session(home: Path, capsys) -> None:
    main(["session", "start", "Waits", "--id", "w"])
    main(["wait", "add", "Decide", "--project", "demo", "--option", "Yes|yes"])
    state = collect(home)
    assert state["waiting_on_you"][0]["session"] == "w"
    assert state["sessions"][0]["id"] == "w"


def test_old_state_and_fixtures_validate() -> None:
    for name in ("state.example.json", "state.demo.json", "state.rooms.example.json"):
        jsonschema.validate(json.loads((ROOT / "fixtures" / name).read_text(encoding="utf-8")),
                            SCHEMA)
    demo = json.loads((ROOT / "fixtures" / "state.demo.json").read_text(encoding="utf-8"))
    assert demo["sessions"] and all("counts" in p and "folder" in p for p in demo["projects"])


def test_session_env_reaches_child(home: Path, tmp_path: Path, capsys) -> None:
    main(["session", "start", "Child", "--id", "kid"])
    capsys.readouterr()
    main(["run", "--project", "demo", "--pipeline", "p", "--stage", "s", "--", sys.executable,
          "-c", "import os;print(os.environ.get('TIRITH_SESSION'))"])
    assert capsys.readouterr().out.strip() == "kid"
