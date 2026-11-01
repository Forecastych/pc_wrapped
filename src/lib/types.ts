// Зеркало структур из src-tauri/src/model.rs.

export interface DayStats {
  date: string;
  keys: number;
  keyCounts: Record<string, number>;
  clicksLeft: number;
  clicksRight: number;
  clicksMiddle: number;
  clicksOther: number;
  wheelNotches: number;
  mouseMm: number;
  activeSeconds: number;
  hourlyKeys: number[];
  hourlyClicks: number[];
  hourlyMouseMm: number[];
  hourlyActive: number[];
  apps: Record<string, number>;
  peakKpm: number;
  peakKpmAt: string;
  firstActive: string;
  lastActive: string;
  longestSessionSec: number;
}

export type Metric = 'keys' | 'clicks' | 'mouse' | 'scroll' | 'active';

export interface Settings {
  dpi: number;
  trayMetrics: Metric[];
  miniMetrics: Metric[];
  miniClickthrough: boolean;
  miniOpen: boolean;
  notifications: boolean;
  weeklyReport: boolean;
  monthlyReport: boolean;
  breakReminder: boolean;
  breakMinutes: number;
  lastWeekly: string;
  lastMonthly: string;
}

/** Периоды в шапке плюс прошлая календарная неделя и месяц — для итогов из уведомлений. */
export type PeriodKey = 'today' | 'week' | 'month' | 'year' | 'all' | 'lastweek' | 'lastmonth';

/** Карточка уведомления; `action` — что сделать по клику. */
export interface ToastData {
  kind: 'week' | 'month' | 'break' | 'test';
  title: string;
  body: string;
  action?: 'lastweek' | 'lastmonth';
}
