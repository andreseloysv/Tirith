"""Where ``tirith`` is, for everything that runs it outside a terminal.

Run actions from the app, the macOS link handler and launchd have no ``tirith`` on PATH. The resolver gives
the absolute command instead: ``[app] tirith`` in config.toml, else the repo's ``bin/tirith``, else
``uv run --project <repo>/collector tirith``. ``rewrite`` turns a stored bare ``tirith ...`` into it.
"""

from __future__ import annotations

import re
import shlex
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
# a bare ``tirith`` word where a command starts: line start, or after ``&&`` ``||`` ``;`` ``|`` ``(``
BARE = re.compile(r"(?m)((?:^|&&|\|\||[;|(])[ \t]*)tirith(?=[ \t]|$)")


def tirith_command(configured: str | None = None, repo: Path | None = None) -> str:
    """The shell-ready command that runs the collector CLI (no trailing space)."""
    repo = Path(repo or REPO)
    if configured:
        return shlex.quote(str(Path(configured).expanduser()))
    script = repo / "bin" / "tirith"
    if script.is_file():
        return shlex.quote(str(script))
    return "uv run --project " + shlex.quote(str(repo / "collector")) + " tirith"


def configured_path(home: Path | None) -> str | None:
    """``[app] tirith`` of ``<home>/config.toml``."""
    from tirith.home import load_config, tirith_home  # late: home does not import this module
    return load_config(Path(home) if home is not None else tirith_home()).app_tirith


def resolved(home: Path | None = None) -> str:
    return tirith_command(configured_path(home))


def rewrite(command: str, home: Path | None = None, cmd: str | None = None) -> str:
    """``command`` with each bare leading ``tirith`` replaced by the resolved command."""
    text = str(command or "")
    if not BARE.search(text):
        return text
    cmd = cmd or resolved(home)
    return BARE.sub(lambda m: m.group(1) + cmd, text)
