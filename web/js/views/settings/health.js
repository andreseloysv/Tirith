// The factory's health, in Settings: its measures against their targets and the CEO's last pass, with Improve now.
// It was the Overview's first card; since 2026-11-23 the Overview shows only what waits for the owner and who works.
import { html } from '../../../vendor/preact-htm.js';
import { useFetch, useStore } from '../../store.js';
import { ACTIVE } from '../../common.js';
import { Status } from '../overview/status.js';

export function Health() {
  const s = useStore();
  const [metrics] = useFetch('/api/metrics', ['metrics', 'review']);
  const working = Object.values(s.lanes).filter((l) => ACTIVE.includes(l.status)).length;
  return html`<${Status} data=${metrics} working=${working} />`;
}
