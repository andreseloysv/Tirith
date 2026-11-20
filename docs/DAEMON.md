# The daemon's modules

The overview each module of `rust/tirith/src` carried in its header, kept here so the code stays short.

## `bots.rs`

Bots: the owner's own agents, factory roles with `scope = "own"` in `~/.tirith/config.toml` (`[roles.bot-<id>]`, extending the manager). Each has a name, a face (shape and
colour) and its own conversation (`bot-<id>`), run like a project's manager with the manager's model and its own room on the phones. Routines are `[[routes]]` on `schedule`
in that file (their last runs stay in kv as `routine:<id>:<routine>` stamps, not configuration): tasks a bot does by itself at a time of chosen weekdays (the machine's local
time); the lanes scheduler's tick hands a due one to the bot once a day, and the bot makes them itself with `tirith routine add|remove|list`. The page writes config.toml
with `toml_edit`, keeping its comments, and the bots that used to live in kv move there once, at daemon start. A bot learns in its folder (`<tirith home>/manager/bot-<id>`): `memory.md`, which Tirith hands it as it is now at
the start of every turn and routine, and skills in `.agents/skills/<name>/SKILL.md`, its own and the shared ones of `<tirith home>/bots/skills` (linked in before every turn).
The bots' agent finds them natively: Muse reads `.agents/skills`, and the agent's own `.<command>/skills` is a link to the same folder. Agents may write in a bot's folder
(never in its hidden folders, where agent settings live, except skills folders) and in the shared skills; the page's Library shows, corrects and deletes all of it.
A bot may have a model of its own (role `bot-<id>` of `models.rs`, chosen in its card); without one it has the managers'. A routine answer that starts with ∅ (nothing new)
stays on the page and never reaches the phones (`rooms::to_phones`), so only news does.

## `factory.rs`

The factory file: `factory/factory.toml` and its prompts (`factory/prompts/*.md`) hold what used to be fixed in
Rust — each agent's flags, the roles with their settings, `[limits]`, `[plan]`, `[cards]` and `[metrics.*]` — read
as data (docs/FACTORY.md). The loader is strict: an unknown key (`deny_unknown_fields` for `[limits]`, key lists
for the rest), a role on a missing agent or prompt, or an unknown measure makes a version invalid, and the daemon
keeps the last good one while the error shows on the page (`factory_error` of `/api/state`) and is told to the
manager of no project. It loads the repository's `factory/` at run time (`factory` in config.toml points
elsewhere) and reloads when a file changes or config.toml does, with the embedded copy as the fallback.
`~/.tirith/config.toml` is laid over it key by key and wins; its yesterday's sections are read in memory under
the factory's names (`config.rs`), the file itself untouched except by the page's `toml_edit` writes (the bots'
roles and their schedule routes), and keys nothing reads are an error the owner sees. A role `extends` another (the lead and the reviewer extend
the manager), and an agent the factory does not name runs with the `default` agent's flags. The `[[routes]]` of
the factory say who hears each event the code emits (a closed list in `factory.rs`): which role, as a `turn` now
or a `note` with its next turn, a line in the chat (`chat`), and whether that line and the answer of a turn the
event starts reach the phones (`phone`). A route on a missing event or role is invalid; routes in config.toml
stand next to the factory's.

Every version of the factory has a stamp (`Factory::version`): the main commit that last changed `factory/`,
plus a short hash of what config.toml lays over it (`<commit>+<hash>`; `embedded` when no checkout is at hand).
Every lane, chat (turn) row, owner card, event, review and score is born carrying it in its `factory` column,
with `role_hash` next to it: a short hash of the setting of the role it ran with — its resolved table, its
agent's table, its prompt texts and the page's model override for it (`Factory::role_hash`, `App::stamp_for`).
A lane's events and cards carry the lane's stamp; a line no role wrote (the owner's, a room's, Tirith's own)
keeps the version with an empty hash. Comparing two versions, two models or two prompts is then one query
(`GROUP BY factory, role_hash` over `lanes`, `chat`, `owner_items`, `events` or `scores`), with no extra runs.

## `guard.rs`

The commit guard: git hooks in every project that refuse a commit whose message, file names or added lines contain one of the owner's forbidden words (whole word, any case; a word may be a regex). The words come only from
config.toml: the project's `guard_words`, else `[guard] words`; `guard = false` opts a project out.

## `lanes.rs`

Lanes: one agent, one plan item, one reusable worktree slot (`<tirith home>/lanes/<project>/slot-N`) on its own branch. The autopilot starts a project's open plan items
when the machine has room; a lane whose process ends goes to the merge queue, where the checks (never the agent) decide that its item is done.

## `manager.rs`

The managers: one long-lived agent session per project (stream-json in and out), and one for what belongs to no project. A message goes to the session of its project, which starts with its first message and rests after the manager role's quiet
`idle_minutes`; its text, its tool calls and the end of each turn go straight to that project's chat on the page.
Everything else the daemon has to say is an event: `emit_event` keeps its row, shows it on the page and passes it
to the factory's routes. A turn's answer reaches the phones only when the owner started that turn or an event
that started it has `phone = true` in its route.

## `merge.rs`

The merge queue. Up to five lanes of one project ride one train: in the project's merge slot (`<tirith home>/lanes/<project>/merge`) their branches are merged with main, the project's checks and the items' own checks run, and
only a green train moves main (fast-forward), with the plan ticks in the same commits. A red train is split into single lanes; a red single lane is first compared with main, so a broken main never blames a lane. A project's
deploy stages are no checks: they run after main has moved, on that main (`deploy`).

## `metrics.rs`

Health metrics and the reviewer.

Every minute code computes the metrics from the daemon's own data (lanes, runs, plans, kv). A metric that
turns red starts an episode: the mechanical fix when there is one, then one note to the manager. Still red
after the delayed `metric.red` route's `after_minutes`, one review runs for that episode: that route's turn
role (the reviewer, `access = "read"`) runs one fresh turn like any turn role, and its answer goes to the
owner's chat and, as `review.done`, to the lead. Back to green ends the episode.
While everything is green no model is called.

## `models.rs`

Models per role and the fallback chain.

Roles are the factory's: "manager" (a project's chat), "lead" (the chat of no project), "worker" (lanes) and
"reviewer" (one fresh turn), each with its `session`, `scope`, `extends`, `access`, `idle_minutes`,
`max_per_day` and `runs_on` in `factory/factory.toml`; code finds a role by that shape, never by a fixed
list of names. Each has an agent alias, a model, an effort and a fallback
chain. The effective setting is the owner's override from the page (kv `models:<role>`), else
`[roles.<role>]` of `config.toml` laid over the factory's role. Agents are aliases
(`[agents.<alias>]`); the real command lives only in config.toml. Optional per alias: `model_flag`, `effort_flag` ("" = none) and `resume_args`
(`{session}` and `{message}` are filled in); each alias's flags come from the factory.

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

Network sandbox. Every process Tirith starts (lane agents, the manager, turn-role reviews, merge-queue checks) runs inside a macOS `sandbox-exec` profile that denies outbound network except loopback and local sockets, with its proxy
variables pointing at the daemon's own HTTP/CONNECT proxy (`sandbox/proxy.rs`). The proxy authenticates the caller (`who` + token), applies the block list, and records each new destination as an event.

## `sandbox/proxy.rs`

The daemon's HTTP proxy for sandboxed processes: `CONNECT host:port` and plain-HTTP absolute-form requests. It authenticates the caller, refuses blocked destinations (by name, then by the addresses they resolve to), records each
new destination of each caller, and pipes the bytes.
