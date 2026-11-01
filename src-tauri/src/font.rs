//! Рисуем цифры прямо в значке трея, как Afterburner. Свой пиксельный шрифт 3×5:
//! никаких зависимостей, а на значке 16×16 влезает четыре символа.

const GLYPHS: &[(char, [&str; 5])] = &[
    ('0', ["###", "#.#", "#.#", "#.#", "###"]),
    ('1', [".#.", "##.", ".#.", ".#.", "###"]),
    ('2', ["###", "..#", "###", "#..", "###"]),
    ('3', ["###", "..#", ".##", "..#", "###"]),
    ('4', ["#.#", "#.#", "###", "..#", "..#"]),
    ('5', ["###", "#..", "###", "..#", "###"]),
    ('6', ["###", "#..", "###", "#.#", "###"]),
    ('7', ["###", "..#", ".#.", ".#.", ".#."]),
    ('8', ["###", "#.#", "###", "#.#", "###"]),
    ('9', ["###", "#.#", "###", "..#", "###"]),
    ('.', ["...", "...", "...", "...", ".#."]),
    ('k', ["#..", "#.#", "##.", "#.#", "#.#"]),
    ('M', ["#.#", "###", "###", "#.#", "#.#"]),
    ('m', ["...", "...", "###", "###", "#.#"]),
    ('h', ["#..", "#..", "###", "#.#", "#.#"]),
    (':', ["...", ".#.", "...", ".#.", "..."]),
];

fn glyph(c: char) -> Option<&'static [&'static str; 5]> {
    GLYPHS.iter().find(|(g, _)| *g == c).map(|(_, rows)| rows)
}

/// Число в 1–4 символа: 999, 1.2k, 12k, 123k, 1.2M.
pub fn compact(v: f64) -> String {
    let v = v.max(0.0);
    if v < 10.0 && v.fract() != 0.0 {
        return format!("{:.1}", (v * 10.0).floor() / 10.0);
    }
    if v < 1000.0 {
        return format!("{}", v.round() as u64);
    }
    if v < 10_000.0 {
        return trim(format!("{:.1}k", (v / 100.0).floor() / 10.0));
    }
    if v < 1_000_000.0 {
        return format!("{}k", (v / 1000.0).floor() as u64);
    }
    if v < 10_000_000.0 {
        return trim(format!("{:.1}M", (v / 100_000.0).floor() / 10.0));
    }
    format!("{}M", (v / 1_000_000.0).floor() as u64)
}

fn trim(s: String) -> String {
    s.replace(".0k", "k").replace(".0M", "M")
}

/// Километры для значка: 0.4, 3.4, 12, 123.
pub fn compact_km(km: f64) -> String {
    if km < 10.0 {
        format!("{:.1}", (km * 10.0).floor() / 10.0)
    } else {
        compact(km.floor())
    }
}

/// Скруглённая плашка цвета `bg` размером `size×size` (RGBA).
pub fn background(size: u32, bg: [u8; 4]) -> Vec<u8> {
    let s = size.max(8) as i32;
    let mut px = vec![0u8; (s * s * 4) as usize];
    let radius = (s as f32 * 0.22).max(2.0);
    for y in 0..s {
        for x in 0..s {
            let cx = (x as f32 + 0.5).clamp(radius, s as f32 - radius);
            let cy = (y as f32 + 0.5).clamp(radius, s as f32 - radius);
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            let a = (radius - d + 0.5).clamp(0.0, 1.0);
            if a > 0.0 {
                let i = ((y * s + x) * 4) as usize;
                px[i..i + 4].copy_from_slice(&[bg[0], bg[1], bg[2], (bg[3] as f32 * a) as u8]);
            }
        }
    }
    px
}

/// Накладывает текст по маске прозрачности (0–255 на пиксель) поверх плашки.
pub fn compose(mask: &[u8], size: u32, bg: [u8; 4], fg: [u8; 4]) -> Vec<u8> {
    let mut px = background(size, bg);
    for (i, &m) in mask.iter().enumerate().take((size * size) as usize) {
        if m == 0 {
            continue;
        }
        let a = m as f32 / 255.0;
        let p = &mut px[i * 4..i * 4 + 4];
        for c in 0..3 {
            p[c] = (p[c] as f32 * (1.0 - a) + fg[c] as f32 * a).round() as u8;
        }
        p[3] = p[3].max(m);
    }
    px
}

