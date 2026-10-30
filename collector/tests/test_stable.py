from datetime import datetime, timedelta, timezone

from tirith import stable

T0 = datetime(2026, 1, 1, tzinfo=timezone.utc)


def at(seconds):
    return T0 + timedelta(seconds=seconds)


def _src(tmp_path, tag="1"):
    src = tmp_path / "src"
    (src / "tirith").mkdir(parents=True, exist_ok=True)
    (src / "tirith" / "a.py").write_text(f"v = {tag}\n")
    (src / "tests").mkdir(exist_ok=True)
    (src / "pyproject.toml").write_text("[project]\n")
    return src


def test_stable_advances_only_when_test_passes(home, tmp_path):
    src = _src(tmp_path, "1")
    seen = []
    assert stable.promote(home, src, lambda d: seen.append(d) or True, at(0))["promoted"]
    first = stable.current_dir(home)
    assert first == seen[0].resolve() and (first / "tirith" / "a.py").read_text() == "v = 1\n"
    _src(tmp_path, "2")
    result = stable.promote(home, src, lambda d: False, at(10))
    assert not result["promoted"]
    assert stable.current_dir(home) == first  # stayed
    assert not (stable.stable_root(home) / "releases" / result["release"]).exists()
    assert stable.status(home)["last_rejected"]["reason"]


def test_stable_test_that_crashes_is_red(home, tmp_path):
    def boom(_):
        raise OSError("no runner")

    assert not stable.promote(home, _src(tmp_path), boom, at(0))["promoted"]
    assert stable.current_dir(home) is None


def test_stable_red_health_rolls_back_after_grace(home, tmp_path):
    src = _src(tmp_path, "1")
    stable.promote(home, src, lambda d: True, at(0))
    first = stable.current_dir(home)
    _src(tmp_path, "2")
    stable.promote(home, src, lambda d: True, at(100))
    second = stable.current_dir(home)
    assert second != first
    assert stable.guard(home, "red", at(200)) is None          # red just seen
    assert stable.guard(home, "amber", at(250)) is None        # recovered: timer reset
    assert stable.guard(home, "red", at(300)) is None
    assert stable.guard(home, "red", at(300 + stable.RED_S - 1)) is None
    result = stable.guard(home, "red", at(300 + stable.RED_S))
    assert result["rolled_back"]
    assert stable.current_dir(home) == first
    assert not second.exists()
    assert stable.status(home)["last_rollback"]["to"] == first.name
    assert stable.guard(home, "red", at(5000)) is None         # nothing on trial any more


def test_stable_red_after_trial_window_keeps_release(home, tmp_path):
    src = _src(tmp_path)
    stable.promote(home, src, lambda d: True, at(0))
    stable.promote(home, src, lambda d: True, at(10))
    second = stable.current_dir(home)
    late = stable.WATCH_S + 100
    assert stable.guard(home, "red", at(late)) is None
    assert stable.guard(home, "red", at(late + 1000)) is None
    assert stable.current_dir(home) == second


def test_stable_rollback_without_previous(home, tmp_path):
    stable.promote(home, _src(tmp_path), lambda d: True, at(0))
    assert not stable.rollback(home, "x", at(1))["rolled_back"]
    assert stable.current_dir(home) is not None


def test_stable_old_releases_are_pruned(home, tmp_path):
    src = _src(tmp_path)
    for i in range(6):
        stable.promote(home, src, lambda d: True, at(i * 10))
    folders = list((stable.stable_root(home) / "releases").iterdir())
    assert len(folders) == stable.KEEP
    assert stable.current_dir(home) in [f.resolve() for f in folders]
