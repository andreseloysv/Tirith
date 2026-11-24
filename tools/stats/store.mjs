// The counter's store: page views in SQLite. A visitor is only a hash of the day's secret, the address and the browser
// name; the secret is new every day and never kept, so nobody can be followed from one day to the next. No cookie, no
// stored address.
import { DatabaseSync } from 'node:sqlite';
import { createHash, randomBytes } from 'node:crypto';

export function open(file) {
  const db = new DatabaseSync(file);
  db.exec(`CREATE TABLE IF NOT EXISTS hits (day TEXT, path TEXT, ref TEXT, visitor TEXT, width INTEGER);
    CREATE INDEX IF NOT EXISTS hits_day ON hits (day);`);
  const insert = db.prepare('INSERT INTO hits VALUES (?, ?, ?, ?, ?)');
  let salt = { day: '', secret: '' };
  const secret = (day) => {
    if (salt.day !== day) salt = { day, secret: randomBytes(16).toString('hex') };
    return salt.secret;
  };
  return {
    hit({ path, ref, width, ip, ua }) {
      const day = new Date().toISOString().slice(0, 10);
      const visitor = createHash('sha256').update(`${secret(day)}|${ip}|${ua}`).digest('hex').slice(0, 16);
      insert.run(day, path, ref, visitor, width);
    },
    days: (n) => db.prepare('SELECT day, COUNT(*) AS views, COUNT(DISTINCT visitor) AS visitors FROM hits WHERE day >= date(\'now\', ?) GROUP BY day ORDER BY day DESC').all(`-${n} days`),
    top: (col, n) => db.prepare(`SELECT ${col === 'ref' ? 'ref' : 'path'} AS name, COUNT(*) AS views, COUNT(DISTINCT visitor) AS visitors FROM hits WHERE day >= date('now', ?) GROUP BY name ORDER BY views DESC LIMIT 15`).all(`-${n} days`),
  };
}
