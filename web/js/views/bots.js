// Bots view: one bot (#/bots/<id>, chosen by its face in the sidebar): its conversation (the manager chat on the bot's
// conversation `bot-<id>`) and its panel, in the content area like any other view: the face (asleep, on watch or
// working) and three tabs. Routines: each with a switch, Run now, Edit and Delete, and a form for a new one. Library: what
// the bot learned, its memory.md and its skills (its own and the shared ones of every bot), to read, correct or delete.
// Phone: one QR that opens the bot's chat on the owner's phone. The sidebar's "+" and "Edit" open the bot's card (BotCard,
// drawn by the app over any view): name, shape, colour and its model (the managers' unless it has one of its own).
import { html, useState, useEffect, useRef, useLayoutEffect } from '../../vendor/preact-htm.js';
import { S, useStore, useFetch, api, setUi } from '../store.js';
import { ICON, Icon } from '../common.js';
import { Chat } from './chat.js';
import { md } from '../md.js';
import { optionsFor } from '../model-controls.js';
import { markSeen } from '../unread.js';
import { Face, NewFace, SHAPES, COLORS } from './bots/face.js';

const DAY = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
const WEEKDAYS = [1, 2, 3, 4, 5], DAILY = [1, 2, 3, 4, 5, 6, 7];
const SEL_KEY = 'tirith.bot';
export const keyOf = (b) => `bot-${b.id}`;
const isOn = (r) => r.on !== false;

export function recall() { try { return localStorage.getItem(SEL_KEY) || ''; } catch { return ''; } }
function remember(id) { try { if (id) localStorage.setItem(SEL_KEY, id); else localStorage.removeItem(SEL_KEY); } catch { /* private window */ } }

export function daysLabel(d = []) {
  const s = [...d].sort().join();
  return s === DAILY.join() ? 'Every day' : s === WEEKDAYS.join() ? 'Weekdays' : s === '6,7' ? 'Weekends' : [...d].sort().map((n) => DAY[n - 1]).join(', ');
}
const hm = (at) => String(at || '').split(':').map((x) => Number(x) || 0);
export const timeLabel = (at) => { const [h, m] = hm(at); return new Date(2000, 0, 1, h, m).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }); };

/** The next time the routine runs (the machine's local time), or null when it is off. */
export function nextRun(r, now = new Date()) {
  if (!isOn(r) || !(r.days || []).length) return null;
  const [h, m] = hm(r.at);
  for (let i = 0; i < 8; i++) {
    const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() + i, h, m);
    if (d > now && r.days.includes(((d.getDay() + 6) % 7) + 1)) return d;
  }
  return null;
}
function whenLabel(d) {
  if (!d) return '';
  const day = new Date(), t = d.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
  const diff = Math.round((new Date(d.getFullYear(), d.getMonth(), d.getDate()) - new Date(day.getFullYear(), day.getMonth(), day.getDate())) / 86400000);
  return `${diff === 0 ? 'today' : diff === 1 ? 'tomorrow' : DAY[(d.getDay() + 6) % 7]} ${t}`;
}
const soonest = (bot) => (bot.routines || []).map((r) => nextRun(r)).filter(Boolean).sort((a, b) => a - b)[0] || null;
/** What the face shows: working on a turn, on watch (a routine is on) or asleep. */
export const stateOf = (bot, busy) => (busy ? 'work' : (bot.routines || []).some(isOn) ? 'watch' : 'sleep');

/* ---------- the bot's card: name, shape, colour and model ---------- */
/** The model controls of Settings (a custom element), for one bot. */
function ModelControls({ M, value, onChange }) {
  const ref = useRef(null);
  useLayoutEffect(() => {
    ref.current.onChange = onChange;
    ref.current.configure({ choices: M.choices, value: value || {} });
  });
  return html`<model-controls ref=${ref}></model-controls>`;
}
const modelName = (v) => (v ? v.model || v.agent || 'default' : '');

