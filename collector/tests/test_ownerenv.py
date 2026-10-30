"""The owner's shell environment reaches the collector and what it starts (fake login shells, no real one)."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

import pytest

from tirith import ownerenv, selfcheck
from tirith import timeouts as timeouts_mod

SECRET = "s3cr3t-value-for-test"


def fake_shell(path: Path, body: str) -> str:
    path.write_text("#!/bin/sh\n" + body)
    path.chmod(0o755)
    return str(path)


def dump(pairs: dict[str, str]) -> str:
    """A shell that prints rc-file noise, the marker, then an ``env -0`` style dump."""
    data = "".join(f"{k}={v}\\0" for k, v in pairs.items())
    return f"echo 'welcome banner'\nprintf '{ownerenv.MARK}'\nprintf '{data}'\n"


@pytest.fixture(autouse=True)
def fresh(monkeypatch):
    monkeypatch.setattr(ownerenv, "_status", {"state": "pending", "added": 0, "reason": ""})
    monkeypatch.delenv("OWNER_ENV_KEY", raising=False)


def test_owner_env_reaches_collector_and_child(tmp_path, home, monkeypatch):
    shell = fake_shell(tmp_path / "sh", dump({"OWNER_ENV_KEY": SECRET, "PWD": "/elsewhere", "bad key": "x"}))
    monkeypatch.setenv("SHELL", shell)
    got = ownerenv.load(home)
    assert got["state"] == "ok" and got["added"] == 1
    assert os.environ["OWNER_ENV_KEY"] == SECRET
    child = subprocess.run(["/bin/sh", "-c", "printf %s \"$OWNER_ENV_KEY\""], capture_output=True, text=True)
    assert child.stdout == SECRET
    os.environ.pop("OWNER_ENV_KEY", None)


def test_owner_env_existing_values_win(tmp_path, home, monkeypatch):
    monkeypatch.setenv("SHELL", fake_shell(tmp_path / "sh", dump({"OWNER_ENV_KEY": "from-shell"})))
    monkeypatch.setenv("OWNER_ENV_KEY", "already")
    got = ownerenv.load(home)
    assert os.environ["OWNER_ENV_KEY"] == "already" and got["added"] == 0


def test_owner_env_failure_and_timeout(tmp_path, home, monkeypatch):
    monkeypatch.setenv("SHELL", fake_shell(tmp_path / "bad", "exit 3\n"))
    assert ownerenv.load(home)["state"] == "failed"
    monkeypatch.setenv("SHELL", fake_shell(tmp_path / "nomark", "echo hello\n"))
    assert ownerenv.load(home)["state"] == "failed"
    monkeypatch.setenv("SHELL", str(tmp_path / "missing"))
    assert ownerenv.load(home)["state"] == "failed"
    monkeypatch.setattr(ownerenv, "TIMEOUT_S", 1.0)
    monkeypatch.setattr(timeouts_mod, "KILL_GRACE_S", 0.5)
    monkeypatch.setenv("SHELL", fake_shell(tmp_path / "slow", "sleep 30\n"))
    got = ownerenv.load(home)
    assert got["state"] == "failed" and "took more than" in got["reason"]
    assert "OWNER_ENV_KEY" not in os.environ


def test_owner_env_values_never_in_state_or_logs(tmp_path, home, monkeypatch):
    monkeypatch.setenv("SHELL", fake_shell(tmp_path / "sh", dump({"OWNER_ENV_KEY": SECRET})))
    ownerenv.load(home)
    os.environ.pop("OWNER_ENV_KEY", None)
    assert SECRET not in repr(ownerenv.status())
    book = {"checks": {"ownerenv": selfcheck._ownerenv()}, "level": "green", "at": "x"}
    assert SECRET not in repr(selfcheck.public(book))
    for path in home.rglob("*"):
        if path.is_file():
            assert SECRET.encode() not in path.read_bytes(), path


def test_owner_env_health_check_says_so(tmp_path, home, monkeypatch):
    assert selfcheck._ownerenv()["level"] == "ok"
    monkeypatch.setenv("SHELL", fake_shell(tmp_path / "bad", "exit 1\n"))
    ownerenv.load(home)
    check = selfcheck._ownerenv()
    assert check["level"] == "failed" and "shell variables" in check["text"]
