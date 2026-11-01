import type { PeriodKey } from './types';

const pad = (n: number) => String(n).padStart(2, '0');
export const iso = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;

export function parseIso(s: string): Date {
  const [y, m, d] = s.split('-').map(Number);
  return new Date(y, m - 1, d);
}

export function addDays(d: Date, n: number) {
  const x = new Date(d);
  x.setDate(x.getDate() + n);
  return x;
}

/** Диапазон дат периода. Неделя и месяц — скользящие (последние 7 и 30 дней), год — календарный. */
export function periodRange(p: PeriodKey, today: Date, first: string): { from: string; to: string } {
  const to = iso(today);
  switch (p) {
    case 'today':
      return { from: to, to };
    case 'week':
      return { from: iso(addDays(today, -6)), to };
    case 'month':
      return { from: iso(addDays(today, -29)), to };
    case 'year':
      return { from: `${today.getFullYear()}-01-01`, to };
    case 'all':
      return { from: first < to ? first : to, to };
  }
}

export const PERIOD_LABEL: Record<PeriodKey, string> = {
  today: 'Сегодня',
  week: 'Неделя',
  month: 'Месяц',
  year: 'Год',
  all: 'Всё время',
};

const MONTHS_GEN = ['января', 'февраля', 'марта', 'апреля', 'мая', 'июня', 'июля', 'августа', 'сентября', 'октября', 'ноября', 'декабря'];
export function dateLabel(s: string, withYear = false) {
  const d = parseIso(s);
  return `${d.getDate()} ${MONTHS_GEN[d.getMonth()]}${withYear ? ' ' + d.getFullYear() : ''}`;
}
