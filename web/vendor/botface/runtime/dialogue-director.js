import { MORPH_IDS, STATE_IDS } from "../catalog.js";
import {
  ChineseSampledVoiceVoice,
  DIALOGUE_ENGLISH_MODES,
  isSampledDialogueVoice,
} from "./speech-voice.js";
import { analyzeSpeechUnits } from "./speech-unit-analyzer.js";

export { DIALOGUE_ENGLISH_MODES };

export const DIALOGUE_VOICES = Object.freeze([
  Object.freeze({ id: "playful", label: { zh: "中英淘气高音", en: "Bilingual Playful" }, description: { zh: "sampled speech 双语路由 · 更高音、更跳跃", en: "Bilingual sampled speech routing with a higher, bouncier profile" } }),
  Object.freeze({ id: "clean", label: { zh: "中英 SampledVoice", en: "Bilingual SampledVoice" }, description: { zh: "中文拼音声调 + 官方英文音素分析", en: "Mandarin Pinyin tones plus the upstream English analyzer" } }),
  Object.freeze({ id: "gameboy", label: { zh: "Game Boy", en: "Game Boy" }, description: { zh: "短促方波电子音", en: "Short square-wave bleeps" } }),
  Object.freeze({ id: "rpg", label: { zh: "经典 RPG", en: "Classic RPG" }, description: { zh: "柔和三角波文字音", en: "Soft triangle-wave text bleeps" } }),
]);

const VOICE_IDS = new Set(DIALOGUE_VOICES.map(({ id }) => id));
const PUNCTUATION_PAUSE = Object.freeze({
  ",": 90, "，": 90, ";": 130, "；": 130, ":": 130, "：": 130,
  ".": 210, "。": 210, "!": 220, "！": 220, "?": 240, "？": 240,
  "…": 180, "\n": 260,
});
const clamp = (value, min, max) => Math.min(max, Math.max(min, value));

function normalizedDuration(value, fallback = 300) {
  const duration = Number(value);
  if (!Number.isFinite(duration)) return fallback;
  return clamp(Math.round(duration), 0, 20000);
}

function textCharacterDuration(character, rate = 1, voice = "playful") {
  const isCjk = /[\u3400-\u9fff\uf900-\ufaff]/u.test(character);
  const base = voice === "playful"
    ? (isCjk ? 92 : 58)
    : voice === "clean" ? (isCjk ? 104 : 64) : (isCjk ? 118 : 72);
  return (base + (PUNCTUATION_PAUSE[character] || 0)) / rate;
}

function normalizeNode(node, index) {
  if (!node || typeof node !== "object") throw new TypeError(`Dialogue node ${index + 1} must be an object`);
  const id = typeof node.id === "string" && node.id ? node.id : `dialogue-node-${index + 1}`;
  if (node.type === "text") {
    const text = String(node.text ?? "");
    return { id, type: "text", text };
  }
  if (node.type === "state") {
    if (!STATE_IDS.includes(node.state)) throw new RangeError(`Unknown dialogue state at node ${index + 1}: ${node.state}`);
    return { id, type: "state", state: node.state, duration: normalizedDuration(node.duration, 320) };
  }
  if (node.type === "rotate") {
    const angle = Number(node.angle);
    if (!Number.isFinite(angle)) throw new RangeError(`Invalid dialogue rotation at node ${index + 1}`);
    return { id, type: "rotate", angle: clamp(angle, -180, 180), duration: normalizedDuration(node.duration, 260) };
  }
  if (node.type === "morph") {
    if (!MORPH_IDS.includes(node.effect)) throw new RangeError(`Unknown dialogue Morph at node ${index + 1}: ${node.effect}`);
    return { id, type: "morph", effect: node.effect, duration: normalizedDuration(node.duration, 700) };
  }
  if (node.type === "pause") return { id, type: "pause", duration: normalizedDuration(node.duration, 450) };
  throw new RangeError(`Unknown dialogue node type at node ${index + 1}: ${node.type}`);
}

