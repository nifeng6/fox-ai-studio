use base64::Engine;
use serde::Serialize;
use std::io::Cursor;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_minimized: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenSize {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub physical_width: u32,
    pub physical_height: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorPos {
    pub x: i32,
    pub y: i32,
}

fn rgba_to_base64_png(img: xcap::image::RgbaImage) -> Result<String, String> {
    let dyn_img = xcap::image::DynamicImage::ImageRgba8(img);
    let mut buf = Vec::new();
    dyn_img
        .write_to(&mut Cursor::new(&mut buf), xcap::image::ImageFormat::Png)
        .map_err(|e| format!("PNG encode error: {}", e))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&buf))
}

fn encode_as_jpeg(img: &xcap::image::DynamicImage, quality: u8) -> Result<Vec<u8>, String> {
    let rgb = img.to_rgb8();
    let mut buf = Vec::new();
    let mut encoder = xcap::image::codecs::jpeg::JpegEncoder::new_with_quality(
        Cursor::new(&mut buf), quality
    );
    encoder.encode(
        rgb.as_raw(),
        rgb.width(),
        rgb.height(),
        xcap::image::ExtendedColorType::Rgb8,
    ).map_err(|e| format!("JPEG encode error: {}", e))?;
    Ok(buf)
}

pub struct CaptureResult {
    pub base64: String,
    pub width: u32,
    pub height: u32,
    pub resized_base64: String,
    pub resized_width: u32,
    pub resized_height: u32,
    pub grid_map: String,
}

/// Clean screenshot for the agent loop — no grid overlay, just scaled to a
/// standard resolution and JPEG-compressed for fast API transport.
pub struct CleanCapture {
    pub jpeg_base64: String,
    pub display_width: u32,
    pub display_height: u32,
    pub physical_width: u32,
    pub physical_height: u32,
    pub cursor_display_x: i32,
    pub cursor_display_y: i32,
}

pub fn capture_clean_screenshot(monitor_index: Option<u32>) -> Result<CleanCapture, String> {
    let monitors = xcap::Monitor::all().map_err(|e| format!("Monitor list: {}", e))?;
    let idx = monitor_index.unwrap_or(0) as usize;
    let monitor = monitors.get(idx).ok_or_else(|| "Monitor not found".to_string())?;
    let img = monitor.capture_image().map_err(|e| format!("Capture: {}", e))?;
    let phys_w = img.width();
    let phys_h = img.height();

    // NO scaling — keep original physical resolution for coordinate accuracy.
    // AI coordinates will be in physical pixel space, matching exactly what
    // Enigo/input.rs expects. This eliminates all coordinate conversion errors.
    let dyn_img = xcap::image::DynamicImage::ImageRgba8(img);
    let buf = encode_as_jpeg(&dyn_img, 75)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);

    // GetCursorPos returns logical coordinates; we need to convert to physical
    // pixels to match the screenshot's coordinate space.
    #[cfg(target_os = "windows")]
    let (cur_x, cur_y) = {
        let (raw_x, raw_y) = crate::commands::selection::win::cursor_pos();
        let (enigo_w, enigo_h, xcap_w, xcap_h) = crate::commands::input::get_coordinate_spaces_pub();
        if xcap_w > 0 && enigo_w > 0 && xcap_w != enigo_w {
            let sx = xcap_w as f64 / enigo_w as f64;
            let sy = xcap_h as f64 / enigo_h as f64;
            let px = (raw_x as f64 * sx).round() as i32;
            let py = (raw_y as f64 * sy).round() as i32;
            log::info!("[desktop] cursor logical({},{}) → physical({},{}) scale=({:.3},{:.3})", raw_x, raw_y, px, py, sx, sy);
            (px, py)
        } else {
            log::info!("[desktop] cursor=({},{}) no DPI scale", raw_x, raw_y);
            (raw_x, raw_y)
        }
    };
    #[cfg(not(target_os = "windows"))]
    let (cur_x, cur_y) = (0i32, 0i32);

    log::info!(
        "[desktop] clean capture: {}x{}, cursor_physical=({},{}), jpeg={}KB",
        phys_w, phys_h, cur_x, cur_y, buf.len() / 1024
    );

    Ok(CleanCapture {
        jpeg_base64: b64,
        display_width: phys_w,
        display_height: phys_h,
        physical_width: phys_w,
        physical_height: phys_h,
        cursor_display_x: cur_x,
        cursor_display_y: cur_y,
    })
}

