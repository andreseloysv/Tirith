const clamp = (value, min, max) => Math.min(max, Math.max(min, value));

function measure(buffer) {
  let energy = 0;
  let peak = 0;
  for (const sample of buffer) {
    energy += sample * sample;
    peak = Math.max(peak, Math.abs(sample));
  }
  return {
    peak,
    rms: buffer.length ? Math.sqrt(energy / buffer.length) : 0,
  };
}

function normalizeChunk(buffer, targetRms) {
  const { rms } = measure(buffer);
  if (rms < 0.0005) return { buffer, voiced: false };
  const gain = clamp(targetRms / rms, 0.58, 1.75);
  const normalized = new Float32Array(buffer.length);
  for (let index = 0; index < buffer.length; index += 1) normalized[index] = buffer[index] * gain;
  return { buffer: normalized, voiced: true };
}

export function expandDialogueVisualCues(cues) {
  const visualCues = [];
  cues.forEach((cue, cueIndex) => {
    const characters = Array.from(cue.char || "");
    if (characters.length <= 1) {
      visualCues.push(Object.freeze({ ...cue, groupCharacter: cue.char, characterIndex: 0, characterCount: characters.length || 1 }));
      return;
    }
    const nextStart = cues[cueIndex + 1]?.start ?? cue.end;
    const visibleSpan = Math.max(0, Math.min(cue.duration, nextStart - cue.start || cue.duration));
    const spacing = visibleSpan / characters.length;
    characters.forEach((character, characterIndex) => {
      visualCues.push(Object.freeze({
        ...cue,
        char: character,
        groupCharacter: cue.char,
        characterIndex,
        characterCount: characters.length,
        start: cue.start + spacing * characterIndex,
        duration: spacing,
        end: cue.start + spacing * (characterIndex + 1),
      }));
    });
  });
  return Object.freeze(visualCues);
}

/**
 * Renders all synthesized chunks into one sample-accurate phrase buffer.
 * Adjacent voiced chunks overlap with an equal-power crossfade; punctuation
 * and spaces remain explicit silent regions and reset the overlap.
 */
export function renderDialogueAudioTimeline(outputs, {
  sampleRate = 48000,
  crossfadeMs = 10,
  targetRms = 0.115,
  ceiling = 0.94,
} = {}) {
  const safeSampleRate = Math.max(8000, Number(sampleRate) || 48000);
  const maxCrossfade = Math.max(0, Math.round(safeSampleRate * crossfadeMs / 1000));
  const chunks = [];
  let cursor = 0;
  let previous = null;

  for (const output of outputs) {
    const source = output.buffer instanceof Float32Array ? output.buffer : new Float32Array(0);
    const normalized = normalizeChunk(source, targetRms);
    const overlap = previous?.voiced && normalized.voiced
      ? Math.min(maxCrossfade, Math.floor(previous.buffer.length * 0.22), Math.floor(normalized.buffer.length * 0.22))
      : 0;
    const startSample = Math.max(0, cursor - overlap);
    const chunk = { ...normalized, output, overlap, startSample };
    chunks.push(chunk);
    cursor = startSample + normalized.buffer.length;
    previous = chunk;
  }

  const phrase = new Float32Array(cursor);
  for (const chunk of chunks) {
    const { buffer, overlap, startSample } = chunk;
    for (let index = 0; index < buffer.length; index += 1) {
      const outputIndex = startSample + index;
      if (index < overlap) {
        const progress = overlap <= 1 ? 1 : index / (overlap - 1);
        const previousGain = Math.cos(progress * Math.PI * 0.5);
        const nextGain = Math.sin(progress * Math.PI * 0.5);
        phrase[outputIndex] = phrase[outputIndex] * previousGain + buffer[index] * nextGain;
      } else {
        phrase[outputIndex] += buffer[index];
      }
    }
  }

  const { peak } = measure(phrase);
  if (peak > ceiling) {
    const gain = ceiling / peak;
    for (let index = 0; index < phrase.length; index += 1) phrase[index] *= gain;
  }

  const cues = Object.freeze(chunks.map((chunk) => Object.freeze({
    char: chunk.output.char,
    phoneme: chunk.output.phoneme,
    pitch: chunk.output.pitch,
    start: chunk.startSample / safeSampleRate * 1000,
    duration: chunk.buffer.length / safeSampleRate * 1000,
    end: (chunk.startSample + chunk.buffer.length) / safeSampleRate * 1000,
    voiced: chunk.voiced,
  })));
  const visualCues = expandDialogueVisualCues(cues);

  return Object.freeze({
    buffer: phrase,
    cues,
    visualCues,
    duration: phrase.length / safeSampleRate * 1000,
    sampleRate: safeSampleRate,
  });
}
