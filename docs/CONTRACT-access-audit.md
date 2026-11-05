# Contract: agent access audit (v1)

The owner sees what every agent (`anthropic`, `muse`, …) touched, who started whom, and every pipeline and git
hook across all projects, without relying on agents to report. Agents are named by alias only (see
`docs/CONTRACT-hooks.md`, "Agent aliases").

**The audit only records. It never blocks, never asks the owner, never delays a call.** Blocking stays in
`docs/CONTRACT-gates.md`; a gate decision is itself one more audited fact (`decision`), not a part of the audit.
Idea: «Agentenzugriffe» (neoid).

## Sources (nothing depends on the agent's goodwill)

1. Hook events (`PreToolUse` / `PostToolUse` / `SubagentStop`), see `docs/CONTRACT-hooks.md`: tool name and input.
2. Git hooks and pipeline stages run by Tirith itself (`tirith run`, merge queue, installed git hooks): each
   writes its own row.
3. The collector's process/log reading for lanes, to fill `parent` and to catch what no hook reported.

If a source fails, the audit drops that row and the call goes on (append only, no network, every exception
caught, exit 0).

## What counts as an access

| `category` | Counts when | `target` (what is recorded) |
|---|---|---|
| `credential` | Reading a secret store or key file (`~/.ssh/*`, `.env*`, `~/.aws`, `~/.config/gh`, keychain/`security`, token files, `~/.tirith/config.toml` agent settings), or a command printing one | the path or store name, never its content |
| `project_env` | Reading `<home>/env/<project>.toml` (`tirith project env …`) or a command that dumps the environment (`env`, `printenv`, `echo $NAME` of a project variable) | project id and variable NAMES, never values |
| `git_push` | `git push`, `git remote` changes, `git config` of credentials/remotes, writes to `.git/hooks` | remote + refs, or the changed config key |
| `gh` | any `gh …` command (PRs, issues, API, auth, workflow runs) | the subcommand line without tokens |
| `api` | Network calls from a tool: `curl`, `wget`, `http(ie)`, `fetch`/web tools, SDK calls to a host, `ssh`/`scp` | scheme + host + path (query string removed) |
| `browser` | Browser or computer-use tools, `open <url>`, headless browser launches | URL host + path, or the tool name |
| `agent_spawn` | A sub-agent / lane / `tirith lane run` start | child alias + lane/session; fills `parent` |
| `git_hook` | A git hook fires (pre-commit, pre-push, …) | hook name, repo, exit code |
| `pipeline` | A `tirith run` stage starts or ends | project, stage, exit code |

A call may match several categories; one row per category. Reads and writes inside the lane's own worktree
that are not listed above are NOT accesses (they are already in `files`, `docs/CONTRACT-activity.md`).

## Record format

File `<home>/audit/access.jsonl` (`TIRITH_HOME` wins), one JSON object per line, UTF-8, append only, mode `0600`
(directory `0700`). One write per row, below the pipe-atomic size, so concurrent writers never interleave;
a line over 4 KB has `target` cut and `truncated: true`.

```json
{"kind": "access", "v": 1, "ts": "2026-11-04T10:00:00Z", "id": "a-3f9c…",
 "agent": "anthropic", "lane": "w1-data", "session": "s-1", "agent_session_id": "…",
 "parent": {"agent": "muse", "lane": null, "session": "s-0"},
 "project": "chromasol", "category": "git_push", "action": "git push origin lane/x",
 "target": "origin lane/x", "tool": "Bash", "decision": "allowed", "outcome": "ok", "cwd": "/abs/dir"}
```

- `agent`: alias. `lane`, `session`, `project`: as in hook rows, or `null`. `parent`: who created this agent
  (alias + lane + session), `null` for an agent started by the owner.
- `decision`: `allowed`, `blocked` (a gate said no, with `rule`), or `unknown`. `outcome`: `ok`, `error`
  or `null` (not known yet; the `PostToolUse` row is not a second row, it is not written).
- **Redaction (always, before writing):** never write a secret value. Strip tokens, passwords, `Authorization`
  headers, `KEY=value` values and URL userinfo/query strings; a stripped part becomes `***`. A row that cannot
  be redacted is written with `action` = the category only.
- Rows are never edited or deleted by an agent. Pipeline code ignores `kind: "access"` rows in `events.jsonl`
  and the audit file alike.

## What reached outside the machine

The owner's first question is what an agent reached outside the machine, so every row that did carries
`external`: a list of `"<kind> <destination>"`.

- From the call (`access_audit.external`): `web <host>`, `api <host>` (an `api.` host or a path like `/api/`,
  `/v1/`, `/graphql`), `websocket <host>` (`ws://`, `wss://`), `ftp <host>`, `ssh <host>` (ssh, scp, rsync, sftp),
  `git <host>` or `git remote` (push, pull, fetch, clone, ls-remote), `github api.github.com` (`gh`),
  `package <registry>` (npm, pnpm, yarn, pip, uv, cargo, brew, gem, go installs), `search <query>` (WebSearch),
  `mcp <server>`. Loopback is not external. A URL's user and password are never kept.
