// The page's views: [route id, label, component].
import { Overview } from '../views/overview.js';
import { Pipelines } from '../views/pipelines.js';
import { Agents } from '../views/agents.js';
import { Bots } from '../views/bots.js';
import { Rooms } from '../views/rooms.js';
import { Access } from '../views/access.js';
import { Settings } from '../views/settings.js';
import { Connectors } from '../views/connectors.js';

/** Views of everything, at the top of the menu (the bots are their faces); views of one project, under it. */
export const TOP = [['overview', 'Overview', Overview], ['connectors', 'Connectors', Connectors], ['settings', 'Settings', Settings]];
export const BOTS = ['bots', 'Bots', Bots];
export const SUB = [['pipelines', 'Pipelines', Pipelines], ['agents', 'Agents', Agents], ['rooms', 'Rooms', Rooms], ['accesos', 'Access', Access]];
export const VIEWS = [...TOP, BOTS, ...SUB];

/** The view of a route id (Overview when it is unknown). */
export const viewOf = (id) => VIEWS.find((v) => v[0] === id) || VIEWS[0];
