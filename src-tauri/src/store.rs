//! Хранение на диске: `days/2026-11-01.json` на каждый день и `settings.json`.
//! Пишем через временный файл и переименование, чтобы при выключении питания не получить обрывок JSON.

use crate::model::{DayStats, Settings};
use chrono::{Duration, NaiveDate};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn new(root: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(root.join("days"))?;
        Ok(Store { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn day_path(&self, date: &str) -> PathBuf {
        self.root.join("days").join(format!("{date}.json"))
    }

    fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, path)
    }

    pub fn load_day(&self, date: &str) -> Option<DayStats> {
        let text = fs::read_to_string(self.day_path(date)).ok()?;
        let mut d: DayStats = serde_json::from_str(&text).ok()?;
        d.date = date.to_string();
        d.normalize();
        Some(d)
    }

    pub fn save_day(&self, day: &DayStats) -> io::Result<()> {
        let json = serde_json::to_vec(day).map_err(io::Error::other)?;
        Self::write_atomic(&self.day_path(&day.date), &json)
    }

    /// Все сохранённые дни в диапазоне (включительно). Пропуски — дни, когда компьютер не включали.
    pub fn load_range(&self, from: &str, to: &str) -> Vec<DayStats> {
        let (Ok(mut d), Ok(end)) = (NaiveDate::parse_from_str(from, "%Y-%m-%d"), NaiveDate::parse_from_str(to, "%Y-%m-%d")) else {
            return vec![];
        };
        let mut out = vec![];
        let mut guard = 0;
        while d <= end && guard < 4000 {
            if let Some(day) = self.load_day(&d.format("%Y-%m-%d").to_string()) {
                out.push(day);
            }
            d += Duration::days(1);
            guard += 1;
        }
        out
    }

    /// Самый ранний день с данными — для «за всё время».
    pub fn first_date(&self) -> Option<String> {
        fs::read_dir(self.root.join("days"))
            .ok()?
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .filter_map(|n| n.strip_suffix(".json").map(str::to_string))
            .filter(|n| NaiveDate::parse_from_str(n, "%Y-%m-%d").is_ok())
            .min()
    }

    pub fn load_settings(&self) -> Settings {
        fs::read_to_string(self.root.join("settings.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save_settings(&self, s: &Settings) -> io::Result<()> {
        let json = serde_json::to_vec_pretty(s).map_err(io::Error::other)?;
        Self::write_atomic(&self.root.join("settings.json"), &json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_range() {
        let dir = std::env::temp_dir().join(format!("pcw-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let st = Store::new(dir.clone()).unwrap();
        let mut d = DayStats::new("2026-10-30");
        d.keys = 42;
        d.key_counts.insert(0x41, 40);
        d.apps.insert("Code".into(), 60);
        st.save_day(&d).unwrap();
        st.save_day(&DayStats::new("2026-11-01")).unwrap();
        assert_eq!(st.load_day("2026-10-30").unwrap(), d);
        let r = st.load_range("2026-10-29", "2026-11-01");
        assert_eq!(r.iter().map(|d| d.date.as_str()).collect::<Vec<_>>(), ["2026-10-30", "2026-11-01"]);
        assert_eq!(st.first_date().as_deref(), Some("2026-10-30"));

        let mut s = Settings::default();
        s.dpi = 1600;
        st.save_settings(&s).unwrap();
        assert_eq!(st.load_settings(), s);
        // повреждённый файл — не падаем
        fs::write(dir.join("days").join("2026-10-31.json"), "{oops").unwrap();
        assert!(st.load_day("2026-10-31").is_none());
        let _ = fs::remove_dir_all(&dir);
    }
}
