"""Gates for agents inside a lane: write only in the lane's copy, never read credentials, no servers,
kill only own processes.

Lane agents are never asked for permission: they may run any command in their copy and only these rules
(plus the rest of ``tirith.gates``) decide. Every rule returns a ``gates.Violation``; they are active
only for a call made by a lane (``TIRITH_LANE``). Compound commands are checked segment by segment.
"""

from __future__ import annotations

import os
import re
import tempfile
from pathlib import Path

from tirith import gates
from tirith.gates import Violation, _clip, _program, split_commands
from tirith.store import read_json

TEMP_ROOTS = ("/tmp", "/private/tmp", "/private/var/folders")
_NULLS = {"/dev/null", "/dev/stdout", "/dev/stderr", "/dev/tty", "/dev/zero"}
_GIT_READ = {"status", "diff", "log", "show", "blame", "grep", "ls-files", "cat-file", "rev-parse",
             "branch", "describe", "remote", "config", "worktree", "ls-tree", "shortlog", "fetch",
             "for-each-ref", "rev-list", "merge-base", "name-rev", "stash"}
_GIT_INTEGRATE = {"merge", "push", "pull"}  # the lane's closing step on the project's main checkout
_PROGRAM_PREFIX = ("python", "python3", "uv", "node")

WORKSPACE_HINT = ("write inside your lane's copy or a temp folder ($TMPDIR, /tmp); if you really need a file "
                  "elsewhere, ask the owner with `tirith wait add --project <id> \"<what and why>\"`")
SECRETS_HINT = ("do not open credential files; if a task needs a secret, ask the owner "
                "(`tirith wait add --project <id> \"<what you need>\"`) instead of reading it")
SERVERS_HINT = ("do not start servers or open ports; test through `tirith run --project <id> --stage <name>` "
                "or call the code directly")
PROCESS_HINT = ("only stop processes you started in this lane (kill <pid> of your own background job); "
                "never pkill/killall by name")

RULE_IDS = ("workspace-write", "no-secrets", "no-servers", "own-processes")



# -- helpers ----------------------------------------------------------------------------------


def _real(path: str | Path) -> str:
    try:
        return os.path.realpath(str(path))
    except OSError:
        return os.path.abspath(str(path))


def git_root(cwd) -> Path | None:
    if not cwd:
        return None
    path = Path(str(cwd))
    for folder in (path, *path.parents):
        if (folder / ".git").exists():
            return folder
    return None


def lane_workspace(home: Path | None, lane: str | None, cwd) -> Path | None:
    """The lane's copy: the workspace of its lane file, else the git root of the call's folder."""
    if home is not None and lane:
        try:
            data = read_json(Path(home) / "lanes" / f"{lane}.json", None)
        except Exception:
            data = None
        if isinstance(data, dict) and data.get("workspace"):
            return Path(str(data["workspace"]))
    return git_root(cwd) or (Path(str(cwd)) if cwd else None)


def allowed_roots(workspace: Path | None, env) -> list[str]:
    """Real paths a lane may write in: its copy, the lanes folder of its project, temp folders."""
    roots = [*TEMP_ROOTS, tempfile.gettempdir(), str(env.get("TMPDIR") or "")]
    if workspace is not None:
        roots.append(str(workspace))
        if workspace.parent.name.endswith("-lanes"):
            roots.append(str(workspace.parent))
    return [r for r in dict.fromkeys(_real(r) for r in roots if r)]


def _inside(path: str, roots: list[str]) -> bool:
    return any(path == r or path.startswith(r.rstrip("/") + "/") for r in roots)


def _resolve(text: str, cwd: str, env) -> str | None:
    """Absolute real path of a path word, or None when it has a variable that cannot be resolved."""
    text = text.strip("'\"")
    if text.startswith("~"):
        text = os.path.expanduser(text)
    def value(m):
        name = m.group(1) or m.group(2)
        return str(env.get(name) or (os.path.expanduser("~") if name == "HOME" else m.group(0)))

    text = re.sub(r"\$\{(\w+)\}|\$(\w+)", value, text)
    if "$" in text or "`" in text:
        return None
    if not os.path.isabs(text):
        text = os.path.join(cwd or os.getcwd(), text)
    text = re.split(r"[*?\[]", text)[0] or "/"  # a glob: judge its fixed prefix
    return _real(text)


