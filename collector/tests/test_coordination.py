"""Real SQLite transactions, competing workers, restart recovery and mailbox delivery."""

from concurrent.futures import ThreadPoolExecutor
from datetime import date
import json
import os
from pathlib import Path
import threading

import pytest

from tirith import autopilot, coordination, roadmap
from tirith.store import write_json_atomic


def item(title, task_id, needs=(), writes=(), done=False):
    return f"- [{'x' if done else ' '}] " + roadmap.with_metadata(title, {
        "id": task_id, "depends_on": list(needs), "writes": list(writes)}) + "\n"


@pytest.fixture
def repo(tmp_path):
    folder = tmp_path / "project"
    (folder / "docs").mkdir(parents=True)
    (folder / "docs/TODO.md").write_text("## 1. Work\n\n" + item("First", "i-11111111", writes=["src/api"]) +
                                         item("Second", "i-22222222", needs=["i-11111111"], writes=["src/ui"]))
    return folder


def lane(owner, claim):
    return {"id": owner, "project": "demo", **claim}


def test_permanent_id_survives_rename_and_status_changes():
    text = "## 1. Work\n\n" + item("Original", "i-11111111", writes=["src"])
    renamed = roadmap.edit(text, "1", "Original", "Renamed")
    started = roadmap.set_item(renamed, "1", "i-11111111", "Renamed (en curso: worker)")
    task = roadmap.parse(started)[0]["items"][0]
    assert task["id"] == "i-11111111"
    assert task["writes"] == ["src"]
    assert task["text"] == "Renamed (en curso: worker)"
    assert roadmap.find_item(roadmap.parse(started), "1", "i-11111111") == task


def test_renaming_a_legacy_item_freezes_its_original_id():
    text = "## 1. Work\n\n- [ ] Original\n"
    changed = roadmap.edit(text, "1", "Original", "New description")
    assert roadmap.parse(changed)[0]["items"][0]["id"] == roadmap.item_id("Original")


def test_repeated_legacy_titles_get_distinct_permanent_ids():
    text = "## 1. First\n\n- [ ] Same title\n\n## 2. Second\n\n- [ ] Same title\n"
    text = roadmap.edit(text, "1", "Same title", "Renamed first")
    text = roadmap.edit(text, "2", "Same title", "Renamed second")
    tasks = coordination.plan_rows(roadmap.parse(text))
    assert tasks[0]["id"] != tasks[1]["id"]


def test_legacy_repeated_titles_can_be_claimed_in_different_steps(home, repo):
    (repo / "docs/TODO.md").write_text("## 1. First\n\n- [ ] Work\n\n## 2. Second\n\n- [ ] Work\n")
    first = coordination.claim(home, repo, "demo", "1", "Work", "one")
    second = coordination.claim(home, repo, "demo", "2", "Work", "two")
    assert first["task_id"] != second["task_id"]


def test_competing_claims_have_exactly_one_owner(home, repo):
    barrier = threading.Barrier(2)

    def claim(owner):
        barrier.wait(timeout=5)
        try:
            return coordination.claim(home, repo, "demo", "1", "First", owner)
        except coordination.CoordinationError:
            return None

    with ThreadPoolExecutor(max_workers=2) as pool:
        results = list(pool.map(claim, ["one", "two"]))
    assert sum(result is not None for result in results) == 1
    with coordination.database(home) as db:
        assert db.execute("SELECT COUNT(*) FROM tasks WHERE owner IS NOT NULL").fetchone()[0] == 1


def test_claim_is_separate_from_start_and_same_owner_is_idempotent(home, repo):
    claim = coordination.claim(home, repo, "demo", "1", "First", "one")
    assert coordination.claim(home, repo, "demo", "1", "First", "one") == claim
    with coordination.database(home) as db:
        assert db.execute("SELECT phase FROM tasks WHERE owner='one'").fetchone()[0] == "claimed"
    coordination.update(home, lane("one", claim), phase="running")
    with coordination.database(home) as db:
        assert db.execute("SELECT phase FROM tasks WHERE owner='one'").fetchone()[0] == "running"


def test_dependencies_wait_for_integration_not_worker_report(home, repo):
    claim = coordination.claim(home, repo, "demo", "1", "First", "one")
    coordination.update(home, lane("one", claim), phase="verifying")
    with pytest.raises(coordination.CoordinationError, match="prerequisites"):
        coordination.claim(home, repo, "demo", "1", "Second", "two")
    path = repo / "docs/TODO.md"
    path.write_text(path.read_text().replace("- [ ] First", "- [x] First"))
    assert coordination.claim(home, repo, "demo", "1", "Second", "two")["task_id"] == "i-22222222"


