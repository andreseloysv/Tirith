"""Project setup: custom rules, the `setup` block of state.json, `tirith setup` edits, setup requests."""

from __future__ import annotations

import io
import json
from pathlib import Path

import pytest

from conftest import write_config
from tirith import gates
from tirith import inbox as inbox_mod
from tirith import manifest as mf
from tirith import setup as setup_mod
from tirith.cli import main
from tirith.collector import Collector
from tirith.hooks import INSTALL_EVENTS, hook_command
from tirith.home import load_config
from tirith.store import read_jsonl

jsonschema = pytest.importorskip("jsonschema")
ROOT = Path(__file__).resolve().parent.parent.parent
SCHEMA = json.loads((ROOT / "collector" / "state.schema.json").read_text(encoding="utf-8"))

BASE = """# Demo pipelines
deploy = "none"

[[pipelines]]
id = "ci"
name = "CI"

# compile everything
[[pipelines.stages]]
name = "build"
run = "make build"

[[pipelines.stages]]
name = "test"
run = "make test"

[[pipelines]]
id = "nightly"
name = "Nightly"

[[pipelines.stages]]
name = "soak"
run = "make soak"
"""

RULE = """
[[rules]]
id = "no-force-push"
match = "git push --force*"
action = "block"
message = "force pushes rewrite history"
instead = "git push (without --force); ask the owner if history must change"
description = "Nobody may rewrite the shared history by force-pushing."
"""


@pytest.fixture
def proj(home: Path, tmp_path: Path) -> Path:
    folder = tmp_path / "proj"
    folder.mkdir()
    (folder / "tirith.toml").write_text(BASE, encoding="utf-8")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(folder)}],
                 '[agents.anthropic]\ncommand = "alpha-cli"\nsettings = "%s"\n' % (tmp_path / "settings.json"))
    return folder


def _manifest(folder: Path) -> mf.Manifest:
    return mf.load(folder)


def _call(monkeypatch, cwd: Path, tool: str, tool_input: dict) -> int:
    payload = {"hook_event_name": "PreToolUse", "session_id": "s1", "cwd": str(cwd),
               "tool_name": tool, "tool_input": tool_input}
    monkeypatch.setattr("sys.stdin", io.StringIO(json.dumps(payload)))
    return main(["hook", "--agent", "anthropic"])


# -- [[rules]] in tirith.toml -----------------------------------------------------------------


def test_rules_parse(tmp_path: Path) -> None:
    m = mf.parse_text(BASE + RULE + '''
[[rules]]
id = "no-harness-edits"
paths = ["harness/**", "*.lock"]
action = "warn"
message = "the harness is owner territory"
instead = "ask the owner"
description = "Agents may not edit the harness."

[[rules]]
id = "no-curl-bash"
regex = "curl .*\\\\| *(ba)?sh"
action = "block"
message = "piping downloads into a shell"
instead = "download, read, then run"
description = "Piping downloads into a shell is risky."
''', tmp_path / "tirith.toml")
    assert not m.problems
    assert [r.id for r in m.rules] == ["no-force-push", "no-harness-edits", "no-curl-bash"]
    assert m.rules[0].match == "git push --force*" and m.rules[0].action == "block"
    assert m.rules[1].paths == ["harness/**", "*.lock"] and m.rules[1].action == "warn"
    assert m.rules[2].regex


