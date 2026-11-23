// The shapes a bot can have, in the picker's order, and the colours: each shape's outline as an SVG path in the box's
// units (outline.js, from the same r(theta) the 3D body is made of) and its eyes, capsules [x, y, lean, width, length].
// The thirteen bodies of the owner's example (shapes3d.js), and the app icon's star.
import { SHAPES3D as S } from '../shapes3d.js';
import { outlineD } from './outline.js';
import { HERO_OUTLINE, HERO_EYES } from './hero2d.js';

/** name: [outline path, eyes] */
export const SHAPES = {
  round: [outlineD(S.round), S.round.eyes],
  cloud: [outlineD(S.cloud), S.cloud.eyes],
  tile: [outlineD(S.tile), S.tile.eyes],
  star: [outlineD(S.star), S.star.eyes],
  clover: [outlineD(S.clover), S.clover.eyes],
  blob: [outlineD(S.blob), S.blob.eyes],
  flower: [outlineD(S.flower), S.flower.eyes],
  drop: [outlineD(S.drop), S.drop.eyes],
  pill: [outlineD(S.pill), S.pill.eyes],
  tri: [outlineD(S.tri), S.tri.eyes],
  dome: [outlineD(S.dome), S.dome.eyes],
  burst: [outlineD(S.burst), S.burst.eyes],
  hex: [outlineD(S.hex), S.hex.eyes],
  hero: [outlineD(HERO_OUTLINE), HERO_EYES],
};
/** The picker's ring around a chosen shape: its outline, `grow` box units outside it. */
export const ringOf = (name, grow = 11) => outlineD(name === 'hero' ? HERO_OUTLINE : S[name] || S.round, grow);
export const COLORS = ['#f2f2f2', '#9a6b45', '#e5484d', '#f07a3a', '#f2a93b', '#4fb86e', '#3fb6a8', '#3b82f6', '#8b5cf6', '#ec4899', '#8c8c8c'];
