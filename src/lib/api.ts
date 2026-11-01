// Мост к Rust. В обычном браузере (npm run dev без Tauri) работает на выдуманных данных,
// чтобы интерфейс можно было делать и смотреть без сборки приложения.
import type { DayStats, Settings } from './types';
import { addDays, iso } from './dates';

export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

type Unlisten = () => void;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

async function tauriListen<T>(event: string, cb: (p: T) => void): Promise<Unlisten> {
  const { listen } = await import('@tauri-apps/api/event');
  return listen<T>(event, (e) => cb(e.payload));
}

export async function windowLabel(): Promise<string> {
  if (!inTauri) return new URLSearchParams(location.search).get('window') ?? 'main';
  const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow');
  return getCurrentWebviewWindow().label;
}

export async function startDragging() {
  if (!inTauri) return;
  const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow');
  await getCurrentWebviewWindow().startDragging();
}

// ---------- Демо-данные ----------

function rng(seed: number) {
  return () => {
    seed = (seed * 1664525 + 1013904223) >>> 0;
    return seed / 4294967296;
  };
}

const LETTERS = 'ETAOINSHRDLUCMFWYPVBGKQJXZ';
const APPS: [string, number][] = [
  ['Google Chrome', 3.2],
  ['Visual Studio Code', 2.4],
  ['Telegram Desktop', 1.4],
  ['World of Warcraft', 2.8],
  ['Discord', 0.9],
  ['Проводник', 0.4],
  ['Battle.net', 0.3],
  ['Steam', 0.3],
];

function fakeDay(date: string, r: () => number, scale = 1): DayStats {
  const d = new Date(date);
  const weekend = d.getDay() === 0 || d.getDay() === 6;
  const z = () => new Array<number>(24).fill(0);
  const day: DayStats = {
    date, keys: 0, keyCounts: {}, clicksLeft: 0, clicksRight: 0, clicksMiddle: 0, clicksOther: 0, wheelNotches: 0, mouseMm: 0,
    activeSeconds: 0, hourlyKeys: z(), hourlyClicks: z(), hourlyMouseMm: z(), hourlyActive: z(), apps: {}, peakKpm: 0, peakKpmAt: '',
    firstActive: '', lastActive: '', longestSessionSec: 0,
  };
  if (r() < 0.08) return day;
  const start = 11 + Math.floor(r() * 3) - (weekend ? 0 : 1);
  const end = Math.min(23, start + 9 + Math.floor(r() * 5));
  for (let h = 0; h < 24; h++) {
    const inside = (h >= start && h <= end) || (h < 3 && r() < 0.45);
    if (!inside) continue;
    const a = Math.floor(1800 + r() * 1800) * scale;
    const keys = Math.floor(a * (0.4 + r() * 0.9));
    const clicks = Math.floor(a * (0.12 + r() * 0.15));
    const mm = a * (14 + r() * 18);
    day.hourlyActive[h] = Math.floor(a);
    day.hourlyKeys[h] = keys;
    day.hourlyClicks[h] = clicks;
    day.hourlyMouseMm[h] = mm;
  }
  day.activeSeconds = day.hourlyActive.reduce((x, y) => x + y, 0);
  day.keys = day.hourlyKeys.reduce((x, y) => x + y, 0);
  const clicks = day.hourlyClicks.reduce((x, y) => x + y, 0);
  day.clicksLeft = Math.floor(clicks * 0.86);
  day.clicksRight = Math.floor(clicks * 0.11);
  day.clicksMiddle = Math.floor(clicks * 0.02);
  day.clicksOther = clicks - day.clicksLeft - day.clicksRight - day.clicksMiddle;
  day.mouseMm = day.hourlyMouseMm.reduce((x, y) => x + y, 0);
  day.wheelNotches = Math.floor(day.activeSeconds * (0.08 + r() * 0.1));
  // клавиши: пробел и буквы по частоте, плюс WASD, Backspace, Enter
  let left = day.keys;
  const put = (vk: number, n: number) => {
    n = Math.min(left, Math.floor(n));
    if (n > 0) {
      day.keyCounts[vk] = (day.keyCounts[vk] ?? 0) + n;
      left -= n;
    }
  };
  put(0x20, day.keys * 0.15);
  put(0x08, day.keys * 0.07);
  put(0x0d, day.keys * 0.04);
  put(0x57, day.keys * 0.05);
  put(0x41, day.keys * 0.03);
  put(0x44, day.keys * 0.03);
  put(0x10, day.keys * 0.03);
  put(0x11, day.keys * 0.03);
  [...LETTERS].forEach((c, i) => put(c.charCodeAt(0), day.keys * 0.045 * Math.pow(0.88, i)));
  // остаток раскидываем по буквам, чтобы цифры и Esc не лезли в топ
  [...LETTERS].forEach((c, i) => put(c.charCodeAt(0), (left * 0.9) / (i + 2)));
  put(0x31, left / 2);
  put(0x1b, left);
  let rest = day.activeSeconds;
  for (const [name, w] of APPS) {
    const sec = Math.floor(day.activeSeconds * (w / 12) * (0.4 + r() * 1.2));
    if (sec > 0 && rest > 0) {
      day.apps[name] = Math.min(rest, sec);
      rest -= day.apps[name];
    }
  }
  day.peakKpm = 140 + Math.floor(r() * 200);
  const ph = start + Math.floor(r() * (end - start));
  day.peakKpmAt = `${String(ph).padStart(2, '0')}:${String(Math.floor(r() * 60)).padStart(2, '0')}`;
  day.firstActive = `${String(start).padStart(2, '0')}:${String(Math.floor(r() * 60)).padStart(2, '0')}`;
  day.lastActive = `${String(end).padStart(2, '0')}:${String(Math.floor(r() * 60)).padStart(2, '0')}`;
  day.longestSessionSec = Math.floor(3600 + r() * 4 * 3600);
  return day;
}

