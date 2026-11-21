// Times: an ISO time's milliseconds, a duration, how long ago, a clock time, seconds between two times.

/** Milliseconds of an ISO time (UTC when it has no zone), or null. */
export function ms(iso) {
  let s = String(iso || '').trim();
  if (/^\d{4}-\d\d-\d\d[T ]\d\d:\d\d(:\d\d(\.\d+)?)?$/.test(s)) s = s.replace(' ', 'T') + 'Z';
  const t = Date.parse(s);
  return Number.isNaN(t) ? null : t;
}

export function dur(sec) {
  if (sec == null || Number.isNaN(sec)) return '';
  sec = Math.max(0, Math.floor(sec));
  const h = Math.floor(sec / 3600), m = Math.floor((sec % 3600) / 60), s = sec % 60;
  const p2 = (n) => (n < 10 ? '0' : '') + n;
  if (h > 0) return `${h}h ${p2(m)}m`;
  if (m > 0) return `${m}m ${p2(s)}s`;
  return `${s}s`;
}

export function ago(iso) {
  const t = ms(iso);
  if (t == null) return '';
  const s = Math.floor((Date.now() - t) / 1000);
  if (s < 10) return 'now';
  if (s < 60) return `${s} s`;
  if (s < 3600) return `${Math.floor(s / 60)} min`;
  if (s < 86400) return `${Math.floor(s / 3600)} h`;
  return `${Math.floor(s / 86400)} d`;
}

export function clockTime(iso) {
  const t = ms(iso);
  if (t == null) return '';
  const d = new Date(t), n = new Date(), p = (x) => (x < 10 ? '0' : '') + x;
  const hm = `${p(d.getHours())}:${p(d.getMinutes())}`;
  if (d.toDateString() === n.toDateString()) return hm;
  return `${['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'][d.getMonth()]} ${d.getDate()} ${hm}`;
}

/** Seconds between two ISO times (or until now). */
export function elapsed(start, end) {
  const a = ms(start);
  if (a == null) return null;
  const b = end ? ms(end) : Date.now();
  return Math.max(0, Math.round((b - a) / 1000));
}
