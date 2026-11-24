// The bot's card, drawn by the app over any view (the sidebar's "+" and "Edit"): name, shape, colour, its model
// (the managers' unless it has one of its own) and its factory shape.
import { html, useState, useEffect, useRef } from '../../../vendor/preact-htm.js';
import { useFetch, api } from '../../store.js';
import { ICON, Icon } from '../../common.js';
import { Face, SHAPES, COLORS, ringOf } from './face.js';
import { useCardModel, modelSection } from './card-model.js';
import { factorySection } from './card-factory.js';
import { connectorsSection } from './card-connectors.js';
import { materialSection } from './card-material.js';

/** Focus the name; Esc closes the card. */
function useCardKeys(input, onClose) {
  useEffect(() => {
    if (input.current) input.current.focus();
    const key = (e) => { if (e.key === 'Escape') { e.preventDefault(); onClose(); } };
    addEventListener('keydown', key);
    return () => removeEventListener('keydown', key);
  }, []);
}

/** The shape and colour pickers; each shape wears the material being chosen. */
function pickers(shape, setShape, color, setColor, look) {
  return html`<div class="bshapes" role="radiogroup" aria-label="Shape">${Object.keys(SHAPES).map((k) => html`<button type="button" role="radio" key=${k}
          aria-checked=${k === shape} aria-label=${k} class=${k === shape ? 'on' : ''} onClick=${() => setShape(k)}><span class="bface"><${Face} ...${{ ...look, shape: k, color }} size=${58} still />${k === shape
            ? html`<svg class="bring" viewBox="-12 -12 124 124" aria-hidden="true"><path d=${ringOf(k)} /></svg>` : null}</span></button>`)}</div>
        <div class="bcolors" role="radiogroup" aria-label="Colour">${COLORS.map((c) => html`<button type="button" role="radio" key=${c}
          aria-checked=${c === color} aria-label=${c} class=${c === color ? 'on' : ''} style=${`--c:${c}`} onClick=${() => setColor(c)}><i></i></button>`)}</div>`;
}

/** Delete (asked once), Cancel and Save. */
function footer({ bot, ask, setAsk, del, onClose, name, valid, save }) {
  return html`<footer>
        ${bot.id && !bot.system ? (ask ? html`<span class="bask">Delete ${bot.name} with its routines, memory and own skills?</span><button type="button" class="bbtn bdel" onClick=${del}>Delete</button>
            <button type="button" class="bbtn" onClick=${() => setAsk(false)}>Keep</button>`
          : html`<button type="button" class="blink bdel" onClick=${() => setAsk(true)}>Delete bot</button>`) : null}
        ${ask ? null : html`<span class="bsp"></span><button type="button" class="bbtn" onClick=${onClose}>Cancel</button>
          <button type="button" class="bbtn pri" disabled=${!name.trim() || !valid} onClick=${save}>${bot.id ? 'Save' : 'Create'}</button>`}
      </footer>`;
}

export function BotCard({ bot, onClose, onSaved }) {
  const [name, setName] = useState(bot.name || '');
  const [shape, setShape] = useState(bot.shape || 'cloud');
  const [color, setColor] = useState(bot.color || '#ec4899');
  const [session, setSession] = useState(bot.session || 'chat');
  const [access, setAccess] = useState(bot.access === 'read' ? 'read' : '');
  const [routes, setRoutes] = useState(bot.routes || []);
  const [material, setMaterial] = useState(bot.material || 'solid');
  const [eye, setEye] = useState(bot.eye || '#161616');
  const [gradient, setGradient] = useState(bot.gradient || 'ocean-signal');
  const [gstart, setGstart] = useState(bot.gstart || '#3b82f6');
  const [gend, setGend] = useState(bot.gend || '#8b5cf6');
  const [gangle, setGangle] = useState(bot.gangle == null ? '135' : String(bot.gangle));
  const [glass, setGlass] = useState(bot.glass || 'iridescent-orb');
  const look = { material, eye, gradient, gstart, gend, gangle: Number(gangle), glass };
  const setLook = (k, v) => ({ material: setMaterial, eye: setEye, gradient: setGradient, gstart: setGstart, gend: setGend, gangle: setGangle, glass: setGlass })[k](v);
  const [err, setErr] = useState('');
  const [ask, setAsk] = useState(false);
  const [M] = useFetch('/api/models', ['models']);
  const cm = useCardModel(bot, M), valid = cm.valid;
  const input = useRef(null);
  useCardKeys(input, onClose);
  const EVENTS = (M && M.events) || [];
  const save = async () => {
    if (!name.trim() || !valid) return;
    try {
      const b = await api('POST', '/api/bots', { id: bot.id, name: name.trim(), shape, color, session, access, routes, material, eye,
        gradient: material === 'gradient' ? gradient : '', gstart: gradient === 'custom' ? gstart : '', gend: gradient === 'custom' ? gend : '',
        gangle: material === 'gradient' && gradient === 'custom' ? Number(gangle) : null, glass: material === 'rainbow-glass' ? glass : '' });
      await cm.persist(b);
      onSaved(b);
    } catch (e) { setErr(e.message); }
  };
  const del = () => api('POST', `/api/bots/${encodeURIComponent(bot.id)}/delete`).then(() => onSaved(null), (e) => setErr(e.message));
  return html`<div class="bmback" onClick=${onClose}>
    <div class="bmodal" role="dialog" aria-modal="true" aria-label=${bot.id ? 'Edit bot' : 'New bot'} onClick=${(e) => e.stopPropagation()}>
      <header><h2>${bot.id ? 'Edit bot' : 'New bot'}</h2><button type="button" class="bx" aria-label="Close" onClick=${onClose}><${Icon} d=${ICON.close} size=${14} w=${1.6} /></button></header>
      <div class="bmbody">
        <div class="bprev"><${Face} ...${{ ...look, shape, color }} size=${132} /></div>
        <input ref=${input} class="bnamein" placeholder="Name your bot" maxlength="40" aria-label="Name" value=${name}
          onInput=${(e) => setName(e.target.value)} onKeyDown=${(e) => e.key === 'Enter' && save()} />
        ${pickers(shape, setShape, color, setColor, look)}
        ${materialSection(look, setLook)}
        ${M ? modelSection(M, cm) : null}
        ${M ? factorySection({ bot, session, setSession, access, setAccess, routes, setRoutes, EVENTS }) : null}
        ${connectorsSection(bot)}
        ${err ? html`<p class="berr">${err}</p>` : null}
      </div>
      ${footer({ bot, ask, setAsk, del, onClose, name, valid, save })}
    </div>
  </div>`;
}
