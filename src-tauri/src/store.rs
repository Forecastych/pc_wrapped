//! Хранение на диске: `days/2026-11-01.json` на каждый день и `settings.json`.
//! Пишем через временный файл и переименование, чтобы при выключении питания не получить обрывок JSON.
//!
//! Папку с данными человек выбирает при первом запуске. В %APPDATA% остаётся только `location.json`
//! с путём к ней.

use crate::model::{DayStats, Settings};
use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

pub struct Store {
    /// Папку можно сменить на ходу (настройки → «Перенести»), поэтому она за замком.
    root: RwLock<PathBuf>,
}

impl Store {
    pub fn new(root: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(root.join("days"))?;
        Ok(Store { root: RwLock::new(root) })
    }

    pub fn root(&self) -> PathBuf {
        self.root.read().unwrap().clone()
    }

    fn day_path(root: &Path, date: &str) -> PathBuf {
        root.join("days").join(format!("{date}.json"))
    }

    /// Переносит дни и настройки в `to` и дальше пишет туда. Запись на время переноса ждёт
    /// на замке, так что ни один день не уедет в старую папку.
    pub fn relocate(&self, to: &Path) -> io::Result<()> {
        let mut root = self.root.write().unwrap();
        move_data(&root, to)?;
        *root = to.to_path_buf();
        Ok(())
    }

    fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, path)
    }

    pub fn load_day(&self, date: &str) -> Option<DayStats> {
        let text = fs::read_to_string(Self::day_path(&self.root(), date)).ok()?;
        let mut d: DayStats = serde_json::from_str(&text).ok()?;
        d.date = date.to_string();
        d.normalize();
        Some(d)
    }

    pub fn save_day(&self, day: &DayStats) -> io::Result<()> {
        let json = serde_json::to_vec(day).map_err(io::Error::other)?;
        let root = self.root.read().unwrap();
        Self::write_atomic(&Self::day_path(&root, &day.date), &json)
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
        fs::read_dir(self.root().join("days"))
            .ok()?
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .filter_map(|n| n.strip_suffix(".json").map(str::to_string))
            .filter(|n| NaiveDate::parse_from_str(n, "%Y-%m-%d").is_ok())
            .min()
    }

    pub fn load_settings(&self) -> Settings {
        fs::read_to_string(self.root().join("settings.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save_settings(&self, s: &Settings) -> io::Result<()> {
        let json = serde_json::to_vec_pretty(s).map_err(io::Error::other)?;
        let root = self.root.read().unwrap();
        Self::write_atomic(&root.join("settings.json"), &json)
    }
}

const LOCATION: &str = "location.json";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Location {
    data_dir: PathBuf,
}

/// Выбранная папка с данными; `None` — ещё не выбирали.
pub fn read_location(app_dir: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(app_dir.join(LOCATION)).ok()?;
    let loc: Location = serde_json::from_str(&text).ok()?;
    loc.data_dir.is_absolute().then_some(loc.data_dir)
}

pub fn write_location(app_dir: &Path, data_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(app_dir)?;
    let json = serde_json::to_vec_pretty(&Location { data_dir: data_dir.to_path_buf() }).map_err(io::Error::other)?;
    Store::write_atomic(&app_dir.join(LOCATION), &json)
}

/// Выбрали корень диска или обычную папку — данные кладём в подпапку `pc_wrapped`, чтобы не мусорить.
/// Если папка так и называется или в ней уже лежат данные pc_wrapped — берём её как есть.
pub fn data_dir_for(picked: &Path) -> PathBuf {
    let named = picked.file_name().is_some_and(|n| n.eq_ignore_ascii_case("pc_wrapped"));
    if named || picked.join("settings.json").is_file() || picked.join("days").is_dir() {
        picked.to_path_buf()
    } else {
        picked.join("pc_wrapped")
    }
}

/// Копирует дни и настройки из `from` в `to`, сверяет размеры и только потом удаляет старое.
/// Если в `to` уже есть данные pc_wrapped (переустановка, перенос с другого компьютера), из двух
/// файлов за один день остаётся тот, где больше времени за компом.
fn move_data(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to.join("days"))?;
    if let (Ok(a), Ok(b)) = (fs::canonicalize(from), fs::canonicalize(to)) {
        if a == b {
            return Ok(());
        }
    }
    let mut moved = vec![];
    if let Ok(entries) = fs::read_dir(from.join("days")) {
        for e in entries {
            let src = e?.path();
            if src.extension().is_none_or(|x| x != "json") {
                continue;
            }
            let dst = to.join("days").join(src.file_name().unwrap_or_default());
            if !dst.exists() || active_secs(&src) > active_secs(&dst) {
                copy_checked(&src, &dst)?;
            }
            moved.push(src);
        }
    }
    let settings = from.join("settings.json");
    if settings.is_file() {
        copy_checked(&settings, &to.join("settings.json"))?;
        moved.push(settings);
    }
    // всё на новом месте — теперь старое можно убрать; папки удаляются, только если опустели
    for p in &moved {
        let _ = fs::remove_file(p);
    }
    let _ = fs::remove_dir(from.join("days"));
    let _ = fs::remove_dir(from);
    Ok(())
}

