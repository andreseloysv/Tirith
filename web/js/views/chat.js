// The manager chat: the old page's right column (v-manager.js + livesteps.js), same markup and classes.
// It shows ONE conversation: the selected project's with its own manager, or (no project selected) the one of no
// project, where health reports, the CEO's reviews and questions across projects live. Rows ({id, at, role, text,
// project}) come from useTalk(project); a run of manager/tool rows is one turn of the manager, drawn CLI-style
// (livesteps) with its final text as a markdown bubble. Owner messages may list attached files (the daemon appends
// them to the text); pictures show as thumbnails.
import { html, Component, useState, useEffect, useLayoutEffect, useRef, useMemo } from '../../vendor/preact-htm.js';
import { useStore, useTalk, api } from '../store.js';
import { clockTime, dur, elapsed, Span } from '../common.js';
import { md, esc, boldLead, ctxHtml } from '../md.js';

const SHOWN = 200;        /* messages drawn at first; "Show earlier" adds as many */
const STEPS_LIVE = 8;     /* steps of a running turn on screen */
const DIFF_SHOWN = 20;
const PIC_MAX = 6, FILE_BYTES = 25 * 1024 * 1024;
const PIC_EXT = /\.(png|jpe?g|gif|webp)$/i;
const ATTACHED = '\n\nAttached files (open them with your file-reading tool):\n';
const PROJECT_TAG = /^\[project ([\w.-]+)\] /;   /* rows from before every project had its own conversation */
const DRAFT_KEY = 'tirith.chat.draft';
const MERGED = /^merged on ([\w.-]+) at ([0-9a-f]{6,12}): ([\s\S]*)$/;
const CHECKS = /\s*\(check:\s*`[^`]*`\)/gi;

const SEND = html`<svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true"><path d="M8 13V3.2M3.8 7.2L8 3l4.2 4.2" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
const DOC = html`<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M4 1.8h5l3 3v9.4H4z M9 1.8v3h3" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/></svg>`;
const CLIP = html`<svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true"><path d="M13.2 7.6L8 12.8a3 3 0 01-4.2-4.2l5.5-5.5a2 2 0 012.9 2.9L6.7 11.4a1 1 0 01-1.4-1.4l4.8-4.8" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>`;

/** A component that repaints only when `same(old props, new props)` says they differ. */
function pure(Inner, same) {
  return class extends Component {
    shouldComponentUpdate(next) { return !same(this.props, next); }
    render(props) { return html`<${Inner} ...${props} />`; }
  };
}

function store(key, val) {
  try {
    if (val === undefined) return localStorage.getItem(key) || '';
    if (val) localStorage.setItem(key, val); else localStorage.removeItem(key);
  } catch { /* private window: the draft just does not survive */ }
  return '';
}
function size(n) { return n < 1024 ? n + ' B' : n < 1048576 ? Math.round(n / 1024) + ' KB' : (n / 1048576).toFixed(1) + ' MB'; }
const projName = (projects, id) => (projects.find((p) => p.id === id) || {}).name || id;

/* ---------- the owner's text: project tag and attached files ---------- */
/** {text, project, files: [{path, name, url, pic}]} of an owner message as the daemon stored it. */
function ownerParts(raw) {
  let text = String(raw || ''), project = '', files = [];
  const at = text.indexOf(ATTACHED);
  if (at >= 0) {
    files = text.slice(at + ATTACHED.length).split('\n').map((l) => l.replace(/^- /, '').trim()).filter(Boolean).map(fileOf);
    text = text.slice(0, at);
  }
  const m = PROJECT_TAG.exec(text);
  if (m) { project = m[1]; text = text.slice(m[0].length); }
  return { text, project, files };
}
/** …/attachments/<id>/<name> -> its URL on the daemon. */
function fileOf(path) {
  const m = /\/attachments\/([0-9a-f]+)\/([^/]+)$/.exec(path), name = path.split('/').pop();
  return { path, name, url: m ? `/api/attachments/${m[1]}/${encodeURIComponent(m[2])}` : '', pic: !!m && PIC_EXT.test(name) };
}