export function BotCard({ bot, onClose, onSaved }) {
  const [name, setName] = useState(bot.name || '');
  const [shape, setShape] = useState(bot.shape || 'cloud');
  const [color, setColor] = useState(bot.color || '#ec4899');
  const [err, setErr] = useState('');
  const [ask, setAsk] = useState(false);
  const [M] = useFetch('/api/models', ['models']);
  const role = bot.id && M && M.roles[`bot-${bot.id}`];
  const [own, setOwn] = useState(null);      /* null: as it is; true or false: the owner's choice */
  const [model, setModel] = useState(null);  /* the model chosen here */
  const hasOwn = own == null ? !!(role && role.source === 'page') : own;
  const value = model || (role && role.source === 'page' ? role : M && M.roles.manager) || {};
  const valid = !M || !hasOwn || optionsFor(M.choices, value).valid;
  const input = useRef(null);
  useEffect(() => {
    if (input.current) input.current.focus();
    const key = (e) => { if (e.key === 'Escape') { e.preventDefault(); onClose(); } };
    addEventListener('keydown', key);
    return () => removeEventListener('keydown', key);
  }, []);
  const save = async () => {
    if (!name.trim() || !valid) return;
    try {
      const b = await api('POST', '/api/bots', { id: bot.id, name: name.trim(), shape, color });
      if (own === false && role && role.source === 'page') await api('DELETE', `/api/models/bot-${b.id}`);
      else if (hasOwn && (model || own)) await api('POST', `/api/models/bot-${b.id}`, value);
      onSaved(b);
    } catch (e) { setErr(e.message); }
  };
  const del = () => api('POST', `/api/bots/${encodeURIComponent(bot.id)}/delete`).then(() => onSaved(null), (e) => setErr(e.message));
  return html`<div class="bmback" onClick=${onClose}>
    <div class="bmodal" role="dialog" aria-modal="true" aria-label=${bot.id ? 'Edit bot' : 'New bot'} onClick=${(e) => e.stopPropagation()}>
      <header><h2>${bot.id ? 'Edit bot' : 'New bot'}</h2><button type="button" class="bx" aria-label="Close" onClick=${onClose}><${Icon} d=${ICON.close} size=${14} w=${1.6} /></button></header>
      <div class="bmbody">
        <div class="bprev"><${Face} shape=${shape} color=${color} size=${132} /></div>
        <input ref=${input} class="bnamein" placeholder="Name your bot" maxlength="40" aria-label="Name" value=${name}
          onInput=${(e) => setName(e.target.value)} onKeyDown=${(e) => e.key === 'Enter' && save()} />
        <div class="bshapes" role="radiogroup" aria-label="Shape">${Object.keys(SHAPES).map((k) => html`<button type="button" role="radio" key=${k}
          aria-checked=${k === shape} aria-label=${k} class=${k === shape ? 'on' : ''} onClick=${() => setShape(k)}><${Face} shape=${k} color=${color} size=${46} still /></button>`)}</div>
        <div class="bcolors" role="radiogroup" aria-label="Colour">${COLORS.map((c) => html`<button type="button" role="radio" key=${c}
          aria-checked=${c === color} aria-label=${c} class=${c === color ? 'on' : ''} style=${`--c:${c}`} onClick=${() => setColor(c)}><i></i></button>`)}</div>
        ${M ? html`<details class="bmodel"><summary><span class="bcap">Model</span> ${hasOwn ? modelName(value) : `the managers' · ${modelName(M.roles.manager)}`}</summary>
          <label class="bchk"><input type="checkbox" checked=${hasOwn} onChange=${(e) => setOwn(e.target.checked)} />A model of its own, not the managers'</label>
          ${hasOwn ? html`<${ModelControls} M=${M} value=${value} onChange=${(v) => { setModel(v); setOwn(true); }} />` : null}</details>` : null}
        ${err ? html`<p class="berr">${err}</p>` : null}
      </div>
      <footer>
        ${bot.id ? (ask ? html`<span class="bask">Delete ${bot.name} with its routines, memory and own skills?</span><button type="button" class="bbtn bdel" onClick=${del}>Delete</button>
            <button type="button" class="bbtn" onClick=${() => setAsk(false)}>Keep</button>`
          : html`<button type="button" class="blink bdel" onClick=${() => setAsk(true)}>Delete bot</button>`) : null}
        ${ask ? null : html`<span class="bsp"></span><button type="button" class="bbtn" onClick=${onClose}>Cancel</button>
          <button type="button" class="bbtn pri" disabled=${!name.trim() || !valid} onClick=${save}>${bot.id ? 'Save' : 'Create'}</button>`}
      </footer>
    </div>
  </div>`;
}

