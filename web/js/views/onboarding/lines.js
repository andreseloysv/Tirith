// What the new assistant says in the welcome (view.js), in the owner's language: its structure is the owner's
// example of 2026-11-24 (a greeting by name, "what would you like to call me?", a little about me, what I can do,
// and an offer with its choices).
import { t } from '../../i18n.js';

const list = (head, items) => [head, '', ...items.map((i) => `- ${i}`)].join('\n');

export const hello = (name) => (name
  ? t("Hi {name}! I'm your new personal assistant. I can help with your work and everyday life, and take things off your plate.", { name })
  : t("Hi! I'm your new personal assistant. I can help with your work and everyday life, and take things off your plate."));

export const askName = () => t('What would you like to call me? You can always change this later.');

export const met = (bot, name) => (name ? t('{bot} it is! Nice to meet you, {name}.', { bot, name }) : t('{bot} it is! Nice to meet you.', { bot }));

export const about = () => list(t('A little about me:'), [
  t("I keep working while you're away, here on your computer."),
  t('I use your apps and websites, like email and calendar, just like you do.'),
  t('I ask you before anything important, and you can see everything I do.'),
]);

export const can = () => list(t("Here's some of what I can do for you:"), [
  t('Summarize your new email every morning'),
  t('Draft replies and send them once you approve'),
  t('Watch a website and tell you when something changes'),
  t('Remind you of what matters, before it is urgent'),
]);

export const offer = () => t("Want me to find a few things I can help with? I'll only look: nothing gets sent, deleted or changed.");

/** The offer's choices: [id, words]. */
export const choices = () => [['suggest', t('Suggest a task')], ['email', t('Check my email every morning')], ['else', t('Something else')]];

/** What the owner asks the assistant when they choose a suggestion. */
export const suggestAsk = () => t('Suggest a few tasks you could take off my plate. Only look: send, delete or change nothing.');

/** Every line the assistant said, in order, as its chat keeps them. */
export const said = (owner, bot) => [hello(owner), askName(), met(bot, owner), about(), can(), offer()];
