"""Model dropdown data and the out-of-credits fallback: detection, one relaunch per agent, no loops."""

from __future__ import annotations

import json
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pytest

from conftest import make_hooks_mutable, make_repo, write_lane_manifest, write_plan
from tirith import dispatch as dispatch_mod
from tirith import fallback, inbox as inbox_mod, launch as launch_mod, models
from tirith.cli import main, relaunch_lane
from tirith.home import Config, load_config
from tirith.store import read_json, read_jsonl, read_lanes, write_lane

NOW = datetime(2026, 10, 29, 12, 0, tzinfo=timezone.utc)

CONFIG = """[[projects]]
id = "demo"
name = "Demo"
repo = "{repo}"

[models]
choices = ["a-big", "a-small", "b-fast"]
efforts = ["low", "high"]

[models.workers]
agent = "muse"
fallback = ["anthropic"]

[models.workers.projects.other]
fallback = []

[agents.anthropic]
launch = "agent-a --model a-small {{brief}}"
model_prefixes = ["a-"]

[agents.muse]
launch = "agent-m --model b-fast {{brief}}"
model_prefixes = ["b-"]
"""


def _at(seconds_ago: float) -> str:
    return (NOW - timedelta(seconds=seconds_ago)).strftime("%Y-%m-%dT%H:%M:%SZ")


@pytest.fixture
def cfg_home(home: Path, tmp_path: Path) -> Path:
    repo = make_repo(tmp_path / "demo")
    write_lane_manifest(repo)
    write_plan(repo)
    (home / "config.toml").write_text(CONFIG.format(repo=repo), encoding="utf-8")
    return home


def _log(path: Path, lines: list) -> str:
    path.write_text("\n".join(l if isinstance(l, str) else json.dumps(l) for l in lines) + "\n", encoding="utf-8")
    return str(path)


def _lane(home: Path, lane_id: str, log: str, agent: str = "muse", tried: list | None = None,
          ended: float = 60, result: str = "failed", **extra) -> dict:
    lane = {"id": lane_id, "project": "demo", "step": "1", "feature": "Thing", "class": "stonewright",
            "agent": agent, "tried": tried or [], "log": log, "brief": str(home / "brief.md"),
            "workspace": str(home / "ws"), "started_at": _at(ended + 600), "ended_at": _at(ended),
            "result": result, **extra}
    write_lane(home, lane)
    return lane


# -- dropdown data ------------------------------------------------------------------------------


def test_state_offers_models_by_agent_prefix_and_efforts(cfg_home: Path) -> None:
    config = load_config(cfg_home)
    st = models.state(config, [], fallback.status(config, [], NOW))
    assert st["choices"] == ["a-big", "a-small", "b-fast"]
    assert st["choices_by_agent"]["anthropic"] == ["a-big", "a-small"]
    assert st["choices_by_agent"]["muse"] == ["b-fast"]
    assert st["efforts"] == ["low", "high"]
    assert st["workers"]["fallback"] == ["anthropic"]
    assert st["agent_status"]["muse"] == {"state": "ok", "since": None}


def test_alias_without_prefixes_gets_every_model_and_efforts_default(home: Path) -> None:
    (home / "config.toml").write_text('[models]\nchoices = ["x", "y"]\n[agents.anthropic]\ncommand = "c"\n',
                                      encoding="utf-8")
    config = load_config(home)
    st = models.state(config)
    assert st["choices_by_agent"]["anthropic"] == ["x", "y"] and st["choices_by_agent"]["muse"] == ["x", "y"]
    assert st["efforts"] == list(models.EFFORTS)


# -- the chain ----------------------------------------------------------------------------------


def test_chain_follows_overrides(cfg_home: Path) -> None:
    config = load_config(cfg_home)
    assert models.resolve_chain(config, "demo", "1") == ["muse", "anthropic"]
    assert models.resolve_chain(config, "other", "1") == ["muse"]  # an empty list overrides
    assert models.resolve_chain(Config()) == []


def test_set_fallback_roundtrip_and_cli(cfg_home: Path, capsys: pytest.CaptureFixture) -> None:
    assert main(["--home", str(cfg_home), "models", "set", "workers", "--fallback", "anthropic,muse"]) == 0
    assert models.resolve_chain(load_config(cfg_home)) == ["muse", "anthropic"]
    assert load_config(cfg_home).models["workers"]["fallback"] == ["anthropic", "muse"]
    assert main(["--home", str(cfg_home), "models", "set", "workers", "--fallback", ""]) == 0
    assert load_config(cfg_home).models["workers"]["fallback"] == []
    assert main(["--home", str(cfg_home), "models", "set", "manager", "--fallback", "muse"]) == 3
    assert main(["--home", str(cfg_home), "models", "set", "workers", "--fallback", "bad alias"]) == 3
    main(["--home", str(cfg_home), "models", "set", "workers", "--fallback", "anthropic"])
    main(["--home", str(cfg_home), "models", "show"])
    assert "workers fallback: anthropic" in capsys.readouterr().out