@pytest.mark.parametrize("needs", [["i-99999999"], ["i-11111111"]])
def test_missing_and_cyclic_dependencies_fail_closed(home, repo, needs):
    path = repo / "docs/TODO.md"
    path.write_text("## 1. Work\n\n" + item("First", "i-11111111", needs=needs))
    with pytest.raises(coordination.CoordinationError):
        coordination.claim(home, repo, "demo", "1", "First", "one")


def test_overlapping_writes_are_reserved_across_steps(home, repo):
    path = repo / "docs/TODO.md"
    path.write_text(path.read_text() + "\n## 2. Other\n\n" + item("Conflict", "i-33333333", writes=["src/api/client.py"]))
    coordination.claim(home, repo, "demo", "1", "First", "one")
    with pytest.raises(coordination.CoordinationError, match="write paths"):
        coordination.claim(home, repo, "demo", "2", "Conflict", "two")


def test_crash_recovery_fences_the_old_attempt(home, repo):
    first = coordination.claim(home, repo, "demo", "1", "First", "one", now=100)
    assert coordination.recover(home, now=701) == ["i-11111111"]
    replacement = coordination.claim(home, repo, "demo", "1", "First", "two", now=702)
    assert replacement["task_attempt"] != first["task_attempt"]
    with pytest.raises(coordination.CoordinationError, match="stale report"):
        coordination.update(home, lane("one", first), phase="verifying")


def test_expired_live_worker_and_verification_keep_their_claims(home, repo):
    claim = coordination.claim(home, repo, "demo", "1", "First", "one", now=100)
    write_json_atomic(home / "lanes/one.json", {"project": "demo", "pid": os.getpid()})
    assert coordination.recover(home, now=701) == []
    coordination.update(home, lane("one", claim), phase="verifying", now=702)
    write_json_atomic(home / "lanes/one.json", {"project": "demo", "ended_at": "now"})
    write_json_atomic(home / "mergeq.json", {"queue": [{"project": "demo", "lane": "one"}]})
    assert coordination.recover(home, now=2000) == []


def test_orphaned_verification_recovers_when_no_queue_owns_it(home, repo):
    first = coordination.claim(home, repo, "demo", "1", "First", "one", now=100)
    coordination.update(home, lane("one", first), phase="verifying", now=100)
    assert coordination.recover(home, now=701) == ["i-11111111"]


def test_live_controller_keeps_ownership_while_waiting_for_stages(home, repo):
    coordination.claim(home, repo, "demo", "1", "First", "one", now=100)
    write_json_atomic(home / "lanes/one.json", {"project": "demo", "controller_pid": os.getpid()})
    assert coordination.recover(home, now=701) == []


def test_unreadable_worker_state_does_not_transfer_ownership(home, repo):
    coordination.claim(home, repo, "demo", "1", "First", "one", now=100)
    path = home / "lanes/one.json"
    path.parent.mkdir(exist_ok=True)
    path.write_text("{broken")
    assert coordination.recover(home, now=701) == []


def test_messages_acknowledge_only_the_delivered_snapshot_and_deduplicate(home):
    for name in ("one", "two"):
        write_json_atomic(home / f"lanes/{name}.json", {"project": "demo"})
    first = coordination.send(home, "demo", "one", "two", "First", "delivery-1")
    delivered = coordination.inbox(home, "demo", "two")
    second = coordination.send(home, "demo", "one", "two", "Later", "delivery-2")
    coordination.acknowledge(home, "demo", "two", [row["id"] for row in delivered])
    assert [row["id"] for row in coordination.inbox(home, "demo", "two")] == [second]
    assert coordination.send(home, "demo", "one", "two", "First", "delivery-1") == first
    assert len(coordination.inbox(home, "demo", "two")) == 1
    with pytest.raises(coordination.CoordinationError, match="different message"):
        coordination.send(home, "demo", "one", "two", "Changed", "delivery-1")
    with pytest.raises(coordination.CoordinationError, match="project"):
        coordination.send(home, "other", "one", "two", "Secret", "delivery-3")


@pytest.mark.parametrize("identity", [{"item": "First"}, {"task_id": "i-11111111"}])
def test_ready_items_in_a_busy_step_are_independent(home, repo, identity):
    path = repo / "docs/TODO.md"
    path.write_text(path.read_text() + item("Independent", "i-33333333", writes=["docs"]))
    road = roadmap.build(repo, "demo", [], date(2026, 11, 2))
    projects = [{"id": "demo", "roadmap": road}]
    agents = [{"id": "one", "project": "demo", "step": "1", "status": "working", **identity}]
    ready, _ = autopilot.candidates(projects, agents, home=home)
    assert [row["item"] for row in ready.values()] == ["Independent"]


