//! Карточки-уведомления: что в них писать и когда пора показывать итоги недели и месяца.

use crate::model::{DayStats, Settings};
use chrono::{Datelike, Duration, NaiveDate};
use serde::Serialize;

/// То, что уходит в окно-карточку.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Toast {
    pub kind: &'static str,
    pub title: String,
    pub body: String,
    /// Какие итоги открыть по клику (`lastweek` / `lastmonth`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Report {
    Week,
    Month,
}

const MONTHS: [&str; 12] = ["январь", "февраль", "март", "апрель", "май", "июнь", "июль", "август", "сентябрь", "октябрь", "ноябрь", "декабрь"];

/// ISO-неделя, `2026-W45`.
pub fn week_key(d: NaiveDate) -> String {
    let w = d.iso_week();
    format!("{}-W{:02}", w.year(), w.week())
}

pub fn month_key(d: NaiveDate) -> String {
    format!("{:04}-{:02}", d.year(), d.month())
}

/// Прошлая календарная неделя, понедельник–воскресенье.
pub fn last_week(today: NaiveDate) -> (NaiveDate, NaiveDate) {
    let monday = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    (monday - Duration::days(7), monday - Duration::days(1))
}

/// Прошлый календарный месяц целиком.
pub fn last_month(today: NaiveDate) -> (NaiveDate, NaiveDate) {
    let end = today.with_day(1).unwrap() - Duration::days(1);
    (end.with_day(1).unwrap(), end)
}

/// Какие итоги пора показать: сначала неделя, потом месяц. Ключ — что записать в настройки после показа.
pub fn due(today: NaiveDate, s: &Settings) -> Option<(Report, String, NaiveDate, NaiveDate)> {
    if s.weekly_report {
        let key = week_key(today);
        if s.last_weekly != key {
            let (from, to) = last_week(today);
            return Some((Report::Week, key, from, to));
        }
    }
    if s.monthly_report {
        let key = month_key(today);
        if s.last_monthly != key {
            let (from, to) = last_month(today);
            return Some((Report::Month, key, from, to));
        }
    }
    None
}

fn plural<'a>(n: u64, one: &'a str, few: &'a str, many: &'a str) -> &'a str {
    match (n % 10, n % 100) {
        (1, x) if x != 11 => one,
        (2..=4, x) if !(12..=14).contains(&x) => few,
        _ => many,
    }
}

fn spaced(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('\u{a0}');
        }
        out.push(c);
    }
    out
}

fn duration(sec: u64) -> String {
    match (sec / 3600, sec % 3600 / 60) {
        (0, m) => format!("{m}\u{a0}мин"),
        (h, 0) => format!("{h}\u{a0}ч"),
        (h, m) => format!("{h}\u{a0}ч {m}\u{a0}мин"),
    }
}

/// Карточка с итогами. `None`, если за период за компом не сидели — показывать нечего.
pub fn report(kind: Report, from: NaiveDate, days: &[DayStats]) -> Option<Toast> {
    let active: u64 = days.iter().map(|d| d.active_seconds).sum();
    if active == 0 {
        return None;
    }
    let keys: u64 = days.iter().map(|d| d.keys).sum();
    let km = days.iter().map(|d| d.mouse_mm).sum::<f64>() / 1_000_000.0;
    let body = format!(
        "{} за компом · {} {} · {} км мышкой",
        duration(active),
        spaced(keys),
        plural(keys, "нажатие", "нажатия", "нажатий"),
        format!("{km:.1}").replace('.', ",")
    );
    Some(match kind {
        Report::Week => Toast { kind: "week", title: "Твоя неделя за компом".into(), body, action: Some("lastweek") },
        Report::Month => Toast {
            kind: "month",
            title: format!("Твой {} за компом", MONTHS[from.month0() as usize]),
            body,
            action: Some("lastmonth"),
        },
    })
}

pub fn break_toast(session_sec: u64) -> Toast {
    Toast {
        kind: "break",
        title: "Пора размяться".into(),
        body: format!("Ты за компом без перерыва уже {}. Встань, пройдись, посмотри в окно — счётчики подождут.", duration(session_sec / 60 * 60)),
        action: None,
    }
}

pub fn sample() -> Toast {
    Toast {
        kind: "test",
        title: "Вот так выглядит уведомление".into(),
        body: "Появляется справа снизу на четыре секунды и не забирает фокус. Поверх игр и видео на весь экран не показывается.".into(),
        action: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn calendar_periods() {
        assert_eq!(week_key(d("2026-11-02")), "2026-W45");
        assert_eq!(week_key(d("2026-11-01")), "2026-W44");
        // ISO-неделя первых дней января может принадлежать прошлому году
        assert_eq!(week_key(d("2027-01-01")), "2026-W53");
        assert_eq!(last_week(d("2026-11-04")), (d("2026-10-26"), d("2026-11-01")));
        assert_eq!(last_week(d("2026-11-02")), (d("2026-10-26"), d("2026-11-01")));
        assert_eq!(last_month(d("2026-11-01")), (d("2026-10-01"), d("2026-10-31")));
        assert_eq!(last_month(d("2026-01-15")), (d("2025-12-01"), d("2025-12-31")));
        assert_eq!(last_month(d("2026-03-31")), (d("2026-02-01"), d("2026-02-28")));
    }

    #[test]
    fn due_goes_week_then_month_and_respects_switches() {
        let mut s = Settings::default();
        let today = d("2026-11-02");
        assert_eq!(due(today, &s).unwrap().0, Report::Week);
        s.last_weekly = week_key(today);
        let (kind, key, from, to) = due(today, &s).unwrap();
        assert_eq!((kind, key.as_str(), from, to), (Report::Month, "2026-11", d("2026-10-01"), d("2026-10-31")));
        s.last_monthly = key;
        assert!(due(today, &s).is_none());
        // новая неделя, но итоги недели выключены
        s.weekly_report = false;
        assert!(due(d("2026-11-09"), &s).is_none());
    }

    #[test]
    fn report_text() {
        let mut a = DayStats::new("2026-10-05");
        a.active_seconds = 3 * 3600 + 25 * 60;
        a.keys = 12_001;
        a.mouse_mm = 1_300_000.0;
        let mut b = DayStats::new("2026-10-06");
        b.active_seconds = 3600;
        b.keys = 20_000;
        let t = report(Report::Month, d("2026-10-01"), &[a, b]).unwrap();
        assert_eq!(t.title, "Твой октябрь за компом");
        assert_eq!(t.body, "4\u{a0}ч 25\u{a0}мин за компом · 32\u{a0}001 нажатие · 1,3 км мышкой");
        assert_eq!(t.action, Some("lastmonth"));
        assert!(report(Report::Week, d("2026-10-26"), &[DayStats::new("2026-10-26")]).is_none());
        assert!(break_toast(3720).body.contains("1\u{a0}ч 2\u{a0}мин"));
    }
}
