// Every area's Spanish words in one book (../i18n.js): each area keeps its own file.
import app from './es/app.js';
import setup from './es/setup.js';
import home from './es/home.js';
import settings from './es/settings.js';
import bots from './es/bots.js';
import chat from './es/chat.js';
import projects from './es/projects.js';
import connectors from './es/connectors.js';
import daemon from './es/daemon.js';
import onboarding from './es/onboarding.js';

export default { ...app, ...setup, ...home, ...settings, ...bots, ...chat, ...projects, ...connectors, ...daemon, ...onboarding };