const DEMO_DAYS = 420;
let demo: Map<string, DayStats> | null = null;
let live: DayStats | null = null;
let demoSettings: Settings = {
  dpi: 800,
  trayMetrics: ['keys', 'mouse'],
  miniMetrics: ['keys', 'clicks', 'mouse', 'scroll'],
  miniClickthrough: false,
  miniOpen: false,
};

function ensureDemo() {
  if (demo) return demo;
  demo = new Map();
  const r = rng(42);
  const today = new Date();
  for (let i = DEMO_DAYS; i >= 1; i--) {
    const d = iso(addDays(today, -i));
    demo.set(d, fakeDay(d, r));
  }
  // сегодня: часть дня уже прошла
  const now = new Date();
  live = fakeDay(iso(now), rng(7), Math.min(1, (now.getHours() + 1) / 24) * 0.9);
  live.hourlyKeys.forEach((_, h) => {
    if (h > now.getHours()) {
      live!.hourlyKeys[h] = live!.hourlyActive[h] = live!.hourlyClicks[h] = live!.hourlyMouseMm[h] = 0;
    }
  });
  return demo;
}

const liveListeners = new Set<(d: DayStats) => void>();
let ticker: ReturnType<typeof setInterval> | null = null;
function startDemoTicker() {
  if (ticker) return;
  ticker = setInterval(() => {
    if (!live) return;
    const h = new Date().getHours();
    const k = 2 + Math.floor(Math.random() * 6);
    live.keys += k;
    live.hourlyKeys[h] += k;
    live.keyCounts[0x20] = (live.keyCounts[0x20] ?? 0) + 1;
    live.keyCounts[0x45] = (live.keyCounts[0x45] ?? 0) + k - 1;
    const mm = 40 + Math.random() * 160;
    live.mouseMm += mm;
    live.hourlyMouseMm[h] += mm;
    if (Math.random() < 0.5) {
      live.clicksLeft++;
      live.hourlyClicks[h]++;
    }
    live.wheelNotches += Math.random() < 0.3 ? 2 : 0;
    live.activeSeconds++;
    live.hourlyActive[h]++;
    const snap = structuredClone(live);
    liveListeners.forEach((cb) => cb(snap));
  }, 1000);
}

// ---------- Публичный API ----------

export async function getToday(): Promise<DayStats> {
  if (inTauri) return tauriInvoke('get_today');
  ensureDemo();
  return structuredClone(live!);
}

export async function getDays(from: string, to: string): Promise<DayStats[]> {
  if (inTauri) return tauriInvoke('get_days', { from, to });
  const m = ensureDemo();
  const out = [...m.values()].filter((d) => d.date >= from && d.date <= to);
  if (live && live.date >= from && live.date <= to) out.push(structuredClone(live));
  return out;
}

export async function firstDate(): Promise<string> {
  if (inTauri) return (await tauriInvoke<string | null>('first_date')) ?? iso(new Date());
  return [...ensureDemo().keys()][0];
}

export async function getSettings(): Promise<Settings> {
  if (inTauri) return tauriInvoke('get_settings');
  return structuredClone(demoSettings);
}

export async function setSettings(settings: Settings): Promise<void> {
  if (inTauri) return tauriInvoke('set_settings', { settings });
  demoSettings = structuredClone(settings);
}

export async function toggleMini(): Promise<void> {
  if (inTauri) return tauriInvoke('toggle_mini');
  window.open('?window=mini', 'mini', 'width=232,height=168');
}

export async function openDataDir(): Promise<void> {
  if (inTauri) return tauriInvoke('open_data_dir');
}

export async function getAutostart(): Promise<boolean> {
  if (inTauri) return tauriInvoke('get_autostart');
  return false;
}

export async function setAutostart(on: boolean): Promise<void> {
  if (inTauri) return tauriInvoke('set_autostart', { on });
}

export async function onLive(cb: (d: DayStats) => void): Promise<Unlisten> {
  if (inTauri) return tauriListen('live', cb);
  ensureDemo();
  liveListeners.add(cb);
  startDemoTicker();
  return () => liveListeners.delete(cb);
}

export async function onSettings(cb: (s: Settings) => void): Promise<Unlisten> {
  if (inTauri) return tauriListen('settings', cb);
  return () => {};
}