def test_task_state_survives_new_database_connections(home, repo):
    first = coordination.claim(home, repo, "demo", "1", "First", "one")
    with coordination.database(home) as db:
        assert db.execute("SELECT attempt FROM tasks WHERE owner='one'").fetchone()[0] == first["task_attempt"]
    with pytest.raises(coordination.CoordinationError, match="already claimed"):
        coordination.claim(home, repo, "demo", "1", "First", "two")


def test_failed_process_start_releases_the_task(home, repo, monkeypatch):
    from conftest import write_config
    from tirith import cli
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}])
    brief = repo / "brief.md"
    brief.write_text("Work on First")
    monkeypatch.setattr(cli, "_lane_run_refusal", lambda *a: "")
    monkeypatch.setattr(cli, "_plan_begin", lambda *a: None)

    def failed(*args, **kwargs):
        raise FileNotFoundError("agent executable missing")

    monkeypatch.setattr(cli, "_run_child", failed)
    with pytest.raises(FileNotFoundError):
        cli.main(["lane", "run", "--project", "demo", "--step", "1", "--item", "First",
                  "--id", "one", "--class", "warden", "--feature", "First", "--brief", str(brief),
                  "--workspace", str(repo), "--", "missing-agent"])
    assert coordination.reservations(home, "demo") == []


def test_claim_uses_final_lane_name_when_a_branch_is_retained(home, repo, tmp_path, monkeypatch):
    from conftest import write_config
    from tirith import cli
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}])
    brief = repo / "brief.md"
    brief.write_text("First")
    workspace = tmp_path / "worker-copy"
    workspace.mkdir()
    monkeypatch.setattr(cli, "_lane_run_refusal", lambda *a: "")
    monkeypatch.setattr(cli, "_plan_begin", lambda *a: None)
    monkeypatch.setattr(cli.launch_mod, "branch_exists", lambda repo, branch: branch == "lane/first")
    monkeypatch.setattr(cli, "_lane_workspace", lambda *a: workspace)

    def run(home, record, *args):
        assert record["id"] == "first-2"
        coordination.update(home, record, phase="running")
        return 0, "Finished"

    monkeypatch.setattr(cli, "_run_child", run)
    assert cli.main(["lane", "run", "--project", "demo", "--step", "1", "--item", "First",
                     "--class", "warden", "--feature", "First", "--brief", str(brief), "--", "agent"]) == 0


def test_worker_hook_delivers_without_acknowledging(home, capsys):
    from tirith import hooks
    for name in ("one", "two"):
        write_json_atomic(home / f"lanes/{name}.json", {"id": name, "project": "demo"})
    coordination.send(home, "demo", "one", "two", "Contract updated", "api-change")
    assert hooks.decide("anthropic", {"lane": "two", "event": "PostToolUse"}, {}, home) == 0
    output = json.loads(capsys.readouterr().out)
    assert "Contract updated" in output["hookSpecificOutput"]["additionalContext"]
    assert len(coordination.inbox(home, "demo", "two")) == 1


def test_stale_worker_hook_is_blocked_before_any_tool(home, repo, monkeypatch, capsys):
    from tirith import hooks
    first = coordination.claim(home, repo, "demo", "1", "First", "one")
    write_json_atomic(home / "lanes/one.json", lane("one", first))
    monkeypatch.setenv("TIRITH_TASK_ATTEMPT", "old-attempt")
    assert hooks.decide("anthropic", {"lane": "one", "event": "PreToolUse"}, {}, home) == 2
    assert "stale" in capsys.readouterr().err


def test_stale_cli_heartbeat_cannot_renew_a_reused_lane(home, repo, monkeypatch, capsys):
    from tirith.cli import main
    claim = coordination.claim(home, repo, "demo", "1", "First", "one")
    write_json_atomic(home / "lanes/one.json", lane("one", claim))
    monkeypatch.setenv("TIRITH_LANE", "one")
    monkeypatch.setenv("TIRITH_TASK_ATTEMPT", "older-attempt")
    assert main(["task", "heartbeat", "--project", "demo", "--lane", "one"]) == 3
    assert "stale" in capsys.readouterr().err


