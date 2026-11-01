// Таблица по дням для Excel. Числа — числами, даты — датами: Excel сам покажет их в формате системы.
import { km, scrollMeters } from './aggregate';
import type { DayStats } from './types';
import { xlsx, type Cell } from './xlsx';

const round = (v: number, digits: number) => Number(v.toFixed(digits));

const HEAD = [
  'Дата',
  'Нажатий',
  'Кликов',
  'Левой',
  'Правой',
  'Колёсиком',
  'Мышкой, км',
  'Колесом, м',
  'За компом, ч',
  'Сел',
  'Встал',
  'Самая длинная сессия, мин',
  'Пик нажатий в минуту',
  'Главная программа',
];

export function daysRows(days: DayStats[]): Cell[][] {
  return days.map((d) => {
    const top = Object.entries(d.apps).sort((a, b) => b[1] - a[1])[0]?.[0] ?? '';
    return [
      { date: d.date },
      d.keys,
      d.clicksLeft + d.clicksRight + d.clicksMiddle + d.clicksOther,
      d.clicksLeft,
      d.clicksRight,
      d.clicksMiddle,
      round(km(d.mouseMm), 3),
      round(scrollMeters(d.wheelNotches), 1),
      round(d.activeSeconds / 3600, 2),
      d.firstActive,
      d.lastActive,
      Math.round(d.longestSessionSec / 60),
      d.peakKpm,
      top,
    ];
  });
}

export const daysXlsx = (days: DayStats[]) => xlsx('pc_wrapped', HEAD, daysRows(days));
