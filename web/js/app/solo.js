// A home with no project has only the owner's bots: the factory's own (the Manager, the CEO, the Coach), its health
// and its roles' models stay off the page until there is something to build. The daemon runs none of it either
// while there is no project (rust/tirith/src/metrics/mod.rs, `asleep`).

/** `s`: the store. */
export const solo = (s) => !s.projects.length;

/** The bots the page shows: those not hidden and, while there is no project, only the owner's. */
export const shownBots = (bots, s) => bots.filter((b) => b.show !== false && !(solo(s) && b.system));
