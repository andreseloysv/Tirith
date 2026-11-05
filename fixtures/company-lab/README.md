# company-lab fixture

Tiny shop app (`src/shop.py`, tests in `tests/`) used as a disposable test project
for `tirith lab run <scenario>`. Data only: no runner lives here.

`scenarios.json` lists 5 scenarios ordered by size (typo, small feature, login in
3 parts, 5-step unattended app, provoked failure) and 5 events (arrival mid-run,
vague goal, owner request, flaky test, restart). Each has an id, owner request,
expected outcome and a check. Run the runner on a copy, never on this directory.
