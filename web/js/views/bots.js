// Bots view: one bot (#/bots/<id>, chosen by its face in the sidebar): its conversation (the manager chat on the bot's
// conversation `bot-<id>`) and its panel, in the content area like any other view: the face (asleep, on watch or
// working) and three tabs. Routines: each with a switch, Run now, Edit and Delete, and a form for a new one. Library: what
// the bot learned, its memory.md and its skills (its own and the shared ones of every bot), to read, correct or delete.
// Números (numbers.js): its runs, cost, scores, skills and memory in short lines.
// Phone: one QR that opens the bot's chat on the owner's phone. The sidebar's "+" and "Edit" open the bot's card (BotCard,
// drawn by the app over any view): name, shape, colour, its model (the managers' unless it has one of its own) and its
// factory shape — the events it hears (chosen from the closed list, each as a turn or a note, with its chat line and
// phones), its session and its access: all that makes a bot a CEO or a judge is that configuration, for a system bot
// exactly as for one of the owner's, saved in config.toml where its routes replace the factory's for it.
//
// The parts live in bots/: view.js (the view), panel.js, routines.js, library.js, skills.js, phone.js and intro.js
// (what it shows), card.js, card-model.js, card-factory.js and
// card-connectors.js (the card; Permissions: the connectors it is granted and their limits), templates.js («Desde plantilla»: a bot from
// one of the factory's templates), when.js and state.js (what it computes).
export { Bots } from './bots/view.js';
export { BotCard } from './bots/card.js';
export { BotTemplates } from './bots/templates.js';
export { keyOf, recall, stateOf } from './bots/state.js';
export { daysLabel, timeLabel, nextRun } from './bots/when.js';
