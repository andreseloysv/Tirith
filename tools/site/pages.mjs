// The two kinds of marketplace page, built from bots.json.
import { head, bar, foot, card, esc } from './pieces.mjs';

export function index(data) {
  const root = '../';
  const cats = [...new Set(data.bots.map((b) => b.category))];
  const tabs = ['All', ...cats].map((c, i) => `<button type="button" class="mtab" data-cat="${c}" aria-pressed="${i === 0}">${c}</button>`).join('');
  return head(root, 'Marketplace | OpenBot', 'Bots you can add to OpenBot, starting with the ones from the OpenBot team.')
    + bar(root) + `
<section class="narrow mhead"><h1>Marketplace</h1>
<p class="lede">Bots made by the OpenBot team. Each one lists what it does, what it needs and what it asks you before doing.</p>
<div class="mtabs" role="group" aria-label="Categories"><a class="mtab" href="./" aria-current="true">${esc(data.collection.title)}</a>${tabs}</div></section>
<section class="narrow mlist"><h2 class="msub">${esc(data.collection.title)}</h2>
${data.bots.map((b) => card(b, root)).join('\n')}</section>
<section class="narrow mnote"><p>Publishing your own bot is not open yet. When it is, it will be listed here.</p></section>
` + foot(root);
}

const list = (items) => `<ul>${items.map((i) => `<li>${esc(i)}</li>`).join('')}</ul>`;
const part = (h, sub, body) => `<section class="mpart"><h2>${h}</h2><p class="sub">${sub}</p>${body}</section>`;

export function detail(b) {
  const root = '../../../';
  return head(root, `${b.title} | OpenBot Marketplace`, b.summary) + bar(root) + `
<div class="narrow mdetail"><a class="back" href="../../">Back to Marketplace</a>
<header class="mtop"><img src="${root}img/faces/${b.face}.webp" width="120" height="120" alt="">
<div><h1>${esc(b.title)}</h1><p class="by">by ${esc(b.by)} · ${esc(b.category)}</p><p class="lede">${esc(b.about)}</p>
<h2 class="how">Add it today</h2>
<p>This bot ships inside OpenBot. In the app open <strong>Bots</strong>, choose <strong>Desde plantilla</strong> (from a template) and pick <strong>${esc(b.name)}</strong>. <a href="${root}#install">Install OpenBot</a> first if you have not.</p></div></header>
${part('Skills', 'Playbooks it can run', `<ul>${b.skills.map((s) => `<li><code>${esc(s.name)}</code> ${esc(s.text)}</li>`).join('')}</ul>`)}
${part('Routines', 'Jobs that run on their own', list(b.routines.map((r) => `${r.title}: ${r.when}`)))}
${part('Integrations', 'What it needs', list(b.integrations))}
${part('Asks first', 'It stops and asks you before', list(b.asks_first))}
</div>
` + foot(root);
}
