import type { DayStats } from './types';

export interface Summary {
  dayCount: number;
  keys: number;
  clicks: number;
  clicksLeft: number;
  clicksRight: number;
  clicksMiddle: number;
  clicksOther: number;
  wheelNotches: number;
  mouseMm: number;
  activeSeconds: number;
  keyCounts: [number, number][];
  keyMap: Record<number, number>;
  apps: [string, number][];
  hourlyKeys: number[];
  hourlyClicks: number[];
  hourlyMouseMm: number[];
  hourlyActive: number[];
  peakKpm: { value: number; date: string; at: string } | null;
  longestSession: { sec: number; date: string } | null;
  bestDay: { date: string; keys: number } | null;
  bestMouseDay: { date: string; mm: number } | null;
  typicalStart: string | null;
  typicalEnd: string | null;
  nightShare: number;
  perDay: { date: string; keys: number; mouseMm: number; activeSeconds: number }[];
}

const zeros = () => new Array<number>(24).fill(0);

function toMin(hm: string): number | null {
  const m = /^(\d{2}):(\d{2})$/.exec(hm);
  return m ? +m[1] * 60 + +m[2] : null;
}
function fromMin(x: number) {
  const h = Math.floor(x / 60) % 24;
  return `${String(h).padStart(2, '0')}:${String(Math.round(x % 60)).padStart(2, '0')}`;
}
function median(xs: number[]): number | null {
  if (!xs.length) return null;
  const s = [...xs].sort((a, b) => a - b);
  const i = s.length >> 1;
  return s.length % 2 ? s[i] : (s[i - 1] + s[i]) / 2;
}

/** Склеивает дни в одну сводку за период. */
export function aggregate(days: DayStats[]): Summary {
  const s: Summary = {
    dayCount: 0,
    keys: 0,
    clicks: 0,
    clicksLeft: 0,
    clicksRight: 0,
    clicksMiddle: 0,
    clicksOther: 0,
    wheelNotches: 0,
    mouseMm: 0,
    activeSeconds: 0,
    keyCounts: [],
    keyMap: {},
    apps: [],
    hourlyKeys: zeros(),
    hourlyClicks: zeros(),
    hourlyMouseMm: zeros(),
    hourlyActive: zeros(),
    peakKpm: null,
    longestSession: null,
    bestDay: null,
    bestMouseDay: null,
    typicalStart: null,
    typicalEnd: null,
    nightShare: 0,
    perDay: [],
  };
  const apps = new Map<string, number>();
  const starts: number[] = [];
  const ends: number[] = [];

  for (const d of days) {
    const used = d.activeSeconds > 0 || d.keys > 0;
    if (used) s.dayCount++;
    s.keys += d.keys;
    s.clicksLeft += d.clicksLeft;
    s.clicksRight += d.clicksRight;
    s.clicksMiddle += d.clicksMiddle;
    s.clicksOther += d.clicksOther;
    s.wheelNotches += d.wheelNotches;
    s.mouseMm += d.mouseMm;
    s.activeSeconds += d.activeSeconds;
    for (const [k, v] of Object.entries(d.keyCounts ?? {})) s.keyMap[+k] = (s.keyMap[+k] ?? 0) + v;
    for (const [k, v] of Object.entries(d.apps ?? {})) apps.set(k, (apps.get(k) ?? 0) + v);
    for (let h = 0; h < 24; h++) {
      s.hourlyKeys[h] += d.hourlyKeys?.[h] ?? 0;
      s.hourlyClicks[h] += d.hourlyClicks?.[h] ?? 0;
      s.hourlyMouseMm[h] += d.hourlyMouseMm?.[h] ?? 0;
      s.hourlyActive[h] += d.hourlyActive?.[h] ?? 0;
    }
    if (d.peakKpm && (!s.peakKpm || d.peakKpm > s.peakKpm.value)) s.peakKpm = { value: d.peakKpm, date: d.date, at: d.peakKpmAt };
    if (d.longestSessionSec && (!s.longestSession || d.longestSessionSec > s.longestSession.sec))
      s.longestSession = { sec: d.longestSessionSec, date: d.date };
    if (d.keys && (!s.bestDay || d.keys > s.bestDay.keys)) s.bestDay = { date: d.date, keys: d.keys };
    if (d.mouseMm && (!s.bestMouseDay || d.mouseMm > s.bestMouseDay.mm)) s.bestMouseDay = { date: d.date, mm: d.mouseMm };
    // «Обычно садишься» считаем только по дням, когда за компом провели хотя бы полчаса
    if (d.activeSeconds >= 1800) {
      const a = toMin(d.firstActive);
      const b = toMin(d.lastActive);
      if (a !== null) starts.push(a);
      if (b !== null) ends.push(b < (a ?? 0) ? b + 1440 : b);
    }
    s.perDay.push({ date: d.date, keys: d.keys, mouseMm: d.mouseMm, activeSeconds: d.activeSeconds });
  }

  s.clicks = s.clicksLeft + s.clicksRight + s.clicksMiddle + s.clicksOther;
  s.keyCounts = Object.entries(s.keyMap)
    .map(([k, v]) => [+k, v] as [number, number])
    .sort((a, b) => b[1] - a[1]);
  s.apps = [...apps.entries()].sort((a, b) => b[1] - a[1]);
  const ms = median(starts);
  const me = median(ends);
  s.typicalStart = ms === null ? null : fromMin(ms);
  s.typicalEnd = me === null ? null : fromMin(me);
  const night = s.hourlyActive.slice(0, 6).reduce((a, b) => a + b, 0);
  s.nightShare = s.activeSeconds ? night / s.activeSeconds : 0;
  return s;
}

export const km = (mm: number) => mm / 1_000_000;
/** Один щелчок колеса ≈ 1,5 см прокрутки страницы на экране (так же считает трей). */
export const scrollMeters = (notches: number) => notches * 0.015;
