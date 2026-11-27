// When a bot's routines run, in words: their days, their time, the next run.
import { t } from '../../i18n.js';
import { everySecs } from './every.js';
export const DAY = [t('Mon'), t('Tue'), t('Wed'), t('Thu'), t('Fri'), t('Sat'), t('Sun')];
export const WEEKDAYS = [1, 2, 3, 4, 5], DAILY = [1, 2, 3, 4, 5, 6, 7];
export const isOn = (r) => r.on !== false;

export function daysLabel(d = []) {
  const s = [...d].sort().join();
  return s === DAILY.join() ? t('Every day') : s === WEEKDAYS.join() ? t('Weekdays') : s === '6,7' ? t('Weekends') : [...d].sort().map((n) => DAY[n - 1]).join(', ');
}
const hm = (at) => String(at || '').split(':').map((x) => Number(x) || 0);
export const timeLabel = (at) => { const [h, m] = hm(at); return new Date(2000, 0, 1, h, m).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }); };

/** The next time the routine runs (the machine's local time), or null when it is off. */
export function nextRun(r, now = new Date()) {
  if (r.every) { const s = everySecs(r.every), last = Date.parse(r.last || ''); return isOn(r) && s ? new Date(Math.max(now.getTime(), (last || 0) + s * 1000)) : null; }
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
  const day = new Date(), time = d.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
  const diff = Math.round((new Date(d.getFullYear(), d.getMonth(), d.getDate()) - new Date(day.getFullYear(), day.getMonth(), day.getDate())) / 86400000);
  return diff === 0 ? t('today {time}', { time }) : diff === 1 ? t('tomorrow {time}', { time }) : `${DAY[(d.getDay() + 6) % 7]} ${time}`;
}
export const soonest = (bot) => (bot.routines || []).map((r) => nextRun(r)).filter(Boolean).sort((a, b) => a - b)[0] || null;
