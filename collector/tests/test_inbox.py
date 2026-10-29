"""The owner's inbox: file format, CLI, watcher, tirith://inbox link, state.inbox and the brain answer."""

from __future__ import annotations

import json
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pytest

from conftest import make_repo, write_config, write_lane_manifest
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith.cli import main, register_lane
from tirith.collector import Collector
from tirith.store import read_jsonl


def _add(capsys, *argv: str) -> str:
    assert main(["inbox", "add", *argv]) == 0
    return capsys.readouterr().out.strip()


def test_add_writes_one_open_row(home: Path, capsys) -> None:
    rid = _add(capsys, "--kind", "ask", "--project", "demo", "--step", "2", "--text", "What is blocking?")
    (row,) = read_jsonl(home / "inbox.jsonl")
    assert row["id"] == rid and row["kind"] == "ask" and row["status"] == "open"
    assert (row["project"], row["step"], row["text"], row["from"]) == ("demo", "2", "What is blocking?", "owner")
    assert row["at"].endswith("Z")


def test_finish_needs_no_text_but_ask_does(home: Path, capsys) -> None:
    assert _add(capsys, "--kind", "finish", "--project", "demo", "--step", "1")
    assert main(["inbox", "add", "--kind", "ask", "--project", "demo", "--step", "1"]) == 3
    assert "needs text" in capsys.readouterr().err
    assert len(read_jsonl(home / "inbox.jsonl")) == 1


def test_text_is_cut_to_4_kb_on_a_character_boundary(home: Path) -> None:
    row = inbox_mod.add_request(home, "ask", "demo", "1", "é" * 3000)
    assert len(row["text"].encode()) <= 4096 and row["text"].endswith("é")


def test_answer_done_and_list(home: Path, capsys) -> None:
    first = _add(capsys, "--kind", "finish", "--project", "demo", "--step", "1")
    second = _add(capsys, "--kind", "ask", "--project", "demo", "--step", "2", "--text", "why?")
    assert main(["inbox", "answer", second, "**Because** of the tests."]) == 0
    assert main(["inbox", "done", first]) == 0
    capsys.readouterr()
    by_id = {r["id"]: r for r in inbox_mod.requests(home)}
    assert by_id[first]["status"] == "done"
    assert by_id[second]["status"] == "answered"
    assert by_id[second]["answers"][0]["by"] == "steward" and by_id[second]["answers"][0]["auto"] is False
    assert main(["inbox", "list", "--open"]) == 0
    assert capsys.readouterr().out.strip() == ""
    assert main(["inbox", "list", "--json"]) == 0
    assert [json.loads(l)["id"] for l in capsys.readouterr().out.splitlines()] == [first, second]
    rows = read_jsonl(home / "inbox.jsonl")  # append-only: the requests are never rewritten
    assert [r["kind"] for r in rows] == ["finish", "ask", "answer", "done"]


def test_answer_by_brain_and_errors(home: Path, capsys) -> None:
    rid = _add(capsys, "--kind", "ask", "--project", "demo", "--step", "2", "--text", "why?")
    assert main(["inbox", "answer", rid, "Auto.", "--by", "brain"]) == 0
    assert inbox_mod.requests(home)[0]["answers"][0]["by"] == "brain"
    assert main(["inbox", "answer", "in-nope", "x"]) == 3
    assert main(["inbox", "done", "in-nope"]) == 3
    assert "no request" in capsys.readouterr().err


def test_list_open_only(home: Path, capsys) -> None:
    done = _add(capsys, "--kind", "finish", "--project", "demo", "--step", "1")
    open_ = _add(capsys, "--kind", "finish", "--project", "demo", "--step", "2")
    main(["inbox", "done", done])
    capsys.readouterr()
    assert main(["inbox", "list", "--open"]) == 0
    out = capsys.readouterr().out
    assert open_ in out and done not in out


def test_watch_emits_open_requests_then_new_ones(home: Path) -> None:
    old = inbox_mod.add_request(home, "finish", "demo", "1")
    answered = inbox_mod.add_request(home, "ask", "demo", "2", "q")
    inbox_mod.add_answer(home, answered["id"], "a")
    seen: list[dict] = []
    ticks = {"n": 0}

    def sleep(_s: float) -> None:
        ticks["n"] += 1
        if ticks["n"] == 1:
            seen.append({"added": inbox_mod.add_request(home, "ask", "demo", "3", "new one")["id"]})

    inbox_mod.watch(home, seen.append, sleep=sleep, should_stop=lambda: ticks["n"] >= 2)
    ids = [r.get("id") or r["added"] for r in seen]
    assert ids[0] == old["id"] and answered["id"] not in ids
    assert seen[-1]["id"] == seen[-2]["added"]  # emitted once, on the next read
    assert json.loads(inbox_mod.watch_line(seen[0]))["kind"] == "finish"


