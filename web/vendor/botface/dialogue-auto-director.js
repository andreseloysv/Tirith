import { MORPH_IDS, STATE_IDS } from "./catalog.js";

const availableStates = new Set(STATE_IDS);
const availableMorphs = new Set(MORPH_IDS);

const MOODS = Object.freeze({
  greeting: Object.freeze({
    states: Object.freeze(["happy", "playful", "idle"]),
    morphs: Object.freeze(["wave"]),
    morphChance: 0.72,
  }),
  question: Object.freeze({
    states: Object.freeze(["curious", "thinking", "confused"]),
    morphs: Object.freeze(["dots"]),
    morphChance: 0.34,
  }),
  thinking: Object.freeze({
    states: Object.freeze(["thinking", "curious", "searching"]),
    morphs: Object.freeze(["dots", "radar"]),
    morphChance: 0.3,
  }),
  celebrate: Object.freeze({
    states: Object.freeze(["celebrate", "excited", "proud"]),
    morphs: Object.freeze(["ball", "gather"]),
    morphChance: 0.78,
  }),
  positive: Object.freeze({
    states: Object.freeze(["happy", "excited", "proud"]),
    morphs: Object.freeze(["wave", "ball"]),
    morphChance: 0.44,
  }),
  playful: Object.freeze({
    states: Object.freeze(["playful", "laughing", "happy"]),
    morphs: Object.freeze(["ball", "wave"]),
    morphChance: 0.62,
  }),
  alert: Object.freeze({
    states: Object.freeze(["surprised", "scared", "alerting"]),
    morphs: Object.freeze(["bang"]),
    morphChance: 0.7,
  }),
  sad: Object.freeze({
    states: Object.freeze(["sad", "shy", "drowsy"]),
    morphs: Object.freeze([]),
    morphChance: 0,
  }),
  angry: Object.freeze({
    states: Object.freeze(["angry", "suspicious", "confused"]),
    morphs: Object.freeze(["bang"]),
    morphChance: 0.28,
  }),
  conclusion: Object.freeze({
    states: Object.freeze(["proud", "happy", "idle"]),
    morphs: Object.freeze(["gather"]),
    morphChance: 0.38,
  }),
  neutral: Object.freeze({
    states: Object.freeze(["idle", "listening", "happy"]),
    morphs: Object.freeze([]),
    morphChance: 0,
  }),
});

const MOOD_RULES = Object.freeze([
  ["celebrate", /恭喜|庆祝|成功了|完成了|搞定|太棒|万岁|congrat|celebrat|hooray|nailed it/iu],
  ["alert", /警告|危险|注意|小心|吓|害怕|震惊|竟然|哇|warning|danger|careful|scared|shocked|wow/iu],
  ["sad", /抱歉|遗憾|难过|伤心|失败|糟糕|sorry|sad|unfortunately|failed|terrible/iu],
  ["angry", /生气|愤怒|讨厌|可恶|不能接受|angry|furious|hate|unacceptable/iu],
  ["playful", /哈哈|嘿嘿|开玩笑|逗你|好玩|haha|hehe|lol|just kidding|funny/iu],
  ["question", /[？?]|为什么|怎么|如何|什么|哪[个里]|是否|能否|可以吗|是不是|\b(?:why|how|what|where|when|who|can|could|would|should|is|are|do|does)\b/iu],
  ["thinking", /让我想|思考|分析|考虑|原因|也许|可能|不确定|think|consider|analy[sz]e|maybe|perhaps|reason/iu],
  ["greeting", /你好|您好|嗨|欢迎|早上好|下午好|晚上好|\b(?:hello|hi|hey|welcome|morning)\b/iu],
  ["conclusion", /所以|总之|结论|最后|最终|结果是|换句话说|therefore|finally|in short|the result/iu],
  ["positive", /太好|有了|当然|可以|喜欢|开心|不错|很好|找到|完成|great|good|done|found|love|happy|awesome|yes/iu],
]);