/* ---------- routines ---------- */
function RoutineForm({ bot, r, onDone }) {
  const [title, setTitle] = useState(r.title || '');
  const [prompt, setPrompt] = useState(r.prompt || '');
  const [days, setDays] = useState(r.days || WEEKDAYS);
  const [at, setAt] = useState(r.at || '08:00');
  const [err, setErr] = useState('');
  const ok = title.trim() && prompt.trim() && days.length && at;
  const save = (e) => {
    e.preventDefault();
    if (!ok) return;
    api('POST', `/api/bots/${encodeURIComponent(bot.id)}/routines`, { id: r.id, title: title.trim(), prompt: prompt.trim(), days, at }).then(onDone, (x) => setErr(x.message));
  };
  const flip = (n) => setDays(days.includes(n) ? days.filter((d) => d !== n) : [...days, n].sort());
  return html`<form class="bform" onSubmit=${save}>
    <label><span class="bcap">Title</span><input value=${title} placeholder="Daily job offer check" maxlength="80" onInput=${(e) => setTitle(e.target.value)} /></label>
    <label><span class="bcap">What ${bot.name} does each time</span><textarea rows="3" value=${prompt} placeholder="Check my inbox for new job offers and tell me about each one: company, role, link."
      onInput=${(e) => setPrompt(e.target.value)}></textarea></label>
    <div class="bfield"><span class="bcap">Days</span>
      <div class="bdays">${DAY.map((d, i) => html`<button type="button" key=${d} aria-pressed=${days.includes(i + 1)} aria-label=${d} class=${days.includes(i + 1) ? 'on' : ''} onClick=${() => flip(i + 1)}>${d.slice(0, 2)}</button>`)}</div>
      <div class="bpre"><button type="button" onClick=${() => setDays(DAILY)}>Every day</button><button type="button" onClick=${() => setDays(WEEKDAYS)}>Weekdays</button></div></div>
    <label class="btime"><span class="bcap">Around</span><input type="time" value=${at} onInput=${(e) => setAt(e.target.value)} /></label>
    ${err ? html`<p class="berr">${err}</p>` : null}
    <div class="bfoot"><button type="button" class="bbtn" onClick=${onDone}>Cancel</button><button type="submit" class="bbtn pri" disabled=${!ok}>Save routine</button></div>
  </form>`;
}

