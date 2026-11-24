import {
  SampledVoiceEngine,
  EnglishAnalyzer,
  PitchManager,
  WebPlayer,
  WebSampler,
} from "./vendor/tts/tts.browser.mjs";
import { renderDialogueAudioTimeline } from "./dialogue-audio-timeline.js";
import { analyzeSpeechUnits } from "./speech-unit-analyzer.js";

const SPRITE_AUDIO_URL = new URL("./vendor/tts/english-sprite.wav", import.meta.url);
const SPRITE_MAP_URL = new URL("./vendor/tts/english-sprite.json", import.meta.url);
const SAMPLE_PHONEME = /^[a-z]$/u;
const LATIN_CHARACTER = /^[a-z]$/iu;
const SAMPLED_VOICE_IDS = new Set(["clean", "playful"]);
export const DIALOGUE_ENGLISH_MODES = Object.freeze(["phonetic", "letters"]);
const DIGIT_PHONETICS = Object.freeze({
  0: "zero", 1: "one", 2: "two", 3: "three", 4: "four",
  5: "five", 6: "six", 7: "seven", 8: "eight", 9: "nine",
});
const PUNCTUATIONS = Object.freeze([
  ".", ",", "!", "?", ";", ":", "'", "\"", "(", ")", "~",
  "。", "，", "！", "？", "；", "：", "、", "…", "\n",
]);
const TONE_PITCH_RATIOS = Object.freeze({
  0: 0.98,
  1: 1.06,
  2: 1.02,
  3: 0.92,
  4: 0.96,
});

const VOICE_PROFILES = Object.freeze({
  clean: Object.freeze({
    pitch: 1.5,
    speed: 3.8,
    randomness: 0.04,
    melodyRate: 0.055,
    melodyAmplitude: 0.08,
    toneStrength: 0.72,
    volume: 0.64,
    crossfadeMs: 10,
  }),
  playful: Object.freeze({
    pitch: 1.94,
    speed: 4.4,
    randomness: 0.1,
    melodyRate: 0.11,
    melodyAmplitude: 0.14,
    toneStrength: 0.42,
    volume: 0.56,
    crossfadeMs: 12,
  }),
});

function pinyinLetters(value) {
  return String(value || "")
    .toLowerCase()
    .replaceAll("ü", "v")
    .replaceAll("u:", "v")
    .replaceAll("ê", "e")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/gu, "")
    .replace(/[^a-z]/gu, "");
}

function mergedTokens(phonetic) {
  const phonemes = Array.from(pinyinLetters(phonetic)).filter((phoneme) => SAMPLE_PHONEME.test(phoneme));
  return phonemes.map((phoneme, index) => ({
    phoneme,
    mergeWithNext: index < phonemes.length - 1,
  }));
}

/**
 * Mandarin adapter for sampled speech. Like its Japanese dictionary analyzer,
 * each visible character produces one group of phonemes that are merged into
 * a single chirp. pinyin-pro supplies phrase-aware pinyin and lexical tones.
 */
export class ChineseTextAnalyzer {
  constructor({ englishMode = "phonetic" } = {}) {
    this.englishAnalyzer = new EnglishAnalyzer();
    this.englishMode = DIALOGUE_ENGLISH_MODES.includes(englishMode) ? englishMode : "phonetic";
  }

  setEnglishMode(mode) {
    this.englishMode = DIALOGUE_ENGLISH_MODES.includes(mode) ? mode : "phonetic";
  }

