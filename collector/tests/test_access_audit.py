from __future__ import annotations

import io
import json
from pathlib import Path

from tirith import access_audit, hooks
from tirith.store import lane_path, write_json_atomic


def _run(home: Path, payload: dict, monkeypatch, event="PreToolUse") -> None:
    monkeypatch.setenv("TIRITH_LANE", "w1")
    hooks.run_hook(home, "anthropic", event, io.StringIO(json.dumps(payload)))


def test_pretooluse_writes_access_line_without_secrets(tmp_path, monkeypatch) -> None:
    home = tmp_path / "home"
    write_json_atomic(lane_path(home, "w1"), {"id": "w1", "item": "Audit it", "item_step": "21"})
    cmd = "curl -H 'Authorization: Bearer abc123SECRET' https://x.io -d password=hunter2\nsecond line"
    _run(home, {"hook_event_name": "PreToolUse", "session_id": "s", "cwd": "/tmp",
                "tool_name": "Bash", "tool_input": {"command": cmd}}, monkeypatch)
    files = list((home / "access").glob("*.ndjson"))
    assert len(files) == 1
    row = json.loads(files[0].read_text().splitlines()[0])
    assert row["agent"] == "anthropic" and row["lane"] == "w1" and row["system"] == "network"
    assert row["plan"] == "21: Audit it" and row["ts"]
    text = files[0].read_text()
    assert "abc123SECRET" not in text and "hunter2" not in text and "second line" not in text


def test_other_events_are_not_logged(tmp_path, monkeypatch) -> None:
    home = tmp_path / "home"
    _run(home, {"hook_event_name": "PostToolUse", "cwd": "/tmp", "tool_name": "Edit",
                "tool_input": {"file_path": "/tmp/a"}}, monkeypatch, event="PostToolUse")
    assert not (home / "access").exists()


def test_system_and_scrub() -> None:
    assert access_audit.system_of("Bash", {"command": "git status"}) == "git"
    assert access_audit.system_of("Edit", {}) == "files"
    assert "s3cr3t" not in access_audit.scrub("API_KEY=s3cr3t run")
    assert "pw" not in access_audit.scrub("git clone https://u:pw@host/r")


def test_recent_reads_all_projects_oldest_first(tmp_path) -> None:
    from tirith.store import append_jsonl

    append_jsonl(tmp_path / "access" / "tirith.ndjson", {"ts": "2026-11-05T10:00:00Z", "agent": "anthropic"})
    append_jsonl(tmp_path / "access" / "_loose.ndjson", {"ts": "2026-11-05T09:00:00Z", "agent": "muse"})
    rows = access_audit.recent(tmp_path)
    assert [(r["agent"], r["project"]) for r in rows] == [("muse", None), ("anthropic", "tirith")]
    assert access_audit.recent(tmp_path / "none") == []


def test_external_names_what_a_call_reaches() -> None:
    e = access_audit.external
    assert e("Bash", {"command": "curl -s https://api.github.com/repos/x/y"}) == ["api api.github.com"]
    assert e("Bash", {"command": "node am.mjs watch --url wss://agentsmessenger.com/socket"}) == ["websocket agentsmessenger.com"]
    assert e("WebFetch", {"url": "https://docs.python.org/3/"}) == ["web docs.python.org"]
    assert e("Bash", {"command": "git clone git@github.com:a/b.git"}) == ["git github.com"]
    assert e("Bash", {"command": "cd x && git push origin main"}) == ["git remote"]
    assert e("Bash", {"command": "ssh -p 22 me@build.example.com uptime"}) == ["ssh build.example.com"]
    assert e("Bash", {"command": "uv sync && npm install left-pad"}) == ["package pypi.org", "package registry.npmjs.org"]
    assert e("Bash", {"command": "gh run list"}) == ["github api.github.com"]
    assert e("WebSearch", {"query": "pytest xdist"}) == ["search pytest xdist"]
    assert e("mcp__docs__batch", {}) == ["mcp docs"]
    assert e("Bash", {"command": "curl https://user:pw@example.com/api/v1/x"}) == ["api example.com"]  # no credentials
    assert e("Bash", {"command": "curl http://localhost:8080/x"}) == [] and e("Bash", {"command": "ls"}) == []


