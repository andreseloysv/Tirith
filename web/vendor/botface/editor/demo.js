import {
  BOT_FACE_EFFECTS,
  BOT_FACE_SHAPES,
} from "../bot-face.js";
import {
  MORPH_LABELS_ZH as effectLabels,
  SHAPE_LABELS_ZH as shapeLabels,
  STATE_IDS as orderedStates,
  STATE_LABELS_ZH as stateLabels,
} from "../catalog.js";
import {
  DEFAULT_MATERIAL,
  GLASS_PRESETS,
  GRADIENT_PRESETS,
  MATERIAL_IDS,
  MATERIAL_LABELS,
  SOLID_PRESETS,
  materialCssBackground,
  resolveMaterial,
} from "../materials.js";
import { setupDialogueWorkbench } from "./dialogue-editor.js";
const stateInput = document.querySelector("#demo-state");
const shapeInput = document.querySelector("#demo-shape");
const stateGrid = document.querySelector("#state-grid");
const shapeGrid = document.querySelector("#shape-grid");
const sizeInput = document.querySelector("#demo-size");
const sizeOutput = document.querySelector("#demo-size-output");
const eyeColorInput = document.querySelector("#demo-eye-color");
const speedSelect = document.querySelector("#demo-speed");
const pointerInput = document.querySelector("#demo-pointer");
const pauseButton = document.querySelector("#demo-pause");
const sequenceList = document.querySelector("#sequence-list");
const sequenceMorphGrid = document.querySelector("#sequence-morph-grid");
const selectedSequenceStepOutput = document.querySelector("#selected-sequence-step");
const sequenceLoop = document.querySelector("#sequence-loop");
const sequenceSummary = document.querySelector("#sequence-summary");
const addSequenceStepButton = document.querySelector("#add-sequence-step");
const previewSequenceButton = document.querySelector("#preview-sequence");
const stopSequenceButton = document.querySelector("#stop-sequence");
const bot = document.querySelector("#demo-bot");
const previewStage = document.querySelector("#preview-stage");
const contextTitle = document.querySelector("#context-title");
const contextDescription = document.querySelector("#context-description");
const readout = document.querySelector("#demo-readout");
const runtime = document.querySelector("#demo-runtime");
const code = document.querySelector("#component-code");
const materialTabs = document.querySelector("#material-tabs");
const materialPresets = document.querySelector("#material-presets");
const materialCustom = document.querySelector("#material-custom");
const materialReadout = document.querySelector("#material-readout");
const sequenceWorkspace = document.querySelector("#sequence-workspace");
const dialogueWorkspace = document.querySelector("#dialogue-workspace");

let codeMode = "static";
let previewMode = "sequence";
let dialogueWorkbench = null;
let selectedStepIndex = 1;
let playingStepIndex = -1;
let sequenceRun = 0;
let sequencePlaying = false;
const materialConfig = { ...DEFAULT_MATERIAL };
const sequence = [
  { state: "idle", hold: 1000, morph: "gather", morphHold: 700 },
  { state: "thinking", hold: 2400, morph: "send", morphHold: 700 },
  { state: "celebrate", hold: 1600, morph: null, morphHold: 1000 },
];

function botThumbnail({ state, shape, size }) {
  const preview = document.createElement("bot-face");
  preview.setAttribute("state", state);
  preview.setAttribute("shape", shape);
  preview.setAttribute("size", size);
  preview.setAttribute("decorative", "");
  preview.setAttribute("thumbnail", "");
  return preview;
}

for (const state of orderedStates) {
  const button = document.createElement("button");
  button.type = "button";
  button.dataset.stateOption = state;
  button.setAttribute("aria-label", `选择${stateLabels[state]}状态`);
  button.append(botThumbnail({ state, shape: "blob", size: 34 }));
  const label = document.createElement("span");
  label.className = "state-option-copy";
  label.innerHTML = `<strong>${stateLabels[state]}</strong><code>${state}</code>`;
  const markers = document.createElement("span");
  markers.className = "state-role-markers";
  button.append(label, markers);
  stateGrid.append(button);
}

