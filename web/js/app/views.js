// The page's views: [route id, label, component].
import { Overview } from '../views/overview.js';
import { Pipelines } from '../views/pipelines.js';
import { Agents } from '../views/agents.js';
import { Bots } from '../views/bots.js';
import { Rooms } from '../views/rooms.js';
import { Access } from '../views/access.js';
import { Settings } from '../views/settings.js';
import { Connectors } from '../views/connectors.js';
import { t } from '../i18n.js';

/** Views of everything, at the top of the menu (the bots are their faces); views of one project, under it. Access is
    in Settings, behind its button (#/settings/access); a project's own (#/accesos/<project>) is kept for old links. */
export const TOP = [['overview', 'Overview', Overview], ['connectors', 'Connectors', Connectors], ['settings', 'Settings', Settings]];
export const BOTS = ['bots', 'Bots', Bots];
export const SUB = [['pipelines', 'Pipelines', Pipelines], ['agents', 'Agents', Agents], ['rooms', 'Rooms', Rooms]];
export const VIEWS = [...TOP, BOTS, ...SUB, ['accesos', 'Access', Access]];

/** The menu's name of a view, in the chosen language. */
export const labelOf = (label) => ({ Overview: t('Overview'), Connectors: t('Connectors'), Settings: t('Settings'), Bots: t('Bots'),
  Pipelines: t('Pipelines'), Agents: t('Agents'), Rooms: t('Rooms'), Access: t('Access') })[label] || label;

/** The view of a route id (Overview when it is unknown). */
export const viewOf = (id) => VIEWS.find((v) => v[0] === id) || VIEWS[0];
