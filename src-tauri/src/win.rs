//! Всё, что завязано на Windows API.
//!
//! Ввод читаем через Raw Input, а не через глобальные хуки: хук встаёт в цепочку обработки
//! каждого нажатия и может добавить задержку в играх, а Raw Input просто присылает копию
//! событий в наше невидимое окно. Поэтому программа не влияет на отзывчивость ввода.

use crate::tracker::Tracker;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, DeleteDC, DeleteObject, DrawTextW, GdiFlush, GetTextExtentPoint32W, SelectObject,
    SetBkMode, SetTextColor, ANTIALIASED_QUALITY, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET,
    DIB_RGB_COLORS, DT_CENTER, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FF_SWISS, FW_BOLD, OUT_TT_PRECIS, TRANSPARENT,
};
use windows_sys::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION};
use windows_sys::Win32::UI::Input::{
    GetRawInputData, RegisterRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RIDEV_INPUTSINK, RID_INPUT,
    RIM_TYPEKEYBOARD, RIM_TYPEMOUSE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetForegroundWindow, GetMessageW, GetSystemMetrics, GetWindowThreadProcessId,
    RegisterClassW, HWND_MESSAGE, MSG, SM_CXSMICON, WM_INPUT, WNDCLASSW,
};

static SINK: OnceLock<Arc<Mutex<Tracker>>> = OnceLock::new();

const RI_KEY_BREAK: u16 = 1;
const MOUSE_MOVE_ABSOLUTE: u16 = 1;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Запускает поток с невидимым окном, которое получает Raw Input от всех мышей и клавиатур.
pub fn start_input(tracker: Arc<Mutex<Tracker>>) {
    if SINK.set(tracker).is_err() {
        return;
    }
    std::thread::Builder::new()
        .name("raw-input".into())
        .spawn(|| unsafe { input_thread() })
        .expect("raw input thread");
}

unsafe fn input_thread() {
    let class = wide("pc_wrapped_raw_input");
    let hinst = GetModuleHandleW(std::ptr::null());
    let mut wc: WNDCLASSW = std::mem::zeroed();
    wc.lpfnWndProc = Some(wndproc);
    wc.hInstance = hinst;
    wc.lpszClassName = class.as_ptr();
    RegisterClassW(&wc);
    let hwnd = CreateWindowExW(
        0,
        class.as_ptr(),
        std::ptr::null(),
        0,
        0,
        0,
        0,
        0,
        HWND_MESSAGE,
        std::ptr::null_mut(),
        hinst,
        std::ptr::null(),
    );
    if hwnd.is_null() {
        return;
    }
    let devices = [
        // мышь: Generic Desktop / Mouse
        RAWINPUTDEVICE { usUsagePage: 0x01, usUsage: 0x02, dwFlags: RIDEV_INPUTSINK, hwndTarget: hwnd },
        // клавиатура: Generic Desktop / Keyboard
        RAWINPUTDEVICE { usUsagePage: 0x01, usUsage: 0x06, dwFlags: RIDEV_INPUTSINK, hwndTarget: hwnd },
    ];
    if RegisterRawInputDevices(devices.as_ptr(), devices.len() as u32, std::mem::size_of::<RAWINPUTDEVICE>() as u32) == 0 {
        return;
    }
    let mut msg: MSG = std::mem::zeroed();
    while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
        DispatchMessageW(&msg);
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_INPUT {
        handle_input(lparam as HRAWINPUT);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

unsafe fn handle_input(h: HRAWINPUT) {
    // RAWINPUT для мыши и клавиатуры влезает в 64 байта; берём с запасом и с выравниванием.
    let mut buf = [0u64; 16];
    let mut size = std::mem::size_of_val(&buf) as u32;
    let got = GetRawInputData(h, RID_INPUT, buf.as_mut_ptr().cast(), &mut size, std::mem::size_of::<RAWINPUTHEADER>() as u32);
    if got == u32::MAX || got == 0 {
        return;
    }
    let raw = &*(buf.as_ptr() as *const RAWINPUT);
    let Some(sink) = SINK.get() else { return };
    let Ok(mut t) = sink.lock() else { return };
    match raw.header.dwType {
        RIM_TYPEKEYBOARD => {
            let kb = raw.data.keyboard;
            t.on_key(kb.VKey, kb.Flags & RI_KEY_BREAK == 0);
        }
        RIM_TYPEMOUSE => {
            let m = raw.data.mouse;
            // планшеты и удалённый рабочий стол присылают абсолютные координаты — их не считаем
            if m.usFlags & MOUSE_MOVE_ABSOLUTE == 0 {
                t.on_move(m.lLastX, m.lLastY);
            }
            let flags = m.Anonymous.Anonymous.usButtonFlags;
            if flags != 0 {
                t.on_buttons(flags, m.Anonymous.Anonymous.usButtonData as i16);
            }
        }
        _ => {}
    }
}

/// Размер маленького значка с учётом масштаба экрана: 16 при 100%, 24 при 150%, 32 при 200%.
pub fn small_icon_size() -> u32 {
    let s = unsafe { GetSystemMetrics(SM_CXSMICON) };
    if s > 0 { s as u32 } else { 16 }
}

/// Название программы в активном окне: «Описание файла» из exe (Google Chrome, Battle.net),
/// а если его нет — имя exe без расширения.
pub struct Foreground {
    cache: HashMap<String, String>,
}

impl Foreground {
    pub fn new() -> Self {
        Foreground { cache: HashMap::new() }
    }

    pub fn current(&mut self) -> Option<String> {
        let path = unsafe { foreground_exe()? };
        if let Some(n) = self.cache.get(&path) {
            return Some(n.clone());
        }
        let name = unsafe { file_description(&path) }.unwrap_or_else(|| exe_stem(&path));
        if self.cache.len() > 500 {
            self.cache.clear();
        }
        self.cache.insert(path, name.clone());
        Some(name)
    }
}

pub fn exe_stem(path: &str) -> String {
    let file = path.rsplit(['\\', '/']).next().unwrap_or(path);
    file.strip_suffix(".exe").or_else(|| file.strip_suffix(".EXE")).unwrap_or(file).to_string()
}

unsafe fn foreground_exe() -> Option<String> {
    let hwnd = GetForegroundWindow();
    if hwnd.is_null() {
        return None;
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == 0 {
        return None;
    }
    let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if h.is_null() {
        return None;
    }
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len);
    CloseHandle(h);
    if ok == 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buf[..len as usize]))
}

