import {
  compileDialogue,
  BOT_FACE_DIALOGUE_VOICES,
  BOT_FACE_EFFECTS,
} from "../bot-face.js";
import {
  MORPH_LABELS_ZH as effectLabels,
  STATE_IDS as orderedStates,
  STATE_LABELS_ZH as stateLabels,
} from "../catalog.js";
import { planDialogueActions } from "../dialogue-auto-director.js";

const rotations = Object.freeze([
  { value: -24, label: "向左转 24°" },
  { value: -12, label: "向左转 12°" },
  { value: 0, label: "回到正面" },
  { value: 12, label: "向右转 12°" },
  { value: 24, label: "向右转 24°" },
]);

const actionCatalog = Object.freeze([
  ...orderedStates.map((state) => ({ type: "state", value: state, label: stateLabels[state], code: state, duration: 320, icon: "●", group: "全部表情 · 39" })),
  ...rotations.map(({ value, label }) => ({ type: "rotate", value: String(value), label, code: `${value}deg`, duration: 260, icon: value < 0 ? "↙" : value > 0 ? "↘" : "↑", group: "旋转 · 5" })),
  ...BOT_FACE_EFFECTS.map((effect) => ({ type: "morph", value: effect, label: effectLabels[effect], code: effect, duration: 700, icon: "◇", group: "Morph · 14" })),
  ...[250, 500, 1000].map((duration) => ({ type: "pause", value: "pause", label: `停顿 ${duration / 1000} 秒`, code: `${duration}ms`, duration, icon: "Ⅱ", group: "停顿 · 3" })),
]);

const typeLabels = Object.freeze({ state: "表情", rotate: "旋转", morph: "Morph", pause: "停顿" });
let actionCounter = 0;

function formatTime(milliseconds) {
  const safe = Math.max(0, Number(milliseconds) || 0);
  const minutes = Math.floor(safe / 60000);
  const seconds = (safe / 1000 % 60).toFixed(1).padStart(4, "0");
  return `${minutes}:${seconds}`;
}

function actionLabel(type, value) {
  if (type === "state") return stateLabels[value] || value;
  if (type === "rotate") return rotations.find((item) => String(item.value) === String(value))?.label || `旋转 ${value}°`;
  if (type === "morph") return effectLabels[value] || value;
  return "停顿";
}

function tokenToNode(token) {
  const type = token.dataset.actionType;
  const duration = Number(token.dataset.duration) || 0;
  const id = token.dataset.actionId;
  if (type === "state") return { id, type, state: token.dataset.value, duration };
  if (type === "rotate") return { id, type, angle: Number(token.dataset.value), duration };
  if (type === "morph") return { id, type, effect: token.dataset.value, duration };
  return { id, type: "pause", duration };
}

function appendText(nodes, text) {
  const normalized = text.replaceAll("\u00a0", " ");
  if (!normalized) return;
  const previous = nodes.at(-1);
  if (previous?.type === "text") previous.text += normalized;
  else nodes.push({ type: "text", text: normalized });
}

