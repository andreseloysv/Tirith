// One agent's row: its name, one line of where it stands, and its one action: Set up, Try again, or Check (the same
// steps again: they find everything in place, sign it in again if it was signed out, and run its check turn). While
// it signs in, what the browser needs: the code to confirm, the sign-in page, a box for a code the browser gives.
import { html, useState } from '../../../vendor/preact-htm.js';
import { t } from '../../i18n.js';

const STEP = {
  machine: t('Preparing this computer…'),
  install: t('Installing…'),
  signin: t('Sign in with your browser'),
  check: t('Checking that it works…'),
};
const FAILED = {
  machine: t('Could not prepare this computer'),
  install: t('Could not install it'),
  signin: t('The sign-in did not finish'),
  check: t('It did not pass its check'),
};

function state(a) {
  const j = a.job;
  if (j && j.running) return { text: STEP[j.step] || t('Working…'), dot: 'blue' };
  if (j && !j.ok && j.error) return { text: FAILED[j.step] || t('It did not finish'), dot: 'red', again: true };
  if (a.connected) return { text: t('Connected'), dot: 'green' };
  return { text: a.installed ? t('Installed, not connected yet') : t('Not installed'), dot: 'grey' };
}

function Paste({ a, act }) {
  const [code, setCode] = useState('');
  const send = (e) => { e.preventDefault(); if (code.trim()) act(`/api/setup/agents/${a.alias}/code`, { code: code.trim() }); };
  return html`<form class="su-paste" onSubmit=${send}>
    <input value=${code} onInput=${(e) => setCode(e.target.value)} placeholder=${t('Paste the code your browser shows')} aria-label=${t('Code from the browser')} />
    <button type="submit" class="hm-btn hm-yes" disabled=${!code.trim()}>${t('Send')}</button></form>`;
}

/* The sign-in page its login opened by itself is usually in front already; the button opens the one it printed,
   whose page may hand back a code to paste, so the box shows only after that. */
function SignIn({ a, act }) {
  const j = a.job;
  const [opened, setOpened] = useState(false);
  const open = () => { setOpened(true); act(`/api/setup/agents/${a.alias}/open`); };
  return html`<div class="su-sign">
    ${j.code ? html`<p class="su-code">${t('Confirm this code in your browser:')} <b>${j.code}</b></p>` : null}
    ${j.link ? html`<button type="button" class="hm-btn" onClick=${open}>${t('Open the sign-in page')}</button>` : null}
    ${j.paste && opened ? html`<${Paste} a=${a} act=${act} />` : null}
  </div>`;
}

export function AgentRow({ a, act }) {
  const j = a.job;
  const s = state(a);
  const running = j && j.running;
  const start = () => act(`/api/setup/agents/${a.alias}`);
  const label = s.again ? t('Try again') : a.connected ? t('Check') : t('Set up');
  return html`<div class="su-row">
    <div class="su-top"><span class=${`dot ${s.dot}`}></span><b>${a.name}</b><span class="su-state">${s.text}</span>
      ${running ? null : html`<button type="button" class=${a.connected ? 'hm-btn' : 'hm-btn hm-yes'} onClick=${start}>${label}</button>`}</div>
    ${running && j.step === 'signin' ? html`<${SignIn} a=${a} act=${act} />` : null}
    ${s.again ? html`<p class="hm-err">${j.error}</p>` : null}
  </div>`;
}
