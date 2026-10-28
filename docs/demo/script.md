# Tirith demo: narration script (about 2:00 spoken, 2:55 total)

Video: `../Tirith-video/tirith-demo.mp4` (1920x1080, H.264/AAC, loudness -16 LUFS), recorded live by `record.mjs`; the
narration text below is `narration.json`, voiced by Google Cloud Text-to-Speech (en-US Studio). Spoken time is about
2:02 (the rest is the screen working: builds, tests and clicks). Pipelines and agents shown are real: the recorder starts
them through the `tirith` CLI while filming. Words used: "coding agents" only; no non-Meta AI product is named.

| Time | Scene | On screen |
|---|---|---|
| 0:00 | Title card | title.png |
| 0:03 | 1. Problem | Flow graph of the whole fleet; cursor drifts over the heroes |
| 0:17 | 2. One command | Terminal card: `./run` |
| 0:26 | 3. Live Overview | Overview in the session "Hackathon demo"; a request, an agent and activity appear live |
| 0:40 | 4. Pipelines | Tirith's own checks (collector tests, schema, web check, a live Muse Spark call), then the real ChromaSol Android build, test, lint; stage logs |
| 1:22 | 5. Agents | Kanban with the live agent and its log; then the earlier heroes via session + search |
| 1:41 | 6. Flow | Session graph, click an edge to read its messages; whole graph with rooms, click a room edge |
| 2:02 | 7. The brain | Card of the summaries Muse Spark wrote into the state: live agent, a hero, a room |
| 2:21 | 8. Search and filters | Project, session and search (`android`) |
| 2:33 | 9. Built during the hackathon | Card: what was built since 2026-10-28; which Meta models do the work |
| 2:49 | Closing card | Repo URL, track, models |

## Narration

**0:03 1. Problem**

> I run several coding agents at the same time. And all day I ask the same questions. What is everyone doing? Is something stuck? Is something waiting for me? I was keeping the whole project in my head. So, during this hackathon, I built Tirith.

**0:17 2. One command**

> One command starts it. Dot slash run. A collector watches my agents, and Tirith opens in the browser, live. No server, no account.

**0:26 3. Live Overview**

> This is the Overview, for a fresh session called Hackathon demo. I ask the Steward to check the Android build. An agent starts, a request for me appears, and the activity feed fills in. All live.

**0:40 4. Pipelines**

> Pipelines groups every run: running, failed, passed. This is Tirith's own test pipeline, running for real. Each step is a node, with its own duration. Click a step, and its log opens on the side.

> Next, a real Android build of my app, ChromaSol: build, test, lint, compiled by Gradle right now. Click a step while it runs, and the log is right there.

> When a run ends, it turns green, and the Steward gets a report.

**1:22 5. Agents**

> Agents is a board of heroes. Each agent has a name, a rank, and a live timer. Click a card for its summary and its log. And these earlier heroes are the coding agents that built Tirith.

**1:41 6. Flow**

> Flow shows how information passes. I talk to the Steward, the orchestrator. The Steward briefs the agents, they report back, and rooms on agents messenger join in. Every edge counts its messages. Click one, and you can read them all.

**2:02 7. The brain**

> The brain is Muse Spark, on the Meta Model API. It reads each agent's log, and says in one sentence what it is doing. Here, the agent recording this demo, and one of the heroes that built Tirith. It spots when something needs the owner. And it summarizes whole chat rooms. Here, agents trading hackathon ideas, in one line.

**2:21 8. Search and filters**

> Everything filters by session, and by project. A project is just any folder. And search finds heroes, features, runs, steps, and messages.

**2:33 9. Built during the hackathon**

> Everything you just saw was built during this hackathon, starting October twenty eighth: the collector, the commit guard, the web interface, flows, rooms and sessions. Meta's models power it. Muse Spark is the brain inside the product, and Tirith launches and watches Muse Code agents.

**2:49 Closing card**

> Tirith. See your coding agents work. The code is on GitHub.

## Notes

- The hackathon rule: only what was built during the hackathon is shown. The title card, scene 9 and the closing card
  say so; the whole repository starts at the first commit of 2026-10-28.
- Meta models: Muse Spark (Meta Model API) is the product's brain; Tirith launches and watches Muse Code coding agents (summaries,
  needs-you detection, room summaries, hero names). One pipeline stage makes a live Muse Spark call.
- Nothing political or war related; the API key is never shown.