@pytest.mark.parametrize("body,rule", [
    ('match = "x"\naction = "block"\nmessage = "m"\ninstead = "i"', "rule-id"),
    ('id = "r"\naction = "block"\nmessage = "m"\ninstead = "i"', "rule-match"),
    ('id = "r"\nmatch = "x"\nregex = "y"\naction = "block"\nmessage = "m"\ninstead = "i"', "rule-match"),
    ('id = "r"\nregex = "("\naction = "block"\nmessage = "m"\ninstead = "i"', "rule-regex"),
    ('id = "r"\npaths = "harness/**"\naction = "block"\nmessage = "m"\ninstead = "i"', "rule-paths"),
    ('id = "r"\nmatch = "x"\naction = "deny"\nmessage = "m"\ninstead = "i"', "rule-action"),
    ('id = "r"\nmatch = "x"\naction = "block"\ninstead = "i"', "rule-message"),
    ('id = "r"\nmatch = "x"\naction = "block"\nmessage = "m"', "rule-instead"),
    ('id = "raw-stage"\nmatch = "x"\naction = "block"\nmessage = "m"\ninstead = "i"', "rule-id"),
])
def test_bad_rules_are_agent_style_errors(tmp_path: Path, body: str, rule: str) -> None:
    m = mf.parse_text(BASE + "\n[[rules]]\n" + body + "\n", tmp_path / "tirith.toml")
    found = [p for p in m.problems if p.rule == rule]
    assert found, [str(p) for p in m.problems]
    assert found[0].line == BASE.count("\n") + 2  # the [[rules]] header
    assert "Fix:" in str(found[0]) and not m.rules


def test_duplicate_rule_id(tmp_path: Path) -> None:
    m = mf.parse_text(BASE + RULE + RULE, tmp_path / "tirith.toml")
    assert [p.rule for p in m.problems] == ["rule-duplicate"] and len(m.rules) == 1


def test_check_reports_bad_rules(proj: Path, home: Path, capsys) -> None:
    (proj / "tirith.toml").write_text(BASE + '\n[[rules]]\nid = "r"\nmatch = "x"\naction = "nope"\n'
                                      'message = "m"\ninstead = "i"\n', encoding="utf-8")
    assert main(["check", str(proj), "--ci"]) == 1
    assert "rule-action" in capsys.readouterr().out


# -- enforcement ------------------------------------------------------------------------------


@pytest.fixture
def ruled(proj: Path) -> Path:
    (proj / "tirith.toml").write_text(BASE + RULE + '''
[[rules]]
id = "no-harness-edits"
paths = ["harness/**"]
action = "block"
message = "the harness is owner territory"
instead = "ask the owner to change it"
description = "Agents may not edit the harness folder."

[[rules]]
id = "mind-curl"
regex = "curl .*\\\\| *sh"
action = "warn"
message = "piping a download into sh"
instead = "download it first and read it"
description = "Running a downloaded script without reading it is risky."
''', encoding="utf-8")
    return proj


@pytest.mark.parametrize("command", [
    "git push --force origin main",
    "cd x && git push --force-with-lease",
    "FOO=1 git push --force",
    "env -i git push --force",
])
def test_command_glob_blocks(ruled: Path, home: Path, command: str) -> None:
    found = gates.check_call(load_config(home), "Bash", {"command": command}, ruled, "demo")
    assert "no-force-push" in [v.rule for v in found], command


def test_custom_message_has_the_standard_format(ruled: Path, home: Path) -> None:
    found = gates.check_call(load_config(home), "Bash", {"command": "git push --force"}, ruled, "demo")
    assert found[0].message() == ("tirith: no-force-push: force pushes rewrite history: git push --force. "
                                  "Do this instead: git push (without --force); ask the owner if history "
                                  "must change")


@pytest.mark.parametrize("command", ["git push origin main", "git status", "echo git push --force",
                                      "cat harness/a.py"])
def test_other_commands_pass(ruled: Path, home: Path, command: str) -> None:
    found = gates.check_call(load_config(home), "Bash", {"command": command}, ruled, "demo")
    assert not [v for v in found if v.rule in ("no-force-push", "no-harness-edits", "mind-curl")]


