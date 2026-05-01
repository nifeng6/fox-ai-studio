use enigo::{Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};

pub fn new_enigo_instance() -> Result<Enigo, String> {
    Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init error: {}", e))
}

/// Get current cursor position via system API (logical coordinates).
fn get_cursor_pos() -> (i32, i32) {
    #[cfg(target_os = "windows")]
    {
        return crate::commands::selection::win::cursor_pos();
    }
    #[cfg(not(target_os = "windows"))]
    {
        (0, 0)
    }
}

/// Smooth mouse movement using smoothstep easing — simulates human cursor motion.
fn smooth_move(e: &mut Enigo, from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> Result<(), String> {
    let dx = (to_x - from_x) as f64;
    let dy = (to_y - from_y) as f64;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 3.0 {
        e.move_mouse(to_x, to_y, Coordinate::Abs)
            .map_err(|er| format!("move: {}", er))?;
        return Ok(());
    }

    let steps = ((dist / 6.0).clamp(10.0, 50.0)) as u32;
    let step_ms = if dist > 400.0 { 3 } else { 6 };

    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let ease = t * t * (3.0 - 2.0 * t);
        let ix = from_x + (dx * ease).round() as i32;
        let iy = from_y + (dy * ease).round() as i32;
        e.move_mouse(ix, iy, Coordinate::Abs)
            .map_err(|er| format!("smooth step: {}", er))?;
        std::thread::sleep(std::time::Duration::from_millis(step_ms));
    }
    e.move_mouse(to_x, to_y, Coordinate::Abs)
        .map_err(|er| format!("final move: {}", er))?;
    Ok(())
}

