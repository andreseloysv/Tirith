"""Owner commands as actions: the refusal rule, --run on inbox answer / wait add, running, last_run."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

import pytest

from tirith import actions as actions_mod
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith.cli import main
from tirith.collector import Collector
from tirith.store import read_jsonl

REFUSAL = "tirith: owner-command: commands for the owner must be actions. Do this instead: tirith inbox answer <id>"
FENCED = ("Recibido. La tarea es tuya, porque instala algo en tu Mac:\n\n"
          "```sh\ncd ~/DEVELOP/Tirith/collector && uv run tirith url install\n```")


def _request(home: Path) -> str:
    return inbox_mod.add_request(home, "ask", "demo", "1", "what now?")["id"]


# -- the rule ---------------------------------------------------------------------------------


@pytest.mark.parametrize("text", [
    FENCED,
    "Run this:\n```bash\nmake install\n```",
    "```zsh\nls\n```",
    "```console\n$ tirith status\n```",
    "```\ncd ~/x && uv sync\n```",
    "```\nnpm install\n```",
    "```\n./run\n```",
    "```\n$ git pull\n```",
    "~~~sh\nls\n~~~",
    "Ejecuta `uv run tirith url install` y listo.",
    "Ejecuta `cd ~/DEVELOP/Tirith/collector && uv run tirith url install` y listo.",
    "Try `$ ls -la` first",
    "Try `./run --fast`",
])
def test_commands_in_text_are_refused(text: str) -> None:
    with pytest.raises(actions_mod.OwnerCommandError):
        actions_mod.check_text(text)


@pytest.mark.parametrize("text", [
    "Plain text with no command.",
    "Edit `docs/TODO.md` and `web/js/v-plan.js`; the step is `done`.",
    "Use the `tirith` tool or `npm` (alone).",
    "```json\n{\"cd\": 1}\n```",
    "```python\nprint('hi')\n```",
    "```\nsome output without commands\nsecond line\n```",
    "A **bold** claim with `inline_code` and a `Run` button.",
])
def test_ordinary_text_passes(text: str) -> None:
    actions_mod.check_text(text)


def test_inbox_answer_refuses_with_exit_2_and_writes_nothing(home: Path, capsys) -> None:
    rid = _request(home)
    assert main(["inbox", "answer", rid, FENCED]) == 2
    err = capsys.readouterr().err.strip()
    assert err.startswith(REFUSAL) and '--run "<command>" --label "<what it does>"' in err
    assert [r["kind"] for r in read_jsonl(home / "inbox.jsonl")] == ["ask"]


def test_wait_add_refuses_commands(home: Path, capsys) -> None:
    assert main(["wait", "add", "Install it with `uv run tirith url install`", "--project", "demo"]) == 2
    assert "owner-command" in capsys.readouterr().err
    assert not (home / "waits.jsonl").exists()


REAL_ANSWERS = [  # the steward's answers that started this rule (copied, not read from the home folder)
    "Recibido. La tarea de registrar el manejador es tuya:\n\n```sh\ncd ~/DEVELOP/Tirith/collector && uv run tirith url install\n```",
    "1. Instalar los hooks:\n```sh\ncd ~/DEVELOP/Tirith/collector && uv run tirith hook install --agent anthropic\n```\n2. Protegerlos.",
    "Pruébalo tú en una terminal:\n\n```sh\ncd ~/DEVELOP/Tirith/collector && uv run tirith lane run --interactive --project tirith -- agent\n```",
]


def test_real_answers_with_commands_are_refused() -> None:
    for text in REAL_ANSWERS:
        assert actions_mod.find_owner_command(text), text
    assert not actions_mod.find_owner_command("Lo lleva este agente.")


# -- declaring --------------------------------------------------------------------------------


def test_answer_with_actions_is_stored_and_folded(home: Path, capsys) -> None:
    rid = _request(home)
    assert main(["inbox", "answer", rid, "The handler installs something on your Mac.",
                 "--run", "uv run tirith url install", "--label", "Install the link handler",
                 "--run", "echo two", "--cwd", "/tmp"]) == 0
    (row,) = [r for r in read_jsonl(home / "inbox.jsonl") if r.get("kind") == "answer"]
    first, second = row["actions"]
    assert first["id"].startswith("ac-") and first["label"] == "Install the link handler"
    assert first["command"] == "uv run tirith url install" and first["created_by"] == "steward"
    assert second["label"] == "echo two" and second["cwd"] == "/tmp" and first["cwd"] == "/tmp"
    (req,) = inbox_mod.requests(home)
    assert [a["id"] for a in req["answers"][0]["actions"]] == [first["id"], second["id"]]


def test_wait_add_with_actions(home: Path, capsys) -> None:
    assert main(["wait", "add", "Install the handler", "--project", "demo", "--run", "echo hi",
                 "--label", "Say hi"]) == 0
    (row,) = read_jsonl(home / "waits.jsonl")
    assert row["actions"][0]["label"] == "Say hi" and row["actions"][0]["command"] == "echo hi"


def test_label_without_run_is_an_error(home: Path, capsys) -> None:
    rid = _request(home)
    assert main(["inbox", "answer", rid, "text", "--label", "x"]) == 2


# -- running ----------------------------------------------------------------------------------


def _declare(home: Path, command: str, cwd: str | None = None) -> str:
    rid = _request(home)
    args = ["inbox", "answer", rid, "Do it.", "--run", command]
    if cwd:
        args += ["--cwd", cwd]
    assert main(args) == 0
    return actions_mod.list_actions(home)[-1]["id"]


def test_run_passes_and_logs(home: Path, tmp_path: Path, capsys) -> None:
    aid = _declare(home, "pwd; echo out; echo err >&2", str(tmp_path))
    assert main(["action", "run", aid]) == 0
    rows = actions_mod.run_rows(home)
    assert [(r["status"], r["exit_code"]) for r in rows] == [("running", None), ("passed", 0)]
    assert all(r["action_id"] == aid and r["at"].endswith("Z") for r in rows)
    log = (home / "actions" / (aid + ".log")).read_text()
    assert str(tmp_path.resolve()) in log and "out" in log and "err" in log
    run = actions_mod.last_run(home, aid)
    assert run["status"] == "passed" and run["log_tail"][-1] == "err"


def test_run_fail_and_log_tail_is_20_lines(home: Path, tmp_path: Path, capsys) -> None:
    aid = _declare(home, "seq 1 50; exit 3", str(tmp_path))
    assert main(["action", "run", aid]) == 1
    run = actions_mod.last_run(home, aid)
    assert (run["status"], run["exit_code"]) == ("failed", 3)
    assert run["log_tail"] == [str(n) for n in range(31, 51)]


def test_never_run_has_no_last_run(home: Path) -> None:
    aid = _declare(home, "true")
    assert actions_mod.last_run(home, aid) is None


def test_refuses_while_running_and_unknown_ids(home: Path, tmp_path: Path, capsys) -> None:
    aid = _declare(home, "true", str(tmp_path))
    actions_mod.append_jsonl(home / "inbox.jsonl", {"kind": "action-run", "action_id": aid, "status": "running",
                                                    "exit_code": None, "at": actions_mod.now_iso()})
    assert main(["action", "run", aid]) == 3
    assert "already running" in capsys.readouterr().err
    assert main(["action", "run", "ac-nope"]) == 3


def test_default_cwd_is_the_project_folder(home: Path, tmp_path: Path, capsys) -> None:
    from conftest import make_repo, write_config
    repo = make_repo(tmp_path / "repo")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}])
    aid = _declare(home, "pwd")
    assert main(["action", "run", aid]) == 0
    assert str(repo.resolve()) in (home / "actions" / (aid + ".log")).read_text()


def test_action_list(home: Path, capsys) -> None:
    aid = _declare(home, "true")
    assert main(["action", "list", "--json"]) == 0
    assert json.loads(capsys.readouterr().out)["id"] == aid


# -- the browser link -------------------------------------------------------------------------


def test_run_link_needs_the_token(home: Path, tmp_path: Path, capsys) -> None:
    aid = _declare(home, "echo linked", str(tmp_path))
    assert main(["url", "open", "tirith://run?action=" + aid + "&token=wrong"]) == 3
    assert actions_mod.last_run(home, aid) is None
    token = launch_mod.launch_token(home)
    assert main(["url", "open", "tirith://run?action=" + aid + "&token=" + token]) == 0
    assert actions_mod.last_run(home, aid)["status"] == "passed"


def test_run_link_never_takes_command_text(home: Path, tmp_path: Path) -> None:
    token = launch_mod.launch_token(home)
    assert main(["url", "open", "tirith://run?command=touch%20x&token=" + token]) == 3


# -- the collector ----------------------------------------------------------------------------


def test_state_carries_last_run(home: Path, tmp_path: Path, capsys) -> None:
    aid = _declare(home, "echo hi", str(tmp_path))
    assert main(["wait", "add", "Please", "--project", "demo", "--run", "false", "--cwd", str(tmp_path)]) == 0
    wait_action = actions_mod.list_actions(home)[-1]["id"]
    assert main(["action", "run", aid]) == 0
    assert main(["action", "run", wait_action]) == 1
    state = Collector(home, client=None).build_state(datetime.now(timezone.utc))
    ans_action = state["inbox"][-1]["answers"][0]["actions"][0]
    assert ans_action["last_run"]["status"] == "passed" and ans_action["last_run"]["log_tail"] == ["hi"]
    wait = next(w for w in state["waiting_on_you"] if w["actions"])
    assert wait["actions"][0]["last_run"]["status"] == "failed"
    assert wait["actions"][0]["last_run"]["exit_code"] == 1
