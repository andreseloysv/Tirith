"""Keep going until done: a lane that ends incomplete is resumed with what is missing (tirith.keepgoing)."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

from conftest import make_hooks_mutable, write_config
from test_lane_run import _lane_run_setup
from tirith import asks as asks_mod
from tirith import gates
from tirith import keepgoing as keepgoing_mod
from tirith.cli import main
from tirith.home import load_config
from tirith.store import read_json

FAKE = '''
import json, os, subprocess, sys
here = os.path.dirname(os.path.abspath(__file__))
with open(os.path.join(here, "calls.jsonl"), "a") as f:
    f.write(json.dumps(sys.argv[1:]) + "\\n")
calls = sum(1 for _ in open(os.path.join(here, "calls.jsonl")))
mode = open(os.path.join(here, "mode")).read().strip()
resumed = "--resume" in sys.argv
open("work" + str(calls) + ".txt", "w").write("x")
if mode == "never" or (mode == "fix-on-resume" and not resumed):
    pass  # leaves the file uncommitted and gives no plan report
else:
    subprocess.run(["git", "add", "-A"], check=True)
    subprocess.run(["git", "-c", "user.email=a@b.c", "-c", "user.name=a", "commit", "-q", "--no-verify",
                    "-m", "work"], check=True)
    sys.path.insert(0, @ROOT@)
    from tirith.cli import main
    main(["plan", "report", "--status", "done"])
print(json.dumps({"type": "system", "session_id": "sess-1"}))
print(json.dumps({"type": "result", "result": "Finished.", "session_id": "sess-1"}))
'''.replace("@ROOT@", repr(str(Path(__file__).resolve().parent.parent)))


def _run(home: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch, mode: str, tests_green: bool = True):
    repo = _lane_run_setup(home, tmp_path, monkeypatch)
    agent = tmp_path / "agent"
    agent.mkdir()
    (agent / "fake.py").write_text(FAKE, encoding="utf-8")
    (agent / "mode").write_text(mode, encoding="utf-8")
    monkeypatch.setattr(keepgoing_mod.claimcheck_mod, "test_runs",
                        lambda *a: [{"at": 9e12, "counts": [], "run": "r"}] if tests_green else [])
    try:
        code = main(["lane", "run", "--project", "demo", "--class", "warden", "--feature", "Loop",
                     "--brief", str(repo / "brief.md"), "--step", "1", "--item", "Do the thing.",
                     "--id", "loop", "--workspace", str(repo), "--as-agent", "anthropic", "--",
                     sys.executable, str(agent / "fake.py"), "-p", "the brief", "--output-format",
                     "stream-json"])
    finally:
        make_hooks_mutable(repo)
    calls = [json.loads(l) for l in (agent / "calls.jsonl").read_text().splitlines()]
    return code, calls, read_json(home / "lanes" / "loop.json", {})


def test_missing_report_and_uncommitted_work_resume_the_same_session(home, tmp_path, monkeypatch) -> None:
    code, calls, lane = _run(home, tmp_path, monkeypatch, "fix-on-resume")
    assert code == 0 and len(calls) == 2
    first, second = calls
    assert first[:2] == ["-p", "the brief"] and "--resume" not in first
    assert second[0] == "-p" and "Missing:" in second[1] and "no plan report" in second[1]
    assert "uncommitted changes" in second[1] and "round 1 of 3" in second[1]
    assert second[second.index("--resume") + 1] == "sess-1"
    assert lane["outcome"]["status"] == "done" and lane["result"] == "done"
    assert keepgoing_mod.round_label(lane) == "round 1/3"
    assert not asks_mod.queue_items(home, only_open=True)


def test_three_rounds_then_one_manager_queue_item(home, tmp_path, monkeypatch) -> None:
    code, calls, lane = _run(home, tmp_path, monkeypatch, "never")
    assert len(calls) == 1 + keepgoing_mod.MAX_ROUNDS
    assert all("--resume" in c for c in calls[1:])
    (item,) = [i for i in asks_mod.queue_items(home, only_open=True) if i["kind"] == keepgoing_mod.QUEUE_KIND]
    assert item["id"] == "sq-incomplete-loop" and "no plan report" in item["text"]
    assert keepgoing_mod.round_label(lane) == "round 3/3"


def test_failing_tests_are_listed(home, tmp_path, monkeypatch) -> None:
    _, calls, _ = _run(home, tmp_path, monkeypatch, "fix-on-resume", tests_green=False)
    assert "no passing test run recorded" in calls[1][1]


def test_alias_without_resume_flag_starts_again_with_the_previous_report(home) -> None:
    write_config(home, [], extra='[agents.plain]\ncommand = "plainagent"\n')
    config = load_config(home)
    built = keepgoing_mod.resume_command(config, "plain", None, ["plainagent", "brief"], None,
                                         "Missing: x", "I did y", "THE BRIEF")
    assert built is not None
    argv, stdin = built
    assert argv[0] == "plainagent"
    text = stdin or argv[-1]
    assert "THE BRIEF" in text and "I did y" in text and "Missing: x" in text


def test_session_id_is_the_last_one_in_the_log(tmp_path) -> None:
    log = tmp_path / "a.log"
    log.write_text('{"session_id": "a"}\nnoise\n{"type":"result","session_id":"b"}\n', encoding="utf-8")
    assert keepgoing_mod.session_id(log) == "b" and keepgoing_mod.session_id(tmp_path / "none") is None


EXISTING_GATE_IDS = ("subagent-tool", "agent-cli", "guard-reinstall", "hooks-path", "no-verify", "git-hooks",
                     "raw-stage", "settings-edit", "roadmap-edit")


def test_existing_gate_ids_are_still_present_and_active(home) -> None:
    source = Path(gates.__file__).read_text(encoding="utf-8")
    for gate_id in EXISTING_GATE_IDS:
        assert f'"{gate_id}"' in source, gate_id
    config = load_config(home)
    got = {v.rule for v in gates.check_call(config, "Bash", {"command": "git commit --no-verify -m x"}, "/tmp")}
    assert "no-verify" in got
