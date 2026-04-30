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

/// Convert physical pixel coordinates (from xcap screenshot) to logical
/// coordinates that Enigo expects.
///
/// On Windows with DPI scaling:
///   - xcap captures at physical pixel resolution (e.g. 2560x1440)
///   - GetSystemMetrics returns logical resolution (e.g. 1707x960 at 150% DPI)
///   - Enigo operates in logical coordinate space
///
/// This function dynamically reads current screen info each time
/// to handle multi-monitor and DPI changes correctly.
pub fn physical_to_logical(x: i32, y: i32) -> (i32, i32) {
    let (enigo_w, enigo_h, phys_w, phys_h) = get_coordinate_spaces();
    log::info!(
        "[input] physical_to_logical: GetSystemMetrics={}x{}, xcap_physical={}x{}, input=({},{})",
        enigo_w, enigo_h, phys_w, phys_h, x, y
    );

    // Enigo uses GetSystemMetrics internally for Coordinate::Abs.
    // In a DPI-aware process (like Tauri), GetSystemMetrics returns the
    // physical resolution, so enigo_w == phys_w and no extra scaling is needed.
    // We only need to convert if they genuinely differ.
    if enigo_w == 0 || phys_w == 0 || (enigo_w == phys_w && enigo_h == phys_h) {
        log::info!("[input] SystemMetrics==xcap ({}x{}), no DPI scaling. Passing ({},{}) to Enigo.", enigo_w, enigo_h, x, y);
        return (x, y);
    }

    let scale_x = phys_w as f64 / enigo_w as f64;
    let scale_y = phys_h as f64 / enigo_h as f64;
    let lx = (x as f64 / scale_x).round() as i32;
    let ly = (y as f64 / scale_y).round() as i32;
    log::info!(
        "[input] DPI scaling active! xcap={}x{}, SystemMetrics={}x{}, scale=({:.4},{:.4}). ({},{}) → ({},{})",
        phys_w, phys_h, enigo_w, enigo_h, scale_x, scale_y, x, y, lx, ly
    );
    (lx, ly)
}

/// Returns (enigo_screen_w, enigo_screen_h, xcap_physical_w, xcap_physical_h).
pub fn get_coordinate_spaces_pub() -> (u32, u32, u32, u32) {
    get_coordinate_spaces()
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
    let (lx, ly) = physical_to_logical(x, y);
    let (cx, cy) = get_cursor_pos();
    log::info!("[input] mouse_move: saved=({},{}), target_phys=({},{}), target_logical=({},{})", cx, cy, x, y, lx, ly);
    let mut e = new_enigo_instance()?;
    smooth_move(&mut e, cx, cy, lx, ly)?;
    Ok(())
}

#[tauri::command]
pub fn mouse_click(x: i32, y: i32, button: Option<String>) -> Result<(), String> {
    let (lx, ly) = physical_to_logical(x, y);
    let (saved_x, saved_y) = get_cursor_pos();
    log::info!("[input] mouse_click: saved=({},{}), target_phys=({},{}), target_logical=({},{})", saved_x, saved_y, x, y, lx, ly);
    let mut e = new_enigo_instance()?;
    smooth_move(&mut e, saved_x, saved_y, lx, ly)?;
    std::thread::sleep(std::time::Duration::from_millis(50));
    let btn = parse_button(button.as_deref().unwrap_or("left"));
    e.button(btn, Direction::Click)
        .map_err(|err| format!("click: {}", err))?;
    Ok(())
}

#[tauri::command]
pub fn mouse_double_click(x: i32, y: i32) -> Result<(), String> {
    let (lx, ly) = physical_to_logical(x, y);
    let (saved_x, saved_y) = get_cursor_pos();
    log::info!("[input] mouse_double_click: saved=({},{}), target=({},{})", saved_x, saved_y, lx, ly);
    let mut e = new_enigo_instance()?;
    smooth_move(&mut e, saved_x, saved_y, lx, ly)?;
    std::thread::sleep(std::time::Duration::from_millis(40));
    e.button(Button::Left, Direction::Click)
        .map_err(|err| format!("click1: {}", err))?;
    std::thread::sleep(std::time::Duration::from_millis(60));
    e.button(Button::Left, Direction::Click)
        .map_err(|err| format!("click2: {}", err))?;
    Ok(())
}

