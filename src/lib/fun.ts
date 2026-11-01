// Перевод сухих чисел в понятные сравнения: «это три марафона».
import { plural } from './format';

const fmt1 = (x: number) => (x >= 10 ? Math.round(x).toLocaleString('ru-RU') : x.toFixed(1).replace('.', ',').replace(',0', ''));

function times(x: number, one: string, few: string, many: string) {
  const n = x >= 10 ? Math.round(x) : Math.round(x * 10) / 10;
  if (!Number.isInteger(n)) return `${fmt1(n)} ${few}`; // «1,5 марафона»
  return `${n.toLocaleString('ru-RU')} ${plural(n, one, few, many)}`;
}

/** Путь мышки (метры) → сравнение. */
export function mouseFun(m: number): string {
  if (m < 1) return 'мышка пока отдыхает';
  if (m < 105) return `это ${times(m / 12, 'автобус', 'автобуса', 'автобусов')} в длину`;
  if (m < 1000) return `это ${times(m / 105, 'футбольное поле', 'футбольных поля', 'футбольных полей')}`;
  if (m < 42_195) return `это ${times(m / 400, 'круг', 'круга', 'кругов')} по стадиону`;
  if (m < 109_000) return `это ${times(m / 42_195, 'марафон', 'марафона', 'марафонов')}`;
  return `это ${times(m / 109_000, 'круг', 'круга', 'кругов')} по МКАДу`;
}

/** Прокрутка колесом (метры) → высота. */
export function scrollFun(m: number): string {
  if (m < 3) return 'колесо почти не крутили';
  if (m < 100) return `как подняться на ${times(m / 3, 'этаж', 'этажа', 'этажей')}`;
  if (m < 540) return `это ${times(m / 330, 'Эйфелева башня', 'Эйфелевы башни', 'Эйфелевых башен')}`;
  if (m < 8849) return `это ${times(m / 828, 'Бурдж-Халифа', 'Бурдж-Халифы', 'Бурдж-Халиф')}`;
  return `это ${times(m / 8849, 'Эверест', 'Эвереста', 'Эверестов')}`;
}

/** Нажатия → страницы текста (≈1800 знаков на странице). */
export function keysFun(keys: number): string {
  if (keys < 100) return 'клавиатура скучает';
  const pages = keys / 1800;
  if (pages < 1000) return `хватило бы на ${times(pages, 'страницу', 'страницы', 'страниц')} текста`;
  return `это ${times(keys / 3_000_000, '«Война и мир»', '«Войны и мира»', '«Войн и миров»')}`;
}

/** Клики → шаги по 75 см. */
export function clicksFun(clicks: number): string {
  if (clicks < 10) return 'мышь почти не щёлкала';
  const m = clicks * 0.75;
  return m < 1000 ? `если клик — это шаг, это ${Math.round(m)} м пешком` : `если клик — это шаг, это ${fmt1(m / 1000)} км пешком`;
}

/** Часы за компом → фильмы по 2 часа. */
export function activeFun(sec: number): string {
  const films = sec / 7200;
  if (films < 1) return 'меньше одного фильма';
  return `это ${times(films, 'полнометражный фильм', 'полнометражных фильма', 'полнометражных фильмов')}`;
}
