import type { Summary } from './aggregate';
import { scrollMeters } from './aggregate';

export type SlideKey = 'intro' | 'active' | 'mouse' | 'scroll' | 'clicks' | 'keys' | 'topkeys' | 'apps' | 'clock' | 'records' | 'summary';

export interface SlideDef {
  key: SlideKey;
  bg: string;
  fg: string;
}

const T = {
  violet: ['var(--violet)', 'var(--milk)'],
  lemon: ['var(--lemon)', 'var(--ink)'],
  mint: ['var(--mint)', 'var(--ink)'],
  pink: ['var(--pink)', 'var(--ink)'],
  sky: ['var(--sky)', 'var(--ink)'],
  ink: ['var(--ink)', 'var(--milk)'],
  tangerine: ['var(--tangerine)', 'var(--ink)'],
} as const;

export function buildSlides(s: Summary): SlideDef[] {
  const out: [SlideKey, keyof typeof T][] = [['intro', 'violet']];
  if (s.activeSeconds >= 60) out.push(['active', 'tangerine']);
  if (s.mouseMm >= 1000) out.push(['mouse', 'pink']);
  if (scrollMeters(s.wheelNotches) >= 3) out.push(['scroll', 'sky']);
  if (s.clicks >= 10) out.push(['clicks', 'mint']);
  if (s.keys >= 50) out.push(['keys', 'lemon']);
  if (s.keyCounts.length >= 3) out.push(['topkeys', 'ink']);
  if (s.apps.length >= 1) out.push(['apps', 'mint']);
  if (s.activeSeconds >= 600) out.push(['clock', 'violet']);
  if (s.longestSession || s.peakKpm) out.push(['records', 'pink']);
  out.push(['summary', 'lemon']);
  return out.map(([key, t]) => ({ key, bg: T[t][0], fg: T[t][1] }));
}