for (const shape of BOT_FACE_SHAPES) {
  const button = document.createElement("button");
  button.type = "button";
  button.dataset.shapeOption = shape;
  button.setAttribute("aria-label", `选择${shapeLabels[shape]}形状`);
  button.append(botThumbnail({ state: "idle", shape, size: 38 }));
  const label = document.createElement("span");
  label.innerHTML = `<strong>${shapeLabels[shape]}</strong><code>${shape}</code>`;
  button.append(label);
  shapeGrid.append(button);
}

shapeInput.value = "blob";
stateInput.value = sequence[selectedStepIndex].state;

function escapeAttribute(value) {
  return String(value).replaceAll("&", "&amp;").replaceAll('"', "&quot;");
}

function presetCatalog(material = materialConfig.material) {
  if (material === "solid") return SOLID_PRESETS;
  if (material === "gradient") return GRADIENT_PRESETS;
  return GLASS_PRESETS;
}

function presetBackground(material, preset) {
  return materialCssBackground(material, preset);
}

function applySuggestedEyeColor(preset = null) {
  if (preset?.eyeColor) {
    if (eyeColorInput.value.toLowerCase() === "#ffffff" || eyeColorInput.dataset.materialSuggested === "true") {
      eyeColorInput.value = preset.eyeColor;
      eyeColorInput.dataset.materialSuggested = "true";
    }
    return;
  }
  if (eyeColorInput.dataset.materialSuggested === "true") {
    eyeColorInput.value = "#ffffff";
    delete eyeColorInput.dataset.materialSuggested;
  }
}

function setMaterialAttributes(element) {
  element.setAttribute("material", materialConfig.material);
  element.setAttribute("color", materialConfig.color);
  if (materialConfig.material === "gradient") {
    if (materialConfig.gradientPreset === "custom") {
      element.removeAttribute("gradient-preset");
      element.setAttribute("gradient-start", materialConfig.gradientStart);
      element.setAttribute("gradient-end", materialConfig.gradientEnd);
      element.setAttribute("gradient-angle", String(materialConfig.gradientAngle));
    } else {
      element.setAttribute("gradient-preset", materialConfig.gradientPreset);
      element.removeAttribute("gradient-start");
      element.removeAttribute("gradient-end");
      element.removeAttribute("gradient-angle");
    }
  } else {
    element.removeAttribute("gradient-preset");
    element.removeAttribute("gradient-start");
    element.removeAttribute("gradient-end");
    element.removeAttribute("gradient-angle");
  }
  if (materialConfig.material === "rainbow-glass") element.setAttribute("glass-preset", materialConfig.glassPreset);
  else element.removeAttribute("glass-preset");
}

function syncMaterialSelection() {
  const resolved = resolveMaterial(materialConfig);
  materialTabs.querySelectorAll("[data-material]").forEach((button) => {
    const selected = button.dataset.material === materialConfig.material;
    button.classList.toggle("is-selected", selected);
    button.setAttribute("aria-pressed", String(selected));
  });
  materialPresets.querySelectorAll("[data-material-preset]").forEach((button) => {
    const selected = materialConfig.material === "solid"
      ? button.dataset.materialPreset === resolved.preset
      : button.dataset.materialPreset === (materialConfig.material === "gradient" ? materialConfig.gradientPreset : materialConfig.glassPreset);
    button.classList.toggle("is-selected", selected);
    button.setAttribute("aria-pressed", String(selected));
  });
  const preset = presetCatalog().find(({ id }) => id === resolved.preset);
  materialReadout.textContent = `${MATERIAL_LABELS[materialConfig.material].zh} · ${preset?.label.zh || "自定义"}`;
}

function materialControl(label, type, value, attributes = {}) {
  const control = document.createElement("label");
  control.innerHTML = `<span>${label}</span>`;
  const input = document.createElement("input");
  input.type = type;
  input.value = String(value);
  for (const [name, setting] of Object.entries(attributes)) input.setAttribute(name, String(setting));
  control.append(input);
  return { control, input };
}