const VISION_TARGET_WIDTH: u32 = 1280;
const GRID_COLS: u32 = 5;
const GRID_ROWS: u32 = 4;

fn draw_pixel_digit(img: &mut xcap::image::RgbaImage, x0: u32, y0: u32, digit: u8, color: xcap::image::Rgba<u8>, scale: u32) {
    #[rustfmt::skip]
    const DIGITS: [[u8; 15]; 10] = [
        [1,1,1, 1,0,1, 1,0,1, 1,0,1, 1,1,1],
        [0,1,0, 1,1,0, 0,1,0, 0,1,0, 1,1,1],
        [1,1,1, 0,0,1, 1,1,1, 1,0,0, 1,1,1],
        [1,1,1, 0,0,1, 1,1,1, 0,0,1, 1,1,1],
        [1,0,1, 1,0,1, 1,1,1, 0,0,1, 0,0,1],
        [1,1,1, 1,0,0, 1,1,1, 0,0,1, 1,1,1],
        [1,1,1, 1,0,0, 1,1,1, 1,0,1, 1,1,1],
        [1,1,1, 0,0,1, 0,1,0, 0,1,0, 0,1,0],
        [1,1,1, 1,0,1, 1,1,1, 1,0,1, 1,1,1],
        [1,1,1, 1,0,1, 1,1,1, 0,0,1, 1,1,1],
    ];
    let d = &DIGITS[digit as usize % 10];
    for row in 0..5u32 {
        for col in 0..3u32 {
            if d[(row * 3 + col) as usize] == 1 {
                for sy in 0..scale {
                    for sx in 0..scale {
                        let px = x0 + col * scale + sx;
                        let py = y0 + row * scale + sy;
                        if px < img.width() && py < img.height() {
                            img.put_pixel(px, py, color);
                        }
                    }
                }
            }
        }
    }
}

fn draw_number_at(img: &mut xcap::image::RgbaImage, cx: u32, cy: u32, num: u32, color: xcap::image::Rgba<u8>, scale: u32) {
    let s = num.to_string();
    let digit_w = 3 * scale + scale;
    let total_w = s.len() as u32 * digit_w;
    let total_h = 5 * scale;
    let start_x = cx.saturating_sub(total_w / 2);
    let start_y = cy.saturating_sub(total_h / 2);

    let bg = xcap::image::Rgba([0u8, 0, 0, 180]);
    let pad = scale;
    for by in start_y.saturating_sub(pad)..=(start_y + total_h + pad).min(img.height() - 1) {
        for bx in start_x.saturating_sub(pad)..=(start_x + total_w + pad).min(img.width() - 1) {
            let orig = img.get_pixel(bx, by);
            let a = bg[3] as u16;
            let r = ((orig[0] as u16 * (255 - a) + bg[0] as u16 * a) / 255) as u8;
            let g = ((orig[1] as u16 * (255 - a) + bg[1] as u16 * a) / 255) as u8;
            let b = ((orig[2] as u16 * (255 - a) + bg[2] as u16 * a) / 255) as u8;
            img.put_pixel(bx, by, xcap::image::Rgba([r, g, b, 255]));
        }
    }

    let mut dx = start_x;
    for ch in s.chars() {
        if let Some(d) = ch.to_digit(10) {
            draw_pixel_digit(img, dx, start_y, d as u8, color, scale);
            dx += digit_w;
        }
    }
}