/* ---------- grouping: messages and manager turns ---------- */
/** [{kind: 'msg', m} | {kind: 'turn', key, rows}] in order; a turn is a run of manager / tool messages. */
function group(rows) {
  const out = [];
  let turn = null;
  for (const m of rows) {
    if (m.role === 'manager' || m.role === 'tool') {
      if (!turn) { turn = { kind: 'turn', key: 't' + m.id, rows: [] }; out.push(turn); }
      turn.rows.push(m);
    } else {
      turn = null;
      out.push({ kind: 'msg', key: 'm' + m.id, m });
    }
  }
  return out;
}

/** The steps of a turn and its answer: the manager's texts after the last tool call (only once it ended). */
function splitTurn(rows, running) {
  let last = -1;
  rows.forEach((r, i) => { if (r.role === 'tool') last = i; });
  if (running) return { steps: rows, answer: [] };
  return { steps: rows.slice(0, last + 1), answer: rows.slice(last + 1) };
}

/* ---------- one tool call, CLI style ---------- */
/** "Read: docs/TODO.md" -> "Read docs/TODO.md"; "Grep: x" -> 'Grep "x"'; others stay as the daemon wrote them. */
function toolLine(text) {
  const m = /^(\w+): ([\s\S]*)$/.exec(text);
  if (!m) return text;
  const [, name, arg] = m;
  if (/^(Read|Edit|MultiEdit|Write|NotebookEdit|Glob|LS|WebFetch)$/.test(name)) return `${name} ${arg}`;
  if (name === 'Grep') return `Grep "${arg}"`;
  return text;
}
/** A tool text whose later lines are +/- lines: the diff rows, else null. */
function diffOf(text) {
  const lines = String(text).split('\n');
  if (lines.length < 2) return null;
  const rows = lines.slice(1).filter((l) => l !== '');
  if (!rows.length || !rows.every((l) => /^[+-]/.test(l))) return null;
  const path = (/^\w+:? (\S+)/.exec(lines[0]) || [])[1] || '';
  return { head: lines[0], path, rows: rows.map((l) => [l[0], l.slice(1).replace(/^ /, '')]) };
}

function Diff({ d }) {
  const [open, setOpen] = useState(false);
  const shown = open ? d.rows : d.rows.slice(0, DIFF_SHOWN);
  const adds = d.rows.filter((r) => r[0] === '+').length, dels = d.rows.length - adds;
  return html`<div class="ls-diff"><div class="ls-dh">${d.path} <span class="ls-dn">+${adds} −${dels}</span></div>
    ${shown.map((r) => html`<div class=${`ls-dl ${r[0] === '+' ? 'add' : 'del'}`}><span class="ls-sg">${r[0] === '+' ? '+' : '−'}</span>${r[1]}</div>`)}
    ${d.rows.length > DIFF_SHOWN ? html`<button type="button" class="ls-more" onClick=${() => setOpen(!open)}>${open ? 'collapse' : `show all ${d.rows.length} lines`}</button>` : null}</div>`;
}

function Step({ r, typing }) {
  if (r.role === 'tool') {
    const d = diffOf(r.text);
    return html`<li class="ls-tool"><span class="ls-dot">●</span><span class="ls-t">${toolLine(d ? d.head : r.text)}${d ? html`<${Diff} d=${d} />` : null}</span></li>`;
  }
  return html`<li class=${`ls-say${typing ? ' typing' : ''}`}><span class="ls-t">${r.text}${typing ? html`<span class="ls-cur" aria-hidden="true">▍</span>` : null}</span></li>`;
}

function Working({ start, onStop }) {
  return html`<div class="ls-line" role="status"><span class="ls-spin" aria-hidden="true"></span>Manager is working · <${Span} start=${start} />
    <button type="button" class="ls-stop" onClick=${onStop} title="Stop this turn (Esc)">Stop</button></div>`;
}

