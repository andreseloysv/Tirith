// The language NeoBot speaks, English, Spanish or German (../../i18n.js): in Settings, and on the Welcome screen,
// where Settings cannot be reached yet. A pick applies at once (the page is drawn again in it).
import { html } from '../../../vendor/preact-htm.js';
import { LANGS, getLang, setLang, t } from '../../i18n.js';

export function LanguagePicker() {
  const cur = getLang();
  return html`<div class="seg" role="group" aria-label=${t('Language')}>${LANGS.map(([k, l]) => html`<button type="button"
    aria-pressed=${k === cur} class=${k === cur ? 'on' : ''} onClick=${() => k !== cur && setLang(k)}>${l}</button>`)}</div>`;
}

export function LanguageCard() {
  return html`<div class="card scard themecard"><div class="phead"><b>${t('Language')}</b><span class="mut sm">${t('The words of NeoBot and of your bots')}</span></div>
    <${LanguagePicker} /></div>`;
}