  analyze(text) {
    const units = analyzeSpeechUnits(text);
    const groups = new Array(units.length);
    let index = 0;
    while (index < units.length) {
      const unit = units[index];
      if (unit.isCjk) {
        groups[index] = mergedTokens(unit.phonetic);
        index += 1;
        continue;
      }
      if (LATIN_CHARACTER.test(unit.character)) {
        let end = index + 1;
        while (end < units.length && LATIN_CHARACTER.test(units[end].character)) end += 1;
        const run = units.slice(index, end).map(({ character }) => character).join("");
        const routed = this.englishMode === "letters"
          ? Array.from(run, (character) => mergedTokens(character))
          : this.englishAnalyzer.analyze(run);
        for (let offset = 0; offset < routed.length; offset += 1) groups[index + offset] = routed[offset];
        index = end;
        continue;
      }
      if (Object.hasOwn(DIGIT_PHONETICS, unit.character)) {
        groups[index] = mergedTokens(DIGIT_PHONETICS[unit.character]);
        index += 1;
        continue;
      }
      groups[index] = [{ phoneme: unit.character, mergeWithNext: false }];
      index += 1;
    }
    return groups;
  }
}

/**
 * The upstream PitchManager still performs sample resampling and melody.
 * This subclass only feeds it Mandarin tone context before each utterance.
 */
export class MandarinPitchManager extends PitchManager {
  constructor({ toneStrength = 0.65, ...options } = {}) {
    super(options);
    this.basePitch = this.pitch;
    this.baseSpeed = this.speed;
    this.toneStrength = toneStrength;
    this.tones = [];
    this.toneCursor = 0;
  }

  prepare(text, rate = 1, tokenGroups = []) {
    const units = analyzeSpeechUnits(text);
    this.tones = tokenGroups.flatMap((tokens, index) => (
      tokens?.some(({ phoneme }) => SAMPLE_PHONEME.test(phoneme))
        ? [units[index]?.isCjk ? units[index].tone : 0]
        : []
    ));
    this.toneCursor = 0;
    this.pitch = this.basePitch * Math.pow(Math.max(0.5, Number(rate) || 1), 0.16);
    this.speed = this.baseSpeed * Math.max(0.5, Number(rate) || 1);
  }

  calculatePitch(characterIndex) {
    const melodicPitch = super.calculatePitch(characterIndex);
    const tone = this.tones[this.toneCursor++] || 0;
    const toneRatio = TONE_PITCH_RATIOS[tone] || TONE_PITCH_RATIOS[0];
    return melodicPitch * (1 + (toneRatio - 1) * this.toneStrength);
  }
}

export function isSampledDialogueVoice(voice) {
  return SAMPLED_VOICE_IDS.has(voice);
}

export class ScheduledWebPlayer extends WebPlayer {
  constructor(sampleRate) {
    super(sampleRate);
    this.activeSources = new Set();
  }

  playScheduled(buffer, { leadIn = 0.06 } = {}) {
    const context = this.audioContext;
    const audioBuffer = context.createBuffer(1, buffer.length, this.sampleRate || context.sampleRate);
    audioBuffer.copyToChannel(buffer, 0);
    const source = context.createBufferSource();
    const gain = context.createGain();
    source.buffer = audioBuffer;
    source.playbackRate.value = 1;
    gain.gain.value = this.volume;
    source.connect(gain).connect(context.destination);
    const startTime = context.currentTime + Math.max(0.04, Math.min(0.12, Number(leadIn) || 0.06));
    const fallbackLatency = (Number(context.baseLatency) || 0) + (Number(context.outputLatency) || 0);
    const duration = buffer.length / (this.sampleRate || context.sampleRate) * 1000;
    let lastPosition = (context.currentTime - startTime - fallbackLatency) * 1000;
    const playback = {
      context,
      source,
      startTime,
      duration,
      ended: false,
      position: () => {
        if (context.state === "closed") return duration;
        if (context.state === "suspended") return lastPosition;
        const timestamp = context.getOutputTimestamp?.();
        let candidate;
        if (timestamp && timestamp.performanceTime > 0 && Number.isFinite(timestamp.contextTime) && Number.isFinite(timestamp.performanceTime)) {
          const now = globalThis.performance?.now?.() ?? Date.now();
          const age = now - timestamp.performanceTime;
          if (age >= 0 && age < 250) {
            const outputContextTime = timestamp.contextTime + age / 1000;
            candidate = (outputContextTime - startTime) * 1000;
          }
        }
        if (!Number.isFinite(candidate)) candidate = (context.currentTime - startTime - fallbackLatency) * 1000;
        lastPosition = Math.max(lastPosition, candidate);
        return lastPosition;
      },
      stop: () => {
        if (playback.ended) return;
        try { source.stop(); } catch { /* source may not have started */ }
      },
    };
    source.addEventListener("ended", () => {
      playback.ended = true;
      this.activeSources.delete(source);
    }, { once: true });
    this.activeSources.add(source);
    source.start(startTime);
    return playback;
  }