function renderMaterialCustom() {
  materialCustom.replaceChildren();
  if (materialConfig.material === "solid") {
    const custom = materialControl("自定义纯色", "color", materialConfig.color);
    custom.input.addEventListener("input", () => {
      materialConfig.color = custom.input.value;
      syncMaterialSelection();
      syncDemo();
    });
    materialCustom.append(custom.control);
    return;
  }
  if (materialConfig.material === "gradient") {
    const resolved = resolveMaterial(materialConfig);
    if (resolved.kind === "soft") {
      const note = document.createElement("p");
      note.textContent = "柔焦预设由近白雾底与 4 个大尺度径向色团组成；色团固定在镜头方向，并自动裁切到当前形状。选择其他预设即可快速更换整套配色。";
      materialCustom.append(note);
      return;
    }
    const start = materialControl("起始色", "color", materialConfig.gradientPreset === "custom" ? materialConfig.gradientStart : resolved.stops[0].color);
    const end = materialControl("结束色", "color", materialConfig.gradientPreset === "custom" ? materialConfig.gradientEnd : resolved.stops.at(-1).color);
    const angle = materialControl("方向", "range", materialConfig.gradientPreset === "custom" ? materialConfig.gradientAngle : resolved.angle, { min: 0, max: 360, step: 1 });
    const output = document.createElement("output");
    output.textContent = `${angle.input.value}°`;
    angle.control.append(output);
    const update = () => {
      materialConfig.gradientPreset = "custom";
      materialConfig.gradientStart = start.input.value;
      materialConfig.gradientEnd = end.input.value;
      materialConfig.gradientAngle = Number(angle.input.value);
      output.textContent = `${angle.input.value}°`;
      syncMaterialSelection();
      syncDemo();
    };
    for (const input of [start.input, end.input, angle.input]) input.addEventListener("input", update);
    materialCustom.append(start.control, end.control, angle.control);
    return;
  }
  const note = document.createElement("p");
  note.textContent = "玻璃预设由彩虹基底、体积暗部、局部高光和折射轮廓组成，并会自动贴合全部 18 种形状。";
  materialCustom.append(note);
}

function renderMaterialEditor() {
  materialTabs.replaceChildren();
  for (const material of MATERIAL_IDS) {
    const button = document.createElement("button");
    button.type = "button";
    button.dataset.material = material;
    button.innerHTML = `<strong>${MATERIAL_LABELS[material].zh}</strong><code>${material}</code>`;
    button.addEventListener("click", () => {
      stopEditorSequence();
      materialConfig.material = material;
      if (material !== "gradient") applySuggestedEyeColor();
      renderMaterialEditor();
      syncDemo();
    });
    materialTabs.append(button);
  }
  materialPresets.replaceChildren();
  for (const preset of presetCatalog()) {
    const button = document.createElement("button");
    button.type = "button";
    button.dataset.materialPreset = preset.id;
    button.innerHTML = `<i style="--material-swatch:${presetBackground(materialConfig.material, preset)}"></i><span><strong>${preset.label.zh}</strong><code>${preset.id}</code></span>`;
    button.addEventListener("click", () => {
      stopEditorSequence();
      if (materialConfig.material === "solid") materialConfig.color = preset.color;
      else if (materialConfig.material === "gradient") materialConfig.gradientPreset = preset.id;
      else materialConfig.glassPreset = preset.id;
      applySuggestedEyeColor(preset);
      renderMaterialEditor();
      syncDemo();
    });
    materialPresets.append(button);
  }
  renderMaterialCustom();
  syncMaterialSelection();
}

