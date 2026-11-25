// The welcome on the Home right after the first agent connects, instead of "All quiet": the first bot (NeoBot, which
// every new home has) greets the owner, asks what to call it — a name, or it stays NeoBot —, says what it is and can
// do, and offers a first step. Its words come one by one, as if typed. Its parts: lines.js (the words), act.js (what
// the buttons do), state.js (how far it got), talk.js (the bubbles).
import { html, useState, useEffect } from '../../../vendor/preact-htm.js';
import { t } from '../../i18n.js';
import { lookOf } from '../bots/face/face-map.js';
import { hello, askName, met, about, can, offer, choices } from './lines.js';
import { loadWelcome, saveWelcome } from './state.js';
import { rename, choose, skip } from './act.js';
import { Talk } from './talk.js';

const PACE = 900; /* ms a line takes to "type" */

function linesOf(st, bot, owner) {
  const first = [{ text: hello(owner, st.was || bot.name) }, { text: askName() }];
  if (!st.named) return first;
  return [...first, { me: true, text: st.named }, { text: met(st.named, owner) }, { text: about() }, { text: can() }, { text: offer() }];
}

/* the answer to "what would you like to call me?": a name typed, or the one it has */
function nameForm({ bot, name, setName, busy, named }) {
  const send = (e) => { e.preventDefault(); if (name.trim()) named(name.trim()); };
  return html`<form class="hm-reply" onSubmit=${send}><label class="hm-box"><textarea rows="1" value=${name} placeholder=${t('A name for me')}
      onInput=${(e) => setName(e.target.value)} onKeyDown=${(e) => { if (e.key === 'Enter' && !e.shiftKey) send(e); }} />
    <button type="submit" class="hm-send" disabled=${!name.trim() || busy} aria-label=${t('Send')}>↑</button></label>
    <button type="button" class="ob-keep" disabled=${busy} onClick=${() => named(bot.name)}>${t('Keep {name}', { name: bot.name })}</button></form>`;
}

/** `bot`: the bot that owes its welcome (GET /api/me says which); `owner`: their first name, or ''. */
export function Onboarding({ bot, owner }) {
  const [st, setSt] = useState(() => loadWelcome(bot.id));
  const [shown, setShown] = useState(st.named ? 99 : 0);
  const [name, setName] = useState('');
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  const lines = linesOf(st, bot, owner);
  useEffect(() => {
    if (shown >= lines.length) return undefined;
    const timer = setTimeout(() => setShown((n) => n + 1), lines[shown] && lines[shown].me ? 0 : PACE);
    return () => clearTimeout(timer);
  }, [shown, lines.length]);
  const act = async (work) => { setBusy(true); setErr(''); try { await work(); } catch (x) { setErr(String(x.message || x)); } setBusy(false); };
  const named = (called) => act(async () => {
    if (called !== bot.name) await rename(bot, called);
    const next = { ...st, was: bot.name, named: called };
    saveWelcome(next); setSt(next);
  });
  const done = shown >= lines.length;
  return html`<div class="hm-page"><div class="home ob">
    <div class="ob-skip"><button type="button" class="alink" disabled=${busy} onClick=${() => act(() => skip(bot.id))}>${t('Skip')}</button></div>
    <${Talk} lines=${lines.slice(0, shown)} typing=${!done} look=${lookOf(bot)} />
    ${done && !st.named ? nameForm({ bot, name, setName, busy, named }) : null}
    ${done && st.named ? html`<ol class="ob-choices">${choices().map(([id, words], i) => html`<li key=${id}><button type="button" disabled=${busy}
      onClick=${() => act(() => choose(id, st, owner))}><b>${'ABC'[i]}</b><span>${words}</span></button></li>`)}</ol>` : null}
    ${err ? html`<p class="hm-err" role="alert">${err}</p>` : null}
  </div></div>`;
}
