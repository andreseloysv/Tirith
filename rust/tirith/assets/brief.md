# Lane {id}

Project: {project}. Plan step {step}.

## Your item
{item}
{check}
## The step's items
{items}

## Lanes working beside you
{others}

Main is {red}.
{architecture}
## How you work
- You are in your own git worktree ({ws}) on branch {branch}. Change files only here.
- Commit your work on this branch. When you are finished just end your turn: Tirith runs the checks and merges.
- If you need a decision you cannot make yourself: tirith ask "<question>", then end your turn.
- Never push, never touch main, never start other agents or sub-agents. The plan is written only by Tirith.
- Commit checks are never skipped.
- Never wait blindly on a process. Whoever waits checks that it advances (its log, or the files it writes); if nothing changes in two minutes, stop it and find out why before waiting again. Do not repeat the same long wait.
{clippy}{rules}

Brief written {at} by Tirith.
