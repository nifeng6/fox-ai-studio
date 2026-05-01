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

/// Draw coordinate ruler marks along the edges of the screenshot,
/// plus subtle full-screen grid lines for better coordinate estimation.
/// This helps the AI model estimate coordinates more accurately.
fn draw_coordinate_ruler(img: &mut xcap::image::RgbaImage) {
    let w = img.width();
    let h = img.height();
    
    // Ruler colors - bright and opaque
    let ruler_color = xcap::image::Rgba([0u8, 255, 0, 200]); // bright green, more opaque
    let text_color = xcap::image::Rgba([255u8, 255, 255, 255]); // pure white
    let bg_color = xcap::image::Rgba([0u8, 0, 0, 200]); // darker background
    let minor_tick_color = xcap::image::Rgba([0u8, 200, 0, 150]); // minor ticks
    let grid_major_color = xcap::image::Rgba([255u8, 255, 0, 40]); // subtle yellow major grid
    let grid_minor_color = xcap::image::Rgba([255u8, 255, 0, 15]); // very subtle yellow minor grid
    
    // Use a scale of 2 for more visible digits
    let scale = 2u32;
    
    // Draw full-screen grid lines FIRST (before edge rulers, so rulers overlay grid)
    let grid_interval = if w > 2000 { 400 } else { 200 };
    let grid_minor_interval = grid_interval / 2;
    
    // Minor vertical grid lines (every half-interval)
    let mut gx = grid_minor_interval;
    while gx < w as i32 {
        let ux = gx as u32;
        for gy in 0..h {
            if ux < w && gy < h {
                let pixel = img.get_pixel(ux, gy);
                let a = grid_minor_color[3] as u16;
                let r = ((pixel[0] as u16 * (255 - a) + grid_minor_color[0] as u16 * a) / 255) as u8;
                let g = ((pixel[1] as u16 * (255 - a) + grid_minor_color[1] as u16 * a) / 255) as u8;
                let b = ((pixel[2] as u16 * (255 - a) + grid_minor_color[2] as u16 * a) / 255) as u8;
                img.put_pixel(ux, gy, xcap::image::Rgba([r, g, b, 255]));
            }
        }
        gx += grid_minor_interval;
    }
    
    // Minor horizontal grid lines
    let mut gy = grid_minor_interval;
    while gy < h as i32 {
        let uy = gy as u32;
        for gx in 0..w {
            if gx < w && uy < h {
                let pixel = img.get_pixel(gx, uy);
                let a = grid_minor_color[3] as u16;
                let r = ((pixel[0] as u16 * (255 - a) + grid_minor_color[0] as u16 * a) / 255) as u8;
                let g = ((pixel[1] as u16 * (255 - a) + grid_minor_color[1] as u16 * a) / 255) as u8;
                let b = ((pixel[2] as u16 * (255 - a) + grid_minor_color[2] as u16 * a) / 255) as u8;
                img.put_pixel(gx, uy, xcap::image::Rgba([r, g, b, 255]));
            }
        }
        gy += grid_minor_interval;
    }
    
    // Major vertical grid lines
    let mut gx = grid_interval;
    while gx < w as i32 {
        let ux = gx as u32;
        for gy in 0..h {
            if ux < w && gy < h {
                let pixel = img.get_pixel(ux, gy);
                let a = grid_major_color[3] as u16;
                let r = ((pixel[0] as u16 * (255 - a) + grid_major_color[0] as u16 * a) / 255) as u8;
                let g = ((pixel[1] as u16 * (255 - a) + grid_major_color[1] as u16 * a) / 255) as u8;
                let b = ((pixel[2] as u16 * (255 - a) + grid_major_color[2] as u16 * a) / 255) as u8;
                img.put_pixel(ux, gy, xcap::image::Rgba([r, g, b, 255]));
            }
        }
        gx += grid_interval;
    }
    
    // Major horizontal grid lines
    let mut gy = grid_interval;
    while gy < h as i32 {
        let uy = gy as u32;
        for gx in 0..w {
            if gx < w && uy < h {
                let pixel = img.get_pixel(gx, uy);
                let a = grid_major_color[3] as u16;
                let r = ((pixel[0] as u16 * (255 - a) + grid_major_color[0] as u16 * a) / 255) as u8;
                let g = ((pixel[1] as u16 * (255 - a) + grid_major_color[1] as u16 * a) / 255) as u8;
                let b = ((pixel[2] as u16 * (255 - a) + grid_major_color[2] as u16 * a) / 255) as u8;
                img.put_pixel(gx, uy, xcap::image::Rgba([r, g, b, 255]));
            }
        }
        gy += grid_interval;
    }
    
    // Draw coordinate labels at grid intersections
    let mut lx = grid_interval;
    while lx < w as i32 {
        let mut ly = grid_interval;
        while ly < h as i32 {
            let label = format!("({},{})", lx, ly);
            let label_len = label.len() as u32;
            let digit_w = 3 * scale + scale;
            let total_w = label_len * digit_w;
            let total_h = 5 * scale;
            let start_x = (lx as u32 + 3).min(w - total_w - 1);
            let start_y = (ly as u32 + 3).min(h - total_h - 1);
            
            // Draw background
            let pad = scale;
            for by in start_y.saturating_sub(pad)..=(start_y + total_h + pad).min(h - 1) {
                for bx in start_x.saturating_sub(pad)..=(start_x + total_w + pad).min(w - 1) {
                    let orig = img.get_pixel(bx, by);
                    let a = bg_color[3] as u16;
                    let r = ((orig[0] as u16 * (255 - a) + bg_color[0] as u16 * a) / 255) as u8;
                    let g = ((orig[1] as u16 * (255 - a) + bg_color[1] as u16 * a) / 255) as u8;
                    let b = ((orig[2] as u16 * (255 - a) + bg_color[2] as u16 * a) / 255) as u8;
                    img.put_pixel(bx, by, xcap::image::Rgba([r, g, b, 255]));
                }
            }
            
            // Draw coordinate label in yellow
            let label_color = xcap::image::Rgba([255u8, 255, 0, 255]);
            let mut dx = start_x;
            for ch in label.chars() {
                if let Some(d) = ch.to_digit(10) {
                    draw_pixel_digit(img, dx, start_y, d as u8, label_color, scale);
                    dx += digit_w;
                } else {
                    // Draw comma/paren as a single pixel
                    dx += digit_w / 2;
                }
            }
            
            ly += grid_interval;
        }
        lx += grid_interval;
    }
    
    // Now draw edge rulers (overlay on top of grid)
    let label_interval = if w > 2000 { 400 } else if w > 1000 { 200 } else { 100 };
    let tick_interval = label_interval / 2;
    
    // Draw minor ticks along the top edge (X axis)
    let mut x = tick_interval;
    while x < w as i32 {
        let ux = x as u32;
        for dy in 0..4u32 {
            if ux < w && dy < h {
                let pixel = img.get_pixel(ux, dy);
                let a = minor_tick_color[3] as u16;
                let r = ((pixel[0] as u16 * (255 - a) + minor_tick_color[0] as u16 * a) / 255) as u8;
                let g = ((pixel[1] as u16 * (255 - a) + minor_tick_color[1] as u16 * a) / 255) as u8;
                let b = ((pixel[2] as u16 * (255 - a) + minor_tick_color[2] as u16 * a) / 255) as u8;
                img.put_pixel(ux, dy, xcap::image::Rgba([r, g, b, 255]));
            }
        }
        x += tick_interval;
    }
    
    // Draw major ticks and number labels along the top edge (X axis)
    let mut x = label_interval;
    while x < w as i32 {
        let ux = x as u32;
        // Major tick line (taller)
        for dy in 0..8u32 {
            if ux < w && dy < h {
                let pixel = img.get_pixel(ux, dy);
                let a = ruler_color[3] as u16;
                let r = ((pixel[0] as u16 * (255 - a) + ruler_color[0] as u16 * a) / 255) as u8;
                let g = ((pixel[1] as u16 * (255 - a) + ruler_color[1] as u16 * a) / 255) as u8;
                let b = ((pixel[2] as u16 * (255 - a) + ruler_color[2] as u16 * a) / 255) as u8;
                img.put_pixel(ux, dy, xcap::image::Rgba([r, g, b, 255]));
            }
        }
        
        // Number label background
        let label = x.to_string();
        let label_len = label.len() as u32;
        let digit_w = 3 * scale + scale;
        let total_w = label_len * digit_w;
        let total_h = 5 * scale;
        let start_x = ux.saturating_sub(total_w / 2);
        let start_y = 8u32;
        
        let pad = scale;
        for by in start_y.saturating_sub(pad)..=(start_y + total_h + pad).min(h - 1) {
            for bx in start_x.saturating_sub(pad)..=(start_x + total_w + pad).min(w - 1) {
                let orig = img.get_pixel(bx, by);
                let a = bg_color[3] as u16;
                let r = ((orig[0] as u16 * (255 - a) + bg_color[0] as u16 * a) / 255) as u8;
                let g = ((orig[1] as u16 * (255 - a) + bg_color[1] as u16 * a) / 255) as u8;
                let b = ((orig[2] as u16 * (255 - a) + bg_color[2] as u16 * a) / 255) as u8;
                img.put_pixel(bx, by, xcap::image::Rgba([r, g, b, 255]));
            }
        }
        
        let mut dx = start_x;
        for ch in label.chars() {
            if let Some(d) = ch.to_digit(10) {
                draw_pixel_digit(img, dx, start_y, d as u8, text_color, scale);
                dx += digit_w;
            }
        }
        
        x += label_interval;
    }
    
    // Draw minor ticks along the left edge (Y axis)
    let mut y = tick_interval;
    while y < h as i32 {
        let uy = y as u32;
        for dx in 0..4u32 {
            if dx < w && uy < h {
                let pixel = img.get_pixel(dx, uy);
                let a = minor_tick_color[3] as u16;
                let r = ((pixel[0] as u16 * (255 - a) + minor_tick_color[0] as u16 * a) / 255) as u8;
                let g = ((pixel[1] as u16 * (255 - a) + minor_tick_color[1] as u16 * a) / 255) as u8;
                let b = ((pixel[2] as u16 * (255 - a) + minor_tick_color[2] as u16 * a) / 255) as u8;
                img.put_pixel(dx, uy, xcap::image::Rgba([r, g, b, 255]));
            }
        }
        y += tick_interval;
    }
    
    // Draw major ticks and number labels along the left edge (Y axis)
    let mut y = label_interval;
    while y < h as i32 {
        let uy = y as u32;
        for dx in 0..8u32 {
            if dx < w && uy < h {
                let pixel = img.get_pixel(dx, uy);
                let a = ruler_color[3] as u16;
                let r = ((pixel[0] as u16 * (255 - a) + ruler_color[0] as u16 * a) / 255) as u8;
                let g = ((pixel[1] as u16 * (255 - a) + ruler_color[1] as u16 * a) / 255) as u8;
                let b = ((pixel[2] as u16 * (255 - a) + ruler_color[2] as u16 * a) / 255) as u8;
                img.put_pixel(dx, uy, xcap::image::Rgba([r, g, b, 255]));
            }
        }
        
        let label = y.to_string();
        let label_len = label.len() as u32;
        let digit_w = 3 * scale + scale;
        let total_w = label_len * digit_w;
        let total_h = 5 * scale;
        let start_x = 8u32;
        let start_y = uy.saturating_sub(total_h / 2);
        
        let pad = scale;
        for by in start_y.saturating_sub(pad)..=(start_y + total_h + pad).min(h - 1) {
            for bx in start_x.saturating_sub(pad)..=(start_x + total_w + pad).min(w - 1) {
                let orig = img.get_pixel(bx, by);
                let a = bg_color[3] as u16;
                let r = ((orig[0] as u16 * (255 - a) + bg_color[0] as u16 * a) / 255) as u8;
                let g = ((orig[1] as u16 * (255 - a) + bg_color[1] as u16 * a) / 255) as u8;
                let b = ((orig[2] as u16 * (255 - a) + bg_color[2] as u16 * a) / 255) as u8;
                img.put_pixel(bx, by, xcap::image::Rgba([r, g, b, 255]));
            }
        }
        
        let mut dx = start_x;
        for ch in label.chars() {
            if let Some(d) = ch.to_digit(10) {
                draw_pixel_digit(img, dx, start_y, d as u8, text_color, scale);
                dx += digit_w;
            }
        }
        
        y += label_interval;
    }
    
    // Draw origin marker at (0,0) - a bright cross to make the origin obvious
    for i in 0..15u32 {
        if i < w { img.put_pixel(i, 0, xcap::image::Rgba([255, 0, 0, 255])); }
        if i < h { img.put_pixel(0, i, xcap::image::Rgba([255, 0, 0, 255])); }
    }
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
    let mut rgba_img = img;
    
    // Draw coordinate ruler to help the AI model estimate positions
    draw_coordinate_ruler(&mut rgba_img);
    
    // Draw mouse cursor crosshair at current position for visual reference
    #[cfg(target_os = "windows")]
    {
        let (cur_x, cur_y) = {
            let (raw_x, raw_y) = crate::commands::selection::win::cursor_pos();
            let (enigo_w, enigo_h, xcap_w, xcap_h) = crate::commands::input::get_coordinate_spaces_pub();
            if xcap_w > 0 && enigo_w > 0 && xcap_w != enigo_w {
                let sx = xcap_w as f64 / enigo_w as f64;
                let sy = xcap_h as f64 / enigo_h as f64;
                ((raw_x as f64 * sx).round() as i32, (raw_y as f64 * sy).round() as i32)
            } else {
                (raw_x, raw_y)
            }
        };
        let cx = cur_x.max(0) as u32;
        let cy = cur_y.max(0) as u32;
        let crosshair_color = xcap::image::Rgba([255u8, 50, 50, 220]); // red crosshair
        let cross_size = 12u32;
        let img_w = phys_w;
        let img_h = phys_h;
        // Horizontal line
        for dx in cx.saturating_sub(cross_size)..=cx.saturating_add(cross_size).min(img_w - 1) {
            if dx < img_w && cy < img_h {
                let pixel = rgba_img.get_pixel(dx, cy);
                let a = crosshair_color[3] as u16;
                let r = ((pixel[0] as u16 * (255 - a) + crosshair_color[0] as u16 * a) / 255) as u8;
                let g = ((pixel[1] as u16 * (255 - a) + crosshair_color[1] as u16 * a) / 255) as u8;
                let b = ((pixel[2] as u16 * (255 - a) + crosshair_color[2] as u16 * a) / 255) as u8;
                rgba_img.put_pixel(dx, cy, xcap::image::Rgba([r, g, b, 255]));
            }
        }
        // Vertical line
        for dy in cy.saturating_sub(cross_size)..=cy.saturating_add(cross_size).min(img_h - 1) {
            if cx < img_w && dy < img_h {
                let pixel = rgba_img.get_pixel(cx, dy);
                let a = crosshair_color[3] as u16;
                let r = ((pixel[0] as u16 * (255 - a) + crosshair_color[0] as u16 * a) / 255) as u8;
                let g = ((pixel[1] as u16 * (255 - a) + crosshair_color[1] as u16 * a) / 255) as u8;
                let b = ((pixel[2] as u16 * (255 - a) + crosshair_color[2] as u16 * a) / 255) as u8;
                rgba_img.put_pixel(cx, dy, xcap::image::Rgba([r, g, b, 255]));
            }
        }
    }
    
    let dyn_img = xcap::image::DynamicImage::ImageRgba8(rgba_img);
    let buf = encode_as_jpeg(&dyn_img, 88)?;
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

/// Enumerate desktop icons from the Windows Shell SysListView32 control.
/// This is the CRITICAL function for precise desktop icon positioning.
/// EnumChildWindows cannot find desktop icons because they are ListView items,
/// not separate windows. This function uses LVM_GETITEMPOSITION and LVM_GETITEMTEXT
/// to get each icon's exact screen coordinates.
///
/// Returns a list of desktop icon names and their center pixel coordinates
/// (converted to physical pixel space to match the screenshot).
#[cfg(target_os = "windows")]
fn get_desktop_icons() -> Vec<String> {
    use std::ffi::c_void;

    // ── Helper: encode a Rust &str to a null-terminated UTF-16 (wide) vector ──
    fn w(s: &str) -> Vec<u16> {
        let mut v: Vec<u16> = s.encode_utf16().collect();
        v.push(0); // null terminator
        v
    }

    // ── Static FFI declarations (W-suffix = UTF-16 strings) ──
    extern "system" {
        fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> *mut c_void;
        fn FindWindowExW(hwndParent: *mut c_void, hwndChildAfter: *mut c_void,
                         lpszClass: *const u16, lpszWindow: *const u16) -> *mut c_void;
        fn SendMessageTimeoutW(hWnd: *mut c_void, Msg: u32, wParam: usize, lParam: isize,
                               fuFlags: u32, uTimeout: u32, lpdwResult: *mut usize) -> i32;
        fn ClientToScreen(hWnd: *mut c_void, lpPoint: *mut POINT) -> i32;
        fn GetWindowThreadProcessId(hWnd: *mut c_void, lpdwProcessId: *mut u32) -> u32;
        fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> *mut c_void;
        fn ReadProcessMemory(hProcess: *mut c_void, lpBaseAddress: *const c_void,
                             lpBuffer: *mut c_void, nSize: usize,
                             lpNumberOfBytesRead: *mut usize) -> i32;
        fn CloseHandle(hObject: *mut c_void) -> i32;
        fn GetCurrentProcessId() -> u32;
        // LoadLibraryA / GetProcAddress use ANSI/byte strings – these are safe to declare here
        fn LoadLibraryA(lpLibFileName: *const u8) -> *mut c_void;
        fn GetProcAddress(hModule: *mut c_void, lpProcName: *const u8) -> *mut c_void;
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct POINT { x: i32, y: i32 }

    const MEM_COMMIT: u32 = 0x1000;
    const MEM_RESERVE: u32 = 0x2000;
    const MEM_RELEASE: u32 = 0x8000;
    const PAGE_READWRITE: u32 = 0x04;
    const PROCESS_VM_READ: u32 = 0x0010;
    const PROCESS_VM_OPERATION: u32 = 0x0008;
    const PROCESS_VM_WRITE: u32 = 0x0020;
    const SMTO_NORMAL: u32 = 0x0000;

    // ListView messages
    const LVM_GETITEMCOUNT: u32 = 0x1004;
    const LVM_GETITEMPOSITION: u32 = 0x1010;
    const LVM_GETITEMTEXTW: u32 = 0x1073;

    // LVITEM structure for LVM_GETITEMTEXT
    #[repr(C)]
    struct LVITEMW {
        mask: u32,
        iItem: i32,
        iSubItem: i32,
        state: u32,
        stateMask: u32,
        pszText: *mut u16,  // pointer to text buffer in remote process
        cchTextMax: i32,
        iImage: i32,
        lParam: isize,
        iIndent: i32,
        iGroupId: i32,
        cColumns: u32,
        puColumns: u32,
    }

    const LVIF_TEXT: u32 = 0x0001;

    let mut icons = Vec::new();

    // Wide-string class names for FindWindowW / FindWindowExW
    let w_progman = w("Progman");
    let w_worker_w = w("WorkerW");
    let w_shell_defview = w("SHELLDLL_DefView");
    let w_syslistview = w("SysListView32");

    // Step 1: Find the Progman window
    let progman = unsafe { FindWindowW(w_progman.as_ptr(), std::ptr::null()) };
    if progman.is_null() {
        log::warn!("[desktop_icons] Progman window not found");
        return icons;
    }

    // Step 2: Send 0x052C to spawn WorkerW (required on Windows 10+)
    let mut result: usize = 0;
    unsafe {
        SendMessageTimeoutW(progman, 0x052C, 0, 0, SMTO_NORMAL, 1000, &mut result);
    }

    // Step 3: Find the WorkerW window that contains SHELLDLL_DefView
    let mut shell_view: *mut c_void = std::ptr::null_mut();
    let mut worker_w: *mut c_void = std::ptr::null_mut();

    loop {
        worker_w = unsafe {
            FindWindowExW(std::ptr::null_mut(), worker_w,
                         w_worker_w.as_ptr(), std::ptr::null())
        };
        if worker_w.is_null() { break; }

        let sv = unsafe {
            FindWindowExW(worker_w, std::ptr::null_mut(),
                         w_shell_defview.as_ptr(), std::ptr::null())
        };
        if !sv.is_null() {
            shell_view = sv;
            break;
        }
    }

    if shell_view.is_null() {
        // Fallback: try direct child of Progman
        shell_view = unsafe {
            FindWindowExW(progman, std::ptr::null_mut(),
                         w_shell_defview.as_ptr(), std::ptr::null())
        };
    }

    if shell_view.is_null() {
        log::warn!("[desktop_icons] SHELLDLL_DefView not found");
        return icons;
    }

    // Step 4: Find SysListView32 (the desktop icon list)
    let listview = unsafe {
        FindWindowExW(shell_view, std::ptr::null_mut(),
                     w_syslistview.as_ptr(), std::ptr::null())
    };
    if listview.is_null() {
        log::warn!("[desktop_icons] SysListView32 not found");
        return icons;
    }

    // Step 5: Get the icon count
    let count = unsafe {
        let r = SendMessageTimeoutW(listview, LVM_GETITEMCOUNT, 0, 0, SMTO_NORMAL, 1000, &mut result);
        if r == 0 { 0i32 } else { result as i32 }
    };

    log::info!("[desktop_icons] Found {} desktop icons", count);

    if count == 0 { return icons; }

    // Step 6: Get the Explorer process ID for cross-process memory access
    let mut explorer_pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(listview, &mut explorer_pid); }
    if explorer_pid == 0 {
        log::warn!("[desktop_icons] Could not get Explorer PID");
        return icons;
    }

    // Step 7: Open Explorer process for reading
    let h_process = unsafe {
        OpenProcess(PROCESS_VM_READ | PROCESS_VM_OPERATION | PROCESS_VM_WRITE, 0, explorer_pid)
    };
    if h_process.is_null() {
        log::warn!("[desktop_icons] Could not open Explorer process (pid={})", explorer_pid);
        return icons;
    }

    // Step 8: Dynamically load VirtualAllocEx, VirtualFreeEx, WriteProcessMemory, SendMessageW
    // We use LoadLibraryA + GetProcAddress (byte-string variants, no conflict with W-suffix decls)
    type VirtualAllocExFn = unsafe extern "system" fn(*mut c_void, *const c_void, usize, u32, u32) -> *mut c_void;
    type VirtualFreeExFn = unsafe extern "system" fn(*mut c_void, *const c_void, usize, u32) -> i32;
    type WriteProcessMemoryFn = unsafe extern "system" fn(*mut c_void, *mut c_void, *const c_void, usize, *mut usize) -> i32;
    type SendMessageWFn = unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize;

    let kernel32 = unsafe { LoadLibraryA(b"kernel32.dll\0".as_ptr()) };
    let user32 = unsafe { LoadLibraryA(b"user32.dll\0".as_ptr()) };

    let virtual_alloc_ex: Option<VirtualAllocExFn> = unsafe {
        if kernel32.is_null() { None }
        else {
            let proc = GetProcAddress(kernel32, b"VirtualAllocEx\0".as_ptr());
            if proc.is_null() { None } else { Some(std::mem::transmute(proc)) }
        }
    };

    let virtual_free_ex: Option<VirtualFreeExFn> = unsafe {
        if kernel32.is_null() { None }
        else {
            let proc = GetProcAddress(kernel32, b"VirtualFreeEx\0".as_ptr());
            if proc.is_null() { None } else { Some(std::mem::transmute(proc)) }
        }
    };

    let write_process_memory: Option<WriteProcessMemoryFn> = unsafe {
        if kernel32.is_null() { None }
        else {
            let proc = GetProcAddress(kernel32, b"WriteProcessMemory\0".as_ptr());
            if proc.is_null() { None } else { Some(std::mem::transmute(proc)) }
        }
    };

    let send_message_w: Option<SendMessageWFn> = unsafe {
        if user32.is_null() { None }
        else {
            let proc = GetProcAddress(user32, b"SendMessageW\0".as_ptr());
            if proc.is_null() { None } else { Some(std::mem::transmute(proc)) }
        }
    };

    if virtual_alloc_ex.is_none() || virtual_free_ex.is_none() ||
       write_process_memory.is_none() || send_message_w.is_none() {
        log::warn!("[desktop_icons] Could not load required API functions");
        unsafe { CloseHandle(h_process); }
        return icons;
    }

    let virtual_alloc_ex = virtual_alloc_ex.unwrap();
    let virtual_free_ex = virtual_free_ex.unwrap();
    let write_process_memory = write_process_memory.unwrap();
    let send_message_w = send_message_w.unwrap();

    // Allocate memory in Explorer's process
    let mem_size = std::mem::size_of::<LVITEMW>() + 520; // LVITEM + text buffer (260 UTF-16 chars)
    let remote_base = unsafe {
        virtual_alloc_ex(h_process, std::ptr::null(), mem_size, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE)
    };
    if remote_base.is_null() {
        log::warn!("[desktop_icons] VirtualAllocEx in Explorer failed");
        unsafe { CloseHandle(h_process); }
        return icons;
    }

    let remote_lvitem = remote_base;
    let remote_text_buf = unsafe { (remote_base as *mut u8).add(std::mem::size_of::<LVITEMW>()) as *mut u16 };

    // Step 9: Get DPI scale info for coordinate conversion
    let (enigo_w, enigo_h, phys_w, phys_h) = crate::commands::input::get_coordinate_spaces_pub();
    let dpi_scale_x = if enigo_w > 0 && phys_w > enigo_w { phys_w as f64 / enigo_w as f64 } else { 1.0 };
    let dpi_scale_y = if enigo_h > 0 && phys_h > enigo_h { phys_h as f64 / enigo_h as f64 } else { 1.0 };

    log::info!("[desktop_icons] DPI scale: x={:.3}, y={:.3} (enigo={}x{}, phys={}x{})",
               dpi_scale_x, dpi_scale_y, enigo_w, enigo_h, phys_w, phys_h);

    // Step 10: Enumerate each desktop icon
    let max_icons = count.min(50);
    for i in 0..max_icons {
        // Get icon position
        let mut pt = POINT { x: 0, y: 0 };
        let mut remote_pt = POINT { x: 0, y: 0 };

        // Write a POINT struct to remote memory for LVM_GETITEMPOSITION
        let mut bytes_written: usize = 0;
        unsafe {
            write_process_memory(h_process, remote_base, &mut remote_pt as *mut POINT as *mut c_void,
                                std::mem::size_of::<POINT>(), &mut bytes_written);
        }

        // Send LVM_GETITEMPOSITION
        let pos_result = unsafe {
            send_message_w(listview, LVM_GETITEMPOSITION, i as usize,
                          remote_base as isize)
        };

        // Read back the POINT from remote memory
        let mut bytes_read: usize = 0;
        unsafe {
            ReadProcessMemory(h_process, remote_base, &mut pt as *mut POINT as *mut c_void,
                             std::mem::size_of::<POINT>(), &mut bytes_read);
        }

        if pos_result == 0 || bytes_read == 0 {
            continue;
        }

        // Convert client coordinates to screen coordinates
        let mut screen_pt = pt;
        unsafe { ClientToScreen(listview, &mut screen_pt); }

        // Get icon text
        let mut local_lvitem = LVITEMW {
            mask: LVIF_TEXT,
            iItem: i,
            iSubItem: 0,
            state: 0,
            stateMask: 0,
            pszText: remote_text_buf as *mut u16,
            cchTextMax: 260,
            iImage: 0,
            lParam: 0,
            iIndent: 0,
            iGroupId: 0,
            cColumns: 0,
            puColumns: 0,
        };

        // Write LVITEM to remote memory
        let mut bytes_written: usize = 0;
        unsafe {
            write_process_memory(h_process, remote_lvitem, &mut local_lvitem as *mut LVITEMW as *mut c_void,
                                std::mem::size_of::<LVITEMW>(), &mut bytes_written);
        }

        // Send LVM_GETITEMTEXTW
        let text_len = unsafe {
            send_message_w(listview, LVM_GETITEMTEXTW, i as usize,
                          remote_lvitem as isize)
        };

        // Read back the text from remote memory
        let mut text_buf = [0u16; 260];
        let mut bytes_read: usize = 0;
        unsafe {
            ReadProcessMemory(h_process, remote_text_buf as *const c_void,
                             text_buf.as_mut_ptr() as *mut c_void,
                             260 * 2, &mut bytes_read);
        }

        let name = if text_len > 0 {
            String::from_utf16_lossy(&text_buf[..text_len as usize])
        } else {
            format!("Icon_{}", i)
        };

        // Desktop icon center: the position from LVM_GETITEMPOSITION is the top-left
        // of the icon's text area. The actual icon center is roughly:
        // x: same (icon is centered horizontally above the text)
        // y: about 25-30 pixels above the text position (icon height ~48px, text ~16px)
        // But for clicking, the icon center (including the image above text) is better.
        // We estimate the full icon center by moving y up by ~25 pixels.
        let icon_center_x = screen_pt.x + 37;  // typical icon width ~75px, center offset ~37
        let icon_center_y = screen_pt.y - 5;    // position is top of text, icon center is a bit above

        // Convert from logical to physical coordinates to match screenshot
        let phys_x = (icon_center_x as f64 * dpi_scale_x).round() as i32;
        let phys_y = (icon_center_y as f64 * dpi_scale_y).round() as i32;

        icons.push(format!(
            "[桌面图标] \"{}\" at ({},{})",
            name, phys_x, phys_y
        ));

        log::info!("[desktop_icons] Icon {}: '{}' client=({},{}) screen=({},{}) phys=({},{})",
                   i, name, pt.x, pt.y, icon_center_x, icon_center_y, phys_x, phys_y);
    }

    // Cleanup
    unsafe {
        virtual_free_ex(h_process, remote_base, 0, MEM_RELEASE);
        CloseHandle(h_process);
    }

    icons
}

/// Get clickable UI elements from screen using Windows Accessibility (MSAA/EnumChildWindows)
/// PLUS desktop icons via Shell API.
/// Returns a text description with element names and positions.
///
/// CRITICAL: Desktop icons are obtained via get_desktop_icons() which uses
/// the SysListView32 control - this provides EXACT coordinates that the AI
/// can use for pixel-perfect positioning. Without this, the AI must guess
/// coordinates from the screenshot, which is wildly inaccurate (±150px).
#[tauri::command]
pub fn get_screen_elements() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;

        let mut all_elements = Vec::new();

        // ===== PRIORITY 1: Desktop icons (most critical for accurate positioning) =====
        let desktop_icons = get_desktop_icons();
        if !desktop_icons.is_empty() {
            all_elements.push("=== 桌面图标 (Desktop Icons) ===".to_string());
            all_elements.extend(desktop_icons);
        }

        // ===== PRIORITY 2: Window-level elements via EnumChildWindows =====
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

            // Skip desktop SysListView32 items (we handle those via get_desktop_icons)
            let mut class_buf = [0u16; 128];
            let class_len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), 128);
            let class_name = if class_len > 0 {
                String::from_utf16_lossy(&class_buf[..class_len as usize])
            } else {
                String::new()
            };
            if class_name == "SysListView32" { return 1; }

            let mut rect = WinRect { left: 0, top: 0, right: 0, bottom: 0 };
            GetWindowRect(hwnd, &mut rect);

            if rect.right - rect.left < 10 || rect.bottom - rect.top < 10 { return 1; }

            let cx = (rect.left + rect.right) / 2;
            let cy = (rect.top + rect.bottom) / 2;

            // Convert logical to physical coordinates
            let (enigo_w, enigo_h, phys_w, phys_h) = crate::commands::input::get_coordinate_spaces_pub();
            let dpi_scale_x = if enigo_w > 0 && phys_w > enigo_w { phys_w as f64 / enigo_w as f64 } else { 1.0 };
            let dpi_scale_y = if enigo_h > 0 && phys_h > enigo_h { phys_h as f64 / enigo_h as f64 } else { 1.0 };
            let phys_cx = (cx as f64 * dpi_scale_x).round() as i32;
            let phys_cy = (cy as f64 * dpi_scale_y).round() as i32;

            col.elements.push(format!(
                "\"{}\" at ({},{}) size {}x{}",
                title, phys_cx, phys_cy,
                ((rect.right - rect.left) as f64 * dpi_scale_x).round() as i32,
                ((rect.bottom - rect.top) as f64 * dpi_scale_y).round() as i32
            ));

            if col.elements.len() >= 80 { return 0; }
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
        let window_elements = col.take().map(|c| c.elements).unwrap_or_default();

        if !window_elements.is_empty() {
            all_elements.push("=== 窗口和控件 (Windows & Controls) ===".to_string());
            all_elements.extend(window_elements);
        }

        let result = all_elements.join("\n");

        // Truncate to 4000 chars (increased from 3000 to accommodate desktop icons)
        let truncated = if result.len() > 4000 { &result[..4000] } else { &result };

        Ok(truncated.to_string())
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
