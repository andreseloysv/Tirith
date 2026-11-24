/* The marketplace's category tabs: show the cards of one category. Without this script every card shows. */
document.addEventListener('DOMContentLoaded', function () {
  var tabs = document.querySelectorAll('button.mtab');
  tabs.forEach(function (t) {
    t.addEventListener('click', function () {
      var cat = t.dataset.cat;
      tabs.forEach(function (o) { o.setAttribute('aria-pressed', String(o === t)); });
      document.querySelectorAll('.mcard').forEach(function (c) { c.hidden = cat !== 'All' && c.dataset.cat !== cat; });
    });
  });
});
