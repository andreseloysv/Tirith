#!/usr/bin/env bash
# Builds the final mp4 from <out>/scenes.json (record.mjs), <out>/audio (narrate.mjs) and <out>/{title,close}.png (cards.mjs).
# Usage: ./assemble.sh [workDir] [finalMp4]       default workDir: ../../../Tirith-video/work
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
WORK=$(cd "${1:-$HERE/../../../Tirith-video/work}" && pwd)
FINAL=${2:-$WORK/../tirith-demo.mp4}
FF=/opt/homebrew/bin/ffmpeg; FP=/opt/homebrew/bin/ffprobe
X=0.5            # cross-fade between cards and the recording
TITLE=3.0        # opening card
CLOSE_NARR=0.6   # closing card: narration starts this long after the card appears
dur() { "$FP" -v error -show_entries format=duration -of csv=p=0 "$1"; }

read -r VIDEO START END BUILT < <(python3 - "$WORK" <<'PY'
import json, sys
d = json.load(open(sys.argv[1] + '/scenes.json'))
print(d['video'], max(0, d['scenes']['problem'] - 0.35), d['scenes']['end'], d['scenes'].get('built_card', d['scenes']['built']))
PY
)
MAIN=$(python3 -c "print(round($END - $START, 3))")
CLOSE=$(python3 -c "print(round($(dur "$WORK/audio/close.wav") + $CLOSE_NARR + 1.6, 3))")
TOTAL=$(python3 -c "print(round($TITLE + $MAIN + $CLOSE - 2*$X, 3))")
echo "recording $START..$END ($MAIN s), total $TOTAL s"

# audio: every narration clip placed at its scene time
INPUTS=(); FILT=""; N=0; LABELS=""
while read -r CLIP AT; do
  OUT_AT=$(python3 -c "print(int(round(($AT - $START + $TITLE - $X) * 1000)))")
  INPUTS+=(-i "$WORK/audio/$CLIP.wav"); FILT+="[$N:a]adelay=${OUT_AT}|${OUT_AT},apad[a$N];"; LABELS+="[a$N]"; N=$((N+1))
done < <(python3 - "$WORK" <<'PY'
import json, sys
for k, v in json.load(open(sys.argv[1] + '/scenes.json'))['narration'].items(): print(k, v)
PY
)
CLOSE_AT=$(python3 -c "print(int(round(($TITLE + $MAIN - 2*$X + $CLOSE_NARR) * 1000)))")
INPUTS+=(-i "$WORK/audio/close.wav"); FILT+="[$N:a]adelay=${CLOSE_AT}|${CLOSE_AT},apad[a$N];"; LABELS+="[a$N]"; N=$((N+1))
FILT+="${LABELS}amix=inputs=$N:normalize=0,atrim=0:$TOTAL,loudnorm=I=-16:TP=-1.5:LRA=11[aout]"
"$FF" -y -loglevel error "${INPUTS[@]}" -filter_complex "$FILT" -map "[aout]" -ar 48000 -ac 2 "$WORK/narration-mix.wav"

# video: title card -> recording -> closing card
"$FF" -y -loglevel error \
  -loop 1 -framerate 30 -t "$TITLE" -i "$WORK/title.png" \
  -ss "$START" -t "$MAIN" -i "$VIDEO" \
  -loop 1 -framerate 30 -t "$CLOSE" -i "$WORK/close.png" \
  -i "$WORK/narration-mix.wav" \
  -loop 1 -framerate 30 -t "$MAIN" -i "$WORK/built.png" \
  -filter_complex "[0:v]fps=30,format=yuv420p[t];[1:v]fps=30,scale=1920:1080,format=yuv420p[m0];[4:v]fps=30,format=yuv420p[bc];
   [m0][bc]overlay=enable='gte(t,$(python3 -c "print(round($BUILT - $START, 3))"))'[m];[2:v]fps=30,format=yuv420p[c];
   [t][m]xfade=transition=fade:duration=$X:offset=$(python3 -c "print($TITLE - $X)")[tm];
   [tm][c]xfade=transition=fade:duration=$X:offset=$(python3 -c "print($TITLE + $MAIN - 2*$X)")[v]" \
  -map "[v]" -map 3:a -t "$TOTAL" -c:v libx264 -preset slow -crf 17 -pix_fmt yuv420p -r 30 -c:a aac -b:a 192k -movflags +faststart "$FINAL"
"$FF" -y -loglevel error -i "$FINAL" -vf scale=1280:720 -c:v libx264 -preset slow -crf 20 -pix_fmt yuv420p -c:a copy -movflags +faststart "${FINAL%.mp4}-720p.mp4"
echo "wrote $FINAL ($(dur "$FINAL") s) and ${FINAL%.mp4}-720p.mp4"
