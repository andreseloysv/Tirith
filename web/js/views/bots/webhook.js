// A bot's webhook in its Routines tab: the address to copy (made once on agentsmessenger.com, sealed for this
// Tirith), or the button that makes it.
import { html, useState } from '../../../vendor/preact-htm.js';
import { useFetch, api } from '../../store.js';
import { t } from '../../i18n.js';

/** `[hook, reload, error]`: the bot's webhook as the daemon keeps it ({url} or {url: null}). */
export const useWebhook = (bot) => useFetch(`/api/bots/${encodeURIComponent(bot.id)}/webhook`, ['bots'], [bot.id]);

/** Copies `text`, saying so for two seconds; `onFail` when the browser refuses. */
export function useCopy(onFail) {
  const [copied, setCopied] = useState(false);
  const copy = (text) => navigator.clipboard.writeText(text).then(() => { setCopied(true); setTimeout(() => setCopied(false), 2000); },
    () => onFail(t('Could not copy: select it and copy it by hand.')));
  return [copied, copy];
}

export function Webhook({ bot }) {
  const base = `/api/bots/${encodeURIComponent(bot.id)}/webhook`;
  const [hook, reload, err] = useWebhook(bot);
  const [busy, setBusy] = useState(false);
  const [fail, setFail] = useState('');
  const [copied, copy] = useCopy(setFail);
  const act = (path) => {
    setBusy(true);
    api('POST', base + path, {}).then(() => { setFail(''); reload(); }, (x) => setFail(x.message)).finally(() => setBusy(false));
  };
  if (!hook) return html`<p class="bnote">${err ? t('Could not read the webhook: {error}', { error: err.message }) : t('Loading…')}</p>`;
  return html`<div class="bhook"><span class="bcap">${t('Webhook address')}</span>
    ${hook.url ? html`<div class="bhookrow"><input readonly value=${hook.url} aria-label=${t('Webhook address')} onFocus=${(e) => e.target.select()} />
        <button type="button" class="bbtn" onClick=${() => copy(hook.url)}>${copied ? t('Copied') : t('Copy')}</button></div>
      <span class="bmut">${t('GitHub, a script or any service posts here. What arrives reaches {name} sealed, as data and never as orders.', { name: bot.name })}
        <button type="button" class="blink" disabled=${busy} onClick=${() => act('/delete')}>${t('Delete address')}</button></span>`
    : html`<span class="bmut">${t('One address for every event routine of {name}; only this NeoBot can read what arrives.', { name: bot.name })}</span>
      <button type="button" class="bbtn" disabled=${busy} onClick=${() => act('')}>${busy ? t('Making…') : t('Make the address')}</button>`}
    ${fail ? html`<p class="berr">${fail}</p>` : null}</div>`;
}