def test_recovery_preserves_workspace_and_marks_dead_lane_ended(home, repo):
    first = coordination.claim(home, repo, "demo", "1", "First", "one", now=100)
    write_json_atomic(home / "lanes/one.json", {**lane("one", first), "workspace": str(repo)})
    coordination.recover(home, now=701)
    recovered = json.loads((home / "lanes/one.json").read_text())
    assert recovered["ended_at"] and recovered["result"] == "failed"
    assert recovered["workspace"] == str(repo) and (repo / "docs/TODO.md").exists()


def test_message_ids_are_not_reused_after_retention_cleanup(home, monkeypatch):
    monkeypatch.setattr(coordination, "MAX_ACKNOWLEDGED_MESSAGES", 0)
    for name in ("one", "two"):
        write_json_atomic(home / f"lanes/{name}.json", {"project": "demo"})
    first = coordination.send(home, "demo", "one", "two", "Old", "old-key")
    coordination.acknowledge(home, "demo", "two", [first])
    second = coordination.send(home, "demo", "one", "two", "New", "new-key")
    assert second > first
    coordination.acknowledge(home, "demo", "two", [first])
    assert [row["id"] for row in coordination.inbox(home, "demo", "two")] == [second]


def test_metadata_cannot_override_checkboxes_and_stays_readable():
    steps = roadmap.parse('## 1. Work\n\n- [ ] Work <!-- tirith:{"id":"i-11111111","done":true} -->\n')
    assert not steps[0]["items"][0]["done"]
    with pytest.raises(coordination.CoordinationError, match="metadata"):
        coordination.plan_rows(steps)


def test_cli_configures_legacy_tasks_and_lists_blockers(home, tmp_path, capsys):
    from conftest import make_repo, write_config, write_plan
    from tirith.cli import main
    repo = make_repo(tmp_path / "git-project")
    write_plan(repo, "## 1. Work\n\n- [ ] First\n- [ ] Second\n")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}])
    assert main(["plan", "configure", "--project", "demo", "--step", "1", "--item", "First", "--writes", "src/api"]) == 0
    first_id = roadmap.item_id("First")
    assert main(["plan", "configure", "--project", "demo", "--step", "1", "--item", "Second", "--depends-on", first_id, "--writes", "web"]) == 0
    capsys.readouterr()
    assert main(["task", "list", "--project", "demo"]) == 0
    tasks = {row["text"]: row for row in json.loads(capsys.readouterr().out)}
    assert tasks["First"]["ready"]
    assert not tasks["Second"]["ready"] and tasks["Second"]["blocked_by"] == [first_id]
    path = repo / "docs/TODO.md"
    before = path.read_text()
    assert main(["plan", "configure", "--project", "demo", "--step", "1", "--item", "Second", "--depends-on", "i-99999999"]) == 3
    assert path.read_text() == before
    assert main(["plan", "configure", "--project", "demo", "--step", "1", "--item", "Second", "--depends-on", "--writes", "src/api/client.py"]) == 0
    coordination.claim(home, repo, "demo", "1", "First", "one")
    capsys.readouterr()
    assert main(["task", "list", "--project", "demo"]) == 0
    tasks = {row["text"]: row for row in json.loads(capsys.readouterr().out)}
    assert not tasks["Second"]["ready"] and tasks["Second"]["reserved_by"] == [first_id]


def _ready(repo, home, agents=()):
    road = roadmap.build(repo, "demo", [], date(2026, 11, 2))
    found, _ = autopilot.candidates([{"id": "demo", "roadmap": road}], list(agents), home=home)
    return [row["item"] for row in found.values()]


def test_conflicts_two_ready_items_on_the_same_files_are_not_launched_together(home, repo):
    (repo / "docs/TODO.md").write_text("## 1. Work\n\n" + item("A", "i-aaaaaaaa", writes=["src/api"]) +
                                       item("B", "i-bbbbbbbb", writes=["src/api/client.py"]) +
                                       item("C", "i-cccccccc", writes=["docs"]))
    assert _ready(repo, home) == ["A", "C"]


def test_conflicts_an_item_waits_while_a_working_lane_touches_its_files(home, repo):
    (repo / "docs/TODO.md").write_text("## 1. Work\n\n" + item("A", "i-aaaaaaaa", writes=["src"]) +
                                       item("B", "i-bbbbbbbb", writes=["src/ui"]) +
                                       item("C", "i-cccccccc", writes=["docs"]))
    agents = [{"id": "one", "project": "demo", "step": "1", "status": "working", "task_id": "i-aaaaaaaa"}]
    assert _ready(repo, home, agents) == ["C"]
