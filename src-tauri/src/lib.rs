mod font;
mod model;
mod notify;
mod store;
mod tracker;
#[cfg(windows)]
mod win;

use model::{DayStats, Settings, METRICS};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use store::Store;
use tauri::image::Image;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder};
use tracker::{Now, Tracker};

struct AppState {
    tracker: Arc<Mutex<Tracker>>,
    store: Arc<Store>,
    /// %APPDATA%\io.github.pcwrapped: тут лежит `location.json`, а пока папку не выбрали — и сами данные.
    app_dir: PathBuf,
    /// Человек уже выбрал, где хранить данные. Пока нет, главное окно спрашивает.
    data_chosen: AtomicBool,
    settings: Mutex<Settings>,
    /// Что сейчас нарисовано в каждом значке трея, чтобы не перерисовывать то же самое.
    tray_text: Mutex<HashMap<String, String>>,
    /// Итоги, которые открыть, когда главное окно загрузится (клик по уведомлению при закрытом окне).
    pending_story: Mutex<Option<String>>,
    /// Итоги из последнего уведомления: щелчок по значку в трее вскоре после него открывает их.
    toast_story: Mutex<Option<(String, Instant)>>,
}

// ---------- Команды для интерфейса ----------

#[tauri::command]
fn get_today(state: State<'_, AppState>) -> DayStats {
    state.tracker.lock().unwrap().day.clone()
}

#[tauri::command]
fn get_days(state: State<'_, AppState>, from: String, to: String) -> Vec<DayStats> {
    let today = state.tracker.lock().unwrap().day.clone();
    let mut days: Vec<DayStats> = state.store.load_range(&from, &to).into_iter().filter(|d| d.date != today.date).collect();
    if today.date >= from && today.date <= to {
        days.push(today);
    }
    days.sort_by(|a, b| a.date.cmp(&b.date));
    days
}

#[tauri::command]
fn first_date(state: State<'_, AppState>) -> Option<String> {
    let today = state.tracker.lock().unwrap().day.date.clone();
    Some(state.store.first_date().map_or(today.clone(), |d| d.min(today)))
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn set_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    let mut s = settings;
    s.dpi = s.dpi.clamp(100, 32000);
    s.tray_metrics.retain(|m| METRICS.contains(&m.as_str()));
    s.mini_metrics.retain(|m| METRICS.contains(&m.as_str()));
    s.break_minutes = s.break_minutes.clamp(15, 240);
    {
        // отметки о показанных итогах ведёт только Rust: окно настроек присылает то, что видело при открытии
        let cur = state.settings.lock().unwrap();
        s.last_weekly = cur.last_weekly.clone();
        s.last_monthly = cur.last_monthly.clone();
    }
    state.store.save_settings(&s).map_err(|e| e.to_string())?;
    state.tracker.lock().unwrap().set_dpi(s.dpi);
    if let Some(w) = app.get_webview_window("mini") {
        let _ = w.set_ignore_cursor_events(s.mini_clickthrough);
    }
    *state.settings.lock().unwrap() = s.clone();
    state.tray_text.lock().unwrap().clear();
    let day = state.tracker.lock().unwrap().day.clone();
    sync_tray(&app, &s, &day);
    let _ = app.emit("settings", &s);
    Ok(())
}

// Окна создаём только из async-команд: синхронная команда выполняется в главном потоке,
// и WebviewWindowBuilder::build() в ней на Windows зависает намертво.
#[tauri::command]
async fn toggle_mini(app: AppHandle) {
    toggle_mini_window(&app);
}

#[tauri::command]
async fn open_main(app: AppHandle) {
    show_main(&app);
}

#[tauri::command]
async fn notify_test(app: AppHandle) {
    show_toast(&app, &notify::sample());
}

/// Итоги слайдами (`lastweek` / `lastmonth`) по щелчку в трее после уведомления.
fn open_story(app: &AppHandle, action: String) {
    if let Some(w) = app.get_webview_window("toast") {
        let _ = w.close();
    }
    if app.get_webview_window("main").is_some() {
        let _ = app.emit_to("main", "open-story", &action);
    } else {
        *app.state::<AppState>().pending_story.lock().unwrap() = Some(action);
    }
    show_main(app);
}

