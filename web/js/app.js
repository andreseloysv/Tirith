// The shell: ONE sidebar, the header (crumb, search, health) and the current view. The sidebar is one surface: at its far
// left the bots' big faces (each in its state, with its unread answers; the "+" adds a bot), and next to them the menu:
// Overview, Settings and the projects, folded. A face opens its bot (#/bots/<id>); a project opens into its own Pipelines,
// Agents, Rooms and Access (#/<view>/<project>), and its name opens its overview (plan and manager). Which projects are
// open is remembered, and so is folding one: the project you are in opens on arrival unless you folded it, and its arrow
// or its name folds it again while you stay there. The bot's card (new or edit) is drawn here, over any view.
// Each part lives in its own file under app/; this entry only draws the shell.
import { html, render } from '../vendor/preact-htm.js';
import { App } from './app/shell.js';
import './app/reload-key.js';

render(html`<${App} />`, document.getElementById('app'));