fn copy_checked(src: &Path, dst: &Path) -> io::Result<()> {
    let tmp = dst.with_extension("tmp");
    let n = fs::copy(src, &tmp)?;
    if n != fs::metadata(src)?.len() {
        let _ = fs::remove_file(&tmp);
        return Err(io::Error::other(format!("{} скопировался не целиком", src.display())));
    }
    fs::rename(&tmp, dst)
}

fn active_secs(p: &Path) -> u64 {
    fs::read_to_string(p)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v.get("activeSeconds")?.as_u64())
        .unwrap_or(0)
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

    #[test]
    fn relocate_merges_and_cleans_up() {
        let base = std::env::temp_dir().join(format!("pcw-move-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let (old, new) = (base.join("old"), base.join("new"));
        let day = |date: &str, secs: u64| {
            let mut d = DayStats::new(date);
            d.active_seconds = secs;
            d
        };
        // в новой папке уже есть данные: 31-е там полнее, 30-е — беднее
        let there = Store::new(new.clone()).unwrap();
        there.save_day(&day("2026-10-30", 10)).unwrap();
        there.save_day(&day("2026-10-31", 900)).unwrap();

        let st = Store::new(old.clone()).unwrap();
        st.save_day(&day("2026-10-29", 100)).unwrap();
        st.save_day(&day("2026-10-30", 500)).unwrap();
        st.save_day(&day("2026-10-31", 50)).unwrap();
        st.save_settings(&Settings { dpi: 1600, ..Settings::default() }).unwrap();

        st.relocate(&new).unwrap();
        assert_eq!(st.root(), new);
        let secs: Vec<u64> = st.load_range("2026-10-29", "2026-10-31").iter().map(|d| d.active_seconds).collect();
        assert_eq!(secs, [100, 500, 900]);
        assert_eq!(st.load_settings().dpi, 1600);
        assert!(!old.exists(), "старая папка должна исчезнуть, раз в ней ничего не осталось");
        // запись идёт уже в новое место
        st.save_day(&day("2026-11-01", 1)).unwrap();
        assert!(new.join("days").join("2026-11-01.json").is_file());
        // перенос в ту же папку ничего не ломает
        st.relocate(&new).unwrap();
        assert_eq!(st.load_range("2026-10-29", "2026-11-01").len(), 4);
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn location_and_subfolder() {
        let base = std::env::temp_dir().join(format!("pcw-loc-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        assert_eq!(read_location(&base), None);
        let target = base.join("Data").join("pc_wrapped");
        write_location(&base, &target).unwrap();
        assert_eq!(read_location(&base), Some(target.clone()));

        assert_eq!(data_dir_for(&base.join("Data")), target);
        assert_eq!(data_dir_for(&target), target);
        assert_eq!(data_dir_for(Path::new("D:\\")), Path::new("D:\\pc_wrapped"));
        // папка со старыми данными под другим именем — берём как есть
        let other = base.join("stats");
        fs::create_dir_all(other.join("days")).unwrap();
        assert_eq!(data_dir_for(&other), other);
        let _ = fs::remove_dir_all(&base);
    }
}
