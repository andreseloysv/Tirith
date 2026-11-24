// Who runs a room's project: the bot whose chat it is (its own face), a project's Project Manager (the manager
// role's face), or (no project) the manager of no project. Its face opens that agent's chat.
import { go } from '../../common.js';
import { keyOf, stateOf } from './bits.js';
import { t } from '../../i18n.js';

function botAgent(bot, s) {
  return { shape: bot.shape, color: bot.color, state: stateOf(bot, !!s.busy[keyOf(bot)]), label: t('Chat with {name}', { name: bot.name }), open: () => go('bots', bot.id) };
}

function managerAgent(key, s, M, roleFace) {
  const f = roleFace('manager'), pname = (s.projects.find((p) => p.id === key) || {}).name || key;
  const who = t((M && M.roles.manager && M.roles.manager.name) || 'Project Manager');
  return { shape: f.shape || null, color: f.color || '#8e7cf0', state: s.busy[key] ? 'work' : 'watch', label: t('Chat with the {who} of {name}', { who, name: pname }), open: () => go('overview', key) };
}

/** `agentFor(key)` with the store `s`, the models `M` and the bots. */
export function agentOf(s, M, bots) {
  return (key) => {
    key = key || 'manager-general';
    const bot = (bots || []).find((b) => keyOf(b) === key);
    if (bot) return botAgent(bot, s);
    const roleFace = (role) => (M && M.roles[role] && M.roles[role].face) || {};
    if (s.projects.some((p) => p.id === key)) return managerAgent(key, s, M, roleFace);
    if (key === 'manager-general') {
      const f = roleFace('manager-general');
      return { shape: f.shape || null, color: f.color || '#3fb6a8', state: s.busy[key] ? 'work' : 'watch', label: t('Chat with the Manager'), open: () => go('overview', '') };
    }
    return { shape: null, state: 'watch', label: t('Agent'), open: null };
  };
}