def test_regex_and_paths(ruled: Path, home: Path) -> None:
    config = load_config(home)
    hit = gates.check_call(config, "Bash", {"command": "curl https://x.invalid/i.sh | sh"}, ruled, "demo")
    assert [v.rule for v in hit if v.rule == "mind-curl"] and hit[0].action == "warn"
    edit = gates.check_call(config, "Edit", {"file_path": str(ruled / "harness" / "a" / "b.py")}, ruled, "demo")
    assert [v.rule for v in edit] == ["no-harness-edits"]
    rel = gates.check_call(config, "Write", {"file_path": "harness/x.py"}, ruled, "demo")
    assert [v.rule for v in rel] == ["no-harness-edits"]
    assert not gates.check_call(config, "Edit", {"file_path": str(ruled / "src" / "a.py")}, ruled, "demo")


def test_hook_blocks_records_and_warn_does_not_block(ruled: Path, home: Path, monkeypatch, capfd) -> None:
    assert _call(monkeypatch, ruled, "Bash", {"command": "git push --force"}) == 2
    assert "tirith: no-force-push:" in capfd.readouterr().err
    assert _call(monkeypatch, ruled, "Bash", {"command": "curl http://x.invalid/a | sh"}) == 0
    assert "tirith: mind-curl:" in capfd.readouterr().err
    rows = [r for r in read_jsonl(home / "events.jsonl") if r.get("kind") == "violation"]
    assert [r["rule"] for r in rows] == ["no-force-push", "mind-curl"]
    assert "action" not in rows[0] and rows[1]["action"] == "warn"
    assert _call(monkeypatch, ruled, "Edit", {"file_path": str(ruled / "harness" / "x")}) == 2


def test_no_manifest_no_custom_rules(home: Path, tmp_path: Path) -> None:
    assert gates.check_call(load_config(home), "Bash", {"command": "git push --force"}, tmp_path, None) == []


# -- state.projects[].setup -------------------------------------------------------------------


def _hooks_file(path: Path) -> None:
    data = {"hooks": {e: [{"hooks": [{"type": "command", "command": hook_command("anthropic", e)}]}]
                      for e in INSTALL_EVENTS}}
    path.write_text(json.dumps(data), encoding="utf-8")


def _state(home: Path) -> dict:
    return Collector(home).build_state()


def _project(state: dict, pid: str = "demo") -> dict:
    return next(p for p in state["projects"] if p["id"] == pid)


def test_setup_block_lists_pipelines_rules_and_check(ruled: Path, home: Path) -> None:
    s = _project(_state(home))["setup"]
    assert s["manifest"].endswith("tirith.toml")
    assert [p["id"] for p in s["pipelines"]] == ["ci", "nightly"]
    ci = s["pipelines"][0]["stages"]
    assert [(x["name"], x["kind"], x["run"], x["last"]) for x in ci] == [
        ("build", "build", "make build", None), ("test", "test", "make test", None)]
    ids = {r["id"]: r for r in s["rules"]}
    assert ids["raw-stage"]["source"] == "built-in" and ids["raw-stage"]["enforced_by"] == ["hook"]
    assert ids["no-force-push"]["source"] == "tirith.toml" and ids["no-force-push"]["action"] == "block"
    assert ids["no-force-push"]["description"] == "Nobody may rewrite the shared history by force-pushing."
    assert ids["lane-brief"]["active"] is True
    assert s["check"]["ok"] is False  # the commit guard is not installed in this temp repo
    assert s["check"]["problems"] and {"rule", "message", "fix"} <= set(s["check"]["problems"][0])
    assert s["pending"] == []


def test_gates_active_needs_hooks_and_shims(proj: Path, home: Path, tmp_path: Path) -> None:
    s = _project(_state(home))["setup"]
    assert s["gates_active"] is False
    assert s["gates_fix"][0] == "tirith hook install --agent anthropic"
    assert not [r for r in s["rules"] if r["id"] == "raw-stage"][0]["active"]
    assert [r for r in s["rules"] if r["id"] == "lane-brief"][0]["active"]
    assert not (tmp_path / "settings.json").exists()  # reading never installs
    _hooks_file(tmp_path / "settings.json")
    machine = setup_mod.machine_state(load_config(home), tmp_path / "shims")
    assert machine.hooks_ok and not machine.shims_ok and not machine.active
    assert machine.fix == [f"tirith shim install --dir {tmp_path / 'shims'}"]