function Routine({ bot, r, onEdit }) {
  const [err, setErr] = useState('');
  const [ask, setAsk] = useState(false);
  const [ran, setRan] = useState(false);
  const base = `/api/bots/${encodeURIComponent(bot.id)}`;
  /* resolves to whether it worked */
  const post = (path, body) => api('POST', base + path, body).then(() => { setErr(''); return true; }, (e) => { setErr(e.message); return false; });
  const run = () => post(`/run/${encodeURIComponent(r.id)}`).then((ok) => { if (ok) { setRan(true); setTimeout(() => setRan(false), 4000); } });
  const next = nextRun(r);
  return html`<li class=${`brr${isOn(r) ? '' : ' off'}`}>
    <span class="brico"><${Icon} d=${ICON.clock} size=${18} w=${1.4} /></span>
    <span class="brtxt"><b>${r.title}</b><span class="bmut">${daysLabel(r.days)} around ${timeLabel(r.at)}</span>
      <span class="bcap">${isOn(r) ? (next ? `Next · ${whenLabel(next)}` : '') : 'Paused'}</span>
      ${ask ? html`<span class="bacts"><span class="bask">Delete this routine?</span><button type="button" class="bdel" onClick=${() => post('/routines', { id: r.id, delete: true })}>Delete</button>
          <button type="button" onClick=${() => setAsk(false)}>Keep</button></span>`
        : html`<span class="bacts"><button type="button" onClick=${run} disabled=${ran}>${ran ? 'Started' : 'Run now'}</button>
          <button type="button" onClick=${onEdit}>Edit</button><button type="button" class="bdel" onClick=${() => setAsk(true)}>Delete</button></span>`}
      ${err ? html`<span class="berr">${err}</span>` : null}</span>
    <button type="button" class=${`bswitch${isOn(r) ? ' on' : ''}`} role="switch" aria-checked=${isOn(r)} aria-label=${`${r.title}: ${isOn(r) ? 'on' : 'off'}`}
      title=${isOn(r) ? 'On: switch off' : 'Off: switch on'} onClick=${() => post('/routines', { id: r.id, on: !isOn(r) })}><i></i></button>
  </li>`;
}

/* ---------- the Library: memory and skills ---------- */
const SKILL_START = '---\nname: {name}\ndescription: When to use it, in one sentence.\n---\n\n1. ';
const descOf = (t) => (/^description:\s*(.+)$/m.exec(t || '') || [])[1] || '';
const bodyOf = (t) => String(t || '').replace(/^---\n[\s\S]*?\n---\n?/, '').trim();

function Editor({ e, onSave, onCancel, err }) {
  const [name, setName] = useState(e.name || '');
  const [text, setText] = useState(e.text || '');
  const [shared, setShared] = useState(!!e.shared);
  const named = !e.fresh || /^[a-z][a-z0-9-]{0,63}$/.test(name);
  const body = e.memory ? { memory: true, text } : { name: e.fresh ? name : e.name, shared: e.fresh ? shared : e.shared, text: e.fresh ? text.replace('{name}', name) : text };
  return html`<div class="bform bedit">
    ${e.fresh ? html`<label><span class="bcap">Name · lowercase and dashes</span><input value=${name} placeholder="check-job-portal" maxlength="64" onInput=${(x) => setName(x.target.value.trim())} /></label>
      <label class="bchk"><input type="checkbox" checked=${shared} onChange=${(x) => setShared(x.target.checked)} />Shared with every bot</label>` : null}
    <textarea class="bcode" rows=${e.memory ? 12 : 14} value=${text} aria-label=${e.memory ? 'Memory' : 'Skill'} onInput=${(x) => setText(x.target.value)}></textarea>
    ${err ? html`<p class="berr">${err}</p>` : null}
    <div class="bfoot"><button type="button" class="bbtn" onClick=${onCancel}>Cancel</button>
      <button type="button" class="bbtn pri" disabled=${!named} onClick=${() => onSave(body)}>Save</button></div>
  </div>`;
}

function Skill({ s, open, onOpen, onEdit, onDelete }) {
  const [ask, setAsk] = useState(false);
  return html`<li class="bsk">
    <button type="button" class="bskh" aria-expanded=${open} onClick=${onOpen}><b>${s.name}</b><span class="bmut">${descOf(s.text)}</span></button>
    ${open ? html`<div class="bskb md" dangerouslySetInnerHTML=${{ __html: md(bodyOf(s.text)) }} />` : null}
    <span class="bacts">${ask ? html`<span class="bask">Delete ${s.shared ? 'this skill for every bot' : 'this skill'}?</span>
        <button type="button" class="bdel" onClick=${onDelete}>Delete</button><button type="button" onClick=${() => setAsk(false)}>Keep</button>`
      : html`<button type="button" onClick=${onOpen}>${open ? 'Hide' : 'Read'}</button><button type="button" onClick=${onEdit}>Edit</button>
        <button type="button" class="bdel" onClick=${() => setAsk(true)}>Delete</button>`}</span>
  </li>`;
}

