// Rooms view: the owner's phone first — one QR that always works (it is used once, and the page asks for the next one
// by itself): scanned once, the phone has a chat with the manager of every project — then each project's encrypted
// room on agentsmessenger.com (who else is in it, the last message, Invite another Tirith, Leave), Join with a code
// from another Tirith, and the public rooms Tirith listens to: summary, participants, the last messages with the
// owner's mentions marked, and a box to say something. Tirith makes a project's room itself.
// Data: GET /api/rooms, refetched on SSE "room" / "room-mention", every 15 s and when the window comes back.
// Its parts are in web/js/views/rooms/.
export { Rooms } from './rooms/view.js';
