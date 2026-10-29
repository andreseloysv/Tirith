"""File references in the chat: the paths a message names, which of them exist, and opening one.

The collector lists, per project, the paths named in chat messages that exist inside a registered project
folder (``state.file_refs``); the page links only those. A click opens the file through the app
(``open_file``) or the ``tirith://open`` link, which resolves the path again here and refuses anything
outside the registered project folders. See ``docs/CONTRACT-files.md``.
"""

from __future__ import annotations

import os
import re
import shlex
import subprocess
import sys
from pathlib import Path
from urllib.parse import parse_qs, urlencode, urlsplit

REPO = Path(__file__).resolve().parents[2]  # the tirith repo: the root of a message with no project

# a word of a message: what is left between spaces, quotes, brackets and markdown marks
WORD = re.compile(r"[^\s`'\"<>()\[\]{}|,;*]+")
SHAPE = re.compile(r"(?:~/|\.{1,2}/|/)?[\w.@+-]+(?:/[\w.@+-]+)*/?")
REF = re.compile(r"(.+?)(?::(\d+)(?::(\d+))?)?")
NAME_EXT = re.compile(r"[\w@+-]*[A-Za-z][\w@+-]*(?:\.[\w-]+)*\.[A-Za-z][A-Za-z0-9]{0,9}|\.[A-Za-z][\w.-]*")
TRAIL = ".,:;!?"
MAX_REFS = 2000  # checked paths per collector cycle

# editors that take a line: `code -g file:line:col`, `subl file:line:col`
GOTO_FLAG = {"code", "code-insiders", "codium", "cursor", "windsurf"}
COLON_FORM = {"subl", "zed", "idea", "pycharm", "webstorm"}
# opened with the default app on macOS; everything else goes to the default text editor (`open -t`),
# so a script, an app or a `.command` file is never run by a click
VIEW = {".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp", ".tif", ".tiff", ".heic", ".ico", ".icns",
        ".pdf", ".mp4", ".mov", ".m4a", ".mp3", ".wav"}


class FileRefError(Exception):
    """A path that cannot be opened; the message says why."""


def looks_like_path(path: str) -> bool:
    """True for ``web/js/v-plan.js``, ``app/src-tauri/icons/``, ``README.md``; not for words, URLs, flags."""
    if not path or "://" in path or path.startswith("-") or not SHAPE.fullmatch(path):
        return False
    if not re.search(r"[A-Za-z]", path):
        return False
    return ("/" in path.strip("/") or path.endswith("/") or path.startswith(("~/", "./", "../", "/"))
            or bool(NAME_EXT.fullmatch(path)))


def split_ref(text: str) -> tuple[str, int | None, int | None] | None:
    """``"cli.py:42:7."`` -> ``("cli.py", 42, 7)``; None when it is not a path."""
    match = REF.fullmatch(text.rstrip(TRAIL))
    if not match or not looks_like_path(match.group(1)):
        return None
    line, col = match.group(2), match.group(3)
    return match.group(1), int(line) if line else None, int(col) if col else None


def norm(path: str) -> str:
    """The key a path is listed under: no leading ``./``, no trailing ``/``."""
    while path.startswith("./"):
        path = path[2:]
    return path.rstrip("/") or path


def refs_in(text: str) -> list[str]:
    """Every path-like word of a message (normalized, each once). A superset of what the page links."""
    out: list[str] = []
    for word in WORD.findall(str(text or "")):
        ref = split_ref(word)
        if ref and norm(ref[0]) not in out:
            out.append(norm(ref[0]))
    return out


def _real(path: Path) -> Path | None:
    try:
        return path.resolve(strict=True)
    except (OSError, RuntimeError):
        return None


def resolve(path: str, root: Path | None, roots: list[Path]) -> Path:
    """The real file ``path`` names (relative to ``root``); refuses one outside every folder of ``roots``."""
    path = str(path or "").strip()
    if not looks_like_path(path):
        raise FileRefError(f"not a file path: {path!r}")
    target = Path(os.path.expanduser(path)) if path.startswith("~/") else Path(path)
    if not target.is_absolute():
        if root is None:
            raise FileRefError(f"{path}: the message has no project folder to look in")
        target = root / target
    real = _real(target)
    if real is None:
        raise FileRefError(f"{path}: no such file")
    if not any(real == r or r in real.parents for r in roots):
        raise FileRefError(f"{path}: outside the registered project folders")
    return real


def folders_of(projects: list[dict]) -> dict[str, Path]:
    """``{project id: real folder}`` from ``state.projects`` rows (or anything with ``id`` and ``folder``)."""
    out: dict[str, Path] = {}
    for row in projects:
        folder = row.get("folder")
        real = _real(Path(str(folder)).expanduser()) if folder else None
        if row.get("id") and real is not None and real.is_dir():
            out[str(row["id"])] = real
    return out


