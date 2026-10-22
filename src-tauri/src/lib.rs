mod commands;
mod db;
mod watcher;

use db::Db;
use std::sync::atomic::Ordering;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow, WindowEvent, Wry};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const HOTKEY: &str = "CmdOrCtrl+Shift+V";
const TRAY_ID: &str = "main";

/// Item menu bercentang, disimpan agar statusnya bisa disinkronkan dari popup.
struct TrayItems {
    pause: CheckMenuItem<Wry>,
    autostart: CheckMenuItem<Wry>,
}

/// Aktifkan/nonaktifkan autostart, sinkronkan ke tray dan popup, lalu kembalikan status sebenarnya.
pub fn set_autostart(app: &AppHandle, enabled: bool) -> bool {
    let launcher = app.autolaunch();
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    if let Err(e) = result {
        eprintln!("gagal mengubah autostart: {e}");
    }
    let actual = launcher.is_enabled().unwrap_or(false);
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.autostart.set_checked(actual);
    }
    let _ = app.emit_to("popup", "autostart-changed", actual);
    actual
}

/// Ubah status jeda dan sinkronkan ke menu tray, tooltip, dan popup.
pub fn set_paused(app: &AppHandle, paused: bool) {
    watcher::PAUSED.store(paused, Ordering::SeqCst);
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.pause.set_checked(paused);
    }
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let tooltip = if paused { "ClipTray (dijeda)" } else { "ClipTray" };
        let _ = tray.set_tooltip(Some(tooltip));
    }
    let _ = app.emit_to("popup", "paused-changed", paused);
}

fn popup(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("popup")
}

/// Letakkan popup di dekat kursor, tetap di dalam area kerja monitor tempat kursor berada.
fn position_near_cursor(app: &AppHandle, win: &WebviewWindow) {
    let Ok(cursor) = app.cursor_position() else { return };
    let monitor = app
        .monitor_from_point(cursor.x, cursor.y)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let Ok(size) = win.outer_size() else { return };

    let area = monitor.work_area();
    let (left, top) = (area.position.x as f64, area.position.y as f64);
    let right = left + area.size.width as f64;
    let bottom = top + area.size.height as f64;
    let (w, h) = (size.width as f64, size.height as f64);

    let mut x = cursor.x;
    let mut y = cursor.y;
    if x + w > right {
        x = cursor.x - w;
    }
    if y + h > bottom {
        y = cursor.y - h;
    }
    x = x.clamp(left, (right - w).max(left));
    y = y.clamp(top, (bottom - h).max(top));
    let _ = win.set_position(PhysicalPosition::new(x as i32, y as i32));
}

pub fn show_popup(app: &AppHandle) {
    let Some(win) = popup(app) else { return };
    position_near_cursor(app, &win);
    let _ = win.show();
    let _ = win.set_focus();
    let _ = app.emit_to("popup", "popup-shown", ());
}

pub fn hide_popup(app: &AppHandle) {
    if let Some(win) = popup(app) {
        let _ = win.hide();
    }
}

fn toggle_popup(app: &AppHandle) {
    match popup(app) {
        Some(win) if win.is_visible().unwrap_or(false) => hide_popup(app),
        Some(_) => show_popup(app),
        None => {}
    }
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let autostart_on = app.autolaunch().is_enabled().unwrap_or(false);
    let open = MenuItem::with_id(app, "open", "Buka", true, None::<&str>)?;
    let clear = MenuItem::with_id(app, "clear", "Hapus semua", true, None::<&str>)?;
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        "Start with Windows",
        true,
        autostart_on,
        None::<&str>,
    )?;
    let pause = CheckMenuItem::with_id(app, "pause", "Jeda pencatatan", true, false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Keluar", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[&open, &pause, &sep, &clear, &autostart, &sep2, &quit],
    )?;
    app.manage(TrayItems {
        pause,
        autostart: autostart.clone(),
    });

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("ClipTray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "open" => show_popup(app),
            "pause" => set_paused(app, !watcher::PAUSED.load(Ordering::SeqCst)),
            "clear" => {
                if let Err(e) = app.state::<Db>().clear(false) {
                    eprintln!("gagal menghapus riwayat: {e}");
                }
                let _ = app.emit_to("popup", "history-changed", ());
            }
            "autostart" => {
                let enabled = app.autolaunch().is_enabled().unwrap_or(false);
                set_autostart(app, !enabled);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_popup(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Harus pertama: instance kedua hanya membuka popup di instance yang sudah jalan.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_popup(app);
        }))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        toggle_popup(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db = Db::open(&dir.join("history.db"))?;
            app.manage(db);

            let handle = app.handle().clone();
            setup_tray(&handle)?;

            // Jika hotkey bentrok, popup tetap bisa dibuka lewat klik tray.
            if let Err(e) = app.global_shortcut().register(HOTKEY) {
                eprintln!("gagal mendaftarkan hotkey {HOTKEY}: {e}");
            }

            if let Some(win) = popup(&handle) {
                let w = win.clone();
                win.on_window_event(move |event| match event {
                    WindowEvent::Focused(false) => {
                        let _ = w.hide();
                    }
                    WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        let _ = w.hide();
                    }
                    _ => {}
                });
            }

            watcher::spawn(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_history,
            commands::copy_item,
            commands::set_pinned,
            commands::delete_item,
            commands::clear_all,
            commands::get_paused,
            commands::set_paused,
            commands::get_autostart,
            commands::set_autostart,
            commands::hide_popup
        ])
        .run(tauri::generate_context!())
        .expect("gagal menjalankan ClipTray");
}