function hashSeed(value) {
  let hash = 2166136261;
  for (const character of String(value)) {
    hash ^= character.codePointAt(0);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

function seededRandom(seed) {
  let value = seed >>> 0;
  return () => {
    value += 0x6d2b79f5;
    let mixed = value;
    mixed = Math.imul(mixed ^ mixed >>> 15, mixed | 1);
    mixed ^= mixed + Math.imul(mixed ^ mixed >>> 7, mixed | 61);
    return ((mixed ^ mixed >>> 14) >>> 0) / 4294967296;
  };
}

function pick(values, random, avoid = null) {
  const choices = values.filter((value) => value !== avoid);
  const pool = choices.length ? choices : values;
  return pool[Math.floor(random() * pool.length)];
}

function roundedBetween(minimum, maximum, random, step = 10) {
  return Math.round((minimum + (maximum - minimum) * random()) / step) * step;
}

function splitIntoClauses(text) {
  const clauses = [];
  let buffer = "";
  const characters = Array.from(text);
  const breaker = /[，,；;：:。！？!?\n…]/u;
  for (let index = 0; index < characters.length; index += 1) {
    const character = characters[index];
    buffer += character;
    if (!breaker.test(character)) continue;
    if (breaker.test(characters[index + 1] || "")) continue;
    clauses.push(buffer);
    buffer = "";
  }
  if (buffer) clauses.push(buffer);
  return clauses.filter(Boolean);
}

function classifyMood(text) {
  for (const [mood, pattern] of MOOD_RULES) {
    if (pattern.test(text)) return mood;
  }
  return "neutral";
}

function punctuationPause(text, random) {
  const trimmed = text.trimEnd();
  if (!trimmed) return 0;
  if (/(?:…{2,}|\.{3,})$/u.test(trimmed)) return roundedBetween(480, 720, random, 20);
  if (/\n+$/u.test(text)) return roundedBetween(360, 580, random, 20);
  if (/[；;：:]$/u.test(trimmed) && random() < 0.72) return roundedBetween(280, 440, random, 20);
  if (/[，,]$/u.test(trimmed) && random() < 0.38) return roundedBetween(180, 300, random, 20);
  if (/[。！？!?]$/u.test(trimmed) && random() < 0.32) return roundedBetween(220, 380, random, 20);
  return 0;
}

function visibleLength(text) {
  return Array.from(text.replace(/[\s，,；;：:。！？!?…]/gu, "")).length;
}

function actionValue(node) {
  return node.state ?? node.angle ?? node.effect ?? "pause";
}

/**
 * Builds a text-preserving dialogue script with semantically constrained
 * variation. Pass a seed to reproduce a take; omit it to create a fresh take.
 */
export function planDialogueActions(text, { seed } = {}) {
  if (typeof text !== "string" || !text.trim()) throw new TypeError("Dialogue text must contain visible characters");
  const normalizedSeed = hashSeed(seed ?? `${text}\u0000${Date.now()}\u0000${Math.random()}`);
  const random = seededRandom(normalizedSeed);
  const clauses = splitIntoClauses(text);
  const maxActions = Math.min(14, Math.max(3, Math.ceil(visibleLength(text) / 7) + 2));
  const script = [];
  let actionCount = 0;
  let previousMood = null;
  let previousState = null;
  let previousRotation = 0;

  const addAction = (node) => {
    if (actionCount >= maxActions) return false;
    script.push(node);
    actionCount += 1;
    return true;
  };

  clauses.forEach((clause, index) => {
    if (!clause.trim()) {
      script.push({ type: "text", text: clause });
      return;
    }

    const mood = classifyMood(clause);
    const profile = MOODS[mood];
    const moodChanged = mood !== previousMood;
    const strongMood = !["neutral", "greeting", "positive"].includes(mood);
    const shouldSetState = index === 0 || (moodChanged && (strongMood || random() < 0.7)) || random() < 0.2;
    let insertedState = false;
    if (shouldSetState) {
      const state = pick(profile.states.filter((value) => availableStates.has(value)), random, previousState);
      insertedState = addAction({ type: "state", state, duration: roundedBetween(260, 460, random, 20) });
      if (insertedState) previousState = state;
    }

    const clauseLength = visibleLength(clause);
    const shouldRotate = index > 0 && clauseLength >= 3 && random() < (insertedState ? 0.2 : 0.38);
    if (shouldRotate) {
      const rotation = pick([-24, -12, 12, 24], random, previousRotation);
      if (addAction({ type: "rotate", angle: rotation, duration: roundedBetween(220, 360, random, 20) })) previousRotation = rotation;
    }

    script.push({ type: "text", text: clause });

    if (profile.morphs.length && random() < profile.morphChance) {
      const morphs = profile.morphs.filter((value) => availableMorphs.has(value));
      if (morphs.length) addAction({ type: "morph", effect: pick(morphs, random), duration: roundedBetween(580, 820, random, 20) });
    }

    const pause = punctuationPause(clause, random);
    if (pause && index < clauses.length - 1) addAction({ type: "pause", duration: pause });
    previousMood = mood;
  });

  const actions = script.filter(({ type }) => type !== "text");
  const signature = actions.map((node) => `${node.type}:${actionValue(node)}`).join("|");
  return Object.freeze({
    script: Object.freeze(script.map((node) => Object.freeze(node))),
    seed: normalizedSeed,
    actionCount: actions.length,
    clauseCount: clauses.filter((clause) => clause.trim()).length,
    signature,
  });
}

export { classifyMood, splitIntoClauses };
