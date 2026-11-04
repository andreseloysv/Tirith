"""The desktop app rebuilds and restarts itself when its Rust code changes on main (macOS, signed only).

Every slow-work pass the collector calls :func:`maybe_start`. It fingerprints the Rust side of the app on the
repo's checked-out branch (git tree ids of ``app/src-tauri``, ``rust/`` and the Cargo files, so a dirty working
tree or a lane never counts) and compares it with ``<home>/app-update.json``. When it differs, and has not
already failed, one detached ``tirith app-update`` is started (its own session: it outlives the app and the
collector it will quit). :func:`update` builds with ``scripts/package.sh`` (signed with the Developer ID
identity), refuses a bundle that is not signed with one, quits the running app, swaps the installed ``.app`` and
opens it again. A failed build is recorded for that fingerprint and not retried until main changes again.
No servers, no ports. See ``docs/CONTRACT-launch.md`` ("App self-update").
"""

from __future__ import annotations

import hashlib
import os
import subprocess
import sys
import time
import tomllib
from pathlib import Path
from typing import Callable

from tirith import timeouts as timeouts_mod
from tirith.home import now_iso
from tirith.store import write_json_atomic

PATHS = ("app/src-tauri", "rust", "Cargo.toml", "Cargo.lock")
RECORD = "app-update.json"
BUILD_TIMEOUT_S = 3600
APP_NAME = "Tirith"


def repo_dir(home: Path) -> Path:
    cfg = _app_config(home)
    if cfg.get("repo_dir"):
        return Path(str(cfg["repo_dir"])).expanduser()
    return Path(__file__).resolve().parents[2]


def _app_config(home: Path) -> dict:
    try:
        return tomllib.loads((home / "config.toml").read_text()).get("app", {}) or {}
    except (OSError, tomllib.TOMLDecodeError):
        return {}


def enabled(home: Path, platform: str | None = None) -> bool:
    """macOS only; ``[app] auto_update = false`` turns it off."""
    if (platform or sys.platform) != "darwin":
        return False
    return _app_config(home).get("auto_update", True) is not False


def install_path(home: Path) -> Path | None:
    """The installed ``Tirith.app`` to replace: ``[app] install_path``, else /Applications, else ~/Applications.
    None when the app is not installed (nothing to restart)."""
    cfg = _app_config(home).get("install_path")
    for cand in ([Path(str(cfg)).expanduser()] if cfg else
                 [Path("/Applications") / f"{APP_NAME}.app", Path.home() / "Applications" / f"{APP_NAME}.app"]):
        if cand.is_dir():
            return cand
    return None


def fingerprint(repo: Path) -> str | None:
    """Hash of the Rust sources as committed on the repo's current branch; None when git cannot say."""
    res = timeouts_mod.run(["git", "-C", str(repo), "rev-parse", *[f"HEAD:{p}" for p in PATHS]], "git", text=True)
    if res.returncode != 0:
        return None
    return hashlib.sha256(res.stdout.encode()).hexdigest()[:16]


def read_record(home: Path) -> dict:
    import json
    try:
        data = json.loads((home / RECORD).read_text())
        return data if isinstance(data, dict) else {}
    except (OSError, ValueError):
        return {}


def needed(home: Path, repo: Path) -> str | None:
    """The new fingerprint when the app must be rebuilt, else None."""
    fp = fingerprint(repo)
    rec = read_record(home)
    if fp is None or fp == rec.get("built") or fp == rec.get("failed") or fp == rec.get("building"):
        return None
    if rec.get("built") is None and rec.get("failed") is None:
        # first look: the installed app is taken as current, only a later change rebuilds it
        _write(home, built=fp, note="baseline")
        return None
    return fp


def _write(home: Path, **fields) -> None:
    rec = read_record(home)
    rec.update(fields, at=now_iso())
    write_json_atomic(home / RECORD, rec)