  stopAll() {
    for (const source of this.activeSources) {
      try { source.stop(); } catch { /* source may already have ended */ }
    }
    this.activeSources.clear();
  }
}

export class ChineseSampledVoiceVoice {
  constructor() {
    this.records = new Map();
    this.recordPromises = new Map();
    this.spriteMapPromise = null;
  }

  async _spriteMap() {
    if (!this.spriteMapPromise) {
      this.spriteMapPromise = fetch(SPRITE_MAP_URL).then((response) => {
        if (!response.ok) throw new Error(`Unable to load SampledVoice sprite map: ${response.status}`);
        return response.json();
      });
    }
    return this.spriteMapPromise;
  }

  async _createRecord(voice) {
    const profile = VOICE_PROFILES[voice];
    if (!profile) return null;
    const spriteMap = await this._spriteMap();
    const sampler = new WebSampler(SPRITE_AUDIO_URL.href, spriteMap, {
      maxRetries: 2,
      minSilenceDurationMs: 50,
    });
    const analyzer = new ChineseTextAnalyzer();
    const effect = new MandarinPitchManager(profile);
    const engine = new SampledVoiceEngine({
      analyzer,
      sampler,
      effect,
      spaceDelay: 0.035,
      punctuationDelay: 0.24,
      punctuations: PUNCTUATIONS,
    });
    await engine.load();
    const player = new ScheduledWebPlayer(sampler.sampleRate);
    player.volume = profile.volume;
    const record = { analyzer, engine, effect, player, profile, sampleRate: sampler.sampleRate || 48000 };
    this.records.set(voice, record);
    return record;
  }

  async _record(voice) {
    if (!isSampledDialogueVoice(voice)) return null;
    if (this.records.has(voice)) return this.records.get(voice);
    if (!this.recordPromises.has(voice)) {
      const pending = this._createRecord(voice).catch((error) => {
        this.recordPromises.delete(voice);
        throw error;
      });
      this.recordPromises.set(voice, pending);
    }
    return this.recordPromises.get(voice);
  }

  async resume(voice) {
    const record = await this._record(voice);
    if (!record) return null;
    const context = record.player.audioContext;
    if (context.state === "suspended") await context.resume();
    return record;
  }

  async speech(text, voice, rate = 1, { englishMode = "phonetic" } = {}) {
    const record = await this.resume(voice);
    if (!record) return null;
    record.analyzer.setEnglishMode(englishMode);
    const tokenGroups = record.analyzer.analyze(text);
    record.effect.prepare(text, rate, tokenGroups);
    const outputs = [];
    for await (const output of record.engine.synthesize(text).speak()) outputs.push(output);
    const timeline = renderDialogueAudioTimeline(outputs, {
      sampleRate: record.sampleRate,
      crossfadeMs: record.profile.crossfadeMs,
      targetRms: voice === "playful" ? 0.105 : 0.115,
    });
    return Object.freeze({ ...timeline, voice });
  }

  play(buffer, voice) {
    const record = this.records.get(voice);
    if (!record || !(buffer instanceof Float32Array) || buffer.length === 0) return null;
    return record.player.playScheduled(buffer);
  }

  async suspend() {
    await Promise.all(Array.from(this.records.values(), async ({ player }) => {
      const context = player.audioContext;
      if (context.state === "running") await context.suspend();
    }));
  }

  stop() {
    for (const record of this.records.values()) {
      record.player.stopAll();
    }
  }
}
