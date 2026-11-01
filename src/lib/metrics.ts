import type { Metric } from './types';
import { km, scrollMeters } from './aggregate';
import { hours, kmText, num } from './format';
import { activeFun, clicksFun, keysFun, mouseFun, scrollFun } from './fun';

export interface Totals {
  keys: number;
  clicks: number;
  mouseMm: number;
  wheelNotches: number;
  activeSeconds: number;
}

export interface MetricDef {
  key: Metric;
  title: string;
  color: string;
  value: (t: Totals) => string;
  unit: (t: Totals) => string;
  fun: (t: Totals) => string;
}

export const METRICS: MetricDef[] = [
  {
    key: 'keys',
    title: 'Нажатий',
    color: 'var(--lemon)',
    value: (t) => num(t.keys),
    unit: () => '',
    fun: (t) => keysFun(t.keys),
  },
  {
    key: 'mouse',
    title: 'Мышкой',
    color: 'var(--pink)',
    value: (t) => kmText(km(t.mouseMm)),
    unit: () => 'км',
    fun: (t) => mouseFun(t.mouseMm / 1000),
  },
  {
    key: 'scroll',
    title: 'Колесом',
    color: 'var(--sky)',
    value: (t) => num(scrollMeters(t.wheelNotches)),
    unit: () => 'м',
    fun: (t) => scrollFun(scrollMeters(t.wheelNotches)),
  },
  {
    key: 'clicks',
    title: 'Кликов',
    color: 'var(--mint)',
    value: (t) => num(t.clicks),
    unit: () => '',
    fun: (t) => clicksFun(t.clicks),
  },
  {
    key: 'active',
    title: 'За компом',
    color: 'var(--tangerine)',
    value: (t) => hours(t.activeSeconds),
    unit: () => '',
    fun: (t) => activeFun(t.activeSeconds),
  },
];

export const metricDef = (k: Metric) => METRICS.find((m) => m.key === k)!;