def test_watch_new_only_skips_existing(home: Path) -> None:
    inbox_mod.add_request(home, "finish", "demo", "1")
    seen: list[dict] = []
    inbox_mod.watch(home, seen.append, include_existing=False, should_stop=lambda: True)
    assert seen == []


def test_broken_lines_and_orphan_answers_are_ignored(home: Path) -> None:
    rid = inbox_mod.add_request(home, "finish", "demo", "1")["id"]
    with (home / "inbox.jsonl").open("a", encoding="utf-8") as fh:
        fh.write("not json\n" + json.dumps({"id": "ghost", "kind": "answer", "text": "x"}) + "\n")
    assert [r["id"] for r in inbox_mod.requests(home)] == [rid]


# -- tirith://inbox -----------------------------------------------------------------------------


def test_url_roundtrip() -> None:
    url = inbox_mod.build_url("ask", "my proj", "3", "what & why?", "t0k")
    assert inbox_mod.parse_url(url) == {"kind": "ask", "project": "my proj", "step": "3",
                                        "text": "what & why?", "token": "t0k"}
    with pytest.raises(inbox_mod.InboxError):
        inbox_mod.parse_url("tirith://launch?project=a&step=1")


def test_url_open_inbox_with_token_adds_a_request(home: Path) -> None:
    token = launch_mod.launch_token(home)
    assert main(["url", "open", inbox_mod.build_url("ask", "demo", "2", "how is it going?", token)]) == 0
    (req,) = inbox_mod.requests(home)
    assert (req["kind"], req["project"], req["step"], req["text"]) == ("ask", "demo", "2", "how is it going?")


@pytest.mark.parametrize("token", ["wrong-token-wrong-token", ""])
def test_url_open_inbox_with_bad_token_is_refused(home: Path, capsys, token: str) -> None:
    launch_mod.launch_token(home)
    assert main(["url", "open", inbox_mod.build_url("finish", "demo", "2", "", token)]) == 3
    assert inbox_mod.requests(home) == []
    assert "token" in capsys.readouterr().err


# -- collector ----------------------------------------------------------------------------------

PLAN = "# TODO\n\n## 1. First\n\n- [x] Done thing.\n- [ ] Open thing.\n"
NOW = datetime(2026, 10, 29, 12, 0, tzinfo=timezone.utc)


class AnswerClient:
    def __init__(self) -> None:
        self.prompts: list[str] = []

    def chat(self, messages: list[dict], *, max_tokens: int = 512) -> str:
        if "owner of a project asks" not in messages[-1]["content"]:
            return "{}"  # other brain jobs (the architecture map) are not under test
        self.prompts.append(messages[-1]["content"])
        return "The step has one open item; no agent is on it."


@pytest.fixture
def project(home: Path, tmp_path: Path):
    repo = make_repo(tmp_path / "demo")
    write_lane_manifest(repo)
    (repo / "docs").mkdir()
    (repo / "docs" / "TODO.md").write_text(PLAN, encoding="utf-8")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}])
    return repo


def _at(seconds_ago: int) -> str:
    return (NOW - timedelta(seconds=seconds_ago)).strftime("%Y-%m-%dT%H:%M:%SZ")


def test_state_inbox_folds_rows_and_keeps_the_last_100(home: Path) -> None:
    for n in range(105):
        inbox_mod.add_request(home, "finish", "demo", str(n))
    answered = inbox_mod.add_request(home, "ask", "demo", "1", "q")
    inbox_mod.add_answer(home, answered["id"], "a", by="steward")
    state = Collector(home, client=None).build_state(NOW)
    assert len(state["inbox"]) == 100
    assert state["inbox"][-1]["status"] == "answered"
    assert state["inbox"][-1]["answers"] == [{"at": state["inbox"][-1]["answers"][0]["at"], "by": "steward",
                                              "text": "a", "auto": False, "lane": None}]
    assert state["inbox"][-1]["lanes"] == []


def test_state_inbox_validates_against_the_schema(home: Path) -> None:
    jsonschema = pytest.importorskip("jsonschema")
    rid = inbox_mod.add_request(home, "ask", "demo", "1", "q")["id"]
    inbox_mod.add_answer(home, rid, "a", by="brain", auto=True)
    schema = json.loads((Path(__file__).resolve().parents[1] / "state.schema.json").read_text(encoding="utf-8"))
    jsonschema.validate(Collector(home, client=None).build_state(NOW), schema)