#[tauri::command]
fn take_pending_story(state: State<'_, AppState>) -> Option<String> {
    state.pending_story.lock().unwrap().take()
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DataDir {
    path: String,
    chosen: bool,
    /// Что предложить при первом запуске: `pc_wrapped` на несистемном диске, если такой есть.
    suggested: String,
    system_only: bool,
}

#[tauri::command]
fn get_data_dir(state: State<'_, AppState>) -> DataDir {
    #[cfg(windows)]
    let drive = win::data_drives().into_iter().next();
    #[cfg(not(windows))]
    let drive: Option<String> = None;
    DataDir {
        path: state.store.root().display().to_string(),
        chosen: state.data_chosen.load(Ordering::Relaxed),
        system_only: drive.is_none(),
        suggested: drive.map_or_else(|| state.app_dir.clone(), |d| PathBuf::from(d).join("pc_wrapped")).display().to_string(),
    }
}

/// Системный диалог выбора папки. Async — потому что диалог блокирующий и в главном потоке ему нельзя.
#[tauri::command]
async fn pick_folder(app: AppHandle, start: Option<String>) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let mut dialog = app.dialog().file().set_title("Где хранить статистику pc_wrapped");
    // диалог открываем у ближайшей существующей папки: предложенной D:\pc_wrapped может ещё не быть
    if let Some(dir) = start.as_deref().map(Path::new).and_then(|p| p.ancestors().find(|a| a.is_dir())) {
        dialog = dialog.set_directory(dir);
    }
    if let Some(w) = app.get_webview_window("main") {
        dialog = dialog.set_parent(&w);
    }
    let picked = dialog.blocking_pick_folder()?.into_path().ok()?;
    Some(store::data_dir_for(&picked).display().to_string())
}

/// Переносит данные в новую папку и запоминает её.
#[tauri::command]
async fn set_data_dir(app: AppHandle, path: String) -> Result<String, String> {
    let state = app.state::<AppState>();
    let target = PathBuf::from(path.trim());
    if !target.is_absolute() {
        return Err("нужен полный путь, например D:\\pc_wrapped".into());
    }
    state.store.relocate(&target).map_err(|e| format!("не получилось перенести данные: {e}"))?;
    let settings = state.settings.lock().unwrap().clone();
    state.store.save_settings(&settings).map_err(|e| e.to_string())?;
    save_today(&app);
    store::write_location(&state.app_dir, &target).map_err(|e| format!("данные перенесены, но путь не запомнился: {e}"))?;
    state.data_chosen.store(true, Ordering::Relaxed);
    Ok(target.display().to_string())
}

/// Сохраняет картинку или таблицу в `exports` рядом с данными и показывает файл в проводнике.
/// Байты приходят сырым телом запроса (без JSON-массива на мегабайты), имя — в заголовке `x-name`.
#[tauri::command]
fn save_export(state: State<'_, AppState>, request: tauri::ipc::Request<'_>) -> Result<String, String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("нет данных".into());
    };
    let raw = request.headers().get("x-name").and_then(|v| v.to_str().ok()).unwrap_or_default();
    let mut name: String = raw.chars().filter(|c| c.is_ascii_alphanumeric() || "._-".contains(*c)).collect();
    if name.trim_matches('.').is_empty() {
        name = "pc_wrapped".into();
    }
    let dir = state.store.root().join("exports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = unique_path(&dir, &name);
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // explorer понимает кавычки только вокруг пути, а не вокруг всего аргумента
        let _ = std::process::Command::new("explorer").raw_arg(format!("/select,\"{}\"", path.display())).spawn();
    }
    Ok(path.display().to_string())
}

/// `name.png`, а если такой уже есть — `name (2).png` и дальше.
fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let p = Path::new(name);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("pc_wrapped");
    let ext = p.extension().and_then(|s| s.to_str()).map(|e| format!(".{e}")).unwrap_or_default();
    let mut path = dir.join(name);
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem} ({n}){ext}"));
        n += 1;
    }
    path
}

#[tauri::command]
fn open_data_dir(state: State<'_, AppState>) {
    let _ = std::process::Command::new("explorer").arg(state.store.root()).spawn();
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(app: AppHandle, on: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    let res = if on { al.enable() } else { al.disable() };
    res.map_err(|e| e.to_string())
}

// ---------- Окна ----------

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("pc_wrapped")
        .inner_size(1120.0, 780.0)
        .min_inner_size(820.0, 600.0)
        .center()
        .build();
}

