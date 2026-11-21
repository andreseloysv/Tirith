// The Library's mail recipe: a short Google Apps Script, with the bot's webhook address already in it, that sends the
// owner's new mail labelled tirith/<bot name> to the bot every 5 minutes and marks it read.
import { html, useState } from '../../../vendor/preact-htm.js';
import { useWebhook, useCopy } from './webhook.js';

/** The script for `url` and the label `tirith/<name>`. */
export const mailScript = (url, name) => `// Sends the new mail labelled tirith/${name} to ${name}, in Tirith, every 5 minutes, and marks it read.
const HOOK = ${JSON.stringify(url)};
const LABEL = ${JSON.stringify(`tirith/${name}`)};

function sendToTirith() {
  const label = GmailApp.getUserLabelByName(LABEL);
  if (!label) return;
  label.getThreads(0, 50).forEach((thread) => thread.getMessages().filter((m) => m.isUnread()).forEach((m) => {
    UrlFetchApp.fetch(HOOK, { method: 'post', contentType: 'application/json', payload: JSON.stringify({
      title: m.getSubject(), url: thread.getPermalink(), from: m.getFrom(), subject: m.getSubject(),
      date: m.getDate().toISOString(), text: m.getPlainBody().slice(0, 15000) }) });
    m.markRead();
  }));
}

// Run once: every 5 minutes from now on.
function install() {
  ScriptApp.getProjectTriggers().forEach((t) => ScriptApp.deleteTrigger(t));
  ScriptApp.newTrigger('sendToTirith').timeBased().everyMinutes(5).create();
}
`;

export function MailScript({ bot }) {
  const [hook] = useWebhook(bot);
  const [fail, setFail] = useState('');
  const [copied, copy] = useCopy(setFail);
  const [open, setOpen] = useState(false);
  const text = hook && hook.url ? mailScript(hook.url, bot.name) : '';
  return html`<div class="bmail">
    <h4><span>Mail to ${bot.name}</span>${text ? html`<button type="button" class="blink" onClick=${() => setOpen(!open)}>${open ? 'Hide' : 'Show the script'}</button>` : null}</h4>
    ${!text ? html`<p class="bnote">Make the webhook address in Routines (a routine on an event from outside) and the script to send ${bot.name} your mail appears here.</p>`
    : html`<p class="bnote">1. In script.google.com, a new project: paste this, save, and run <b>install</b> once (allow it to read your Gmail).<br />
        2. In Gmail, label the mail for ${bot.name} <b>tirith/${bot.name}</b> (a filter does it by itself): the new ones reach its event routines (when <b>*</b>) every 5 minutes.</p>
      ${open ? html`<pre class="bscript">${text}</pre>` : null}
      <button type="button" class="bbtn" onClick=${() => copy(text)}>${copied ? 'Copied' : 'Copy the script'}</button>`}
    ${fail ? html`<p class="berr">${fail}</p>` : null}</div>`;
}
