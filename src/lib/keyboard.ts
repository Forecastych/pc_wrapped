// Раскладка для тепловой карты: виртуальные коды Windows и подписи.
// Ширина клавиши в «юнитах» (1 = обычная буквенная клавиша).

export interface Key {
  vk: number;
  label: string;
  w?: number;
}

const k = (vk: number, label: string, w = 1): Key => ({ vk, label, w });
const letters = (s: string) => [...s].map((c) => k(c.charCodeAt(0), c));

export const KEYBOARD: Key[][] = [
  [k(0x1b, 'Esc'), ...Array.from({ length: 12 }, (_, i) => k(0x70 + i, `F${i + 1}`))],
  [k(0xc0, '`'), ...[...'1234567890'].map((c) => k(c.charCodeAt(0), c)), k(0xbd, '-'), k(0xbb, '='), k(0x08, 'Backspace', 2)],
  [k(0x09, 'Tab', 1.5), ...letters('QWERTYUIOP'), k(0xdb, '['), k(0xdd, ']'), k(0xdc, '\\', 1.5)],
  [k(0x14, 'Caps', 1.75), ...letters('ASDFGHJKL'), k(0xba, ';'), k(0xde, "'"), k(0x0d, 'Enter', 2.25)],
  [k(0x10, 'Shift', 2.25), ...letters('ZXCVBNM'), k(0xbc, ','), k(0xbe, '.'), k(0xbf, '/'), k(0x10, 'Shift', 2.75)],
  [k(0x11, 'Ctrl', 1.5), k(0x5b, 'Win', 1.25), k(0x12, 'Alt', 1.25), k(0x20, 'Пробел', 6.25), k(0x12, 'Alt', 1.25), k(0x11, 'Ctrl', 1.5)],
];

const NAMES: Record<number, string> = {
  0x08: 'Backspace',
  0x09: 'Tab',
  0x0d: 'Enter',
  0x10: 'Shift',
  0x11: 'Ctrl',
  0x12: 'Alt',
  0x14: 'Caps Lock',
  0x1b: 'Esc',
  0x20: 'Пробел',
  0x25: '←',
  0x26: '↑',
  0x27: '→',
  0x28: '↓',
  0x2e: 'Delete',
  0x5b: 'Win',
  0xa0: 'Shift',
  0xa1: 'Shift',
  0xa2: 'Ctrl',
  0xa3: 'Ctrl',
};

/** Человеческое название клавиши по коду. */
export function keyName(vk: number): string {
  if (NAMES[vk]) return NAMES[vk];
  if ((vk >= 0x30 && vk <= 0x39) || (vk >= 0x41 && vk <= 0x5a)) return String.fromCharCode(vk);
  if (vk >= 0x70 && vk <= 0x87) return `F${vk - 0x6f}`;
  for (const row of KEYBOARD) for (const key of row) if (key.vk === vk) return key.label;
  return `#${vk}`;
}
