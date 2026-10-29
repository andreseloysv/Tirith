"""File references in the chat: detection, the existence list in the state, the refusal outside the
project folders, the opener and the tirith://open link (docs/CONTRACT-files.md)."""

from __future__ import annotations

import json
import os
from pathlib import Path

import pytest

from conftest import write_config
from tirith import filerefs
from tirith import launch as launch_mod
from tirith.cli import main
from tirith.store import read_jsonl


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    root = tmp_path / "demo"
    for rel in ("web/js/v-plan.js", "docs/TODO.md", "collector/tirith/cli.py", "README.md",
                "app/src-tauri/icons/icon.png"):
        (root / rel).parent.mkdir(parents=True, exist_ok=True)
        (root / rel).write_text("x\n", encoding="utf-8")
    return root


@pytest.mark.parametrize("text, ref", [
    ("web/js/v-plan.js", ("web/js/v-plan.js", None, None)),
    ("app/src-tauri/icons/", ("app/src-tauri/icons/", None, None)),
    ("collector/tirith/cli.py:42", ("collector/tirith/cli.py", 42, None)),
    ("collector/tirith/cli.py:42:7.", ("collector/tirith/cli.py", 42, 7)),
    ("docs/TODO.md.", ("docs/TODO.md", None, None)),
    ("README.md", ("README.md", None, None)),
    ("./bin/tirith", ("./bin/tirith", None, None)),
    ("~/notes/x.md", ("~/notes/x.md", None, None)),
    (".github/workflows/ci.yml", (".github/workflows/ci.yml", None, None)),
])
def test_split_ref_finds_paths_and_lines(text: str, ref) -> None:
    assert filerefs.split_ref(text) == ref


@pytest.mark.parametrize("text", [
    "https://example.com/a/b.js", "file:///tmp/x.md", "tirith", "docs", "--project", "12:30", "v1.2",
    "uv run tirith", "1/2", "a:b", "$HOME/x", "",
])
def test_split_ref_ignores_words_urls_and_commands(text: str) -> None:
    assert filerefs.split_ref(text) is None


def test_refs_in_a_message() -> None:
    text = ("Changed `web/js/v-plan.js` and **docs/TODO.md**, see [the cli](collector/tirith/cli.py:42). "
            "Run `uv run pytest` from https://example.com/a/b.js; README.md.")
    assert filerefs.refs_in(text) == ["web/js/v-plan.js", "docs/TODO.md", "collector/tirith/cli.py", "README.md"]
    assert "example.com/a/b.js" not in " ".join(filerefs.refs_in(text))


def test_resolve_inside_and_refusals(repo: Path, tmp_path: Path) -> None:
    roots = [repo.resolve()]
    assert filerefs.resolve("web/js/v-plan.js", repo, roots) == (repo / "web/js/v-plan.js").resolve()
    assert filerefs.resolve(str(repo / "docs/TODO.md"), None, roots) == (repo / "docs/TODO.md").resolve()
    (tmp_path / "secret.txt").write_text("s", encoding="utf-8")
    with pytest.raises(filerefs.FileRefError, match="outside"):
        filerefs.resolve("../secret.txt", repo, roots)
    with pytest.raises(filerefs.FileRefError, match="outside"):
        filerefs.resolve(str(tmp_path / "secret.txt"), repo, roots)
    os.symlink(tmp_path / "secret.txt", repo / "docs" / "link.md")
    with pytest.raises(filerefs.FileRefError, match="outside"):
        filerefs.resolve("docs/link.md", repo, roots)
    with pytest.raises(filerefs.FileRefError, match="no such file"):
        filerefs.resolve("web/js/missing.js", repo, roots)
    with pytest.raises(filerefs.FileRefError, match="no project folder"):
        filerefs.resolve("web/js/v-plan.js", None, roots)
    with pytest.raises(filerefs.FileRefError, match="not a file path"):
        filerefs.resolve("rm -rf ~", repo, roots)


def test_build_lists_only_existing_paths_per_project(repo: Path, tmp_path: Path) -> None:
    (tmp_path / "outside.md").write_text("x", encoding="utf-8")
    messages = [("demo", "See `web/js/v-plan.js:12` and `web/js/gone.js`, also ../outside.md"),
                ("demo", "docs/TODO.md and app/src-tauri/icons/"),
                ("other", "web/js/v-plan.js"),
                (None, f"{repo}/README.md and {tmp_path}/outside.md")]
    refs = filerefs.build(messages, [{"id": "demo", "folder": str(repo)}, {"id": "other", "folder": None}])
    assert refs == {"": [f"{repo}/README.md"],
                    "demo": ["app/src-tauri/icons", "docs/TODO.md", "web/js/v-plan.js"]}


def test_chat_texts_cover_requests_answers_says_and_waits() -> None:
    got = list(filerefs.chat_texts(
        [{"project": "p", "text": "q", "answers": [{"text": "a"}]}], [{"text": "s"}],
        [{"project": "p", "text": "w"}]))
    assert got == [("p", "q"), ("p", "a"), (None, "s"), ("p", "w")]


