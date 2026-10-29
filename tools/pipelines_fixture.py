"""Build web/demo/state.pipelines.js: the demo state with a busy run list (15 runs, 3 projects) for the Pipelines view.

    python3 tools/pipelines_fixture.py   ->   web/demo/state.pipelines.js
Open it with web/index.html#state=demo/state.pipelines.js&view=pipelines
"""
import json
from datetime import datetime, timedelta, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
NOW = datetime(2026, 10, 28, 19, 40, tzinfo=timezone.utc)

STAGES = {
    "android-release": ["build", "tests", "lint", "16kb", "sign", "device", "store", "review"],
    "ci": ["build", "test", "ship"],
    "ios-testflight": ["export", "archive", "upload"],
    "git-hooks": ["pre-commit", "pre-push"],
}
LOGS = {
    "build": "BUILD SUCCESSFUL in %ds", "tests": "tests 82 failures 0", "test": "170 passed in %ds",
    "lint": "no network: OK", "16kb": "16 KB: OK", "sign": "signReleaseBundle", "device": "5 of 5 devices green",
    "export": "export succeeded", "archive": "archive succeeded", "upload": "upload accepted",
    "ship": "shipped", "pre-commit": "lint clean", "pre-push": "tests green", "store": "uploaded to the track",
    "review": "waiting for the store",
}
FAIL = {
    ("archive", "ios-testflight"): ["xcodebuild: archiving", "error: No signing certificate \"iOS Distribution\" found", "** ARCHIVE FAILED **"],
    ("test", "ci"): ["collector/tests/test_gates.py::test_raw_stage FAILED", "E   AssertionError: expected exit 2, got 0", "1 failed, 169 passed in 41s"],
    ("tests", "android-release"): ["CanvasViewModelTest > lowMemory FAILED", "    java.lang.OutOfMemoryError: Java heap space", "tests 82 failures 1"],
    ("lint", "android-release"): ["app/src/main/res/layout/canvas.xml:14: error: Missing contentDescription", "1 error, 0 warnings", "lint: FAILED"],
    ("pre-push", "git-hooks"): ["make test", "FAILED collector/tests/test_cli.py::test_run", "error: pre-push hook failed (exit 2)"],
}


def iso(t):
    return t.strftime("%Y-%m-%dT%H:%M:%SZ")


def make_run(project, pipeline, n, ago_min, plan, lane=None, branch=None, source=None, session=None):
    """plan: [(status, seconds)] per stage; stages after a failure are skipped, the rest stay pending."""
    start = NOW - timedelta(minutes=ago_min)
    t, stages, broken = start, [], False
    for i, name in enumerate(STAGES[pipeline]):
        status, secs = plan[i] if i < len(plan) else ("pending", 0)
        if broken and status == "pending":
            status = "skipped"
        if status in ("passed", "failed", "warning"):
            if status == "failed":
                log = FAIL.get((name, pipeline), ["failed"])
            else:
                log = [LOGS.get(name, "ok").replace("%d", str(secs))]
            stages.append({"name": name, "status": status, "started_at": iso(t), "ended_at": iso(t + timedelta(seconds=secs)),
                           "duration_s": secs, "log_tail": log})
            t += timedelta(seconds=secs)
            broken = broken or status == "failed"
        elif status == "running":
            stages.append({"name": name, "status": status, "started_at": iso(t), "ended_at": None, "duration_s": None,
                           "log_tail": [LOGS.get(name, "working")]})
        else:
            stages.append({"name": name, "status": status, "started_at": None, "ended_at": None, "duration_s": None, "log_tail": []})
    states = [s["status"] for s in stages]
    status = "failed" if "failed" in states else "running" if "running" in states else "passed"
    return {"run": "%s#%d" % (pipeline, n), "project": project, "pipeline": pipeline, "status": status,
            "started_at": iso(start), "ended_at": None if status == "running" else iso(t), "stages": stages,
            "session": session, "lane": lane, "worktree": None, "branch": branch, "source": source}