def test_settings_request_with_fallback_is_applied(cfg_home: Path) -> None:
    req = inbox_mod.add_request(cfg_home, "setup", "settings", "setup", json.dumps(
        {"action": "models-set", "role": "workers", "model": "a-big", "fallback": ["anthropic"]}))
    assert main(["--home", str(cfg_home), "setup", "apply", req["id"]]) == 0
    raw = load_config(cfg_home).models["workers"]
    assert raw["model"] == "a-big" and raw["fallback"] == ["anthropic"]
    assert "fallback=anthropic" in inbox_mod.setup_summary(inbox_mod.find_request(cfg_home, req["id"])["setup"])


# -- detection ----------------------------------------------------------------------------------


@pytest.mark.parametrize("line", [
    "Error: HTTP 429 Too Many Requests",
    "request failed with status 402",
    "You have exceeded your quota for this month",
    "Your credit balance is too low to run this",
    "insufficient credits",
    "rate limit reached, retry later",
    "5-hour usage limit reached",
    {"type": "error", "error": {"message": "rate_limit_error: slow down"}},
    {"type": "result", "subtype": "error_during_execution", "result": "out of credits"},
])
def test_detects_out_of_credits(cfg_home: Path, tmp_path: Path, line) -> None:
    log = _log(tmp_path / "l.log", ["working on it", line])
    assert fallback.out_of_credits(load_config(cfg_home), log)


@pytest.mark.parametrize("lines", [
    ["compiling", "tests: 3 failed", "AssertionError: expected 4"],
    [{"type": "assistant", "message": "I will write the credits page and a rate limiter"}],
    [],
])
def test_ordinary_failures_are_not_out_of_credits(cfg_home: Path, tmp_path: Path, lines) -> None:
    assert not fallback.out_of_credits(load_config(cfg_home), _log(tmp_path / "l.log", lines))
    assert not fallback.out_of_credits(load_config(cfg_home), str(tmp_path / "missing.log"))


def test_only_the_last_lines_count_and_patterns_are_configurable(cfg_home: Path, tmp_path: Path) -> None:
    log = _log(tmp_path / "l.log", ["quota exceeded"] + [f"step {i}" for i in range(80)])
    config = load_config(cfg_home)
    assert not fallback.out_of_credits(config, log)
    config.models["out_of_credits_patterns"] = ["step 79"]
    assert fallback.out_of_credits(config, log)


# -- relaunch once, never in a loop -------------------------------------------------------------


def test_relaunch_once_with_next_alias_and_note_in_the_thread(cfg_home: Path, tmp_path: Path) -> None:
    config = load_config(cfg_home)
    req = inbox_mod.add_request(cfg_home, "finish", "demo", "1", "")
    log = _log(tmp_path / "m.log", ["working", "Error: 429 rate limit exceeded"])
    _lane(cfg_home, "muse-lane", log, request_id=req["id"])
    calls: list[tuple] = []

    def relauncher(home, lane, alias):
        calls.append((lane["id"], alias))
        return "anthropic-lane"
    state = {"handled": {}, "escalated": {}, "stop": {}, "reviews": {}, "overdue": {}}
    for _ in range(3):  # the dispatcher runs every cycle
        dispatch_mod.run(cfg_home, config, NOW, agents=[], lanes=read_lanes(cfg_home),
                         status_of=lambda lane, now: "failed", relauncher=relauncher)
    assert calls == [("muse-lane", "anthropic")]
    answers = inbox_mod.find_request(cfg_home, req["id"])["answers"]
    assert answers[-1]["text"] == "muse ran out of credits → continued with anthropic"
    assert answers[-1]["lane"] == "anthropic-lane"
    assert read_json(cfg_home / "lanes" / "muse-lane.json", None)["out_of_credits_at"]
    assert any("continued with anthropic" in r["text"] for r in read_jsonl(cfg_home / "flows.jsonl"))
    assert read_json(cfg_home / "dispatch.json", None)["fallbacks"]["muse-lane"] == "anthropic-lane"


def test_chain_is_exhausted_not_looped(cfg_home: Path, tmp_path: Path) -> None:
    config = load_config(cfg_home)
    log = _log(tmp_path / "m.log", ["HTTP 429"])
    second = _lane(cfg_home, "second", log, agent="anthropic", tried=["muse"])
    assert fallback.next_alias(config, second) is None
    calls: list = []
    state: dict = {}
    assert fallback.run(cfg_home, config, NOW, state, [second], lambda *a: calls.append(a) or "x")
    assert calls == [] and state["fallbacks"]["second"] == "none"
    # the first lane could still go to anthropic, but a lane that already tried it cannot go back
    first = _lane(cfg_home, "first", log, agent="muse", tried=["anthropic"])
    assert fallback.next_alias(config, first) is None


