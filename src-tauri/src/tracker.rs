//! Счётчики текущего дня. Сюда приходят события ввода (сотни в секунду) и тик раз в секунду.
//! Обработчики ввода ничего не считают по часам и не трогают диск: только плюсуют числа,
//! поэтому на производительность это не влияет.

use crate::model::DayStats;
use std::time::{Duration, Instant};

/// Простой дольше этого — человек отошёл, время не идёт.
pub const IDLE_AFTER: Duration = Duration::from_secs(120);
/// Перерыв дольше этого обрывает «сессию за компьютером».
pub const SESSION_BREAK: Duration = Duration::from_secs(300);

pub const RI_LEFT_DOWN: u16 = 0x0001;
pub const RI_RIGHT_DOWN: u16 = 0x0004;
pub const RI_MIDDLE_DOWN: u16 = 0x0010;
pub const RI_BUTTON4_DOWN: u16 = 0x0040;
pub const RI_BUTTON5_DOWN: u16 = 0x0100;
pub const RI_WHEEL: u16 = 0x0400;
pub const RI_HWHEEL: u16 = 0x0800;

/// Что тику нужно знать о текущем моменте.
pub struct Now {
    pub date: String,
    pub hour: usize,
    pub hm: String,
    pub instant: Instant,
}

impl Now {
    pub fn local() -> Self {
        let t = chrono::Local::now();
        Now {
            date: t.format("%Y-%m-%d").to_string(),
            hour: t.format("%H").to_string().parse().unwrap_or(0),
            hm: t.format("%H:%M").to_string(),
            instant: Instant::now(),
        }
    }
}

pub struct Tracker {
    pub day: DayStats,
    pressed: [bool; 256],
    mm_per_count: f64,
    hour: usize,
    hm: String,
    minute: String,
    minute_keys: u64,
    last_input: Option<Instant>,
    session_start: Option<Instant>,
    last_active: Option<Instant>,
    /// Что-то изменилось с последнего сохранения.
    pub dirty: bool,
}

impl Tracker {
    pub fn new(day: DayStats, dpi: u32, now: &Now) -> Self {
        let mut day = day;
        day.normalize();
        Tracker {
            day,
            pressed: [false; 256],
            mm_per_count: 25.4 / dpi.max(100) as f64,
            hour: now.hour,
            hm: now.hm.clone(),
            minute: String::new(),
            minute_keys: 0,
            last_input: None,
            session_start: None,
            last_active: None,
            dirty: false,
        }
    }

    pub fn set_dpi(&mut self, dpi: u32) {
        self.mm_per_count = 25.4 / dpi.max(100) as f64;
    }

    fn touch(&mut self) {
        self.last_input = Some(Instant::now());
        self.dirty = true;
    }

    /// Клавиша: считаем только нажатие отпущенной клавиши, автоповтор при удержании не в счёт.
    pub fn on_key(&mut self, vk: u16, down: bool) {
        if vk == 0 || vk >= 255 {
            return;
        }
        let i = vk as usize;
        if !down {
            self.pressed[i] = false;
            return;
        }
        if self.pressed[i] {
            return;
        }
        self.pressed[i] = true;
        self.day.keys += 1;
        *self.day.key_counts.entry(vk).or_insert(0) += 1;
        self.day.hourly_keys[self.hour] += 1;
        if self.minute != self.hm {
            self.minute = self.hm.clone();
            self.minute_keys = 0;
        }
        self.minute_keys += 1;
        if self.minute_keys > self.day.peak_kpm {
            self.day.peak_kpm = self.minute_keys;
            self.day.peak_kpm_at = self.hm.clone();
        }
        self.touch();
    }

    /// Относительное движение мыши в отсчётах сенсора.
    pub fn on_move(&mut self, dx: i32, dy: i32) {
        if dx == 0 && dy == 0 {
            return;
        }
        let mm = ((dx as f64).hypot(dy as f64)) * self.mm_per_count;
        self.day.mouse_mm += mm;
        self.day.hourly_mouse_mm[self.hour] += mm;
        self.touch();
    }

    /// Флаги кнопок из RAWMOUSE.usButtonFlags и данные колеса.
    pub fn on_buttons(&mut self, flags: u16, wheel: i16) {
        let mut clicks = 0;
        if flags & RI_LEFT_DOWN != 0 {
            self.day.clicks_left += 1;
            clicks += 1;
        }
        if flags & RI_RIGHT_DOWN != 0 {
            self.day.clicks_right += 1;
            clicks += 1;
        }
        if flags & RI_MIDDLE_DOWN != 0 {
            self.day.clicks_middle += 1;
            clicks += 1;
        }
        if flags & (RI_BUTTON4_DOWN | RI_BUTTON5_DOWN) != 0 {
            self.day.clicks_other += 1;
            clicks += 1;
        }
        self.day.hourly_clicks[self.hour] += clicks;
        if flags & (RI_WHEEL | RI_HWHEEL) != 0 {
            self.day.wheel_notches += (wheel as f64).abs() / 120.0;
        }
        if flags != 0 {
            self.touch();
        }
    }