def test_the_network_watch_records_a_lanes_new_connections_once(tmp_path) -> None:
    from tirith import netwatch
    lsof = "\n".join(["p500", "cpython3", "n10.0.0.2:5555->140.82.112.3:443", "n127.0.0.1:1->127.0.0.1:2",
                      "p900", "cSafari", "n10.0.0.2:6666->17.1.1.1:443"])
    ps = "  500  400\n  400  300\n  900    1\n"
    run = lambda argv: lsof if argv[0] == "lsof" else ps  # noqa: E731
    lanes = [{"id": "lane-a", "pid": 300, "project": "demo", "agent": "anthropic"}, {"id": "old", "pid": 1, "ended_at": "x"}]
    netwatch._last["at"] = 0.0
    rows = netwatch.sample(tmp_path, lanes, now=1000.0, run=run, name=lambda h: "lb.github.com", resolve=lambda d: set())
    assert [r["external"] for r in rows] == [["https github.com:443"]]  # python3 under lane-a; Safari and loopback are not
    assert rows[0]["domain"] == "github.com" and rows[0]["address"] == "140.82.112.3" and rows[0]["reverse"] == "lb.github.com"
    assert rows[0]["tool"] == "connection" and rows[0]["system"] == "network" and "python3" in rows[0]["action"]
    assert rows[0]["runtime"] is False  # python3 is a process the lane's work started, not the agent itself
    assert netwatch.sample(tmp_path, lanes, now=1005.0, run=run) == []  # not before EVERY_S
    assert netwatch.sample(tmp_path, lanes, now=1100.0, run=run, name=lambda h: "lb.github.com") == []  # seen once
    logged = [json.loads(l) for l in (tmp_path / "access" / "demo.ndjson").read_text().splitlines()]
    assert len(logged) == 1 and logged[0]["lane"] == "lane-a"


def test_the_domain_a_lane_named_wins_over_the_reverse_name(tmp_path) -> None:
    from tirith import netwatch
    (tmp_path / "access").mkdir()
    (tmp_path / "access" / "demo.ndjson").write_text(json.dumps(
        {"lane": "lane-a", "tool": "Bash", "external": ["api api.github.com", "git remote"]}) + "\n")
    lane = {"id": "lane-a", "project": "demo"}
    assert netwatch.lane_domains(tmp_path, lane) == ["api.github.com"]
    resolve = lambda d: {"140.82.112.6"} if d == "api.github.com" else set()  # noqa: E731
    assert netwatch.domain_of("140.82.112.6", "lb-140-82-112-6-iad.github.com", ["api.github.com"], resolve) == "api.github.com"
    assert netwatch.domain_of("1.2.3.4", "ec2-1-2-3-4.compute.amazonaws.com", [], resolve) == "amazonaws.com"
    assert netwatch.domain_of("1.2.3.4", "1.2.3.4", [], resolve) is None  # no name at all: the address stays
    assert netwatch.base_domain("news.bbc.co.uk") == "bbc.co.uk" and netwatch.base_domain("10.0.0.1") is None


def test_a_known_domain_names_an_address_with_no_reverse_name() -> None:
    from tirith import netwatch
    resolve = lambda d: {"160.79.104.10"} if d == "api.anthropic.com" else set()  # noqa: E731
    assert netwatch.domain_of("160.79.104.10", "160.79.104.10", [], resolve) == "api.anthropic.com"
    rows = [{"tool": "connection", "external": ["https 160.79.104.10:443"]}, {"tool": "Bash"}]
    netwatch.annotate(rows, resolve, certs=lambda a: {})
    assert rows[0]["domain"] == "api.anthropic.com" and rows[0]["address"] == "160.79.104.10" and "domain" not in rows[1]


def test_the_servers_certificate_names_an_unknown_address() -> None:
    from tirith import netwatch
    text = ("subject=C=US, ST=New York, L=New York, O=Datadog, Inc., CN=*.logs.us5.datadoghq.com\n"
            "X509v3 Subject Alternative Name: \n    DNS:*.logs.us5.datadoghq.com, DNS:logs.us5.datadoghq.com\n")
    assert netwatch.read_cert("34.149.66.165", run=lambda argv: text) == {"domain": "logs.us5.datadoghq.com", "org": "Datadog"}
    text = "subject=CN=downloads.example.org\nX509v3 Subject Alternative Name: \n    DNS:downloads.example.org, DNS:x\n"
    assert netwatch.read_cert("35.190.46.17", run=lambda argv: text) == {"domain": "downloads.example.org"}
    assert netwatch.read_cert("10.0.0.1", run=lambda argv: "") == {}
    rows = [{"tool": "connection", "external": ["https 35.190.46.17:443"]}]
    netwatch.annotate(rows, resolve=lambda d: set(), certs=lambda a: {"domain": "downloads.example.org"})
    assert rows[0]["domain"] == "downloads.example.org"