def test_no_relaunch_for_ordinary_failure_stop_old_lane_or_done(cfg_home: Path, tmp_path: Path) -> None:
    config = load_config(cfg_home)
    bad = _log(tmp_path / "b.log", ["HTTP 429"])
    lanes = [
        _lane(cfg_home, "plain", _log(tmp_path / "p.log", ["tests failed"])),
        _lane(cfg_home, "stopped", bad, stop={"by": "owner", "reason": "x", "at": _at(70)}),
        _lane(cfg_home, "old", bad, ended=7200),
        _lane(cfg_home, "done", bad, result="done"),
    ]
    calls: list = []
    fallback.run(cfg_home, config, NOW, {}, lanes, lambda *a: calls.append(a) or "x")
    assert calls == []


def test_a_failing_relaunch_is_remembered_not_retried(cfg_home: Path, tmp_path: Path) -> None:
    config = load_config(cfg_home)
    lane = _lane(cfg_home, "m", _log(tmp_path / "m.log", ["HTTP 429"]))
    state: dict = {}

    def boom(*args):
        raise launch_mod.LaunchError("no")
    assert fallback.run(cfg_home, config, NOW, state, [lane], boom)
    assert state["fallbacks"]["m"].startswith("error")
    assert not fallback.run(cfg_home, config, NOW, state, [lane], boom)


# -- status and the pick ------------------------------------------------------------------------


def test_status_and_pick_alias_skip_the_agent_that_ran_out(cfg_home: Path, tmp_path: Path) -> None:
    config = load_config(cfg_home)
    assert fallback.pick_alias(config, [], NOW, "demo", "1") == "muse"
    hit = {"id": "x", "agent": "muse", "out_of_credits_at": _at(1800)}
    assert fallback.status(config, [hit], NOW)["muse"] == {"state": "out", "since": _at(1800)}
    assert fallback.pick_alias(config, [hit], NOW, "demo", "1") == "anthropic"
    stale = {**hit, "out_of_credits_at": _at(7200)}  # over an hour ago: usable again
    assert fallback.status(config, [stale], NOW)["muse"]["state"] == "ok"
    assert fallback.pick_alias(config, [stale], NOW, "demo", "1") == "muse"
    both = [hit, {"id": "y", "agent": "anthropic", "out_of_credits_at": _at(60)}]
    assert fallback.pick_alias(config, both, NOW, "demo", "1") == "muse"  # all out: the first
    assert fallback.pick_alias(Config(), [], NOW) is None


# -- the real relaunch --------------------------------------------------------------------------


def test_relaunch_lane_reuses_a_clean_worktree_else_makes_a_new_one(cfg_home: Path, tmp_path: Path,
                                                                     monkeypatch) -> None:
    repo = Path(load_config(cfg_home).projects[0].repo)
    ws = launch_mod.create_worktree(repo, "muse-lane")
    brief = cfg_home / "brief.md"
    brief.write_text("# Lane: Thing\n", encoding="utf-8")
    lane = _lane(cfg_home, "muse-lane", "", workspace=str(ws), brief=str(brief))
    spawned: list[list[str]] = []
    monkeypatch.setattr(launch_mod, "spawn_detached", lambda argv, home, log: spawned.append(argv) or 1)

    new_id = relaunch_lane(cfg_home, lane, "anthropic")
    argv = spawned[-1]
    assert new_id != "muse-lane" and argv[argv.index("--workspace") + 1] == str(ws)
    assert argv[argv.index("--as-agent") + 1] == "anthropic"
    assert argv[argv.index("--fallback-from") + 1] == "muse-lane" and argv[argv.index("--tried") + 1] == "muse"
    assert "agent-a" in argv and argv[argv.index("--model") + 1] == "a-small"

    (ws / "scratch.txt").write_text("unfinished\n", encoding="utf-8")  # uncommitted work: do not reuse
    other = relaunch_lane(cfg_home, lane, "anthropic")
    assert spawned[-1][spawned[-1].index("--workspace") + 1] != str(ws)
    assert other != "muse-lane" and (ws.parent / other).is_dir()


def test_lane_run_records_agent_and_fallback_from(cfg_home: Path, tmp_path: Path,
                                                  monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("TIRITH_LANE", raising=False)
    monkeypatch.setattr("tirith.cli.verify_guard", lambda *a, **k: [])
    repo = Path(load_config(cfg_home).projects[0].repo)
    brief = cfg_home / "brief.md"
    brief.write_text("# Lane: Thing\n", encoding="utf-8")
    _lane(cfg_home, "first", "", agent="muse")
    code = main(["--home", str(cfg_home), "lane", "run", "--id", "second", "--project", "demo",
                 "--class", "stonewright", "--feature", "Thing", "--brief", str(brief), "--workspace", str(repo),
                 "--as-agent", "anthropic", "--fallback-from", "first", "--tried", "muse", "--step", "1", "--item", "Do the thing.", "--",
                 sys.executable, "-c", "print('ok')"])
    make_hooks_mutable(repo)
    assert code == 0
    lane = read_json(cfg_home / "lanes" / "second.json", None)
    assert (lane["agent"], lane["fallback_from"], lane["fallback_lane"], lane["tried"]) == (
        "anthropic", "muse", "first", ["muse"])
