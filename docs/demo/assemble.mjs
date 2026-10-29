// Builds the final files from <work>/scenes.json (record.mjs) and <work>/audio (narrate.mjs):
//   docs/demo/out/tirith-demo.mp4   H.264 + AAC, 1920x1080, narration at -16 LUFS
//   docs/demo/out/tirith-demo.gif   10 s loop of the rule scene (an agent blocked, then the command to use), 1200 px wide
// Usage: node assemble.mjs [workDir]        default workDir: docs/demo/out/work
import { readFileSync, mkdirSync, statSync } from 'node:fs';
import { execFileSync, spawnSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const work = resolve(process.argv[2] || join(here, 'out/work'));
const outDir = join(here, 'out');
const FF = '/opt/homebrew/bin/ffmpeg';
mkdirSync(outDir, { recursive: true });
const ff = (...a) => execFileSync(FF, ['-y', '-loglevel', 'error', ...a], { stdio: 'inherit' });

const d = JSON.parse(readFileSync(join(work, 'scenes.json'), 'utf8'));
// map recorder wall marks onto the video's clock using its two hard cuts: the card -> app (scene "ask") and app -> card (scene "brain")
const log = spawnSync(FF, ['-i', d.video, '-vf', "select='gt(scene,0.08)',showinfo", '-an', '-f', 'null', '-'], { encoding: 'utf8' }).stderr;
const cuts = [...log.matchAll(/pts_time:([0-9.]+)/g)].map((m) => +m[1]);
const cutAsk = cuts.find((t) => Math.abs(t - d.scenes.cut_ask) < 4), cutBrain = [...cuts].reverse().find((t) => Math.abs(t - d.scenes.brain) < 8);
if (cutAsk === undefined || cutBrain === undefined) throw new Error('hard cuts not found in the video: ' + cuts);
const oA = d.scenes.cut_ask - cutAsk, oB = d.scenes.brain - cutBrain;
const vt = (w) => +(w - (w <= d.scenes.cut_ask ? oA : w >= d.scenes.brain ? oB : oA + (oB - oA) * (w - d.scenes.cut_ask) / (d.scenes.brain - d.scenes.cut_ask))).toFixed(2);
console.log('cuts', cuts, 'offsets', oA.toFixed(2), oB.toFixed(2));
for (const k of Object.keys(d.scenes)) d.scenes[k] = vt(d.scenes[k]);
for (const k of Object.keys(d.narration)) d.narration[k] = vt(d.narration[k]);
const start = Math.max(0, d.scenes.problem - 0.1), main = +(Math.min(d.duration, d.scenes.end + 0.3) - start).toFixed(3);
console.log(`recording ${start}..${start + main} (${main} s)`, d.scenes);

// audio: every narration clip placed at its scene time, then loudness-normalised
const clips = Object.entries(d.narration), inputs = [];
let filt = '', labels = '';
clips.forEach(([clip, at], i) => {
  const ms = Math.round((at - start) * 1000);
  inputs.push('-i', join(work, 'audio', clip + '.wav'));
  filt += `[${i}:a]adelay=${ms}|${ms},apad[a${i}];`; labels += `[a${i}]`;
});
filt += `${labels}amix=inputs=${clips.length}:normalize=0,atrim=0:${main},loudnorm=I=-16:TP=-1.5:LRA=11[aout]`;
ff(...inputs, '-filter_complex', filt, '-map', '[aout]', '-ar', '48000', '-ac', '2', join(work, 'narration-mix.wav'));

const mp4 = join(outDir, 'tirith-demo.mp4');
ff('-ss', String(start), '-t', String(main), '-i', d.video, '-i', join(work, 'narration-mix.wav'), '-map', '0:v', '-map', '1:a',
  '-vf', 'fps=30,scale=1920:1080,format=yuv420p', '-t', String(main), '-c:v', 'libx264', '-preset', 'slow', '-crf', '20',
  '-pix_fmt', 'yuv420p', '-c:a', 'aac', '-b:a', '160k', '-movflags', '+faststart', mp4);

const gif = join(outDir, 'tirith-demo.gif');
const gs = +(d.scenes.rule + (+process.env.GIF_OFFSET || 1.0)).toFixed(2);
ff('-ss', String(gs), '-t', '10', '-i', d.video,
  '-vf', 'fps=10,scale=1200:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=96:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4',
  '-loop', '0', gif);
for (const f of [mp4, gif]) console.log(f, (statSync(f).size / 1048576).toFixed(1) + ' MB');