function componentAttributes(state) {
  const attributes = [
    `state="${escapeAttribute(state)}"`,
    `shape="${escapeAttribute(shapeInput.value)}"`,
    `size="${sizeInput.value}"`,
  ];
  if (materialConfig.material === "solid") attributes.push(`color="${escapeAttribute(materialConfig.color)}"`);
  else attributes.push(`material="${materialConfig.material}"`);
  if (materialConfig.material === "gradient") {
    if (materialConfig.gradientPreset === "custom") {
      attributes.push(
        `gradient-start="${materialConfig.gradientStart}"`,
        `gradient-end="${materialConfig.gradientEnd}"`,
        `gradient-angle="${materialConfig.gradientAngle}"`,
      );
    } else attributes.push(`gradient-preset="${materialConfig.gradientPreset}"`);
  }
  if (materialConfig.material === "rainbow-glass") attributes.push(`glass-preset="${materialConfig.glassPreset}"`);
  if (eyeColorInput.value.toLowerCase() !== "#ffffff") attributes.push(`eye-color="${escapeAttribute(eyeColorInput.value)}"`);
  if (speedSelect.value !== "1") attributes.push(`speed="${speedSelect.value}"`);
  if (pointerInput.checked) attributes.push("follow-pointer");
  return attributes.join("\n  ");
}

function staticCode() {
  return `<script type="module" src="./bot-face/bot-face.js"></script>\n\n<bot-face\n  ${componentAttributes(stateInput.value)}\n  label="${stateLabels[stateInput.value]}动画"\n></bot-face>`;
}

function sequenceStepCode(step) {
  const fields = [`state: "${step.state}"`, `hold: ${step.hold}`];
  if (step.morph) fields.push(`morph: "${step.morph}"`, `morphHold: ${step.morphHold}`);
  return `  { ${fields.join(", ")} }`;
}

function sequenceCode() {
  const first = sequence[0];
  const steps = sequence.map(sequenceStepCode).join(",\n");
  return `<script type="module" src="./bot-face/bot-face.js"></script>\n\n<button id="start-bot-sequence">播放动画</button>\n<button id="stop-bot-sequence">停止</button>\n\n<bot-face\n  id="status-bot"\n  ${componentAttributes(first.state)}\n  label="任务状态"\n></bot-face>\n\n<script type="module">\n  const bot = document.querySelector("#status-bot");\n  const sequence = [\n${steps}\n  ];\n\n  document.querySelector("#start-bot-sequence")\n    .addEventListener("click", () => {\n      bot.playSequence(sequence, { loop: ${sequenceLoop.checked} });\n    });\n\n  document.querySelector("#stop-bot-sequence")\n    .addEventListener("click", () => bot.stopSequence());\n</script>`;
}

function dialogueCode() {
  const script = JSON.stringify(dialogueWorkbench?.script || [], null, 2);
  const indentedScript = script.split("\n").map((line, index) => index ? `  ${line}` : line).join("\n");
  return `<script type="module" src="./bot-face/bot-face.js"></script>\n\n<button id="play-bot-dialogue">播放对话</button>\n\n<bot-face\n  id="dialogue-bot"\n  ${componentAttributes(stateInput.value)}\n  label="会表演的对话角色"\n></bot-face>\n\n<script type="module">\n  const bot = document.querySelector("#dialogue-bot");\n  const dialogue = ${indentedScript};\n\n  document.querySelector("#play-bot-dialogue")\n    .addEventListener("click", () => {\n      bot.performDialogue(dialogue, {\n        voice: "${dialogueWorkbench?.voice || "playful"}",\n        englishMode: "${dialogueWorkbench?.englishMode || "phonetic"}",\n        rate: ${dialogueWorkbench?.rate || 1}\n      });\n    });\n</script>`;
}

function contextText(state) {
  const label = stateLabels[state];
  if (state === "idle") return ["等待任务", "状态可以随时切换"];
  if (state === "celebrate") return ["任务已完成", "结果已经准备好了"];
  if (["sad", "scared", "confused", "alerting"].includes(state)) return [`当前状态：${label}`, "你可以随时切换状态"];
  return [`正在${label}`, "通常只需要几秒钟"];
}