export function compileDialogue(script, { rate = 1, voice = "playful", englishMode = "phonetic" } = {}) {
  if (!Array.isArray(script) || !script.length) throw new TypeError("Dialogue requires at least one text or action node");
  const safeRate = clamp(Number(rate) || 1, 0.5, 2);
  const voiceId = VOICE_IDS.has(voice) ? voice : "playful";
  const safeEnglishMode = DIALOGUE_ENGLISH_MODES.includes(englishMode) ? englishMode : "phonetic";
  const nodes = script.map(normalizeNode).filter((node) => node.type !== "text" || node.text.length);
  if (!nodes.some((node) => node.type === "text" && node.text.trim())) throw new TypeError("Dialogue requires some visible text");
  const duration = nodes.reduce((total, node) => {
    if (node.type !== "text") return total + node.duration + (node.type === "morph" ? 420 : 0);
    return total + Array.from(node.text).reduce((sum, character) => sum + textCharacterDuration(character, safeRate, voiceId), 0);
  }, 0);
  return Object.freeze({ nodes: Object.freeze(nodes), duration, rate: safeRate, voice: voiceId, englishMode: safeEnglishMode });
}

class CharacterVoice {
  constructor() {
    this.context = null;
    this.active = new Set();
    this.sampled = new ChineseSampledVoiceVoice();
    this.lastStyle = "playful";
  }

  async resume(style) {
    const activeStyle = style || this.lastStyle;
    this.lastStyle = activeStyle;
    if (isSampledDialogueVoice(activeStyle)) {
      try {
        return await this.sampled.resume(activeStyle);
      } catch (error) {
        console.warn("Morph Bot could not load the SampledVoice voice bank; dialogue will continue silently.", error);
        return null;
      }
    }
    const AudioContextClass = globalThis.AudioContext || globalThis.webkitAudioContext;
    if (!AudioContextClass) return null;
    if (!this.context) this.context = new AudioContextClass();
    if (this.context.state === "suspended") await this.context.resume();
    return this.context;
  }

  async suspend() {
    if (this.context?.state === "running") await this.context.suspend();
    await this.sampled.suspend();
  }

  stop() {
    for (const source of this.active) {
      try { source.stop(); } catch { /* source may already be stopped */ }
    }
    this.active.clear();
    this.sampled.stop();
  }

  async speech(text, style, rate, options) {
    if (!isSampledDialogueVoice(style)) return null;
    try {
      return await this.sampled.speech(text, style, rate, options);
    } catch (error) {
      console.warn("Morph Bot could not synthesize SampledVoice dialogue; dialogue will continue silently.", error);
      return null;
    }
  }

  playSample(buffer, style) {
    return this.sampled.play(buffer, style);
  }

  chirp(unitOrCharacter, style = "rpg", rate = 1) {
    const unit = typeof unitOrCharacter === "string" ? analyzeSpeechUnits(unitOrCharacter)[0] : unitOrCharacter;
    if (!this.context || this.context.state !== "running" || !unit?.speakable) return;
    const character = unit.character;
    const code = character.codePointAt(0) || 0;
    const preset = style === "gameboy"
      ? { wave: "square", base: 185, range: 145, length: 0.045, gain: 0.028 }
      : style === "rpg"
        ? { wave: "triangle", base: 280, range: 180, length: 0.058, gain: 0.038 }
        : { wave: "triangle", base: 280, range: 180, length: 0.058, gain: 0.038 };
    const now = this.context.currentTime;
    const oscillator = this.context.createOscillator();
    const gain = this.context.createGain();
    const frequency = (preset.base + ((code * 17) % preset.range)) * Math.sqrt(rate);
    oscillator.type = preset.wave;
    oscillator.frequency.setValueAtTime(frequency, now);
    gain.gain.setValueAtTime(0.0001, now);
    gain.gain.exponentialRampToValueAtTime(preset.gain, now + 0.008);
    gain.gain.exponentialRampToValueAtTime(0.0001, now + preset.length);
    oscillator.connect(gain).connect(this.context.destination);
    oscillator.addEventListener("ended", () => this.active.delete(oscillator), { once: true });
    this.active.add(oscillator);
    oscillator.start(now);
    oscillator.stop(now + preset.length + 0.01);
  }
}

