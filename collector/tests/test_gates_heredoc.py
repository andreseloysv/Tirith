"""Here-document bodies and quoted text are data, never commands the gates should block."""

from __future__ import annotations

from tirith import gates
from tirith.home import Config

SKIP = "--no-" + "verify"


def test_heredoc_bodies_are_data_not_commands():
    cfg = Config(agents={"muse": {"command": "muse"}})
    line = ("cat > docs/notes.md <<'EOF'\n"
            "Run muse exec to start it; never set core.hooksPath.\n"
            f"git commit {SKIP} is refused.\n"
            "EOF\n"
            "echo done")
    assert gates.check_call(cfg, "Bash", {"command": line}, "/tmp") == []
    assert gates.check_call(cfg, "Bash", {"command": "cat <<-END\n\tmuse exec x\n\tEND"}, "/tmp") == []


def test_commands_after_a_heredoc_are_still_checked():
    cfg = Config(agents={"muse": {"command": "muse"}})
    line = "cat > x.txt <<EOF\nhello\nEOF\nmuse exec go"
    assert [v.rule for v in gates.check_call(cfg, "Bash", {"command": line}, "/tmp")] == ["agent-cli"]


def test_hooks_path_only_counts_in_a_git_command():
    cfg = Config()
    assert gates.check_call(cfg, "Bash", {"command": "grep -r core.hooksPath docs"}, "/tmp") == []
    rules = [v.rule for v in gates.check_call(
        cfg, "Bash", {"command": "git config core.hooksPath /dev/null"}, "/tmp")]
    assert "hooks-path" in rules
