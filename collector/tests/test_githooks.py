"""Installed git hooks record each run and run tirith.toml [[hooks]]; temp repos only."""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

from conftest import GIT, make_hooks_mutable, make_repo, write_config
from tirith.cli import main
from tirith.collector import Collector
import tirith_guard

WORD = "bannedword"
IDENT = ["-c", "user.email=t@example.invalid", "-c", "user.name=Tirith Test"]


@pytest.fixture
def env(home: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    """A guarded temp repo, a `tirith` shim first in PATH and the repo as project `demo`."""
    shim = tmp_path / "bin"
    shim.mkdir()
    (shim / "tirith").write_text(f'#!/bin/sh\nexec "{sys.executable}" -m tirith "$@"\n')
    (shim / "tirith").chmod(0o755)
    monkeypatch.setenv("PATH", f"{shim}{os.pathsep}{os.environ['PATH']}")
    monkeypatch.setenv("PYTHONPATH", str(Path(__file__).resolve().parent.parent))
    repo = make_repo(tmp_path / "repo")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo),
                         "color": "#123456", "guard_words": [WORD]}])
    assert main(["guard", "install", "demo"]) == 0
    yield repo
    make_hooks_mutable(repo)


def commit(repo: Path, message: str = "change", name: str = "a.txt", text: str = "hello\n"):
    (repo / name).write_text(text, encoding="utf-8")
    subprocess.run([GIT, "-C", str(repo), "add", name], check=True)
    return subprocess.run([GIT, "-C", str(repo), *IDENT, "commit", "-q", "-m", message],
                          capture_output=True, text=True)