/* ---------- messages ---------- */
const mdCache = new Map();   /* message id -> HTML (texts never change) */
function mdOf(key, text) {
  let h = mdCache.get(key);
  if (h == null) {
    h = md(boldLead(text));
    mdCache.set(key, h);
    if (mdCache.size > 800) mdCache.delete(mdCache.keys().next().value);
  }
  return h;
}

function Pics({ files, onZoom }) {
  if (!files || !files.length) return null;
  const pics = files.filter((f) => f.pic), rest = files.filter((f) => !f.pic);
  return html`<div class="mc-pics">
    ${pics.map((f, i) => html`<button class="mc-pic" type="button" title="Open full size" aria-label=${`Image ${i + 1}`} onClick=${() => onZoom(f.url)}>
      <img src=${f.url} alt=${`Image ${i + 1}`} loading="lazy" /></button>`)}
    ${rest.map((f) => html`<${FileChip} name=${f.name} title=${f.path} href=${f.url} />`)}</div>`;
}

function FileChip({ name, bytes, title, href, onRemove }) {
  const inner = html`${DOC}<span class="mc-fname">${name}</span>${bytes != null ? html`<span class="mc-fsize">${size(bytes)}</span>` : null}
    ${onRemove ? html`<button type="button" aria-label=${`Remove ${name}`} title="Remove" onClick=${onRemove}>×</button>` : null}`;
  return href ? html`<a class="mc-file" title=${title} href=${href} target="_blank" rel="noopener">${inner}</a>`
    : html`<span class="mc-file" title=${title}>${inner}</span>`;
}

/** One stored message that is not part of a manager turn (`project`: the conversation it is drawn in). */
const Msg = pure(({ m, project, projects, onZoom }) => {
  const time = clockTime(m.at);
  if (m.role === 'owner') {
    const p = ownerParts(m.text);
    const ctx = p.project ? `on ${projName(projects, p.project)}` : '';
    return html`<div class="mc-m me"><${Pics} files=${p.files} onZoom=${onZoom} />
      ${p.text ? html`<div class="mc-b">${p.text}</div>` : null}
      <div class="mc-meta" dangerouslySetInnerHTML=${{ __html: (ctx ? ctxHtml(ctx) + ' · ' : '') + esc(time) }} /></div>`;
  }
  let text = m.text, ctx = 'Tirith', cls = 'mgr note';
  const merged = m.role === 'tirith' && MERGED.exec(text);
  /* a merge reads as one line; in the conversation of no project (rows from before the split) it names its project */
  if (merged) text = `On main${merged[1] === project ? '' : ` of ${projName(projects, merged[1])}`} (\`${merged[2]}\`): ${merged[3].replace(CHECKS, '')}`;
  if (m.role === 'room') {
    const r = /^(.+?) \((.+?)\): ([\s\S]*)$/.exec(text);
    if (r) { ctx = `${r[1]} · room ${r[2]}`; text = r[3]; } else ctx = 'Room';
    cls = 'mgr room';
  } else if (m.role === 'ceo') { ctx = 'CEO review'; cls = 'mgr ceo'; }
  else if (m.role !== 'tirith') { ctx = m.role; }
  return html`<div class=${`mc-m ${cls}`}><div class="mc-b md" dangerouslySetInnerHTML=${{ __html: mdOf(m.id, text) }} />
    <div class="mc-meta" dangerouslySetInnerHTML=${{ __html: ctxHtml(ctx) + ' · ' + esc(time) }} /></div>`;
}, (a, b) => a.m === b.m && a.projects === b.projects);

