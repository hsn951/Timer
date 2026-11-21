// Tauri IPC 封装 + 浏览器预览用的本地模拟实现
const tauri = window.__TAURI__;
export const isDesktop = !!tauri;
const core = tauri?.core;
const ev = tauri?.event;

const listeners = new Map();

function emit(name, payload) {
  (listeners.get(name) || []).forEach((fn) => fn(payload));
}

export function listen(name, cb) {
  if (isDesktop) {
    return ev.listen(name, (e) => cb(e.payload));
  }
  if (!listeners.has(name)) listeners.set(name, []);
  listeners.get(name).push(cb);
  return Promise.resolve(() => {
    const arr = listeners.get(name) || [];
    const i = arr.indexOf(cb);
    if (i >= 0) arr.splice(i, 1);
  });
}

export async function invoke(cmd, args = {}) {
  if (isDesktop) return core.invoke(cmd, args);
  return mockInvoke(cmd, args);
}

export function bootstrap() {
  return invoke("bootstrap");
}

/* ------------------------- 浏览器预览 mock ------------------------- */
const mock = {
  settings: {
    volume: 0.7,
    fade: true,
    muted: false,
    sound_id: "s1",
    always_on_top: false,
    autostart: false,
    accent: "pink",
    work_min: 25,
    break_min: 5,
    long_break_min: 15,
    rounds: 4,
    auto_start: true,
  },
  presets: [
    { id: "p1", name: "泡面", ms: 180000, color: "#FFB3C7" },
    { id: "p2", name: "咖啡", ms: 240000, color: "#FFD98E" },
    { id: "p3", name: "冥想", ms: 600000, color: "#9FD3FF" },
  ],
  sounds: [
    { id: "s1", name: "清脆铃声", builtin: true, duration_ms: 3200 },
    { id: "s2", name: "柔和钟声", builtin: true, duration_ms: 4100 },
    { id: "s3", name: "我的铃声.mp3", builtin: false, duration_ms: 2800 },
  ],
  timer: {
    kind: "countdown",
    phase: "work",
    round: 1,
    rounds: 4,
    label: "专注",
    target_ms: 1500000,
    elapsed_ms: 0,
    remaining_ms: 1500000,
    running: false,
    finished: false,
    laps: [],
  },
};

let timerHandle = null;

const previewParams = new URLSearchParams(location.search);
const previewVariant = previewParams.get("mock");

function snapshot() {
  const t = mock.timer;
  const remaining =
    t.kind === "stopwatch" ? 0 : Math.max(0, (t.target_ms || 0) - t.elapsed_ms);
  return { ...t, remaining_ms: remaining, rounds: mock.settings.rounds };
}

if (previewVariant === "stopwatch") {
  Object.assign(mock.timer, {
    kind: "stopwatch",
    label: "秒表",
    target_ms: 0,
    elapsed_ms: 142350,
    running: true,
    laps: [41200, 101150],
  });
} else if (previewVariant === "pomodoro") {
  Object.assign(mock.timer, {
    kind: "pomodoro",
    phase: "work",
    round: 2,
    target_ms: 1500000,
    elapsed_ms: 420000,
    running: true,
    label: "专注",
  });
} else if (previewVariant === "mini") {
  Object.assign(mock.timer, {
    kind: "countdown",
    label: "专注",
    target_ms: 1500000,
    elapsed_ms: 420000,
    running: true,
  });
}
if (mock.timer.running) startMockLoop();

function startMockLoop() {
  if (timerHandle) return;
  timerHandle = setInterval(() => {
    const t = mock.timer;
    if (!t.running) return;
    t.elapsed_ms += 50;
    if (t.kind !== "stopwatch" && t.elapsed_ms >= t.target_ms) {
      t.elapsed_ms = t.target_ms;
      if (t.kind === "pomodoro" && mock.settings.auto_start) {
        const next = t.phase === "work" ? "short" : "work";
        t.phase = next;
        t.round = next === "work" ? t.round + 1 : t.round;
        t.target_ms = (next === "work" ? mock.settings.work_min : mock.settings.break_min) * 60000;
        t.elapsed_ms = 0;
        t.label = next === "work" ? "专注" : "休息";
      } else {
        t.running = false;
        t.finished = true;
      }
      emit("timer://finished", { label: t.label });
      emit("timer://state", snapshot());
    }
    emit("timer://update", snapshot());
  }, 50);
}

function mockInvoke(cmd, args = {}) {
  const t = mock.timer;
  switch (cmd) {
    case "bootstrap":
      return {
        timer: snapshot(),
        settings: mock.settings,
        presets: mock.presets,
        sounds: mock.sounds,
        version: "0.1.0-preview",
      };
    case "start":
      if (t.finished) {
        t.elapsed_ms = 0;
        t.finished = false;
      }
      if (t.kind === "pomodoro") {
        t.target_ms =
          (t.phase === "work" ? mock.settings.work_min : mock.settings.break_min) * 60000;
      }
      t.running = true;
      t.finished = false;
      startMockLoop();
      emit("timer://state", snapshot());
      return null;
    case "pause":
      t.running = false;
      emit("timer://state", snapshot());
      return null;
    case "reset":
      t.elapsed_ms = 0;
      t.running = false;
      t.finished = false;
      t.laps = [];
      emit("timer://state", snapshot());
      return null;
    case "lap":
      if (t.kind === "stopwatch" && t.running) t.laps.unshift(t.elapsed_ms);
      emit("timer://state", snapshot());
      return null;
    case "switch_kind":
      t.kind = args.kind;
      t.elapsed_ms = 0;
      t.running = false;
      t.finished = false;
      t.laps = [];
      t.label = args.kind === "countdown" ? "倒计时" : args.kind === "pomodoro" ? "专注" : "秒表";
      t.target_ms =
        args.kind === "countdown" ? 300000 : args.kind === "pomodoro" ? mock.settings.work_min * 60000 : 0;
      emit("timer://state", snapshot());
      return null;
    case "set_countdown":
      t.target_ms = Math.max(1000, args.ms);
      t.elapsed_ms = 0;
      t.finished = false;
      t.label = "倒计时";
      emit("timer://state", snapshot());
      return null;
    case "start_preset": {
      const p = mock.presets.find((x) => x.id === args.id);
      if (!p) return null;
      t.kind = "countdown";
      t.target_ms = p.ms;
      t.elapsed_ms = 0;
      t.label = p.name;
      t.running = true;
      t.finished = false;
      startMockLoop();
      emit("timer://state", snapshot());
      return null;
    }
    case "add_preset": {
      const p = {
        id: "p" + (mock.presets.length + 1),
        name: args.name || "计时器",
        ms: Math.max(1000, args.ms),
        color: args.color || "#FFB3C7",
      };
      mock.presets.push(p);
      return p;
    }
    case "remove_preset":
      mock.presets = mock.presets.filter((x) => x.id !== args.id);
      return null;
    case "update_settings":
      Object.assign(mock.settings, args.patch || {});
      return mock.settings;
    case "set_mode":
      return null;
    case "import_sound":
      emit("toast", { text: "浏览器预览模式不支持导入音频" });
      return null;
    case "preview_sound":
      return null;
    case "stop_sound":
      return null;
    case "remove_sound":
      mock.sounds = mock.sounds.filter((s) => s.id !== args.id);
      if (mock.settings.sound_id === args.id) mock.settings.sound_id = mock.sounds[0]?.id ?? null;
      return null;
    case "hide_window":
      emit("toast", { text: "浏览器预览：窗口已隐藏（模拟）" });
      return null;
    default:
      return null;
  }
}