    pub fn idle(&self, at: Instant) -> bool {
        match self.last_input {
            Some(t) => at.saturating_duration_since(t) >= IDLE_AFTER,
            None => true,
        }
    }

    /// Раз в секунду: время за компом, программы, сессии, смена суток.
    /// Возвращает завершившийся день, если наступили новые сутки.
    pub fn tick(&mut self, now: &Now, app: Option<&str>) -> Option<DayStats> {
        let mut finished = None;
        if now.date != self.day.date {
            let old = std::mem::replace(&mut self.day, DayStats::new(&now.date));
            finished = Some(old);
            self.minute.clear();
            self.minute_keys = 0;
            self.session_start = None;
        }
        self.hour = now.hour.min(23);
        self.hm = now.hm.clone();

        if !self.idle(now.instant) {
            self.day.active_seconds += 1;
            self.day.hourly_active[self.hour] += 1;
            if let Some(app) = app {
                *self.day.apps.entry(app.to_string()).or_insert(0) += 1;
            }
            if self.day.first_active.is_empty() {
                self.day.first_active = self.hm.clone();
            }
            self.day.last_active = self.hm.clone();

            let continues = matches!(self.last_active, Some(t) if now.instant.saturating_duration_since(t) <= SESSION_BREAK);
            if !continues || self.session_start.is_none() {
                self.session_start = Some(now.instant);
            }
            self.last_active = Some(now.instant);
            let len = now.instant.saturating_duration_since(self.session_start.unwrap()).as_secs() + 1;
            if len > self.day.longest_session_sec {
                self.day.longest_session_sec = len;
            }
            self.dirty = true;
        }
        finished
    }

    /// Последнее движение ввода — для тестов.
    #[cfg(test)]
    pub fn set_last_input(&mut self, t: Instant) {
        self.last_input = Some(t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now(date: &str, hour: usize, hm: &str, at: Instant) -> Now {
        Now { date: date.into(), hour, hm: hm.into(), instant: at }
    }

    #[test]
    fn keys_ignore_autorepeat() {
        let t0 = Instant::now();
        let mut t = Tracker::new(DayStats::new("2026-11-01"), 800, &now("2026-11-01", 10, "10:00", t0));
        t.on_key(0x41, true);
        t.on_key(0x41, true); // автоповтор
        t.on_key(0x41, true);
        t.on_key(0x41, false);
        t.on_key(0x41, true);
        t.on_key(0x08, true);
        assert_eq!(t.day.keys, 3);
        assert_eq!(t.day.key_counts[&0x41], 2);
        assert_eq!(t.day.key_counts[&0x08], 1);
        assert_eq!(t.day.hourly_keys[10], 3);
        assert_eq!(t.day.peak_kpm, 3);
    }

    #[test]
    fn mouse_distance_and_buttons() {
        let t0 = Instant::now();
        let mut t = Tracker::new(DayStats::new("2026-11-01"), 800, &now("2026-11-01", 0, "00:00", t0));
        t.on_move(800, 0); // один дюйм
        t.on_move(0, 0);
        assert!((t.day.mouse_mm - 25.4).abs() < 1e-9);
        t.on_buttons(RI_LEFT_DOWN | RI_RIGHT_DOWN, 0);
        t.on_buttons(0x0002, 0); // отпускание левой — не клик
        t.on_buttons(RI_WHEEL, -240);
        assert_eq!(t.day.clicks(), 2);
        assert_eq!(t.day.wheel_notches, 2.0);
    }

    #[test]
    fn tick_counts_active_time_sessions_and_rollover() {
        let t0 = Instant::now();
        let mut t = Tracker::new(DayStats::new("2026-11-01"), 800, &now("2026-11-01", 9, "09:00", t0));
        // без ввода — простой
        assert!(t.tick(&now("2026-11-01", 9, "09:00", t0), Some("Chrome")).is_none());
        assert_eq!(t.day.active_seconds, 0);

        t.set_last_input(t0);
        for s in 0..10 {
            t.tick(&now("2026-11-01", 9, "09:00", t0 + Duration::from_secs(s)), Some("Chrome"));
        }
        assert_eq!(t.day.active_seconds, 10);
        assert_eq!(t.day.apps["Chrome"], 10);
        assert_eq!(t.day.longest_session_sec, 10);
        assert_eq!(t.day.first_active, "09:00");

        // перерыв 10 минут: новая сессия, длинная остаётся прежней
        let later = t0 + Duration::from_secs(700);
        t.set_last_input(later);
        t.tick(&now("2026-11-01", 9, "09:11", later), Some("Code"));
        assert_eq!(t.day.longest_session_sec, 10);
        assert_eq!(t.day.last_active, "09:11");

        // полночь
        let old = t.tick(&now("2026-11-02", 0, "00:00", later + Duration::from_secs(1)), None).unwrap();
        assert_eq!(old.date, "2026-11-01");
        assert_eq!(t.day.date, "2026-11-02");
        assert_eq!(t.day.active_seconds, 1);
    }
}