/// Public wrapper for smooth_move, used by tools.rs
pub fn smooth_move_pub(e: &mut Enigo, from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> Result<(), String> {
    smooth_move(e, from_x, from_y, to_x, to_y)
}

// No restore — cursor stays at target after each action

/// Returns (enigo_screen_w, enigo_screen_h, xcap_physical_w, xcap_physical_h).
pub fn get_coordinate_spaces_pub() -> (u32, u32, u32, u32) {
    get_coordinate_spaces()
}

/// Get the actual DPI scale factor of the primary monitor.
/// Returns (scale_x, scale_y) where scale > 1.0 means physical > logical.
///
/// CRITICAL: On Windows 10/11 with display scaling (e.g. 125%, 150%):
/// - In a DPI-aware process (which Tauri 2 is by default), GetSystemMetrics
///   returns PHYSICAL pixel dimensions (matching xcap).
/// - But GetCursorPos() and GetWindowRect() return LOGICAL coordinates
///   (scaled down by the DPI factor).
/// - xcap screenshots are always in PHYSICAL pixels.
/// - Enigo's Coordinate::Abs uses PHYSICAL pixels in a DPI-aware process.
///
/// So the correct approach is:
///   1. AI coordinates come from physical-pixel screenshots → already physical
///   2. GetCursorPos() returns logical → must convert to physical
///   3. Enigo move_mouse(Abs) expects physical → no conversion needed
///   4. GetWindowRect returns logical → must convert to physical
pub fn get_dpi_scale() -> (f64, f64) {
    #[cfg(target_os = "windows")]
    {
        // Use the actual monitor DPI via GetDeviceCaps to get the true scale
        extern "system" {
            fn GetDC(hwnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
            fn ReleaseDC(hwnd: *mut std::ffi::c_void, hdc: *mut std::ffi::c_void) -> i32;
            fn GetDeviceCaps(hdc: *mut std::ffi::c_void, index: i32) -> i32;
        }
        const LOGPIXELSX: i32 = 88;
        const LOGPIXELSY: i32 = 90;

        let hdc = unsafe { GetDC(std::ptr::null_mut()) };
        if hdc.is_null() {
            log::warn!("[input] GetDC failed, assuming 1.0 DPI scale");
            return (1.0, 1.0);
        }
        let dpi_x = unsafe { GetDeviceCaps(hdc, LOGPIXELSX) } as f64;
        let dpi_y = unsafe { GetDeviceCaps(hdc, LOGPIXELSY) } as f64;
        unsafe { ReleaseDC(std::ptr::null_mut(), hdc); }

        let scale_x = dpi_x / 96.0;
        let scale_y = dpi_y / 96.0;
        log::info!("[input] DPI scale from GetDeviceCaps: ({:.3}, {:.3}) (DPI={:.0}x{:.0})", scale_x, scale_y, dpi_x, dpi_y);
        (scale_x, scale_y)
    }
    #[cfg(not(target_os = "windows"))]
    {
        (1.0, 1.0)
    }
}

/// Convert logical coordinates (from GetCursorPos/GetWindowRect) to
/// physical pixel coordinates (matching xcap screenshots).
pub fn logical_to_physical(x: i32, y: i32) -> (i32, i32) {
    let (sx, sy) = get_dpi_scale();
    if sx == 1.0 && sy == 1.0 {
        return (x, y);
    }
    let px = (x as f64 * sx).round() as i32;
    let py = (y as f64 * sy).round() as i32;
    log::info!("[input] logical_to_physical: ({},{}) → ({},{}) scale=({:.3},{:.3})", x, y, px, py, sx, sy);
    (px, py)
}

/// Convert physical pixel coordinates (from xcap screenshots / AI) to
/// logical coordinates (for APIs that expect logical, like GetCursorPos).
pub fn physical_to_logical(x: i32, y: i32) -> (i32, i32) {
    let (sx, sy) = get_dpi_scale();
    if sx == 1.0 && sy == 1.0 {
        return (x, y);
    }
    let lx = (x as f64 / sx).round() as i32;
    let ly = (y as f64 / sy).round() as i32;
    log::info!("[input] physical_to_logical: ({},{}) → ({},{}) scale=({:.3},{:.3})", x, y, lx, ly, sx, sy);
    (lx, ly)
}

/// Get current cursor position in PHYSICAL pixel coordinates.
/// This is the correct coordinate space for xcap screenshots and Enigo Abs.
pub fn get_cursor_pos_physical() -> (i32, i32) {
    #[cfg(target_os = "windows")]
    {
        let (lx, ly) = crate::commands::selection::win::cursor_pos();
        logical_to_physical(lx, ly)
    }
    #[cfg(not(target_os = "windows"))]
    {
        (0, 0)
    }
}

fn get_coordinate_spaces() -> (u32, u32, u32, u32) {
    #[cfg(target_os = "windows")]
    {
        extern "system" {
            fn GetSystemMetrics(nIndex: i32) -> i32;
        }
        const SM_CXSCREEN: i32 = 0;
        const SM_CYSCREEN: i32 = 1;

        let sys_w = unsafe { GetSystemMetrics(SM_CXSCREEN) } as u32;
        let sys_h = unsafe { GetSystemMetrics(SM_CYSCREEN) } as u32;

        let (phys_w, phys_h) = xcap::Monitor::all().ok().and_then(|m| {
            m.first().map(|mon| (mon.width(), mon.height()))
        }).unwrap_or((sys_w, sys_h));

        (sys_w, sys_h, phys_w, phys_h)
    }
    #[cfg(not(target_os = "windows"))]
    {
        (0u32, 0u32, 0u32, 0u32)
    }
}

fn parse_button(btn: &str) -> Button {
    match btn {
        "right" => Button::Right,
        "middle" => Button::Middle,
        _ => Button::Left,
    }
}

#[tauri::command]
pub fn mouse_move(x: i32, y: i32) -> Result<(), String> {
    // x, y come from AI as physical pixel coordinates (matching screenshot)
    // Enigo Coordinate::Abs in DPI-aware process also uses physical coordinates
    // So we pass them directly, but must convert cursor pos to physical first
    let (cx, cy) = get_cursor_pos_physical();
    log::info!("[input] mouse_move: cursor_phys=({},{}), target_phys=({},{})", cx, cy, x, y);
    let mut e = new_enigo_instance()?;
    smooth_move(&mut e, cx, cy, x, y)?;
    Ok(())
}

#[tauri::command]
pub fn mouse_click(x: i32, y: i32, button: Option<String>) -> Result<(), String> {
    let (cx, cy) = get_cursor_pos_physical();
    log::info!("[input] mouse_click: cursor_phys=({},{}), target_phys=({},{})", cx, cy, x, y);
    let mut e = new_enigo_instance()?;
    smooth_move(&mut e, cx, cy, x, y)?;
    // Wait for mouse to settle at target position
    std::thread::sleep(std::time::Duration::from_millis(60));
    let btn = parse_button(button.as_deref().unwrap_or("left"));
    e.button(btn, Direction::Click)
        .map_err(|err| format!("click: {}", err))?;
    // Verify the click landed
    std::thread::sleep(std::time::Duration::from_millis(30));
    let (final_x, final_y) = get_cursor_pos_physical();
    log::info!("[input] mouse_click: final cursor=({},{})", final_x, final_y);
    Ok(())
}

#[tauri::command]
pub fn mouse_double_click(x: i32, y: i32) -> Result<(), String> {
    let (cx, cy) = get_cursor_pos_physical();
    log::info!("[input] mouse_double_click: cursor_phys=({},{}), target_phys=({},{})", cx, cy, x, y);
    let mut e = new_enigo_instance()?;
    smooth_move(&mut e, cx, cy, x, y)?;
    // Wait longer for mouse to settle before clicking
    std::thread::sleep(std::time::Duration::from_millis(80));
    e.button(Button::Left, Direction::Click)
        .map_err(|err| format!("click1: {}", err))?;
    // Windows double-click requires ~100ms gap between clicks
    std::thread::sleep(std::time::Duration::from_millis(100));
    e.button(Button::Left, Direction::Click)
        .map_err(|err| format!("click2: {}", err))?;
    // Verify cursor landed at target
    std::thread::sleep(std::time::Duration::from_millis(50));
    let (final_x, final_y) = get_cursor_pos_physical();
    log::info!("[input] mouse_double_click: final cursor=({},{})", final_x, final_y);
    Ok(())
}

#[tauri::command]
pub fn mouse_drag(from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> Result<(), String> {
    // All coordinates are already physical pixels — pass directly to Enigo
    let (cx, cy) = get_cursor_pos_physical();
    log::info!("[input] mouse_drag: cursor_phys=({},{}), from=({},{}), to=({},{})", cx, cy, from_x, from_y, to_x, to_y);
    let mut e = new_enigo_instance()?;

    smooth_move(&mut e, cx, cy, from_x, from_y)?;
    std::thread::sleep(std::time::Duration::from_millis(50));
    e.button(Button::Left, Direction::Press)
        .map_err(|err| format!("press: {}", err))?;
    std::thread::sleep(std::time::Duration::from_millis(40));

    let dx = (to_x - from_x) as f64;
    let dy = (to_y - from_y) as f64;
    let steps = 20;
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let ease = t * t * (3.0 - 2.0 * t);
        let ix = from_x + (dx * ease).round() as i32;
        let iy = from_y + (dy * ease).round() as i32;
        e.move_mouse(ix, iy, Coordinate::Abs)
            .map_err(|err| format!("drag step: {}", err))?;
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    e.move_mouse(to_x, to_y, Coordinate::Abs)
        .map_err(|err| format!("drag final: {}", err))?;
    std::thread::sleep(std::time::Duration::from_millis(30));
    e.button(Button::Left, Direction::Release)
        .map_err(|err| format!("release: {}", err))?;
    Ok(())
}

#[tauri::command]
pub fn mouse_scroll(x: i32, y: i32, direction: String, amount: i32) -> Result<(), String> {
    let (cx, cy) = get_cursor_pos_physical();
    log::info!("[input] mouse_scroll: cursor_phys=({},{}), target_phys=({},{})", cx, cy, x, y);
    let mut e = new_enigo_instance()?;
    smooth_move(&mut e, cx, cy, x, y)?;
    std::thread::sleep(std::time::Duration::from_millis(30));

    let scroll_amount = match direction.as_str() {
        "up" => amount,
        "down" => -amount,
        _ => -amount,
    };
    e.scroll(scroll_amount, enigo::Axis::Vertical)
        .map_err(|err| format!("scroll: {}", err))?;
    Ok(())
}

#[tauri::command]
pub fn keyboard_type(text: String) -> Result<(), String> {
    let mut e = new_enigo_instance()?;
    e.text(&text)
        .map_err(|err| format!("type error: {}", err))
}

fn parse_key(key: &str) -> Key {
    match key.to_lowercase().as_str() {
        "enter" | "return" => Key::Return,
        "tab" => Key::Tab,
        "space" => Key::Space,
        "backspace" => Key::Backspace,
        "delete" => Key::Delete,
        "escape" | "esc" => Key::Escape,
        "up" | "arrowup" => Key::UpArrow,
        "down" | "arrowdown" => Key::DownArrow,
        "left" | "arrowleft" => Key::LeftArrow,
        "right" | "arrowright" => Key::RightArrow,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "shift" => Key::Shift,
        "control" | "ctrl" => Key::Control,
        "alt" => Key::Alt,
        "meta" | "super" | "win" | "command" => Key::Meta,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        other => {
            let chars: Vec<char> = other.chars().collect();
            if chars.len() == 1 {
                Key::Unicode(chars[0])
            } else {
                Key::Unicode(' ')
            }
        }
    }
}

#[tauri::command]
pub fn keyboard_key(key: String, modifiers: Option<Vec<String>>) -> Result<(), String> {
    let mut e = new_enigo_instance()?;
    let mods = modifiers.unwrap_or_default();
    for m in &mods {
        e.key(parse_key(m), Direction::Press)
            .map_err(|err| format!("mod press error: {}", err))?;
    }
    e.key(parse_key(&key), Direction::Click)
        .map_err(|err| format!("key click error: {}", err))?;
    for m in mods.iter().rev() {
        e.key(parse_key(m), Direction::Release)
            .map_err(|err| format!("mod release error: {}", err))?;
    }
    Ok(())
}

#[tauri::command]
pub fn debug_coordinate_info(test_x: i32, test_y: i32) -> Result<String, String> {
    let (sys_w, sys_h, phys_w, phys_h) = get_coordinate_spaces();
    let (dpi_sx, dpi_sy) = get_dpi_scale();
    let (lx, ly) = physical_to_logical(test_x, test_y);
    let (cursor_lx, cursor_ly) = get_cursor_pos();
    let (cursor_px, cursor_py) = get_cursor_pos_physical();

    let info = format!(
        "=== Fox AI Coordinate Debug ===\n\
         Screen: GetSystemMetrics={}x{}, xcap_physical={}x{}\n\
         DPI scale: {:.3}x{:.3} (from GetDeviceCaps)\n\
         Physical ({},{}) → Logical ({},{})\n\
         Cursor logical: ({},{}) → physical: ({},{})\n\
         Enigo uses: physical coordinates (DPI-aware process)",
        sys_w, sys_h, phys_w, phys_h,
        dpi_sx, dpi_sy,
        test_x, test_y, lx, ly,
        cursor_lx, cursor_ly, cursor_px, cursor_py,
    );
    log::info!("[debug_coordinate_info]\n{}", info);
    Ok(info)
}

#[tauri::command]
pub fn keyboard_hotkey(keys: Vec<String>) -> Result<(), String> {
    let mut e = new_enigo_instance()?;
    for k in &keys {
        e.key(parse_key(k), Direction::Press)
            .map_err(|err| format!("hotkey press error: {}", err))?;
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    for k in keys.iter().rev() {
        e.key(parse_key(k), Direction::Release)
            .map_err(|err| format!("hotkey release error: {}", err))?;
    }
    Ok(())
}

/// Execute a sequence of actions atomically with smooth transitions.
/// Each step is: { "action": "move"|"click"|"press"|"release"|"type"|"key"|"wait", ...params }
#[tauri::command]
pub fn action_sequence(steps: Vec<serde_json::Value>) -> Result<String, String> {
    let mut e = new_enigo_instance()?;
    let (mut cur_x, mut cur_y) = get_cursor_pos_physical();
    let mut results = Vec::new();

    for (i, step) in steps.iter().enumerate() {
        let action = step.get("action").and_then(|v| v.as_str()).unwrap_or("");
        log::info!("[action_seq] step {}: action={}, cursor_phys=({},{})", i, action, cur_x, cur_y);

        match action {
            "move" => {
                let x = step.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let y = step.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                // x, y are physical pixels from AI — pass directly
                smooth_move(&mut e, cur_x, cur_y, x, y)?;
                cur_x = x;
                cur_y = y;
                results.push(format!("moved to ({},{})", x, y));
            }
            "click" => {
                let btn_str = step.get("button").and_then(|v| v.as_str()).unwrap_or("left");
                let btn = parse_button(btn_str);
                if let (Some(x), Some(y)) = (step.get("x").and_then(|v| v.as_i64()), step.get("y").and_then(|v| v.as_i64())) {
                    let x = x as i32;
                    let y = y as i32;
                    smooth_move(&mut e, cur_x, cur_y, x, y)?;
                    cur_x = x;
                    cur_y = y;
                }
                std::thread::sleep(std::time::Duration::from_millis(40));
                e.button(btn, Direction::Click).map_err(|er| format!("click: {}", er))?;
                results.push(format!("clicked {} at ({},{})", btn_str, cur_x, cur_y));
            }
            "press" => {
                let btn_str = step.get("button").and_then(|v| v.as_str()).unwrap_or("left");
                let btn = parse_button(btn_str);
                if let (Some(x), Some(y)) = (step.get("x").and_then(|v| v.as_i64()), step.get("y").and_then(|v| v.as_i64())) {
                    let x = x as i32;
                    let y = y as i32;
                    smooth_move(&mut e, cur_x, cur_y, x, y)?;
                    cur_x = x;
                    cur_y = y;
                }
                std::thread::sleep(std::time::Duration::from_millis(30));
                e.button(btn, Direction::Press).map_err(|er| format!("press: {}", er))?;
                results.push(format!("pressed {} at ({},{})", btn_str, cur_x, cur_y));
            }
            "release" => {
                let btn_str = step.get("button").and_then(|v| v.as_str()).unwrap_or("left");
                let btn = parse_button(btn_str);
                if let (Some(x), Some(y)) = (step.get("x").and_then(|v| v.as_i64()), step.get("y").and_then(|v| v.as_i64())) {
                    let x = x as i32;
                    let y = y as i32;
                    smooth_move(&mut e, cur_x, cur_y, x, y)?;
                    cur_x = x;
                    cur_y = y;
                }
                e.button(btn, Direction::Release).map_err(|er| format!("release: {}", er))?;
                results.push(format!("released {} at ({},{})", btn_str, cur_x, cur_y));
            }
            "type" => {
                let text = step.get("text").and_then(|v| v.as_str()).unwrap_or("");
                e.text(text).map_err(|er| format!("type: {}", er))?;
                results.push(format!("typed '{}'", if text.len() > 30 { &text[..30] } else { text }));
            }
            "key" => {
                let key_str = step.get("key").and_then(|v| v.as_str()).unwrap_or("");
                let mods: Vec<String> = step.get("modifiers")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|m| m.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                for m in &mods {
                    e.key(parse_key(m), Direction::Press).map_err(|er| format!("mod press: {}", er))?;
                }
                e.key(parse_key(key_str), Direction::Click).map_err(|er| format!("key: {}", er))?;
                for m in mods.iter().rev() {
                    e.key(parse_key(m), Direction::Release).map_err(|er| format!("mod release: {}", er))?;
                }
                results.push(format!("key '{}'", key_str));
            }
            "wait" => {
                let ms = step.get("ms").and_then(|v| v.as_u64()).unwrap_or(200);
                std::thread::sleep(std::time::Duration::from_millis(ms));
                results.push(format!("waited {}ms", ms));
            }
            _ => {
                results.push(format!("unknown action '{}'", action));
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(30));
    }

    Ok(results.join("; "))
}
