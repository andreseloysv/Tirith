// Cel shading for a face of one flat colour (the owner: «aplícales un celshading para ver si me gusta más», then
// «el celshading debería moverse cuando los avatares 3D se mueven»): the body takes its light in hard-edged bands,
// as a cartoon does — a shadow along the side away from the light, its own colour, and a thin lit crescent on the
// side towards it — and the light belongs to the body, so the bands move with it: they lean as it sways, hop and
// squash with it, and as it turns round (its 3D turn) the light turns with it and sweeps across, the whole visible
// side going into shadow when the light is behind. Each band is the head's own outline moved towards or away from
// the light and cut by the head, so the bands keep its shape whatever it is. Only the solid material is shaded;
// gradient and glass keep their own light.
const NS = "http://www.w3.org/2000/svg";
export const CEL = true;                /* the owner is trying it: false brings the flat faces back */
const LIGHT = [-0.387, -0.507, 0.77];   /* towards the light, unturned: left, up, out of the screen (unit length) */
const RADIUS = 114;                     /* head units: how far the shadow's edge can travel across the body */
const RIM = 9;                          /* head units: the lit crescent's width */
const DARK = 0.24;                      /* how much darker the shadow is (of the colour's lightness) */
const LIGHTER = 0.3;                    /* how much lighter the lit crescent is (of what is left to white) */

function make(tag, parent, attrs = {}) {
  const el = document.createElementNS(NS, tag);
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, v);
  parent.appendChild(el);
  return el;
}

/* the colour `hex` (#rgb or #rrggbb) darkened (k < 0) or lightened (k > 0), keeping its hue */
function tone(hex, k) {
  const h = hex.replace("#", ""), full = h.length === 3 ? [...h].map((c) => c + c).join("") : h;
  const rgb = [0, 2, 4].map((i) => parseInt(full.slice(i, i + 2), 16));
  if (rgb.some((v) => Number.isNaN(v))) return hex;
  const out = rgb.map((v) => Math.round(k < 0 ? v * (1 + k) : v + (255 - v) * k));
  return `#${out.map((v) => v.toString(16).padStart(2, "0")).join("")}`;
}

/* where the bands sit for a body turned `turn` radians: the lit part's shift towards the light (the shadow is what
   it leaves uncovered: from a sliver when the light is in front to all of the body when it is behind) and the
   colour's shift away from it (what it leaves is the lit crescent) */
function bands(turn, pitch) {
  const cp = Math.cos(pitch), sp = Math.sin(pitch);
  const ly = LIGHT[1] * cp + LIGHT[2] * sp, lz0 = LIGHT[2] * cp - LIGHT[1] * sp; /* tipped up or down first */
  const c = Math.cos(turn), s = Math.sin(turn);
  const lx = LIGHT[0] * c + lz0 * s, lz = lz0 * c - LIGHT[0] * s;
  const n = Math.hypot(lx, ly) || 1, reach = RADIUS * (1 - lz);
  return { shade: [(lx / n) * reach, (ly / n) * reach], rim: [(-lx / n) * RIM, (-ly / n) * RIM] };
}

const put = (el, name, value) => { if (el[`_${name}`] !== value) { el[`_${name}`] = value; el.setAttribute(name, value); } };
const shift = ([x, y]) => `translate(${x.toFixed(2)} ${y.toFixed(2)})`;

export class CelShade {
  constructor(svg, eyes) {
    const clip = eyes[0].parentNode, id = `${svg.id || "botface"}-cel-lit`;
    const defs = svg.querySelector("defs") || make("defs", svg);
    this.litClip = make("path", make("clipPath", defs, { id }));
    this.group = document.createElementNS(NS, "g");
    this.group.setAttribute("class", "botface-cel");
    this.group.setAttribute("clip-path", clip.getAttribute("clip-path"));
    clip.parentNode.insertBefore(this.group, clip);
    this.shadow = make("path", this.group);
    const lit = make("g", this.group, { "clip-path": `url(#${id})` });
    this.rim = make("path", lit);
    this.body = make("path", lit);
    this.colour = "";
  }

  /** The bands for head outline `path` in `material` (the material system's), the body turned `turn` radians and
      tipped `pitch` radians (down when positive), as it looks somewhere. */
  sync(path, material, turn = 0, pitch = 0) {
    const on = CEL && material && material.material === "solid" && typeof material.color === "string" && material.color[0] === "#";
    if (this.group.style.display !== (on ? "" : "none")) this.group.style.display = on ? "" : "none";
    if (!on || !path) return;
    if (material.color !== this.colour) {
      this.colour = material.color;
      this.shadow.setAttribute("fill", tone(material.color, -DARK));
      this.rim.setAttribute("fill", tone(material.color, LIGHTER));
      this.body.setAttribute("fill", material.color);
    }
    for (const el of [this.shadow, this.rim, this.body, this.litClip]) put(el, "d", path);
    const { shade, rim } = bands(turn, pitch);
    put(this.litClip, "transform", shift(shade));
    put(this.body, "transform", shift(rim));
  }

  destroy() {
    this.group.remove();
    this.litClip.parentNode.remove();
  }
}