def test_open_argv_editor_and_defaults(repo: Path) -> None:
    f, d = repo / "docs/TODO.md", repo / "web"
    assert filerefs.open_argv(f, 42, None, "code") == ["code", "-g", f"{f}:42:1"]
    assert filerefs.open_argv(f, None, None, "/usr/local/bin/subl -n") == ["/usr/local/bin/subl", "-n", f"{f}:1:1"]
    assert filerefs.open_argv(f, 3, 4, "myed --line {line} --col {col} '{file}'") == \
        ["myed", "--line", "3", "--col", "4", str(f)]
    assert filerefs.open_argv(f, 3, None, "nano") == ["nano", str(f)]
    assert filerefs.open_argv(d, 3, None, "code") == ["code", str(d)]
    assert filerefs.open_argv(f, 3, None, None, "darwin") == ["open", "-t", str(f)]
    png = repo / "app/src-tauri/icons/icon.png"
    assert filerefs.open_argv(png, None, None, None, "darwin") == ["open", str(png)]
    assert filerefs.open_argv(d, None, None, None, "darwin") == ["open", str(d)]
    bundle = repo / "Tool.app"
    bundle.mkdir()
    assert filerefs.open_argv(bundle, None, None, None, "darwin") == ["open", "-R", str(bundle)]
    script = repo / "run.command"
    script.write_text("echo hi", encoding="utf-8")
    assert filerefs.open_argv(script, None, None, None, "darwin") == ["open", "-t", str(script)]
    script.chmod(0o755)
    with pytest.raises(filerefs.FileRefError, match="executable"):
        filerefs.open_argv(script, None, None, None, "linux")
    assert filerefs.open_argv(f, None, None, None, "linux") == ["xdg-open", str(f)]
    with pytest.raises(filerefs.FileRefError):
        filerefs.open_argv(f, None, None, None, "win32")


def test_open_file_refuses_outside_and_opens_inside(repo: Path, tmp_path: Path) -> None:
    ran: list[list[str]] = []
    projects = [{"id": "demo", "folder": str(repo)}]
    (tmp_path / "x.md").write_text("x", encoding="utf-8")
    with pytest.raises(filerefs.FileRefError, match="outside"):
        filerefs.open_file(projects, "demo", "../x.md", spawn=ran.append)
    with pytest.raises(filerefs.FileRefError, match="outside"):
        filerefs.open_file(projects, "", str(tmp_path / "x.md"), spawn=ran.append)
    assert ran == []
    target = filerefs.open_file(projects, "demo", "docs/TODO.md", 7, None, "code", spawn=ran.append)
    assert target == (repo / "docs/TODO.md").resolve() and ran == [["code", "-g", f"{target}:7:1"]]


def test_parse_open_url() -> None:
    url = filerefs.build_open_url("demo", "web/js/a b.js", "tok", 12)
    assert filerefs.parse_open_url(url) == {"project": "demo", "path": "web/js/a b.js", "line": 12, "col": None}
    assert filerefs.parse_open_url("tirith://open?path=x.md&line=abc&col=-1")["line"] is None
    with pytest.raises(filerefs.FileRefError):
        filerefs.parse_open_url("tirith://open?project=demo")


def _url_setup(home: Path, repo: Path, monkeypatch: pytest.MonkeyPatch) -> list[list[str]]:
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}], extra='[app]\neditor = "code"\n')
    ran: list[list[str]] = []
    monkeypatch.setattr(filerefs, "_spawn", ran.append)
    return ran


def test_url_open_opens_a_file_at_its_line(home: Path, repo: Path, monkeypatch, capsys) -> None:
    ran = _url_setup(home, repo, monkeypatch)
    url = filerefs.build_open_url("demo", "collector/tirith/cli.py", launch_mod.launch_token(home), 42)
    assert main(["url", "open", url]) == 0
    target = (repo / "collector/tirith/cli.py").resolve()
    assert ran == [["code", "-g", f"{target}:42:1"]] and "opened" in capsys.readouterr().out


def test_url_open_refuses_a_path_outside_the_projects(home: Path, repo: Path, tmp_path: Path, monkeypatch,
                                                      capsys) -> None:
    ran = _url_setup(home, repo, monkeypatch)
    token = launch_mod.launch_token(home)
    for path in ("../tirith-home/launch.json", str(home / "launch.json"), "~/.ssh/id_rsa"):
        assert main(["url", "open", filerefs.build_open_url("demo", path, token)]) == 3
    assert "outside" in capsys.readouterr().err and ran == []


def test_url_open_needs_the_token(home: Path, repo: Path, monkeypatch) -> None:
    ran = _url_setup(home, repo, monkeypatch)
    launch_mod.launch_token(home)
    assert main(["url", "open", filerefs.build_open_url("demo", "docs/TODO.md", "wrong")]) == 3
    assert ran == []
    rows = [r for r in read_jsonl(home / "events.jsonl") if r.get("kind") == "violation"]
    assert rows and rows[-1]["rule"] == "launch-token"


def test_state_carries_file_refs(home: Path, repo: Path) -> None:
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}])
    assert main(["inbox", "add", "--kind", "chat", "--project", "demo", "--text",
                 "What changed in `web/js/v-plan.js` and web/js/nothing.js?"]) == 0
    main(["collect", "--once"])
    state = json.loads((home / "state.json").read_text(encoding="utf-8"))
    refs = state["file_refs"]["demo"]  # Tirith's own waits may name the project folder too
    assert "web/js/v-plan.js" in refs and "web/js/nothing.js" not in refs
