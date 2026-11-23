// What the Bots view shows before there is any bot: the deck's first page and "Create your first bot".
import { html } from '../../../vendor/preact-htm.js';
import { Face, NewFace } from './face.js';

const SAMPLE = [['dome', '#3b82f6', 'watch'], ['cloud', '#ec4899', 'idle'], ['pill', '#4fb86e', 'idle'], ['round', '#8c8c8c', 'sleep'], ['blob', '#f07a3a', 'watch'], ['burst', '#8b5cf6', 'idle']];

export function Intro({ onNew, onTemplate }) {
  return html`<div class="bintro">
    <div class="btop"><span>OpenBot</span><span>Bots</span><span class="bnum">Agents that keep watch</span></div><div class="brule"></div>
    <h1>Bots that keep watch.</h1>
    <p class="bsub">They do your tasks on a schedule, <b>and only speak up when it matters.</b></p>
    <span class="bdash"></span>
    <p class="blead">Give a bot a name and a face, then tell it what to watch and when: “every weekday at 8, check my inbox for new job offers”. It sets the routine up itself and reports in its chat, here and on your phone.</p>
    <div class="blineup">${SAMPLE.map(([shape, color, state], i) => html`<span class="bpop" key=${shape} style=${`animation-delay:${0.2 + i * 0.12}s`}>
      <${Face} shape=${shape} color=${color} state=${state} size=${[84, 90, 80, 76, 86, 80][i]} /></span>`)}
      <button type="button" class="bpop bfirst" style="animation-delay:.95s" onClick=${onNew} aria-label="Create a bot"><${NewFace} size=${76} /></button></div>
    <div class="bfoot"><button type="button" class="bbtn pri bbig" onClick=${onNew}>Create your first bot</button>
      <button type="button" class="bbtn bbig" onClick=${onTemplate}>Desde plantilla</button></div>
    <div class="bkeys"><div class="bk">Keywords</div>
      <div><h4>Signal</h4><p>Only what concerns you.</p></div>
      <div><h4>Routines</h4><p>Each bot has its tasks and its times.</p></div>
      <div><h4>You decide</h4><p>A bot never acts for you unasked.</p></div></div>
  </div>`;
}