function renderSequence() {
  selectedStepIndex = Math.max(0, Math.min(selectedStepIndex, sequence.length - 1));
  sequenceList.replaceChildren();
  sequence.forEach((step, index) => {
    const article = document.createElement("article");
    article.className = "sequence-step";
    article.dataset.sequenceIndex = String(index);
    article.classList.toggle("is-selected", index === selectedStepIndex);
    article.classList.toggle("is-playing", index === playingStepIndex);
    article.innerHTML = `
      <button class="sequence-identity" type="button" data-select-sequence-step aria-pressed="${index === selectedStepIndex}">
        <span class="sequence-number">${index + 1}</span>
        <bot-face state="${step.state}" shape="${shapeInput.value}" size="34" eye-color="${eyeColorInput.value}" thumbnail decorative></bot-face>
        <span><strong>${stateLabels[step.state]}</strong><code>${step.state}</code></span>
      </button>
      <label class="sequence-field"><span>状态停留</span><input data-step-hold type="number" min="0" max="600" step="0.1" value="${step.hold / 1000}" /><i>秒</i></label>
      <div class="sequence-morph-readout"><span>停留后播放</span><strong>${step.morph ? effectLabels[step.morph] : "不触发"}</strong><code>${step.morph || "none"}</code></div>
      <label class="sequence-field${step.morph ? "" : " is-disabled"}"><span>Morph 保持</span><input data-step-morph-hold type="number" min="0" max="600" step="0.1" value="${step.morphHold / 1000}"${step.morph ? "" : " disabled"} /><i>秒</i></label>
      <div class="sequence-actions">
        <button type="button" data-sequence-action="up" aria-label="上移第 ${index + 1} 步"${index === 0 ? " disabled" : ""}>↑</button>
        <button type="button" data-sequence-action="down" aria-label="下移第 ${index + 1} 步"${index === sequence.length - 1 ? " disabled" : ""}>↓</button>
        <button type="button" data-sequence-action="delete" aria-label="删除第 ${index + 1} 步"${sequence.length === 1 ? " disabled" : ""}>删除</button>
      </div>`;
    sequenceList.append(article);
  });
  renderMorphPalette();
  syncSequenceSummary();
  syncStateMarkers();
}

function renderMorphPalette() {
  const selected = sequence[selectedStepIndex].morph;
  selectedSequenceStepOutput.textContent = `第 ${selectedStepIndex + 1} 步`;
  sequenceMorphGrid.replaceChildren();
  for (const effect of [null, ...BOT_FACE_EFFECTS]) {
    const button = document.createElement("button");
    button.type = "button";
    button.dataset.morphOption = effect || "none";
    button.classList.toggle("is-selected", selected === effect);
    button.setAttribute("aria-pressed", String(selected === effect));
    button.innerHTML = `<strong>${effect ? effectLabels[effect] : "不触发"}</strong><code>${effect || "none"}</code>`;
    sequenceMorphGrid.append(button);
  }
}

function syncSequenceSummary(text = "") {
  const stateHold = sequence.reduce((total, step) => total + step.hold, 0) / 1000;
  sequenceSummary.textContent = text || `${sequence.length} 步 · 状态停留 ${Number(stateHold.toFixed(1))} 秒`;
}

function syncStateMarkers() {
  stateGrid.querySelectorAll("[data-state-option]").forEach((button) => {
    const state = button.dataset.stateOption;
    const markers = sequence.flatMap((step, index) => step.state === state
      ? [`<i class="sequence-role${index === selectedStepIndex ? " is-selected" : ""}">${index + 1}</i>`]
      : []);
    button.querySelector(".state-role-markers").innerHTML = markers.join("");
    button.classList.toggle("has-sequence-step", markers.length > 0);
  });
}

