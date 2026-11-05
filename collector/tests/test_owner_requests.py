"""Owner requests: each chat request is tied to a plan item or an answer and listed with its status."""

from __future__ import annotations

from tirith import inbox as inbox_mod


def _req(rid, kind="chat", step="", answers=(), lanes=(), status="open", frm="owner"):
    return {"id": rid, "at": "2026-11-05T10:00:00Z", "kind": kind, "project": "demo", "step": step, "text": "t",
            "from": frm, "status": status, "answers": list(answers), "lanes": list(lanes)}


def test_owner_requests_link_to_plan_item_or_answer() -> None:
    ans = {"at": "2026-11-05T10:01:00Z", "by": "manager", "text": "ok", "auto": False, "lane": None}
    rows = [_req("a", "finish", step="3", status="answered", answers=[ans]),
            _req("b", answers=[ans], status="answered"),
            _req("c"),
            _req("d", "plan-new"),
            _req("e", "autopilot"),
            _req("f", lanes=["lane-1"])]
    got = {r["id"]: r for r in inbox_mod.owner_requests(rows)}
    assert "e" not in got
    assert got["a"]["link"] == {"type": "plan", "step": "3", "lanes": []}
    assert got["b"]["link"] == {"type": "answer", "by": "manager", "at": ans["at"]} and got["b"]["status"] == "answered"
    assert got["c"]["link"] is None and got["c"]["status"] == "open"
    assert got["d"]["link"]["type"] == "plan" and got["f"]["link"]["lanes"] == ["lane-1"]


def test_owner_requests_in_state(home) -> None:
    from datetime import datetime, timezone
    from tirith.collector import Collector
    from tirith.cli import main
    main(["manager", "hello"])
    inbox_mod.add_request(home, "chat", project="demo", text="why is nothing moving?")
    state = Collector(home, client=None).build_state(datetime(2026, 11, 5, tzinfo=timezone.utc))
    assert any(r["text"] == "why is nothing moving?" and r["status"] == "open" for r in state["owner_requests"])