def test_brain_answers_an_unanswered_question_after_120_s(project, home: Path) -> None:
    cfg = home / "config.toml"
    cfg.write_text(cfg.read_text() + "\n[dispatch]\nenabled = false\n")
    client = AnswerClient()
    collector = Collector(home, client=client)
    old = inbox_mod.add_request(home, "ask", "demo", "1", "What is left?", at=_at(130))["id"]
    young = inbox_mod.add_request(home, "ask", "demo", "1", "And now?", at=_at(30))["id"]
    finish = inbox_mod.add_request(home, "finish", "demo", "1", at=_at(500))["id"]
    collector.build_state(NOW)
    collector.worker.drain(5)
    state = collector.build_state(NOW)
    by_id = {r["id"]: r for r in state["inbox"]}
    assert by_id[old]["status"] == "answered"
    assert by_id[old]["answers"][0]["by"] == "brain" and by_id[old]["answers"][0]["auto"] is True
    assert "open item" in by_id[old]["answers"][0]["text"]
    assert by_id[young]["status"] == "open" and by_id[finish]["status"] == "open"
    (prompt,) = client.prompts
    assert "Step 1: First" in prompt and "[ ] Open thing." in prompt and "[x] Done thing." in prompt
    assert "What is left?" in prompt
    collector.build_state(NOW)  # answered once only
    collector.worker.drain(5)
    assert len(client.prompts) == 1
    assert len([r for r in read_jsonl(home / "inbox.jsonl") if r["kind"] == "answer"]) == 1


def test_a_steward_answer_stops_the_brain(project, home: Path) -> None:
    client = AnswerClient()
    collector = Collector(home, client=client)
    rid = inbox_mod.add_request(home, "ask", "demo", "1", "What is left?", at=_at(300))["id"]
    inbox_mod.add_answer(home, rid, "Handled by the steward.")
    collector.build_state(NOW)
    collector.worker.drain(5)
    assert client.prompts == []


def test_no_brain_no_auto_answer(project, home: Path) -> None:
    inbox_mod.add_request(home, "ask", "demo", "1", "What is left?", at=_at(300))
    state = Collector(home, client=None).build_state(NOW)
    assert state["inbox"][0]["status"] == "open" and state["inbox"][0]["answers"] == []


# -- the link between requests and lanes ----------------------------------------------------------


def test_answer_lane_is_stored_and_listed(home: Path, capsys) -> None:
    rid = _add(capsys, "--kind", "finish", "--project", "demo", "--step", "1")
    assert main(["inbox", "answer", rid, "Taken.", "--lane", "web-panel"]) == 0
    assert read_jsonl(home / "inbox.jsonl")[-1]["lane"] == "web-panel"
    assert inbox_mod.requests(home)[0]["answers"][0]["lane"] == "web-panel"
    assert main(["inbox", "answer", rid, "Plain."]) == 0
    assert "lane" not in read_jsonl(home / "inbox.jsonl")[-1]


def test_lane_start_and_run_store_request_id(home: Path, capsys) -> None:
    assert main(["lane", "start", "--project", "demo", "--class", "warden", "--feature", "x", "--id", "l1",
                 "--request", "in-abc12345"]) == 0
    assert main(["lane", "start", "--project", "demo", "--class", "warden", "--feature", "y", "--id", "l2"]) == 0
    capsys.readouterr()
    assert json.loads((home / "lanes" / "l1.json").read_text())["request_id"] == "in-abc12345"
    assert json.loads((home / "lanes" / "l2.json").read_text())["request_id"] is None


def test_state_inbox_lanes_come_from_answers_and_agents(home: Path) -> None:
    jsonschema = pytest.importorskip("jsonschema")
    asked = inbox_mod.add_request(home, "finish", "demo", "1")["id"]
    plain = inbox_mod.add_request(home, "finish", "demo", "2")["id"]
    inbox_mod.add_answer(home, asked, "Taken.", lane="old-lane")
    for lane_id, request_id in (("new-lane", asked), ("old-lane", asked), ("other", None)):
        register_lane(home, lane_id=lane_id, project="demo", hero_class="warden", feature=lane_id,
                      workspace=None, log=None, pid=None, brief=None, step="1", request_id=request_id)
    state = Collector(home, client=None).build_state(NOW)
    by_id = {r["id"]: r for r in state["inbox"]}
    assert by_id[asked]["lanes"] == ["old-lane", "new-lane"]
    assert by_id[asked]["answers"][0]["lane"] == "old-lane"
    assert by_id[plain]["lanes"] == []
    assert {a["id"]: a["request_id"] for a in state["agents"]} == {"new-lane": asked, "old-lane": asked, "other": None}
    schema = json.loads((Path(__file__).resolve().parents[1] / "state.schema.json").read_text(encoding="utf-8"))
    jsonschema.validate(state, schema)