fn toggle_mini_window(app: &AppHandle) {
    let state = app.state::<AppState>();
    if let Some(w) = app.get_webview_window("mini") {
        let _ = w.close();
        let mut s = state.settings.lock().unwrap();
        s.mini_open = false;
        let _ = state.store.save_settings(&s);
        return;
    }
    open_mini_window(app);
    let mut s = state.settings.lock().unwrap();
    s.mini_open = true;
    let _ = state.store.save_settings(&s);
}

fn open_mini_window(app: &AppHandle) {
    let clickthrough = app.state::<AppState>().settings.lock().unwrap().mini_clickthrough;
    let mut b = WebviewWindowBuilder::new(app, "mini", WebviewUrl::App("index.html".into()))
        .title("pc_wrapped mini")
        .inner_size(232.0, 168.0)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false);
    if let Ok(Some(m)) = app.primary_monitor() {
        let sf = m.scale_factor();
        let w = m.size().width as f64 / sf;
        let h = m.size().height as f64 / sf;
        b = b.position(w - 260.0, h - 240.0);
    }
    if let Ok(w) = b.build() {
        let _ = w.set_ignore_cursor_events(clickthrough);
    }
}

// ---------- Уведомления ----------

const TOAST_W: f64 = 340.0;
const TOAST_H: f64 = 112.0;
/// Сколько после уведомления с итогами щелчок по трею открывает их, а не просто окно.
const STORY_AFTER_TOAST: Duration = Duration::from_secs(120);

/// Игра или видео на весь экран, презентация, заблокированный экран — молчим.
fn do_not_disturb() -> bool {
    #[cfg(windows)]
    {
        win::do_not_disturb()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Карточка справа снизу, как в мессенджерах. Окно не активируется и не ловит мышь: клики
/// проходят в то, что под ним, а при наведении карточка бледнеет. Вызывать не из главного потока.
fn show_toast(app: &AppHandle, toast: &notify::Toast) {
    *app.state::<AppState>().toast_story.lock().unwrap() = toast.action.map(|a| (a.to_string(), Instant::now()));
    if app.get_webview_window("toast").is_some() {
        // новая карточка заменяет старую и заново запускает её таймер
        let _ = app.emit_to("toast", "toast", toast);
        return;
    }
    let json = serde_json::to_string(toast).unwrap_or_else(|_| "null".into());
    let mut b = WebviewWindowBuilder::new(app, "toast", WebviewUrl::App("index.html".into()))
        .title("pc_wrapped")
        .inner_size(TOAST_W, TOAST_H)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .focusable(false)
        // класс на body ставим ещё при разборе HTML: иначе до запуска скриптов мелькнёт фиолетовый фон
        .initialization_script(format!(
            "window.__TOAST__ = {json};\
             new MutationObserver((_, o) => {{ if (document.body) {{ document.body.classList.add('toast'); o.disconnect(); }} }})\
             .observe(document, {{ childList: true, subtree: true }});"
        ));
    if let Some((x, y)) = toast_position(app) {
        b = b.position(x, y);
    }
    let Ok(w) = b.build() else { return };
    let _ = w.set_ignore_cursor_events(true);
    watch_toast_hover(app);
}

/// Сквозное окно не получает событий мыши, поэтому наведение проверяем сами, раз в 100 мс.
/// Поток один на все карточки и заканчивается, когда карточка закрылась.
fn watch_toast_hover(app: &AppHandle) {
    static WATCHING: AtomicBool = AtomicBool::new(false);
    if WATCHING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let mut over = false;
        loop {
            std::thread::sleep(Duration::from_millis(100));
            let Some(w) = app.get_webview_window("toast") else {
                WATCHING.store(false, Ordering::SeqCst);
                // карточку могли пересоздать, пока мы выходили
                if app.get_webview_window("toast").is_some() && !WATCHING.swap(true, Ordering::SeqCst) {
                    continue;
                }
                return;
            };
            let (Ok(c), Ok(p), Ok(s)) = (app.cursor_position(), w.outer_position(), w.outer_size()) else { continue };
            let now = c.x >= p.x as f64
                && c.x < p.x as f64 + s.width as f64
                && c.y >= p.y as f64
                && c.y < p.y as f64 + s.height as f64;
            if now != over {
                over = now;
                let _ = app.emit_to("toast", "toast-hover", over);
            }
        }
    });
}

fn spawn_toast(app: &AppHandle, toast: notify::Toast) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move { show_toast(&app, &toast) });
}

