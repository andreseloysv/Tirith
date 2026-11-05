# Contract: shared tasks and local worker inboxes

Code: `collector/tirith/coordination.py`, `roadmap.py`, `plangate.py`, `autopilot.py`.
Tests: `collector/tests/test_coordination.py`. No server, port, or model call.

## Identity and plan metadata

`tirith plan add` assigns a permanent random `i-…` ID and prints it. A compact
`<!-- tirith:{"id":"i-…","depends_on":[],"writes":[]} -->` comment stores that
identity and coordination declarations beside the checklist item. The parser
keeps the comment out of visible task text. Status updates, checks, and text
edits preserve it. Editing a legacy item freezes its old text-derived ID;
unchanged legacy plans keep working without a migration or mass plan commit.
Repeated legacy titles are scoped by step until configured or started; starting
a task persists its runtime ID in the existing plan-start update.

Only Tirith's existing plan writer changes this metadata. For example:

```sh
tirith plan add --project demo --step 1 --text "Define the API" --writes src/api
# Use the printed ID as the prerequisite:
tirith plan add --project demo --step 1 --text "Build the UI" --depends-on i-11111111 --writes web
tirith plan configure --project demo --step 1 --item i-22222222 --writes web docs/ui
tirith task list --project demo
```

The IDs above are placeholders. `configure --depends-on` or `--writes` with no
values clears that declaration. Configure a task before it is claimed.
Dependencies must name existing, unambiguous IDs and form an acyclic graph.
Deleting a prerequisite does not make a dependent ready: missing IDs fail closed.
Prerequisites become satisfied when their plan checkbox is ticked by the existing
integration/verification path, never merely from an agent's `done` report.

## Ownership and scheduling

`<TIRITH_HOME>/coordination.sqlite3` is runtime coordination state. SQLite
transactions serialize claims across processes, including on Windows. A claim
reads the current plan, validates prerequisites, assigns an owner and random
attempt token, and reserves declared write paths in one transaction. Directory
reservations conflict with their descendants. Write declarations describe
coordination scope; the existing workspace and tool gates still enforce writes.
Legacy items without declarations make no additional path reservation.

The CLI reserves work before making a worktree or starting a detached launcher.
The child adopts that same reservation. A failed worktree/launcher/process start
releases it. Claiming and running are separate phases. Successful reports enter
`verifying`; integration remains under the existing merge queue and claim checks.
Standing manager/chat sessions keep their existing shared standing-item rules.

The autopilot considers individual items. A lane or review on a known item does
not hide unrelated ready items in its step. Explicit prerequisites, reservations,
and the merge queue still block the affected item. An old lane record with no
item remains a conservative reservation of its whole step. Autopilot continues
to propose work to the manager; it does not launch agents itself.

## Leases and failures

Claims have a 600-second renewable lease. Agent hooks, `tirith task heartbeat`,
and the collector renew or inspect it. Expiration alone never displaces a live
worker or its controller (including a controller waiting for a test stage).
The collector releases expired claimed/running attempts only when their
worker is gone, recording an unfinished lane as failed without deleting its work.
Verification reservations are retained while the integration queue owns them;
an expired verification with neither a live worker nor a queue entry recovers too. Restarting the
collector preserves claims and messages.

Every coordinated report and hook checks its attempt token. An older attempt
cannot report or release a replacement's claim. `TIRITH_TASK_ATTEMPT` also fences
reused lane IDs. Recovery leaves the branch and workspace intact.

## Durable local messages

```sh
tirith task send --project demo --sender lane-a --to lane-b --key api-v1 "API is ready"
tirith task inbox --project demo --lane lane-b
tirith task ack --project demo --lane lane-b 1 2
tirith task heartbeat --project demo --lane lane-b
```

Sender and recipient must belong to the project. A worker can act only as its
own lane. Delivery keys make retries idempotent; reusing a key with different
content is refused. Acknowledgements name only IDs in the delivered snapshot;
later arrivals stay pending. Acknowledged rows are omitted from inbox reads.
The most recent 1,000 retain delivery-key deduplication; older acknowledged
entries are pruned. Inboxes hold at most 1,000 pending messages, each limited
to 16,000 characters.
Only a successful database transaction counts as delivery; disk/lock failures
are surfaced rather than reported as sent. CLI sends also appear in Flows.

Anthropic workers receive pending messages through their existing session/prompt/
post-tool hooks, without acknowledging them automatically. Every agent alias can
use the CLI inbox protocol described in its brief. New workers still start only
through Tirith's plan-linked lane launcher.

## Planner: no two lanes on the same files

The autopilot (`autopilot.candidates`) never proposes an item whose `writes` overlap those of a lane that is
working or being checked/merged, a claimed task, or an item already proposed in the same pass (the first in
plan order goes; the other comes back once it lands). `coordination.claim` refuses the overlap as a last guard.