export class DialogueDirector {
  constructor(bot) {
    this.bot = bot;
    this.voice = new CharacterVoice();
    this.token = 0;
    this.paused = false;
    this.playing = false;
    this.elapsed = 0;
    this.total = 0;
    this.spokenText = "";
    this.initialPaused = false;
  }

  _dispatch(type, detail = {}) {
    this.bot.dispatchEvent(new CustomEvent(type, { detail }));
  }

  async _wait(duration, token, activeNode) {
    if (duration <= 0) return token === this.token;
    return new Promise((resolve) => {
      let remaining = duration;
      let previous = performance.now();
      const inspect = (now) => {
        if (token !== this.token) { resolve(false); return; }
        if (!(this.paused || this.bot.paused)) {
          const delta = Math.max(0, now - previous);
          remaining -= delta;
          this.elapsed = Math.min(this.total, this.elapsed + delta);
          this._dispatch("dialogueprogress", {
            elapsed: this.elapsed,
            duration: this.total,
            progress: this.total ? this.elapsed / this.total : 0,
            text: this.spokenText,
            node: activeNode,
          });
        }
        previous = now;
        if (remaining <= 0) { resolve(true); return; }
        requestAnimationFrame(inspect);
      };
      requestAnimationFrame(inspect);
    });
  }

  async _followSpeechClock(speech, playback, token, activeNode, onCue) {
    if (!playback) return false;
    const cues = speech.visualCues || speech.cues;
    const startingElapsed = this.elapsed;
    return new Promise((resolve) => {
      let cueIndex = 0;
      const inspect = () => {
        if (token !== this.token) { resolve(false); return; }
        let position = playback.position();
        if (playback.ended) position = speech.duration;
        const started = position >= 0;
        const visiblePosition = Math.max(0, Math.min(speech.duration, position));
        while (started && cueIndex < cues.length && cues[cueIndex].start <= visiblePosition + 0.5) {
          onCue(cues[cueIndex]);
          cueIndex += 1;
        }
        this.elapsed = Math.min(this.total, startingElapsed + visiblePosition);
        this._dispatch("dialogueprogress", {
          elapsed: this.elapsed,
          duration: this.total,
          progress: this.total ? this.elapsed / this.total : 0,
          text: this.spokenText,
          node: activeNode,
          audioTime: visiblePosition,
        });
        if (visiblePosition >= speech.duration) {
          while (cueIndex < cues.length) {
            onCue(cues[cueIndex]);
            cueIndex += 1;
          }
          this.elapsed = Math.min(this.total, startingElapsed + speech.duration);
          resolve(true);
          return;
        }
        requestAnimationFrame(inspect);
      };
      requestAnimationFrame(inspect);
    });
  }