/// Правый нижний угол рабочей области (над панелью задач). Если там стоит мини-окно — над ним.
fn toast_position(app: &AppHandle) -> Option<(f64, f64)> {
    let m = app.primary_monitor().ok()??;
    let sf = m.scale_factor();
    let wa = m.work_area();
    let x = (wa.position.x as f64 + wa.size.width as f64) / sf - TOAST_W - 12.0;
    let mut y = (wa.position.y as f64 + wa.size.height as f64) / sf - TOAST_H - 12.0;
    if let Some(mini) = app.get_webview_window("mini") {
        if let (Ok(p), Ok(sz)) = (mini.outer_position(), mini.outer_size()) {
            let (mx, my) = (p.x as f64 / sf, p.y as f64 / sf);
            let (mw, mh) = (sz.width as f64 / sf, sz.height as f64 / sf);
            if mx < x + TOAST_W && mx + mw > x && my < y + TOAST_H && my + mh > y {
                y = my - TOAST_H - 8.0;
            }
        }
    }
    Some((x, y))
}

/// Итоги недели или месяца, если пора. Отметка ставится, даже если показывать нечего
/// (за период нет данных), — иначе диск проверялся бы каждые 15 секунд всю неделю.
fn maybe_report(app: &AppHandle, settings: &Settings, today: &str) -> bool {
    let Ok(date) = chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d") else { return false };
    let Some((kind, key, from, to)) = notify::due(date, settings) else { return false };
    let state = app.state::<AppState>();
    let days = state.store.load_range(&from.to_string(), &to.to_string());
    let toast = notify::report(kind, from, &days);
    {
        let mut s = state.settings.lock().unwrap();
        match kind {
            notify::Report::Week => s.last_weekly = key,
            notify::Report::Month => s.last_monthly = key,
        }
        let _ = state.store.save_settings(&s);
    }
    match toast {
        Some(t) => {
            spawn_toast(app, t);
            true
        }
        None => false,
    }
}

// ---------- Трей ----------

const INK: [u8; 4] = [29, 20, 66, 255];

fn metric_style(metric: &str) -> ([u8; 4], &'static str) {
    match metric {
        "keys" => ([255, 227, 61, 255], "Нажатий"),
        "clicks" => ([42, 216, 161, 255], "Кликов"),
        "mouse" => ([255, 92, 168, 255], "Мышкой, км"),
        "scroll" => ([69, 176, 255, 255], "Колесом, м"),
        _ => ([255, 138, 31, 255], "За компом, ч"),
    }
}

/// Значение метрики за сегодня: (текст в значке, полное значение для подсказки).
fn metric_value(metric: &str, d: &DayStats) -> (String, String) {
    let full = |n: u64| {
        let s = n.to_string();
        let mut out = String::new();
        for (i, c) in s.chars().enumerate() {
            if i > 0 && (s.len() - i) % 3 == 0 {
                out.push(' ');
            }
            out.push(c);
        }
        out
    };
    match metric {
        "keys" => (font::compact(d.keys as f64), full(d.keys)),
        "clicks" => (font::compact(d.clicks() as f64), full(d.clicks())),
        "mouse" => {
            let km = d.mouse_mm / 1_000_000.0;
            (font::compact_km(km), format!("{:.2}", km).replace('.', ","))
        }
        "scroll" => {
            let m = scroll_meters(d.wheel_notches);
            (font::compact(m.floor()), full(m.round() as u64))
        }
        _ => {
            let h = d.active_seconds as f64 / 3600.0;
            (font::compact_km(h), format!("{} ч {} мин", d.active_seconds / 3600, d.active_seconds % 3600 / 60))
        }
    }
}

/// Один щелчок колеса прокручивает страницу примерно на 1,5 см экрана.
pub fn scroll_meters(notches: f64) -> f64 {
    notches * 0.015
}

fn icon_size() -> u32 {
    #[cfg(windows)]
    {
        win::small_icon_size()
    }
    #[cfg(not(windows))]
    {
        16
    }
}

/// Значок: системный шрифт, если получилось, иначе свой пиксельный.
fn tray_pixels(text: &str, size: u32, bg: [u8; 4]) -> Vec<u8> {
    // На 16px системный шрифт в 3–4 символа не помещается без каши — там чётче пиксельный 3×5.
    #[cfg(windows)]
    if size > 16 {
        if let Some(mask) = win::text_mask(text, size) {
            return font::compose(&mask, size, bg, INK);
        }
    }
    font::render(text, size, bg, INK)
}

