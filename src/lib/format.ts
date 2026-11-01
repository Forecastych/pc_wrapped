export function plural(n: number, one: string, few: string, many: string) {
  const a = Math.abs(n) % 100,
    b = a % 10;
  if (a > 10 && a < 20) return many;
  if (b > 1 && b < 5) return few;
  if (b === 1) return one;
  return many;
}

export const num = (n: number) => Math.round(n).toLocaleString('ru-RU');

export function count(n: number, one: string, few: string, many: string) {
  return `${num(n)} ${plural(Math.round(n), one, few, many)}`;
}

/** Километры: 0,37 / 3,4 / 12 / 1 234. */
export function kmText(km: number) {
  if (km < 1) return km.toFixed(2).replace('.', ',');
  if (km < 10) return km.toFixed(1).replace('.', ',');
  return num(km);
}

export function hours(sec: number): string {
  if (sec > 0 && sec < 60) return 'меньше минуты';
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  if (h === 0) return `${m} мин`;
  return m ? `${h} ч ${m} мин` : `${h} ч`;
}

export const pct = (x: number) => `${Math.round(x * 100)}%`;