def test_env_names_lists_the_variables_a_call_uses_never_values() -> None:
    e = access_audit.env_names
    assert e("Bash", {"command": 'curl -H "Authorization: Bearer $GITHUB_TOKEN" x; echo ${GEMINI_API_KEY:-no} $HOME $1'}) == \
        ["GITHUB_TOKEN", "GEMINI_API_KEY"]  # the shell's own names and positions are left out
    assert e("Bash", {"command": "printenv OPENAI_API_KEY; export BUILD_FLAVOR=dev"}) == ["OPENAI_API_KEY", "BUILD_FLAVOR"]
    code = 'k = os.environ["BRAIN_API_KEY"]\nh = os.getenv("TIRITH_HOME")\nconst t = process.env.SLACK_TOKEN'
    assert e("Write", {"content": code}) == ["BRAIN_API_KEY", "SLACK_TOKEN"]  # read by code an agent wrote
    assert e("Read", {"file_path": "x"}) == [] and access_audit.env_secret("SLACK_TOKEN")
    assert not access_audit.env_secret("BUILD_FLAVOR")


def test_a_destination_shows_the_key_its_lane_had_for_it(tmp_path) -> None:
    names = access_audit.secret_names({"GITHUB_TOKEN": "x", "GH_ENTERPRISE_TOKEN": "y", "ANTHROPIC_API_KEY": "z",
                                       "TIRITH_LANE_TOKEN": "t", "PATH": "/bin"})
    assert names == ["ANTHROPIC_API_KEY", "GH_ENTERPRISE_TOKEN", "GITHUB_TOKEN"]  # names only, Tirith's own left out
    write_json_atomic(lane_path(tmp_path, "lane-a"), {"id": "lane-a", "env_secrets": ["GITHUB_TOKEN"]})
    rows = [{"tool": "proxy", "lane": "lane-a", "domain": "api.github.com", "external": ["https api.github.com:443"]},
            {"tool": "proxy", "lane": "lane-a", "domain": "pypi.org", "external": ["https pypi.org:443"]},
            {"tool": "proxy", "lane": "old", "domain": "api.github.com"}]
    access_audit.annotate_keys(tmp_path, rows)
    assert (rows[0]["keys"], rows[0]["login"]) == (["GITHUB_TOKEN"], False)
    assert (rows[1]["keys"], rows[1]["login"]) == ([], True)  # no key for it: its own saved login, if any
    assert "keys" not in rows[2]  # a lane from before keys were noted


def test_words_inside_quotes_are_no_command() -> None:
    # a sed that rewrote "ls; npm test" into "ls; npm install" inside a test file was taken for a package install
    e = access_audit.external
    assert e("Bash", {"command": "sed -i 's/ls; npm test/ls; npm install/' tests/test_x.py"}) == []
    assert e("Bash", {"command": 'git commit -m "fix ssh deploy and git push docs"'}) == []
    assert e("Bash", {"command": 'curl -s "https://api.example.com/v1/x"'}) == ["api api.example.com"]  # a quoted URL is
    assert e("Bash", {"command": "npm install left-pad"}) == ["package registry.npmjs.org"]


def test_a_work_lane_never_gets_a_name_reserved_for_tirith_s_own(tmp_path) -> None:
    from conftest import make_repo
    from tirith import launch
    repo = make_repo(tmp_path / "repo")
    # "Manager de guardia…" gave "manager-de-guardia…", and the lane was counted as the manager, not as work
    assert launch.lane_id_for("Manager de guardia, chat y piloto", set(), repo).startswith("step-manager-de-guardia")
    assert launch.lane_id_for("CEO: revisar", set(), repo).startswith("step-ceo-")
    assert launch.lane_id_for("Fix the parser", set(), repo) == "fix-the-parser"
