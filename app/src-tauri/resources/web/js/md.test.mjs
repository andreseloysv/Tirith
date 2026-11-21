// The app serves the page from web/ (frontendDist in tauri.conf.json), so the suite lives next to md.js
// there; importing it registers its tests in this run. Run with:
//   node --test app/src-tauri/resources/web/js/md.test.mjs
import '../../../../../web/js/md.test.mjs';
