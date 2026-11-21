// Every live 3D bot, and the observer that tells each whether it is on screen (off screen nothing is drawn).
export const bots3 = new Set();
export const seen3 = typeof IntersectionObserver === 'function'
  ? new IntersectionObserver((es) => es.forEach((e) => { if (e.target.bot3d) e.target.bot3d.visible = e.isIntersecting; }), { rootMargin: '120px' }) : null;
