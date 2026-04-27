use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

static WATCHER_RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardChange {
    pub text: String,
}

#[tauri::command]
pub fn start_clipboard_watcher(app: AppHandle) -> Result<(), String> {
    if WATCHER_RUNNING.load(Ordering::SeqCst) {
        return Ok(());
    }
    WATCHER_RUNNING.store(true, Ordering::SeqCst);

    std::thread::spawn(move || {
        let mut clipboard = match arboard::Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to open clipboard: {}", e);
                WATCHER_RUNNING.store(false, Ordering::SeqCst);
                return;
            }
        };

        let mut last_text = clipboard.get_text().unwrap_or_default();

        while WATCHER_RUNNING.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(800));

            let current = match clipboard.get_text() {
                Ok(t) => t,
                Err(_) => continue,
            };

            if !current.is_empty() && current != last_text {
                last_text = current.clone();

                let trimmed = current.trim();
                if !trimmed.is_empty() && trimmed.len() < 5000 {
                    let _ = app.emit("clipboard:changed", ClipboardChange {
                        text: trimmed.to_string(),
                    });
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn stop_clipboard_watcher() -> Result<(), String> {
    WATCHER_RUNNING.store(false, Ordering::SeqCst);
    Ok(())
}