/** One turn of the manager: its steps (live while running, a summary line once ended) and its answer. */
const Turn = pure(({ rows, running, start, onStop }) => {
  const [open, setOpen] = useState(false);
  const { steps, answer } = splitTurn(rows, running);
  let block = null;
  if (running) {
    const tail = steps.slice(-STEPS_LIVE), last = steps[steps.length - 1];
    block = html`<div class="livesteps run">
      ${tail.length ? html`<ul class="ls-list" aria-live="polite">${tail.map((r) => html`<${Step} key=${r.id} r=${r} typing=${r === last && r.role === 'manager'} />`)}</ul>` : null}
      <${Working} start=${start} onStop=${onStop} /></div>`;
  } else if (steps.length) {
    const end = answer.length ? answer[answer.length - 1].at : steps[steps.length - 1].at;
    const total = elapsed(start || steps[0].at, end);
    block = html`<div class=${`livesteps done${open ? ' open' : ''}`}>
      <button type="button" class="ls-sum" aria-expanded=${open} onClick=${() => setOpen(!open)}>
        ${steps.length} ${steps.length === 1 ? 'step' : 'steps'} · ${dur(total)} · <span class="ls-show">${open ? 'hide' : 'show'}</span></button>
      ${open ? html`<ul class="ls-list">${steps.map((r) => html`<${Step} key=${r.id} r=${r} />`)}</ul>` : null}</div>`;
  }
  return html`${block}${answer.map((r) => html`<div class="mc-m mgr" key=${r.id}><div class="mc-b md" dangerouslySetInnerHTML=${{ __html: mdOf(r.id, r.text) }} />
    <div class="mc-meta" dangerouslySetInnerHTML=${{ __html: ctxHtml('Manager') + ' · ' + esc(clockTime(r.at)) }} /></div>`)}`;
}, (a, b) => a.k === b.k && a.n === b.n && a.running === b.running && a.start === b.start);

/** A message sent from this page that the daemon has not shown yet (sending) or refused (failed). */
function Local({ o, onRetry, onCopy }) {
  const files = o.pics.map((p) => ({ name: p.name, url: p.url, pic: !!p.url, path: p.name }));
  return html`<div class=${`mc-m me local ${o.state}`}>
    ${files.length ? html`<div class="mc-pics">${files.map((f) => (f.pic
      ? html`<span class="mc-pic"><img src=${f.url} alt=${f.name} /></span>`
      : html`<${FileChip} name=${f.name} title=${f.name} />`))}</div>` : null}
    ${o.text ? html`<div class="mc-b">${o.text}</div>` : null}
    <div class="mc-meta">${o.state === 'failed'
      ? html`<b class="mc-bad">Not sent</b> · <button type="button" class="mc-lnk" onClick=${onRetry}>Retry</button> · <button type="button" class="mc-lnk" onClick=${onCopy}>Copy text</button>${o.err ? html`<br /><span class="mc-why">${o.err}</span>` : null}`
      : html`Sending… · ${clockTime(new Date(o.at).toISOString())}`}</div></div>`;
}

/* ---------- the chat ---------- */
let cidN = 0;

/** The conversation of `project` ('' or none: the one of no project). Mount it with key=project, so the draft, the
    files being attached and the scroll position belong to one conversation. */