function Library({ bot }) {
  const path = `/api/bots/${encodeURIComponent(bot.id)}/library`;
  const [lib, reload, err] = useFetch(path, ['library', 'busy'], [bot.id]);
  const [edit, setEdit] = useState(null); /* {memory}, a skill, or {fresh} for a new one */
  const [open, setOpen] = useState('');
  const [ask, setAsk] = useState(false);
  const [fail, setFail] = useState('');
  useEffect(() => { setEdit(null); setOpen(''); }, [bot.id]);
  if (!lib) return html`<p class="bnote">${err ? `Could not read what ${bot.name} learned: ${err.message}` : 'Loading…'}</p>`;
  const save = (body) => api('POST', path, body).then(() => { setEdit(null); setAsk(false); setFail(''); reload(); }, (x) => setFail(x.message));
  const editor = (e) => html`<${Editor} key=${e.name || 'memory'} e=${e} err=${fail} onSave=${save} onCancel=${() => { setEdit(null); setFail(''); }} />`;
  const list = (skills, none) => (skills.length ? html`<ul class="brout">${skills.map((s) => (edit && !edit.memory && edit.name === s.name && edit.shared === s.shared
    ? html`<li key=${s.name}>${editor(s)}</li>`
    : html`<${Skill} key=${(s.shared ? 's:' : '') + s.name} s=${s} open=${open === s.name + s.shared} onOpen=${() => setOpen(open === s.name + s.shared ? '' : s.name + s.shared)}
        onEdit=${() => setEdit(s)} onDelete=${() => save({ name: s.name, shared: s.shared, delete: true })} />`))}</ul>` : html`<p class="bnote">${none}</p>`);
  const memory = lib.memory.trim();
  return html`<div class="blib">
    <h4><span>Memory</span>${edit ? null : html`<span class="bacts">${ask ? html`<span class="bask">Forget everything?</span>
        <button type="button" class="bdel" onClick=${() => save({ memory: true, delete: true })}>Clear</button><button type="button" onClick=${() => setAsk(false)}>Keep</button>`
      : html`<button type="button" onClick=${() => setEdit({ memory: true, text: lib.memory })}>Edit</button>${memory ? html`<button type="button" class="bdel" onClick=${() => setAsk(true)}>Clear</button>` : null}`}</span>`}</h4>
    ${edit && edit.memory ? editor(edit) : memory ? html`<div class="bmem md" dangerouslySetInnerHTML=${{ __html: md(memory) }} />`
      : html`<p class="bnote">Nothing remembered yet. ${bot.name} keeps here what you prefer, what you corrected and what worked, and reads it at every turn.</p>`}
    <h4><span>Skills</span>${edit ? null : html`<button type="button" class="blink" onClick=${() => setEdit({ fresh: true, text: SKILL_START })}>+ New skill</button>`}</h4>
    ${edit && edit.fresh ? editor(edit) : null}
    ${list(lib.skills.filter((s) => !s.shared), `No skill yet. When ${bot.name} works out a procedure it will repeat, it writes it down here and uses it next time.`)}
    <h4><span>Shared skills · every bot</span></h4>
    ${list(lib.skills.filter((s) => s.shared), 'None yet. A skill shared here is in every bot\'s skills.')}
  </div>`;
}

