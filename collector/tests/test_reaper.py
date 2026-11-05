"""Tirith kills every process left under Tirith-lanes/<lane>/ (exe or cwd): SIGTERM, then SIGKILL."""
import os
import subprocess
import sys
import time
from pathlib import Path

from tirith import procs, reaper

SLEEP = "import time; time.sleep(60)"


def _gone(proc: subprocess.Popen, seconds: float = 10.0) -> bool:
    try:
        proc.wait(timeout=seconds)
    except subprocess.TimeoutExpired:
        return False
    return True


def _workspace(tmp_path: Path) -> Path:
    ws = tmp_path / "Tirith-lanes" / "lane-x"
    ws.mkdir(parents=True)
    return ws


def test_reap_lane_leftovers_kills_by_cwd_and_sigkills_term_ignorers(tmp_path: Path) -> None:
    ws = _workspace(tmp_path)
    ready = tmp_path / "ready"
    plain = subprocess.Popen([sys.executable, "-c", SLEEP], cwd=ws, start_new_session=True)
    stubborn = subprocess.Popen(
        [sys.executable, "-c",
         "import signal,time,pathlib; signal.signal(signal.SIGTERM, signal.SIG_IGN); "
         f"pathlib.Path({str(ready)!r}).write_text('1'); time.sleep(60)"], cwd=ws, start_new_session=True)
    outside = subprocess.Popen([sys.executable, "-c", SLEEP], cwd=tmp_path, start_new_session=True)
    try:
        deadline = time.monotonic() + 10
        while not ready.exists() and time.monotonic() < deadline:
            time.sleep(0.05)
        killed = reaper.reap_lane_leftovers(ws, term_wait=0.5)
        assert {plain.pid, stubborn.pid} <= set(killed) and outside.pid not in killed
        assert _gone(plain) and _gone(stubborn)
        assert stubborn.returncode == -9  # it ignored SIGTERM, so SIGKILL
        assert outside.poll() is None
    finally:
        for p in (plain, stubborn, outside):
            p.kill()
            p.wait()


def test_reap_lane_leftovers_kills_by_exe(tmp_path: Path, monkeypatch) -> None:
    ws = _workspace(tmp_path)
    proc = subprocess.Popen([sys.executable, "-c", SLEEP], cwd=tmp_path, start_new_session=True)  # cwd outside the lane
    real = procs.locations
    # (a signed system binary cannot be copied into the lane on macOS, so the lane's exe is faked for this pid)
    monkeypatch.setattr(procs, "locations",
                        lambda pid: (str(ws / "bin" / "tool"), "/") if pid == proc.pid else real(pid))
    try:
        assert proc.pid in reaper.reap_lane_leftovers(ws, term_wait=0.5)
        assert _gone(proc)
    finally:
        proc.kill()
        proc.wait()


def test_reap_lane_leftovers_ignores_main_checkouts_and_ourselves(tmp_path: Path, monkeypatch) -> None:
    repo = tmp_path / "repo"
    repo.mkdir()
    proc = subprocess.Popen([sys.executable, "-c", SLEEP], cwd=repo, start_new_session=True)
    try:
        time.sleep(0.2)
        assert reaper.reap_lane_leftovers(repo) == []  # not under Tirith-lanes
        assert proc.poll() is None
        ws = _workspace(tmp_path)
        monkeypatch.chdir(ws)  # we sit in the lane ourselves: never killed
        assert os.getpid() not in reaper.reap_lane_leftovers(ws, term_wait=0.1)
    finally:
        proc.kill()
        proc.wait()
