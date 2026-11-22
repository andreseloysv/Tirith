// The Library's mail recipe, as text: a short Google Apps Script, with the bot's webhook address already in it, that sends
// the owner's new mail labelled tirith/<bot name> to the bot every 5 minutes. A message is marked read only after Tirith
// answers 2xx, one that fails does not stop the next, and each call stays under the platform's 64 KB whatever the characters.

/** The script for `url` and the label `tirith/<name>`. */
export const mailScript = (url, name) => `// Sends the new mail labelled tirith/${String(name).replace(/\s+/g, ' ')} to Tirith every 5 minutes, and marks it read once Tirith has it.
const HOOK = ${JSON.stringify(url)};
const LABEL = ${JSON.stringify(`tirith/${name}`)};
const LIMIT = 60000; // bytes: the webhook takes 64 KB at most

function bytes(s) { return Utilities.newBlob(s).getBytes().length; }
function clip(s, n) { s = String(s || '').slice(0, n); return /[\\uD800-\\uDBFF]$/.test(s) ? s.slice(0, -1) : s; }

// The call's body: the body of the mail is cut until the whole is under LIMIT bytes, whatever its characters.
function body(m, thread) {
  const make = (text) => JSON.stringify({ title: clip(m.getSubject(), 300), url: thread.getPermalink(), from: clip(m.getFrom(), 300),
    subject: clip(m.getSubject(), 300), date: m.getDate().toISOString(), text: text });
  let text = clip(m.getPlainBody(), 15000), json = make(text);
  while (bytes(json) > LIMIT && text) { text = clip(text, Math.floor(text.length * 0.7)); json = make(text); }
  return json;
}

function send(m, thread) {
  try {
    const res = UrlFetchApp.fetch(HOOK, { method: 'post', contentType: 'application/json', payload: body(m, thread), muteHttpExceptions: true });
    const code = res.getResponseCode();
    if (code >= 200 && code < 300) { m.markRead(); return true; }
    console.warn('Tirith answered ' + code + ': the mail stays unread for the next run');
  } catch (e) { console.warn('Could not send a mail to Tirith: ' + e); }
  return false;
}

function sendToTirith() {
  const label = GmailApp.getUserLabelByName(LABEL);
  if (!label) return;
  label.getThreads(0, 50).forEach((thread) => thread.getMessages().filter((m) => m.isUnread()).forEach((m) => send(m, thread)));
}

// Run once: every 5 minutes from now on.
function install() {
  ScriptApp.getProjectTriggers().forEach((t) => ScriptApp.deleteTrigger(t));
  ScriptApp.newTrigger('sendToTirith').timeBased().everyMinutes(5).create();
}
`;
