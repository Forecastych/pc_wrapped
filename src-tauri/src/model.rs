//! Данные, которые храним на диске и отдаём в интерфейс. Один файл = один день.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct DayStats {
    /// Локальная дата, `2026-11-01`.
    pub date: String,
    /// Нажатия клавиш (только переход «отпущена → нажата», автоповтор не считается).
    pub keys: u64,
    /// Нажатия по виртуальным кодам клавиш Windows. Только счётчики: что именно печатали, не хранится.
    pub key_counts: BTreeMap<u16, u64>,
    pub clicks_left: u64,
    pub clicks_right: u64,
    pub clicks_middle: u64,
    pub clicks_other: u64,
    /// Щелчки колеса (один «щелчок» = 120 единиц Windows).
    pub wheel_notches: f64,
    /// Путь мышки по столу в миллиметрах (по сырым отсчётам сенсора и DPI из настроек).
    pub mouse_mm: f64,
    /// Время за компьютером без простоя дольше двух минут.
    pub active_seconds: u64,
    pub hourly_keys: Vec<u64>,
    pub hourly_clicks: Vec<u64>,
    pub hourly_mouse_mm: Vec<f64>,
    pub hourly_active: Vec<u64>,
    /// Секунды в активном окне по программам.
    pub apps: BTreeMap<String, u64>,
    /// Больше всего нажатий за одну минуту и когда это было (`HH:MM`).
    pub peak_kpm: u64,
    pub peak_kpm_at: String,
    pub first_active: String,
    pub last_active: String,
    pub longest_session_sec: u64,
}

impl DayStats {
    pub fn new(date: &str) -> Self {
        let mut d = DayStats { date: date.to_string(), ..Default::default() };
        d.normalize();
        d
    }

    /// Старые или повреждённые файлы могут прийти с короткими массивами — дотягиваем до 24 часов.
    pub fn normalize(&mut self) {
        self.hourly_keys.resize(24, 0);
        self.hourly_clicks.resize(24, 0);
        self.hourly_mouse_mm.resize(24, 0.0);
        self.hourly_active.resize(24, 0);
    }

    pub fn clicks(&self) -> u64 {
        self.clicks_left + self.clicks_right + self.clicks_middle + self.clicks_other
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// DPI мышки: нужен, чтобы перевести отсчёты сенсора в сантиметры. Обычно 800–1600.
    pub dpi: u32,
    /// Какие счётчики показывать значками в трее: keys, clicks, mouse, scroll, active.
    pub tray_metrics: Vec<String>,
    /// Какие счётчики показывать в мини-окне.
    pub mini_metrics: Vec<String>,
    /// Мини-окно пропускает клики сквозь себя.
    pub mini_clickthrough: bool,
    /// Мини-окно открыто (восстанавливаем при запуске).
    pub mini_open: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            dpi: 800,
            tray_metrics: vec!["keys".into(), "mouse".into()],
            mini_metrics: vec!["keys".into(), "clicks".into(), "mouse".into(), "scroll".into()],
            mini_clickthrough: false,
            mini_open: false,
        }
    }
}

pub const METRICS: [&str; 5] = ["keys", "clicks", "mouse", "scroll", "active"];