def test_gates_active_when_all_installed(proj: Path, home: Path, tmp_path: Path, monkeypatch) -> None:
    real = tmp_path / "real"
    real.mkdir()
    exe = real / "alpha-cli"
    exe.write_text("#!/bin/sh\n", encoding="utf-8")
    exe.chmod(0o755)
    shims = tmp_path / "shims"
    config = load_config(home)
    monkeypatch.setenv("PATH", f"{shims}:{real}:/usr/bin:/bin")
    gates.install_shims(config, shims)
    _hooks_file(tmp_path / "settings.json")
    machine = setup_mod.machine_state(config, shims)
    assert machine.active and machine.problems == [] and machine.fix == []
    rows = setup_mod._rules(_manifest(proj), machine, False)
    assert {r["id"]: r["active"] for r in rows}["raw-stage"] and not {r["id"]: r["active"] for r in rows}["commit-guard"]


def test_gates_disabled_in_config_is_inactive(proj: Path, home: Path, tmp_path: Path) -> None:
    _hooks_file(tmp_path / "settings.json")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(proj)}],
                 '[agents.anthropic]\nsettings = "%s"\n\n[gates]\nenabled = false\n' % (tmp_path / "settings.json"))
    machine = setup_mod.machine_state(load_config(home), tmp_path / "shims")
    assert not machine.active and any("enabled = false" in p for p in machine.problems)


def test_last_results_come_from_runs(proj: Path, home: Path) -> None:
    (proj / "tirith.toml").write_text(BASE + '\n[[hooks]]\nevent = "pre-commit"\nrun = "make lint"\n',
                                      encoding="utf-8")
    base = {"run": "ci#1", "project": "demo", "pipeline": "ci", "cwd": str(proj)}
    for event, stage, at, status in [("start", "build", "2026-10-28T10:00:00Z", "running"),
                                     ("end", "build", "2026-10-28T10:00:07Z", "passed"),
                                     ("start", "test", "2026-10-28T10:00:07Z", "running"),
                                     ("end", "test", "2026-10-28T10:00:09Z", "failed")]:
        (home / "events.jsonl").open("a").write(json.dumps({**base, "event": event, "stage": stage,
                                                            "at": at, "status": status}) + "\n")
    (home / "events.jsonl").open("a").write(json.dumps({
        "kind": "git-hook", "at": "2026-10-28T10:05:00Z", "project": "demo", "hook": "pre-commit",
        "status": "passed", "duration_s": 2}) + "\n")
    s = _project(_state(home))["setup"]
    build, test = s["pipelines"][0]["stages"]
    assert build["last"] == {"status": "passed", "at": "2026-10-28T10:00:07Z", "duration_s": 7, "run": "ci#1"}
    assert test["last"]["status"] == "failed" and test["last"]["duration_s"] == 2
    assert s["pipelines"][1]["stages"][0]["last"] is None
    assert s["hooks"][0]["event"] == "pre-commit" and s["hooks"][0]["run"] == "make lint"
    assert s["hooks"][0]["last"] is None or s["hooks"][0]["last"]["status"] in ("passed", "failed")


def test_no_manifest_and_unreadable(home: Path, tmp_path: Path) -> None:
    folder = tmp_path / "bare"
    folder.mkdir()
    write_config(home, [{"id": "bare", "name": "Bare", "repo": str(folder)}])
    s = _project(_state(home), "bare")["setup"]
    assert s["manifest"] is None and s["pipelines"] == [] and s["hooks"] == []
    assert s["check"]["ok"] is False and s["check"]["problems"][0]["rule"] == "manifest-missing"


def test_state_validates_against_schema(ruled: Path, home: Path) -> None:
    jsonschema.validate(_state(home), SCHEMA)