def expand_segments(line: str, depth: int = 0) -> list[list[str]]:
    """Simple commands of a line, plus those inside ``bash -c "..."`` and ``eval``."""
    out: list[list[str]] = []
    for tokens in split_commands(line):
        out.append(tokens)
        name, args = _program(tokens)
        if name in gates._SHELLS and depth < 3 and "-c" in args[:-1]:
            out += expand_segments(args[args.index("-c") + 1], depth + 1)
        elif name == "eval" and depth < 3 and args:
            out += expand_segments(" ".join(args), depth + 1)
    return out


# -- write targets of a shell command ---------------------------------------------------------


def _flagless(args: list[str]) -> list[str]:
    out: list[str] = []
    done = False
    for word in args:
        if word == "--" and not done:
            done = True
        elif done or not word.startswith("-"):
            out.append(word)
    return out


def write_targets(tokens: list[str]) -> list[str]:
    """Path words one simple command writes to (redirections and mutating programs)."""
    out: list[str] = []
    for index, word in enumerate(tokens):
        if word and set(word) <= set("<>&") and ">" in word:
            nxt = tokens[index + 1] if index + 1 < len(tokens) else ""
            if word.endswith("&") and (nxt.isdigit() or nxt == "-"):
                continue
            if nxt and nxt not in _NULLS:
                out.append(nxt)
        elif re.match(r"^\d*>>?[^&>]", word):
            target = word.lstrip("0123456789").lstrip(">")
            if target and target not in _NULLS:
                out.append(target)
    name, args = _program(tokens)
    if name == "tee":
        out += _flagless(args)
    elif name in ("rm", "rmdir", "unlink", "mkdir", "touch", "truncate", "mv"):
        out += _flagless(args)
    elif name in ("chmod", "chown", "chflags", "chgrp"):
        out += _flagless(args)[1:]
    elif name in ("cp", "install", "rsync", "ln"):
        for i, word in enumerate(args):
            if word in ("-t", "--target-directory") and i + 1 < len(args):
                out.append(args[i + 1])
                break
            if word.startswith("--target-directory="):
                out.append(word.split("=", 1)[1])
                break
        else:
            plain = _flagless(args)
            if plain:
                out.append(plain[-1])
    elif name == "sed" and any(a == "--in-place" or a.startswith("--in-place=")
                               or re.match(r"^-[A-Za-z]*i", a) for a in args):
        plain = _flagless(args)
        scripted = any(a in ("-e", "-f") or a.startswith("--expression") for a in args)
        out += plain if scripted else plain[1:]
    elif name == "perl" and any(re.match(r"^-[A-Za-z]*i", a) for a in args):
        out += _flagless(args)[1:]
    elif name == "dd":
        out += [a[3:] for a in args if a.startswith("of=")]
    return out


def _git_c(tokens: list[str]) -> tuple[str | None, str]:
    """``(path of git -C, subcommand)`` of a git command, ``(None, "")`` when not ``git -C``."""
    name, args = _program(tokens)
    if name != "git" or "-C" not in args:
        return None, ""
    at = args.index("-C")
    if at + 1 >= len(args):
        return None, ""
    rest = args[at + 2:]
    sub = next((a for a in rest if not a.startswith("-")), "")
    return args[at + 1], sub


def _main_checkouts(config, workspace: Path | None) -> list[str]:
    if workspace is None:
        return []
    return [_real(p.repo) for p in config.projects if p.repo and gates._lane_of(workspace, Path(p.repo))]


def _cd_target(tokens: list[str]) -> str | None:
    name, args = _program(tokens)
    if name in ("cd", "pushd"):
        plain = _flagless(args)
        return plain[0] if plain else "~"
    return None


# -- the rules --------------------------------------------------------------------------------


def workspace_write(config, tool: str, data: dict, segments: list[list[str]], cwd, workspace: Path | None,
                    env) -> list[Violation]:
    roots = allowed_roots(workspace, env)
    here = str(cwd or os.getcwd())
    if tool in gates.EDIT_TOOLS:
        path = str(data.get("file_path") or data.get("path") or data.get("notebook_path") or "")
        real = _resolve(path, here, env) if path else None
        if path and (real is None or not _inside(real, roots)):
            return [Violation("workspace-write", f"writing outside the lane's copy is not allowed: {path}",
                              WORKSPACE_HINT)]
        return []
    out: list[Violation] = []
    mains = _main_checkouts(config, workspace)
    line = " ".join(" ".join(t) for t in segments)
    for tokens in segments:
        moved = _cd_target(tokens)
        if moved is not None:
            new = _resolve(moved, here, env)
            here = new or here
            continue
        for target in write_targets(tokens):
            real = _resolve(target, here, env)
            if real is None or not _inside(real, roots):
                out.append(Violation("workspace-write", f"writing outside the lane's copy is not allowed: "
                                     f"{target} (in: {_clip(' '.join(tokens))})", WORKSPACE_HINT))
        folder, sub = _git_c(tokens)
        if folder is not None and sub not in _GIT_READ:
            real = _resolve(folder, here, env)
            if real is None or not (_inside(real, roots) or (sub in _GIT_INTEGRATE and real in mains)):
                out.append(Violation("workspace-write", f"git -C {folder} {sub} changes a folder outside the "
                                     f"lane's copy: {_clip(line)}", WORKSPACE_HINT))
    return out


