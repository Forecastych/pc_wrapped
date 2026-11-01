import { describe, expect, it } from 'vitest';
import { baselineRange, baselineTotals, delta, deltaText } from './compare';
import { daysRows, daysXlsx } from './table';
import { crc32, excelDate } from './xlsx';
import { periodRange } from './dates';
import type { DayStats } from './types';

function day(date: string, p: Partial<DayStats> = {}): DayStats {
  return {
    date, keys: 0, keyCounts: {}, clicksLeft: 0, clicksRight: 0, clicksMiddle: 0, clicksOther: 0, wheelNotches: 0, mouseMm: 0,
    activeSeconds: 0, hourlyKeys: new Array(24).fill(0), hourlyClicks: new Array(24).fill(0), hourlyMouseMm: new Array(24).fill(0),
    hourlyActive: new Array(24).fill(0), apps: {}, peakKpm: 0, peakKpmAt: '', firstActive: '', lastActive: '', longestSessionSec: 0, ...p,
  };
}

const hourly = (h: number, v: number) => Object.assign(new Array(24).fill(0), { [h]: v });

describe('сравнение с прошлым', () => {
  const today = new Date(2026, 10, 4, 12, 30); // ср, 4 ноября, 12:30

  it('диапазоны и проверка, что программа тогда уже считала', () => {
    expect(baselineRange('today', today, '2026-11-01')).toEqual({ from: '2026-10-28', to: '2026-11-03' });
    expect(baselineRange('week', today, '2026-10-01')).toEqual({ from: '2026-10-22', to: '2026-10-28' });
    expect(baselineRange('week', today, '2026-10-25')).toBeNull();
    expect(baselineRange('month', today, '2026-09-01')).toEqual({ from: '2026-09-06', to: '2026-10-05' });
    expect(baselineRange('year', today, '2025-01-01')).toEqual({ from: '2025-01-01', to: '2025-11-04' });
    expect(baselineRange('all', today, '2020-01-01')).toBeNull();
  });

  it('сегодня сравнивается со средним днём к этому же часу', () => {
    const a = day('2026-11-02', {
      activeSeconds: 7200, wheelNotches: 100,
      hourlyKeys: hourly(10, 1000), hourlyActive: Object.assign(hourly(10, 3600), { 12: 1800, 20: 1800 }),
    });
    a.hourlyKeys[12] = 400;
    a.hourlyKeys[20] = 5000; // вечер ещё не наступил — не считается
    const empty = day('2026-11-03'); // не включал комп — в среднее не идёт
    const t = baselineTotals('today', [a, empty], today)!;
    expect(t.keys).toBe(1000 + 400 * 0.5);
    expect(t.activeSeconds).toBe(3600 + 900);
    expect(t.wheelNotches).toBeCloseTo(100 * (4500 / 7200));
    expect(baselineTotals('today', [empty], today)).toBeNull();
  });

  it('проценты', () => {
    expect(delta(120, 100)).toBeCloseTo(0.2);
    expect(delta(5, 0)).toBeNull();
    expect(deltaText(0.2)).toBe('▲ 20%');
    expect(deltaText(-0.355)).toBe('▼ 36%');
    expect(deltaText(0.01)).toBe('≈ как обычно');
    expect(deltaText(14)).toBe('▲ ×15');
  });

  it('прошлые календарные неделя и месяц', () => {
    expect(periodRange('lastweek', today, '')).toEqual({ from: '2026-10-26', to: '2026-11-01' });
    expect(periodRange('lastweek', new Date(2026, 10, 2), '')).toEqual({ from: '2026-10-26', to: '2026-11-01' });
    expect(periodRange('lastmonth', today, '')).toEqual({ from: '2026-10-01', to: '2026-10-31' });
    expect(periodRange('lastmonth', new Date(2026, 0, 10), '')).toEqual({ from: '2025-12-01', to: '2025-12-31' });
  });
});

describe('Excel', () => {
  const d = day('2026-11-01', { keys: 1234, clicksLeft: 10, clicksRight: 2, mouseMm: 1_234_567, wheelNotches: 100, activeSeconds: 5400, apps: { 'My <App> & co': 10, Code: 5 } });

  it('строки: дата датой, числа числами', () => {
    const row = daysRows([d])[0];
    expect(row.slice(0, 9)).toEqual([{ date: '2026-11-01' }, 1234, 12, 10, 2, 0, 1.235, 1.5, 1.5]);
    expect(row[13]).toBe('My <App> & co');
  });

  it('дата Excel и crc32', () => {
    expect(excelDate('1900-03-01')).toBe(61);
    expect(excelDate('2026-11-01')).toBe(46327);
    expect(crc32(new TextEncoder().encode('123456789'))).toBe(0xcbf43926);
  });

  it('zip: сигнатуры и экранирование', () => {
    const z = daysXlsx([d]);
    const v = new DataView(z.buffer);
    expect(v.getUint32(0, true)).toBe(0x04034b50);
    expect(v.getUint32(z.length - 22, true)).toBe(0x06054b50);
    expect(v.getUint16(z.length - 22 + 10, true)).toBe(6);
    const text = new TextDecoder().decode(z);
    expect(text).toContain('My &lt;App&gt; &amp; co');
    expect(text).toContain('<c r="A2" s="2"><v>46327</v></c>');
  });
});