def test_fixtures_have_setup() -> None:
    demo = json.loads((ROOT / "fixtures" / "state.demo.json").read_text(encoding="utf-8"))
    jsonschema.validate(demo, SCHEMA)
    full = [p for p in demo["projects"] if p.get("setup") and p["setup"]["pipelines"]]
    assert len(full) >= 3 and any(r["source"] == "tirith.toml" for p in full for r in p["setup"]["rules"])


# -- editing tirith.toml ----------------------------------------------------------------------


def test_add_stage_inserts_in_its_pipeline_and_keeps_text(proj: Path) -> None:
    setup_mod.add_stage(proj, "ci", "lint", "make lint", kind="test")
    text = (proj / "tirith.toml").read_text(encoding="utf-8")
    assert text.startswith("# Demo pipelines\ndeploy") and "# compile everything\n[[pipelines.stages]]" in text
    m = _manifest(proj)
    assert [s.name for s in m.pipelines[0].stages] == ["build", "test", "lint"]
    assert m.pipelines[0].stages[2].kind == "test" and m.pipelines[1].stages[0].name == "soak"
    assert not m.problems
    # only added lines differ
    assert set(BASE.splitlines()) <= set(text.splitlines())


def test_add_stage_after_and_new_pipeline(proj: Path) -> None:
    setup_mod.add_stage(proj, "ci", "lint", "make lint", after="build")
    assert [s.name for s in _manifest(proj).pipelines[0].stages] == ["build", "lint", "test"]
    setup_mod.add_stage(proj, "release", "ship", "make ship", kind="deploy")
    m = _manifest(proj)
    assert [p.id for p in m.pipelines] == ["ci", "nightly", "release"]
    assert m.pipelines[2].stages[0].kind == "deploy" and not m.problems
    assert [s.name for s in m.pipelines[1].stages] == ["soak"]


def test_add_stage_quotes_awkward_commands(proj: Path) -> None:
    run = 'echo "a \\ b" && printf \'%s\\n\' "é"'
    setup_mod.add_stage(proj, "nightly", "odd", run)
    assert _manifest(proj).pipelines[1].stages[1].run == run


def test_add_stage_errors_are_agent_style(proj: Path) -> None:
    for args, word in [(("ci", "build", "x"), "already exists"), (("ci", "z", "x", "nope"), "kind"),
                       (("ci", "z", "x", None, "ghost"), "--after"), (("ci", "", "x"), "needs"),
                       (("new", "z", "x", None, "build"), "no meaning")]:
        with pytest.raises(setup_mod.SetupError, match=word) as err:
            setup_mod.add_stage(proj, *args)
        assert "Fix:" in str(err.value)
    assert (proj / "tirith.toml").read_text(encoding="utf-8") == BASE


def test_remove_stage(proj: Path) -> None:
    setup_mod.remove_stage(proj, "build")
    text = (proj / "tirith.toml").read_text(encoding="utf-8")
    assert "compile everything" not in text and "make build" not in text
    assert [s.name for s in _manifest(proj).pipelines[0].stages] == ["test"]
    assert "make soak" in text
    with pytest.raises(setup_mod.SetupError, match="not in"):
        setup_mod.remove_stage(proj, "ghost")


def test_remove_stage_ambiguous_needs_pipeline(proj: Path) -> None:
    setup_mod.add_stage(proj, "nightly", "build", "make nightly-build")
    with pytest.raises(setup_mod.SetupError, match="--pipeline"):
        setup_mod.remove_stage(proj, "build")
    setup_mod.remove_stage(proj, "build", "nightly")
    assert [s.name for s in _manifest(proj).pipelines[1].stages] == ["soak"]


