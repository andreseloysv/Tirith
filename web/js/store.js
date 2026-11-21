// The page's data: loaded once from /api/state, then kept current by the daemon's change stream (SSE).
// Components read it with useStore(); views that need other endpoints use useFetch(path, types), which
// refetches when the stream reports a change of one of those types. Repaints are batched per animation frame.
// Every project has its own conversation with its own manager ('' is the conversation of no project): `talks`
// holds the ones the page has opened (useTalk), `busy` the ones whose manager works on a turn.
// Each part lives in its own file under store/; this entry exports what it always did and starts the page's data.
export { S, setUi, useStore, onMessage } from './store/state.js';
export { api, useFetch } from './store/api.js';
export { useTalk, openTalk } from './store/talk.js';
import { boot } from './store/boot.js';

boot();
