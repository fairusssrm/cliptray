// Sembunyikan jendela konsol di build release Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    cliptray_lib::run()
}