function syncThumbnailAppearance() {
  stateGrid.querySelectorAll("bot-face").forEach((preview) => {
    setMaterialAttributes(preview);
    if (preview.getAttribute("eye-color") !== eyeColorInput.value) preview.setAttribute("eye-color", eyeColorInput.value);
  });
  shapeGrid.querySelectorAll("bot-face").forEach((preview) => {
    setMaterialAttributes(preview);
    if (preview.getAttribute("eye-color") !== eyeColorInput.value) preview.setAttribute("eye-color", eyeColorInput.value);
  });
  sequenceList.querySelectorAll("bot-face").forEach((stepBot) => {
    if (stepBot.shape !== shapeInput.value) stepBot.shape = shapeInput.value;
    setMaterialAttributes(stepBot);
    if (stepBot.getAttribute("eye-color") !== eyeColorInput.value) stepBot.setAttribute("eye-color", eyeColorInput.value);
  });
}

function setCodeMode(mode) {
  codeMode = mode;
  document.querySelectorAll("[data-code-mode]").forEach((button) => button.classList.toggle("is-active", button.dataset.codeMode === codeMode));
}

function syncSequencePlaybackUI() {
  sequenceList.querySelectorAll("[data-sequence-index]").forEach((row) => {
    row.classList.toggle("is-selected", Number(row.dataset.sequenceIndex) === selectedStepIndex);
    row.classList.toggle("is-playing", Number(row.dataset.sequenceIndex) === playingStepIndex);
    row.querySelector("[data-select-sequence-step]").setAttribute("aria-pressed", String(Number(row.dataset.sequenceIndex) === selectedStepIndex));
  });
  previewSequenceButton.disabled = sequencePlaying;
  previewSequenceButton.innerHTML = sequencePlaying ? "正在播放…" : "<span>▶</span> 播放时间线";
  stopSequenceButton.disabled = !sequencePlaying;
}

function stopEditorSequence({ resetSummary = true } = {}) {
  sequenceRun += 1;
  bot.stopSequence();
  if (previewMode === "dialogue") dialogueWorkbench?.stop();
  sequencePlaying = false;
  playingStepIndex = -1;
  if (resetSummary) syncSequenceSummary();
  syncSequencePlaybackUI();
}

function syncDemo() {
  bot.state = stateInput.value;
  bot.shape = shapeInput.value;
  bot.size = Number(sizeInput.value);
  setMaterialAttributes(bot);
  bot.setAttribute("eye-color", eyeColorInput.value);
  bot.speed = Number(speedSelect.value);
  bot.toggleAttribute("follow-pointer", pointerInput.checked);
  sizeOutput.textContent = `${sizeInput.value}px`;
  readout.textContent = `${stateInput.value} · ${shapeInput.value} · ${MATERIAL_LABELS[materialConfig.material].zh} · ${sizeInput.value}px`;
  [contextTitle.textContent, contextDescription.textContent] = contextText(stateInput.value);
  code.textContent = codeMode === "sequence" ? sequenceCode() : codeMode === "dialogue" ? dialogueCode() : staticCode();
  stateGrid.querySelectorAll("[data-state-option]").forEach((button) => button.classList.toggle("is-active", button.dataset.stateOption === stateInput.value));
  shapeGrid.querySelectorAll("[data-shape-option]").forEach((button) => button.classList.toggle("is-active", button.dataset.shapeOption === shapeInput.value));
  syncStateMarkers();
  syncThumbnailAppearance();
}

[sizeInput, speedSelect, pointerInput].forEach((input) => input.addEventListener("input", syncDemo));
eyeColorInput.addEventListener("input", () => {
  delete eyeColorInput.dataset.materialSuggested;
  syncDemo();
});

stateGrid.addEventListener("click", (event) => {
  const button = event.target.closest("[data-state-option]");
  if (!button) return;
  stopEditorSequence();
  sequence[selectedStepIndex].state = button.dataset.stateOption;
  stateInput.value = button.dataset.stateOption;
  setCodeMode("sequence");
  renderSequence();
  syncDemo();
  bot.replay();
});

