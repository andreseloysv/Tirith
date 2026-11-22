// When a face plays its acts: each in an order of its own, every one before any comes again and never the same twice
// in a row; lively beside what a bot is doing, now and then while it works elsewhere, rare while it idles.
import { R } from './motion.js';
import { ACTS10, NAMES } from './acts.js';
import { REST, poseOf } from './pose.js';

/* seconds between two acts; none asleep */
const GAP = { lively: [0.5, 1.3], work: [4, 9], watch: [10, 26], idle: [9, 24] };
const AWAY = 1.5; /* not stepped for this long: it was off screen, so it starts again at a time of its own */
const APART = 0.45; /* two faces never start within this of each other */
const decks = new Map(); /* a lively face's deck outlives its short life: the next turn goes on where this one was */
let lastStart = -Infinity;

function shuffled(not) {
  const out = [...NAMES];
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  if (out[0] === not) out.push(out.shift());
  return out;
}

export class Acts {
  /** `lively`: the face beside what its bot is doing; `key`: whose deck it deals from. */
  constructor({ lively = false, key = '' } = {}) {
    this.lively = lively;
    this.reach = lively ? 1 : 0.35;
    this.deck = (lively && decks.get(key)) || { left: [], last: '' };
    if (lively) decks.set(key, this.deck);
    this.now = null; this.next = null; this.playing = null;
  }

  get fast() { return !!(this.lively && this.playing && ACTS10[this.playing.name].turn); }

  play(name, at, flip = Math.random() < 0.5) {
    this.playing = { name, at, flip: flip && !ACTS10[name].oneWay };
  }

  deal() {
    if (!this.deck.left.length) this.deck.left = shuffled(this.deck.last);
    return (this.deck.last = this.deck.left.shift());
  }

  /** The pose at `now` (seconds) of a face in `mood`; `held`: the pointer has it, so nothing new starts. */
  step(now, mood, held = false) {
    const gap = this.lively ? GAP.lively : GAP[mood];
    const back = this.now == null || now - this.now > AWAY;
    this.now = now;
    if (back) {
      this.playing = null;
      this.next = now + (this.lively ? R(0.15, 0.35) : R(1.5, gap ? gap[1] : 10));
    }
    if (this.playing) {
      const { name, at, flip } = this.playing, act = ACTS10[name], x = (now - at) / act.d;
      if (x < 1) return poseOf(act, Math.max(0, x), { flip, reach: this.reach });
      this.playing = null;
      if (gap) this.next = now + R(...gap);
    }
    if (!gap) return REST;
    if (this.next > now + gap[1]) this.next = now + R(...gap); /* it got busier */
    if (held || now < this.next) return REST;
    if (!this.lively && now - lastStart < APART) {
      this.next = now + R(APART, 3 * APART);
      return REST;
    }
    lastStart = now;
    this.play(this.deal(), now);
    return poseOf(ACTS10[this.playing.name], 0, { flip: this.playing.flip, reach: this.reach });
  }
}
