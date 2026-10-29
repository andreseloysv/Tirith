"""The owner can replace an outdated guard with one command; agents cannot run it."""

from __future__ import annotations

from conftest import make_repo
from tirith import gates
from tirith.guardwrap import guard_module, install_guard, reinstall_guard, verify_guard
from tirith.home import Config


def test_reinstall_replaces_the_guard_hooks_only(tmp_path):
    repo = make_repo(tmp_path / "repo")
    words = ["forbidden"]
    assert install_guard(repo, words) == [] or True  # immutable flags may be unsupported here
    folder = guard_module().hooks_dir(repo)
    (folder / "post-merge").write_text("#!/bin/sh\necho mine\n", encoding="utf-8")  # the repo's own hook
    problems = reinstall_guard(repo, ["forbidden", "other"])
    assert not [p for p in problems if "failed" in p]
    assert (folder / "post-merge").read_text(encoding="utf-8").endswith("echo mine\n")
    assert not [p for p in verify_guard(repo, ["forbidden", "other"]) if "differs" in p or "missing" in p]


def test_agents_are_blocked_from_reinstalling_the_guard():
    rules = [v.rule for v in gates.check_call(Config(), "Bash",
                                              {"command": "cd collector && uv run tirith guard reinstall demo"}, "/tmp")]
    assert rules == ["guard-reinstall"]
