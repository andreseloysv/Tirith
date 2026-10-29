#!/usr/bin/env bash
# Real activity for the demo recording, driven through the tirith CLI. Usage: activity.sh <step>
#   session     start the "Hackathon demo" session
#   events      manager line, owner message, a coding-agent lane, a request for the owner
#   pipelines   two real pipelines, one after the other (Tirith checks, then ChromaSol Android build/test/lint)
#   finish      answer the request, close the lane, end the session
# Nothing here edits source code, commits, or posts to a room.
set -uo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
CHROMA=${CHROMASOL_DIR:-$HOME/DEVELOP/colorsApp-lanes/integration}
WORK=${DEMO_WORK:-$ROOT/../Tirith-video/work}
mkdir -p "$WORK"
export TIRITH_SESSION=hackathon-demo
T() { uv run -q --project "$ROOT/collector" tirith "$@"; }

case "${1:-}" in
session)
    T session start "Hackathon demo" --id hackathon-demo ;;
events)
    T manager "Checking the Android build before release"
    T flow --from owner --to manager --kind message --text "Check that the Android build is healthy before we release"
    : > "$WORK/agent.log"
    # a lane: an agent process with a name, a rank and a log; it follows the recorder's progress lines
    nohup uv run -q --project "$ROOT/collector" tirith lane run --project tirith --class herald \
        --feature "Record the Tirith demo video" --id "demo-rec-$(date +%H%M%S)" -- \
        python3 -u -c '
import sys, time
path = sys.argv[1]; pos = 0
while True:
    with open(path) as f:
        f.seek(pos); data = f.read(); pos = f.tell()
    for line in data.splitlines():
        print(line, flush=True)
        if line.startswith("== finished"): sys.exit(0)
    time.sleep(0.5)' "$WORK/agent.log" > "$WORK/lane.out" 2>&1 &
    sleep 1
    T wait add "Approve the Android release build" --project chromasol --id demo-w1
    T note "Demo session started: pipelines next" --project tirith ;;
pipelines)
    T pipeline declare tirith-checks --project tirith --stages collector-tests,schema,web-check,muse-spark-live
    run_ci() { T run --project tirith --pipeline tirith-checks --stage "$1" ${NEW:+--new-run} -- "${@:2}"; NEW=; }
    NEW=1
    cd "$ROOT/collector"
    run_ci collector-tests uv run -q pytest -q --ignore=tests/test_live.py
    run_ci schema uv run -q python -c "
import json, jsonschema
schema = json.load(open('state.schema.json')); state = json.load(open('../fixtures/state.demo.json'))
jsonschema.validate(state, schema); print('state.demo.json matches state.schema.json')"
    run_ci web-check bash -c 'for f in ../web/js/*.js; do node --check "$f" && echo "ok $f"; done'
    run_ci muse-spark-live uv run -q pytest -q tests/test_live.py
    T flow --from manager --to owner --kind decision --text "Tirith checks are green; moving to the Android build"
    T pipeline declare android-release --project chromasol --stages build,test,lint
    cd "$HOME/DEVELOP/colorsApp"   # the ChromaSol project folder; the Makefile lives in the integration worktree
    # --rerun-tasks makes Gradle really rebuild (it only touches build outputs); the make targets are the project's own
    GW="cd $CHROMA/android && JAVA_HOME=/opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home ANDROID_HOME=$HOME/Library/Android/sdk ./gradlew --console=plain --rerun-tasks"
    T run --project chromasol --pipeline android-release --stage build --new-run -- make -C "$CHROMA" GRADLEW="$GW" android-build
    T run --project chromasol --pipeline android-release --stage test -- make -C "$CHROMA" GRADLEW="$GW" android-test
    T run --project chromasol --pipeline android-release --stage lint -- make -C "$CHROMA" android-lint ;;
finish)
    T wait done demo-w1
    T manager "Android build approved; idle"
    echo "== finished: the demo is recorded" >> "$WORK/agent.log"
    sleep 3
    T session end hackathon-demo ;;
*) echo "usage: $0 session|events|pipelines|finish"; exit 2 ;;
esac