fn tray_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let open = MenuItem::with_id(app, "open", "Открыть статистику", true, None::<&str>)?;
    let mini = MenuItem::with_id(app, "mini", "Мини-окно: показать или скрыть", true, None::<&str>)?;
    let through = MenuItem::with_id(app, "through", "Мини-окно: пропускать клики", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
    Menu::with_items(app, &[&open, &mini, &through, &sep, &quit])
}

fn on_menu(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        "open" => show_main(app),
        "mini" => {
            // не строим окно прямо в обработчике меню — по той же причине, что и в командах
            let app = app.clone();
            tauri::async_runtime::spawn(async move { toggle_mini_window(&app) });
        }
        "through" => {
            let state = app.state::<AppState>();
            let mut s = state.settings.lock().unwrap().clone();
            s.mini_clickthrough = !s.mini_clickthrough;
            let _ = set_settings(app.clone(), state, s);
        }
        "quit" => {
            save_today(app);
            app.exit(0);
        }
        _ => {}
    }
}

fn on_tray_click(tray: &tauri::tray::TrayIcon, event: TrayIconEvent) {
    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
        let app = tray.app_handle();
        let story = app.state::<AppState>().toast_story.lock().unwrap().take();
        match story {
            Some((action, at)) if at.elapsed() < STORY_AFTER_TOAST => open_story(app, action),
            _ => show_main(app),
        }
    }
}

/// Создаёт, обновляет и убирает значки-счётчики по списку из настроек. Вызывать из главного потока.
fn sync_tray(app: &AppHandle, settings: &Settings, day: &DayStats) {
    let state = app.state::<AppState>();
    let size = icon_size();
    for metric in METRICS {
        let id = format!("m-{metric}");
        let wanted = settings.tray_metrics.iter().any(|m| m == metric);
        if !wanted {
            if app.remove_tray_by_id(id.as_str()).is_some() {
                state.tray_text.lock().unwrap().remove(&id);
            }
            continue;
        }
        let (text, full) = metric_value(metric, day);
        let (bg, label) = metric_style(metric);
        let tip = format!("{label} сегодня: {full}");
        let key = format!("{text}@{size}");
        if state.tray_text.lock().unwrap().get(&id) == Some(&key) {
            if let Some(t) = app.tray_by_id(id.as_str()) {
                let _ = t.set_tooltip(Some(&tip));
            }
            continue;
        }
        let img = Image::new_owned(tray_pixels(&text, size, bg), size, size);
        if let Some(t) = app.tray_by_id(id.as_str()) {
            let _ = t.set_icon(Some(img));
            let _ = t.set_tooltip(Some(&tip));
        } else {
            let Ok(menu) = tray_menu(app) else { continue };
            let built = TrayIconBuilder::with_id(id.clone())
                .icon(img)
                .tooltip(&tip)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(on_menu)
                .on_tray_icon_event(on_tray_click)
                .build(app);
            if built.is_err() {
                continue;
            }
        }
        state.tray_text.lock().unwrap().insert(id, key);
    }
}

fn save_today(app: &AppHandle) {
    let state = app.state::<AppState>();
    let day = state.tracker.lock().unwrap().day.clone();
    let _ = state.store.save_day(&day);
}

// ---------- Фоновый тик ----------

