use crate::db::Db;
use clipboard_master::{CallbackResult, ClipboardHandler, Master};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

/// Diset oleh `copy_item` agar event clipboard dari app sendiri tidak dicatat ulang.
pub static IGNORE_NEXT: AtomicBool = AtomicBool::new(false);
/// Saat aktif, perubahan clipboard tidak dicatat.
pub static PAUSED: AtomicBool = AtomicBool::new(false);

const MAX_BYTES: usize = 1024 * 1024;
const READ_RETRIES: u32 = 5;
const RETRY_DELAY: Duration = Duration::from_millis(30);

/// Password manager (KeePass, 1Password, Bitwarden, dll.) menandai isi sensitif
/// dengan format ini agar tidak dicatat oleh aplikasi riwayat clipboard.
#[cfg(windows)]
fn is_excluded_by_source() -> bool {
    use windows_sys::Win32::System::DataExchange::{
        IsClipboardFormatAvailable, RegisterClipboardFormatW,
    };
    let name: Vec<u16> = "ExcludeClipboardContentFromMonitorProcessing\0"
        .encode_utf16()
        .collect();
    unsafe {
        let format = RegisterClipboardFormatW(name.as_ptr());
        format != 0 && IsClipboardFormatAvailable(format) != 0
    }
}

#[cfg(not(windows))]
fn is_excluded_by_source() -> bool {
    false
}

struct Handler {
    app: AppHandle,
}

impl Handler {
    /// Clipboard bisa terkunci sesaat oleh app lain, jadi coba beberapa kali.
    fn read_text(&self) -> Option<String> {
        for attempt in 0..READ_RETRIES {
            match self.app.clipboard().read_text() {
                Ok(text) => return Some(text),
                Err(_) if attempt + 1 < READ_RETRIES => thread::sleep(RETRY_DELAY),
                Err(_) => {}
            }
        }
        None
    }
}

impl ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        if IGNORE_NEXT.swap(false, Ordering::SeqCst)
            || PAUSED.load(Ordering::SeqCst)
            || is_excluded_by_source()
        {
            return CallbackResult::Next;
        }
        let Some(text) = self.read_text() else {
            return CallbackResult::Next;
        };
        if text.trim().is_empty() || text.len() > MAX_BYTES {
            return CallbackResult::Next;
        }
        let db = self.app.state::<Db>();
        match db.upsert(&text) {
            Ok(()) => {
                let _ = self.app.emit_to("popup", "history-changed", ());
            }
            Err(e) => eprintln!("gagal menyimpan clip: {e}"),
        }
        CallbackResult::Next
    }

    fn on_clipboard_error(&mut self, error: io::Error) -> CallbackResult {
        eprintln!("clipboard listener error: {error}");
        CallbackResult::Next
    }
}

pub fn spawn(app: AppHandle) {
    thread::Builder::new()
        .name("clipboard-watcher".into())
        .spawn(move || match Master::new(Handler { app }) {
            Ok(mut master) => {
                if let Err(e) = master.run() {
                    eprintln!("clipboard listener berhenti: {e}");
                }
            }
            Err(e) => eprintln!("gagal membuat clipboard listener: {e}"),
        })
        .expect("gagal membuat thread watcher");
}
