"""Blocked destinations: rules, the tool gate, the network watch and the page's request (tirith.netblock)."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

from tirith import dispatch, gates, inbox, netblock, netwatch
from tirith.home import load_config


def test_rules_match_a_domain_its_subdomains_and_address_ranges(tmp_path: Path) -> None:
    assert netblock.add(tmp_path, "https://Facebook.com/x") == "facebook.com"
    netblock.add(tmp_path, "*.datadoghq.com")
    netblock.add(tmp_path, "157.240.0.0/16")
    rules = netblock.rules(tmp_path)
    assert netblock.blocked(rules, "graph.facebook.com:443") == "facebook.com"
    assert netblock.blocked(rules, "logs.us5.datadoghq.com") == "datadoghq.com"
    assert netblock.blocked(rules, "157.240.210.16") == "157.240.0.0/16"
    assert netblock.blocked(rules, "notfacebook.com", "160.79.104.10", None) is None
    assert netblock.remove(tmp_path, "facebook.com") and not netblock.remove(tmp_path, "facebook.com")


def test_the_gate_refuses_a_call_to_a_blocked_destination(home: Path) -> None:
    netblock.add(home, "example.com")
    config = load_config(home)
    found = gates.check_call(config, "Bash", {"command": "curl -s https://api.example.com/v1/x"}, str(home), home=home)
    assert [v.rule for v in found] == ["net-blocked"] and "api.example.com" in found[0].what
    assert gates.check_call(config, "WebFetch", {"url": "https://docs.python.org/3/"}, str(home), home=home) == []


def test_the_watch_stops_a_lane_process_that_reaches_a_blocked_destination(tmp_path: Path) -> None:
    netblock.add(tmp_path, "157.240.0.0/16")
    lsof = "\n".join(["p500", "cpython3", "n10.0.0.2:5555->157.240.210.16:443",
                      "p300", "cagent", "n10.0.0.2:5556->157.240.210.17:443"])
    run = lambda argv: lsof if argv[0] == "lsof" else "  500  300\n"  # noqa: E731
    killed = []
    netwatch._last["at"] = 0.0
    rows = netwatch.sample(tmp_path, [{"id": "lane-a", "pid": 300, "project": "demo"}], now=1000.0, run=run,
                           name=lambda h: h, resolve=lambda d: set(), kill=lambda pid: killed.append(pid) or True)
    assert killed == [500]  # the script, never the agent program itself (pid 300)
    by = {r["address"]: r for r in rows}
    assert by["157.240.210.16"]["blocked"] == "157.240.0.0/16" and by["157.240.210.16"]["action"].startswith("stopped python3")
    assert "was not stopped" in by["157.240.210.17"]["action"]
    assert netwatch.sample(tmp_path, [{"id": "lane-a", "pid": 300, "project": "demo"}], now=1000.0 + netwatch.BLOCKED_EVERY_S,
                           run=run, name=lambda h: h, resolve=lambda d: set(), kill=lambda pid: killed.append(pid) or True)
    assert killed == [500, 500]  # stopped again each time it comes back; the agent's own line is written once


def test_the_page_blocks_and_unblocks_at_once(home: Path) -> None:
    req = inbox.add_request(home, "setup", "all", "setup", json.dumps({"action": "net-block", "match": "example.com"}))
    now = datetime(2026, 11, 5, 8, tzinfo=timezone.utc)
    dispatch.run(home, load_config(home), now, agents=[], lanes=[], status_of=lambda lane, now: "working")
    assert [r["match"] for r in netblock.rules(home)] == ["example.com"]
    row = next(r for r in inbox.requests(home) if r["id"] == req["id"])
    assert row["status"] == "done" and row["answers"][-1]["text"].startswith("Blocked example.com")
    inbox.add_request(home, "setup", "all", "setup", json.dumps({"action": "net-unblock", "match": "example.com"}))
    dispatch.run(home, load_config(home), now, agents=[], lanes=[], status_of=lambda lane, now: "working")
    assert netblock.rules(home) == []