def root_for(project: str | None, folders: dict[str, Path]) -> Path | None:
    """The folder relative paths of a message resolve against: its project's, else the tirith repo."""
    if project:
        return folders.get(project)
    return _real(REPO)


def build(messages, projects: list[dict]) -> dict[str, list[str]]:
    """``state.file_refs``: per project id (``""`` for none), the paths its messages name that exist.

    ``messages`` is an iterable of ``(project, text)``.
    """
    folders = folders_of(projects)
    roots = list(folders.values())
    found: dict[str, set[str]] = {}
    checked: dict[tuple[str, str], bool] = {}
    for project, text in messages:
        key = str(project or "")
        root = root_for(key, folders)
        for path in refs_in(text):
            seen = (key, path)
            if seen not in checked:
                if len(checked) >= MAX_REFS:
                    break
                try:
                    resolve(path, root, roots)
                    checked[seen] = True
                except FileRefError:
                    checked[seen] = False
            if checked[seen]:
                found.setdefault(key, set()).add(path)
    return {key: sorted(paths) for key, paths in sorted(found.items())}


def chat_texts(inbox: list[dict], says: list[dict], waits: list[dict]):
    """``(project, text)`` for every chat message of the state: requests, answers, says and waits."""
    for row in inbox:
        yield row.get("project"), row.get("text")
        for answer in row.get("answers") or []:
            yield row.get("project"), answer.get("text")
    for row in [*says, *waits]:
        yield row.get("project"), row.get("text")


# -- opening ------------------------------------------------------------------------------------


def open_argv(target: Path, line: int | None = None, col: int | None = None,
              editor: str | None = None, platform: str = sys.platform) -> list[str]:
    """The command that opens ``target``: the ``[app] editor`` (with the line when it takes one), else
    the OS default (macOS: text files in the default text editor, folders in Finder)."""
    line, col = line or 1, col or 1
    if editor and editor.strip():
        words = shlex.split(editor)
        if any("{file}" in w for w in words):
            return [w.replace("{file}", str(target)).replace("{line}", str(line)).replace("{col}", str(col))
                    for w in words]
        name = Path(words[0]).name
        if target.is_dir():
            return [*words, str(target)]
        if name in GOTO_FLAG:
            return [*words, "-g", f"{target}:{line}:{col}"]
        if name in COLON_FORM:
            return [*words, f"{target}:{line}:{col}"]
        return [*words, str(target)]
    if platform == "darwin":
        if target.is_dir():  # a bundle (a folder with an extension, e.g. .app) is shown, never started
            return ["open", "-R", str(target)] if target.suffix else ["open", str(target)]
        return ["open", str(target)] if target.suffix.lower() in VIEW else ["open", "-t", str(target)]
    if platform.startswith("win"):
        raise FileRefError("opening files needs [app] editor in config.toml on this platform")
    if target.is_file() and (os.access(target, os.X_OK) or target.suffix == ".desktop"):
        raise FileRefError(f"{target.name} is executable: set [app] editor in config.toml to open it")
    return ["xdg-open", str(target)]


def open_file(projects: list[dict], project: str, path: str, line: int | None = None,
              col: int | None = None, editor: str | None = None, spawn=None) -> Path:
    """Resolve, check and open one path of a chat message. Returns the opened file."""
    folders = folders_of(projects)
    target = resolve(path, root_for(project, folders), list(folders.values()))
    argv = open_argv(target, line, col, editor)
    try:
        (spawn or _spawn)(argv)
    except OSError as error:
        raise FileRefError(f"could not run {argv[0]}: {error}") from error
    return target


def _spawn(argv: list[str]) -> None:
    subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                     start_new_session=True)


# -- tirith://open ------------------------------------------------------------------------------


def parse_open_url(url: str) -> dict:
    """``tirith://open?project=p&path=a/b.py&line=3&token=t`` -> ``{project, path, line, col}``."""
    query = {k: v[-1] for k, v in parse_qs(urlsplit(str(url).strip()).query).items()}
    path = query.get("path", "")
    if not path:
        raise FileRefError("the link names no path")

    def number(key: str) -> int | None:
        value = query.get(key, "")
        return int(value) if value.isdigit() and 0 < int(value) < 10_000_000 else None

    return {"project": query.get("project", ""), "path": path, "line": number("line"), "col": number("col")}


def build_open_url(project: str, path: str, token: str, line: int | None = None) -> str:
    fields = {"project": project, "path": path, **({"line": line} if line else {}), "token": token}
    return "tirith://open?" + urlencode(fields)
