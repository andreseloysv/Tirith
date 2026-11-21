// The manager chat. It shows ONE conversation: the selected project's with its own manager, a bot's, or (no project
// selected) the one of no project, where health reports, the CEO's passes and questions across projects live. Rows
// ({id, at, role, text, project}) come from useTalk(project); a run of manager/tool rows is one turn. The owner never sees
// its commands: while it runs, one tiny line says in plain words what it is doing (intent.js); once it ended, only its
// answer shows. An answer that starts with ∅ (nothing new) is a quiet line. Owner messages may list attached files
// (the daemon appends them to the text); pictures show as thumbnails.
//
// The parts live in chat/: chat.js (the conversation), speaker.js (who speaks), turns.js (grouping), msg.js and
// turn.js (the rows), thread.js, head.js, composer.js and menu.js (what is drawn), scroll.js, attach.js, send.js
// and keys.js (what it does).
export { Chat } from './chat/chat.js';
export { quiet, shownAnswers } from './chat/turns.js';
