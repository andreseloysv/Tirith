"""Fast tests in lanes: changed file -> tests, fallbacks, the adaptive limiter and the untouched gate rules."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

from conftest import GIT, make_repo
from tirith import fasttests, gates, loadlimit, testmap
from tirith.cli import main
from tirith.home import load_config
from tirith.store import read_jsonl


def _tree(root: Path) -> Path:
    pkg, tests, web = root / "collector" / "tirith", root / "collector" / "tests", root / "web" / "js"
    for d in (pkg, tests, web, root / "rust" / "tirith-core" / "src", root / "rust" / "tirith-cli" / "src"):
        d.mkdir(parents=True, exist_ok=True)
    (pkg / "__init__.py").write_text("")
    (pkg / "alpha.py").write_text("X = 1\n")
    (pkg / "beta.py").write_text("from tirith import alpha\n")
    (pkg / "gamma.py").write_text("Y = 1\n")
    for name in ("delta", "eps", "zeta", "eta"):
        (pkg / f"{name}.py").write_text("Z = 1\n")
        (tests / f"test_{name}.py").write_text(f"from tirith import {name}\n")
    (tests / "test_alpha.py").write_text("from tirith import alpha\n")
    (tests / "test_beta_more.py").write_text("from tirith.beta import x\n")
    (tests / "test_gamma.py").write_text("import tirith.gamma\n")
    (web / "chat.js").write_text("")
    (web / "chat.test.mjs").write_text("// chat.js\n")
    (web / "other.test.mjs").write_text("// nothing\n")
    return root


def test_module_maps_to_its_tests_and_importers(tmp_path):
    root = _tree(tmp_path)
    sel = fasttests.select(root, "test", ["collector/tirith/alpha.py"])
    assert not sel.full
    assert sel.tests == ["tests/test_alpha.py", "tests/test_beta_more.py"]  # beta imports alpha
    only = fasttests.select(root, "test", ["collector/tirith/gamma.py"])
    assert only.tests == ["tests/test_gamma.py"]
    assert "tests/test_gamma.py" in fasttests.command(only)


def test_changed_test_file_runs_itself(tmp_path):
    root = _tree(tmp_path)
    sel = fasttests.select(root, "test", ["collector/tests/test_delta.py"])
    assert sel.tests == ["tests/test_delta.py"]


def test_lane_never_runs_the_full_suite_for_a_shared_file(tmp_path):
    root = _tree(tmp_path)
    for files in (["collector/tirith/home.py"], ["collector/tests/conftest.py"], ["collector/tirith/store.py"]):
        sel = fasttests.select(root, "test", files)
        assert not sel.full and sel.tests == [] and fasttests.command(sel) == "true", files
        assert "merge queue runs the full suite" in sel.summary()


def test_lane_never_runs_the_full_suite_for_an_unmapped_file(tmp_path):
    root = _tree(tmp_path)
    for files in (["collector/state.schema.json"], ["scripts/x.sh"], []):
        sel = fasttests.select(root, "test", files)
        assert not sel.full and fasttests.command(sel) == "true", files
    # a mapped change next to an unmapped one still runs its tests
    both = fasttests.select(root, "test", ["collector/state.schema.json", "collector/tirith/gamma.py"])
    assert both.tests == ["tests/test_gamma.py"]


def test_lane_never_runs_the_full_suite_for_a_wide_python_change(tmp_path):
    root = _tree(tmp_path)
    wide = fasttests.select(root, "test", [f"collector/tirith/{m}.py" for m in ("alpha", "delta", "eps", "zeta", "eta")])
    assert not wide.full and "most of the suite" in wide.reason
    assert wide.tests == ["tests/test_alpha.py", "tests/test_delta.py", "tests/test_eps.py",
                          "tests/test_eta.py", "tests/test_zeta.py"]  # by name; the importers are left to the queue
    assert fasttests.command(fasttests.select(root, "test", ["collector/tirith/fasttests.py"])) == "true"


def test_changed_tests_run_in_parallel(tmp_path):
    sel = fasttests.select(_tree(tmp_path), "test", ["collector/tirith/gamma.py"])
    assert "pytest -q -n auto --maxprocesses 4 tests/test_gamma.py" in fasttests.command(sel)


def test_rust_only_the_crate_touched(tmp_path):
    root = _tree(tmp_path)
    cli = fasttests.select(root, "rust-test", ["rust/tirith-cli/src/main.rs"])
    assert cli.crates == ["tirith-cli"] and fasttests.command(cli) == "cargo test -q -p tirith-cli"
    core = fasttests.select(root, "rust-test", ["rust/tirith-core/src/lib.rs"])
    assert core.crates == ["tirith-cli", "tirith-core"]
    assert not fasttests.select(root, "rust-test", ["app/src-tauri/src/main.rs"]).full
    # a Rust change leaves the python test stage with nothing to run
    assert fasttests.command(fasttests.select(root, "test", ["rust/tirith-cli/src/main.rs"])) == "true"


def test_web_only_the_matching_test(tmp_path):
    root = _tree(tmp_path)
    sel = fasttests.select(root, "test", ["web/js/chat.js"])
    assert sel.web == ["web/js/chat.test.mjs"] and sel.tests == []
    assert fasttests.command(sel) == "node web/js/chat.test.mjs"


def test_docs_only_runs_nothing(tmp_path):
    sel = fasttests.select(_tree(tmp_path), "test", ["docs/NOTES.md", "README.md"])
    assert not sel.full and fasttests.command(sel) == "true"


def test_changed_files_from_git(tmp_path):
    repo = make_repo(tmp_path / "repo")
    (repo / "new.py").write_text("x = 1\n")
    (repo / "README.md").write_text("changed\n")
    assert fasttests.changed_files(repo) == ["README.md", "new.py"]


def test_run_records_the_selection(home, tmp_path, capsys):
    repo = make_repo(tmp_path / "proj")
    (repo / "tirith.toml").write_text('[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "test"\nrun = "echo FULLRUN"\n')
    subprocess.run([GIT, "-C", str(repo), "add", "."], check=True)
    subprocess.run([GIT, "-C", str(repo), "-c", "user.email=t@example.invalid", "-c", "user.name=T",
                    "commit", "-q", "-m", "manifest"], check=True)
    (repo / "docs.md").write_text("x\n")
    (home / "config.toml").write_text(f'[[projects]]\nid = "p"\nname = "P"\nrepo = "{repo}"\n', encoding="utf-8")
    old = os.getcwd()
    os.chdir(repo)
    try:
        code = main(["--home", str(home), "run", "--project", "p", "--stage", "test", "--changed"])
    finally:
        os.chdir(old)
    out = capsys.readouterr().out
    assert code == 0 and "changed only" in out and "FULLRUN" not in out
    rows = read_jsonl(home / "selections.jsonl")
    assert rows and rows[-1]["mode"] == "changed" and rows[-1]["stage"] == "test"


def test_limit_follows_load_and_disk():
    gb = loadlimit.GB
    assert loadlimit.limit(4, cpus=8, load=1.0) == (4, "")
    busy, why = loadlimit.limit(4, cpus=8, load=14.0)
    assert busy < 4 and "load" in why
    assert loadlimit.limit(4, cpus=8, load=40.0)[0] == 1  # never below one
    low, why = loadlimit.limit(4, cpus=8, load=0.0, free_bytes=2 * gb, min_free_bytes=5 * gb)
    assert low == 1 and "disk" in why
    near = loadlimit.limit(4, cpus=8, load=0.0, free_bytes=7 * gb, min_free_bytes=5 * gb)[0]
    assert near == 2
    assert loadlimit.limit(0, cpus=2, load=0.0)[0] == 1


def test_current_limit_uses_the_disk_guard(home):
    allowed, why = loadlimit.current(load_config(home), home, 4, free=lambda folder: 0)
    assert allowed == 1 and "disk" in why


def test_slots_are_exclusive(home):
    with loadlimit.slot(home, 1) as first:
        assert first == 0
        with loadlimit.slot(home, 1, wait=0.0, poll=0.01) as second:
            assert second is None  # all taken: waited out, runs anyway
    with loadlimit.slot(home, 2) as again:
        assert again == 0


def test_existing_gate_ids_are_still_present():
    source = Path(gates.__file__).read_text(encoding="utf-8")
    ids = ["agent-cli", "git-hooks", "guard-reinstall", "hooks-path", "no-verify", "raw-stage",
           "roadmap-edit", "settings-edit", "subagent-tool"]
    for rule in ids:
        assert f'"{rule}"' in source, rule
    assert gates.Violation("no-verify", "w", "i").action == "block"


def _stub_tools(bin_dir: Path) -> None:
    bin_dir.mkdir()
    for name, body in (("uv", "exit 0"), ("node", '[ -f "$1" ] || { echo "no such file $1"; exit 1; }')):
        tool = bin_dir / name
        tool.write_text(f"#!/bin/sh\n{body}\n")
        tool.chmod(0o755)


def test_python_and_web_steps_run_from_the_right_folders(tmp_path):
    root = _tree(tmp_path)
    sel = fasttests.select(root, "test", ["collector/tirith/gamma.py", "web/js/chat.js"])
    assert sel.tests and sel.web
    _stub_tools(tmp_path / "bin")
    env = {**os.environ, "PATH": f"{tmp_path / 'bin'}:{os.environ['PATH']}"}
    done = subprocess.run(["/bin/sh", "-c", fasttests.command(sel)], cwd=root, env=env, capture_output=True, text=True)
    assert done.returncode == 0, done.stdout + done.stderr  # `node web/...` used to run inside collector/


def test_the_map_picks_python_tests_only_for_python_changes(tmp_path):
    root = _tree(tmp_path)
    sel = fasttests.select(root, "test", ["collector/tirith/alpha.py", "web/js/chat.js"], mapped=True)
    assert sel.mapped and sel.tests == [] and sel.web == ["web/js/chat.test.mjs"] and sel.record()["mode"] == "map"
    cmd = fasttests.command(sel)
    assert "--testmon" in cmd and "-p no:xdist" in cmd and "-n auto" not in cmd and "node web/js/chat.test.mjs" in cmd
    for other in (["collector/pyproject.toml"], ["collector/tirith/alpha.py", "collector/state.schema.json"]):
        sel = fasttests.select(root, "test", other, mapped=True)
        assert not sel.mapped, other  # testmon does not see data files: names and the import graph
    assert not fasttests.select(root, "test", ["collector/tirith/alpha.py"], mapped=False).mapped
    assert not fasttests.select(root, "rust-test", ["collector/tirith/alpha.py"], mapped=True).mapped


def test_the_map_command_accepts_nothing_selected(tmp_path):
    _stub_tools(tmp_path / "bin")
    (tmp_path / "collector").mkdir()
    env = {**os.environ, "PATH": f"{tmp_path / 'bin'}:{os.environ['PATH']}"}
    for exit_code, expected in ((0, 0), (5, 0), (1, 1)):
        (tmp_path / "bin" / "uv").write_text(f"#!/bin/sh\nexit {exit_code}\n")
        done = subprocess.run(["/bin/sh", "-c", testmap.command()], cwd=tmp_path, env=env)
        assert done.returncode == expected


def test_for_lane_copies_the_map_once_and_records_the_mode(home, tmp_path):
    root = _tree(make_repo(tmp_path / "wt"))  # the files are new to git: all of them count as changed
    first = fasttests.for_lane(home, "p", root, "test")  # no map yet: names and the import graph
    assert not first.mapped and first.tests
    assert testmap.stale(home, "p") and not testmap.usable(home, "p")
    testmap.map_file(home, "p").parent.mkdir(parents=True)
    testmap.map_file(home, "p").write_bytes(b"main map")
    assert not testmap.stale(home, "p")
    second = fasttests.for_lane(home, "p", root, "test")
    assert second.mapped and (root / "collector" / testmap.MAP).read_bytes() == b"main map"
    (root / "collector" / testmap.MAP).write_bytes(b"lane map")  # the lane's own copy is newer: kept
    assert testmap.install(home, "p", root) and (root / "collector" / testmap.MAP).read_bytes() == b"lane map"
    assert [r["mode"] for r in read_jsonl(home / "selections.jsonl")] == ["changed", "map"]


def test_the_map_is_refreshed_only_for_pytest_and_an_old_map(home, tmp_path, monkeypatch):
    started = []
    monkeypatch.setattr("tirith.launch.spawn_detached", lambda argv, h, log: started.append(argv) or 1)
    assert not testmap.refresh(home, "p", tmp_path, "main", "cargo test")
    assert testmap.refresh(home, "p", tmp_path, "main", "cd collector && uv run pytest -q")
    assert started and "testmap" in started[0] and "build" in started[0]
    testmap.map_file(home, "p").write_bytes(b"x")
    assert not testmap.refresh(home, "p", tmp_path, "main", "uv run pytest")  # fresh map
    old = testmap.map_file(home, "p").stat().st_mtime - testmap.MAX_AGE_S - 10
    os.utime(testmap.map_file(home, "p"), (old, old))
    assert testmap.refresh(home, "p", tmp_path, "main", "uv run pytest")


def test_one_full_suite_at_a_time(home):
    with loadlimit.full_suite(home) as first:
        assert first is True
        with loadlimit.full_suite(home, wait=0.0, poll=0.01) as second:
            assert second is False  # waited out: runs anyway
        with loadlimit.slot(home, 1) as slot:  # the stage slots are separate
            assert slot == 0
    with loadlimit.full_suite(home) as again:
        assert again is True


def test_a_ceo_lane_run_skips_the_test_stage(home, tmp_path, monkeypatch, capsys):
    repo = make_repo(tmp_path / "proj")
    (repo / "tirith.toml").write_text('[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "test"\nrun = "echo FULLRUN"\n')
    (home / "config.toml").write_text(f'[[projects]]\nid = "p"\nname = "P"\nrepo = "{repo}"\n', encoding="utf-8")
    monkeypatch.setenv("TIRITH_LANE", "ceo-12345678")
    monkeypatch.chdir(repo)
    assert main(["--home", str(home), "run", "--project", "p", "--stage", "test", "--full"]) == 0
    out = capsys.readouterr().out
    assert "never runs a test suite" in out and "FULLRUN" not in out


def test_build_runs_the_suite_in_its_own_worktree_of_main(home, tmp_path, monkeypatch):
    repo = make_repo(tmp_path / "proj")
    (repo / "collector").mkdir()
    (repo / "collector" / "marker.txt").write_text("v1\n")
    for args in (["add", "."], ["-c", "user.email=t@example.invalid", "-c", "user.name=T", "commit", "-q", "-m", "m"]):
        subprocess.run([GIT, "-C", str(repo), *args], check=True)
    monkeypatch.setattr(testmap, "BASE_COMMAND", 'cp marker.txt "$TESTMON_DATAFILE"')
    head = lambda: subprocess.run([GIT, "-C", str(repo), "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
    ok, output = testmap.build(home, "p", repo, head())
    assert ok, output
    assert testmap.map_file(home, "p").read_text() == "v1\n" and testmap.usable(home, "p")
    (repo / "collector" / "marker.txt").write_text("v2\n")  # a second build moves the same worktree to the new commit
    subprocess.run([GIT, "-C", str(repo), "-c", "user.email=t@example.invalid", "-c", "user.name=T",
                    "commit", "-qam", "m2"], check=True)
    assert testmap.build(home, "p", repo, head())[0] and testmap.map_file(home, "p").read_text() == "v2\n"
    monkeypatch.setattr(testmap, "BASE_COMMAND", "exit 1")
    assert not testmap.build(home, "p", repo, head())[0] and not testmap.usable(home, "p")  # a failed build leaves no map