export function Chat({ project = '' }) {
  const s = useStore();
  const talk = useTalk(project);
  const name = projName(s.projects, project);
  const draftKey = project ? `${DRAFT_KEY}:${project}` : DRAFT_KEY;
  const [shown, setShown] = useState(SHOWN);
  const [draft, setDraft] = useState(() => store(draftKey));
  const [pics, setPics] = useState([]);       /* {file, name, type, size, url (pictures only)} */
  const [out, setOut] = useState([]);         /* local sends: {cid, text, body, project, pics, state, err, at, after} */
  const [err, setErr] = useState(null);
  const [drag, setDrag] = useState(false);
  const [zoom, setZoom] = useState(null);
  const anchor = useRef(null), log = useRef(null), ta = useRef(null), stick = useRef(true), file = useRef(null), lastSent = useRef('');

  const chat = talk.rows, lastId = chat.length ? chat[chat.length - 1].id : 0;
  const busy = !!s.busy[project];
  const items = useMemo(() => group(chat.slice(-shown)), [chat.length, lastId, shown]);
  const hidden = Math.max(0, chat.length - shown);

  /* a local send leaves once the daemon shows an owner message after it with its text */
  useEffect(() => {
    const seen = out.filter((o) => o.state === 'sending' && o.done && chat.some((m) => m.id > o.after && m.role === 'owner' && m.text.startsWith(o.text)));
    if (seen.length) setOut((xs) => xs.filter((o) => !seen.includes(o)));
  }, [lastId, out]);

  /* keep the newest message in view, but only when the list was already at its end */
  useLayoutEffect(() => {
    const el = log.current;
    if (el && stick.current) el.scrollTop = el.scrollHeight;
    else if (el && anchor.current != null) { el.scrollTop = el.scrollHeight - anchor.current; anchor.current = null; }
  });
  const onScroll = () => {
    const el = log.current; /* null once this conversation left the page: a late scroll event of its list */
    if (el) stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < 48;
  };

  /* the box grows with its text up to half the window, then scrolls inside */
  const fit = () => {
    const t = ta.current;
    if (!t) return;
    t.style.height = 'auto';
    const cap = Math.floor(innerHeight * 0.5);
    t.style.height = Math.min(t.scrollHeight, cap) + 'px';
    t.style.overflowY = t.scrollHeight > cap ? 'auto' : 'hidden';
    const el = log.current;
    if (el && stick.current) el.scrollTop = el.scrollHeight;
  };
  useLayoutEffect(fit, [draft]);
  useEffect(() => { addEventListener('resize', fit); return () => removeEventListener('resize', fit); }, []);

  const edit = (v) => { setDraft(v); store(draftKey, v); };

  const interrupt = () => {
    api('POST', '/api/manager/interrupt', { project }).then(() => setErr(null), (e) => setErr(`Could not stop: ${e.message}`));
  };
  /* Esc anywhere outside a field stops the running turn, like the CLI */
  useEffect(() => {
    const key = (e) => {
      if (e.key !== 'Escape' || e.defaultPrevented || !busy || zoom) return;
      const tag = e.target && e.target.tagName;
      if (tag === 'TEXTAREA' || tag === 'INPUT' || tag === 'SELECT') return;
      e.preventDefault(); interrupt();
    };
    addEventListener('keydown', key);
    return () => removeEventListener('keydown', key);
  }, [busy, zoom]);

  /* ---------- attachments ---------- */
  const addFiles = (list) => {
    const files = Array.from(list || []);
    if (!files.length) return;
    let bad = null;
    const ok = files.filter((f) => {
      if (!f.size) { bad = `${f.name || 'The file'} is empty (a folder cannot be attached).`; return false; }
      if (f.size > FILE_BYTES) { bad = `${f.name} is over 25 MB.`; return false; }
      return true;
    });
    setPics((cur) => {
      const room = PIC_MAX - cur.length;
      if (ok.length > room) bad = `At most ${PIC_MAX} files per message.`;
      const add = ok.slice(0, Math.max(0, room)).map((f) => ({
        file: f, name: f.name || 'pasted.png', type: f.type, size: f.size,
        url: /^image\/(png|jpeg|gif|webp)$/.test(f.type) ? URL.createObjectURL(f) : null,
      }));
      return cur.concat(add);
    });
    setErr(bad);
  };
  const unpic = (i) => setPics((cur) => cur.filter((_, k) => k !== i));

  /* files dragged over the chat: the drop target; dropped beside it they do nothing (no navigation) */
  useEffect(() => {
    let off = 0;
    const has = (e) => !!(e.dataTransfer && Array.from(e.dataTransfer.types || []).includes('Files'));
    const over = (e) => {
      if (!has(e)) return;
      e.preventDefault();
      clearTimeout(off);
      const on = !!(e.target.closest && e.target.closest('.mchat'));
      e.dataTransfer.dropEffect = on ? 'copy' : 'none';
      setDrag(on);
    };
    const leave = () => { clearTimeout(off); off = setTimeout(() => setDrag(false), 100); };
    const drop = (e) => {
      if (!has(e)) return;
      e.preventDefault();
      setDrag(false);
      if (e.target.closest && e.target.closest('.mchat')) addFiles(e.dataTransfer.files);
    };
    document.addEventListener('dragover', over);
    document.addEventListener('dragleave', leave);
    document.addEventListener('drop', drop);
    return () => { document.removeEventListener('dragover', over); document.removeEventListener('dragleave', leave); document.removeEventListener('drop', drop); };
  }, []);

  /* ---------- sending: each file first (POST /api/attachments), then the message ---------- */
  const transmit = async (o) => {
    for (const p of o.pics) {
      if (p.saved) continue;
      const r = await fetch(`/api/attachments?name=${encodeURIComponent(p.name)}`, { method: 'POST', body: p.file });
      const t = await r.text();
      if (!r.ok) throw new Error(`${p.name}: ${t || r.statusText}`);
      p.saved = JSON.parse(t).path;
    }
    await api('POST', '/api/chat', { text: o.text, project, attachments: o.pics.map((p) => p.saved) });
  };
  const dispatch = (o) => {
    o.state = 'sending'; o.err = null; o.done = false; o.at = Date.now();
    o.after = chat.length ? chat[chat.length - 1].id : 0;
    stick.current = true;
    setOut((xs) => (xs.includes(o) ? xs.slice() : xs.concat(o)));
    transmit(o).then(() => { o.done = true; setOut((xs) => xs.slice()); }, (e) => {
      o.state = 'failed'; o.err = `Could not send: ${e.message || e}`;
      setOut((xs) => xs.slice());
      /* the text goes back into an empty box, like the old page */
      if (!ta.current || !ta.current.value.trim()) edit(o.text);
    });
  };
  const send = () => {
    const text = draft.trim();
    if (!text && !pics.length) return;
    /* a conversation that could not be read (a daemon older than this page) would swallow the message */
    if (talk.error && !talk.loaded) { setErr('This conversation could not be read, so nothing was sent.'); return; }
    lastSent.current = text;
    dispatch({ cid: ++cidN, text, pics: pics.slice() });
    edit(''); setPics([]); setErr(null);
  };
  const retry = (o) => {
    if (draft === o.text) edit('');
    dispatch(o);
  };
  const copy = (o) => { try { navigator.clipboard.writeText(o.text).catch(() => {}); } catch { /* the text is in the box too */ } };

  const lastOwnerText = () => {
    if (lastSent.current) return lastSent.current;
    for (let i = chat.length - 1; i >= 0; i--) if (chat[i].role === 'owner') return ownerParts(chat[i].text).text;
    return '';
  };
  const onKey = (e) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); send(); }
    else if (e.key === 'Escape' && busy) { e.preventDefault(); e.stopPropagation(); interrupt(); }
    else if (e.key === 'ArrowUp' && !e.target.value && !pics.length && lastOwnerText()) { e.preventDefault(); edit(lastOwnerText()); }
  };
  const onPaste = (e) => {
    const files = (e.clipboardData && e.clipboardData.files) || [];
    if (files.length) { e.preventDefault(); addFiles(files); }
  };

  /* ---------- the thread ---------- */
  const lastItem = items[items.length - 1];
  const runningKey = busy && lastItem && lastItem.kind === 'turn' ? lastItem.key : null;
  /* start of the running turn: the message that started it */
  const trigger = (i) => { for (let k = i - 1; k >= 0; k--) if (items[k].kind === 'msg') return items[k].m.at; return null; };
  const rows = items.map((it, i) => (it.kind === 'msg'
    ? html`<${Msg} key=${it.key} m=${it.m} project=${project} projects=${s.projects} onZoom=${setZoom} />`
    : html`<${Turn} key=${it.key} k=${it.key} rows=${it.rows} n=${it.rows.length} running=${it.key === runningKey}
        start=${trigger(i) || it.rows[0].at} onStop=${interrupt} />`));
  /* busy but no output yet: the working line under the newest message */
  const pending = busy && !runningKey && lastItem
    ? html`<div class="livesteps run"><${Working} start=${lastItem.kind === 'msg' ? lastItem.m.at : null} onStop=${interrupt} /></div>` : null;

  return html`<section class=${`mchat${drag ? ' drop' : ''}`}>
    <div class="mc-head"><h2>Manager <span class="mc-of">${project ? name : 'all projects'}</span></h2>
      ${busy ? html`<span class="mc-presence work" role="status">Working…</span>
        <button class="mc-stop" type="button" onClick=${interrupt} title="Stop the current turn (Esc)">Stop</button>` : null}</div>
    <div class="mc-log" id="mc-log" ref=${log} onScroll=${onScroll}>
      ${hidden ? html`<button type="button" class="mc-earlier" onClick=${() => { const el = log.current; stick.current = false; anchor.current = el.scrollHeight - el.scrollTop; setShown(shown + SHOWN); }}>Show earlier (${hidden})</button>` : null}
      ${talk.error && !talk.loaded ? html`<div class="ierr mc-note">${/not found|not allowed/i.test(talk.error.message)
          ? 'The running Tirith daemon is older than this page: quit Tirith, stop the daemon (pkill -f "tirith serve") and open Tirith again.'
          : `Could not read this conversation: ${talk.error.message}`}</div>` : null}
      ${rows.length || out.length ? rows : !talk.loaded ? null : html`<p class="quiet">${project
        ? `Write to the manager of ${name} here. It works on this project only: its answers, its agents' questions and what lands on main show in this thread.`
        : 'Write here about anything that is not one project\'s: this manager gets the health reports and the CEO\'s reviews. Every project has its own manager: select a project to talk to it.'}</p>`}
      ${pending}
      ${out.map((o) => html`<${Local} key=${o.cid} o=${o} onRetry=${() => retry(o)} onCopy=${() => copy(o)} />`)}
    </div>
    <div class="mc-comp">
      ${err ? html`<div class="ierr mc-note">${err}</div>` : null}
      <div class="mc-box">
        ${pics.length ? html`<div class="mc-strip">${pics.map((p, i) => (p.url
          ? html`<span class="mc-th" title=${p.name}><span class="mc-thi"><img src=${p.url} alt=${p.name} />
              <button type="button" aria-label=${`Remove ${p.name}`} title="Remove" onClick=${() => unpic(i)}>×</button></span>
              <span class="mc-thn">${p.name}</span><span class="mc-ths">${size(p.size)}</span></span>`
          : html`<${FileChip} name=${p.name} bytes=${p.size} title=${p.name} onRemove=${() => unpic(i)} />`))}</div>` : null}
        <textarea ref=${ta} rows="1" maxlength="4000" placeholder=${project ? `Message the manager of ${name}…` : 'Message the manager…'} aria-label="Message to the manager"
          value=${draft} onInput=${(e) => edit(e.target.value)} onKeyDown=${onKey} onPaste=${onPaste}></textarea>
        <div class="mc-row">
          <button class="mc-clip" type="button" aria-label="Attach files" title="Attach images or files (or paste, or drop them here)" onClick=${() => file.current.click()}>${CLIP}</button>
          <input type="file" ref=${file} multiple hidden onChange=${(e) => { addFiles(e.target.files); e.target.value = ''; }} />
          <span class="mc-hint">Enter sends · Shift+Enter new line</span>
          <button class="mc-send" type="button" aria-label="Send" title="Send to the manager" onClick=${send}>${SEND}</button>
        </div>
      </div>
    </div>
    ${zoom ? html`<${Lightbox} src=${zoom} onClose=${() => setZoom(null)} />` : null}
  </section>`;
}

/** Full size: one overlay, closed by a click or Esc. */
function Lightbox({ src, onClose }) {
  const btn = useRef(null);
  useEffect(() => {
    const key = (e) => { if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); onClose(); } };
    document.addEventListener('keydown', key, true);
    if (btn.current) btn.current.focus();
    return () => document.removeEventListener('keydown', key, true);
  }, []);
  return html`<div class="mc-lb" role="dialog" aria-label="Image" onClick=${onClose}>
    <img src=${src} alt="Attached image" /><button ref=${btn} type="button" class="mc-lbx" aria-label="Close">×</button></div>`;
}
