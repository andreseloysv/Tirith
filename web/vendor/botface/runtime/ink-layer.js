// The face's eyes as lines of ink (ink-drawings.js), in place of its two filled eye paths, which the engine still
// moves and which now only show where the eyes are: per eye one group that blinks with it and one for its marks,
// which do not (a brow stays up while the eye shuts), one group between the eyes, and the grey blush under all of
// them. Each frame the engine says how far the expression has changed (draw) and where each eye is (place); the
// lines keep the sheet's pen width on screen at any size of the face.
import { blend } from "./ink-blend.js";
import { SHEET_FOR, drawingFor, traced } from "./ink-drawings.js";

const NS = "http://www.w3.org/2000/svg";
const PEN = [1.5, 0.014, 5];    /* px: at least, per px of the face, at most (the sheet's pen at the face's size) */
const BLUSH = 0.2;              /* the blush: the sheet's light grey, as the eyes' ink over the face's colour */

function make(tag, parent, attrs = {}) {
  const el = document.createElementNS(NS, tag);
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, v);
  parent.appendChild(el);
  return el;
}

/* path data for the first `f` (by length) of the polyline `pts`; a stroke of no length is a dot */
function pathOf(pts, f) {
  const n = pts.length / 2;
  let total = 0;
  for (let i = 1; i < n; i += 1) total += Math.hypot(pts[2 * i] - pts[2 * i - 2], pts[2 * i + 1] - pts[2 * i - 1]);
  if (total < 0.6) return f > 0.5 ? `M${pts[0].toFixed(1)} ${pts[1].toFixed(1)}h0.01` : "";
  let left = total * Math.min(1, f), d = `M${pts[0].toFixed(1)} ${pts[1].toFixed(1)}`;
  for (let i = 1; i < n && left > 0; i += 1) {
    const x0 = pts[2 * i - 2], y0 = pts[2 * i - 1], x1 = pts[2 * i], y1 = pts[2 * i + 1];
    const s = Math.hypot(x1 - x0, y1 - y0), u = s > left ? left / s : 1;
    d += `L${(x0 + (x1 - x0) * u).toFixed(1)} ${(y0 + (y1 - y0) * u).toFixed(1)}`;
    left -= s;
  }
  return d;
}

const put = (el, name, value) => { if (el[`_${name}`] !== value) { el[`_${name}`] = value; el.setAttribute(name, value); } };
const show = (el, on) => { const v = on ? "" : "none"; if (el.style.display !== v) el.style.display = v; };

export class InkLayer {
  constructor(svg, eyes, expressions) {
    this.svg = svg;
    this.expressions = expressions;
    for (const eye of eyes) eye.style.visibility = "hidden";
    this.root = document.createElementNS(NS, "g");
    this.root.setAttribute("class", "botface-ink");
    Object.assign(this.root.style, { fill: "none", stroke: "var(--bg)", strokeLinecap: "round", strokeLinejoin: "round" });
    eyes[0].parentNode.appendChild(this.root); /* with the eyes, inside the head's outline: ink never lands on the page */
    this.groups = {};
    for (const name of ["b0", "b1", "b2", "m0", "m1", "m2", "e0", "e1"]) this.groups[name] = make("g", this.root);
    for (const name of ["b0", "b1", "b2"]) Object.assign(this.groups[name].style, { fill: "var(--bg)", stroke: "none" });
    this.parts = new Map();
    this.from = null;
    this.to = drawingFor(0, null, expressions);
    this.drawn = -1;
    this.ticks = 0;
    this.at = [null, null];
  }

  /** A new expression `index` of `state`, whose own first one is `first`: what is on the face now becomes the
      start; returns the rings the engine places the eyes by. `exact` then says the eyes show a drawing of the
      sheet, which keeps its own openness (only blinks shut it). */
  retarget(index, amount, first, state) {
    const primary = SHEET_FOR[state] ?? first;
    this.state = state; /* the same expression in another state may be drawn otherwise */
    this.fromLook = this.lookAt(amount);
    this.from = blend(this.from, this.to, amount);
    this.to = drawingFor(index, primary, this.expressions);
    this.toLook = this.to.look || [0, 0];
    this.exact = traced(index, primary);
    this.drawn = -1;
    return this.to.rings;
  }

