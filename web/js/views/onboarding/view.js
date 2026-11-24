// The welcome on the Home right after the first agent connects, instead of "All quiet": the new assistant greets
// the owner, asks for a name (that makes the owner's first bot), says what it is and can do, and offers a first
// step. Its words come one by one, as if typed. Its parts: lines.js (the words), act.js (what the buttons do),
// state.js (how far it got), talk.js (the bubbles).
import { html, useState, useEffect } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { t } from '../../i18n.js';
import { hello, askName, met, about, can, offer, choices } from './lines.js';
import { loadWelcome, saveWelcome } from './state.js';
import { makeBot, choose } from './act.js';
import { Talk } from './talk.js';

const PACE = 900; /* ms a line takes to "type" */

function linesOf(st) {
  const first = [{ text: hello(st.owner) }, { text: askName() }];
  if (!st.bot) return first;
  return [...first, { me: true, text: st.bot.name }, { text: met(st.bot.name, st.owner) }, { text: about() }, { text: can() }, { text: offer() }];
}

export function Onboarding() {
  const [st, setSt] = useState(() => loadWelcome() || { step: 'name' });
  const [shown, setShown] = useState(st.bot ? 99 : 0);
  const [name, setName] = useState('');
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  useEffect(() => { if (st.owner === undefined) api('GET', '/api/me').then((r) => setSt((s) => ({ ...s, owner: r.name || '' })), () => setSt((s) => ({ ...s, owner: '' }))); }, []);
  const lines = st.owner === undefined ? [] : linesOf(st);
  useEffect(() => {
    if (shown >= lines.length) return undefined;
    const timer = setTimeout(() => setShown((n) => n + 1), lines[shown] && lines[shown].me ? 0 : PACE);
    return () => clearTimeout(timer);
  }, [shown, lines.length]);
  const update = (next) => { saveWelcome(next); setSt(next); };
  const named = async (e) => {
    e.preventDefault();
    if (!name.trim() || busy) return;
    setBusy(true); setErr('');
    try { const b = await makeBot(name.trim()); update({ ...st, step: 'offer', bot: { id: b.id, name: b.name || name.trim() } }); } catch (x) { setErr(String(x.message || x)); }
    setBusy(false);
  };
  const pick = async (choice) => {
    setBusy(true); setErr('');
    try { saveWelcome({ ...st, step: 'done' }); await choose(choice, st); } catch (x) { setErr(String(x.message || x)); setBusy(false); }
  };
  const done = shown >= lines.length && lines.length > 0;
  return html`<div class="hm-page"><div class="home ob">
    <div class="ob-skip"><button type="button" class="alink" onClick=${() => { update({ ...st, step: 'done' }); location.reload(); }}>${t('Skip')}</button></div>
    <${Talk} lines=${lines.slice(0, shown)} typing=${!done && lines.length > 0} />
    ${done && !st.bot ? html`<form class="hm-reply" onSubmit=${named}><label class="hm-box"><textarea rows="1" value=${name} placeholder=${t('A name for me')}
      onInput=${(e) => setName(e.target.value)} onKeyDown=${(e) => { if (e.key === 'Enter' && !e.shiftKey) named(e); }} /><button type="submit" class="hm-send" disabled=${!name.trim() || busy} aria-label=${t('Send')}>↑</button></label></form>` : null}
    ${done && st.bot ? html`<ol class="ob-choices">${choices().map(([id, words], i) => html`<li key=${id}><button type="button" disabled=${busy} onClick=${() => pick(id)}>
      <b>${'ABC'[i]}</b><span>${words}</span></button></li>`)}</ol>` : null}
    ${err ? html`<p class="hm-err" role="alert">${err}</p>` : null}
  </div></div>`;
}
