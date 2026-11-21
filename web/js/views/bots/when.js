// When a bot's routines run, in words: their days, their time, the next run.
export const DAY = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
export const WEEKDAYS = [1, 2, 3, 4, 5], DAILY = [1, 2, 3, 4, 5, 6, 7];
export const isOn = (r) => r.on !== false;

export function daysLabel(d = []) {
  const s = [...d].sort().join();
  return s === DAILY.join() ? 'Every day' : s === WEEKDAYS.join() ? 'Weekdays' : s === '6,7' ? 'Weekends' : [...d].sort().map((n) => DAY[n - 1]).join(', ');
}
const hm = (at) => String(at || '').split(':').map((x) => Number(x) || 0);
export const timeLabel = (at) => { const [h, m] = hm(at); return new Date(2000, 0, 1, h, m).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }); };

/** The next time the routine runs (the machine's local time), or null when it is off. */
export function nextRun(r, now = new Date()) {
  if (!isOn(r) || !(r.days || []).length) return null;
  const [h, m] = hm(r.at);
  for (let i = 0; i < 8; i++) {
    const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() + i, h, m);
    if (d > now && r.days.includes(((d.getDay() + 6) % 7) + 1)) return d;
  }
  return null;
}
export function whenLabel(d) {
  if (!d) return '';
  const day = new Date(), t = d.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
  const diff = Math.round((new Date(d.getFullYear(), d.getMonth(), d.getDate()) - new Date(day.getFullYear(), day.getMonth(), day.getDate())) / 86400000);
  return `${diff === 0 ? 'today' : diff === 1 ? 'tomorrow' : DAY[(d.getDay() + 6) % 7]} ${t}`;
}
export const soonest = (bot) => (bot.routines || []).map((r) => nextRun(r)).filter(Boolean).sort((a, b) => a - b)[0] || null;