fn start_ticker(app: AppHandle) {
    std::thread::Builder::new()
        .name("ticker".into())
        .spawn(move || {
            #[cfg(windows)]
            let mut fg = win::Foreground::new();
            let mut n: u64 = 0;
            // сколько напоминаний о перерыве уже было в текущей сессии
            let mut reminded: u64 = 0;
            let mut last_toast: Option<std::time::Instant> = None;
            loop {
                std::thread::sleep(Duration::from_secs(1));
                n += 1;
                let now = Now::local();
                #[cfg(windows)]
                let current_app = fg.current();
                #[cfg(not(windows))]
                let current_app: Option<String> = None;

                let state = app.state::<AppState>();
                let (finished, snapshot, dirty, session, idle) = {
                    let mut t = state.tracker.lock().unwrap();
                    let finished = t.tick(&now, current_app.as_deref());
                    let dirty = t.dirty;
                    if n % 60 == 0 {
                        t.dirty = false;
                    }
                    (finished, t.day.clone(), dirty, t.session_secs(now.instant), t.idle(now.instant))
                };
                if let Some(old) = finished {
                    let _ = state.store.save_day(&old);
                }
                if n % 60 == 0 && dirty {
                    let _ = state.store.save_day(&snapshot);
                }
                let settings = state.settings.lock().unwrap().clone();

                // Перерыв: напоминаем на каждой кратной `break_minutes` отметке сессии. Счётчик идёт
                // и при выключенном напоминании, чтобы после включения оно не выстрелило сразу.
                let due_breaks = session / (settings.break_minutes.max(15) as u64 * 60);
                if session == 0 {
                    reminded = 0;
                } else if due_breaks > reminded {
                    reminded = due_breaks;
                    // в игре на весь экран напоминание просто пропускаем: следующее будет через break_minutes
                    if settings.notifications && settings.break_reminder && !do_not_disturb() {
                        spawn_toast(&app, notify::break_toast(session));
                        last_toast = Some(now.instant);
                    }
                }

                // Итоги ждут, пока человек за компом и не в полноэкранной игре, и не наступают на пятки другой карточке.
                let calm = last_toast.map_or(true, |t| now.instant.saturating_duration_since(t) > Duration::from_secs(10));
                if n % 15 == 0 && settings.notifications && !idle && calm && !do_not_disturb() && maybe_report(&app, &settings, &snapshot.date) {
                    last_toast = Some(now.instant);
                }

                if n % 2 == 0 {
                    // значки трея трогаем только из главного потока
                    let a = app.clone();
                    let day = snapshot.clone();
                    let _ = app.run_on_main_thread(move || sync_tray(&a, &settings, &day));
                }
                if !app.webview_windows().is_empty() {
                    let _ = app.emit("live", &snapshot);
                }
            }
        })
        .expect("ticker thread");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--autostart"])))
        .setup(|app| {
            let handle = app.handle().clone();
            let app_dir = app.path().app_data_dir()?;
            // пока папку не выбрали (или диск с ней пропал) — пишем в %APPDATA%, при выборе всё переедет
            let (store, data_chosen) = match store::read_location(&app_dir).map(Store::new) {
                Some(Ok(s)) => (s, true),
                _ => (Store::new(app_dir.clone())?, false),
            };
            let first_run = !store.root().join("settings.json").exists();
            let store = Arc::new(store);
            let settings = store.load_settings();
            let now = Now::local();
            let today = store.load_day(&now.date).unwrap_or_else(|| DayStats::new(&now.date));
            let tracker = Arc::new(Mutex::new(Tracker::new(today, settings.dpi, &now)));
            if first_run {
                let _ = store.save_settings(&settings);
            }

            #[cfg(windows)]
            win::start_input(tracker.clone());

            app.manage(AppState {
                tracker,
                store,
                app_dir,
                data_chosen: AtomicBool::new(data_chosen),
                settings: Mutex::new(settings.clone()),
                tray_text: Mutex::new(HashMap::new()),
                pending_story: Mutex::new(None),
                toast_story: Mutex::new(None),
            });

            // Главный значок приложения: левый клик открывает статистику.
            let menu = tray_menu(&handle)?;
            let mut main_tray = TrayIconBuilder::with_id("main")
                .tooltip("pc_wrapped: статистика за компьютером")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(on_menu)
                .on_tray_icon_event(on_tray_click);
            if let Some(icon) = app.default_window_icon().cloned() {
                main_tray = main_tray.icon(icon);
            }
            main_tray.build(&handle)?;

            let day = handle.state::<AppState>().tracker.lock().unwrap().day.clone();
            sync_tray(&handle, &settings, &day);
            start_ticker(handle.clone());

            let autostarted = std::env::args().any(|a| a == "--autostart");
            if first_run || !data_chosen || !autostarted {
                show_main(&handle);
            }
            if settings.mini_open {
                open_mini_window(&handle);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_today,
            get_days,
            first_date,
            get_settings,
            set_settings,
            toggle_mini,
            open_main,
            open_data_dir,
            get_data_dir,
            pick_folder,
            set_data_dir,
            get_autostart,
            set_autostart,
            notify_test,
            take_pending_story,
            save_export
        ])
        .build(tauri::generate_context!())
        .expect("error while building pc_wrapped");

    app.run(|app, event| match event {
        // Закрыли все окна — продолжаем жить в трее.
        RunEvent::ExitRequested { api, code, .. } => {
            if code.is_none() {
                api.prevent_exit();
            }
        }
        RunEvent::Exit => save_today(app),
        _ => {}
    });
}
