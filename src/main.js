import { bootstrap, invoke, listen, isDesktop } from "./api.js";

const $ = (id) => document.getElementById(id);

const CIRC = 2 * Math.PI * 94;

const ACCENTS = {
  pink: ["#FFA3B8", "#C6A6F8", "#9FD3FF"],
  mint: ["#9FE7CE", "#9FD3FF", "#C6A6F8"],
  lavender: ["#C6A6F8", "#FFA3B8", "#9FD3FF"],
  sky: ["#9FD3FF", "#B7E3D0", "#FFD98E"],
  butter: ["#FFD98E", "#FFB3C7", "#C6A6F8"],
};

const COLORS = ["#FFB3C7", "#FFD98E", "#9FE7CE", "#9FD3FF", "#C6A6F8"];

const state = {
  timer: null,
  settings: null,
  presets: [],
  sounds: [],
  version: "0.1.0",
  mode: "normal",
  newPresetColor: COLORS[0],
  previewTimer: null,
  toastTimer: null,
};

/* ---------------------------- 启动 ---------------------------- */
init().catch((err) => {
  console.error(err);
  toast("初始化失败：" + err);
});

async function init() {
  const boot = await bootstrap();
  state.timer = boot.timer;
  state.settings = boot.settings;
  state.presets = boot.presets || [];
  state.sounds = boot.sounds || [];
  state.version = boot.version || "0.1.0";

  bindStatic();
  applyAccent(state.settings.accent || "pink");
  $("versionText").textContent = "v" + state.version;
  renderSettings();
  renderSounds();
  renderPresets();
  render(state.timer);

  listen("timer://update", (s) => {
    state.timer = s;
    render(s);
  });
  listen("timer://state", (s) => {
    state.timer = s;
    render(s);
  });
  listen("timer://finished", (p) => {
    flashFinished();
    if (p?.label) toast(p.label + " 结束");
  });
  listen("toast", (p) => toast(p.text));

  listen("sound://imported", async (meta) => {
    if (!meta) return;
    state.sounds.push(meta);
    await patchSettings({ sound_id: meta.id });
    renderSounds();
    toast("已导入：" + meta.name);
  });

  listen("settings://changed", async () => {
    const fresh = await bootstrap();
    state.settings = fresh.settings;
    applyAccent(fresh.settings.accent);
    renderSettings();
  });

  const params = new URLSearchParams(location.search);
  if (state.settings.mode === "mini" && !params.has("mock")) setMode("mini");
  if (params.get("view") === "settings") openSettings();
  if (params.get("view") === "preset") openPresetSheet();
  if (previewVariantIsMini()) setMode("mini");

  // 首次最小化到托盘时给一点提示（迷你模式 / 设置页不打扰）
  setTimeout(() => {
    if (location.search.length > 1) return;
    if (state.mode !== "normal" || !$("settingsDrawer").hidden) return;
    if (!localStorage.getItem("hint-tray")) {
      localStorage.setItem("hint-tray", "1");
      toast("关闭窗口会收进托盘，计时继续运行");
    }
  }, 2400);
}

