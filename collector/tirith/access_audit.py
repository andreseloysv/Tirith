"""Access audit: one line per agent tool call in ``<home>/access/<project>.ndjson``.

Docs: ``docs/CONTRACT-hooks.md`` ("Access audit"). Called by the tool hook (``PreToolUse``), so it only
appends, never touches the network and never raises. Secret values are never stored: the action text is
cut to its first line and every token, password, key or ``NAME=value`` secret is replaced by ``***``.
"""

from __future__ import annotations

import re
from pathlib import Path

from tirith.store import append_jsonl, lane_path, read_json

ACCESS_DIR = "access"
ACTION_MAX = 200
LOOSE = "_loose"  # calls outside any project

_SECRET_NAME = r"[A-Za-z0-9_.-]*(?:SECRET|TOKEN|PASSWORD|PASSWD|PWD|API[_-]?KEY|PRIVATE[_-]?KEY|CREDENTIAL|AUTH)[A-Za-z0-9_.-]*"
_PATTERNS = (
    (re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----.*", re.S), "***"),
    (re.compile(r"(?i)\b(bearer|basic|token)(\s+)[^\s'\"]+"), r"\1\2***"),
    (re.compile(r"(?i)\b(authorization\s*[:=]\s*)(?!(?:bearer|basic|token)\b)[^\s'\"]+"), r"\1***"),
    (re.compile(r"(?i)(--?(?:password|passwd|token|secret|api-?key|auth)[=\s]+)[^\s'\"]+"), r"\1***"),
    (re.compile(r"(?i)\b(" + _SECRET_NAME + r")(\s*[=:]\s*)(\"[^\"]*\"|'[^']*'|[^\s'\"]+)"), r"\1\2***"),
    (re.compile(r"(?i)(://[^/\s:@]+:)[^@\s/]+@"), r"\1***@"),
    (re.compile(r"\b(?:sk|pk|ghp|gho|ghs|github_pat|xox[a-z]|AKIA|AIza)[-_A-Za-z0-9]{12,}"), "***"),
    (re.compile(r"\b[A-Fa-f0-9]{32,}\b"), "***"),
    (re.compile(r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_.-]+"), "***"),
)
_URL = re.compile(r"https?://[^\s'\"]+")
_SYSTEMS = (
    ("git", re.compile(r"(?:^|[;&|(]\s*|\s)git\s")),
    ("github", re.compile(r"(?:^|[;&|(]\s*)gh\s")),
    ("tirith", re.compile(r"(?:^|[;&|(]\s*)tirith\s")),
    ("network", re.compile(r"(?:^|[;&|(]\s*)(?:curl|wget|ssh|scp|rsync|nc)\s|https?://")),
    ("package", re.compile(r"(?:^|[;&|(]\s*)(?:npm|pnpm|yarn|pip3?|uv|brew|cargo)\s")),
)
_TOOL_SYSTEMS = {"Read": "files", "Glob": "files", "Grep": "files", "Edit": "files", "Write": "files",
                 "MultiEdit": "files", "NotebookEdit": "files", "WebFetch": "network",
                 "WebSearch": "network", "Task": "agent", "Agent": "agent", "Bash": "shell"}


def scrub(text, limit: int = ACTION_MAX) -> str:
    """First line of ``text`` with secret values masked, cut to ``limit`` characters."""
    out = ""
    for line in str(text or "").splitlines():
        if line.strip():
            out = line.strip()
            break
    for pattern, repl in _PATTERNS:
        out = pattern.sub(repl, out)
    return out if len(out) <= limit else out[:limit - 1] + "…"


def system_of(tool, tool_input) -> str:
    """Which system a call touches: ``git``, ``github``, ``network``, ``files``, ``shell``…"""
    tool = str(tool or "")
    data = tool_input if isinstance(tool_input, dict) else {}
    if tool == "Bash":
        command = str(data.get("command") or data.get("cmd") or "")
        for name, pattern in _SYSTEMS:
            if pattern.search(command):
                return name
        return "shell"
    if tool.startswith("mcp__"):
        return "mcp:" + (tool.split("__")[1] if tool.count("__") >= 2 else tool)
    return _TOOL_SYSTEMS.get(tool, "other")


def action_of(tool, tool_input, cwd=None) -> str:
    """Short, secret-free description of the call (never the file content or a prompt body)."""
    data = tool_input if isinstance(tool_input, dict) else {}
    tool = str(tool or "")
    if tool == "Bash":
        return "Bash: " + scrub(data.get("command") or data.get("cmd"))
    path = data.get("file_path") or data.get("path") or data.get("notebook_path")
    if path:
        base = str(cwd or "").rstrip("/")
        text = str(path)
        return f"{tool} " + scrub(text[len(base) + 1:] if base and text.startswith(base + "/") else text)
    url = data.get("url")
    if url:
        return f"{tool} " + scrub(_URL.search(str(url)).group(0) if _URL.search(str(url)) else url)
    if tool in ("Task", "Agent"):
        return f"{tool}: " + scrub(data.get("description") or data.get("subagent_type"))
    for key in ("pattern", "query"):
        if data.get(key):
            return f"{tool}: " + scrub(data[key])
    return tool


_ENDPOINT = re.compile(r"\b((?:https?|wss?|ftp)://)([^/\s'\"?#]+)([^\s'\"]*)")
_SSH = re.compile(r"(?:^|[;&|(]\s*|\s)(?:ssh|scp|rsync|sftp)\s+(?:-\S+\s+(?:\S+\s+)?)*(?:[\w.-]+@)?([\w.-]+\.[\w.-]+|[\w-]+)(?=[:\s]|$)")
_GIT_REMOTE = re.compile(r"(?:^|[;&|(]\s*|\s)git\s+(?:-C\s+\S+\s+)?(push|pull|fetch|clone|ls-remote)\b")
_GIT_HOST = re.compile(r"(?:git@|https?://)([\w.-]+)[:/]")
_GH = re.compile(r"(?:^|[;&|(]\s*)gh\s")
_INSTALL = re.compile(r"(?:^|[;&|(]\s*)(npm|pnpm|yarn|pip3?|uv|cargo|brew|gem|go)\s+(?:\S+\s+)*?(install|add|i|sync|update|upgrade|get|fetch)\b")
_REGISTRY = {"npm": "registry.npmjs.org", "pnpm": "registry.npmjs.org", "yarn": "registry.yarnpkg.com",
             "pip": "pypi.org", "pip3": "pypi.org", "uv": "pypi.org", "cargo": "crates.io", "brew": "formulae.brew.sh",
             "gem": "rubygems.org", "go": "proxy.golang.org"}


def _kind(scheme: str, path: str, host: str = "") -> str:
    scheme = scheme.lower().rstrip(":/")
    if scheme in ("ws", "wss"):
        return "websocket"
    if scheme == "ftp":
        return "ftp"
    api = host.lower().startswith("api.") or re.search(r"/(?:api|v\d+|graphql|rpc)(?:/|$)|\.json\b", path)
    return "api" if api else "web"


def external(tool, tool_input) -> list[str]:
    """What outside the machine a call reaches, ``"<kind> <host>"`` each (web, api, websocket, ssh, git, github,
    package, search, mcp): from the URLs, hosts and commands in the call. Empty for local work."""
    data = tool_input if isinstance(tool_input, dict) else {}
    tool, out = str(tool or ""), []

    def add(item: str) -> None:
        if item not in out:
            out.append(item)

    if tool.startswith("mcp__"):
        add("mcp " + (tool.split("__")[1] if tool.count("__") >= 2 else tool))
    if tool == "WebSearch":
        add("search " + scrub(data.get("query"), 60))
    text = str(data.get("command") or data.get("cmd") or data.get("url") or "") if tool in ("Bash", "WebFetch") else ""
    for scheme, host, path in _ENDPOINT.findall(text):
        host = re.sub(r"^[^@]*@", "", host)  # never a user or a password
        if host.split(":")[0] not in ("localhost", "127.0.0.1", "0.0.0.0", "[::1]"):
            add(f"{_kind(scheme, path, host)} {host}")
    if tool == "Bash":
        for host in _SSH.findall(text):
            add(f"ssh {host}")
        if _GIT_REMOTE.search(text):
            found = _GIT_HOST.search(text)
            add("git " + (found.group(1) if found else "remote"))
        if _GH.search(text):
            add("github api.github.com")
        for manager, _ in _INSTALL.findall(text):
            add(f"package {_REGISTRY.get(manager, manager)}")
    return out


_ENV_PATTERNS = (
    re.compile(r"\$\{?([A-Za-z_][A-Za-z0-9_]*)"),                                   # $NAME, ${NAME}
    re.compile(r"\b(?:printenv|unset|export)\s+([A-Za-z_][A-Za-z0-9_]*)"),           # printenv NAME, export NAME=
    re.compile(r"\benv\b[^|;&]*\|\s*grep\s+(?:-\w+\s+)*['\"]?([A-Za-z_][A-Za-z0-9_]{2,})"),  # env | grep NAME
    re.compile(r"os\.environ(?:\.get)?\s*[\[(]\s*['\"]([A-Za-z_][A-Za-z0-9_]*)"),  # os.environ["X"], .get("X")
    re.compile(r"os\.getenv\(\s*['\"]([A-Za-z_][A-Za-z0-9_]*)"),                     # os.getenv("X")
    re.compile(r"process\.env(?:\.|\[\s*['\"])([A-Za-z_][A-Za-z0-9_]*)"),          # process.env.X, ["X"]
    re.compile(r"(?:std::)?env::var(?:_os)?\(\s*\"([A-Za-z_][A-Za-z0-9_]*)"),         # Rust env::var("X")
)
# the shell's and the system's own names, and Tirith's: noise when the owner asks which variables agents use
_ENV_IGNORED = {"PATH", "HOME", "PWD", "OLDPWD", "USER", "LOGNAME", "SHELL", "TMPDIR", "TMP", "TEMP", "LANG", "LC_ALL",
                "TERM", "IFS", "RANDOM", "SECONDS", "LINENO", "PPID", "UID", "EUID", "HOSTNAME", "PS1", "PS2", "COLUMNS",
                "LINES", "EDITOR", "PAGER", "SHLVL", "OSTYPE", "BASH_SOURCE", "FUNCNAME", "PIPESTATUS", "REPLY", "OPTARG",
                "OPTIND", "BASHPID", "CI", "VIRTUAL_ENV", "PYTHONPATH", "NODE_ENV", "XDG_CONFIG_HOME", "XDG_CACHE_HOME"}
_ENV_SECRET = re.compile(r"SECRET|TOKEN|PASSWORD|PASSWD|API_?KEY|PRIVATE|CREDENTIAL|AUTH|_KEY$|^KEY_", re.I)


def env_names(tool, tool_input) -> list[str]:
    """The environment variable names a call uses (never their values): in a command, or read by the code a call
    writes. Single letters, digits-only positions and the shell's own names are left out."""
    data = tool_input if isinstance(tool_input, dict) else {}
    tool = str(tool or "")
    if tool == "Bash":
        text = str(data.get("command") or data.get("cmd") or "")
    elif tool in ("Write", "Edit", "MultiEdit", "NotebookEdit"):
        text = " ".join(str(data.get(k) or "") for k in ("content", "new_string", "new_source"))
        text += " ".join(str(e.get("new_string") or "") for e in data.get("edits") or [] if isinstance(e, dict))
    else:
        return []
    out: list[str] = []
    for pattern in _ENV_PATTERNS:
        for name in pattern.findall(text):
            if len(name) > 1 and name not in _ENV_IGNORED and not name.startswith("TIRITH_") and name not in out:
                out.append(name)
    return out[:20]


def env_secret(name: str) -> bool:
    return bool(_ENV_SECRET.search(name))


def plan_point(home: Path, lane_id) -> str | None:
    """The plan point a lane works on: ``<step>: <item>`` from its lane file, or None."""
    if not lane_id:
        return None
    lane = read_json(lane_path(home, lane_id), {}) or {}
    item = str(lane.get("item") or "").strip()
    step = str(lane.get("item_step") or lane.get("step") or "").strip()
    if not item and not step:
        return None
    return (f"{step}: " if step else "") + scrub(item, 160)


def record(home: Path, event: dict, tool_input=None) -> dict | None:
    """Append the access line for one ``PreToolUse`` hook row; other events and tool-less rows are skipped."""
    if event.get("event") != "PreToolUse" or not event.get("tool"):
        return None
    home = Path(home)
    project = re.sub(r"[^A-Za-z0-9._-]", "_", str(event.get("project") or LOOSE)).lstrip(".") or LOOSE
    row = {"ts": event.get("ts"), "agent": event.get("agent"), "lane": event.get("lane"),
           "plan": plan_point(home, event.get("lane")), "system": system_of(event["tool"], tool_input),
           "tool": event["tool"], "action": action_of(event["tool"], tool_input, event.get("cwd"))}
    reached = external(event["tool"], tool_input)
    if reached:
        row["external"] = reached
    names = env_names(event["tool"], tool_input)
    if names:
        row["env"] = names
    append_jsonl(home / ACCESS_DIR / f"{project}.ndjson", row)
    return row


STATE_ROWS = 600  # newest rows put in state.json for the page's «Accesos» view


def recent(home: Path, limit: int = STATE_ROWS) -> list[dict]:
    """The newest ``limit`` rows of every project file, oldest first, each with its ``project``. Read only."""
    from tirith.store import read_jsonl

    rows: list[dict] = []
    try:
        files = sorted((Path(home) / ACCESS_DIR).glob("*.ndjson"))
    except OSError:
        return []
    for path in files:
        project = None if path.stem == LOOSE else path.stem
        for row in read_jsonl(path)[-limit:]:
            rows.append({**row, "project": project})
    rows.sort(key=lambda r: str(r.get("ts") or ""))
    return rows[-limit:]