/// Draw numbered grid regions on the image. Returns the grid map text.
fn draw_numbered_grid(img: &mut xcap::image::RgbaImage) -> String {
    let w = img.width();
    let h = img.height();
    let cell_w = w / GRID_COLS;
    let cell_h = h / GRID_ROWS;
    let line_color = xcap::image::Rgba([255u8, 255, 0, 160]);
    let num_color = xcap::image::Rgba([255u8, 255, 255, 255]);

    for col in 1..GRID_COLS {
        let x = col * cell_w;
        for y in 0..h {
            if x < w { img.put_pixel(x, y, line_color); }
            if x + 1 < w { img.put_pixel(x + 1, y, line_color); }
        }
    }
    for row in 1..GRID_ROWS {
        let y = row * cell_h;
        for x in 0..w {
            if y < h { img.put_pixel(x, y, line_color); }
            if y + 1 < h { img.put_pixel(x, y + 1, line_color); }
        }
    }

    let mut map = String::new();
    let mut region = 1u32;
    for row in 0..GRID_ROWS {
        for col in 0..GRID_COLS {
            let cx = col * cell_w + cell_w / 2;
            let cy = row * cell_h + cell_h / 2;
            draw_number_at(img, cx, cy, region, num_color, 5);

            let x1 = col * cell_w;
            let y1 = row * cell_h;
            let x2 = ((col + 1) * cell_w).min(w);
            let y2 = ((row + 1) * cell_h).min(h);
            map.push_str(&format!(
                "[{}] x={}-{} y={}-{} center=({},{})\n",
                region, x1, x2, y1, y2, cx, cy
            ));
            region += 1;
        }
    }
    map
}

pub fn capture_screen_with_size(monitor_index: Option<u32>) -> Result<CaptureResult, String> {
    let monitors = xcap::Monitor::all().map_err(|e| format!("Monitor list error: {}", e))?;
    let idx = monitor_index.unwrap_or(0) as usize;
    let monitor = monitors.get(idx).ok_or_else(|| "Monitor not found".to_string())?;
    let img = monitor.capture_image().map_err(|e| format!("Capture error: {}", e))?;
    let orig_w = img.width();
    let orig_h = img.height();

    let dyn_img = xcap::image::DynamicImage::ImageRgba8(img);

    let mut orig_buf = Vec::new();
    dyn_img.write_to(&mut Cursor::new(&mut orig_buf), xcap::image::ImageFormat::Png)
        .map_err(|e| format!("PNG encode error: {}", e))?;
    let orig_b64 = base64::engine::general_purpose::STANDARD.encode(&orig_buf);

    let (resized_b64, rw, rh, grid_map) = if orig_w > VISION_TARGET_WIDTH {
        let scale = VISION_TARGET_WIDTH as f64 / orig_w as f64;
        let new_h = (orig_h as f64 * scale).round() as u32;
        let resized = dyn_img.resize_exact(VISION_TARGET_WIDTH, new_h, xcap::image::imageops::FilterType::Lanczos3);
        let mut resized_rgba = resized.to_rgba8();
        let gmap = draw_numbered_grid(&mut resized_rgba);
        let grid_img = xcap::image::DynamicImage::ImageRgba8(resized_rgba);
        let buf = encode_as_jpeg(&grid_img, 75)?;
        (base64::engine::general_purpose::STANDARD.encode(&buf), VISION_TARGET_WIDTH, new_h, gmap)
    } else {
        let mut rgba_copy = dyn_img.to_rgba8();
        let gmap = draw_numbered_grid(&mut rgba_copy);
        let grid_img = xcap::image::DynamicImage::ImageRgba8(rgba_copy);
        let buf = encode_as_jpeg(&grid_img, 75)?;
        (base64::engine::general_purpose::STANDARD.encode(&buf), orig_w, orig_h, gmap)
    };

    Ok(CaptureResult {
        base64: orig_b64,
        width: orig_w,
        height: orig_h,
        resized_base64: resized_b64,
        resized_width: rw,
        resized_height: rh,
        grid_map,
    })
}

#[tauri::command]
pub fn capture_screen(monitor_index: Option<u32>) -> Result<String, String> {
    let result = capture_screen_with_size(monitor_index)?;
    Ok(result.base64)
}

