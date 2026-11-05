"""The Status card (tirith.statuscard) and the sentinel (tirith.sentinel): code first, a small model of any vendor."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

from tirith import ceo, models, sentinel, statuscard
from tirith.home import load_config

NOW = datetime(2026, 11, 5, 10, tzinfo=timezone.utc)


def _state() -> dict:
    return {
        "agents": [{"id": "w1", "project": "tirith", "step": "22", "status": "working", "summary": "writing tests"},
                   {"id": "w0", "project": "tirith", "step": "17", "status": "done", "item": "Fix the thing (check: `x`)",
                    "ended_at": "2026-11-05T09:40:00Z", "review": {"status": "merged", "at": "2026-11-05T09:50:00Z"}},
                   {"id": "manager-1", "status": "working"}],
        "projects": [{"id": "tirith", "roadmap": {"steps": [{"id": "22", "title": "Empresa que crece", "done": 6, "total": 16},
                                                           {"id": "17", "title": "Mejoras", "done": 3, "total": 3}]}}],
        "health": {"level": "amber", "checks": [{"id": "autopilot", "level": "healing", "text": "Items waiting."}]},
        "autopilot": {"open_items": 9, "limit": 4},
        "waiting_on_you": [{"id": "w-1", "text": "Install the guard in SuperPolitika"}],
        "inbox": [{"kind": "autopilot-proposal", "answers": [{"by": "manager", "at": "2026-11-05T09:55:00Z", "text": "wait"}]}],
    }


def test_the_status_card_says_in_four_lines_how_it_goes() -> None:
    st = statuscard.build(_state(), NOW, {"tirith/tirith": "passed"}, merged_today=67)
    assert (st["level"], st["why"], st["merged_today"], st["working"], st["limit"], st["ci"]) == \
        ("ok", "Moving", 67, 1, 4, "green")  # a short healing is normal: only a lasting one is amber
    assert st["steps"] == [{"project": "tirith", "step": "22", "title": "Empresa que crece", "done": 6, "total": 16, "agents": 1}]
    assert st["recent"][0]["text"] == "Fix the thing"  # the check note is left out
    assert st["owner_count"] == 1
    assert statuscard.build(_state(), NOW, {}, lasting=["autopilot"])["level"] == "warn"
    assert statuscard.build(_state(), NOW, {"tirith/tirith": "failed"})["level"] == "bad"
    idle = {**_state(), "agents": []}
    assert statuscard.build(idle, NOW, {})["why"] == "Nothing working while 9 plan items are open"


def test_the_sentinel_is_set_in_settings_for_any_vendor(home: Path) -> None:
    (home / "config.toml").write_text('[models]\nchoices = ["big-1", "spark-mini-1", "luna-2"]\n[models.workers]\nagent = "codex"\n')
    conf = sentinel.settings(home)
    assert (conf["agent"], conf["model"], conf["effort"], conf["every_min"], conf["enabled"]) == \
        ("codex", "spark-mini-1", "low", 5, True)  # the workers' agent and the cheapest-looking of its models
    models.set_models(home, "sentinel", agent="codex", model="luna-2", every_min="10")
    assert (sentinel.settings(home)["model"], sentinel.settings(home)["every_min"]) == ("luna-2", 10)
    models.set_models(home, "sentinel", enabled="off")
    assert sentinel.settings(home)["enabled"] is False
    assert models.state(load_config(home))["sentinel"]["enabled"] == "off"


def test_a_look_is_asked_only_when_due_and_something_changed(home: Path) -> None:
    (home / "config.toml").write_text('[models]\nchoices = ["tiny-haiku"]\n[models.workers]\nagent = "anthropic"\n')
    spawned = []
    state = {**_state(), "status": statuscard.build(_state(), NOW, {})}
    assert sentinel.tick(home, state, spawn=lambda *a: spawned.append(a), now=1000.0)
    assert not sentinel.tick(home, state, spawn=lambda *a: spawned.append(a), now=1010.0)  # one is running
    data = json.loads((home / sentinel.FILE).read_text())
    data.pop("running_since")
    (home / sentinel.FILE).write_text(json.dumps(data))
    assert not sentinel.tick(home, state, spawn=lambda *a: spawned.append(a), now=1000.0 + 6 * 60)  # nothing changed
    state["agents"][0]["summary"] = "stuck on the same error"
    assert sentinel.tick(home, state, spawn=lambda *a: spawned.append(a), now=1000.0 + 12 * 60)
    assert len(spawned) == 2 and spawned[0][0][-2:] == ["sentinel", "run"]


def test_any_output_format_gives_the_verdict() -> None:
    plain = 'Looks fine. {"wake": false, "why": "normal busy work"}'
    lines = '{"type":"result","result":"{\\"wake\\": true, \\"why\\": \\"the manager answers wait to everything\\"}"}'
    assert sentinel.parse(sentinel.answer_text(plain)) == {"wake": False, "why": "normal busy work"}
    assert sentinel.parse(sentinel.answer_text(lines)) == {"wake": True, "why": "the manager answers wait to everything"}
    assert sentinel.parse("no json at all") == {"wake": False, "why": ""}


def test_a_wake_alarms_the_ceo_once(home: Path) -> None:
    (home / "config.toml").write_text('[models]\nchoices = ["tiny-haiku"]\n')
    (home / "sentinel-input.json").write_text(json.dumps({"status": {}}))
    row = sentinel.run(home, call=lambda h, conf, prompt: ('{"wake": true, "why": "lanes fail the same way"}', 0.002))
    assert row["wake"] and row["cost_usd"] == 0.002
    book: dict = {}
    assert ceo.alarm_checks(home, book, NOW) == {"sentinel": "lanes fail the same way"}
    assert ceo.alarm_checks(home, book, NOW) == {}  # once per wake


def test_sentinel_counts_mergeq_as_work() -> None:
    idle = {**_state(), "agents": []}
    assert statuscard.build(idle, NOW, {})["why"] == "Nothing working while 9 plan items are open"
    queue = {"projects": [{"id": "tirith", "merge_queue": {"items": [{"lane": "w9", "status": "running"}]}}]}
    st = statuscard.build({**idle, **queue}, NOW, {})
    assert (st["level"], st["why"]) == ("ok", "Moving")
    review = {"agents": [{"id": "w8", "status": "done", "ended_at": "2026-11-05T09:40:00Z", "review": {"status": "running"}}]}
    assert statuscard.build({**idle, **review}, NOW, {})["why"] == "Moving"
    waiting = {"projects": [{"id": "tirith", "merge_queue": {"items": [{"lane": "w9", "status": "queued"}]}}]}
    assert statuscard.build({**idle, **waiting}, NOW, {})["level"] == "warn"


def test_the_card_says_all_done_when_nothing_is_left_for_the_agents() -> None:
    """Open items that only the owner, a date or a standing role can close: the plan reads as closed, not amber."""
    idle = {**_state(), "agents": [], "health": {"level": "green", "checks": []}}
    closed = {**idle, "autopilot": {"enabled": True, "open_items": 3, "queued": 0, "proposed": 0, "skipped": []}}
    st = statuscard.build(closed, NOW, {"tirith/tirith": "passed"})
    assert (st["level"], st["why"]) == ("done", "All done: nothing is left for the agents")
    assert st["plan"] == {"done": 9, "total": 19}
    held = {**closed, "autopilot": {**closed["autopilot"], "skipped": [{"project": "tirith", "step": "7", "why": "load"}]}}
    assert statuscard.build(held, NOW, {})["level"] == "warn"  # held only for now: still work to do
    canary = {**closed, "autopilot": {**closed["autopilot"], "skipped": [{"project": "canary", "step": "1", "why": "review"}]}}
    assert statuscard.build(canary, NOW, {})["level"] == "done"  # the hourly self-test does not reopen the plan
    off = {**closed, "autopilot": {**closed["autopilot"], "enabled": False}}
    assert statuscard.build(off, NOW, {})["level"] == "warn"  # autopilot off: nothing launches by itself
    assert statuscard.build(closed, NOW, {}, lasting=["disk"])["level"] == "warn"
    assert statuscard.build(closed, NOW, {"tirith/tirith": "failed"})["level"] == "bad"
    empty = {**closed, "autopilot": {**closed["autopilot"], "open_items": 0}}
    assert statuscard.build(empty, NOW, {})["level"] == "done"
