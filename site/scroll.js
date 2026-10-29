/* On-scroll reveals. Plain IntersectionObserver, no libraries. The hidden state exists only when this
   script has added "js" to <html> (and the visitor has not asked for reduced motion, see style.css),
   so without JS or with reduced motion everything is simply visible. */
(function () {
  var root = document.documentElement;
  root.classList.add('js');
  var still = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  function setup() {
    var top = document.querySelector('header.top');
    var hero = document.querySelector('.hero .shot img');
    var items = [];

    function mark(sel, step) {
      document.querySelectorAll(sel).forEach(function (el, i) {
        if (el.classList.contains('rv')) return;
        el.classList.add('rv');
        el.style.setProperty('--d', (step ? i * step : 0) + 'ms');
        items.push(el);
      });
    }
    // hero: staggered entrance on load
    var heroKids = document.querySelectorAll('.hero > *');
    heroKids.forEach(function (el, i) {
      el.classList.add('rv');
      el.style.setProperty('--d', i * 90 + 'ms');
      items.push(el);
    });
    // each group staggers by position; a group is its own parent so indexes restart per group
    document.querySelectorAll('.three, .two').forEach(function (g) {
      Array.prototype.forEach.call(g.children, function (el, i) {
        el.classList.add('rv'); el.style.setProperty('--d', i * 110 + 'ms'); items.push(el);
      });
    });
    document.querySelectorAll('.feature').forEach(function (f) {
      Array.prototype.forEach.call(f.children, function (el, i) {
        el.classList.add('rv'); el.style.setProperty('--d', i * 120 + 'ms'); items.push(el);
      });
    });
    mark('main > section > h2', 0);
    mark('main > section > .sub, main > section > .bullets, main > section > pre, main > section > .note, main > section > .diagram', 0);

    function show(el) { el.classList.add('in'); }
    function showAll() { items.forEach(show); }

    if (still || !('IntersectionObserver' in window)) { showAll(); }
    else {
      var io = new IntersectionObserver(function (entries) {
        entries.forEach(function (e) {
          if (e.isIntersecting) { show(e.target); io.unobserve(e.target); }
        });
      }, { rootMargin: '0px 0px -8% 0px', threshold: 0.05 });
      items.forEach(function (el) { io.observe(el); });
    }

    var ticking = false;
    function frame() {
      ticking = false;
      var y = window.scrollY;
      if (top) top.classList.toggle('scrolled', y > 8);
      if (!still && hero && y < window.innerHeight * 1.5) {
        hero.style.transform = 'translate3d(0,' + Math.min(y * 0.05, 18).toFixed(1) + 'px,0) scale(1.05)';
      }
      if (window.innerHeight + y >= document.documentElement.scrollHeight - 2) showAll();
    }
    window.addEventListener('scroll', function () {
      if (!ticking) { ticking = true; requestAnimationFrame(frame); }
    }, { passive: true });
    frame();
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', setup);
  else setup();
})();
