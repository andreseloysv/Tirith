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


def test_reap_ended_lanes_kills_leftovers_of_ended_lanes_only(tmp_path: Path) -> None:
    from tirith.store import write_lane
    home = tmp_path / "home"
    (home / "lanes").mkdir(parents=True)
    ended, running = _workspace(tmp_path), tmp_path / "Tirith-lanes" / "lane-y"
    running.mkdir()
    a = subprocess.Popen([sys.executable, "-c", SLEEP], cwd=ended, start_new_session=True)
    b = subprocess.Popen([sys.executable, "-c", SLEEP], cwd=running, start_new_session=True)
    try:
        write_lane(home, {"id": "lane-x", "workspace": str(ended), "ended_at": "2026-01-01T00:00:00Z"})
        write_lane(home, {"id": "lane-y", "workspace": str(running)})
        time.sleep(0.2)
        assert a.pid in reaper.reap_ended_lanes(home, term_wait=0.5)
        assert _gone(a) and b.poll() is None
    finally:
        for p in (a, b):
            p.kill()
            p.wait()


def test_reap_skips_live_review(tmp_path: Path) -> None:
    from tirith.store import write_lane
    home = tmp_path / "home"
    (home / "lanes").mkdir(parents=True)
    ws = _workspace(tmp_path)
    a = subprocess.Popen([sys.executable, "-c", SLEEP], cwd=ws, start_new_session=True)
    try:
        write_lane(home, {"id": "lane-x", "workspace": str(ws), "ended_at": "2026-01-01T00:00:00Z",
                          "review": {"status": "running", "pid": os.getpid()}})
        time.sleep(0.2)
        assert reaper.reap_ended_lanes(home, term_wait=0.5) == [] and a.poll() is None
        write_lane(home, {"id": "lane-x", "workspace": str(ws), "ended_at": "2026-01-01T00:00:00Z",
                          "review": {"status": "running", "pid": 2 ** 22 + 12345}})
        assert a.pid in reaper.reap_ended_lanes(home, term_wait=0.5)
    finally:
        a.kill()
        a.wait()


def test_reap_skips_live_merge(tmp_path: Path) -> None:
    from tirith import mergeq
    from tirith.store import write_json_atomic, write_lane
    home = tmp_path / "home"
    (home / "lanes").mkdir(parents=True)
    ws = _workspace(tmp_path)
    a = subprocess.Popen([sys.executable, "-c", SLEEP], cwd=ws, start_new_session=True)
    try:
        write_lane(home, {"id": "lane-x", "workspace": str(ws), "ended_at": "2026-01-01T00:00:00Z"})
        data = mergeq.load(home)
        data["queue"].append({"lane": "lane-x", "status": "running", "pid": os.getpid()})
        write_json_atomic(mergeq._path(home), data)
        time.sleep(0.2)
        assert reaper.reap_ended_lanes(home, term_wait=0.5) == [] and a.poll() is None
    finally:
        a.kill()
        a.wait()
