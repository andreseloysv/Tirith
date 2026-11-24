// Which roles Settings shows (those with no screen of their own), their labels and notes, and the card's constants.
import { html } from '../../../../vendor/preact-htm.js';
import { t } from '../../../i18n.js';

export const BOLT = html`<svg viewBox="0 0 12 12" width="11" height="11" aria-hidden="true"><path d="M7 1.2L2.6 6.8h2.7L4.8 10.8 9.4 5H6.6z" fill="currentColor"/></svg>`;
export const OTHER = '__other__';
export const source = (k) => ({ page: [t('Saved here'), 'page'], config: [t('From config.toml'), ''], default: [t('Default'), ''] })[k];
/** A role with a screen of its own picks its model there, not in Settings: a bot shown in the bar (scope `own`,
    `show` not false — the judge, `show = false`, has none) on its bot screen, and a project's chat role (the
    Project Manager) in each project's chat. */
export const hasScreen = (r = {}) => (r.scope === 'own' ? r.show !== false : r.scope === 'project' && r.session === 'chat');
/* The roles with no screen of their own (today worker and judge), from /api/models: [id, label, note] with the
   role's factory shape as its note, and its factory `name` — or its id, capitalised — as its label. */
export function roleList(M) {
  const notes = { chat: t('A long-lived conversation'), lane: t('Runs plan items in lanes'), turn: t('One fresh turn when routed') };
  return Object.keys((M && M.roles) || {})
    .filter((id) => !hasScreen(M.roles[id]))
    .map((id) => {
      const r = M.roles[id] || {};
      const note = [notes[r.session] || r.session, r.scope && t(r.scope), r.access === 'read' ? t('read-only') : ''].filter(Boolean).join(' · ');
      return [id, t(r.name || id.charAt(0).toUpperCase() + id.slice(1)), note];
    });
}