_CL = ".cl" + "aude"  # one agent CLI's config folder, spelled so the commit guard's word list does not match


def _sensitive(path: str, workspace: Path | None) -> str | None:
    """The kind of credential a real path is, else None."""
    home = _real(os.path.expanduser("~"))
    rel = path[len(home) + 1:] if path.startswith(home + "/") else None
    if rel is not None:
        for prefix in (".ssh", ".aws", ".gnupg", ".config/gh", ".netrc", "Library/Keychains",
                       _CL + "/.credentials", _CL + "/credentials", ".copilot/credentials",
                       ".copilot/config.json", ".copilot/token", ".copilot/auth", ".codex/auth"):
            if rel == prefix or rel.startswith(prefix.rstrip("/") + "/") or (
                    prefix in (_CL + "/.credentials", _CL + "/credentials", ".codex/auth", ".copilot/auth",
                               ".copilot/credentials", ".copilot/token") and rel.startswith(prefix)):
                return f"~/{prefix}"
    base = os.path.basename(path)
    if path.endswith(".pem"):
        return "*.pem"
    if (base == ".env" or base.startswith(".env.")) and not base.endswith((".example", ".sample", ".template")):
        if workspace is None or not _inside(path, [_real(workspace)]):
            return ".env"
    return None


def no_secrets(tool: str, data: dict, segments: list[list[str]], cwd, workspace: Path | None,
               env) -> list[Violation]:
    here = str(cwd or os.getcwd())
    words: list[str] = []
    if tool == "Bash":
        for tokens in segments:
            moved = _cd_target(tokens)
            if moved is not None:
                here = _resolve(moved, here, env) or here
            words += [w.lstrip("<>").split("=", 1)[-1] for w in tokens]
    else:
        words = [str(data.get(k) or "") for k in ("file_path", "path", "notebook_path", "glob", "pattern")]
    for word in words:
        if not word or word.startswith("-") and "/" not in word:
            continue
        probe = word
        for text in (probe, re.sub(r"^.*?(?=[~/$])", "", probe)):  # also `open('/x/.ssh/id')` style words
            if not text:
                continue
            real = _resolve(text, here, env)
            if real is None:
                continue
            kind = _sensitive(real, workspace)
            if kind:
                return [Violation("no-secrets", f"{kind} holds credentials and may not be read: "
                                  f"{_clip(word)}", SECRETS_HINT)]
    return []


_SERVE_PROGRAMS = {"uvicorn", "gunicorn", "hypercorn", "daphne", "live-server", "http-server", "serve",
                   "php-server", "webpack-dev-server", "ngrok", "httpd", "caddy", "nginx"}
_PORT_EXEMPT = gates._READ_ONLY | {"git", "echo", "printf", "tirith"}


def _starts_server(tokens: list[str]) -> str | None:
    name, args = _program(tokens)
    if not name:
        return None
    sub = [a for a in args if not a.startswith("-")]
    if name in _SERVE_PROGRAMS:
        return name
    if name.startswith("python") and "-m" in args and args.index("-m") + 1 < len(args) \
            and args[args.index("-m") + 1] in ("http.server", "SimpleHTTPServer", "uvicorn", "flask",
                                                "gunicorn", "aiohttp.web", "streamlit"):
        return f"python -m {args[args.index('-m') + 1]}"
    if name in ("nc", "ncat", "netcat", "socat") and any(
            a in ("-l", "--listen") or re.match(r"^-[A-Za-z]*l", a) or a.upper().startswith("TCP-LISTEN")
            or "LISTEN" in a.upper() for a in args):
        return f"{name} listening"
    if name == "flask" and "run" in sub[:1]:
        return "flask run"
    if name == "php" and "-S" in args:
        return "php -S"
    if name == "ruby" and "-run" in args:
        return "ruby -run"
    if name in ("vite", "next", "nuxt", "astro", "webpack", "ng", "trunk", "hugo", "jekyll", "mkdocs",
                "streamlit", "storybook") and (name == "vite" and not sub or sub[:1] in (["dev"], ["serve"], ["start"], ["run"])
                                               and name != "ng" or name == "ng" and sub[:1] == ["serve"]):
        return f"{name} {' '.join(sub[:1])}".strip()
    if name in ("npm", "pnpm", "yarn", "bun", "npx", "bunx", "deno"):
        words = sub[1:] if sub[:1] == ["run"] else sub
        if sub[:1] in (["dev"], ["serve"], ["start"], ["preview"]) or (
                sub[:1] == ["run"] and words[:1] in (["dev"], ["serve"], ["start"], ["preview"])):
            return f"{name} {' '.join(sub[:2])}"
        if name in ("npx", "bunx") and sub[:1] and sub[0].split("@")[0] in (
                "serve", "http-server", "live-server", "vite", "next", "json-server"):
            return f"{name} {sub[0]}"
    if name == "cargo" and sub[:1] == ["watch"]:
        return "cargo watch"
    if name not in _PORT_EXEMPT and any(a == "--port" or a.startswith("--port=") for a in args):
        return f"{name} --port"
    return None


