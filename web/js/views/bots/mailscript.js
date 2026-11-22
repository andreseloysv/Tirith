// The Library's mail recipe: the script (mailtext.js) with the bot's webhook address in it, and one button that makes the
// address and the routine that reads what the script sends (an event from outside whose `when` is mail:*).
import { html, useState } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { useWebhook, useCopy } from './webhook.js';
import { mailScript } from './mailtext.js';

const TASK = 'A new e-mail reached you: {title}. Its sender, date and text:\n{text}\nTell me in a few lines what it is and what needs my attention. When it needs nothing, answer with one line that starts with ∅.';

export function MailScript({ bot }) {
  const [hook, reload] = useWebhook(bot);
  const [fail, setFail] = useState('');
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);
  const [copied, copy] = useCopy(setFail);
  const [open, setOpen] = useState(false);
  const base = `/api/bots/${encodeURIComponent(bot.id)}`;
  const text = hook && hook.url ? mailScript(hook.url, bot.name) : '';
  const has = (bot.routines || []).some((r) => r.trigger === 'webhook' && /^(mail|\*)/.test(r.when || '')); /* one already listens for mail: not changed */
  const setup = () => {
    setBusy(true);
    api('POST', `${base}/webhook`, {}).then(() => has || api('POST', `${base}/routines`, { title: `Mail to ${bot.name}`, prompt: TASK, trigger: 'webhook', when: 'mail:*' }))
      .then(() => { setFail(''); setDone(true); reload(); }, (e) => setFail(e.message)).finally(() => setBusy(false));
  };
  return html`<div class="bmail">
    <h4><span>Mail to ${bot.name}</span>${text ? html`<button type="button" class="blink" onClick=${() => setOpen(!open)}>${open ? 'Hide' : 'Show the script'}</button>` : null}</h4>
    ${!text ? html`<p class="bnote">${bot.name} reads your mail labelled <b>tirith/${bot.name}</b> every 5 minutes, with no mail server.</p>
        <button type="button" class="bbtn pri" disabled=${busy} onClick=${setup}>${busy ? 'Setting up…' : 'Set it up'}</button>`
    : html`<p class="bnote">1. In script.google.com, a new project: paste this, save, and run <b>install</b> once (allow it to read your Gmail).<br />
        2. In Gmail, label the mail for ${bot.name} <b>tirith/${bot.name}</b> (a filter does it by itself).${done ? ' The routine «Mail to ' + bot.name + '» is ready in Routines: Probar sends it a sample e-mail.' : ''}</p>
      ${open ? html`<pre class="bscript">${text}</pre>` : null}
      <button type="button" class="bbtn pri" onClick=${() => copy(text)}>${copied ? 'Copied' : 'Copy the script'}</button>
      ${done || has ? null : html`<button type="button" class="blink" disabled=${busy} onClick=${setup}>Make the routine</button>`}`}
    ${fail ? html`<p class="berr">${fail}</p>` : null}</div>`;
}
