// Settings, models per role (the old v-models.js): agent, model, effort and the fallback chain of the roles
// with no screen of their own. Save: POST /api/models/{role}; back to config.toml: DELETE. Its parts are in
// web/js/views/settings/models/.
export { hasScreen, roleList } from './models/roles.js';
export { RoleCard } from './models/card.js';
