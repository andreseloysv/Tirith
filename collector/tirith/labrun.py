"""``tirith lab run <scenario>``: run a scenario in a throwaway copy of the company-lab fixture.

The agent is a command speaking JSON lines (so tests use a fake one, no model calls). The runner writes
``{"type":"request","text":..., "workdir":...}`` to its stdin and reads events from its stdout:
``cost`` (usd), ``elapsed`` (seconds), ``item`` (reached main), ``ask`` (question; the owner's answer is
written back as ``{"type":"answer","text":...}``), ``promote`` (role, reason), ``done``.
The owner is scripted: an awake owner answers each ask from the scenario's ``owner_replies`` (or the
event's ``owner_reply``) and each ask counts as an interruption; an unattended scenario puts the owner
to sleep, so asks get no real answer and count as ``blocked_asks`` instead.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
from pathlib import Path

DEFAULT_REPLY = "Go ahead."
SLEEP_REPLY = "(owner is asleep; decide on your own)"


def load_scenario(lab: Path, name: str) -> dict:
    data = json.loads((lab / "scenarios.json").read_text(encoding="utf-8"))
    for sc in data["scenarios"]:
        if sc["id"] == name:
            return {**sc, "check_command": data.get("check_command")}
    raise KeyError(f"unknown scenario {name!r}; known: {', '.join(s['id'] for s in data['scenarios'])}")


def run_scenario(lab: Path, name: str, agent_cmd: list[str], timeout: float = 600) -> dict:
    scenario = load_scenario(lab, name)
    replies = list(scenario.get("owner_replies") or [])
    unattended = bool(scenario.get("unattended"))
    report = {"scenario": name, "cost_usd": 0.0, "seconds": 0.0, "items_to_main": 0,
              "owner_interruptions": 0, "blocked_asks": 0, "promotions": [], "duplicate_items": 0,
              "checked": None, "ok": False}
    seen_items: set[str] = set()
    with tempfile.TemporaryDirectory(prefix="tirith-lab-") as tmp:
        work = Path(tmp) / "company-lab"
        shutil.copytree(lab, work, ignore=shutil.ignore_patterns(".git", "__pycache__", ".pytest_cache"))
        proc = subprocess.Popen(agent_cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, cwd=work)

        def send(obj: dict) -> None:
            try:
                proc.stdin.write(json.dumps(obj) + "\n")
                proc.stdin.flush()
            except (BrokenPipeError, ValueError):
                pass

        send({"type": "request", "text": scenario["owner_request"], "workdir": str(work)})
        try:
            for line in proc.stdout:
                try:
                    ev = json.loads(line)
                except ValueError:
                    continue
                kind = ev.get("type")
                if kind == "cost":
                    report["cost_usd"] += float(ev.get("usd", 0))
                elif kind == "elapsed":
                    report["seconds"] += float(ev.get("seconds", 0))
                elif kind == "item":
                    key = str(ev.get("id") or ev.get("title") or len(seen_items))
                    if key in seen_items:
                        report["duplicate_items"] += 1
                    else:
                        seen_items.add(key)
                        report["items_to_main"] += 1
                elif kind == "ask":
                    if unattended:
                        report["blocked_asks"] += 1
                        send({"type": "answer", "text": SLEEP_REPLY})
                    else:
                        report["owner_interruptions"] += 1
                        send({"type": "answer", "text": replies.pop(0) if replies else DEFAULT_REPLY})
                elif kind == "promote":
                    report["promotions"].append({"role": ev.get("role", "?"), "reason": ev.get("reason", "")})
                elif kind == "done":
                    break
        finally:
            try:
                proc.stdin.close()
            except OSError:
                pass
            try:
                proc.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                proc.kill()
        check = scenario.get("check")
        if check:
            res = subprocess.run(check, shell=True, cwd=work, capture_output=True, text=True)
            report["checked"] = res.returncode == 0
    report["cost_usd"] = round(report["cost_usd"], 6)
    report["ok"] = report["checked"] is not False and report["items_to_main"] >= 1
    return report


def markdown(report: dict) -> str:
    promos = "\n".join(f"- {p['role']}: {p['reason']}" for p in report["promotions"]) or "- none"
    return (f"# Lab run: {report['scenario']} ({'ok' if report['ok'] else 'FAILED'})\n\n"
            f"- cost: ${report['cost_usd']:.4f}\n- time: {report['seconds']:.0f}s\n"
            f"- items to main: {report['items_to_main']}\n"
            f"- owner interruptions: {report['owner_interruptions']} (blocked while asleep: {report['blocked_asks']})\n"
            f"- check: {report['checked']}\n\n## Role promotions\n{promos}\n")


def write_report(report: dict, out: Path) -> tuple[Path, Path]:
    out.mkdir(parents=True, exist_ok=True)
    j, m = out / f"{report['scenario']}.json", out / f"{report['scenario']}.md"
    j.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    m.write_text(markdown(report), encoding="utf-8")
    return j, m
