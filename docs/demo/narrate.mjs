// Narration audio. Usage: node narrate.mjs [outDir]   (default: ../../../Tirith-video/work)
// Reads narration.json, writes <outDir>/audio/<id>.wav and <outDir>/timings.json (seconds per clip).
// Uses Google Cloud Text-to-Speech when TEXT_TO_SPEACH_GOOGLE_API_KEY is set (the key is never printed),
// otherwise the macOS `say` voice named in TIRITH_VOICE (default Samantha).
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const out = resolve(process.argv[2] || join(here, '../../../Tirith-video/work'));
const audio = join(out, 'audio');
mkdirSync(audio, { recursive: true });
const key = process.env.TEXT_TO_SPEACH_GOOGLE_API_KEY;
const voice = process.env.TIRITH_VOICE_GOOGLE || 'en-US-Studio-O';
const lines = JSON.parse(readFileSync(join(here, 'narration.json'), 'utf8'));
const timings = {};

for (const l of lines) {
  const wav = join(audio, l.id + '.wav');
  if (key) {
    const r = await fetch('https://texttospeech.googleapis.com/v1/text:synthesize', {
      method: 'POST',
      headers: { 'content-type': 'application/json', 'x-goog-api-key': key },
      body: JSON.stringify({
        input: { text: l.text },
        voice: { languageCode: 'en-US', name: voice },
        audioConfig: { audioEncoding: 'LINEAR16', sampleRateHertz: 24000, speakingRate: 0.97 },
      }),
    });
    if (!r.ok) throw new Error('TTS failed for ' + l.id + ': HTTP ' + r.status + ' ' + (await r.text()).slice(0, 200));
    const j = await r.json();
    writeFileSync(wav, Buffer.from(j.audioContent, 'base64'));
  } else {
    const aiff = join(audio, l.id + '.aiff');
    execFileSync('say', ['-v', process.env.TIRITH_VOICE || 'Samantha', '-r', '165', '-o', aiff, l.text]);
    execFileSync('/opt/homebrew/bin/ffmpeg', ['-y', '-loglevel', 'error', '-i', aiff, '-ar', '24000', wav]);
  }
  const d = execFileSync('/opt/homebrew/bin/ffprobe', ['-v', 'error', '-show_entries', 'format=duration', '-of', 'csv=p=0', wav]).toString().trim();
  timings[l.id] = +(+d).toFixed(2);
  console.log(l.id, timings[l.id] + 's');
}
writeFileSync(join(out, 'timings.json'), JSON.stringify(timings, null, 2));
console.log('total', Object.values(timings).reduce((a, b) => a + b, 0).toFixed(1) + 's');