shapeGrid.addEventListener("click", (event) => {
  const button = event.target.closest("[data-shape-option]");
  if (!button) return;
  shapeInput.value = button.dataset.shapeOption;
  syncDemo();
});

sequenceList.addEventListener("click", (event) => {
  const row = event.target.closest("[data-sequence-index]");
  if (!row) return;
  const index = Number(row.dataset.sequenceIndex);
  if (event.target.closest("[data-select-sequence-step]")) {
    stopEditorSequence();
    selectedStepIndex = index;
    stateInput.value = sequence[index].state;
    renderSequence();
    syncDemo();
    bot.replay();
    return;
  }

  const action = event.target.closest("[data-sequence-action]")?.dataset.sequenceAction;
  if (!action) return;
  stopEditorSequence();
  if (action === "delete" && sequence.length > 1) {
    sequence.splice(index, 1);
    selectedStepIndex = Math.min(index, sequence.length - 1);
  } else if (action === "up" && index > 0) {
    [sequence[index - 1], sequence[index]] = [sequence[index], sequence[index - 1]];
    selectedStepIndex = index - 1;
  } else if (action === "down" && index < sequence.length - 1) {
    [sequence[index + 1], sequence[index]] = [sequence[index], sequence[index + 1]];
    selectedStepIndex = index + 1;
  }
  stateInput.value = sequence[selectedStepIndex].state;
  setCodeMode("sequence");
  renderSequence();
  syncDemo();
  bot.replay();
});

sequenceList.addEventListener("input", (event) => {
  const row = event.target.closest("[data-sequence-index]");
  if (!row) return;
  const step = sequence[Number(row.dataset.sequenceIndex)];
  if (event.target.matches("[data-step-hold]")) step.hold = Math.max(0, Math.round(Number(event.target.value || 0) * 1000));
  if (event.target.matches("[data-step-morph-hold]")) step.morphHold = Math.max(0, Math.round(Number(event.target.value || 0) * 1000));
  stopEditorSequence({ resetSummary: false });
  setCodeMode("sequence");
  syncSequenceSummary();
  syncDemo();
});

sequenceMorphGrid.addEventListener("click", (event) => {
  const button = event.target.closest("[data-morph-option]");
  if (!button) return;
  stopEditorSequence();
  sequence[selectedStepIndex].morph = button.dataset.morphOption === "none" ? null : button.dataset.morphOption;
  stateInput.value = sequence[selectedStepIndex].state;
  setCodeMode("sequence");
  renderSequence();
  syncDemo();
  bot.replay();
  if (sequence[selectedStepIndex].morph) {
    bot.playMorph(sequence[selectedStepIndex].morph, {
      hold: sequence[selectedStepIndex].morphHold,
      restore: "default",
    });
  }
});

addSequenceStepButton.addEventListener("click", () => {
  stopEditorSequence();
  sequence.push({ state: "idle", hold: 1000, morph: null, morphHold: 1000 });
  selectedStepIndex = sequence.length - 1;
  stateInput.value = "idle";
  setCodeMode("sequence");
  renderSequence();
  syncDemo();
});

sequenceLoop.addEventListener("change", () => {
  stopEditorSequence();
  setCodeMode("sequence");
  syncDemo();
});

previewSequenceButton.addEventListener("click", async () => {
  stopEditorSequence();
  const run = ++sequenceRun;
  sequencePlaying = true;
  setCodeMode("sequence");
  syncSequenceSummary("准备第 1 步…");
  syncSequencePlaybackUI();
  const result = await bot.playSequence(sequence, { loop: sequenceLoop.checked });
  if (run !== sequenceRun) return;
  sequencePlaying = false;
  playingStepIndex = -1;
  syncSequencePlaybackUI();
  syncSequenceSummary(result.cancelled ? "已停止" : "播放完成 · 可再次播放");
});

stopSequenceButton.addEventListener("click", () => stopEditorSequence());

