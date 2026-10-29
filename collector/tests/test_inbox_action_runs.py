"""When the owner runs an action the steward gave, the request is pending for the steward again."""

from __future__ import annotations

from tirith import inbox as inbox_mod
from tirith.store import append_jsonl


def _setup(home):
    rid = inbox_mod.add_request(home, "finish", "demo", "3", at="2026-10-29T07:25:00Z")["id"]
    append_jsonl(home / "inbox.jsonl", {"id": rid, "at": "2026-10-29T07:52:00Z", "kind": "answer",
                                        "by": "steward", "text": "Run it",
                                        "actions": [{"id": "act-1", "label": "Install", "command": "true"}]})
    return rid


def _req(home, rid):
    return next(r for r in inbox_mod.requests(home) if r["id"] == rid)


def test_an_action_result_reopens_the_request_for_the_steward(tmp_path):
    rid = _setup(tmp_path)
    assert _req(tmp_path, rid)["steward_pending"] is False
    append_jsonl(tmp_path / "inbox.jsonl", {"kind": "action-run", "action_id": "act-1", "status": "running",
                                            "at": "2026-10-29T08:00:00Z"})
    assert _req(tmp_path, rid)["steward_pending"] is False  # still running: nothing to answer yet
    append_jsonl(tmp_path / "inbox.jsonl", {"kind": "action-run", "action_id": "act-1", "status": "passed",
                                            "exit_code": 0, "at": "2026-10-29T08:00:05Z"})
    req = _req(tmp_path, rid)
    assert req["steward_pending"] is True and req["action_runs"][-1]["status"] == "passed"
    inbox_mod.add_answer(tmp_path, rid, "It worked.", at="2026-10-29T08:01:00Z")
    assert _req(tmp_path, rid)["steward_pending"] is False


def test_watch_emits_the_request_again_after_an_action_result(tmp_path):
    rid = _setup(tmp_path)
    emitted, rounds = [], iter(range(3))

    def stop():
        n = next(rounds)
        if n == 0:  # between the first and second poll the owner runs the action
            append_jsonl(tmp_path / "inbox.jsonl", {"kind": "action-run", "action_id": "act-1",
                                                    "status": "failed", "exit_code": 1,
                                                    "at": "2026-10-29T08:00:05Z"})
        return n >= 1

    inbox_mod.watch(tmp_path, emitted.append, poll_s=0, should_stop=stop, sleep=lambda s: None)
    assert [r["id"] for r in emitted] == [rid]
    assert '"action_run"' in inbox_mod.watch_line(emitted[0])
