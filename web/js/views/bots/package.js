// A bot as a package: «Import» reads a file and shows what it will do before it makes anything; «Export» writes the bot's signed package.
import { html, useState } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { ICON, Icon } from '../../common.js';
import { PackagePreview } from './package-preview.js';

const save = (name, data) => {
  const a = document.createElement('a');
  a.href = URL.createObjectURL(new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' }));
  a.download = `${name.toLowerCase().replace(/[^a-z0-9]+/g, '-')}.neobot-bot.json`;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
};

function Importer({ onMade }) {
  const [pkg, setPkg] = useState(null), [shown, setShown] = useState(null), [err, setErr] = useState(''), [busy, setBusy] = useState(false);
  const read = async (file) => {
    setErr(''); setShown(null);
    try { const p = JSON.parse(await file.text()); setPkg(p); setShown(await api('POST', '/api/bots/package/preview', { package: p })); } catch (e) { setErr(e.message); }
  };
  const make = () => { setBusy(true); api('POST', '/api/bots/package/import', { package: pkg, hash: shown.hash }).then((r) => onMade(r.bot), (e) => { setErr(e.message); setBusy(false); }); };
  return html`<div class="btlist bpkgbody">
    <input type="file" accept=".json,application/json" aria-label="Bot package" onChange=${(e) => e.target.files[0] && read(e.target.files[0])} />
    ${shown ? html`<${PackagePreview} p=${shown} /><button type="button" class="bbtn pri" disabled=${busy} onClick=${make}>Import this bot</button>` : null}
    ${err ? html`<p class="berr">${err}</p>` : null}</div>`;
}

function Exporter({ bot }) {
  const [about, setAbout] = useState({ description: '', instructions: '' }), [err, setErr] = useState('');
  const set = (k) => (e) => setAbout({ ...about, [k]: e.target.value });
  const go = () => api('POST', `/api/bots/${encodeURIComponent(bot.id)}/export`, about).then((p) => save(bot.name, p), (e) => setErr(e.message));
  return html`<div class="btlist bpkgbody">
    <p class="bmut">Routines, skills and the connectors it is granted go in, signed with this NeoBot's key. Memory, chats, logins and webhook addresses never do.</p>
    <textarea placeholder="Description" aria-label="Description" maxlength="2000" value=${about.description} onInput=${set('description')} />
    <textarea placeholder="Instructions for whoever adds it (optional)" aria-label="Instructions" value=${about.instructions} onInput=${set('instructions')} />
    <button type="button" class="bbtn pri" onClick=${go}>Export ${bot.name}</button>${err ? html`<p class="berr">${err}</p>` : null}</div>`;
}

export function BotPackage({ bot, onClose, onMade }) {
  const title = bot ? 'Export bot' : 'Import bot';
  return html`<div class="bmback" onClick=${onClose}>
    <div class="bmodal btpls" role="dialog" aria-modal="true" aria-label=${title} onClick=${(e) => e.stopPropagation()}>
      <header><h2>${title}</h2><button type="button" class="bx" aria-label="Close" onClick=${onClose}><${Icon} d=${ICON.close} size=${14} w=${1.6} /></button></header>
      ${bot ? html`<${Exporter} bot=${bot} />` : html`<${Importer} onMade=${onMade} />`}
    </div>
  </div>`;
}