export function setupDialogueWorkbench({ bot, onChange = () => {} } = {}) {
  const editor = document.querySelector("#dialogue-editor");
  const actionMenu = document.querySelector("#dialogue-action-menu");
  const actionList = document.querySelector("#dialogue-action-list");
  const addActionButton = document.querySelector("#dialogue-add-action");
  const autoActionButton = document.querySelector("#dialogue-auto-action");
  const undoAutoButton = document.querySelector("#dialogue-undo-auto");
  const autoStatus = document.querySelector("#dialogue-auto-status");
  const inspector = document.querySelector("#dialogue-action-inspector");
  const inspectorType = document.querySelector("#dialogue-inspector-type");
  const inspectorValue = document.querySelector("#dialogue-inspector-value");
  const inspectorDuration = document.querySelector("#dialogue-inspector-duration");
  const deleteActionButton = document.querySelector("#dialogue-delete-action");
  const voiceSelect = document.querySelector("#dialogue-voice");
  const voiceHint = document.querySelector("#dialogue-voice-hint");
  const englishModeSelect = document.querySelector("#dialogue-english-mode");
  const rateSelect = document.querySelector("#dialogue-rate");
  const previewButton = document.querySelector("#preview-dialogue");
  const stopButton = document.querySelector("#stop-dialogue");
  const timeOutput = document.querySelector("#dialogue-time");
  const score = document.querySelector(".dialogue-score");
  const scoreMarkers = document.querySelector("#dialogue-score-markers");
  const bubble = document.querySelector("#dialogue-bubble");
  const caption = document.querySelector("#dialogue-caption");

  let mention = null;
  let menuIndex = 0;
  let selectedToken = null;
  let playing = false;
  let paused = false;
  let active = false;
  let totalDuration = 0;
  let automationSnapshot = null;
  let automationGeneration = 0;

  for (const voice of BOT_FACE_DIALOGUE_VOICES) voiceSelect.add(new Option(`${voice.label.zh} · ${voice.label.en}`, voice.id));

  function syncVoiceHint() {
    const voice = BOT_FACE_DIALOGUE_VOICES.find(({ id }) => id === voiceSelect.value);
    const sampled = ["playful", "clean"].includes(voiceSelect.value);
    englishModeSelect.disabled = !sampled;
    const englishHint = englishModeSelect.value === "phonetic" ? "英文自动组合 th / sh / ng 等音素" : "英文逐字母发声";
    voiceHint.textContent = voice ? `${voice.description.zh} · ${voice.description.en}${sampled ? ` · ${englishHint}` : ""}` : "";
  }
  syncVoiceHint();

  function createToken(action) {
    const token = document.createElement("span");
    const value = action.value ?? action.state ?? action.angle ?? action.effect ?? "pause";
    token.className = "dialogue-token";
    token.contentEditable = "false";
    token.tabIndex = 0;
    token.dataset.actionId = action.id || `dialogue-action-${++actionCounter}`;
    token.dataset.actionType = action.type;
    token.dataset.value = String(value);
    token.dataset.duration = String(action.duration);
    if (action.generated) token.dataset.generated = "true";
    updateToken(token);
    return token;
  }

  function updateToken(token) {
    const type = token.dataset.actionType;
    const label = actionLabel(type, token.dataset.value);
    const duration = Number(token.dataset.duration) || 0;
    token.innerHTML = `<b>${label}</b><small>${duration}ms</small>`;
    token.setAttribute("aria-label", `${typeLabels[type]} ${label}，${duration} 毫秒`);
  }

  function editorScript() {
    const nodes = [];
    const visit = (node) => {
      if (node.nodeType === Node.TEXT_NODE) { appendText(nodes, node.data); return; }
      if (!(node instanceof HTMLElement)) return;
      if (node.matches(".dialogue-token")) { nodes.push(tokenToNode(node)); return; }
      if (node.tagName === "BR") { appendText(nodes, "\n"); return; }
      const block = ["DIV", "P"].includes(node.tagName);
      for (const child of node.childNodes) visit(child);
      if (block && node !== editor && nodes.at(-1)?.text && !nodes.at(-1).text.endsWith("\n")) appendText(nodes, "\n");
    };
    for (const child of editor.childNodes) visit(child);
    if (nodes.at(-1)?.type === "text") nodes.at(-1).text = nodes.at(-1).text.replace(/\n+$/u, "");
    return nodes.filter((node) => node.type !== "text" || node.text.length);
  }

  function cloneScript(script) {
    return script.map((node) => ({ ...node }));
  }

  function renderEditorScript(script, { generated = false } = {}) {
    const fragment = document.createDocumentFragment();
    for (const node of script) {
      if (node.type === "text") fragment.append(document.createTextNode(node.text));
      else fragment.append(createToken({ ...node, generated }));
    }
    editor.replaceChildren(fragment);
  }

  function setAutoStatus(message, tone = "") {
    autoStatus.textContent = message;
    autoStatus.classList.toggle("is-ready", tone === "ready");
    autoStatus.classList.toggle("is-error", tone === "error");
  }

  function clearAutomationHistory(message = "文字或动作已手动调整，可以重新自动编排。") {
    if (!automationSnapshot) return;
    automationSnapshot = null;
    automationGeneration = 0;
    undoAutoButton.hidden = true;
    autoActionButton.querySelector("span").textContent = "自动编排";
    setAutoStatus(message);
  }

  function autoArrangeDialogue() {
    const currentScript = editorScript();
    const text = currentScript.filter(({ type }) => type === "text").map(({ text: value }) => value).join("");
    if (!text.trim()) {
      setAutoStatus("先输入一句话，再让 Bot 自动编排。", "error");
      editor.focus();
      return;
    }
    if (!automationSnapshot) automationSnapshot = cloneScript(currentScript);
    automationGeneration += 1;
    const plan = planDialogueActions(text, {
      seed: `${Date.now()}:${automationGeneration}:${Math.random()}`,
    });
    closeActionMenu();
    selectToken(null);
    renderEditorScript(plan.script, { generated: true });
    autoActionButton.querySelector("span").textContent = "换一版";
    undoAutoButton.hidden = false;
    setAutoStatus(`已读出 ${plan.clauseCount} 个语气片段，插入 ${plan.actionCount} 个动作；再点一次可换一版。`, "ready");
    notifyChange();
  }

  function undoAutoArrange() {
    if (!automationSnapshot) return;
    const snapshot = automationSnapshot;
    automationSnapshot = null;
    automationGeneration = 0;
    renderEditorScript(snapshot);
    undoAutoButton.hidden = true;
    autoActionButton.querySelector("span").textContent = "自动编排";
    setAutoStatus("已恢复自动编排前的文字和动作。", "ready");
    selectToken(null);
    notifyChange();
    editor.focus();
  }

  function nodeDuration(node, rate, voice) {
    if (node.type !== "text") return node.duration + (node.type === "morph" ? 420 : 0);
    if (!node.text) return 0;
    if (!node.text.trim()) {
      const base = voice === "playful" ? 58 : voice === "clean" ? 64 : 72;
      return Array.from(node.text).length * base / rate;
    }
    return compileDialogue([node], { rate, voice }).duration;
  }

  function renderScore() {
    const script = editorScript();
    const rate = Number(rateSelect.value) || 1;
    const voice = voiceSelect.value;
    try { totalDuration = compileDialogue(script, { rate, voice, englishMode: englishModeSelect.value }).duration; } catch { totalDuration = 0; }
    scoreMarkers.replaceChildren();
    let elapsed = 0;
    for (const node of script) {
      if (node.type !== "text" && totalDuration) {
        const marker = document.createElement("i");
        marker.dataset.type = node.type;
        marker.style.left = `${Math.min(100, elapsed / totalDuration * 100)}%`;
        marker.title = `${typeLabels[node.type]} · ${actionLabel(node.type, node.state ?? node.angle ?? node.effect ?? "pause")}`;
        scoreMarkers.append(marker);
      }
      elapsed += nodeDuration(node, rate, voice);
    }
    timeOutput.textContent = `${formatTime(0)} / ${formatTime(totalDuration)}`;
    score.style.setProperty("--dialogue-progress", "0");
  }

  function notifyChange() {
    if (playing) stopPlayback();
    if (selectedToken && !selectedToken.isConnected) selectToken(null);
    renderScore();
    onChange(editorScript());
  }

  function currentMention() {
    const selection = window.getSelection();
    if (!selection?.rangeCount || !editor.contains(selection.anchorNode)) return null;
    const node = selection.anchorNode;
    if (node?.nodeType !== Node.TEXT_NODE) return null;
    const end = selection.anchorOffset;
    const match = node.data.slice(0, end).match(/@([^\s@]*)$/u);
    if (!match) return null;
    return { node, start: end - match[0].length, end, query: match[1].toLowerCase() };
  }

  function filteredActions(query = "") {
    if (!query) return actionCatalog;
    return actionCatalog.filter((action) => `${action.label} ${action.code} ${typeLabels[action.type]}`.toLowerCase().includes(query));
  }

  function renderActionMenu(query = "") {
    const actions = filteredActions(query);
    menuIndex = Math.max(0, Math.min(menuIndex, Math.max(0, actions.length - 1)));
    actionList.replaceChildren();
    if (!actions.length) {
      const empty = document.createElement("p");
      empty.className = "dialogue-action-empty";
      empty.textContent = "没有匹配动作，继续输入或按 Esc 关闭。";
      actionList.append(empty);
      return;
    }
    let previousGroup = "";
    actions.forEach((action, index) => {
      if (action.group !== previousGroup) {
        const group = document.createElement("div");
        group.className = "dialogue-action-group";
        group.textContent = action.group;
        actionList.append(group);
        previousGroup = action.group;
      }
      const button = document.createElement("button");
      button.type = "button";
      button.dataset.actionIndex = String(actionCatalog.indexOf(action));
      button.dataset.filteredIndex = String(index);
      button.dataset.actionType = action.type;
      button.classList.toggle("is-active", index === menuIndex);
      button.setAttribute("role", "option");
      button.setAttribute("aria-selected", String(index === menuIndex));
      button.innerHTML = `<i>${action.icon}</i><span><strong>${action.label}</strong><code>${action.code}</code></span><small>${action.duration}ms</small>`;
      actionList.append(button);
    });
    actionList.querySelector("button.is-active")?.scrollIntoView({ block: "nearest" });
  }

  function openActionMenu() {
    mention = currentMention();
    if (!mention) { closeActionMenu(); return; }
    menuIndex = 0;
    actionMenu.hidden = false;
    renderActionMenu(mention.query);
  }

  function closeActionMenu() {
    actionMenu.hidden = true;
    mention = null;
  }

  function placeCaretAfter(node) {
    const range = document.createRange();
    const selection = window.getSelection();
    range.setStartAfter(node);
    range.collapse(true);
    selection.removeAllRanges();
    selection.addRange(range);
  }

  function insertAction(action) {
    if (!mention?.node?.isConnected) return;
    mention.node.deleteData(mention.start, mention.end - mention.start);
    const token = createToken(action);
    const space = document.createTextNode("\u00a0");
    const reference = mention.node.splitText(mention.start);
    reference.parentNode.insertBefore(token, reference);
    reference.parentNode.insertBefore(space, reference);
    placeCaretAfter(space);
    closeActionMenu();
    selectToken(token);
    notifyChange();
    editor.focus();
  }

  function insertAtCaret(text) {
    editor.focus();
    const selection = window.getSelection();
    const range = selection.rangeCount && editor.contains(selection.anchorNode) ? selection.getRangeAt(0) : document.createRange();
    if (!(selection.rangeCount && editor.contains(selection.anchorNode))) {
      range.selectNodeContents(editor);
      range.collapse(false);
    }
    range.deleteContents();
    const textNode = document.createTextNode(text);
    range.insertNode(textNode);
    const caret = document.createRange();
    caret.setStart(textNode, text.length);
    caret.collapse(true);
    selection.removeAllRanges();
    selection.addRange(caret);
    openActionMenu();
  }

  function selectToken(token) {
    selectedToken?.classList.remove("is-selected");
    selectedToken = token;
    if (!selectedToken) { inspector.hidden = true; return; }
    selectedToken.classList.add("is-selected");
    inspector.hidden = false;
    const type = selectedToken.dataset.actionType;
    inspectorType.textContent = typeLabels[type];
    inspectorValue.replaceChildren();
    const options = type === "state"
      ? orderedStates.map((value) => ({ value, label: `${stateLabels[value]} · ${value}` }))
      : type === "rotate"
        ? rotations.map(({ value, label }) => ({ value: String(value), label }))
        : type === "morph"
          ? BOT_FACE_EFFECTS.map((value) => ({ value, label: `${effectLabels[value]} · ${value}` }))
          : [{ value: "pause", label: "停顿" }];
    for (const option of options) inspectorValue.add(new Option(option.label, option.value));
    inspectorValue.value = selectedToken.dataset.value;
    inspectorDuration.value = String((Number(selectedToken.dataset.duration) || 0) / 1000);
  }

  function clearPlayingToken() {
    editor.querySelectorAll(".dialogue-token.is-playing").forEach((token) => token.classList.remove("is-playing"));
  }

  function syncPlaybackButtons() {
    stopButton.disabled = !playing;
    previewButton.classList.toggle("is-paused", paused);
    previewButton.innerHTML = !playing
      ? "<span>▶</span> 播放对话"
      : paused ? "<span>▶</span> 继续对话" : "<span>Ⅱ</span> 暂停对话";
  }

  function stopPlayback() {
    bot.stopDialogue();
    playing = false;
    paused = false;
    clearPlayingToken();
    score.style.setProperty("--dialogue-progress", "0");
    timeOutput.textContent = `${formatTime(0)} / ${formatTime(totalDuration)}`;
    caption.textContent = "准备好后点击播放";
    syncPlaybackButtons();
  }

  async function startPlayback() {
    const script = editorScript();
    try { compileDialogue(script, { rate: Number(rateSelect.value), voice: voiceSelect.value, englishMode: englishModeSelect.value }); }
    catch {
      editor.focus();
      return;
    }
    playing = true;
    paused = false;
    clearPlayingToken();
    caption.textContent = "…";
    syncPlaybackButtons();
    const result = await bot.performDialogue(script, {
      voice: voiceSelect.value,
      rate: Number(rateSelect.value),
      englishMode: englishModeSelect.value,
    });
    if (!playing) return;
    playing = false;
    paused = false;
    syncPlaybackButtons();
    if (!result.cancelled) caption.textContent = result.text || "播放完成";
  }

  function setActive(nextActive) {
    active = Boolean(nextActive);
    bubble.hidden = !active;
    if (!active && playing) stopPlayback();
  }

  editor.addEventListener("input", () => {
    clearAutomationHistory();
    openActionMenu();
    notifyChange();
  });

  editor.addEventListener("keydown", (event) => {
    if (!actionMenu.hidden) {
      const actions = filteredActions(mention?.query || "");
      if ((event.key === "ArrowDown" || event.key === "ArrowUp") && actions.length) {
        event.preventDefault();
        const offset = event.key === "ArrowDown" ? 1 : -1;
        menuIndex = (menuIndex + offset + actions.length) % actions.length;
        renderActionMenu(mention?.query || "");
      } else if (event.key === "Enter" && actions.length) {
        event.preventDefault();
        insertAction(actions[menuIndex]);
      } else if (event.key === "Escape") {
        event.preventDefault();
        closeActionMenu();
      }
    }
  });

  editor.addEventListener("paste", (event) => {
    event.preventDefault();
    clearAutomationHistory();
    insertAtCaret(event.clipboardData.getData("text/plain"));
    closeActionMenu();
    notifyChange();
  });

  editor.addEventListener("click", (event) => {
    const token = event.target.closest(".dialogue-token");
    if (token) selectToken(token);
  });

  editor.addEventListener("keydown", (event) => {
    const token = event.target.closest?.(".dialogue-token");
    if (token && ["Backspace", "Delete"].includes(event.key)) {
      event.preventDefault();
      clearAutomationHistory();
      token.remove();
      selectToken(null);
      notifyChange();
    }
  });

  actionList.addEventListener("mousedown", (event) => event.preventDefault());
  actionList.addEventListener("click", (event) => {
    const button = event.target.closest("[data-action-index]");
    if (button) insertAction(actionCatalog[Number(button.dataset.actionIndex)]);
  });
  addActionButton.addEventListener("click", () => {
    clearAutomationHistory();
    insertAtCaret("@");
  });
  autoActionButton.addEventListener("click", autoArrangeDialogue);
  undoAutoButton.addEventListener("click", undoAutoArrange);

  inspectorValue.addEventListener("change", () => {
    if (!selectedToken) return;
    clearAutomationHistory();
    selectedToken.dataset.value = inspectorValue.value;
    updateToken(selectedToken);
    notifyChange();
  });
  inspectorDuration.addEventListener("input", () => {
    if (!selectedToken) return;
    clearAutomationHistory();
    const milliseconds = Math.min(20000, Math.max(0, Math.round((Number(inspectorDuration.value) || 0) * 1000)));
    selectedToken.dataset.duration = String(milliseconds);
    updateToken(selectedToken);
    notifyChange();
  });
  deleteActionButton.addEventListener("click", () => {
    if (!selectedToken) return;
    clearAutomationHistory();
    selectedToken.remove();
    selectToken(null);
    notifyChange();
    editor.focus();
  });

  rateSelect.addEventListener("change", () => { renderScore(); onChange(editorScript()); });
  englishModeSelect.addEventListener("change", () => { syncVoiceHint(); renderScore(); onChange(editorScript()); });
  voiceSelect.addEventListener("change", () => { syncVoiceHint(); renderScore(); onChange(editorScript()); });
  previewButton.addEventListener("click", () => {
    if (!playing) startPlayback();
    else if (paused) { paused = false; bot.resumeDialogue(); syncPlaybackButtons(); }
    else { paused = true; bot.pauseDialogue(); syncPlaybackButtons(); }
  });
  stopButton.addEventListener("click", stopPlayback);

  bot.addEventListener("dialogueprogress", (event) => {
    if (!active) return;
    const { elapsed, duration, progress, text } = event.detail;
    score.style.setProperty("--dialogue-progress", String(Math.min(1, Math.max(0, progress))));
    timeOutput.textContent = `${formatTime(elapsed)} / ${formatTime(duration)}`;
    if (text) caption.textContent = text;
  });
  bot.addEventListener("dialoguecharacter", (event) => {
    if (active) caption.textContent = event.detail.text || "…";
  });
  bot.addEventListener("dialogueaction", (event) => {
    if (!active) return;
    clearPlayingToken();
    editor.querySelector(`[data-action-id="${CSS.escape(event.detail.node.id)}"]`)?.classList.add("is-playing");
  });
  bot.addEventListener("dialogueend", () => clearPlayingToken());

  document.addEventListener("mousedown", (event) => {
    if (!actionMenu.hidden && !actionMenu.contains(event.target) && !editor.contains(event.target) && event.target !== addActionButton) closeActionMenu();
    if (selectedToken && !inspector.contains(event.target) && !selectedToken.contains(event.target)) selectToken(null);
  });

  editor.append(
    createToken({ type: "state", value: "idle", duration: 300 }),
    document.createTextNode(" 你好，我是 Bot Face。"),
    createToken({ type: "state", value: "thinking", duration: 450 }),
    document.createTextNode(" 让我想一下……"),
    createToken({ type: "rotate", value: "-12", duration: 260 }),
    document.createTextNode(" 有了！"),
    createToken({ type: "state", value: "happy", duration: 320 }),
    createToken({ type: "morph", value: "wave", duration: 650 }),
    document.createTextNode(" 这个回答可以直接演出来。"),
  );
  renderScore();
  syncPlaybackButtons();

  return {
    get script() { return editorScript(); },
    get voice() { return voiceSelect.value; },
    get englishMode() { return englishModeSelect.value; },
    get rate() { return Number(rateSelect.value) || 1; },
    setActive,
    stop: stopPlayback,
  };
}
