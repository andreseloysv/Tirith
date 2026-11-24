// Access, inside Settings behind its button (the owner's wish of 2026-11-23): the card that opens it, and the page
// itself (#/settings/access) with the way back. What it shows is the Access view: what the agents reached outside
// the machine, the block list and the network guard, and the connectors.
import { html } from '../../../vendor/preact-htm.js';
import { go } from '../../common.js';
import { Access } from '../access.js';
import { t } from '../../i18n.js';

export function AccessCard() {
  return html`<div class="card scard setopen"><div class="phead"><b>${t('Access')}</b><span class="mut sm">${t('What your agents reached on the internet, and what is blocked')}</span></div>
    <button type="button" class="sadd" onClick=${() => go('settings', 'access')}>${t('Open')}</button></div>`;
}

export function AccessPage() {
  return html`<div class="setsub">
    <button type="button" class="setback" onClick=${() => go('settings', '')}><span aria-hidden="true">‹</span> ${t('Settings')}</button>
    <${Access} project="" />
  </div>`;
}
