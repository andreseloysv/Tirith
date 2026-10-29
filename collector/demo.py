#!/usr/bin/env python3
"""Fill a throwaway TIRITH_HOME with live demo data for the web page.

Usage:  python3 demo.py [home-dir] [repo-dir]

Creates (or reuses) a tiny demo git project, installs its commit guard,
registers finished and running lanes, runs a small pipeline, adds notes and
waits, maps the architecture, and collects once. A background ticker lane
keeps writing log lines so `tirith collect` shows live movement; stop it
with the printed pid. Everything happens in temp/demo folders only.
"""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from pathlib import Path

COLLECTOR_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(COLLECTOR_DIR))

TICKER = (
    "import time\n"
    "for i in range(600):\n"
    "    print(f'tick {i}: polishing the gatehouse stones', flush=True)\n"
    "    time.sleep(2)\n"
)


def run_cli(env: dict, *argv: str, check: bool = True) -> subprocess.CompletedProcess:
    cmd = [sys.executable, "-m", "tirith", *argv]
    print("+", " ".join(argv))
    return subprocess.run(cmd, env=env, cwd=str(COLLECTOR_DIR), check=check,
                          capture_output=False, text=True)


def main() -> int:
    home = Path(sys.argv[1]).expanduser().resolve() if len(sys.argv) > 1 else \
        Path(tempfile.mkdtemp(prefix="tirith-demo-home-"))
    repo = Path(sys.argv[2]).expanduser().resolve() if len(sys.argv) > 2 else \
        home.parent / f"{home.name}-repo"
    home.mkdir(parents=True, exist_ok=True)
    repo.mkdir(parents=True, exist_ok=True)
    env = {**os.environ, "TIRITH_HOME": str(home)}

    if not (repo / ".git").exists():
        git = "/usr/bin/git" if Path("/usr/bin/git").exists() else "git"
        subprocess.run([git, "init", "-q", str(repo)], check=True)
        (repo / "src").mkdir(exist_ok=True)
        (repo / "src" / "main.py").write_text("print('demo')\n", encoding="utf-8")
        (repo / "README.md").write_text("# Demo project\n", encoding="utf-8")
        subprocess.run([git, "-C", str(repo), "add", "."], check=True)
        subprocess.run([git, "-C", str(repo), "-c", "user.email=d@example.invalid",
                        "-c", "user.name=Demo", "commit", "-q", "-m", "initial"], check=True)

    (home / "config.toml").write_text(
        "[[projects]]\n"
        'id = "demo"\nname = "Demo Hold"\n'
        f'repo = "{repo}"\n'
        'color = "#2f7d5b"\n', encoding="utf-8")

    run_cli(env, "guard", "install", "demo", check=False)
    run_cli(env, "manager", "Watching the demo lanes from the tower")
    run_cli(env, "note", "Demo seeded for the citadel", "--project", "demo")
    run_cli(env, "wait", "add", "Look at the demo citadel and press on", "--project", "demo")
    run_cli(env, "map", "demo", check=False)
    run_cli(env, "pipeline", "declare", "demo-release", "--project", "demo",
            "--stages", "build,test,ship")
    run_cli(env, "run", "--project", "demo", "--pipeline", "demo-release",
            "--stage", "build", "--new-run", "--",
            sys.executable, "-c", "print('BUILD OK')")
    run_cli(env, "run", "--project", "demo", "--pipeline", "demo-release",
            "--stage", "test", "--", sys.executable, "-c", "print('12 tests passed')")

    # One finished lane (renown + report), via lane run so the guard gate is real.
    result = run_cli(env, "lane", "run", "--project", "demo", "--class", "stonewright",
                     "--feature", "Lay the foundation stones", "--id", "demo-foundation",
                     "--", sys.executable, "-c",
                     "print('foundation laid'); print('20 stones set')", check=False)
    if result.returncode != 0:
        print("guarded lane run skipped (guard not fully intact here); "
              "registering the lane manually instead")
        run_cli(env, "lane", "start", "--project", "demo", "--class", "stonewright",
                "--feature", "Lay the foundation stones", "--id", "demo-foundation",
                "--workspace", str(repo), check=False)
        run_cli(env, "lane", "end", "demo-foundation", "--result", "done",
                "--report", "Foundation laid; 20 stones set.", check=False)

    # One live lane: a ticker process registered with its pid and log.
    ticker_log = home / "logs" / "demo-ticker.log"
    ticker_log.parent.mkdir(parents=True, exist_ok=True)
    ticker_script = home / "ticker.py"
    ticker_script.write_text(TICKER, encoding="utf-8")
    with ticker_log.open("ab") as handle:
        ticker = subprocess.Popen([sys.executable, str(ticker_script)],
                                  stdout=handle, stderr=subprocess.STDOUT,
                                  start_new_session=True)
    run_cli(env, "lane", "start", "--project", "demo", "--class", "swiftrider",
            "--feature", "Polish the gatehouse stones", "--id", "demo-ticker",
            "--workspace", str(repo), "--log", str(ticker_log),
            "--pid", str(ticker.pid), check=False)

    run_cli(env, "collect", "--once")
    run_cli(env, "status")
    print()
    print(f"TIRITH_HOME={home}")
    print(f"state: {home / 'state.json'}")
    print(f"ticker pid: {ticker.pid} (kill it when done)")
    print(f"watch live: TIRITH_HOME={home} tirith collect")
    return 0


if __name__ == "__main__":
    sys.exit(main())
