// The conversation of one project, one bot, or of no project: head, thread, model picker and the box.
import { html, useState, useRef, useMemo } from '../../../vendor/preact-htm.js';
import { ChatModels } from '../chat-models.js';
import { store } from './util.js';
import { group } from './turns.js';
import { useSpeaker } from './speaker.js';
import { useScroll, useFit } from './scroll.js';
import { useAttach } from './attach.js';
import { useSend } from './send.js';
import { useStop, boxKeys } from './keys.js';
import { head } from './head.js';
import { thread } from './thread.js';
import { composer } from './composer.js';
import { Lightbox } from './lightbox.js';

const SHOWN = 200;        /* messages drawn at first; "Show earlier" adds as many */
const DRAFT_KEY = 'tirith.chat.draft';

/** The conversation of `project` ('' or none: the one of no project). Mount it with key=project, so the draft, the
    files being attached and the scroll position belong to one conversation. A bot's conversation (`bot-<id>`, or a
    system bot's key) passes `who` (the bot's name) and `hint` (what an empty conversation says). Who speaks here —
    a project's Project Manager, or the bot — has its face and name in the head and beside its messages, and the
    model picker under the thread chooses only that one's model: a project's chat picks the Project Manager's
    (for every project), a bot's screen its bot's. The face and name come from /api/models (the factory role). */
export function Chat({ project = '', who = '', hint = '' }) {
  project = project || 'manager-general';   /* no project: the manager of no project's conversation */
  const sp = useSpeaker(project, who);
  const { talk, busy, isProject, whoName, role } = sp;
  const draftKey = project ? `${DRAFT_KEY}:${project}` : DRAFT_KEY;
  const [shown, setShown] = useState(SHOWN);
  const [draft, setDraft] = useState(() => store(draftKey));
  const [err, setErr] = useState(null);
  const [zoom, setZoom] = useState(null);
  const ta = useRef(null), file = useRef(null);
  const att = useAttach(setErr);
  const chat = talk.rows, lastId = chat.length ? chat[chat.length - 1].id : 0;
  const items = useMemo(() => group(chat.slice(-shown)), [chat.length, lastId, shown]);
  const hidden = Math.max(0, chat.length - shown);
  const edit = (v) => { setDraft(v); store(draftKey, v); };
  const scroll = useScroll();
  const send = useSend({ project, talk, chat, lastId, draft, edit, pics: att.pics, setPics: att.setPics, setErr, ta, stick: scroll.stick });
  useFit(ta, scroll.log, scroll.stick, draft);
  const interrupt = useStop(project, busy, zoom, setErr);
  const keys = boxKeys({ chat, busy, pics: att.pics, lastSent: send.lastSent, send: send.send, interrupt, edit, addFiles: att.addFiles });
  const onEarlier = () => scroll.earlier(() => setShown(shown + SHOWN));

  return html`<section class=${`mchat${att.drag ? ' drop' : ''}`}>
    ${head(sp, project, interrupt)}
    ${thread({ sp, project, hint, items, hidden, scroll, onEarlier, setZoom, send })}
    <${ChatModels} role=${role} label=${whoName} note=${isProject ? `The model of the ${whoName}: it applies to all projects.` : `The model of ${whoName}.`} />
    ${composer({ sp, err, att, ta, file, draft, edit, keys, send: send.send })}
    ${zoom ? html`<${Lightbox} src=${zoom} onClose=${() => setZoom(null)} />` : null}
  </section>`;
}
