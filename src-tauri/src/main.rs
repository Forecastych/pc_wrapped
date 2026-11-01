// В релизе не открываем консольное окно рядом с программой.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    pc_wrapped_lib::run()
}
