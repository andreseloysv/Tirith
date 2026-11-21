// A lane's log, readable: the tail of its stream-json (or plain) output parsed into entries — the agent's text,
// commands, edits, reads, test runs, errors, results — newest first, one look per kind (port of the old
// logview.js; same classes: .logv .lchips .llist .le). Its parts are in web/js/views/boards/logview/.
export { parseLog } from './logview/parse.js';
export { LogView } from './logview/view.js';
