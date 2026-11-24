// What the menu offers: only models an agent of config.toml runs (Codex's from its catalog), each with a readable
// name and the efforts it takes, and the effort a newly picked model keeps.
import { ownerOf, PROVIDERS } from './options.js';
import { t } from '../i18n.js';

const LEVELS = ['low', 'medium', 'high', 'xhigh', 'max', 'ultra'];
const NAMES = { low: t('Low'), medium: t('Medium'), high: t('High'), xhigh: t('Extra high'), max: t('Max'), ultra: t('Ultra') };
/** An effort's name on screen ('' when none is set). */
export const effortName = effort => NAMES[effort] || effort || '';
/** A provider's short name: "Codex" for "OpenAI · Codex". */
export const providerName = agent => (PROVIDERS[agent] || agent).split(' · ').pop();

/** A model id as people read it: words capitalized (a short one with no vowel in capitals), the version numbers
 *  at its end joined with dots ("x-pro-5-5" → "X Pro 5.5"). */
export function readable(slug) {
  const parts = slug.split('-');
  let end = parts.length;
  while (end > 1 && /^\d+(\.\d+)?$/.test(parts[end - 1])) end--;
  const word = w => (/^[a-z]{2,3}$/.test(w) && !/[aeiouy]/.test(w) ? w.toUpperCase() : w[0].toUpperCase() + w.slice(1));
  const version = parts.slice(end).join('.');
  return [...parts.slice(0, end).filter(Boolean).map(word), version].filter(Boolean).join(' ');
}

/** Every model that can run: {agent, slug, name, efforts}. A model no existing agent claims is left out. */
export function offered(choices = {}) {
  const agents = choices.agents || [];
  const codex = agents.includes('codex')
    ? (choices.codex || []).map(m => ({ agent: 'codex', slug: m.slug, name: m.name || readable(m.slug), efforts: m.efforts || [] }))
    : [];
  const listed = (choices.models || [])
    .filter(slug => !codex.some(m => m.slug === slug))
    .map(slug => ({ agent: ownerOf(choices, slug), slug, name: readable(slug), efforts: choices.efforts || [] }));
  return [...listed, ...codex].filter(m => agents.includes(m.agent));
}

/** The effort a newly picked model keeps: the same when it takes it, else the nearest level below, else its lowest. */
export function keptEffort(effort, efforts) {
  if (!effort || !efforts.length) return '';
  if (efforts.includes(effort)) return effort;
  const below = efforts.filter(e => LEVELS.indexOf(e) <= LEVELS.indexOf(effort));
  return below.at(-1) || efforts[0];
}
