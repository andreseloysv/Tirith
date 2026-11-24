// What an agent is doing, in a few plain words, from its latest step: the owner never sees the command itself.
// A step is a chat row ({role, text}: 'tool' rows read "Tool: detail" or the command an exec agent ran, 'manager'
// rows are its own words) or a hook event's summary. Words of the agent's own are used when they are short and free of code.

import { t } from './i18n.js';
const RULES = [
  [/^(Skill|read_skill|use_skill)\b/i, () => t('Using a skill')],
  [/\btirith routine\b/i, () => t('Setting up a routine')],
  [/\btirith ask-owner\b/i, () => t('Writing a question for you')],
  [/\btirith (plan|lane start|lane stop|lane delete)\b/i, () => t('Planning the work')],
  [/\btirith (status|lane log|answer|card)\b/i, () => t('Checking on the agents')],
  [/\btirith (room|say)\b/i, () => t('Writing in a room')],
  [/^(WebFetch|WebSearch|web_search|web_fetch|browser_\w+|fetch_url)\b|\b(curl|wget)\b|https?:\/\//i, () => t('Looking on the web')],
  [/^(Write|Edit|MultiEdit|NotebookEdit|write_file|edit_file|create_file|apply_patch|str_replace\w*)\b|^(mkdir|cp|mv|touch|tee|rm)\b/i, () => t('Writing')],
  [/^(Read|read_file|read_many_files|Glob|Grep|LS|list_dir|list_directory|search_files|glob|grep)\b|^(cat|head|tail|ls|rg|find|sed -n|wc|less)\b/i, () => t('Reading')],
  [/^(TodoWrite|todo_\w+|update_plan)\b/i, () => t('Planning the work')],
  [/^(Task|Agent|spawn_agent)\b/i, () => t('Asking a helper')],
  [/\bgit\b/i, () => t('Looking at the code')],
  [/\b(cargo|npm|npx|pnpm|yarn|node|python3?|pytest|make|gradle|gradlew|xcodebuild|swift|go test)\b/i, () => t('Running checks')],
];

/** Its own words when they are a short plain sentence: the first line, without Markdown, cut at a few words. */
function ownWords(text) {
  const line = String(text || '').split('\n').map((l) => l.trim()).find(Boolean) || '';
  if (!line || /`|\$ |^[#>|-]|https?:\/\/|\/\w+\//.test(line)) return '';
  const plain = line.replace(/\*\*|__|[*_]/g, '').replace(/[:.]+$/, '');
  return plain.length > 64 ? `${plain.slice(0, 61).replace(/\s+\S*$/, '')}…` : plain;
}

/** A few plain words for a step; 'Thinking' before the first one. */
export function intentOf(step) {
  if (!step) return t('Thinking');
  if (step.role === 'manager') return ownWords(step.text) || t('Thinking');
  const text = String(step.text != null ? step.text : step.summary || '').replace(/^Bash: /, '').trim();
  for (const [re, words] of RULES) if (re.test(text)) return words();
  return t('Working');
}
