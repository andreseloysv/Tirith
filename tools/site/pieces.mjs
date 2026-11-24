// Small HTML pieces of the marketplace pages: head, bar, footer and the bot card.
export const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);

export function head(root, title, desc) {
  const css = ['01-base', '02-nav-hero', '03-intro-features', '04-jobs-pricing', '05-install', '06-faq-footer'];
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${esc(title)}</title>
<meta name="description" content="${esc(desc)}">
<meta name="theme-color" content="#ffffff">
<link rel="icon" href="${root}favicon.ico" sizes="16x16 32x32 48x48">
<link rel="apple-touch-icon" href="${root}img/apple-touch-icon.png">
${css.map((c) => `<link rel="stylesheet" href="${root}css/${c}.css">`).join('\n')}
<link rel="stylesheet" href="${root}css/07-marketplace.css">
<script src="${root}js/page.js"></script>
<script src="${root}js/market.js" defer></script>
</head>`;
}

export function bar(root) {
  const a = (h, t) => `<a href="${root}${h}">${t}</a>`;
  return `<body>
<a class="skip" href="#main">Skip to content</a>
<header class="top"><div class="wrap">
  <a class="brand" href="${root}"><img src="${root}img/openbot-icon-180.png" width="30" height="30" alt="">OpenBot</a>
  <nav aria-label="Sections">${a('#features', 'Features')}${a('#jobs', 'Jobs')}${a('#pricing', 'Pricing')}${a('#compare', 'Compare')}<a href="${root}marketplace/" aria-current="page">Marketplace</a>${a('#install', 'Install')}${a('#faq', 'FAQ')}</nav>
  <div class="end"><a class="btn small line" href="https://github.com/andreseloysv/Tirith">GitHub</a><a class="btn small dark" href="${root}#install">Install</a></div>
</div></header>
<main id="main">`;
}

export const foot = (root) => `</main>
<footer><div class="wrap">
  <div><a class="brand" href="${root}"><img src="${root}img/openbot-icon-180.png" width="28" height="28" alt="">OpenBot</a>
  <p class="fine">© 2026 OpenBot contributors · MIT license<br>The command, folders and settings are still called <code>tirith</code>.</p></div>
  <div><h4>Product</h4><ul><li><a href="${root}#features">Features</a></li><li><a href="${root}marketplace/">Marketplace</a></li><li><a href="${root}#install">Install</a></li></ul></div>
  <div><h4>Docs</h4><ul><li><a href="https://github.com/andreseloysv/Tirith/blob/main/docs/BOTS.md">Bots</a></li></ul></div>
  <div><h4>Project</h4><ul><li><a href="https://github.com/andreseloysv/Tirith">GitHub</a></li></ul></div>
</div></footer>
</body>
</html>
`;

export const card = (b, root) => `<article class="mcard" data-cat="${esc(b.category)}">
  <img src="${root}img/faces/${b.face}.webp" width="56" height="56" alt="">
  <div><h3><a href="${root}marketplace/bots/${b.id}/">${esc(b.title)}</a></h3>
  <p class="by">by ${esc(b.by)}</p><p>${esc(b.summary)}</p></div>
  <a class="btn small" href="${root}marketplace/bots/${b.id}/">View</a>
</article>`;
