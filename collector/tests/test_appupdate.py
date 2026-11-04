import pathlib
import subprocess
from types import SimpleNamespace

from tirith import appupdate


def _repo(tmp_path):
    repo = tmp_path / "repo"
    (repo / "app/src-tauri").mkdir(parents=True)
    (repo / "rust").mkdir()
    (repo / "app/src-tauri/lib.rs").write_text("a")
    (repo / "rust/x.rs").write_text("a")
    (repo / "Cargo.toml").write_text("a")
    (repo / "Cargo.lock").write_text("a")

    def g(*a):
        return subprocess.run(["git", "-C", str(repo), "-c", "user.name=t", "-c", "user.email=t@t", *a],
                              check=True, capture_output=True)
    g("init", "-q")
    g("add", ".")
    g("commit", "-qm", "x")
    return repo, g


def _home(tmp_path, repo, installed):
    home = tmp_path / "home"
    home.mkdir()
    (home / "config.toml").write_text(f'[app]\nrepo_dir = "{repo}"\ninstall_path = "{installed}"\n')
    return home


def test_only_rust_changes_on_the_branch_trigger_a_rebuild(tmp_path):
    repo, g = _repo(tmp_path)
    installed = tmp_path / "Tirith.app"
    installed.mkdir()
    home = _home(tmp_path, repo, installed)
    started = []
    popen = lambda argv, **kw: started.append(argv)  # noqa: E731
    assert not appupdate.maybe_start(home, popen, "darwin")  # first look: baseline
    assert not appupdate.maybe_start(home, popen, "darwin")
    (repo / "web.js").write_text("x")
    g("add", ".")
    g("commit", "-qm", "web")
    assert not appupdate.maybe_start(home, popen, "darwin")  # not Rust
    (repo / "rust/x.rs").write_text("b")
    assert not appupdate.maybe_start(home, popen, "darwin")  # uncommitted: not on main yet
    g("commit", "-qam", "rust")
    assert appupdate.maybe_start(home, popen, "darwin")
    assert "app-update" in started[0]
    assert not appupdate.maybe_start(home, popen, "darwin")  # already building


def test_disabled_off_mac_and_without_installed_app(tmp_path):
    repo, _ = _repo(tmp_path)
    home = _home(tmp_path, repo, tmp_path / "missing.app")
    assert not appupdate.maybe_start(home, lambda *a, **k: None, "linux")
    assert not appupdate.maybe_start(home, lambda *a, **k: None, "darwin")  # no installed app
    (home / "config.toml").write_text("[app]\nauto_update = false\n")
    assert not appupdate.enabled(home, "darwin")


def _runner(calls, signed=True, build_ok=True):
    def run(argv, **kw):
        calls.append(argv)
        err = ""
        rc = 0
        if argv[0] == "sh":
            rc = 0 if build_ok else 1
        if argv[0] == "codesign" and argv[1] == "-dvv":
            err = "Authority=Developer ID Application: X" if signed else "Signature=adhoc"
        if argv[0] == "pgrep":
            rc = 1
        if argv[0] == "ditto":
            pathlib.Path(argv[2]).mkdir(parents=True)
        if argv[0] == "rm":
            import shutil
            shutil.rmtree(argv[-1], ignore_errors=True)
        return SimpleNamespace(returncode=rc, stdout="", stderr=err)
    return run


def _setup_update(tmp_path):
    repo, _ = _repo(tmp_path)
    installed = tmp_path / "Tirith.app"
    installed.mkdir()
    (installed / "old").write_text("1")
    (repo / "target/release/bundle/macos/Tirith.app").mkdir(parents=True)
    return installed, _home(tmp_path, repo, installed)


def test_update_installs_a_signed_build(tmp_path, monkeypatch):
    monkeypatch.delenv("CARGO_TARGET_DIR", raising=False)
    installed, home = _setup_update(tmp_path)
    calls = []
    assert appupdate.update(home, "fp1", _runner(calls), lambda s: None) == 0
    assert appupdate.read_record(home)["built"] == "fp1"
    assert installed.is_dir() and not (installed / "old").exists()
    assert not any(c[0] == "open" for c in calls)  # it was not running: nothing to reopen


def test_unsigned_or_failed_build_is_not_installed_nor_retried(tmp_path, monkeypatch):
    monkeypatch.delenv("CARGO_TARGET_DIR", raising=False)
    installed, home = _setup_update(tmp_path)
    assert appupdate.update(home, "fp2", _runner([], signed=False), lambda s: None) == 1
    assert (installed / "old").exists()
    rec = appupdate.read_record(home)
    assert rec["failed"] == "fp2" and "not signed" in rec["error"]
    assert appupdate.update(home, "fp3", _runner([], build_ok=False), lambda s: None) == 1
    assert appupdate.read_record(home)["failed"] == "fp3"
