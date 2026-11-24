import { pinyin } from "./vendor/pinyin/index.mjs";

const CJK_PATTERN = /[\u3400-\u9fff\uf900-\ufaff]/u;
const SPEAKABLE_PATTERN = /[\p{L}\p{N}]/u;
const VOWEL_BY_LETTER = Object.freeze(["a", "i", "u", "e", "o"]);

function hashString(value) {
  let hash = 2166136261;
  for (const character of value) {
    hash ^= character.codePointAt(0) || 0;
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}
function vowelFamily(source, fallbackHash) {
  const normalized = String(source || "").toLowerCase().replaceAll("ü", "v");
  if (normalized.includes("a")) return "a";
  if (normalized.includes("e")) return "e";
  if (normalized.includes("i")) return "i";
  if (normalized.includes("o")) return "o";
  if (normalized.includes("u") || normalized.includes("v")) return "u";
  return VOWEL_BY_LETTER[fallbackHash % VOWEL_BY_LETTER.length];
}

export function analyzeSpeechUnits(text) {
  const source = String(text ?? "");
  const analyzed = pinyin(source, {
    type: "all",
    toneType: "none",
    toneSandhi: true,
    traditional: true,
  });

  return Object.freeze(analyzed.map((entry) => {
    const character = entry.origin;
    const isCjk = CJK_PATTERN.test(character) && entry.isZh;
    const phonetic = isCjk ? entry.pinyin : character.toLowerCase();
    const hash = hashString(phonetic || character);
    const initial = isCjk ? entry.initial : (/^[a-z]$/i.test(character) ? character.toLowerCase() : "");
    const final = isCjk ? entry.final : "";
    return Object.freeze({
      character,
      phonetic,
      initial,
      final,
      vowel: vowelFamily(entry.finalBody || final || phonetic, hash),
      tone: isCjk ? Number(entry.num) || 0 : 0,
      isCjk,
      speakable: SPEAKABLE_PATTERN.test(character),
      hash,
    });
  }));
}