def git_hook_rows(home: Path) -> list[dict]:
    path = home / "events.jsonl"
    rows = [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []
    return [r for r in rows if r.get("kind") == "git-hook"]


def test_install_writes_three_hooks_and_verifies(env: Path) -> None:
    hooks = env / ".git" / "hooks"
    for name in ("pre-commit", "commit-msg", "pre-push"):
        text = (hooks / name).read_text()
        assert tirith_guard.MARKER in text and "TIRITH_GUARD_VERSION=2" in text
    assert tirith_guard.verify(env, [WORD]) == []
    assert main(["guard", "verify", "demo"]) == 0


def test_owner_guard_text_is_kept_around_the_recording() -> None:
    v1, v2 = tirith_guard.render_v1([WORD]), tirith_guard.render([WORD])
    for name, old in v1.items():
        head, sep, body = old.partition("set -euo pipefail\n")
        assert v2[name].startswith(head + sep)
        assert body in v2[name]


def test_commit_allowed_and_blocked_are_recorded(env: Path, home: Path) -> None:
    ok = commit(env)
    assert ok.returncode == 0, ok.stderr
    blocked = commit(env, name="b.txt", text=f"a {WORD} here\n")
    assert blocked.returncode != 0 and "commit blocked" in blocked.stderr
    subprocess.run([GIT, "-C", str(env), "reset", "-q"], check=True)
    msg = commit(env, message=f"{WORD} in message", name="c.txt")
    assert msg.returncode != 0
    rows = git_hook_rows(home)
    got = [(r["hook"], r["result"]) for r in rows]
    assert ("pre-commit", "allowed") in got and ("commit-msg", "allowed") in got
    assert ("pre-commit", "blocked") in got and ("commit-msg", "blocked") in got
    blocked_row = next(r for r in rows if r["hook"] == "pre-commit" and r["result"] == "blocked")
    assert blocked_row["reason"] and blocked_row["at"].endswith("Z")
    assert os.path.realpath(blocked_row["repo"]) == os.path.realpath(env)
    assert blocked_row["lane"] is None


def test_lane_is_recorded(env: Path, home: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("TIRITH_LANE", 'lane "x"')
    assert commit(env).returncode == 0
    assert git_hook_rows(home)[-1]["lane"] == 'lane "x"'


def test_toml_hooks_run_through_tirith_run(env: Path, home: Path) -> None:
    (env / "tirith.toml").write_text(
        '[[hooks]]\nevent = "pre-commit"\nrun = "echo ran > hook-ran.txt"\n'
        '[[hooks]]\nevent = "pre-push"\nrun = "false"\n', encoding="utf-8")
    assert commit(env, name="t.txt").returncode == 0
    assert (env / "hook-ran.txt").read_text().strip() == "ran"
    runs = [json.loads(line) for line in (home / "events.jsonl").read_text().splitlines()]
    stage = [r for r in runs if r.get("stage") == "pre-commit" and r.get("pipeline") == "git-hooks"]
    assert [r["event"] for r in stage] == ["start", "end"] and stage[-1]["status"] == "passed"
    assert stage[0]["project"] == "demo"

    (env / "tirith.toml").write_text(
        '[[hooks]]\nevent = "pre-commit"\nrun = "exit 3"\n', encoding="utf-8")
    failed = commit(env, name="u.txt")
    assert failed.returncode != 0
    last = git_hook_rows(home)[-1]
    assert last["hook"] == "pre-commit" and last["result"] == "blocked"
    assert "tirith run --stage pre-commit failed: exit 3" in last["reason"]


def test_hooks_run_without_tirith_on_path(env: Path, home: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    (env / "tirith.toml").write_text(
        '[[hooks]]\nevent = "pre-commit"\nrun = "false"\n', encoding="utf-8")
    monkeypatch.setenv("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
    assert commit(env).returncode == 0
    assert git_hook_rows(home)[-1]["result"] == "allowed"


def push(repo: Path, remote: Path) -> subprocess.CompletedProcess:
    if not remote.exists():
        subprocess.run([GIT, "init", "-q", "--template=", "--bare", str(remote)], check=True)
    return subprocess.run([GIT, "-C", str(repo), "push", "-q", str(remote), "HEAD:refs/heads/x"],
                          capture_output=True, text=True)


def test_pre_push_runs_tirith_check(env: Path, home: Path, tmp_path: Path) -> None:
    remote = tmp_path / "remote.git"
    refused = push(env, remote)  # no tirith.toml: `tirith check` fails
    assert refused.returncode != 0
    last = git_hook_rows(home)[-1]
    assert last["hook"] == "pre-push" and last["result"] == "blocked"
    assert last["reason"] == "tirith check failed"

    (env / "tirith.toml").write_text(
        'deploy = "none"\n[[pipelines]]\nid = "ci"\n'
        '[[pipelines.stages]]\nname = "build"\nrun = "true"\n'
        '[[pipelines.stages]]\nname = "test"\nrun = "true"\n', encoding="utf-8")
    sent = push(env, remote)
    assert sent.returncode == 0, sent.stderr
    last = git_hook_rows(home)[-1]
    assert (last["hook"], last["result"]) == ("pre-push", "allowed")


def test_version_one_guard_still_verifies_with_reinstall_steps(home: Path, tmp_path: Path, capsys) -> None:
    repo = make_repo(tmp_path / "old")
    try:
        write_config(home, [{"id": "old", "name": "Old", "repo": str(repo),
                             "color": "#123456", "guard_words": [WORD]}])
        (repo / ".git" / "hooks").mkdir(exist_ok=True)  # make_repo uses no template, so no hooks folder
        for name, text in tirith_guard.render_v1([WORD]).items():
            hook = repo / ".git" / "hooks" / name
            hook.write_text(text)
            hook.chmod(0o755)
            if hasattr(os, "chflags"):
                os.chflags(hook, hook.stat().st_flags | 0x2)  # UF_IMMUTABLE
        assert tirith_guard.verify(repo, [WORD]) == []
        steps = tirith_guard.outdated(repo, [WORD])
        assert "older version" in steps[0]
        assert any("chflags nouchg" in s for s in steps) and any("rm " in s for s in steps)
        assert steps[-1].endswith(f"tirith guard install {repo}")
        capsys.readouterr()
        assert main(["guard", "verify", "old"]) == 0
        out = capsys.readouterr().out
        assert "guard OK" in out and "older version" in out
        main(["guard", "install", "old"])  # never touches the old hooks, no mixed install
        assert not (repo / ".git" / "hooks" / "pre-push").exists()
        # a tampered version 1 hook is still a failure
        make_hooks_mutable(repo)
        hook = repo / ".git" / "hooks" / "pre-commit"
        hook.write_text(hook.read_text() + "# tampered\n")
        assert any("differs" in p for p in tirith_guard.verify(repo, [WORD]))
        assert tirith_guard.outdated(repo, [WORD]) == []
    finally:
        make_hooks_mutable(repo)


def test_collector_shows_git_hooks_pipeline_and_guard_flows(env: Path, home: Path) -> None:
    assert commit(env).returncode == 0
    assert commit(env, name="b.txt", text=f"{WORD}\n").returncode != 0
    state = Collector(home, client=None).cycle()
    run = next(p for p in state["pipelines"] if p["pipeline"] == "git-hooks")
    assert run["project"] == "demo"
    stages = {s["name"]: s["status"] for s in run["stages"]}
    assert stages["commit-msg"] == "passed"
    assert stages["pre-commit"] == "failed"  # the latest pre-commit run was blocked
    assert stages["pre-push"] == "pending" and run["status"] == "failed"
    edges = [f for f in state["flows"] if f["from"] == "guard" and f["to"].startswith("pipeline:git-hooks")]
    texts = [f["text"] for f in edges]
    assert "pre-commit allowed" in texts and any(t.startswith("pre-commit blocked") for t in texts)
    assert all(f["project"] == "demo" for f in edges)
    again = Collector(home, client=None).cycle()
    assert len([f for f in again["flows"] if f["from"] == "guard" and f["to"].startswith("pipeline:git-hooks")]) \
        == len(edges)
    assert not any(e.get("kind") == "git-hook" for e in state["loose_events"])
