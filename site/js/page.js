/* The bar's line once the page scrolls, and the sections that rise in as they come into view. Without this
   script (or with reduced motion) everything is simply visible: the hidden state needs the "js" class it adds. */
(function () {
  var root = document.documentElement;
  var still = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  if (!still && 'IntersectionObserver' in window) root.classList.add('js');

  function bar() {
    var top = document.querySelector('header.top');
    if (!top) return;
    var on = function () { top.classList.toggle('scrolled', window.scrollY > 8); };
    on();
    window.addEventListener('scroll', on, { passive: true });
  }

  function reveal() {
    if (!root.classList.contains('js')) return;
    var groups = ['.split', '.faces', '.head', '.fcard', '.more > div', '.jobs', '.plan', '.compare', '.install', '.guide', '.faq', '.last h2'];
    var items = [];
    groups.forEach(function (sel) {
      document.querySelectorAll(sel).forEach(function (el, i) {
        el.classList.add('rv');
        el.style.setProperty('--d', (i % 4) * 80 + 'ms');
        items.push(el);
      });
    });
    var seen = new IntersectionObserver(function (entries) {
      entries.forEach(function (e) {
        if (!e.isIntersecting) return;
        e.target.classList.add('in');
        seen.unobserve(e.target);
      });
    }, { rootMargin: '0px 0px -8% 0px', threshold: 0.08 });
    items.forEach(function (el) { seen.observe(el); });
  }

  document.addEventListener('DOMContentLoaded', function () { bar(); reveal(); });
})();