def test_add_and_remove_rule(proj: Path) -> None:
    rule = setup_mod.build_rule("no-force-push", "block", "force pushes rewrite history",
                                "git push without --force", match="git push --force*",
                                description="Nobody may force-push.")
    setup_mod.add_rule(proj, rule)
    setup_mod.add_rule(proj, setup_mod.build_rule("no-vendor", "warn", "vendor is generated", "edit src",
                                                  paths=["vendor/**", "*.lock"], description="Vendor files are generated."))
    text = (proj / "tirith.toml").read_text(encoding="utf-8")
    assert text.startswith(BASE.rstrip("\n")) and text.endswith("\n")
    m = _manifest(proj)
    assert not m.problems and [r.id for r in m.rules] == ["no-force-push", "no-vendor"]
    assert m.rules[1].paths == ["vendor/**", "*.lock"]
    with pytest.raises(setup_mod.SetupError, match="already exists"):
        setup_mod.add_rule(proj, rule)
    setup_mod.remove_rule(proj, "no-force-push")
    assert [r.id for r in _manifest(proj).rules] == ["no-vendor"]
    setup_mod.remove_rule(proj, "no-vendor")
    assert (proj / "tirith.toml").read_text(encoding="utf-8").strip() == BASE.strip()
    with pytest.raises(setup_mod.SetupError, match="not in"):
        setup_mod.remove_rule(proj, "no-vendor")


def test_build_rule_validates() -> None:
    for kwargs in [dict(rule_id="", action="block", message="m", instead="i", match="x"),
                   dict(rule_id="a", action="block", message="m", instead="i"),
                   dict(rule_id="a", action="zap", message="m", instead="i", match="x"),
                   dict(rule_id="a", action="block", message="m", instead="i", regex="(")]:
        with pytest.raises(setup_mod.SetupError, match="Fix"):
            setup_mod.build_rule(**kwargs)


def test_edit_a_file_with_rules_before_pipelines(proj: Path) -> None:
    (proj / "tirith.toml").write_text(RULE.lstrip() + "\n" + BASE, encoding="utf-8")
    setup_mod.add_stage(proj, "nightly", "report", "make report")
    m = _manifest(proj)
    assert not m.problems and [s.name for s in m.pipelines[1].stages] == ["soak", "report"]
    assert [r.id for r in m.rules] == ["no-force-push"]


# -- the CLI ----------------------------------------------------------------------------------


def test_cli_add_stage_and_rule_then_check(proj: Path, home: Path, capsys) -> None:
    assert main(["setup", "add-stage", "--project", "demo", "--pipeline", "ci", "--name", "lint",
                 "--run", "make lint", "--kind", "test", "--after", "build"]) == 0
    out = capsys.readouterr().out
    assert "added stage 'lint'" in out and "tirith check" in out
    assert main(["setup", "add-rule", "--project", "demo", "--id", "no-force-push", "--match",
                 "git push --force*", "--action", "block", "--message", "force pushes rewrite history",
                 "--instead", "git push without --force", "--description", "Nobody may force-push."]) == 0
    assert main(["setup", "show", "--project", "demo"]) == 0
    shown = capsys.readouterr().out
    assert "lint [test] make lint" in shown and "rule no-force-push (block)" in shown
    assert main(["setup", "remove-stage", "--project", "demo", "--name", "lint"]) == 0
    assert main(["setup", "remove-rule", "--project", "demo", "--id", "no-force-push"]) == 0
    assert (proj / "tirith.toml").read_text(encoding="utf-8").strip() == BASE.strip()


def test_cli_errors_exit_3_and_changes_introducing_problems_exit_1(proj: Path, home: Path, capsys) -> None:
    assert main(["setup", "add-stage", "--project", "demo", "--pipeline", "ci", "--name", "build",
                 "--run", "x"]) == 3
    assert "already exists" in capsys.readouterr().err
    assert main(["setup", "add-rule", "--project", "demo", "--id", "r", "--action", "block",
                 "--message", "m", "--instead", "i", "--description", "d"]) == 3
    capsys.readouterr()
    # removing the only test stage leaves the pipelines without tests: check now reports it, exit 1
    assert main(["setup", "remove-stage", "--project", "demo", "--name", "test", "--pipeline", "ci"]) == 1
    assert "pipeline-coverage" in capsys.readouterr().out


