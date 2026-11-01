// Таблица по дням для Excel: точка с запятой, десятичная запятая, BOM — иначе русский Excel
// склеит всё в одну колонку и покажет кириллицу кракозябрами.
import { km, scrollMeters } from './aggregate';
import type { DayStats } from './types';

const dec = (v: number, digits: number) => v.toFixed(digits).replace('.', ',');
const cell = (s: string) => (/[;"\r\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s);

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

export function daysCsv(days: DayStats[]): string {
  const rows = days.map((d) => {
    const top = Object.entries(d.apps).sort((a, b) => b[1] - a[1])[0]?.[0] ?? '';
    return [
      d.date,
      d.keys,
      d.clicksLeft + d.clicksRight + d.clicksMiddle + d.clicksOther,
      d.clicksLeft,
      d.clicksRight,
      d.clicksMiddle,
      dec(km(d.mouseMm), 3),
      dec(scrollMeters(d.wheelNotches), 1),
      dec(d.activeSeconds / 3600, 2),
      d.firstActive,
      d.lastActive,
      Math.round(d.longestSessionSec / 60),
      d.peakKpm,
      cell(top),
    ].join(';');
  });
  return '﻿' + [HEAD.join(';'), ...rows].join('\r\n') + '\r\n';
}