#[tauri::command]
pub fn capture_window(window_id: u32) -> Result<String, String> {
    let windows = xcap::Window::all().map_err(|e| format!("Window list error: {}", e))?;
    let win = windows
        .into_iter()
        .find(|w| w.id() == window_id)
        .ok_or_else(|| "Window not found".to_string())?;
    let img = win.capture_image().map_err(|e| format!("Capture error: {}", e))?;
    rgba_to_base64_png(img)
}

#[tauri::command]
pub fn list_windows() -> Result<Vec<WindowInfo>, String> {
    let windows = xcap::Window::all().map_err(|e| format!("Window list error: {}", e))?;
    let result: Vec<WindowInfo> = windows
        .into_iter()
        .filter(|w| {
            !w.title().is_empty()
                && w.width() > 0
                && w.height() > 0
                && !w.is_minimized()
        })
        .map(|w| WindowInfo {
            id: w.id(),
            name: w.title().to_string(),
            x: w.x(),
            y: w.y(),
            width: w.width(),
            height: w.height(),
            is_minimized: w.is_minimized(),
        })
        .collect();
    Ok(result)
}

#[tauri::command]
pub fn get_screen_size(monitor_index: Option<u32>) -> Result<ScreenSize, String> {
    let monitors = xcap::Monitor::all().map_err(|e| format!("Monitor list error: {}", e))?;
    let idx = monitor_index.unwrap_or(0) as usize;
    let monitor = monitors.get(idx).ok_or_else(|| "Monitor not found".to_string())?;

    let physical_w = monitor.width();
    let physical_h = monitor.height();
    let scale = monitor.scale_factor() as f64;
    let logical_w = if scale > 0.0 { (physical_w as f64 / scale).round() as u32 } else { physical_w };
    let logical_h = if scale > 0.0 { (physical_h as f64 / scale).round() as u32 } else { physical_h };

    Ok(ScreenSize {
        width: logical_w,
        height: logical_h,
        scale_factor: scale,
        physical_width: physical_w,
        physical_height: physical_h,
    })
}

#[tauri::command]
pub fn get_cursor_position() -> Result<CursorPos, String> {
    #[cfg(target_os = "windows")]
    {
        use std::mem::MaybeUninit;
        #[repr(C)]
        struct POINT { x: i32, y: i32 }
        extern "system" { fn GetCursorPos(lp_point: *mut POINT) -> i32; }
        let mut point = MaybeUninit::<POINT>::uninit();
        let ok = unsafe { GetCursorPos(point.as_mut_ptr()) };
        if ok != 0 {
            let p = unsafe { point.assume_init() };
            Ok(CursorPos { x: p.x, y: p.y })
        } else {
            Err("GetCursorPos failed".to_string())
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(CursorPos { x: 0, y: 0 })
    }
}

#[tauri::command]
pub fn read_file_base64(path: String) -> Result<String, String> {
    let data = std::fs::read(&path).map_err(|e| format!("Read error: {}", e))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&data))
}

