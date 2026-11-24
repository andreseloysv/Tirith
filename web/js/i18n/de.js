// Every area's German words in one book (../i18n.js): each area keeps its own file.
import app from './de/app.js';
import setup from './de/setup.js';
import home from './de/home.js';
import settings from './de/settings.js';
import bots from './de/bots.js';
import chat from './de/chat.js';
import projects from './de/projects.js';
import connectors from './de/connectors.js';
import daemon from './de/daemon.js';
import onboarding from './de/onboarding.js';

export default { ...app, ...setup, ...home, ...settings, ...bots, ...chat, ...projects, ...connectors, ...daemon, ...onboarding };