/* ---------- the bot on the phone: one QR that opens its chat there ---------- */
function Phone({ bot }) {
  const key = keyOf(bot);
  const [data, reload, err] = useFetch('/api/rooms', ['room']);
  const [fail, setFail] = useState('');
  const asked = useRef(0);
  const room = ((data && data.rooms) || []).find((r) => r.kind === 'manager' && r.project === key);
  // The QR on the screen must be the one that works: the rooms are read again on a clock and when the window comes back.
  useEffect(() => {
    const again = () => { if (!document.hidden) reload(); };
    const timer = setInterval(again, 15000);
    addEventListener('focus', again);
    return () => { clearInterval(timer); removeEventListener('focus', again); };
  }, []);
  // No room yet, or its QR was used or is old: the next one is asked for by itself, at most every 10 s.
  useEffect(() => {
    if (!data || (room && room.invite) || Date.now() - asked.current < 10000) return;
    asked.current = Date.now();
    api('POST', '/api/rooms', { project: key, title: bot.name }).then(() => { setFail(''); reload(); }, (e) => setFail(e.message));
  });
  const src = room && room.invite ? `/api/rooms/${encodeURIComponent(room.id)}/qr.svg?kind=invite&v=${encodeURIComponent(room.invite)}` : '';
  const phones = room ? (room.devices || []).length : 0;
  return html`<div class="bphone">
    ${src ? html`<img class="bqr" src=${src} alt=${`QR code that opens ${bot.name} on your phone`} width="216" height="216" />`
      : html`<div class="bqr bqrwait">${fail || (err && !data) ? 'No QR right now' : 'Making the QR…'}</div>`}
    <p class="bscan">Scan it with your phone's camera to talk to ${bot.name} there.</p>
    <p class="bnote">It works once; the next one appears here by itself.${phones ? ` ${phones} phone${phones > 1 ? 's' : ''} already in this chat.` : ''}</p>
    ${fail ? html`<p class="berr">${fail}</p>` : null}
  </div>`;
}

const TABS = [['routines', 'Routines'], ['library', 'Library'], ['phone', 'Phone']];
function Panel({ bot, busy, onEdit }) {
  const [form, setForm] = useState(null); /* null, {} for a new routine, or the routine being changed */
  const [tab, setTab] = useState('routines');
  useEffect(() => setForm(null), [bot.id]);
  const st = stateOf(bot, busy), next = soonest(bot), list = bot.routines || [];
  const status = st === 'work' ? 'Working' : st === 'watch' ? `On watch${next ? ` · next ${whenLabel(next)}` : ''}` : 'Asleep · no routine on';
  const done = () => setForm(null);
  return html`<div class="bhero">
      <${Face} shape=${bot.shape} color=${bot.color} size=${136} state=${st} />
      <h2>${bot.name}</h2>
      <p class="bstat"><i class=${`bdot ${st}`}></i>${status}</p>
      <button type="button" class="blink" onClick=${onEdit}>Edit bot</button>
    </div>
    <section class="bsec">
      <header><div class="btabs" role="tablist">${TABS.map(([k, label]) => html`<button type="button" role="tab" key=${k}
          aria-selected=${tab === k} class=${tab === k ? 'on' : ''} onClick=${() => setTab(k)}>${label}${k === 'routines' && list.length ? html` <span class="bn">${list.length}</span>` : null}</button>`)}</div>
      </header>
      ${form || tab !== 'routines' ? null : html`<div class="baddrow"><button type="button" class="blink" onClick=${() => setForm({})}>+ New routine</button></div>`}
      ${tab === 'library' ? html`<${Library} bot=${bot} />` : tab === 'phone' ? html`<${Phone} key=${bot.id} bot=${bot} />` : null}
      ${tab === 'routines' && form && !form.id ? html`<${RoutineForm} bot=${bot} r=${form} onDone=${done} />` : null}
      ${tab !== 'routines' ? null : list.length ? html`<ul class="brout">${list.map((r) => (form && form.id === r.id
          ? html`<li key=${r.id}><${RoutineForm} bot=${bot} r=${r} onDone=${done} /></li>`
          : html`<${Routine} key=${r.id} bot=${bot} r=${r} onEdit=${() => setForm(r)} />`))}</ul>`
        : form ? null : html`<p class="bnote">No routine yet. Tell ${bot.name} in the chat what to do and when, as “every weekday at 8, check my inbox for job offers”: it sets the routine up itself. Or add one here.</p>`}
    </section>`;
}