/// Get clickable UI elements from screen using Windows Accessibility (MSAA/EnumChildWindows).
/// Returns a text description with element names and positions.
#[tauri::command]
pub fn get_screen_elements() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;

        extern "system" {
            fn GetDesktopWindow() -> *mut c_void;
            fn EnumChildWindows(
                hWndParent: *mut c_void,
                lpEnumFunc: unsafe extern "system" fn(*mut c_void, isize) -> i32,
                lParam: isize,
            ) -> i32;
            fn IsWindowVisible(hWnd: *mut c_void) -> i32;
            fn GetWindowTextW(hWnd: *mut c_void, lpString: *mut u16, nMaxCount: i32) -> i32;
            fn GetWindowRect(hWnd: *mut c_void, lpRect: *mut WinRect) -> i32;
            fn GetClassNameW(hWnd: *mut c_void, lpClassName: *mut u16, nMaxCount: i32) -> i32;
            fn GetCurrentProcessId() -> u32;
        }

        // Also use GetWindowThreadProcessId via dynamic lookup to avoid FFI conflict
        type GWTPIDFn = unsafe extern "system" fn(*mut c_void, *mut u32) -> u32;
        fn get_gwtpid() -> Option<GWTPIDFn> {
            extern "system" {
                fn LoadLibraryA(lpLibFileName: *const u8) -> *mut c_void;
                fn GetProcAddress(hModule: *mut c_void, lpProcName: *const u8) -> *mut c_void;
            }
            unsafe {
                let lib = LoadLibraryA(b"user32.dll\0".as_ptr());
                if lib.is_null() { return None; }
                let proc = GetProcAddress(lib, b"GetWindowThreadProcessId\0".as_ptr());
                if proc.is_null() { return None; }
                Some(std::mem::transmute(proc))
            }
        }

        #[repr(C)]
        struct WinRect { left: i32, top: i32, right: i32, bottom: i32 }

        struct ElementCollector {
            elements: Vec<String>,
            own_pid: u32,
            gwtpid: Option<GWTPIDFn>,
        }

        static COLLECTOR: std::sync::Mutex<Option<ElementCollector>> = std::sync::Mutex::new(None);

        unsafe extern "system" fn enum_cb(hwnd: *mut c_void, _lparam: isize) -> i32 {
            if IsWindowVisible(hwnd) == 0 { return 1; }

            let mut collector = COLLECTOR.lock().unwrap();
            let col = match collector.as_mut() { Some(c) => c, None => return 0 };

            // Skip our own windows
            if let Some(func) = col.gwtpid {
                let mut pid: u32 = 0;
                func(hwnd, &mut pid);
                if pid == col.own_pid { return 1; }
            }

            let mut title_buf = [0u16; 256];
            let title_len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 256);
            let title = if title_len > 0 {
                String::from_utf16_lossy(&title_buf[..title_len as usize])
            } else {
                return 1; // skip unnamed windows
            };

            if title.is_empty() || title.len() > 100 { return 1; }

            let mut class_buf = [0u16; 128];
            let class_len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), 128);
            let _class_name = if class_len > 0 {
                String::from_utf16_lossy(&class_buf[..class_len as usize])
            } else {
                String::new()
            };

            let mut rect = WinRect { left: 0, top: 0, right: 0, bottom: 0 };
            GetWindowRect(hwnd, &mut rect);

            if rect.right - rect.left < 10 || rect.bottom - rect.top < 10 { return 1; }

            let cx = (rect.left + rect.right) / 2;
            let cy = (rect.top + rect.bottom) / 2;

            col.elements.push(format!(
                "\"{}\" at ({},{}) size {}x{}",
                title, cx, cy,
                rect.right - rect.left, rect.bottom - rect.top
            ));

            if col.elements.len() >= 50 { return 0; } // limit
            1
        }

        let own_pid = unsafe { GetCurrentProcessId() };
        let gwtpid = get_gwtpid();

        {
            let mut col = COLLECTOR.lock().map_err(|e| e.to_string())?;
            *col = Some(ElementCollector {
                elements: Vec::new(),
                own_pid,
                gwtpid,
            });
        }

        let desktop = unsafe { GetDesktopWindow() };
        unsafe { EnumChildWindows(desktop, enum_cb, 0); }

        let mut col = COLLECTOR.lock().map_err(|e| e.to_string())?;
        let elements = col.take().map(|c| c.elements).unwrap_or_default();

        Ok(elements.join("\n"))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok("Accessibility API not available on this platform".to_string())
    }
}

#[tauri::command]
pub fn write_debug_log(filename: String, content: String) -> Result<String, String> {
    let desktop = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
    let debug_dir = std::path::PathBuf::from(&desktop).join("Desktop").join("fox-ai-screenshots");
    std::fs::create_dir_all(&debug_dir).map_err(|e| format!("mkdir error: {}", e))?;
    let path = debug_dir.join(&filename);
    std::fs::write(&path, &content).map_err(|e| format!("write error: {}", e))?;
    let p = path.display().to_string();
    log::info!("[debug-log] saved: {}", p);
    Ok(p)
}
