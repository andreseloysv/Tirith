# The daemon's modules

The overview each module of `rust/tirith/src` carried in its header, kept here so the code stays short.

## `brain.rs`

The brain: small text jobs for a cheap, fast model of the Meta Model API (OpenAI-style `/chat/completions`). Never on a hot path: callers use the cached answer or a plain fallback. The key comes only from the environment
(`META_API_KEY` or `META_SI_API_KEY`) and is never logged or stored.

## `guard.rs`

The commit guard: git hooks in every project that refuse a commit whose message, file names or added lines contain one of the owner's forbidden words (whole word, any case; a word may be a regex). The words come only from
config.toml: the project's `guard_words`, else `[guard] words`; `guard = false` opts a project out.

## `lanes.rs`

Lanes: one agent, one plan item, one reusable worktree slot (`<tirith home>/lanes/<project>/slot-N`) on its own branch. The autopilot starts a project's open plan items
when the machine has room; a lane whose process ends goes to the merge queue, where the checks (never the agent) decide that its item is done.

## `manager.rs`

The managers: one long-lived agent session per project (stream-json in and out), and one for what belongs to no project. A message goes to the session of its project, which starts with its first message and rests after 30 quiet
minutes; its text, its tool calls and the end of each turn go straight to that project's chat on the page.

## `merge.rs`

The merge queue. Up to five lanes of one project ride one train: in the project's merge slot (`<tirith home>/lanes/<project>/merge`) their branches are merged with main, the project's checks and the items' own checks run, and
only a green train moves main (fast-forward), with the plan ticks in the same commits. A red train is split into single lanes; a red single lane is first compared with main, so a broken main never blames a lane. A project's
deploy stages are no checks: they run after main has moved, on that main (`deploy`).

## `metrics.rs`

Health metrics and the CEO.

Every minute code computes the metrics from the daemon's own data (lanes, runs, plans, kv). A metric that
turns red starts an episode: the mechanical fix when there is one, then one note to the manager. Still red
`[ceo] review_after_minutes` later, one CEO review runs for that episode: a fresh, independent, read-only
session whose answer goes to the owner's chat and to the manager. Back to green ends the episode.
While everything is green no model is called.

## `models.rs`

Models per role and the fallback chain.

Roles: "manager" (the chat session), "workers" (lanes), "ceo" (one-shot review), "brain" (small text jobs
through the model API: only its model matters). Each has an agent alias, a model, an effort and a fallback
chain. The effective setting is the owner's override from the page (kv `models:<role>`), else
`[models.<role>]` of `config.toml`, else a default. Agents are aliases (`[agents.<alias>]`); the real command
lives only in config.toml. Optional per alias: `model_flag`, `effort_flag` ("" = none) and `resume_args`
(`{session}` and `{message}` are filled in); the known aliases have defaults.

## `plan.rs`

The plan: the project's markdown checklist (`## N. Title` steps, `- [ ] item` lines). Its content changes (new item, ticked item) are written to the file at once and committed in batches; which lane works on an item lives in the
daemon, not in the file. The daemon is the only writer. An item may carry its own check: `- [ ] Text (check: `cmd`)`; the merge queue runs it, and ticks a merged item in the merge commit itself.

## `projects.rs`

Adding a project (the page's "+", `tirith project add`): the folder gets a git repository, the commit guard and a
plan with a first step, then a `[[projects]]` block appended to config.toml (its existing text is never rewritten).

## `rooms.rs`

agentsmessenger.com rooms: public rooms Tirith watches (`[[rooms]]` in config.toml), and private `sealed-` rooms where the owner talks to the managers from a phone, end-to-end encrypted, compatible with `am.mjs` and
the agentsmessenger.com chat page. One task per room long-polls it.

Who gives orders: only a phone paired with a one-use QR code (`tirith-pair <code>`) is the owner, in every room made
here (each project gets a room, each room gets every phone). Other agents reach the manager as information. After 30
agent messages in a row with nothing from a paired phone, Tirith stops delivering and says so once, until a phone writes.

## `rooms/sealed.rs`

agentsmessenger.com's protocol, byte-compatible with its `am.mjs` and chat page: ed25519 identities (`{name, secretKey: hex(seed||pub)}`), canonical JSON, signed events, and the sealed envelope (x25519-xsalsa20-poly1305 with the
ed25519 keys converted to curve25519, one wrapped key per reader).

## `sandbox.rs`

Network sandbox. Every process Tirith starts (lane agents, the manager, CEO reviews, merge-queue checks) runs inside a macOS `sandbox-exec` profile that denies outbound network except loopback and local sockets, with its proxy
variables pointing at the daemon's own HTTP/CONNECT proxy (`sandbox/proxy.rs`). The proxy authenticates the caller (`who` + token), applies the block list, and records each new destination as an event.

## `sandbox/proxy.rs`

The daemon's HTTP proxy for sandboxed processes: `CONNECT host:port` and plain-HTTP absolute-form requests. It authenticates the caller, refuses blocked destinations (by name, then by the addresses they resolve to), records each
new destination of each caller, and pipes the bytes.
