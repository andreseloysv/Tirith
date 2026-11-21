// The choices behind the controls: provider names, which agent a model belongs to, and what one agent offers.
export const PROVIDERS = { codex: 'OpenAI · Codex', anthropic: 'Anthropic', muse: 'Meta · Muse', copilot: 'GitHub · Copilot' };
/** The agent a listed model belongs to (/api/models `choices.owners`, read from config.toml), or '' when none claims it. */
export const ownerOf = (choices = {}, model = '') => (choices.owners || {})[model] || '';
/** A model of another agent brings its agent along: the patch that picks `model` without pairing it with the wrong agent. */
export const pickModel = (choices, value, model) => {
  const owner = ownerOf(choices, model);
  return owner && owner !== value.agent ? { agent: owner, model, effort: '' } : { model };
};
export function optionsFor(choices, value) {
  const { agent = '', model = '', effort = '' } = value;
  const codex = agent === 'codex';
  const models = codex ? choices.codex || [] : (choices.models || []).map(slug => ({ slug, name: slug }));
  const efforts = codex ? models.find(m => m.slug === model)?.efforts || [] : choices.efforts || [];
  const valid = !codex || ((!model || models.some(m => m.slug === model)) && (!effort || efforts.includes(effort)));
  return { codex, models, efforts, valid };
}
