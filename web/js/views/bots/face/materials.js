// The 3D bot's materials and the meshes every bot shares: the glowing body, the glossy eyes, the halo and the ink.
import * as THREE from '../../../../vendor/three/three.module.min.js';

// materials: body, eyes and halo are ShaderMaterials fed THREE.Color values — they go in linear (colour
// management) and come out unconverted, as in the reference; the mouth, tongue and dot are MeshBasicMaterial,
// whose own shader converts back, so they show as their hex.
export const hex3 = (h) => new THREE.Color(h);
export const LIGHT3 = new THREE.Vector3(-0.55, 0.65, 0.75).normalize(); // soft key from the upper left, in view space
const bodyVS = `varying vec3 vN; varying vec3 vV; varying vec3 vWN;
void main(){ vec4 mv = modelViewMatrix * vec4(position,1.0); vN = normalize(normalMatrix * normal); vV = -mv.xyz; vWN = normalize(mat3(modelMatrix) * normal); gl_Position = projectionMatrix * mv; }`;
const bodyFS = `uniform vec3 core; uniform vec3 edge; uniform vec3 low; uniform vec3 L; uniform float glow;
varying vec3 vN; varying vec3 vV; varying vec3 vWN;
void main(){
  vec3 N = normalize(vN), V = normalize(vV);
  float facing = clamp(dot(N, V), 0.0, 1.0), f = 1.0 - facing;
  vec3 col = mix(core, edge, smoothstep(0.05, 0.95, pow(f, 0.75)));      // light inside: a pale core, its own colour at the rim
  float under = clamp((vWN.y + 1.0) / 1.5, 0.0, 1.0);
  col = mix(low, col, 0.72 + 0.28 * under);                                 // warmer and deeper underneath
  float wrap = clamp((dot(N, L) + 0.55) / 1.55, 0.0, 1.0);
  vec3 H = normalize(L + V);
  float spec = pow(max(dot(N, H), 0.0), 48.0) * 0.32 + pow(max(dot(N, H), 0.0), 300.0) * 0.35; // a soft coat and a small hot spot
  vec3 c = col * (0.80 + 0.28 * wrap) * glow + vec3(spec);
  c += mix(edge, vec3(1.0), 0.35) * pow(f, 2.4) * 0.6;                                           // light leaking at the silhouette
  gl_FragColor = vec4(c, 1.0);
}`;
export function bodyMaterial3(color) {
  const base = hex3(color);
  const core = base.clone().lerp(new THREE.Color(1, 1, 1), 0.62);
  const low = base.clone().multiplyScalar(0.78).lerp(new THREE.Color(1, 0.55, 0.1), 0.18);
  return new THREE.ShaderMaterial({ vertexShader: bodyVS, fragmentShader: bodyFS, uniforms: { core: { value: core }, edge: { value: base }, low: { value: low }, L: { value: LIGHT3 }, glow: { value: 1.12 } } });
}
export const eyeMat3 = new THREE.ShaderMaterial({ vertexShader: bodyVS, uniforms: { L: { value: LIGHT3 } }, fragmentShader: `uniform vec3 L; varying vec3 vN; varying vec3 vV; varying vec3 vWN;
void main(){ vec3 N = normalize(vN), V = normalize(vV), H = normalize(L + V);
  vec3 c = vec3(0.07, 0.035, 0.09) + vec3(0.10, 0.07, 0.12) * pow(1.0 - clamp(dot(N, V), 0.0, 1.0), 2.0);
  c += vec3(1.0) * smoothstep(0.86, 0.95, pow(max(dot(N, H), 0.0), 40.0));  // the catchlight
  gl_FragColor = vec4(c, 1.0); }` });
const flat3 = (c) => new THREE.MeshBasicMaterial({ color: c, side: THREE.DoubleSide });
export const EYE_GEO3 = new THREE.SphereGeometry(1, 24, 16), DOT_GEO3 = new THREE.SphereGeometry(1, 12, 8), DOT_MAT3 = new THREE.MeshBasicMaterial({ color: 0xffffff }), HALO_GEO3 = new THREE.PlaneGeometry(1, 1);
export const MOUTH3 = flat3('#2a0a16'), TONGUE3 = flat3('#e2506d'), INK3 = flat3('#1d1030');
export const haloVS = `varying vec2 vUv; void main(){ vUv = uv; gl_Position = projectionMatrix * modelViewMatrix * vec4(position,1.0); }`;
export const haloFS = `uniform vec3 color; uniform float amount; varying vec2 vUv;
void main(){ float d = length(vUv - 0.5) * 2.0; float a = pow(clamp(1.0 - d, 0.0, 1.0), 1.7) * amount; gl_FragColor = vec4(color, a); }`;