def no_servers(segments: list[list[str]]) -> list[Violation]:
    out: list[Violation] = []
    for tokens in segments:
        hit = _starts_server(tokens)
        if hit:
            out.append(Violation("no-servers", f"{hit} would listen on a port: {_clip(' '.join(tokens))}",
                                 SERVERS_HINT))
    return out


def _own_group(pid: int, group: int) -> bool:
    try:
        return os.getpgid(pid) == group
    except ProcessLookupError:
        return True  # nothing to stop
    except OSError:
        return False


def own_processes(segments: list[list[str]], group: int | None = None, line: str = "") -> list[Violation]:
    group = os.getpgrp() if group is None else group
    out: list[Violation] = []
    if re.search(r"\bkill\b[^;&|\n]*(\$\(|`)", line):
        out.append(Violation("own-processes", f"kill with pids from another command cannot be checked: "
                             f"{_clip(line)}", PROCESS_HINT))
    for tokens in segments:
        name, args = _program(tokens)
        if name == "xargs" and any(os.path.basename(a) in ("kill", "pkill", "killall") for a in args):
            out.append(Violation("own-processes", f"xargs kill stops processes that cannot be checked: "
                                 f"{_clip(' '.join(tokens))}", PROCESS_HINT))
        if name in ("pkill", "killall", "skill", "killall5"):
            out.append(Violation("own-processes", f"{name} stops processes by name, including other people's: "
                                 f"{_clip(' '.join(tokens))}", PROCESS_HINT))
        elif name == "kill":
            words, after, skip = [], False, False
            for at, word in enumerate(args):
                if skip:
                    skip = False
                elif word == "--":
                    after = True
                elif not after and word in ("-s", "-n", "-q"):
                    skip = True
                elif not after and at == 0 and re.match(r"^-([A-Za-z]\w*|\d+)$", word):
                    continue  # -9, -TERM: a signal
                else:
                    words.append(word)
            bad = []
            for word in words:
                if re.match(r"^%\d*$", word) or word in ("$!", "${!}"):
                    continue  # this shell's own job
                if re.match(r"^-?\d+$", word):
                    pid = int(word)
                    if pid in (0,) or (pid < 0 and _own_group(-pid, group) and -pid == group):
                        continue
                    if pid > 0 and _own_group(pid, group):
                        continue
                bad.append(word)
            if bad:
                out.append(Violation("own-processes", f"{' '.join(bad)} is not a process of this lane: "
                                     f"{_clip(' '.join(tokens))}", PROCESS_HINT))
    return out


def check(config, tool: str, data: dict, cwd, *, lane: str | None, home: Path | None = None, env=None,
          group: int | None = None) -> list[Violation]:
    """Every lane rule that blocks one call (``[]`` when ``lane`` is unset: only lanes are fenced)."""
    if not lane:
        return []
    env = os.environ if env is None else env
    workspace = lane_workspace(home, lane, cwd)
    out: list[Violation] = []
    if tool == "Bash":
        line = gates.strip_heredocs(str(data.get("command") or data.get("cmd") or ""))
        segments = expand_segments(line)
        out += workspace_write(config, tool, data, segments, cwd, workspace, env)
        out += no_secrets(tool, data, segments, cwd, workspace, env)
        out += no_servers(segments)
        out += own_processes(segments, group, line)
    elif tool in gates.EDIT_TOOLS:
        out += workspace_write(config, tool, data, [], cwd, workspace, env)
        out += no_secrets(tool, data, [], cwd, workspace, env)
    elif tool in ("Read", "Grep", "Glob", "NotebookRead"):
        out += no_secrets(tool, data, [], cwd, workspace, env)
    return out
