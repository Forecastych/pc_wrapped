import { describe, expect, it } from 'vitest';
import { aggregate, scrollMeters } from './aggregate';
import { mouseFun, scrollFun, keysFun, clicksFun, activeFun, compare, pct, DISTANCES, HEIGHTS, TEXTS, DURATIONS, type Ref } from './fun';
import { periodRange } from './dates';
import { keyName } from './keyboard';
import type { DayStats } from './types';

function day(date: string, p: Partial<DayStats> = {}): DayStats {
  return {
    date, keys: 0, keyCounts: {}, clicksLeft: 0, clicksRight: 0, clicksMiddle: 0, clicksOther: 0, wheelNotches: 0, mouseMm: 0,
    activeSeconds: 0, hourlyKeys: new Array(24).fill(0), hourlyClicks: new Array(24).fill(0), hourlyMouseMm: new Array(24).fill(0),
    hourlyActive: new Array(24).fill(0), apps: {}, peakKpm: 0, peakKpmAt: '', firstActive: '', lastActive: '', longestSessionSec: 0, ...p,
  };
}

describe('aggregate', () => {
  const a = day('2026-10-30', {
    keys: 100, keyCounts: { '65': 60, '8': 40 }, clicksLeft: 10, clicksRight: 2, mouseMm: 5000, wheelNotches: 100,
    activeSeconds: 3600, apps: { Code: 3000, Chrome: 600 }, peakKpm: 80, peakKpmAt: '10:15', firstActive: '09:00', lastActive: '23:30',
    longestSessionSec: 2000, hourlyActive: Object.assign(new Array(24).fill(0), { 2: 600, 10: 3000 }),
  });
  const b = day('2026-10-31', {
    keys: 300, keyCounts: { '65': 100, '32': 200 }, clicksLeft: 5, mouseMm: 9000, activeSeconds: 7200, apps: { Chrome: 7000 },
    peakKpm: 120, peakKpmAt: '21:00', firstActive: '11:00', lastActive: '01:30', longestSessionSec: 5000,
  });
  const empty = day('2026-11-01');
  const s = aggregate([a, b, empty]);

  it('суммирует', () => {
    expect(s.dayCount).toBe(2);
    expect(s.keys).toBe(400);
    expect(s.clicks).toBe(17);
    expect(s.mouseMm).toBe(14000);
    expect(s.keyCounts[0]).toEqual([32, 200]);
    expect(s.keyMap[65]).toBe(160);
    expect(s.apps[0]).toEqual(['Chrome', 7600]);
  });

  it('рекорды и привычки', () => {
    expect(s.peakKpm).toEqual({ value: 120, date: '2026-10-31', at: '21:00' });
    expect(s.longestSession?.sec).toBe(5000);
    expect(s.bestDay?.date).toBe('2026-10-31');
    expect(s.typicalStart).toBe('10:00');
    // 23:30 и 01:30 следующего дня → медиана 00:30
    expect(s.typicalEnd).toBe('00:30');
    expect(s.nightShare).toBeCloseTo(600 / 10800);
  });

  it('пустой период не падает', () => {
    const e = aggregate([]);
    expect(e.keys).toBe(0);
    expect(e.typicalStart).toBeNull();
  });
});

describe('fun', () => {
  it('сравнения', () => {
    expect(mouseFun(0.5)).toBe('мышка пока отдыхает');
    expect(scrollFun(1)).toBe('колесо почти не крутили');
    expect(keysFun(50)).toBe('клавиатура скучает');
    expect(clicksFun(3)).toBe('мышь почти не щёлкала');
    expect(activeFun(100)).toBe('меньше одной песни');
    expect(scrollMeters(200)).toBe(3);

    const bus: Ref = { v: 12, forms: ['автобус', 'автобуса', 'автобусов'], tail: ' в длину' };
    const moon: Ref = { v: 384_400_000, forms: ['путь до Луны', 'пути до Луны', 'путей до Луны'], part: 'пути до Луны', min: 0.0001 };
    expect(compare(24, [bus])).toBe('2 автобуса в длину');
    expect(compare(18, [bus])).toBe('1,5 автобуса в длину');
    expect(compare(6, [bus])).toBeUndefined(); // долей у автобуса нет
    expect(compare(1200, [bus])).toBe('100 автобусов в длину'); // больше 40 раз — всё равно в самом крупном
    expect(compare(192_200_000, [moon])).toBe('50% пути до Луны');
    expect(compare(38_440, [moon])).toBe('0,01% пути до Луны');
    expect(compare(3844, [moon])).toBeUndefined();
    expect(pct(0.042)).toBe('4,2%');
    expect(pct(0.0035)).toBe('0,35%');
    expect(pct(0.001)).toBe('0,1%');
  });

  it('справочники: на любом масштабе осмысленная фраза и разнообразие', () => {
    const sweep = (from: number, to: number, f: (x: number) => string) => {
      const seen = new Set<string>();
      for (let x = from; x < to; x *= 1.07) {
        const s = f(x);
        expect(s, `${x}`).not.toMatch(/undefined|NaN|Infinity| 0% |^это $/);
        expect(f(x)).toBe(s); // выбор стабилен
        seen.add(s.replace(/[\d,  %]+/g, '#'));
      }
      return seen.size;
    };
    expect(sweep(1, 2e9, mouseFun)).toBeGreaterThan(20);
    expect(sweep(3, 1e7, scrollFun)).toBeGreaterThan(15);
    expect(sweep(100, 1e9, keysFun)).toBeGreaterThan(10);
    expect(sweep(10, 1e8, clicksFun)).toBeGreaterThan(15);
    expect(sweep(210, 1e8, activeFun)).toBeGreaterThan(12);
    for (const refs of [DISTANCES, HEIGHTS, TEXTS, DURATIONS])
      for (let i = 1; i < refs.length; i++) expect(refs[i].v, refs[i].part).toBeGreaterThan(refs[i - 1].v);
  });
  it('периоды и клавиши', () => {
    const t = new Date(2026, 10, 1);
    expect(periodRange('week', t, '2026-01-01')).toEqual({ from: '2026-10-26', to: '2026-11-01' });
    expect(periodRange('year', t, '2025-05-05').from).toBe('2026-01-01');
    expect(periodRange('all', t, '2025-05-05').from).toBe('2025-05-05');
    expect(keyName(0x20)).toBe('Пробел');
    expect(keyName(0x41)).toBe('A');
    expect(keyName(0x70)).toBe('F1');
  });
});
