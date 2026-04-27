use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

static SELECTION_RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionDetected {
    pub text: String,
    pub mouse_x: f64,
    pub mouse_y: f64,
}

#[cfg(target_os = "windows")]
pub(crate) mod win {
    use std::mem::MaybeUninit;

    #[repr(C)]
    pub struct POINT {
        pub x: i32,
        pub y: i32,
    }

    type HWND = *mut std::ffi::c_void;

    extern "system" {
        pub fn GetCursorPos(lp_point: *mut POINT) -> i32;
        pub fn GetAsyncKeyState(v_key: i32) -> i16;
        pub fn GetForegroundWindow() -> HWND;
        pub fn GetWindowThreadProcessId(h_wnd: HWND, lp_dw_process_id: *mut u32) -> u32;
    }

    pub fn cursor_pos() -> (i32, i32) {
        let mut pt = MaybeUninit::<POINT>::uninit();
        let ok = unsafe { GetCursorPos(pt.as_mut_ptr()) };
        if ok != 0 {
            let p = unsafe { pt.assume_init() };
            (p.x, p.y)
        } else {
            (0, 0)
        }
    }

    pub fn is_lbutton_down() -> bool {
        (unsafe { GetAsyncKeyState(0x01) } & (1i16 << 15)) != 0
    }

    pub fn is_own_window() -> bool {
        let fg = unsafe { GetForegroundWindow() };
        if fg.is_null() {
            return false;
        }
        let mut pid: u32 = 0;
        unsafe { GetWindowThreadProcessId(fg, &mut pid) };
        pid == std::process::id()
    }
}

fn get_selected_text_via_clipboard() -> Option<String> {
    let mut cb = arboard::Clipboard::new().ok()?;
    let old = cb.get_text().unwrap_or_default();

    {
        use enigo::{Direction, Enigo, Key, Keyboard, Settings};
        let mut e = Enigo::new(&Settings::default()).ok()?;
        let _ = e.key(Key::Control, Direction::Press);
        std::thread::sleep(Duration::from_millis(15));
        let _ = e.key(Key::Unicode('c'), Direction::Click);
        std::thread::sleep(Duration::from_millis(15));
        let _ = e.key(Key::Control, Direction::Release);
    }

    std::thread::sleep(Duration::from_millis(80));

    let new_text = cb.get_text().unwrap_or_default();

    if !new_text.is_empty() && new_text != old {
        let old_clone = old.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            if let Ok(mut cb2) = arboard::Clipboard::new() {
                let _ = cb2.set_text(old_clone);
            }
        });
        Some(new_text)
    } else {
        None
    }
}

#[tauri::command]
pub fn start_selection_watcher(app: AppHandle) -> Result<(), String> {
    if SELECTION_RUNNING.load(Ordering::SeqCst) {
        return Ok(());
    }
    SELECTION_RUNNING.store(true, Ordering::SeqCst);

    std::thread::spawn(move || {
        let mut was_down = false;
        let mut press_x: i32 = 0;
        let mut press_y: i32 = 0;
        let mut last_emit_time = Instant::now();
        let mut last_text = String::new();
        let cooldown = Duration::from_millis(600);
        let min_drag_distance: i32 = 10;

        while SELECTION_RUNNING.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(50));

            #[cfg(target_os = "windows")]
            {
                let down = win::is_lbutton_down();

                if down && !was_down {
                    let (px, py) = win::cursor_pos();
                    press_x = px;
                    press_y = py;
                }

                if was_down && !down {
                    if win::is_own_window() {
                        was_down = down;
                        continue;
                    }

                    let (rx, ry) = win::cursor_pos();
                    let dx = (rx - press_x).abs();
                    let dy = (ry - press_y).abs();
                    let drag_dist = ((dx * dx + dy * dy) as f64).sqrt() as i32;

                    if drag_dist >= min_drag_distance && last_emit_time.elapsed() > cooldown {
                        std::thread::sleep(Duration::from_millis(100));

                        if let Some(text) = get_selected_text_via_clipboard() {
                            let trimmed = text.trim().to_string();
                            if !trimmed.is_empty()
                                && trimmed.len() >= 2
                                && trimmed.len() < 5000
                                && trimmed != last_text
                            {
                                last_text = trimmed.clone();
                                last_emit_time = Instant::now();
                                let _ = app.emit(
                                    "selection:detected",
                                    SelectionDetected {
                                        text: trimmed,
                                        mouse_x: rx as f64,
                                        mouse_y: ry as f64,
                                    },
                                );
                            }
                        }
                    }
                }
                was_down = down;
            }

            #[cfg(not(target_os = "windows"))]
            {
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn stop_selection_watcher() -> Result<(), String> {
    SELECTION_RUNNING.store(false, Ordering::SeqCst);
    Ok(())
}
