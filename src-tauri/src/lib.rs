mod font;
mod model;
mod store;
mod tracker;
#[cfg(windows)]
mod win;

use model::{DayStats, Settings, METRICS};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use store::Store;
use tauri::image::Image;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder};
use tracker::{Now, Tracker};

struct AppState {
    tracker: Arc<Mutex<Tracker>>,
    store: Arc<Store>,
    settings: Mutex<Settings>,
    /// Что сейчас нарисовано в каждом значке трея, чтобы не перерисовывать то же самое.
    tray_text: Mutex<HashMap<String, String>>,
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
        show_main(tray.app_handle());
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
            loop {
                std::thread::sleep(Duration::from_secs(1));
                n += 1;
                let now = Now::local();
                #[cfg(windows)]
                let current_app = fg.current();
                #[cfg(not(windows))]
                let current_app: Option<String> = None;

                let state = app.state::<AppState>();
                let (finished, snapshot, dirty) = {
                    let mut t = state.tracker.lock().unwrap();
                    let finished = t.tick(&now, current_app.as_deref());
                    let dirty = t.dirty;
                    if n % 60 == 0 {
                        t.dirty = false;
                    }
                    (finished, t.day.clone(), dirty)
                };
                if let Some(old) = finished {
                    let _ = state.store.save_day(&old);
                }
                if n % 60 == 0 && dirty {
                    let _ = state.store.save_day(&snapshot);
                }
                if n % 2 == 0 {
                    // значки трея трогаем только из главного потока
                    let settings = state.settings.lock().unwrap().clone();
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
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--autostart"])))
        .setup(|app| {
            let handle = app.handle().clone();
            let dir = app.path().app_data_dir()?;
            let first_run = !dir.join("settings.json").exists();
            let store = Arc::new(Store::new(dir)?);
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
                settings: Mutex::new(settings.clone()),
                tray_text: Mutex::new(HashMap::new()),
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
            if first_run || !autostarted {
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
            get_autostart,
            set_autostart
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
