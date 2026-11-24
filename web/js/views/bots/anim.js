// The Animación tab: the bot's face on a small stage with everything it can do — its 63 states, its 14
// one-shot effects, a timed sequence, and a dialogue it performs with a local sampled voice (the text is
// directed first: expression states, turns and effects are laid over its punctuation, never changing a word;
// "Otra toma" directs it anew). Nothing here is saved: it is where the owner sees and tries the animations.
import { html, useState, useRef, useEffect } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { Face, loadFace } from './face.js';
import { keyOf } from './state.js';
import { lookOf } from './face/face-map.js';
import { STATE_CATALOG, MORPH_CATALOG } from '../../../vendor/botface/catalog.js';
import { planDialogueActions } from '../../../vendor/botface/dialogue-auto-director.js';

const DEMO = [
  { state: 'idle', hold: 800, morph: 'gather', morphHold: 600 },
  { state: 'thinking', hold: 1800, morph: 'send', morphHold: 600 },
  { state: 'celebrate', hold: 1600 },
];

export function AnimLab({ bot }) {
  const [state, setState] = useState('idle');
  const [voice, setVoice] = useState('playful');
  const [voices, setVoices] = useState([]);
  const [text, setText] = useState(`Hola, soy ${bot.name}. Ya estoy listo: dime qué quieres que haga.`);
  const [busy, setBusy] = useState('');
  const [err, setErr] = useState('');
  const stage = useRef(null);
  const el = () => stage.current && stage.current.querySelector('bot-face');
  const run = (what, fn) => async () => {
    const f = el();
    if (!f || busy) return;
    setErr(''); setBusy(what);
    try { await fn(f); } catch (e) { setErr(e.message); }
    setBusy('');
  };
  useEffect(() => { loadFace().then((m) => setVoices(m.BOT_FACE_DIALOGUE_VOICES || []), () => {}); return undefined; }, []);
  useEffect(() => () => { const f = el(); if (f) { f.stopSequence && f.stopSequence(); f.stopDialogue && f.stopDialogue(); } }, []);
  const effect = (id) => run('effect', (f) => f.playMorph(id, { hold: 900, restore: 'default' }))();
  const sequence = run('sequence', (f) => f.playSequence(DEMO, { loop: false }));
  const speak = (seed) => run('dialogue', (f) => f.performDialogue(planDialogueActions(text, seed ? { seed } : {}).script, { voice, englishMode: 'phonetic', rate: 1 }))();
  const stop = () => { const f = el(); if (f) { f.stopSequence(); f.stopDialogue(); } setBusy(''); };
  const latest = run('latest', async () => {
    const conv = keyOf(bot);
    const rows = await api('GET', `/api/turns?conv=${encodeURIComponent(conv)}&limit=5`);
    const last = (rows || []).find((r) => r.role === conv && r.answer);
    if (!last) throw new Error('Este bot aún no ha respondido nada.');
    setText(String(last.answer).slice(0, 600));
  });
  return html`<div class="banim">
    <div class="banim-stage" ref=${stage}><${Face} ...${lookOf(bot)} size=${150} state=${state} lively /></div>
    <p class="bnote"><a href="/vendor/botface/editor/index.html" target="_blank" rel="noopener">Abrir el editor completo</a> · secuencias y diálogo paso a paso, con el código listo para copiar.</p>
    <label class="bcap" for="banim-state">Estado</label>
    <select id="banim-state" class="bsel" value=${state} onChange=${(e) => setState(e.target.value)}>
      ${STATE_CATALOG.map((s) => html`<option key=${s.id} value=${s.id}>${s.en} · ${s.id}</option>`)}
    </select>
    <p class="bcap">Efectos de un solo uso</p>
    <div class="banim-effects">${MORPH_CATALOG.map((m) => html`<button type="button" key=${m.id} class="banim-chip" disabled=${!!busy} onClick=${() => effect(m.id)}>${m.en}</button>`)}</div>
    <div class="banim-row">
      <button type="button" class="bbtn" disabled=${!!busy} onClick=${sequence}>Secuencia de muestra</button>
      <button type="button" class="bbtn" disabled=${!busy} onClick=${stop}>Parar</button>
    </div>
    <label class="bcap" for="banim-text">Diálogo con voz local</label>
    <textarea id="banim-text" class="banim-text" rows=${4} maxlength=${600} value=${text} onInput=${(e) => setText(e.target.value)}></textarea>
    <div class="banim-row">
      <select class="bsel" aria-label="Voz" value=${voice} onChange=${(e) => setVoice(e.target.value)}>
        ${voices.map((v) => html`<option key=${v.id} value=${v.id}>${v.label.en}</option>`)}
      </select>
      <button type="button" class="bbtn pri" disabled=${!!busy || !text.trim()} onClick=${() => speak(0)}>${busy === 'dialogue' ? 'Hablando…' : 'Hablar'}</button>
      <button type="button" class="bbtn" disabled=${!!busy || !text.trim()} onClick=${() => speak((Math.random() * 1e9) | 0)}>Otra toma</button>
      <button type="button" class="blink" disabled=${!!busy} onClick=${latest}>Última respuesta</button>
    </div>
    <p class="bnote">La voz se genera en esta página, sin red: sílabas muestreadas sincronizadas con el texto y las acciones. Pulsa «Parar» para cortarla.</p>
    ${err ? html`<p class="berr">${err}</p>` : null}
  </div>`;
}
