/* Where a browser test saves a capture. Default: a temporary folder, so a test run never touches versioned files.
   UPDATE_SCREENSHOTS=1 writes into the versioned captures folder instead (the deliberate refresh). */
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const versioned = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', 'screenshots');
let tmp = null;
export function shotPath(name) {
  if (process.env.UPDATE_SCREENSHOTS === '1') return path.join(versioned, name);
  tmp ??= fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-shots-'));
  return path.join(tmp, name);
}
