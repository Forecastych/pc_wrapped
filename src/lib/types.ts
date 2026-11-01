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
}

export type PeriodKey = 'today' | 'week' | 'month' | 'year' | 'all';