  /** Where the drawing looks (head units of gaze) `amount` of the way into the change: the body turns that way. */
  lookAt(amount) {
    const a = this.fromLook || [0, 0], b = this.toLook || [0, 0];
    return [a[0] + (b[0] - a[0]) * amount, a[1] + (b[1] - a[1]) * amount];
  }

  /** Whether `state`, starting with expression `index`, shows a drawing of the sheet: its change is then seen
      whole, as the lines flow into the new drawing, with no blink to hide it. */
  traces(index, state) {
    return traced(index, SHEET_FOR[state] ?? index);
  }

  /** The lines as they are `amount` (0..1) of the way into the expression. */
  draw(amount) {
    if (this.ticks++ % 30 === 0) this.pen();
    if (amount === this.drawn) return;
    this.drawn = amount;
    const now = blend(this.from, this.to, amount), used = new Set();
    for (const s of now.strokes) {
      const key = `s${s.key}`;
      let el = this.parts.get(key);
      if (!el) this.parts.set(key, el = make("path", this.groups[`${s.kind === 0 ? "e" : "m"}${s.side}`], { "vector-effect": "non-scaling-stroke" }));
      put(el, "d", pathOf(s.pts, s.f));
      used.add(key);
    }
    for (const b of now.blush) {
      const key = `b${b.key}`, grow = 0.55 + 0.45 * b.f;
      let el = this.parts.get(key);
      if (!el) this.parts.set(key, el = make("ellipse", this.groups[`b${b.side}`]));
      put(el, "cx", b.cx.toFixed(1)); put(el, "cy", b.cy.toFixed(1));
      put(el, "rx", (b.rx * grow).toFixed(1)); put(el, "ry", (b.ry * grow).toFixed(1));
      put(el, "transform", `rotate(${b.a.toFixed(1)} ${b.cx.toFixed(1)} ${b.cy.toFixed(1)})`);
      put(el, "opacity", (BLUSH * b.f).toFixed(3));
      used.add(key);
    }
    for (const [key, el] of this.parts) if (!used.has(key)) { el.remove(); this.parts.delete(key); }
  }

  /** Eye `i` drawn at (x, y), scaled sx, sy (sy with its blink), its marks by msy (without), from its middle (cx, cy). */
  place(i, x, y, sx, sy, msy, cx, cy, shown) {
    this.at[i] = { x, y, sx, msy, cx, cy, shown };
    const at = `translate(${x.toFixed(2)} ${y.toFixed(2)})`, from = `translate(${(-cx).toFixed(2)} ${(-cy).toFixed(2)})`;
    const mark = `${at} scale(${sx.toFixed(4)} ${msy.toFixed(4)}) ${from}`;
    put(this.groups[`e${i}`], "transform", `${at} scale(${sx.toFixed(4)} ${sy.toFixed(4)}) ${from}`);
    put(this.groups[`m${i}`], "transform", mark);
    put(this.groups[`b${i}`], "transform", mark);
    for (const g of ["e", "m", "b"]) show(this.groups[`${g}${i}`], shown);
  }

  /** The marks between the eyes, half with each. */
  placeMiddle() {
    const [a, b] = this.at;
    if (!a || !b) return;
    const mid = (k) => (a[k] + b[k]) / 2;
    const t = `translate(${mid("x").toFixed(2)} ${mid("y").toFixed(2)}) scale(${mid("sx").toFixed(4)} ${mid("msy").toFixed(4)}) translate(${(-mid("cx")).toFixed(2)} ${(-mid("cy")).toFixed(2)})`;
    put(this.groups.m2, "transform", t);
    put(this.groups.b2, "transform", t);
    show(this.groups.m2, a.shown || b.shown);
    show(this.groups.b2, a.shown || b.shown);
  }

  /* the pen's width for the face's size on screen */
  pen() {
    const px = this.svg.getBoundingClientRect().width || 96;
    const w = Math.min(PEN[2], Math.max(PEN[0], px * PEN[1])).toFixed(2);
    if (w !== this.width) { this.width = w; this.root.style.strokeWidth = `${w}px`; }
  }

  destroy() {
    this.root.remove();
  }
}
