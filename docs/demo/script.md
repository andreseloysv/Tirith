# Tirith demo: script (2:32, limit 3:00)

For the hackathon judges. Video: `out/tirith-demo.mp4` (1920x1080, H.264/AAC), loop for the README: `out/tirith-demo.gif`.
Everything on screen is the real page (`web/index.html`) on the demo story in `story.mjs`; the recorder drives it
(`record.mjs`). The voice is `narration.json`, spoken with Google Cloud Text-to-Speech when `TEXT_TO_SPEACH_GOOGLE_API_KEY` is set, else the macOS `say` voice (offline); each scene also
carries an on-screen caption, so the video reads with the sound off. Words used: "coding agents" only.

| Time | Scene | On screen |
|---|---|---|
| 0:00 | 1. Problem (card) | The questions asked all day, then "an autonomous manager for your coding agents" |
| 0:12 | 2. One request | Overview of ChromaSol. The owner types one request in the chat; the manager works live in CLI style: reads the plan, adds plan items, writes a brief, starts two agents |
| 0:37 | 3. The plan | Two new steps with progress bars and the agent working on each item |
| 0:50 | 4. A rule in code | Wren's drawer in Agents follows its live steps: the raw test run is blocked, the hook says what to run instead ("Rules broken"), Wren uses `tirith run`; then the run shows in Pipelines. Each caption sits on the screen that shows it |
| 1:23 | 5. Claim check, auto-merge | The report is checked against the real change; "Merged ... Claims verified"; step 4 ticks itself |
| 1:43 | 6. Health dot | Amber (the collector reloads), the panel lists the checks, then green again |
| 1:55 | 7. A real decision | Only "ship in v1.2 or hold for 2.0?" reaches the owner: one click on a button |
| 2:09 | 8. The brain (card) | Muse Spark (Meta Model API): one-sentence summary, lasting name, owner check |
| 2:24 | Closing card | `github.com/andreseloysv/Tirith`, "If it saves you a question, star it" |

## Narration

**1. Problem**
> I run several coding agents at once. All day I ask: what is everyone doing, is something stuck, is something waiting for me? So I built Tirith, a manager that runs the plan.

**2. One request**
> I write one request in the chat: add a dark mode, and make the scan faster. The manager reads the plan, adds the items, writes a brief for each, and starts the agents. I watch it work, live, like in a terminal.

**3. The plan**
> The plan now has two new steps, each with a progress bar and the agent working on it. No plan item, no agent: Tirith refuses to start one without it.

**4. A rule in code**
> Now a rule. Wren tries to run the tests directly. A hook blocks it before it runs, and tells it the exact command to use. Wren switches to it, and the run shows up in Pipelines. Rules are code that blocks, not text the agent may ignore.

**5. Claim check, auto-merge**
> When Wren reports done, Tirith checks the claims against the real change: the files, the tests, the recorded test run. They match, so it merges on its own, and the plan ticks itself. Merging is never a question for me.

**6. Health dot**
> The dot in the header watches Tirith itself. New code landed, the collector reloaded: amber, then green again, with nobody touching it.

**7. A real decision**
> And only a real decision reaches me. Ship dark mode in this release, or hold it? One click, and the manager does the rest.

**8. The brain**
> Behind it, Muse Spark, from the Meta Model API, summarizes each agent's log in one sentence, spots what needs the owner, and gives every agent a lasting name. Without a key, everything still works.

**Closing**
> Tirith. Built during the hackathon, and on GitHub. If it saves you a question, star it.
