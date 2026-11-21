// Lane statuses and projects' colors.

/** Lane statuses: label and the old page's status class. */
export const LANE = {
  queued: ['Queued', 'queued'],
  running: ['Working', 'working'],
  waiting: ['Waiting for the manager', 'waiting'],
  'merge-queued': ['Waiting to merge', 'queued'],
  merging: ['Checks running', 'working'],
  merged: ['On main', 'done'],
  failed: ['Failed', 'failed'],
  stopped: ['Stopped', 'stopped'],
};
export const laneLabel = (s) => (LANE[s] || [s])[0];
export const laneClass = (s) => (LANE[s] || [s, s])[1];
export const ACTIVE = ['queued', 'running', 'waiting', 'merge-queued', 'merging'];

/** Projects' colors by id. */
export function projectColor(projects, id) {
  return (projects.find((p) => p.id === id) || {}).color || '#9ca3af';
}