P, F, R, W = "passed", "failed", "running", "warning"
RUNS = [
    make_run("chromasol", "android-release", 7, 10, [(P, 34), (P, 46), (P, 25), (P, 2), (R, 0)], "w6-perf", "lane/perf", session="2026-10-28-chromasol-release"),
    make_run("tirith", "ci", 15, 3, [(P, 21), (R, 0)], "t-collector", "lane/gitlab"),
    make_run("tirith", "git-hooks", 31, 22, [(P, 3), (P, 44)], None, "main"),
    make_run("tirith", "ci", 14, 41, [(P, 22), (F, 41)], "t-world", "lane/world"),
    make_run("chromasol", "android-release", 6, 95, [(P, 33), (P, 51), (P, 24), (P, 2), (P, 18), (P, 190), (P, 95), (P, 3600)], "owner", "release/1.4"),
    make_run("relagame", "ios-testflight", 3, 150, [(P, 240), (F, 480)], "steward", "main"),
    make_run("tirith", "ci", 13, 175, [(P, 21), (W, 40), (P, 12)], "t-collector", "lane/collector", source="github"),
    make_run("chromasol", "android-release", 5, 240, [(P, 36), (F, 74)], "w1-data", "lane/data"),
    make_run("tirith", "git-hooks", 30, 262, [(P, 4), (F, 44)], None, "lane/hooks"),
    make_run("tirith", "ci", 12, 330, [(P, 20), (P, 38), (P, 11)], "t-world", "lane/world"),
    make_run("relagame", "ios-testflight", 2, 700, [(P, 236), (P, 410), (P, 95)], "owner", "main"),
    make_run("chromasol", "android-release", 4, 900, [(P, 35), (P, 47), (F, 3)], "w7-a11y", "lane/a11y"),
    make_run("tirith", "ci", 11, 1560, [(P, 21), (P, 39), (P, 12)], "t-collector", "main"),
    make_run("chromasol", "android-release", 3, 1700, [(P, 34), (P, 45), (P, 25), (P, 2), (P, 17), (P, 180), (P, 88), (P, 3000)], "steward", "release/1.3"),
    make_run("tirith", "git-hooks", 29, 1900, [(P, 3), (P, 41)], None, "main"),
]
PENDING = {"run": "ios-internal#0", "project": "relagame", "pipeline": "ios-internal", "status": "pending", "started_at": None,
           "ended_at": None, "session": None, "lane": None, "worktree": None, "branch": None, "source": None,
           "stages": [{"name": n, "status": "pending", "started_at": None, "ended_at": None, "duration_s": None, "log_tail": []}
                      for n in ("build", "test", "upload")]}

state = json.loads((ROOT / "fixtures" / "state.demo.json").read_text())
state["pipelines"] = RUNS + [PENDING]
for proj in state["projects"]:
    su = proj.get("setup")
    if not su:
        continue
    if proj["id"] == "relagame":
        su["pipelines"].append({"id": "ios-internal", "name": "iOS internal", "stages": [
            {"name": "build", "kind": "build", "run": "make build", "last": None},
            {"name": "test", "kind": "test", "run": "make test", "last": None},
            {"name": "upload", "kind": "deploy", "run": "make upload-internal", "last": None}]})
    if proj["id"] == "chromasol":
        have = {s["name"] for s in su["pipelines"][0]["stages"]}
        for name, cmd in (("lint", "./gradlew lintRelease"), ("16kb", "./scripts/check-16kb.sh"),
                          ("device", "./scripts/device-test.sh"), ("review", "./scripts/wait-review.sh")):
            if name not in have:
                su["pipelines"][0]["stages"].append({"name": name, "kind": None, "run": cmd, "last": None})
(ROOT / "web" / "demo" / "state.pipelines.js").write_text("window.TIRITH_STATE = " + json.dumps(state, indent=2) + ";\n")
print(len(RUNS), "runs")