def test_cli_unknown_project(home: Path) -> None:
    write_config(home, [])
    with pytest.raises(SystemExit):
        main(["setup", "show", "--project", "ghost"])


# -- setup requests in the inbox --------------------------------------------------------------


def _ask(home: Path, **data) -> dict:
    return inbox_mod.add_request(home, "setup", "demo", "", json.dumps(data))


def test_setup_request_is_recorded_and_shown_as_pending(proj: Path, home: Path) -> None:
    row = _ask(home, action="add-stage", pipeline="ci", name="lint", run="make lint")
    assert row["kind"] == "setup" and row["step"] == "setup"
    s = _project(_state(home))["setup"]
    assert [p["id"] for p in s["pending"]] == [row["id"]]
    assert s["pending"][0]["summary"] == "add-stage ci/lint: make lint"
    inbox = _state(home)["inbox"]
    assert inbox[0]["setup"]["name"] == "lint" and inbox[0]["status"] == "open"
    jsonschema.validate(_state(home), SCHEMA)
    inbox_mod.mark_done(home, row["id"])
    assert _project(_state(home))["setup"]["pending"] == []


def test_setup_request_validation(home: Path) -> None:
    for text in ("add a lint stage", json.dumps({"action": "format-disk"}), json.dumps([1])):
        with pytest.raises(inbox_mod.InboxError, match="setup request"):
            inbox_mod.add_request(home, "setup", "demo", "", text)
    assert inbox_mod.requests(home) == []


def test_setup_request_through_the_cli_and_link(home: Path, capsys) -> None:
    text = json.dumps({"action": "run-stage", "pipeline": "ci", "name": "test"})
    assert main(["inbox", "add", "--kind", "setup", "--project", "demo", "--text", text]) == 0
    assert main(["inbox", "list"]) == 0
    assert "run-stage ci/test" in capsys.readouterr().out
    assert main(["inbox", "add", "--kind", "setup", "--project", "demo", "--text", "nope"]) == 3
    fields = inbox_mod.parse_url(inbox_mod.build_url("setup", "demo", "", text, "tok"))
    assert fields["kind"] == "setup" and json.loads(fields["text"])["name"] == "test"


def test_apply_edits_answers_and_closes(proj: Path, home: Path, capsys) -> None:
    row = _ask(home, action="add-stage", pipeline="ci", name="lint", run="make lint", kind="test")
    assert main(["setup", "apply", row["id"]]) == 0
    assert [s.name for s in _manifest(proj).pipelines[0].stages] == ["build", "test", "lint"]
    req = inbox_mod.find_request(home, row["id"])
    assert req["status"] == "done" and "Added stage" in req["answers"][0]["text"]
    rule = _ask(home, action="add-rule", id="no-force-push", match="git push --force*", effect="warn",
                message="force pushes rewrite history", instead="git push without --force",
                description="Nobody may force-push.")
    assert main(["setup", "apply", rule["id"]]) == 0
    assert _manifest(proj).rules[0].action == "warn"
    bad = _ask(home, action="remove-stage", name="ghost")
    assert main(["setup", "apply", bad["id"]]) == 3
    assert "Could not apply" in inbox_mod.find_request(home, bad["id"])["answers"][0]["text"]
    assert main(["setup", "apply", "in-00000000"]) == 3


def test_apply_run_stage_runs_it_through_tirith_run(proj: Path, home: Path) -> None:
    (proj / "tirith.toml").write_text(BASE.replace("make test", "true"), encoding="utf-8")
    row = _ask(home, action="run-stage", pipeline="ci", name="test")
    assert main(["setup", "apply", row["id"]]) == 0
    ends = [e for e in read_jsonl(home / "events.jsonl") if e.get("event") == "end"]
    assert ends and ends[-1]["stage"] == "test" and ends[-1]["status"] == "passed"
    assert inbox_mod.find_request(home, row["id"])["status"] == "done"


# -- every rule describes itself -------------------------------------------------------------