def maybe_start(home: Path, popen: Callable = subprocess.Popen, platform: str | None = None) -> bool:
    """Start the detached updater when the Rust code on main changed. True when it was started."""
    if not enabled(home, platform) or install_path(home) is None:
        return False
    repo = repo_dir(home)
    fp = needed(home, repo)
    if fp is None:
        return False
    _write(home, building=fp, started=time.time())
    log = home / "logs"
    log.mkdir(parents=True, exist_ok=True)
    out = open(log / "app-update.log", "ab")
    popen([sys.executable, "-m", "tirith", "app-update", "--fingerprint", fp], stdout=out, stderr=out,
          stdin=subprocess.DEVNULL, start_new_session=True, cwd=str(repo / "collector"),
          env={**os.environ, "TIRITH_HOME": str(home)})
    return True


def is_developer_id_signed(app: Path, run: Callable = subprocess.run) -> bool:
    res = run(["codesign", "-dvv", str(app)], capture_output=True, text=True)
    text = (res.stdout or "") + (res.stderr or "")
    if res.returncode != 0 or "Authority=Developer ID Application" not in text:
        return False
    return run(["codesign", "--verify", "--deep", "--strict", str(app)], capture_output=True, text=True).returncode == 0


def build(repo: Path, run: Callable = subprocess.run) -> Path:
    """Run ``scripts/package.sh``; return the built ``.app``. Raises RuntimeError on a failed build."""
    res = run(["sh", str(repo / "scripts" / "package.sh")], cwd=str(repo), capture_output=True, text=True,
              timeout=BUILD_TIMEOUT_S)
    if res.returncode != 0:
        raise RuntimeError(f"package.sh failed ({res.returncode}): {(res.stderr or '')[-400:]}")
    target = Path(os.environ.get("CARGO_TARGET_DIR") or repo / "target")
    app = target / "release" / "bundle" / "macos" / f"{APP_NAME}.app"
    if not app.is_dir():
        raise RuntimeError(f"no bundle at {app}")
    return app


def _running(installed: Path, run: Callable) -> bool:
    return run(["pgrep", "-f", f"{installed}/Contents/MacOS/"], capture_output=True, text=True).returncode == 0


def swap(app: Path, installed: Path, run: Callable = subprocess.run, sleep: Callable = time.sleep) -> bool:
    """Quit the running app, replace ``installed`` with ``app`` (copy first, then rename), open it again if it
    was running. Returns whether it was running."""
    was = _running(installed, run)
    if was:
        run(["osascript", "-e", f'tell application "{APP_NAME}" to quit'], capture_output=True, text=True)
        for _ in range(30):
            if not _running(installed, run):
                break
            sleep(1)
        else:
            run(["pkill", "-f", f"{installed}/Contents/MacOS/"], capture_output=True, text=True)
            sleep(1)
    fresh = installed.with_name(f".{installed.name}.new")
    old = installed.with_name(f".{installed.name}.old")
    for tmp in (fresh, old):
        run(["rm", "-rf", str(tmp)], check=True)
    run(["ditto", str(app), str(fresh)], check=True)
    installed.rename(old)
    try:
        fresh.rename(installed)
    except OSError:
        old.rename(installed)  # keep the working app
        raise
    run(["rm", "-rf", str(old)])
    if was:
        run(["open", str(installed)], check=True)
    return was


def update(home: Path, fingerprint_: str | None = None, run: Callable = subprocess.run,
           sleep: Callable = time.sleep) -> int:
    """Build, check the signature, swap and restart. 0 on success, 1 on failure (recorded, not retried)."""
    repo = repo_dir(home)
    fp = fingerprint_ or fingerprint(repo)
    installed = install_path(home)
    try:
        if installed is None:
            raise RuntimeError("the app is not installed")
        app = build(repo, run)
        if not is_developer_id_signed(app, run):
            raise RuntimeError("the build is not signed with a Developer ID Application identity; not installed")
        swap(app, installed, run, sleep)
    except (RuntimeError, OSError, subprocess.SubprocessError) as exc:
        _write(home, failed=fp, building=None, error=str(exc)[:500])
        print(f"tirith app-update: {exc}", file=sys.stderr, flush=True)
        return 1
    _write(home, built=fp, building=None, error=None)
    print(f"tirith app-update: installed {fp}", flush=True)
    return 0
