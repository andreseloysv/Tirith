"""A project with ``guard = false`` in config.toml has no commit guard on purpose (the canary):
the check that gates lanes must not ask for one."""
import subprocess

from tirith import manifest


def _repo(tmp_path):
    subprocess.run(["git", "init", "-q", str(tmp_path)], check=True)
    (tmp_path / "tirith.toml").write_text(
        'deploy = "none"\n\n[[pipelines]]\nid = "ci"\n\n[[pipelines.stages]]\nname = "build"\nrun = "true"\n\n'
        '[[pipelines.stages]]\nname = "test"\nrun = "true"\n')
    return tmp_path


def test_guard_off_skips_the_commit_guard_rule(tmp_path):
    repo = _repo(tmp_path)
    assert "commit-guard" in {p.rule for p in manifest.check(repo, ["secret"])}
    assert manifest.check(repo, ["secret"], guard=False) == []


def test_autopilot_hold_passes_the_project_guard_setting(tmp_path, monkeypatch):
    from tirith import autopilot, loadlimit
    from tirith.home import Config, ProjectConfig
    seen = {}

    def fake_check(folder, words, ci=False, guard=True):
        seen["guard"] = guard
        return []
    monkeypatch.setattr(manifest, "check", fake_check)
    monkeypatch.setattr(loadlimit, "current", lambda config, home: (5, ""))
    proj = ProjectConfig(id="canary", name="canary", repo=_repo(tmp_path / "r"), color="#888888", guard=False)
    config = Config(projects=[proj])
    autopilot._hold(tmp_path / "home", config, "canary", "1", [], None)
    assert seen == {"guard": False}
