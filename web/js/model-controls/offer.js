// What the controls offer: only models an agent of config.toml runs (Codex's from its catalog), each with the
// efforts it takes, and the effort a newly picked model keeps.
import { ownerOf } from './options.js';

const LEVELS = ['low', 'medium', 'high', 'xhigh', 'max', 'ultra'];
const NAMES = { low: 'Low', medium: 'Medium', high: 'High', xhigh: 'X-High', max: 'Max', ultra: 'Ultra' };
/** An effort's name on screen ('' when none is set). */
export const effortName = effort => NAMES[effort] || effort || '';

/** Every model that can run: {agent, slug, name, efforts}. A model no existing agent claims is left out. */
export function offered(choices = {}) {
  const agents = choices.agents || [];
  const codex = agents.includes('codex')
    ? (choices.codex || []).map(m => ({ agent: 'codex', slug: m.slug, name: m.name || m.slug, efforts: m.efforts || [] }))
    : [];
  const listed = (choices.models || [])
    .filter(slug => !codex.some(m => m.slug === slug))
    .map(slug => ({ agent: ownerOf(choices, slug), slug, name: slug, efforts: choices.efforts || [] }));
  return [...listed, ...codex].filter(m => agents.includes(m.agent));
}

/** The effort a newly picked model keeps: the same when it takes it, else the nearest level below, else its lowest. */
export function keptEffort(effort, efforts) {
  if (!effort || !efforts.length) return '';
  if (efforts.includes(effort)) return effort;
  const below = efforts.filter(e => LEVELS.indexOf(e) <= LEVELS.indexOf(effort));
  return below.at(-1) || efforts[0];
}