/* ---------- no bot yet ---------- */
const SAMPLE = [['dome', '#3b82f6', 'watch'], ['cloud', '#ec4899', 'idle'], ['pill', '#4fb86e', 'idle'], ['round', '#8c8c8c', 'sleep'], ['blob', '#f07a3a', 'watch'], ['burst', '#8b5cf6', 'idle']];
function Intro({ onNew }) {
  return html`<div class="bintro">
    <div class="btop"><span>Tirith</span><span>Bots</span><span class="bnum">Agents that keep watch</span></div><div class="brule"></div>
    <h1>Bots that keep watch.</h1>
    <p class="bsub">They do your tasks on a schedule, <b>and only speak up when it matters.</b></p>
    <span class="bdash"></span>
    <p class="blead">Give a bot a name and a face, then tell it what to watch and when: “every weekday at 8, check my inbox for new job offers”. It sets the routine up itself and reports in its chat, here and on your phone.</p>
    <div class="blineup">${SAMPLE.map(([shape, color, state], i) => html`<span class="bpop" key=${shape} style=${`animation-delay:${0.2 + i * 0.12}s`}>
      <${Face} shape=${shape} color=${color} state=${state} size=${[84, 90, 80, 76, 86, 80][i]} /></span>`)}
      <button type="button" class="bpop bfirst" style="animation-delay:.95s" onClick=${onNew} aria-label="Create a bot"><${NewFace} size=${76} /></button></div>
    <button type="button" class="bbtn pri bbig" onClick=${onNew}>Create your first bot</button>
    <div class="bkeys"><div class="bk">Keywords</div>
      <div><h4>Signal</h4><p>Only what concerns you.</p></div>
      <div><h4>Routines</h4><p>Each bot has its tasks and its times.</p></div>
      <div><h4>You decide</h4><p>A bot never acts for you unasked.</p></div></div>
  </div>`;
}

/** `project` is the second part of the address: the id of the bot to show (else the last one shown, else the first). */
export function Bots({ project: wanted }) {
  const s = useStore();
  const [bots, , err] = useFetch('/api/bots', ['bots']);
  const bot = bots && (bots.find((b) => b.id === wanted) || bots.find((b) => b.id === recall()) || bots[0]);
  const talk = bot && S.talks[keyOf(bot)];
  useEffect(() => { if (bot) remember(bot.id); }, [bot && bot.id]);
  /* what the owner has on the screen is read */
  useEffect(() => {
    const read = () => { if (bot && talk && talk.loaded && !document.hidden) markSeen(bot.id, talk.rows); };
    read();
    document.addEventListener('visibilitychange', read);
    return () => document.removeEventListener('visibilitychange', read);
  }, [bot && bot.id, talk && talk.rows.length, talk && talk.loaded]);

  if (!bots) {
    return html`<div class="bview bsolo"><p class="bnote">${err ? (/not found/i.test(err.message)
      ? 'The running Tirith daemon is older than this page and has no bots yet: rebuild and restart it (the page updates on its own).'
      : `Could not read the bots: ${err.message}`) : 'Loading…'}</p></div>`;
  }
  if (!bots.length) return html`<div class="bview bsolo"><${Intro} onNew=${() => setUi('botCard', {})} /></div>`;
  return html`<div class="bview">
    <div class="bchat"><${Chat} key=${keyOf(bot)} project=${keyOf(bot)} who=${bot.name} models=${false}
      hint=${`Say hello to ${bot.name}. Ask for anything that repeats, as “every weekday at 8, check my inbox for job offers”, and ${bot.name} sets up the routine itself.`} /></div>
    <aside class="bside"><${Panel} bot=${bot} busy=${!!s.busy[keyOf(bot)]} onEdit=${() => setUi('botCard', bot)} /></aside>
  </div>`;
}
