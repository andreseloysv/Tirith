/* playwright-core for the browser tests: from PLAYWRIGHT_FROM (a file path inside some node_modules' parent), else
   this repo's own node_modules (root or web/), never another project's. Missing: the test skips with a warning. */
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const from = [process.env.PLAYWRIGHT_FROM, path.join(root, 'x.js'), path.join(root, 'web', 'x.js')].filter(Boolean);

let found = null;
for (const f of from) {
  try { found = createRequire(f)('playwright-core').chromium; break; } catch {}
}
if (!found) {
  console.warn(`SKIP ${path.basename(process.argv[1] || 'test')}: playwright-core is not installed in this repo `
    + '(npm i --no-save playwright-core in the repo root, or set PLAYWRIGHT_FROM, to run it)');
  process.exit(0);
}
export const chromium = found;