/// RGBA-картинка `size×size`: плашка и текст пиксельным шрифтом. Запасной вариант,
/// если системный шрифт недоступен.
pub fn render(text: &str, size: u32, bg: [u8; 4], fg: [u8; 4]) -> Vec<u8> {
    let s = size.max(8) as i32;
    let mut px = background(size, bg);

    let chars: Vec<char> = text.chars().filter(|c| glyph(*c).is_some()).collect();
    if chars.is_empty() {
        return px;
    }
    // ширина: точка узкая (1 колонка), остальные 3; между символами 1 пиксель
    let widths: Vec<i32> = chars.iter().map(|c| if *c == '.' { 1 } else { 3 }).collect();
    let w1: i32 = widths.iter().sum::<i32>() + widths.len() as i32 - 1;
    let pad = (s / 16).max(1);
    // по ширине — сколько влезает, по высоте вытягиваем до двух раз сильнее: цифры читаются лучше
    let sx = ((s - 2 * pad) / w1).max(1);
    let sy = ((s - 2 * pad) / 5).min(sx * 2).max(1);
    let tw = w1 * sx;
    let th = 5 * sy;
    let mut x0 = (s - tw) / 2;
    let y0 = (s - th) / 2;

    for (c, w) in chars.iter().zip(&widths) {
        let rows = glyph(*c).unwrap();
        let col0 = if *c == '.' { 1 } else { 0 };
        for (ry, row) in rows.iter().enumerate() {
            for rx in 0..*w {
                if row.as_bytes()[(col0 + rx) as usize] != b'#' {
                    continue;
                }
                for dy in 0..sy {
                    for dx in 0..sx {
                        let x = x0 + rx * sx + dx;
                        let y = y0 + ry as i32 * sy + dy;
                        if x >= 0 && y >= 0 && x < s && y < s {
                            let i = ((y * s + x) * 4) as usize;
                            px[i..i + 4].copy_from_slice(&fg);
                        }
                    }
                }
            }
        }
        x0 += (w + 1) * sx;
    }
    px
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_numbers() {
        assert_eq!(compact(0.0), "0");
        assert_eq!(compact(999.0), "999");
        assert_eq!(compact(1000.0), "1k");
        assert_eq!(compact(1250.0), "1.2k");
        assert_eq!(compact(12_345.0), "12k");
        assert_eq!(compact(123_456.0), "123k");
        assert_eq!(compact(1_234_567.0), "1.2M");
        assert_eq!(compact(3.45), "3.4");
        assert_eq!(compact_km(0.37), "0.3");
        assert_eq!(compact_km(12.9), "12");
        for v in [0.0, 5.5, 999.0, 9999.0, 99_999.0, 999_999.0, 9_999_999.0, 99_999_999.0] {
            assert!(compact(v).len() <= 4, "{v} -> {}", compact(v));
        }
    }

    #[test]
    fn compose_blends_mask() {
        let mut mask = vec![0u8; 16 * 16];
        mask[8 * 16 + 8] = 255;
        let px = compose(&mask, 16, [255, 255, 0, 255], [0, 0, 0, 255]);
        let i = (8 * 16 + 8) * 4;
        assert_eq!(&px[i..i + 4], &[0, 0, 0, 255]);
        assert_eq!(&px[(8 * 16 + 2) * 4..(8 * 16 + 2) * 4 + 4], &[255, 255, 0, 255]);
    }

    #[test]
    fn render_fits_and_draws() {
        for size in [16, 20, 24, 32] {
            let px = render("12k", size, [255, 227, 61, 255], [29, 20, 66, 255]);
            assert_eq!(px.len(), (size * size * 4) as usize);
            let ink = px.chunks(4).filter(|p| p == &[29, 20, 66, 255]).count();
            assert!(ink > 10, "size {size}: text not drawn");
            // угол прозрачный
            assert_eq!(px[3], 0);
        }
        let px = render("1.2M", 16, [0, 0, 0, 255], [255, 255, 255, 255]);
        assert!(px.chunks(4).any(|p| p == [255, 255, 255, 255]));
    }
}
