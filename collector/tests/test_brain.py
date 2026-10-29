"""Brain jobs, cache, worker and fallbacks, all with a fake client."""

from __future__ import annotations

import json
import os
from pathlib import Path

from tirith import brain as brain_mod
from tirith.cli import register_lane
from tirith.collector import Collector
from tirith.home import Config


class FakeClient:
    """Counts calls and answers by prompt keyword, like a tiny model."""

    def __init__(self) -> None:
        self.calls: list[str] = []

    def chat(self, messages: list[dict], *, max_tokens: int = 512) -> str:
        prompt = messages[-1]["content"]
        self.calls.append(prompt)
        if "ONE plain sentence" in prompt:
            return "Writing the migration test."
        if "JSON array" in prompt:
            return json.dumps(["Approve the schema choice"])
        if "heroic epithet" in prompt:
            return "Ledgerhand"
        if "Map this project's architecture" in prompt:
            return json.dumps({
                "nodes": [{"id": "core", "label": "Core", "kind": "core", "files": 3},
                          {"id": "ui", "label": "UI", "kind": "ui", "files": 2}],
                "edges": [{"from": "core", "to": "ui", "label": "feeds"}]})
        return "ok"


def test_jobs_parse_model_replies() -> None:
    client = FakeClient()
    assert brain_mod.job_summary(client, "log lines") == "Writing the migration test."
    assert brain_mod.job_waiting(client, "log lines") == ["Approve the schema choice"]
    assert brain_mod.job_epithet(client, "did things", "archivist") == "Ledgerhand"
    arch = brain_mod.job_architecture(client, "a.py\nb.py")
    assert [n["id"] for n in arch["nodes"]] == ["core", "ui"]
    assert arch["edges"][0]["label"] == "feeds"


def test_collector_uses_brain_cache_and_epithet(home: Path, tmp_path: Path) -> None:
    log = tmp_path / "a.log"
    log.write_text("working on migrations\nneeds a decision soon\n", encoding="utf-8")
    register_lane(home, lane_id="a-1", project="demo", hero_class="archivist",
                  feature="Database", workspace=None, log=str(log),
                  pid=os.getpid(), brief=None)
    fake = FakeClient()
    collector = Collector(home, client=fake)
    first = collector.cycle()  # enqueues jobs, summary falls back this cycle
    assert first["brain"]["enabled"] is True
    assert first["agents"][0]["summary"] == "Working on Database."
    collector.worker.drain(timeout_s=10)
    second = collector.cycle()  # cached brain summary + waiting item appear
    assert second["agents"][0]["summary"] == "Writing the migration test."
    assert any(w.get("source") == "brain" and "Approve" in w["text"]
               for w in second["waiting_on_you"])
    collector.worker.drain(timeout_s=10)
    third = collector.cycle()  # epithet from the first brain summary lands
    assert third["agents"][0]["epithet"] == "Ledgerhand"
    calls_after = len(fake.calls)
    collector.cycle()  # unchanged inputs: cache hits, no new model calls
    collector.worker.drain(timeout_s=10)
    assert len(fake.calls) == calls_after
    cache_files = list((home / "brain").glob("*.json"))
    assert len(cache_files) >= 3


def test_architecture_generated_then_refreshed_on_big_change(home: Path, tmp_path: Path) -> None:
    from conftest import write_config
    repo = tmp_path / "proj"
    repo.mkdir()
    (repo / "a.py").write_text("x = 1\n", encoding="utf-8")
    (repo / "b.py").write_text("y = 2\n", encoding="utf-8")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo),
                         "color": "#123456"}])
    fake = FakeClient()
    collector = Collector(home, client=fake)
    collector.cycle()
    collector.worker.drain(timeout_s=10)
    state = collector.cycle()
    arch = state["projects"][0]["architecture"]
    assert arch["source"] == "muse" and [n["id"] for n in arch["nodes"]] == ["core", "ui"]
    arch_calls = sum("Map this project's" in c for c in fake.calls)
    for index in range(10):  # tree grows a lot: 2 -> 12 files
        (repo / f"extra{index}.py").write_text("z = 3\n", encoding="utf-8")
    collector.cycle()
    collector.worker.drain(timeout_s=10)
    assert sum("Map this project's" in c for c in fake.calls) > arch_calls


def test_no_key_means_brain_disabled(home: Path) -> None:
    assert brain_mod.make_client(Config()) is None
    collector = Collector(home)  # auto client: no env key in tests
    state = collector.cycle()
    assert state["brain"]["enabled"] is False
    assert state["brain"]["last_error"] is None


def test_brain_error_is_reported_not_raised(home: Path, tmp_path: Path) -> None:
    class BrokenClient:
        def chat(self, messages: list[dict], *, max_tokens: int = 512) -> str:
            raise brain_mod.BrainError("model API returned HTTP 500")

    log = tmp_path / "a.log"
    log.write_text("some work\n", encoding="utf-8")
    register_lane(home, lane_id="a-1", project="demo", hero_class="warden",
                  feature="Checks", workspace=None, log=str(log),
                  pid=os.getpid(), brief=None)
    collector = Collector(home, client=BrokenClient())
    collector.cycle()
    collector.worker.drain(timeout_s=10)
    state = collector.cycle()
    assert state["brain"]["enabled"] is True
    assert "HTTP 500" in (state["brain"]["last_error"] or "")
    assert state["agents"][0]["summary"] == "Working on Checks."  # fallback still works
