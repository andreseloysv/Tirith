"""GitHub Actions runs and cloud-agent pull requests (recorded JSON, no network)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

jsonschema = pytest.importorskip("jsonschema")

from conftest import make_repo  # noqa: E402
from tirith import github as gh  # noqa: E402
from tirith.cli import main  # noqa: E402
from tirith.collector import Collector  # noqa: E402
from tirith.guardwrap import default_words  # noqa: E402
from tirith.home import Config, load_config  # noqa: E402
from tirith.manifest import check  # noqa: E402

DATA = Path(__file__).parent / "data" / "github-api.json"
SCHEMA = json.loads((Path(__file__).resolve().parents[1] / "state.schema.json")
                    .read_text(encoding="utf-8"))
BASE = "https://api.github.com"


class Recorded:
    """A transport that answers from the recorded fixture and counts requests."""

    def __init__(self, etags: bool = False):
        self.data = json.loads(DATA.read_text(encoding="utf-8"))
        self.calls: list[tuple[str, str, str | None]] = []
        self.etags = etags

    def get(self, url, token, etag):
        self.calls.append((url, token, etag))
        key = url.removeprefix(BASE)
        if key not in self.data:
            raise OSError(f"no recording for {key}")
        if self.etags and etag == "v1":
            return 304, None, etag
        return 200, self.data[key], "v1" if self.etags else None


class Entry:
    def __init__(self, pid: str, folder: Path):
        self.id, self.folder = pid, folder


def _reader(transport, clock=lambda: 1000.0):
    return gh.GithubReader(transport, clock=clock, slug_of=lambda folder: "octo/demo")


def _config(**kw) -> Config:
    return Config(github_agent_logins=["cloudbot[bot]"], **kw)


def test_parse_remote() -> None:
    assert gh.parse_remote("https://github.com/octo/demo.git") == "octo/demo"
    assert gh.parse_remote("git@github.com:octo/demo.git") == "octo/demo"
    assert gh.parse_remote("https://tok@github.com/octo/demo") == "octo/demo"
    assert gh.parse_remote("https://gitlab.com/octo/demo.git") is None
    assert gh.parse_remote("") is None


def test_token_sources(monkeypatch: pytest.MonkeyPatch) -> None:
    cfg = Config(github_token_env="MY_TOK")
    assert gh.token_for(cfg, {}) is None
    assert gh.token_for(cfg, {"MY_TOK": "c"}) == "c"
    assert gh.token_for(cfg, {"MY_TOK": "c", "GH_TOKEN": "b"}) == "b"
    assert gh.token_for(cfg, {"MY_TOK": "c", "GH_TOKEN": "b", "GITHUB_TOKEN": "a"}) == "a"


def test_config_reads_github_table(home: Path) -> None:
    (home / "config.toml").write_text(
        '[github]\ntoken_env = "MY_TOK"\nagent_logins = ["cloudbot[bot]"]\n', encoding="utf-8")
    cfg = load_config(home)
    assert cfg.github_token_env == "MY_TOK"
    assert cfg.github_agent_logins == ["cloudbot[bot]"]


def test_no_token_is_silent(tmp_path: Path) -> None:
    transport = Recorded()
    assert _reader(transport).poll(_config(), [Entry("demo", tmp_path)], env={}) == ([], [])
    assert transport.calls == []


def test_runs_map_to_pipelines_with_jobs_as_stages(tmp_path: Path) -> None:
    runs, _ = _reader(Recorded()).poll(_config(), [Entry("demo", tmp_path)], env={"GH_TOKEN": "t"})
    failed, running = runs
    assert failed["source"] == "github" and failed["project"] == "demo"
    assert failed["pipeline"] == "tirith" and failed["run"].endswith("tirith#12")
    assert failed["status"] == "failed"
    assert failed["html_url"] == "https://github.com/octo/demo/actions/runs/9001"
    collector, web = failed["stages"]
    assert (collector["name"], collector["status"], collector["duration_s"]) == ("collector", "failed", 120)
    assert collector["html_url"].endswith("/job/1")
    assert collector["log_tail"] == ["✓ Set up job", "✗ Collector tests"]
    assert web["status"] == "passed" and web["ended_at"] == "2026-10-28T10:00:40Z"
    assert running["status"] == "running" and running["ended_at"] is None
    assert [s["status"] for s in running["stages"]] == ["running", "pending"]


def test_cloud_agent_pull_requests(tmp_path: Path) -> None:
    _, agents = _reader(Recorded()).poll(_config(), [Entry("demo", tmp_path)], env={"GH_TOKEN": "t"})
    assert len(agents) == 1  # the other author is not a configured agent account
    agent = agents[0]
    assert agent["source"] == "github" and agent["html_url"].endswith("/pull/7")
    assert (agent["name"], agent["epithet"], agent["feature"]) == ("cloudbot", "PR #7", "Fix the parser")
    assert agent["status"] == "done"  # every check passed
    assert agent["files_changed"] == 3 and agent["insertions"] == 40 and agent["deletions"] == 7
    assert [f["change"] for f in agent["files"]] == ["modified", "added", "deleted"]


def test_pr_status_from_checks() -> None:
    pr = {"number": 1, "draft": False, "user": {"login": "x"}}
    ok = {"status": "completed", "conclusion": "success"}
    assert gh.map_pr("o/r", "p", pr, [ok, {"status": "in_progress"}], [])["status"] == "working"
    assert gh.map_pr("o/r", "p", pr, [ok, {"status": "completed", "conclusion": "failure"}], []
                     )["status"] == "failed"
    assert gh.map_pr("o/r", "p", pr, [], [])["status"] == "working"
    assert gh.map_pr("o/r", "p", {**pr, "draft": True}, [ok], [])["status"] == "working"


def test_cached_for_sixty_seconds(tmp_path: Path) -> None:
    transport, now = Recorded(), [1000.0]
    reader = _reader(transport, clock=lambda: now[0])
    args = (_config(), [Entry("demo", tmp_path)])
    first = reader.poll(*args, env={"GH_TOKEN": "t"})
    calls = len(transport.calls)
    now[0] += 59
    assert reader.poll(*args, env={"GH_TOKEN": "t"}) == first
    assert len(transport.calls) == calls
    now[0] += 2
    reader.poll(*args, env={"GH_TOKEN": "t"})
    assert len(transport.calls) == 2 * calls


def test_etag_is_sent_and_304_reuses_the_body(tmp_path: Path) -> None:
    transport, now = Recorded(etags=True), [1000.0]
    reader = _reader(transport, clock=lambda: now[0])
    args = (_config(), [Entry("demo", tmp_path)])
    first = reader.poll(*args, env={"GH_TOKEN": "t"})
    assert all(etag is None for _, _, etag in transport.calls)
    now[0] += 61
    again = reader.poll(*args, env={"GH_TOKEN": "t"})
    assert again == first
    assert any(etag == "v1" for _, _, etag in transport.calls)


def test_api_failure_does_not_raise(tmp_path: Path) -> None:
    transport = Recorded()
    transport.data.pop("/repos/octo/demo/pulls?state=open&per_page=30")
    reader = _reader(transport)
    runs, agents = reader.poll(_config(), [Entry("demo", tmp_path)], env={"GH_TOKEN": "t"})
    assert agents == [] and len(runs) == 2  # what was read before the failure is kept
    assert reader.last_error


def test_collector_state_has_github_work_and_validates(home: Path, tmp_path: Path,
                                                       monkeypatch: pytest.MonkeyPatch) -> None:
    repo = make_repo(tmp_path / "repo")
    assert main(["project", "add", str(repo), "--name", "demo"]) == 0
    (home / "config.toml").write_text(
        (home / "config.toml").read_text(encoding="utf-8")
        + '\n[github]\nagent_logins = ["cloudbot[bot]"]\n', encoding="utf-8")
    monkeypatch.setenv("GH_TOKEN", "t")
    collector = Collector(home, client=None, github_transport=Recorded())
    collector.github.slug_of = lambda folder: "octo/demo"
    state = collector.cycle()
    jsonschema.validate(state, SCHEMA)
    remote = [p for p in state["pipelines"] if p.get("source") == "github"]
    assert [p["status"] for p in remote] == ["running", "failed"]  # newest first
    assert [a["id"] for a in state["agents"] if a.get("source") == "github"] == ["gh-octo-demo-pr7"]
    assert state["projects"][0]["id"] == "demo"
    # remote history must not flood the flows feed
    assert not any(str(f.get("from") or f.get("from_id") or "").startswith("pipeline:github")
                   for f in state["flows"])


def test_collector_without_token_has_no_github_rows(home: Path, tmp_path: Path,
                                                    monkeypatch: pytest.MonkeyPatch) -> None:
    for name in ("GITHUB_TOKEN", "GH_TOKEN"):
        monkeypatch.delenv(name, raising=False)
    repo = make_repo(tmp_path / "repo")
    assert main(["project", "add", str(repo)]) == 0
    transport = Recorded()
    state = Collector(home, client=None, github_transport=transport).cycle()
    assert not any(p.get("source") == "github" for p in state["pipelines"])
    assert transport.calls == []


def test_check_ci_skips_the_commit_guard(tmp_path: Path, home: Path) -> None:
    repo = make_repo(tmp_path / "repo")
    (repo / "tirith.toml").write_text(
        'deploy = "none"\n[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "build"\nrun = "true"\n'
        '[[pipelines.stages]]\nname = "test"\nrun = "true"\n', encoding="utf-8")
    assert {p.rule for p in check(repo, default_words())} == {"commit-guard"}
    assert check(repo, default_words(), ci=True) == []
    assert main(["check", str(repo)]) == 1
    assert main(["check", "--ci", str(repo)]) == 0