  async play(script, { voice = "playful", rate = 1, englishMode = "phonetic" } = {}) {
    const voiceId = VOICE_IDS.has(voice) ? voice : "playful";
    const compiled = compileDialogue(script, { rate, voice: voiceId, englishMode });
    this.stop({ restorePause: true });
    const token = ++this.token;
    this.playing = true;
    this.paused = false;
    this.elapsed = 0;
    this.total = compiled.duration;
    this.spokenText = "";
    this.initialPaused = this.bot.paused;
    if (this.initialPaused) this.bot.play();
    await this.voice.resume(voiceId);
    const speechPlans = new Map();
    if (isSampledDialogueVoice(voiceId)) {
      for (let index = 0; index < compiled.nodes.length; index += 1) {
        if (token !== this.token) return { cancelled: true, index };
        const node = compiled.nodes[index];
        if (node.type !== "text") continue;
        const speech = await this.voice.speech(node.text, voiceId, compiled.rate, { englishMode: compiled.englishMode });
        if (speech) speechPlans.set(index, speech);
      }
      this.total = compiled.nodes.reduce((total, node, index) => {
        if (node.type !== "text") return total + node.duration + (node.type === "morph" ? 420 : 0);
        return total + (speechPlans.get(index)?.duration ?? Array.from(node.text).reduce(
          (sum, character) => sum + textCharacterDuration(character, compiled.rate, voiceId),
          0,
        ));
      }, 0);
    }
    this._dispatch("dialoguestart", {
      script: compiled.nodes,
      duration: this.total,
      voice: voiceId,
      rate: compiled.rate,
      englishMode: compiled.englishMode,
    });

    for (let index = 0; index < compiled.nodes.length; index += 1) {
      if (token !== this.token) return { cancelled: true, index };
      const node = compiled.nodes[index];
      if (node.type === "text") {
        const units = analyzeSpeechUnits(node.text);
        const speech = speechPlans.get(index);
        if (speech) {
          let unitCursor = 0;
          const playback = this.voice.playSample(speech.buffer, voiceId);
          const followed = await this._followSpeechClock(speech, playback, token, node, (cue) => {
            const character = cue.char || "";
            const characterCount = Math.max(1, Array.from(character).length);
            const spokenUnits = units.slice(unitCursor, unitCursor + characterCount);
            unitCursor += characterCount;
            this.spokenText += character;
            this._dispatch("dialoguecharacter", {
              character,
              phonetic: spokenUnits.map((unit) => unit.phonetic).join(" "),
              phoneme: cue.phoneme,
              pitch: cue.pitch,
              audioTime: cue.start,
              groupCharacter: cue.groupCharacter || character,
              text: this.spokenText,
              index,
              node,
            });
          });
          if (!followed) return { cancelled: true, index };
        } else {
          for (const unit of units) {
            if (token !== this.token) return { cancelled: true, index };
            const character = unit.character;
            this.spokenText += character;
            if (!isSampledDialogueVoice(voiceId)) this.voice.chirp(unit, voiceId, compiled.rate);
            this._dispatch("dialoguecharacter", { character, phonetic: unit.phonetic, text: this.spokenText, index, node });
            if (!(await this._wait(textCharacterDuration(character, compiled.rate, voiceId), token, node))) return { cancelled: true, index };
          }
        }
        continue;
      }

      this._dispatch("dialogueaction", { index, node });
      let morphPlayback = null;
      if (node.type === "state") this.bot.setState(node.state, { replay: true });
      else if (node.type === "rotate") this.bot.rotation = node.angle;
      else if (node.type === "morph") {
        morphPlayback = this.bot.playMorph(node.effect, { hold: node.duration, restore: "default" });
      }
      const wait = node.duration + (node.type === "morph" ? 420 : 0);
      if (!(await this._wait(wait, token, node))) return { cancelled: true, index };
      if (morphPlayback) {
        const result = await morphPlayback;
        if (result.cancelled || token !== this.token) return { cancelled: true, index };
      }
    }

    if (token !== this.token) return { cancelled: true };
    this.playing = false;
    if (this.initialPaused) this.bot.pause();
    this._dispatch("dialogueend", { cancelled: false, duration: this.elapsed, text: this.spokenText });
    return { cancelled: false, duration: this.elapsed, text: this.spokenText };
  }

  pause() {
    if (!this.playing || this.paused) return this;
    this.paused = true;
    this.bot.pause();
    this.voice.suspend();
    this._dispatch("dialoguepause", { elapsed: this.elapsed, duration: this.total });
    return this;
  }

  resume() {
    if (!this.playing || !this.paused) return this;
    this.paused = false;
    this.bot.play();
    this.voice.resume();
    this._dispatch("dialogueresume", { elapsed: this.elapsed, duration: this.total });
    return this;
  }

  stop({ restorePause = false } = {}) {
    const wasPlaying = this.playing;
    this.token += 1;
    this.playing = false;
    this.paused = false;
    this.voice.stop();
    if (restorePause && wasPlaying) {
      if (this.initialPaused) this.bot.pause();
      else this.bot.play();
    }
    if (wasPlaying) this._dispatch("dialogueend", { cancelled: true, duration: this.elapsed, text: this.spokenText });
    return this;
  }
}
