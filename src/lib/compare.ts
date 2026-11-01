// «Ты против себя»: с чем сравнивать текущий период и во сколько раз вышло больше или меньше.
import { addDays, iso } from './dates';
import type { Totals } from './metrics';
import type { DayStats, PeriodKey } from './types';

/**
 * Диапазон для сравнения. Сегодня — прошлые 7 дней (берётся среднее к этому часу), неделя и месяц —
 * такое же окно перед текущим, год — тот же отрезок прошлого года. `null`, если сравнивать не с чем:
 * «всё время», или программа в начале того окна ещё не считала, и разница вышла бы выдуманной.
 */
export function baselineRange(p: PeriodKey, today: Date, first: string): { from: string; to: string } | null {
  let r: { from: string; to: string };
  switch (p) {
    case 'today':
      return { from: iso(addDays(today, -7)), to: iso(addDays(today, -1)) };
    case 'week':
      r = { from: iso(addDays(today, -13)), to: iso(addDays(today, -7)) };
      break;
    case 'month':
      r = { from: iso(addDays(today, -59)), to: iso(addDays(today, -30)) };
      break;
    case 'year': {
      const y = today.getFullYear() - 1;
      r = { from: `${y}-01-01`, to: iso(new Date(y, today.getMonth(), today.getDate())) };
      break;
    }
    default:
      return null;
  }
  return first && first <= r.from ? r : null;
}

const zero = (): Totals => ({ keys: 0, clicks: 0, mouseMm: 0, wheelNotches: 0, activeSeconds: 0 });

/** Итог, с которым сравниваем. Для «сегодня» — средний день из `days`, обрезанный по текущее время. */
export function baselineTotals(p: PeriodKey, days: DayStats[], now: Date): Totals | null {
  const t = zero();
  if (p !== 'today') {
    for (const d of days) {
      t.keys += d.keys;
      t.clicks += d.clicksLeft + d.clicksRight + d.clicksMiddle + d.clicksOther;
      t.mouseMm += d.mouseMm;
      t.wheelNotches += d.wheelNotches;
      t.activeSeconds += d.activeSeconds;
    }
    return days.length ? t : null;
  }
  const used = days.filter((d) => d.activeSeconds > 0);
  if (!used.length) return null;
  const h = now.getHours();
  const frac = (now.getMinutes() * 60 + now.getSeconds()) / 3600;
  const upTo = (a: number[]) => a.slice(0, h).reduce((x, y) => x + y, 0) + (a[h] ?? 0) * frac;
  for (const d of used) {
    const active = upTo(d.hourlyActive);
    t.keys += upTo(d.hourlyKeys);
    t.clicks += upTo(d.hourlyClicks);
    t.mouseMm += upTo(d.hourlyMouseMm);
    t.activeSeconds += active;
    // по часам колесо не пишется — делим пропорционально времени за компом
    t.wheelNotches += (d.wheelNotches * active) / d.activeSeconds;
  }
  for (const k of Object.keys(t) as (keyof Totals)[]) t[k] /= used.length;
  return t;
}

/** Относительная разница или `null`, если база нулевая и процент ничего не значит. */
export function delta(cur: number, base: number): number | null {
  return base > 0 ? (cur - base) / base : null;
}

export function deltaText(d: number): string {
  if (Math.abs(d) < 0.03) return '≈ как обычно';
  const v = Math.abs(d) >= 10 ? `×${(1 + d).toFixed(0)}` : `${Math.round(Math.abs(d) * 100)}%`;
  return `${d > 0 ? '▲' : '▼'} ${v}`;
}

export const BASELINE_HINT: Partial<Record<PeriodKey, string>> = {
  today: 'чем обычно к этому часу (среднее за прошлую неделю)',
  week: 'чем за 7 дней до этого',
  month: 'чем за 30 дней до этого',
  year: 'чем за тот же отрезок прошлого года',
};
