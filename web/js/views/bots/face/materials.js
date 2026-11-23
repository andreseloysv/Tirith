// The 3D bot's body material: its 3D shape drawn in one flat colour, exactly the bot's own — as in the owner's
// example (2026-11-23), where the bodies look flat until they turn. No light, no shine, no glow: what the eye reads is
// the outline, and while it turns, the outline of its depth.
import * as THREE from '../../../../vendor/three/three.module.min.js';

// between its cushioned profile and a ball's (geometry.js) by `ball`, as a spin puffs it
const ballVS = `attribute vec3 ballPosition; uniform float ball;
void main(){ gl_Position = projectionMatrix * modelViewMatrix * vec4(mix(position, ballPosition, ball), 1.0); }`;
const flatFS = `uniform vec3 color; void main(){ gl_FragColor = vec4(color, 1.0); }`;

/** The body in colour `color` (#rrggbb). The shader's output is not converted, so it is fed the hex's own numbers. */
export function bodyMaterial3(color) {
  const c = new THREE.Color().setHex(parseInt(String(color).slice(1, 7), 16) || 0, THREE.LinearSRGBColorSpace);
  return new THREE.ShaderMaterial({ vertexShader: ballVS, fragmentShader: flatFS, uniforms: { ball: { value: 0 }, color: { value: c } } });
}
