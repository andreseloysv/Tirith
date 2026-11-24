/* Counts a page view on the owner's own server: no cookie, no storage, nothing that follows a person. A browser that
   asks not to be tracked is not counted. The address is a picture request, so it works from any host. */
(function () {
  if (navigator.doNotTrack === '1' || location.hostname === 'localhost' || location.hostname === '127.0.0.1') return;
  var q = '?p=' + encodeURIComponent(location.pathname) + '&r=' + encodeURIComponent(document.referrer) + '&w=' + window.innerWidth;
  new Image().src = 'https://estudiosv.online/stats/count' + q;
})();