- From what the processes really opened (`tirith/netwatch.py`): a command shows only what it says, not what a
  script does inside (a Python file calling an API, a Node process opening a websocket). Every 15 s the
  collector lists the established TCP connections (`lsof -nP -iTCP -sTCP:ESTABLISHED`), keeps those of a
  process under a working lane (by parent chain up to the lane's `pid` or `controller_pid`), and writes each
  new `(lane, host, port, program)` once (`<home>/netwatch.json`) as a row with `tool: "connection"`,
  `system: "network"`, `action: "<program> connected to <host>:<port>"` and `external: ["<https|http|ssh|git|tcp>
  <host>:<port>"]`. The host is the **domain visited** when one is known: a domain the lane's own calls named
  (`external` of its rows) that resolves to the connection's address, else the base domain of the reverse name
  (`lb-140-82-112-3-iad.github.com` gives `github.com`), else the address. The row also keeps `domain`, `address`
  and `reverse`; the page shows the domain big and the address small beside it. Lookups have a 1 s budget.
  An address with no reverse name is matched against `netwatch.KNOWN_DOMAINS` (model APIs, GitHub, package
  registries, agentsmessenger…), also for rows written before (`netwatch.annotate`, when the state is built).
  Still unknown, the server's TLS certificate names it (`netwatch.read_cert`: an `openssl s_client` handshake
  without a name, its first DNS name or subject, and its organization as `org`), once per address in
  `<home>/netwatch-certs.json`, looked up in the background. A connection of the lane's own process (the agent
  program itself, not a command it ran) has `runtime: true`;
  the page sums those into one "Agent runtime" line per destination and lists what the work opened one by one.
  The page's system and date filters apply to External too.

The «Accesos» page starts with **External**: every such destination, newest first, with its kind (Web, API,
WebSocket, SSH, Git remote, Packages, Search, MCP, HTTPS…), the agent (role and name) and what caused it.

Only the long-running collector watches the network, looks domains up and starts the proxy: a cycle run
inside a test once listed the machine's real connections (a test lane with pid 1 is every process's ancestor)
and ran `openssl` lookups from inside a lane. Certificates are read on port 443 only; plain HTTP has none, and
its domain (the Host header) is seen only by the proxy.

## Blocking a destination

The owner blocks a destination with **Block** on a line of External (or `tirith net block <domain|ip|cidr>`;
`tirith net unblock`, `tirith net list`). The page sends a `setup` request with action `net-block` or
`net-unblock` and match, applied by code in the next dispatcher pass (`tirith/netblock.py`, `<home>/netblock.json`,
`state.netblock`). A rule `example.com` blocks it and its subdomains (`*.example.com` is the same); an address or
a CIDR range blocks addresses. Two guards enforce it:

- **Before a call** (rule `net-blocked`, `gates._net_blocked`): a tool call whose destination (`external`) is
  blocked is refused, for every agent, the owner's own sessions included.
- **After a connection** (`netwatch`): with rules set the watch looks every 3 s; a lane's process that opened a
  connection to a blocked destination (by address, reverse name or certificate domain) gets SIGTERM, and the
  row says `stopped <program>: <dest> is blocked`. The agent program itself (the lane's own process) is never
  stopped: that would stop the agent; its row says so. Cutting a single connection of a process needs a
  firewall with root rights, which Tirith does not use.

## The network guard (lanes in a sandbox, through Tirith's proxy)

`tirith/netproxy.py`, on by default on macOS (`[network] sandbox = false` turns it off; `proxy_port`, default
47811). A lane's agent (every `tirith lane run` that is not interactive) runs under `sandbox-exec` with a profile
that denies every outbound connection but `localhost` and local sockets, so nothing in its process tree (the
agent program, a test, a script) can reach an outside address directly: it gets "Operation not permitted". Its
environment points `HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY` (and the lowercase ones) at Tirith's proxy with the
lane id as user (`http://<lane>:<token>@127.0.0.1:<port>`), `NO_PROXY` at loopback, and `NODE_USE_ENV_PROXY=1`.

The proxy (`tirith net serve`, one per machine, lock `<home>/netproxy.lock`, `<home>/netproxy.json`) is started
by the collector, never from inside a lane (it could not reach anything there). It answers `CONNECT` and plain
HTTP, checks the `netblock` rules on the domain and then on the addresses it resolves to (when a rule is an
address or range), answers `403` with `X-Tirith-Blocked` for a blocked one, else connects and pipes. Each new
`(lane, host, port, program)` and every refusal is an access row with `tool: "proxy"`, the exact domain, and
`runtime: true` when the client is the lane's agent program (by the pid on the other end of the loopback
connection). A lane started from inside another lane is confined already (a sandbox only narrows) and is not
wrapped twice. When the proxy is not running a lane starts without the guard, `lane.network` says why, and
`state.network` shows it on the Accesos page.

Checked on 2026-11-05 with a real agent lane: its model API and connectors went through the proxy by name, a
command's `curl` reached PyPI through it, a direct socket to an address got `EPERM`, and with `datadoghq.com`
blocked the agent's telemetry was refused while the lane still finished in 22 s.

## Retention

- Rotation at 16 MB: `access.jsonl` → `access.jsonl.1` → … → `.3`; the oldest is dropped.
- Rows older than 90 days are dropped at rotation. The owner can set `[audit] retention_days` (7–365) in
  `config.toml`.
- Rotation is done by the collector, never by a hook call.

## Reading it

`state.json` gets `agents[].accesses` (the last 50 rows of that lane, newest last) and `loose_accesses`
(lane unknown), same shape as above. The page shows them per agent and as a tree by `parent`. Counts per
category per agent are derived, not stored.

`state.accesses` also carries the newest 600 rows of `<home>/access/<project>.ndjson` (`ts`, `agent`, `lane`, `plan`,
`system`, `tool`, `action`, plus `project`), oldest first. The page's «Accesos» view (`web/js/v-accesos.js`) lists them
per agent, filtered by system and date, linked to the agent and its plan point. It is look-up only: no alert and no
counter, and nothing from it goes into Your turn. The audit adds no server and no open port.

## Out of scope

Blocking, asking the owner, alerting, and content of secrets. Those belong to `docs/CONTRACT-gates.md`.