def test_every_built_in_rule_has_a_description() -> None:
    for rid in mf.BUILT_IN_RULES:
        text = gates.DESCRIPTIONS.get(rid, "")
        assert text.strip() and text.endswith("."), f"built-in rule {rid} needs a plain description"
    assert set(gates.DESCRIPTIONS) == set(mf.BUILT_IN_RULES)
    assert {rid for rid, _, _ in setup_mod.BUILT_IN} == set(mf.BUILT_IN_RULES)


def test_check_fails_for_a_custom_rule_without_description(proj: Path, home: Path, capsys) -> None:
    (proj / "tirith.toml").write_text(BASE + '\n[[rules]]\nid = "r"\nmatch = "x"\naction = "block"\n'
                                      'message = "m"\ninstead = "i"\n', encoding="utf-8")
    assert main(["check", str(proj), "--ci"]) == 1
    assert "rule-description" in capsys.readouterr().out


def test_add_rule_requires_a_description(proj: Path, home: Path, capsys) -> None:
    argv = ["setup", "add-rule", "--project", "demo", "--id", "r", "--match", "x", "--message", "m", "--instead", "i"]
    with pytest.raises(SystemExit):
        main(argv)
    assert "--description" in capsys.readouterr().err
    with pytest.raises(setup_mod.SetupError, match="description"):
        setup_mod.build_rule("r", "block", "m", "i", match="x")
    assert main(argv + ["--description", "Stops x."]) == 0
    text = (proj / "tirith.toml").read_text(encoding="utf-8")
    assert 'description = "Stops x."' in text and "described = " in text
    assert not _manifest(proj).problems


def test_changed_match_without_new_description_is_flagged(proj: Path, home: Path, capsys) -> None:
    assert main(["setup", "add-rule", "--project", "demo", "--id", "r", "--match", "git push --force*",
                 "--message", "m", "--instead", "i", "--description", "Stops force pushes."]) == 0
    path = proj / "tirith.toml"
    path.write_text(path.read_text(encoding="utf-8").replace("git push --force*", "git push *"), encoding="utf-8")
    problems = _manifest(proj).problems
    assert [p.rule for p in problems] == ["rule-stale"] and "description may be stale" in problems[0].message
    capsys.readouterr()
    assert main(["check", str(proj), "--ci"]) == 1 and "rule-stale" in capsys.readouterr().out
    assert main(["setup", "describe-rule", "--project", "demo", "--id", "r",
                 "--description", "Stops every push."]) == 0
    assert not _manifest(proj).problems
    assert _manifest(proj).rules[0].description == "Stops every push."


def test_state_rules_counts_the_last_24_hours(ruled: Path, home: Path) -> None:
    from datetime import datetime, timedelta, timezone
    now = datetime.now(timezone.utc)
    rows = [("lane-brief", now - timedelta(hours=2), "w1"), ("lane-brief", now - timedelta(minutes=5), "w2"),
            ("lane-brief", now - timedelta(hours=30), "w3"), ("project-check", now - timedelta(hours=1), "w1")]
    with (home / "events.jsonl").open("a", encoding="utf-8") as fh:
        for rule, ts, lane in rows:
            fh.write(json.dumps({"kind": "violation", "ts": ts.strftime("%Y-%m-%dT%H:%M:%SZ"), "rule": rule,
                                 "agent": "anthropic", "lane": lane, "project": "demo",
                                 "message": "tirith: " + rule + ": x. Do this instead: y"}) + "\n")
    rules = {r["id"]: r for r in _state(home)["rules"]}
    assert rules["lane-brief"]["count_24h"] == 2 and rules["lane-brief"]["last_agent"] == "w2"
    assert rules["lane-brief"]["description"] == gates.DESCRIPTIONS["lane-brief"]
    assert rules["lane-brief"]["source"] == "built-in"
    assert rules["project-check"]["count_24h"] == 1
    assert "no-force-push" not in rules  # hooks are not installed here, so the custom rule is not active
    assert all(r["description"].strip() for r in rules.values())
