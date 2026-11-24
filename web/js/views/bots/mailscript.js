// The Library's mail recipe: the script (mailtext.js) with the bot's webhook address in it, and one button that makes the
// address and the routine that reads what the script sends (an event from outside whose `when` is mail:*).
import { html, useState } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { useWebhook, useCopy } from './webhook.js';
import { mailScript } from './mailtext.js';
import { t } from '../../i18n.js';

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
    api('POST', `${base}/webhook`, {}).then(() => has || api('POST', `${base}/routines`, { title: t('Mail to {name}', { name: bot.name }), prompt: TASK, trigger: 'webhook', when: 'mail:*' }))
      .then(() => { setFail(''); setDone(true); reload(); }, (e) => setFail(e.message)).finally(() => setBusy(false));
  };
  return html`<div class="bmail">
    <h4><span>${t('Mail to {name}', { name: bot.name })}</span>${text ? html`<button type="button" class="blink" onClick=${() => setOpen(!open)}>${open ? t('Hide') : t('Show the script')}</button>` : null}</h4>
    ${!text ? html`<p class="bnote">${t('{name} reads your mail labelled {label} every 5 minutes, with no mail server.', { name: bot.name, label: `tirith/${bot.name}` })}</p>
        <button type="button" class="bbtn pri" disabled=${busy} onClick=${setup}>${busy ? t('Setting up…') : t('Set it up')}</button>`
    : html`<p class="bnote">${t('1. In script.google.com, a new project: paste this, save, and run install once (allow it to read your Gmail).')}<br />
        ${t('2. In Gmail, label the mail for {name} tirith/{name} (a filter does it by itself).', { name: bot.name })}${done ? ' ' + t('The routine «Mail to {name}» is ready in Routines: Test sends it a sample e-mail.', { name: bot.name }) : ''}</p>
      ${open ? html`<pre class="bscript">${text}</pre>` : null}
      <button type="button" class="bbtn pri" onClick=${() => copy(text)}>${copied ? t('Copied') : t('Copy the script')}</button>
      ${done || has ? null : html`<button type="button" class="blink" disabled=${busy} onClick=${setup}>${t('Make the routine')}</button>`}`}
    ${fail ? html`<p class="berr">${fail}</p>` : null}</div>`;
}
