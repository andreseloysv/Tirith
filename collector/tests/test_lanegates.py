"""Lane gates: write only in the lane's copy, no secrets, no servers, own processes only."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

import pytest

from tirith import gates, lanegates
from tirith import manifest as mf
from tirith.home import Config, ProjectConfig

HOME = os.path.expanduser("~")


@pytest.fixture
def lane(tmp_path: Path) -> Path:
    work = tmp_path / "proj-lanes" / "perms"
    (work / ".git").mkdir(parents=True)
    (work / "src").mkdir()
    return work


def _rules(lane: Path, tool: str, **inputs) -> list[str]:
    cfg = Config(projects=[ProjectConfig(id="proj", name="proj", repo=lane.parent.parent / "proj")])
    found = gates.check_call(cfg, tool, inputs, str(lane), "proj", lane="perms", env={"TMPDIR": "/tmp", "HOME": HOME})
    return [v.rule for v in found]


def _bash(lane: Path, command: str) -> list[str]:
    return _rules(lane, "Bash", command=command)


def test_all_existing_gate_ids_are_still_present_and_described() -> None:
    old = {"subagent-tool", "agent-cli", "no-verify", "hooks-path", "git-hooks", "settings-edit",
           "roadmap-edit", "raw-stage", "lane-brief", "project-check", "commit-guard"}
    new = {"workspace-write", "no-secrets", "no-servers", "own-processes"}
    assert old <= set(mf.BUILT_IN_RULES) and new <= set(mf.BUILT_IN_RULES)
    assert set(lanegates.RULE_IDS) == new
    assert (old | new) <= set(gates.DESCRIPTIONS)
    # the old gates still fire with a lane set
    assert "no-verify" in _bash(Path("."), "git commit --" + "no-verify -m x")
    assert "subagent-tool" in _rules(Path("."), "Task")


def test_gates_are_off_without_a_lane(lane: Path) -> None:
    assert gates.check_call(Config(), "Bash", {"command": "rm -rf /etc/x"}, str(lane), env={}) == []


@pytest.mark.parametrize("command", [
    "echo hi > out.txt", "echo hi >> src/a.txt", "echo hi > /tmp/x", "cat a | tee /private/tmp/y",
    "rm -rf src", "mkdir -p build/x && touch build/x/y", "cp a.txt src/b.txt", "sed -i '' s/a/b/ src/a.py",
    "ls > /dev/null 2>&1", "uv run pytest 2> /dev/null", "git status", "cd src && rm -f a",
    "chmod +x src/run.sh", "echo hi > $TMPDIR/x", "mv /tmp/a src/a",
])
def test_workspace_write_allows_the_copy_and_temp(lane: Path, command: str) -> None:
    assert "workspace-write" not in _bash(lane, command)


@pytest.mark.parametrize("command", [
    "echo hi > /etc/hosts", "echo hi >> ~/notes.txt", "echo a | tee /Users/x/out", "rm -rf /",
    "rm /opt/other/file", "mv a.txt /var/db/x", "cp a.txt /opt/b", "mkdir /opt/new", "chmod 600 /etc/x",
    "sed -i s/a/b/ /etc/hosts", "git -C /somewhere/else commit -am x", "echo x && rm -rf ~/Documents",
    "cd / && rm -rf tmpx", "echo $(echo hi > /opt/x)", "bash -c 'echo a > /opt/x'", "echo hi > $UNKNOWN/x",
    "rm -rf /opt/..", "cp -t /opt a.txt",
])
def test_workspace_write_denies_outside(lane: Path, command: str) -> None:
    assert "workspace-write" in _bash(lane, command)


def test_workspace_write_follows_symlinks_out_of_the_copy(lane: Path) -> None:
    (lane / "link").symlink_to("/etc")
    assert "workspace-write" in _bash(lane, "echo x > link/f")
    assert "workspace-write" in _rules(lane, "Write", file_path=str(lane / "link" / "f"))


def test_edit_tools_are_held_to_the_copy(lane: Path) -> None:
    for tool in ("Edit", "Write", "NotebookEdit"):
        key = "notebook_path" if tool == "NotebookEdit" else "file_path"
        assert "workspace-write" not in _rules(lane, tool, **{key: str(lane / "src" / "a.py")})
        assert "workspace-write" not in _rules(lane, tool, **{key: "/tmp/x.py"})
        assert "workspace-write" in _rules(lane, tool, **{key: "/etc/hosts"})
        assert "workspace-write" in _rules(lane, tool, **{key: "/opt/x"})
    assert "workspace-write" not in _rules(lane, "Write", file_path="src/relative.py")


def test_git_c_integration_into_the_main_checkout_is_allowed(lane: Path, monkeypatch) -> None:
    monkeypatch.setattr(lanegates, "TEMP_ROOTS", ())  # the test folders live in the temp area
    monkeypatch.setattr(lanegates.tempfile, "gettempdir", lambda: "/nonexistent-tmp")
    main = str(lane.parent.parent / "proj")
    assert "workspace-write" not in _bash(lane, f"git -C {main} merge --no-ff lane/perms")
    assert "workspace-write" not in _bash(lane, f"git -C {main} push origin main")
    assert "workspace-write" not in _bash(lane, f"git -C {main} status")
    assert "workspace-write" in _bash(lane, f"git -C {main} reset --hard")
    assert "workspace-write" in _bash(lane, "git -C /elsewhere merge x")


@pytest.mark.parametrize("command", [
    "cat ~/.ssh/id_rsa", "ls $HOME/.aws", f"cat {HOME}/.gnupg/pubring.kbx", "cat ~/.config/gh/hosts.yml",
    "cat ~/.netrc", "ls ~/Library/Keychains", "cat /etc/ssl/key.pem", "cat ~/.codex/auth.json",
    "cat ~/.copilot/config.json", "cat ~/.cl" "aude/.credentials.json", "cat /elsewhere/.env",
    "grep -r token ~/.ssh", "cd ~/.ssh && cat id_rsa", "echo ok && cat ~/.aws/credentials",
    "bash -c 'cat ~/.ssh/id_ed25519'", "cat < ~/.netrc",
    "python3 -c \"print(open('%s/.ssh/id_rsa').read())\"" % HOME,
])
def test_no_secrets_denies_credential_reads(lane: Path, command: str) -> None:
    assert "no-secrets" in _bash(lane, command)


@pytest.mark.parametrize("command", ["cat src/a.py", "cat .env", "cat .env.example", "ls ~", "ls ~/Documents",
                                     "cat ~/.zshrc", "ls ~/.config/other", "cat README.md"])
def test_no_secrets_allows_ordinary_reads(lane: Path, command: str) -> None:
    assert "no-secrets" not in _bash(lane, command)


def test_no_secrets_covers_the_read_tools(lane: Path) -> None:
    assert "no-secrets" in _rules(lane, "Read", file_path=f"{HOME}/.ssh/id_rsa")
    assert "no-secrets" in _rules(lane, "Grep", pattern="x", path=f"{HOME}/.aws")
    assert "no-secrets" in _rules(lane, "Read", file_path="/somewhere/.env")
    assert "no-secrets" in _rules(lane, "Read", file_path="/somewhere/cert.pem")
    assert "no-secrets" not in _rules(lane, "Read", file_path=str(lane / ".env"))
    assert "no-secrets" not in _rules(lane, "Read", file_path=str(lane / "src" / "a.py"))
    assert "no-secrets" in _rules(lane, "Write", file_path=f"{HOME}/.ssh/authorized_keys")


@pytest.mark.parametrize("command", [
    "python3 -m http.server 8000", "python -m http.server", "npx serve .", "serve dist",
    "uvicorn app:app", "npm run dev", "pnpm dev", "yarn dev", "npm start", "vite", "vite dev", "next dev",
    "nc -l 9000", "nc -lk 9000", "ncat -l 80", "flask run", "php -S localhost:8000", "app --port 3000",
    "cd web && npm run dev &", "echo a; python3 -m http.server", "bash -c 'npm run dev'",
    "gunicorn x:y", "ruby -run -e httpd .", "python3 -m uvicorn a:b",
])
def test_no_servers_denies_listeners(lane: Path, command: str) -> None:
    assert "no-servers" in _bash(lane, command)


@pytest.mark.parametrize("command", [
    "npm test", "npm run build", "vite build", "uv run pytest -q", "grep -- --port README.md",
    "git commit -m 'add --port flag'", "python3 -m pytest", "nc -z host 80", "next build", "cargo test",
])
def test_no_servers_allows_other_commands(lane: Path, command: str) -> None:
    assert "no-servers" not in _bash(lane, command)


def test_own_processes_denies_foreign_and_by_name(lane: Path) -> None:
    other = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(30)"], start_new_session=True)
    try:
        assert "own-processes" in _bash(lane, f"kill {other.pid}")
        assert "own-processes" in _bash(lane, f"kill -9 {other.pid}")
        assert "own-processes" in _bash(lane, f"echo a && kill -TERM {other.pid}")
        assert "own-processes" in _bash(lane, f"kill -s KILL -- {other.pid}")
    finally:
        other.kill()
        other.wait()
    for command in ("pkill node", "killall python", "kill 1", "kill $(pgrep x)", "kill $PID", "kill -9 -1",
                    "sudo pkill -f x"):
        assert "own-processes" in _bash(lane, command), command


def test_own_processes_allows_the_lanes_group(lane: Path) -> None:
    child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(30)"])
    try:
        assert os.getpgid(child.pid) == os.getpgrp()
        assert "own-processes" not in _bash(lane, f"kill {child.pid}")
        assert "own-processes" not in _bash(lane, f"kill -9 {child.pid}")
        assert "own-processes" not in _bash(lane, "sleep 100 & kill $!")
        assert "own-processes" not in _bash(lane, "kill %1")
        assert "own-processes" not in _bash(lane, "kill 99999999")  # nothing there
    finally:
        child.kill()
        child.wait()


def test_a_deny_is_recorded_as_a_violation_with_rule_and_command(home: Path, lane: Path) -> None:
    from tirith import hooks
    from tirith.store import read_jsonl
    event = {"event": "PreToolUse", "tool": "Bash", "cwd": str(lane), "lane": "perms", "project": None,
             "ts": "2026-10-29T00:00:00Z", "session": None}
    payload = {"tool_name": "Bash", "tool_input": {"command": "echo x > /etc/passwd"}, "cwd": str(lane)}
    assert hooks.decide("anthropic", event, payload, home) == 2
    rows = [r for r in read_jsonl(home / "events.jsonl") if r.get("kind") == "violation"]
    assert rows and rows[-1]["rule"] == "workspace-write" and "/etc/passwd" in rows[-1]["message"]
    assert rows[-1]["lane"] == "perms"
    ok = dict(payload, tool_input={"command": "echo x > out.txt"})
    assert hooks.decide("anthropic", event, ok, home) == 0