#[tauri::command]
pub fn mouse_drag(from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> Result<(), String> {
    let (fx, fy) = physical_to_logical(from_x, from_y);
    let (tx, ty) = physical_to_logical(to_x, to_y);
    let (saved_x, saved_y) = get_cursor_pos();
    log::info!("[input] mouse_drag: saved=({},{}), from=({},{}), to=({},{})", saved_x, saved_y, fx, fy, tx, ty);
    let mut e = new_enigo_instance()?;

    smooth_move(&mut e, saved_x, saved_y, fx, fy)?;
    std::thread::sleep(std::time::Duration::from_millis(50));
    e.button(Button::Left, Direction::Press)
        .map_err(|err| format!("press: {}", err))?;
    std::thread::sleep(std::time::Duration::from_millis(40));

    let dx = (tx - fx) as f64;
    let dy = (ty - fy) as f64;
    let steps = 20;
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let ease = t * t * (3.0 - 2.0 * t);
        let cx = fx + (dx * ease).round() as i32;
        let cy = fy + (dy * ease).round() as i32;
        e.move_mouse(cx, cy, Coordinate::Abs)
            .map_err(|err| format!("drag step: {}", err))?;
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    e.move_mouse(tx, ty, Coordinate::Abs)
        .map_err(|err| format!("drag final: {}", err))?;
    std::thread::sleep(std::time::Duration::from_millis(30));
    e.button(Button::Left, Direction::Release)
        .map_err(|err| format!("release: {}", err))?;
    Ok(())
}

#[tauri::command]
pub fn mouse_scroll(x: i32, y: i32, direction: String, amount: i32) -> Result<(), String> {
    let (lx, ly) = physical_to_logical(x, y);
    let (saved_x, saved_y) = get_cursor_pos();
    log::info!("[input] mouse_scroll: saved=({},{}), target=({},{})", saved_x, saved_y, lx, ly);
    let mut e = new_enigo_instance()?;
    smooth_move(&mut e, saved_x, saved_y, lx, ly)?;
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
    let (lx, ly) = physical_to_logical(test_x, test_y);

    let info = format!(
        "GetSystemMetrics: {}x{}\n\
         xcap physical: {}x{}\n\
         DPI same: {}\n\
         Input physical ({},{}) -> Enigo logical ({},{})\n\
         Scale factor: {:.3}x{:.3}",
        sys_w, sys_h,
        phys_w, phys_h,
        sys_w == phys_w && sys_h == phys_h,
        test_x, test_y, lx, ly,
        if sys_w > 0 { phys_w as f64 / sys_w as f64 } else { 1.0 },
        if sys_h > 0 { phys_h as f64 / sys_h as f64 } else { 1.0 },
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
    let (mut cur_x, mut cur_y) = get_cursor_pos();
    let mut results = Vec::new();

    for (i, step) in steps.iter().enumerate() {
        let action = step.get("action").and_then(|v| v.as_str()).unwrap_or("");
        log::info!("[action_seq] step {}: action={}, cursor=({},{})", i, action, cur_x, cur_y);

        match action {
            "move" => {
                let x = step.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let y = step.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let (lx, ly) = physical_to_logical(x, y);
                smooth_move(&mut e, cur_x, cur_y, lx, ly)?;
                cur_x = lx;
                cur_y = ly;
                results.push(format!("moved to ({},{})", lx, ly));
            }
            "click" => {
                let btn_str = step.get("button").and_then(|v| v.as_str()).unwrap_or("left");
                let btn = parse_button(btn_str);
                if let (Some(x), Some(y)) = (step.get("x").and_then(|v| v.as_i64()), step.get("y").and_then(|v| v.as_i64())) {
                    let (lx, ly) = physical_to_logical(x as i32, y as i32);
                    smooth_move(&mut e, cur_x, cur_y, lx, ly)?;
                    cur_x = lx;
                    cur_y = ly;
                }
                std::thread::sleep(std::time::Duration::from_millis(40));
                e.button(btn, Direction::Click).map_err(|er| format!("click: {}", er))?;
                results.push(format!("clicked {} at ({},{})", btn_str, cur_x, cur_y));
            }
            "press" => {
                let btn_str = step.get("button").and_then(|v| v.as_str()).unwrap_or("left");
                let btn = parse_button(btn_str);
                if let (Some(x), Some(y)) = (step.get("x").and_then(|v| v.as_i64()), step.get("y").and_then(|v| v.as_i64())) {
                    let (lx, ly) = physical_to_logical(x as i32, y as i32);
                    smooth_move(&mut e, cur_x, cur_y, lx, ly)?;
                    cur_x = lx;
                    cur_y = ly;
                }
                std::thread::sleep(std::time::Duration::from_millis(30));
                e.button(btn, Direction::Press).map_err(|er| format!("press: {}", er))?;
                results.push(format!("pressed {} at ({},{})", btn_str, cur_x, cur_y));
            }
            "release" => {
                let btn_str = step.get("button").and_then(|v| v.as_str()).unwrap_or("left");
                let btn = parse_button(btn_str);
                if let (Some(x), Some(y)) = (step.get("x").and_then(|v| v.as_i64()), step.get("y").and_then(|v| v.as_i64())) {
                    let (lx, ly) = physical_to_logical(x as i32, y as i32);
                    smooth_move(&mut e, cur_x, cur_y, lx, ly)?;
                    cur_x = lx;
                    cur_y = ly;
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