/* ---------------------------- 渲染 ---------------------------- */
function fmt(ms, cs = false) {
  ms = Math.max(0, Math.round(ms));
  const total = Math.floor(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const pad = (n) => String(n).padStart(2, "0");
  let out = h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
  if (cs && h === 0) out += "." + pad(Math.floor((ms % 1000) / 10));
  return out;
}

function phaseLabel(t) {
  if (t.kind === "pomodoro") {
    return t.phase === "work" ? "专注" : t.phase === "long" ? "长休息" : "小憩";
  }
  return "";
}

function render(t) {
  if (!t) return;
  const stopwatch = t.kind === "stopwatch";
  const elapsed = t.elapsed_ms || 0;
  const remaining = stopwatch ? 0 : Math.max(0, t.remaining_ms ?? (t.target_ms - elapsed));
  const text = stopwatch ? fmt(elapsed, true) : fmt(remaining);

  const digits = $("digits");
  if (digits.textContent !== text) digits.textContent = text;
  digits.classList.toggle("cs", stopwatch && elapsed < 3600000);
  digits.classList.toggle("long", !stopwatch && text.length > 5);

  let sub = "";
  if (stopwatch) {
    sub = elapsed > 0 ? `总计 ${fmt(elapsed)}` : "按下开始，记录每一圈";
  } else if (t.kind === "pomodoro") {
    const round = Math.max(1, t.round || 1);
    sub = `${phaseLabel(t)} · 第 ${round}/${t.rounds || state.settings.rounds} 轮`;
  } else if (t.finished) {
    sub = `${t.label || "倒计时"} · 时间到`;
  } else {
    sub = t.label && t.label !== "倒计时" ? `${t.label} · ${fmt(t.target_ms)}` : `倒计时 · ${fmt(t.target_ms)}`;
  }
  const subEl = $("subLabel");
  if (subEl.textContent !== sub) subEl.textContent = sub;

  // 环形进度
  let frac = 0;
  if (stopwatch) frac = (elapsed % 60000) / 60000;
  else if (t.finished) frac = 1;
  else if (t.target_ms > 0) frac = Math.max(0, Math.min(1, remaining / t.target_ms));
  $("ringProgress").style.strokeDashoffset = String(CIRC * (1 - frac));

  // 番茄轮次点
  const dots = $("phaseDots");
  if (t.kind === "pomodoro") {
    const n = t.rounds || state.settings.rounds || 4;
    if (dots.childElementCount !== n) {
      dots.innerHTML = Array.from({ length: n }, () => "<i></i>").join("");
    }
    [...dots.children].forEach((el, i) => el.classList.toggle("on", i < (t.round || 1)));
    dots.style.display = "";
  } else {
    dots.style.display = "none";
  }

  // 按钮
  const primary = $("btnPrimary");
  primary.textContent = t.running ? "暂停" : t.finished ? "再来一次" : "开始";
  $("btnReset").disabled = !t.running && elapsed === 0;
  const secondary = $("btnSecondary");
  secondary.textContent = stopwatch ? "计次" : t.kind === "pomodoro" ? "跳过" : "+1 分";
  secondary.disabled = stopwatch && !t.running;

  // 计次列表
  const laps = $("laps");
  if (stopwatch && t.laps?.length) {
    laps.innerHTML = t.laps
      .slice(0, 6)
      .map(
        (ms, i) =>
          `<div class="lap-row ${i === 0 ? "fresh" : ""}"><span>第 ${t.laps.length - i} 圈</span><span>${fmt(ms, true)}</span></div>`
      )
      .join("");
    laps.style.display = "";
  } else {
    laps.innerHTML = "";
    laps.style.display = "none";
  }

  setKindUI(t.kind);
  renderMini(t, text, sub);
}

function setKindUI(kind) {
  const order = ["stopwatch", "countdown", "pomodoro"];
  const idx = Math.max(0, order.indexOf(kind));
  $("tabPill").style.transform = `translateX(${idx * 100}%)`;
  [...$("tabs").querySelectorAll("button")].forEach((b, i) => b.classList.toggle("active", i === idx));
  $("presetsSection").style.display = kind === "stopwatch" ? "none" : "";
}

let lastFinishedAt = 0;
function flashFinished() {
  lastFinishedAt = Date.now();
  const card = $("card");
  card.animate(
    [
      { boxShadow: "0 0 0 0 rgba(255,163,184,0.85), 0 22px 50px rgba(120,100,160,0.2)" },
      { boxShadow: "0 0 0 18px rgba(255,163,184,0), 0 22px 50px rgba(120,100,160,0.2)" },
    ],
    { duration: 1200, iterations: 2 }
  );
}

function renderMini(t, text, sub) {
  $("miniDigits").textContent = text;
  $("miniSub").textContent = sub;
  const icon = $("miniToggleIcon");
  icon.innerHTML = t.running
    ? '<path d="M9 5v14M15 5v14"/>'
    : '<path d="M8 5l11 7-11 7V5z"/>';
}

/* ---------------------------- 预设 ---------------------------- */
function renderPresets() {
  const list = $("presetList");
  if (!state.presets.length) {
    list.innerHTML = '<div class="preset-empty">还没有预设计时器，点「新建」添加一个</div>';
    return;
  }
  const activeId = state.timer?.preset_id;
  list.innerHTML = state.presets
    .map(
      (p) => `
      <div class="preset-chip ${activeId === p.id ? "on" : ""}" data-id="${p.id}">
        <span class="preset-dot" style="background:${p.color}"></span>
        <span>${escapeHtml(p.name)}</span>
        <span class="mono" style="font-size:9.5px;color:#a9a3ba">${fmt(p.ms)}</span>
        <button class="del" data-del="${p.id}" title="删除">×</button>
      </div>`
    )
    .join("");
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
}

/* ---------------------------- 设置 ---------------------------- */
function renderSettings() {
  const s = state.settings;
  $("setVolume").value = Math.round((s.volume ?? 0.7) * 100);
  $("volLabel").textContent = Math.round((s.volume ?? 0.7) * 100) + "%";
  $("setFade").checked = !!s.fade;
  $("setPin").checked = !!s.always_on_top;
  $("setAutostart").checked = !!s.autostart;
  $("setWork").value = s.work_min;
  $("setBreak").value = s.break_min;
  $("setLongBreak").value = s.long_break_min;
  $("setRounds").value = s.rounds;
  $("setAuto").checked = !!s.auto_start;
  $("btnPin").classList.toggle("active", !!s.always_on_top);
  renderAccents();
  renderSounds();
}

function renderAccents() {
  const row = $("accentRow");
  row.innerHTML = Object.keys(ACCENTS)
    .map(
      (k) =>
        `<button class="swatch ${state.settings.accent === k ? "on" : ""}" data-accent="${k}" style="background:linear-gradient(135deg,${ACCENTS[k][0]},${ACCENTS[k][1]})"></button>`
    )
    .join("");
}

function renderSounds() {
  const list = $("soundList");
  if (!state.sounds.length) {
    list.innerHTML = '<div class="hint" style="padding:4px 2px">还没有铃声，导入一个试试</div>';
    return;
  }
  list.innerHTML = state.sounds
    .map(
      (s) => `
      <div class="sound-row ${state.settings.sound_id === s.id ? "on" : ""}" data-sound="${s.id}">
        <button class="mini-icon" data-play="${s.id}" title="试听">
          <svg viewBox="0 0 24 24"><path d="M8 5l11 7-11 7V5z"/></svg>
        </button>
        <span class="name">${escapeHtml(s.name)}</span>
        <span class="tag">${s.builtin ? "内置" : "自定义"}</span>
        ${s.builtin ? "" : `<button class="mini-icon" data-remove="${s.id}" title="删除"><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18"/></svg></button>`}
      </div>`
    )
    .join("");
}

function applyAccent(name) {
  const a = ACCENTS[name] || ACCENTS.pink;
  document.documentElement.style.setProperty("--accent-1", a[0]);
  document.documentElement.style.setProperty("--accent-2", a[1]);
  const stops = document.querySelectorAll("#ringGrad stop");
  if (stops[0]) stops[0].setAttribute("stop-color", a[0]);
  if (stops[1]) stops[1].setAttribute("stop-color", a[1]);
  if (stops[2]) stops[2].setAttribute("stop-color", a[2] || a[1]);
}

/* ---------------------------- 事件绑定 ---------------------------- */
function bindStatic() {
  $("btnPrimary").addEventListener("click", togglePrimary);
  $("btnReset").addEventListener("click", () => invoke("reset"));
  $("btnSecondary").addEventListener("click", secondaryAction);
  $("btnClose").addEventListener("click", () => invoke("hide_window"));
  $("btnMini").addEventListener("click", () => setMode("mini"));
  $("btnSettings").addEventListener("click", openSettings);
  $("btnBack").addEventListener("click", closeDrawer);
  $("btnPin").addEventListener("click", () => patchSettings({ always_on_top: !state.settings.always_on_top }));

  $("btnAddPreset").addEventListener("click", openPresetSheet);
  $("btnSheetClose").addEventListener("click", closePresetSheet);
  $("btnSavePreset").addEventListener("click", savePreset);

  $("presetList").addEventListener("click", (e) => {
    const del = e.target.closest("[data-del]");
    if (del) {
      e.stopPropagation();
      removePreset(del.dataset.del);
      return;
    }
    const chip = e.target.closest(".preset-chip");
    if (chip) invoke("start_preset", { id: chip.dataset.id });
  });

  $("tabs").addEventListener("click", (e) => {
    const b = e.target.closest("button[data-kind]");
    if (b && b.dataset.kind !== state.timer?.kind) invoke("switch_kind", { kind: b.dataset.kind });
  });

  $("miniToggle").addEventListener("click", togglePrimary);
  $("miniExpand").addEventListener("click", () => setMode("normal"));

  $("setVolume").addEventListener("input", (e) => {
    $("volLabel").textContent = e.target.value + "%";
  });
  $("setVolume").addEventListener("change", (e) =>
    patchSettings({ volume: Number(e.target.value) / 100 })
  );
  $("setFade").addEventListener("change", (e) => patchSettings({ fade: e.target.checked }));
  $("setPin").addEventListener("change", (e) => patchSettings({ always_on_top: e.target.checked }));
  $("setAutostart").addEventListener("change", (e) => patchSettings({ autostart: e.target.checked }));
  $("setAuto").addEventListener("change", (e) => patchSettings({ auto_start: e.target.checked }));
  ["setWork", "setBreak", "setLongBreak", "setRounds"].forEach((id) => {
    $(id).addEventListener("change", (e) => {
      const key = { setWork: "work_min", setBreak: "break_min", setLongBreak: "long_break_min", setRounds: "rounds" }[id];
      patchSettings({ [key]: Math.max(1, Number(e.target.value) || 1) });
    });
  });

  $("accentRow").addEventListener("click", (e) => {
    const sw = e.target.closest("[data-accent]");
    if (!sw) return;
    applyAccent(sw.dataset.accent);
    patchSettings({ accent: sw.dataset.accent });
  });

  $("btnImportSound").addEventListener("click", importSound);
  $("soundList").addEventListener("click", (e) => {
    const play = e.target.closest("[data-play]");
    if (play) {
      e.stopPropagation();
      previewSound(play.dataset.play);
      return;
    }
    const rm = e.target.closest("[data-remove]");
    if (rm) {
      e.stopPropagation();
      removeSound(rm.dataset.remove);
      return;
    }
    const row = e.target.closest("[data-sound]");
    if (row) {
      patchSettings({ sound_id: row.dataset.sound });
      renderSounds();
    }
  });

  document.addEventListener("keydown", (e) => {
    const tag = document.activeElement?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA") return;
    if (e.code === "Escape") {
      if (!$("presetSheet").hidden) return closePresetSheet();
      if (!$("settingsDrawer").hidden) return closeDrawer();
      if (state.mode === "mini") return setMode("normal");
      return;
    }
    if (e.code === "Space") {
      e.preventDefault();
      togglePrimary();
    } else if (e.key.toLowerCase() === "r") {
      invoke("reset");
    } else if (e.key.toLowerCase() === "l" && state.timer?.kind === "stopwatch") {
      invoke("lap");
    } else if (["1", "2", "3"].includes(e.key)) {
      const kind = ["stopwatch", "countdown", "pomodoro"][Number(e.key) - 1];
      if (kind !== state.timer?.kind) invoke("switch_kind", { kind });
    }
  });
}

async function togglePrimary() {
  const t = state.timer;
  if (!t) return;
  if (t.running) await invoke("pause");
  else await invoke("start");
}

async function secondaryAction() {
  const t = state.timer;
  if (!t) return;
  if (t.kind === "stopwatch") await invoke("lap");
  else if (t.kind === "pomodoro") await invoke("skip_phase");
  else await invoke("add_time", { ms: 60000 });
}

async function patchSettings(patch) {
  const next = await invoke("update_settings", { patch });
  if (next) {
    state.settings = next;
    applyAccent(next.accent);
    renderSettings();
  }
}

async function importSound() {
  await invoke("import_sound");
}

async function previewSound(id) {
  await invoke("preview_sound", { id });
  toast("试听中…");
}

async function removeSound(id) {
  await invoke("remove_sound", { id });
  state.sounds = state.sounds.filter((s) => s.id !== id);
  if (state.settings.sound_id === id) state.settings.sound_id = state.sounds[0]?.id ?? null;
  renderSounds();
}

/* ---------------------------- 预设操作 ---------------------------- */
function openPresetSheet() {
  $("presetName").value = "";
  $("presetMin").value = "3";
  $("presetSec").value = "0";
  state.newPresetColor = COLORS[state.presets.length % COLORS.length];
  renderPresetColors();
  renderQuickDurations();
  updatePresetPreview();
  $("presetSheet").hidden = false;
  setTimeout(() => $("presetName").focus(), 60);
}

function renderQuickDurations() {
  const chips = $("presetQuick");
  chips.innerHTML = [1, 3, 5, 10, 25]
    .map((m) => `<button class="quick-chip" data-min="${m}">${m} 分钟</button>`)
    .join("");
  chips.onclick = (e) => {
    const chip = e.target.closest("[data-min]");
    if (!chip) return;
    $("presetMin").value = chip.dataset.min;
    $("presetSec").value = "0";
    updatePresetPreview();
  };
}

function updatePresetPreview() {
  const min = Number($("presetMin").value) || 0;
  const sec = Number($("presetSec").value) || 0;
  $("presetPreview").textContent = fmt(Math.max(0, (min * 60 + sec) * 1000));
}

["presetMin", "presetSec"].forEach((id) => {
  $(id).addEventListener("input", updatePresetPreview);
});

function renderPresetColors() {
  $("presetColors").innerHTML = COLORS.map(
    (c) =>
      `<button class="color-dot ${state.newPresetColor === c ? "on" : ""}" data-color="${c}" style="background:${c}"></button>`
  ).join("");
  $("presetColors").onclick = (e) => {
    const d = e.target.closest("[data-color]");
    if (!d) return;
    state.newPresetColor = d.dataset.color;
    renderPresetColors();
  };
}

function closePresetSheet() {
  $("presetSheet").hidden = true;
}

async function savePreset() {
  const name = ($("presetName").value || "").trim() || "计时器";
  const min = Number($("presetMin").value) || 0;
  const sec = Number($("presetSec").value) || 0;
  const ms = Math.max(1000, (min * 60 + sec) * 1000);
  const p = await invoke("add_preset", { name, ms, color: state.newPresetColor });
  if (p) {
    state.presets.push(p);
    renderPresets();
    closePresetSheet();
    await invoke("start_preset", { id: p.id });
  }
}

async function removePreset(id) {
  await invoke("remove_preset", { id });
  state.presets = state.presets.filter((p) => p.id !== id);
  renderPresets();
}

/* ---------------------------- 窗口模式 / 抽屉 ---------------------------- */
async function setMode(mode) {
  state.mode = mode;
  document.body.classList.toggle("mini", mode === "mini");
  $("miniView").hidden = mode !== "mini";
  await invoke("set_mode", { mode });
  if (mode === "mini") closeDrawer();
}

function openSettings() {
  $("settingsDrawer").hidden = false;
  renderSettings();
}

function closeDrawer() {
  $("settingsDrawer").hidden = true;
}

/* ---------------------------- 提示 ---------------------------- */
function toast(text) {
  const el = $("toast");
  el.textContent = text;
  el.hidden = false;
  requestAnimationFrame(() => el.classList.add("show"));
  clearTimeout(state.toastTimer);
  state.toastTimer = setTimeout(() => {
    el.classList.remove("show");
    setTimeout(() => (el.hidden = true), 220);
  }, 2200);
}

function previewVariantIsMini() {
  return new URLSearchParams(location.search).get("mock") === "mini";
}

window.__timerApp = { state, toast, isDesktop };

if (new URLSearchParams(location.search).has("debug")) {
  setTimeout(() => {
    const q = (sel) => {
      const el = document.querySelector(sel);
      return el ? Math.round(el.getBoundingClientRect().width) + "@" + Math.round(el.getBoundingClientRect().left) : null;
    };
    const box = document.createElement("div");
    box.style.cssText = "position:fixed;left:0;top:0;z-index:9999;background:#fff;color:#000;font:13px monospace;padding:6px;white-space:pre;line-height:1.5";
    box.textContent = JSON.stringify({
      vw: window.innerWidth, dpr: window.devicePixelRatio,
      body: document.body.scrollWidth, app: q("#app"), card: q(".card"),
      nv: q(".normal-view"), tabs: q(".tabs"), controls: q(".controls"),
      presets: q(".presets"), list: document.querySelector(".preset-list")?.scrollWidth,
      stage: q(".stage"), ring: q(".ring-wrap"), digits: q(".digits"),
    }, null, 1).replace(/[{}"]/g, "");
    document.body.appendChild(box);
  }, 500);
}