bot.addEventListener("sequencestep", (event) => {
  playingStepIndex = event.detail.index;
  stateInput.value = event.detail.state;
  syncSequenceSummary(`第 ${event.detail.index + 1} 步 · ${stateLabels[event.detail.state]}停留 ${event.detail.hold / 1000} 秒`);
  syncSequencePlaybackUI();
  syncDemo();
});

bot.addEventListener("morphstart", (event) => {
  if (!sequencePlaying) return;
  syncSequenceSummary(`第 ${playingStepIndex + 1} 步 · Morph ${effectLabels[event.detail.effect]}`);
});

document.querySelectorAll(".context-tabs [data-context]").forEach((button) => button.addEventListener("click", () => {
  previewStage.dataset.context = button.dataset.context;
  document.querySelectorAll(".context-tabs [data-context]").forEach((candidate) => candidate.classList.toggle("is-active", candidate === button));
}));

function setPreviewMode(mode) {
  previewMode = mode === "dialogue" ? "dialogue" : "sequence";
  document.querySelectorAll("[data-preview-mode]").forEach((button) => {
    const selected = button.dataset.previewMode === previewMode;
    button.classList.toggle("is-active", selected);
    button.setAttribute("aria-pressed", String(selected));
  });
  sequenceWorkspace.hidden = previewMode !== "sequence";
  dialogueWorkspace.hidden = previewMode !== "dialogue";
  previewStage.dataset.mode = previewMode;
  dialogueWorkbench?.setActive(previewMode === "dialogue");
  if (previewMode === "dialogue") {
    stopEditorSequence();
    previewStage.dataset.context = "solo";
    document.querySelectorAll(".context-tabs [data-context]").forEach((button) => button.classList.toggle("is-active", button.dataset.context === "solo"));
    setCodeMode("dialogue");
  } else {
    dialogueWorkbench?.stop();
    setCodeMode("sequence");
  }
  syncDemo();
}

document.querySelectorAll("[data-preview-mode]").forEach((button) => button.addEventListener("click", () => setPreviewMode(button.dataset.previewMode)));

document.querySelectorAll("[data-code-mode]").forEach((button) => button.addEventListener("click", () => {
  if (button.dataset.codeMode === "dialogue") setPreviewMode("dialogue");
  else { setCodeMode(button.dataset.codeMode); syncDemo(); }
}));

pauseButton.addEventListener("click", () => {
  bot.paused = !bot.paused;
  pauseButton.textContent = bot.paused ? "继续动画" : "暂停动画";
  pauseButton.classList.toggle("is-paused", bot.paused);
});

document.querySelector("#copy-component-code").addEventListener("click", async (event) => {
  try {
    await navigator.clipboard.writeText(code.textContent);
    event.currentTarget.textContent = "已复制，可以粘贴了 ✓";
    event.currentTarget.classList.add("is-copied");
    window.setTimeout(() => {
      event.currentTarget.textContent = "复制这段代码";
      event.currentTarget.classList.remove("is-copied");
    }, 1400);
  } catch {
    const selection = window.getSelection();
    const range = document.createRange();
    range.selectNodeContents(code);
    selection.removeAllRanges();
    selection.addRange(range);
    event.currentTarget.textContent = "代码已选中，请按 ⌘C";
  }
});

let lastRuntimeUpdate = 0;
function updateRuntime(now) {
  if (now - lastRuntimeUpdate > 100) {
    const snapshot = bot.snapshot();
    if (snapshot) runtime.textContent = `E${String(snapshot.expressionIndex).padStart(2, "0")} · ${snapshot.morphPhase} · ${snapshot.morphAmount.toFixed(2)}`;
    lastRuntimeUpdate = now;
  }
  requestAnimationFrame(updateRuntime);
}

dialogueWorkbench = setupDialogueWorkbench({
  bot,
  onChange: () => {
    if (previewMode !== "dialogue") return;
    setCodeMode("dialogue");
    code.textContent = dialogueCode();
  },
});
renderMaterialEditor();
renderSequence();
syncDemo();
requestAnimationFrame(updateRuntime);