unsafe fn file_description(path: &str) -> Option<String> {
    let wpath = wide(path);
    let size = GetFileVersionInfoSizeW(wpath.as_ptr(), std::ptr::null_mut());
    if size == 0 {
        return None;
    }
    let mut data = vec![0u8; size as usize];
    if GetFileVersionInfoW(wpath.as_ptr(), 0, size, data.as_mut_ptr().cast()) == 0 {
        return None;
    }
    // язык и кодовая страница первой записи
    let mut ptr: *mut core::ffi::c_void = std::ptr::null_mut();
    let mut len = 0u32;
    let tr = wide("\\VarFileInfo\\Translation");
    if VerQueryValueW(data.as_ptr().cast(), tr.as_ptr(), &mut ptr, &mut len) == 0 || len < 4 {
        return None;
    }
    let lang = *(ptr as *const u16);
    let cp = *(ptr as *const u16).add(1);
    let key = wide(&format!("\\StringFileInfo\\{lang:04x}{cp:04x}\\FileDescription"));
    if VerQueryValueW(data.as_ptr().cast(), key.as_ptr(), &mut ptr, &mut len) == 0 || len == 0 {
        return None;
    }
    let s = std::slice::from_raw_parts(ptr as *const u16, len as usize);
    let s = String::from_utf16_lossy(s);
    let s = s.trim_end_matches('\0').trim();
    if s.is_empty() || s.len() > 60 { None } else { Some(s.to_string()) }
}

/// Рисует текст системным шрифтом (Bahnschrift, жирный, сжатый по ширине) и возвращает маску
/// прозрачности `size×size`. Цифры вытягиваются почти на всю высоту значка, как у Afterburner.
pub fn text_mask(text: &str, size: u32) -> Option<Vec<u8>> {
    let s = size as i32;
    let wtext: Vec<u16> = text.encode_utf16().collect();
    if wtext.is_empty() || s < 8 {
        return None;
    }
    unsafe {
        let hdc = CreateCompatibleDC(std::ptr::null_mut());
        if hdc.is_null() {
            return None;
        }
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = s;
        bmi.bmiHeader.biHeight = -s; // строки сверху вниз
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB as _;
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let bmp = CreateDIBSection(hdc, &bmi, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
        if bmp.is_null() || bits.is_null() {
            DeleteDC(hdc);
            return None;
        }
        let old_bmp = SelectObject(hdc, bmp as _);
        SetBkMode(hdc, TRANSPARENT as _);
        SetTextColor(hdc, 0x00FF_FFFF);

        // Высота шрифта = размер значка; ширину символов ужимаем, пока строка не влезет.
        let face = wide("Bahnschrift");
        let mut width = (s * 9 / 20).max(2);
        let mut font;
        let mut old_font;
        loop {
            font = CreateFontW(
                -s,
                width,
                0,
                0,
                FW_BOLD as _,
                0,
                0,
                0,
                DEFAULT_CHARSET as _,
                OUT_TT_PRECIS as _,
                CLIP_DEFAULT_PRECIS as _,
                ANTIALIASED_QUALITY as _,
                FF_SWISS as _,
                face.as_ptr(),
            );
            if font.is_null() {
                SelectObject(hdc, old_bmp);
                DeleteObject(bmp as _);
                DeleteDC(hdc);
                return None;
            }
            old_font = SelectObject(hdc, font as _);
            let mut sz: SIZE = std::mem::zeroed();
            GetTextExtentPoint32W(hdc, wtext.as_ptr(), wtext.len() as i32, &mut sz);
            if sz.cx <= s - 1 || width <= 2 {
                break;
            }
            SelectObject(hdc, old_font);
            DeleteObject(font as _);
            width -= 1;
        }

        // Цифры сидят выше центра строки (место под хвосты букв), поэтому чуть опускаем.
        let shift = s / 10;
        let mut rc = RECT { left: 0, top: shift, right: s, bottom: s + shift };
        DrawTextW(hdc, wtext.as_ptr(), wtext.len() as i32, &mut rc, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        GdiFlush();

        let px = std::slice::from_raw_parts(bits as *const u8, (s * s * 4) as usize);
        let mask: Vec<u8> = px.chunks_exact(4).map(|p| p[0].max(p[1]).max(p[2])).collect();

        SelectObject(hdc, old_font);
        DeleteObject(font as _);
        SelectObject(hdc, old_bmp);
        DeleteObject(bmp as _);
        DeleteDC(hdc);

        if mask.iter().any(|&a| a > 0) { Some(mask) } else { None }
    }
}
