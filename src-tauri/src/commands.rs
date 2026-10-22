use crate::db::{Clip, Db};
use crate::watcher::{IGNORE_NEXT, PAUSED};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_clipboard_manager::ClipboardExt;

fn notify_changed(app: &AppHandle) {
    let _ = app.emit_to("popup", "history-changed", ());
}

#[tauri::command]
pub fn get_history(db: State<'_, Db>, query: String) -> Result<Vec<Clip>, String> {
    db.list(&query).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn copy_item(app: AppHandle, db: State<'_, Db>, id: i64) -> Result<(), String> {
    let Some(content) = db.touch(id).map_err(|e| e.to_string())? else {
        return Err("item tidak ditemukan".into());
    };
    IGNORE_NEXT.store(true, Ordering::SeqCst);
    if let Err(e) = app.clipboard().write_text(content) {
        IGNORE_NEXT.store(false, Ordering::SeqCst);
        return Err(e.to_string());
    }
    crate::hide_popup(&app);
    notify_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn set_pinned(app: AppHandle, db: State<'_, Db>, id: i64, pinned: bool) -> Result<(), String> {
    db.set_pinned(id, pinned).map_err(|e| e.to_string())?;
    notify_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_item(app: AppHandle, db: State<'_, Db>, id: i64) -> Result<(), String> {
    db.delete(id).map_err(|e| e.to_string())?;
    notify_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn clear_all(app: AppHandle, db: State<'_, Db>, include_pinned: bool) -> Result<(), String> {
    db.clear(include_pinned).map_err(|e| e.to_string())?;
    notify_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn get_paused() -> bool {
    PAUSED.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn set_paused(app: AppHandle, paused: bool) -> bool {
    crate::set_paused(&app, paused);
    paused
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> bool {
    crate::set_autostart(&app, enabled)
}

#[tauri::command]
pub fn hide_popup(app: AppHandle) {
    crate::hide_popup(&app);
}
