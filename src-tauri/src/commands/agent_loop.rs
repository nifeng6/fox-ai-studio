use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
use reqwest::Client;

use crate::commands::provider::ProviderState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStep {
    pub session_id: String,
    pub step: u32,
    pub screenshot_base64: String,
    pub action_description: String,
    pub tool_calls: Vec<ToolCallInfo>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallInfo {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionRequest {
    pub session_id: String,
    pub step: u32,
    pub action: ToolCallInfo,
    pub needs_approval: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentComplete {
    pub session_id: String,
    pub total_steps: u32,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentError {
    pub session_id: String,
    pub step: u32,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    pub session_id: String,
    pub running: bool,
    pub current_step: u32,
    pub max_steps: u32,
}

pub struct AgentLoopState(pub Arc<Mutex<HashMap<String, SessionData>>>);

impl Clone for AgentLoopState {
    fn clone(&self) -> Self {
        AgentLoopState(Arc::clone(&self.0))
    }
}

pub struct SessionData {
    pub running: bool,
    pub current_step: u32,
    pub max_steps: u32,
    pub permission_mode: String,
    pub approved_actions: Mutex<Vec<bool>>,
}

fn build_system_prompt(display_w: u32, display_h: u32) -> String {
    format!(r#"You are a computer use agent that controls a real desktop. You see screenshots and execute mouse/keyboard actions.

<display>
width_px={dw}  height_px={dh}
Coordinate origin (0,0) is at the TOP-LEFT. X goes RIGHT, Y goes DOWN.
All coordinates you provide MUST be within 0..{dw} for x and 0..{dh} for y.
</display>

<rules>
1. OBSERVE the screenshot carefully before EVERY action. Describe what you see.
2. Return EXACTLY ONE tool call per turn. After the action, you will get a new screenshot to verify.
3. Coordinates must target the PRECISE CENTER of the element you want to interact with.
4. For small elements (icons, checkboxes, small buttons): estimate the bounding box of the element and use its center point. A 32×32 icon at position (100,200) means you click (116, 216).
5. For text/labels: click the horizontal center and vertical middle of the text.
6. NEVER guess coordinates. If you cannot clearly identify the target element, say so in your thought and try an alternative approach (e.g. keyboard shortcut, start menu search).
7. Prefer keyboard shortcuts and search when possible — they are more reliable than clicking small icons.
   - To open an app: use open_application tool or press Win key then type the name
   - To close: Alt+F4
   - To switch windows: Alt+Tab
8. For drag operations (e.g. moving chess pieces): identify the EXACT center of the piece to pick up, and the EXACT center of the destination square.
</rules>

<tools>
- mouse_click: {{x, y, button?}} — Move cursor smoothly to (x,y) then click. Default button is "left".
- mouse_double_click: {{x, y}} — Double-click at (x,y).
- mouse_move: {{x, y}} — Move cursor to (x,y) without clicking.
- mouse_drag: {{from_x, from_y, to_x, to_y}} — Smooth drag from one point to another (press at from, release at to).
- mouse_scroll: {{x, y, direction, amount}} — Scroll at position. direction: "up"|"down". amount: number of scroll ticks.
- keyboard_type: {{text}} — Type a text string.
- keyboard_key: {{key, modifiers?}} — Press a key. modifiers: ["ctrl","alt","shift","meta"].
- keyboard_hotkey: {{keys}} — Press key combination simultaneously, e.g. ["ctrl","c"].
- open_application: {{name}} — Open an application by name using the OS launcher.
- wait: {{ms}} — Wait milliseconds before next action.
- task_complete: {{summary}} — Declare the task is finished.
</tools>"#,
    dw = display_w,
    dh = display_h,
    )
}

fn build_tool_definitions() -> serde_json::Value {
    serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "mouse_click",
                "description": "Click at screen coordinates",
                "parameters": {"type": "object", "properties": {
                    "x": {"type": "integer"}, "y": {"type": "integer"},
                    "button": {"type": "string", "enum": ["left","right","middle"]}
                }, "required": ["x","y"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "mouse_double_click",
                "description": "Double click at coordinates",
                "parameters": {"type": "object", "properties": {
                    "x": {"type": "integer"}, "y": {"type": "integer"}
                }, "required": ["x","y"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "mouse_move",
                "description": "Move cursor to coordinates",
                "parameters": {"type": "object", "properties": {
                    "x": {"type": "integer"}, "y": {"type": "integer"}
                }, "required": ["x","y"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "mouse_drag",
                "description": "Drag from one point to another",
                "parameters": {"type": "object", "properties": {
                    "from_x": {"type": "integer"}, "from_y": {"type": "integer"},
                    "to_x": {"type": "integer"}, "to_y": {"type": "integer"}
                }, "required": ["from_x","from_y","to_x","to_y"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "mouse_scroll",
                "description": "Scroll at coordinates",
                "parameters": {"type": "object", "properties": {
                    "x": {"type": "integer"}, "y": {"type": "integer"},
                    "direction": {"type": "string", "enum": ["up","down"]},
                    "amount": {"type": "integer"}
                }, "required": ["x","y","direction","amount"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "keyboard_type",
                "description": "Type text string",
                "parameters": {"type": "object", "properties": {
                    "text": {"type": "string"}
                }, "required": ["text"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "keyboard_key",
                "description": "Press a key with optional modifiers",
                "parameters": {"type": "object", "properties": {
                    "key": {"type": "string"},
                    "modifiers": {"type": "array", "items": {"type": "string"}}
                }, "required": ["key"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "keyboard_hotkey",
                "description": "Press key combination",
                "parameters": {"type": "object", "properties": {
                    "keys": {"type": "array", "items": {"type": "string"}}
                }, "required": ["keys"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "open_application",
                "description": "Open an application by name (uses system search/start menu)",
                "parameters": {"type": "object", "properties": {
                    "name": {"type": "string", "description": "Application name to open"}
                }, "required": ["name"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "wait",
                "description": "Wait for milliseconds",
                "parameters": {"type": "object", "properties": {
                    "ms": {"type": "integer"}
                }, "required": ["ms"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "task_complete",
                "description": "Declare the task is finished",
                "parameters": {"type": "object", "properties": {
                    "summary": {"type": "string"}
                }, "required": ["summary"]}
            }
        }
    ])
}

fn is_dangerous_action(action: &ToolCallInfo) -> bool {
    match action.name.as_str() {
        "keyboard_type" => true,
        "keyboard_key" => {
            if let Some(mods) = action.arguments.get("modifiers") {
                if let Some(arr) = mods.as_array() {
                    return arr.iter().any(|m| {
                        let s = m.as_str().unwrap_or("");
                        s == "alt" || s == "meta" || s == "super" || s == "win"
                    });
                }
            }
            false
        }
        "keyboard_hotkey" => true,
        _ => false,
    }
}

use enigo::{
    Button as EnigoButton, Coordinate as EnigoCoord, Direction as EnigoDir,
    Enigo as EnigoInstance, Mouse as EnigoMouse, Settings as EnigoSettings,
};

/// Map AI's screenshot coordinate to the logical screen coordinate that Enigo expects.
/// The AI works on a resized screenshot (e.g. 1280×720) but we need to click on the
/// actual screen. On Windows with DPI scaling, Enigo uses logical coordinates.
fn map_to_screen(v: i64, img_scale: f64, phys_size: u32, logical_size: u32) -> i32 {
    let physical = (v as f64 * img_scale).round() as i32;
    if logical_size == 0 || phys_size == 0 || logical_size == phys_size {
        return physical;
    }
    let dpi_scale = phys_size as f64 / logical_size as f64;
    (physical as f64 / dpi_scale).round() as i32
}

struct ScreenMapping {
    img_sx: f64,
    img_sy: f64,
    phys_w: u32,
    phys_h: u32,
    logical_w: u32,
    logical_h: u32,
}

impl ScreenMapping {
    fn new(img_sx: f64, img_sy: f64) -> Self {
        let (logical_w, logical_h, phys_w, phys_h) = crate::commands::input::get_coordinate_spaces_pub();
        log::info!(
            "[agent_loop] ScreenMapping: img_scale=({:.2},{:.2}), logical={}x{}, phys={}x{}",
            img_sx, img_sy, logical_w, logical_h, phys_w, phys_h
        );
        Self { img_sx, img_sy, phys_w, phys_h, logical_w, logical_h }
    }

    fn x(&self, v: i64) -> i32 {
        map_to_screen(v, self.img_sx, self.phys_w, self.logical_w)
    }

    fn y(&self, v: i64) -> i32 {
        map_to_screen(v, self.img_sy, self.phys_h, self.logical_h)
    }
}

fn new_agent_enigo() -> Result<EnigoInstance, String> {
    EnigoInstance::new(&EnigoSettings::default()).map_err(|e| format!("Enigo init: {}", e))
}

/// Smoothly move mouse from current position to target (tx, ty) in logical coordinates.
/// Simulates human-like cursor movement with an ease-in-out curve.
fn smooth_move_to(e: &mut EnigoInstance, tx: i32, ty: i32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let (cx, cy) = crate::commands::selection::win::cursor_pos();
    #[cfg(not(target_os = "windows"))]
    let (cx, cy) = (tx, ty);

    let dx = (tx - cx) as f64;
    let dy = (ty - cy) as f64;
    let dist = (dx * dx + dy * dy).sqrt();

    let steps = ((dist / 8.0).clamp(8.0, 40.0)) as u32;
    let step_delay = std::time::Duration::from_millis(
        if dist > 500.0 { 4 } else { 8 }
    );

    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let ease = t * t * (3.0 - 2.0 * t); // smoothstep
        let ix = cx + (dx * ease).round() as i32;
        let iy = cy + (dy * ease).round() as i32;
        e.move_mouse(ix, iy, EnigoCoord::Abs).map_err(|er| er.to_string())?;
        std::thread::sleep(step_delay);
    }
    e.move_mouse(tx, ty, EnigoCoord::Abs).map_err(|er| er.to_string())?;
    Ok(())
}

fn execute_tool_call(action: &ToolCallInfo, sx: f64, sy: f64) -> Result<String, String> {
    let args = &action.arguments;
    let sm = ScreenMapping::new(sx, sy);

    match action.name.as_str() {
        "mouse_click" => {
            let x = sm.x(args.get("x").and_then(|v| v.as_i64()).unwrap_or(0));
            let y = sm.y(args.get("y").and_then(|v| v.as_i64()).unwrap_or(0));
            log::info!("[agent_loop] mouse_click logical=({},{})", x, y);
            let mut e = new_agent_enigo()?;
            smooth_move_to(&mut e, x, y)?;
            std::thread::sleep(std::time::Duration::from_millis(60));
            let btn = match args.get("button").and_then(|v| v.as_str()) {
                Some("right") => EnigoButton::Right,
                Some("middle") => EnigoButton::Middle,
                _ => EnigoButton::Left,
            };
            e.button(btn, EnigoDir::Click).map_err(|e| e.to_string())?;
            Ok(format!("Clicked at logical ({},{})", x, y))
        }
        "mouse_double_click" => {
            let x = sm.x(args.get("x").and_then(|v| v.as_i64()).unwrap_or(0));
            let y = sm.y(args.get("y").and_then(|v| v.as_i64()).unwrap_or(0));
            let mut e = new_agent_enigo()?;
            smooth_move_to(&mut e, x, y)?;
            std::thread::sleep(std::time::Duration::from_millis(40));
            e.button(EnigoButton::Left, EnigoDir::Click).map_err(|e| e.to_string())?;
            std::thread::sleep(std::time::Duration::from_millis(60));
            e.button(EnigoButton::Left, EnigoDir::Click).map_err(|e| e.to_string())?;
            Ok(format!("Double clicked at logical ({},{})", x, y))
        }
        "mouse_move" => {
            let x = sm.x(args.get("x").and_then(|v| v.as_i64()).unwrap_or(0));
            let y = sm.y(args.get("y").and_then(|v| v.as_i64()).unwrap_or(0));
            let mut e = new_agent_enigo()?;
            smooth_move_to(&mut e, x, y)?;
            Ok(format!("Moved to logical ({},{})", x, y))
        }
        "mouse_drag" => {
            let fx = sm.x(args.get("from_x").and_then(|v| v.as_i64()).unwrap_or(0));
            let fy = sm.y(args.get("from_y").and_then(|v| v.as_i64()).unwrap_or(0));
            let tx = sm.x(args.get("to_x").and_then(|v| v.as_i64()).unwrap_or(0));
            let ty = sm.y(args.get("to_y").and_then(|v| v.as_i64()).unwrap_or(0));
            let mut e = new_agent_enigo()?;
            smooth_move_to(&mut e, fx, fy)?;
            std::thread::sleep(std::time::Duration::from_millis(60));
            e.button(EnigoButton::Left, EnigoDir::Press).map_err(|er| er.to_string())?;
            std::thread::sleep(std::time::Duration::from_millis(40));
            let steps = 20;
            let dx = (tx - fx) as f64;
            let dy = (ty - fy) as f64;
            for i in 1..=steps {
                let t = i as f64 / steps as f64;
                let ease = t * t * (3.0 - 2.0 * t);
                let cx = fx + (dx * ease).round() as i32;
                let cy = fy + (dy * ease).round() as i32;
                e.move_mouse(cx, cy, EnigoCoord::Abs).map_err(|er| er.to_string())?;
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            e.move_mouse(tx, ty, EnigoCoord::Abs).map_err(|er| er.to_string())?;
            std::thread::sleep(std::time::Duration::from_millis(30));
            e.button(EnigoButton::Left, EnigoDir::Release).map_err(|er| er.to_string())?;
            Ok(format!("Dragged ({},{}) -> ({},{})", fx, fy, tx, ty))
        }
        "mouse_scroll" => {
            let x = sm.x(args.get("x").and_then(|v| v.as_i64()).unwrap_or(0));
            let y = sm.y(args.get("y").and_then(|v| v.as_i64()).unwrap_or(0));
            let mut e = new_agent_enigo()?;
            smooth_move_to(&mut e, x, y)?;
            std::thread::sleep(std::time::Duration::from_millis(40));
            let dir = args.get("direction").and_then(|v| v.as_str()).unwrap_or("down");
            let amt = args.get("amount").and_then(|v| v.as_i64()).unwrap_or(3) as i32;
            let scroll_val = if dir == "up" { amt } else { -amt };
            e.scroll(scroll_val, enigo::Axis::Vertical).map_err(|er| er.to_string())?;
            Ok(format!("Scrolled {} at ({},{})", dir, x, y))
        }
        "keyboard_type" => {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
            crate::commands::input::keyboard_type(text.clone())?;
            Ok(format!("Typed: {}", if text.len() > 50 { &text[..50] } else { &text }))
        }
        "keyboard_key" => {
            let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let mods = args.get("modifiers").and_then(|v| v.as_array()).map(|arr| {
                arr.iter().filter_map(|m| m.as_str().map(String::from)).collect()
            });
            crate::commands::input::keyboard_key(key.clone(), mods)?;
            Ok(format!("Pressed key: {}", key))
        }
        "keyboard_hotkey" => {
            let keys: Vec<String> = args.get("keys").and_then(|v| v.as_array()).map(|arr| {
                arr.iter().filter_map(|k| k.as_str().map(String::from)).collect()
            }).unwrap_or_default();
            crate::commands::input::keyboard_hotkey(keys.clone())?;
            Ok(format!("Hotkey: {}", keys.join("+")))
        }
        "open_application" => {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            #[cfg(target_os = "windows")]
            {
                std::process::Command::new("cmd")
                    .args(["/C", "start", "", &name])
                    .spawn()
                    .map_err(|e| format!("Failed to open: {}", e))?;
            }
            #[cfg(target_os = "macos")]
            {
                std::process::Command::new("open")
                    .args(["-a", &name])
                    .spawn()
                    .map_err(|e| format!("Failed to open: {}", e))?;
            }
            #[cfg(target_os = "linux")]
            {
                std::process::Command::new("xdg-open")
                    .arg(&name)
                    .spawn()
                    .map_err(|e| format!("Failed to open: {}", e))?;
            }
            std::thread::sleep(std::time::Duration::from_millis(1500));
            Ok(format!("Opened: {}", name))
        }
        "wait" => {
            let ms = args.get("ms").and_then(|v| v.as_u64()).unwrap_or(500);
            std::thread::sleep(std::time::Duration::from_millis(ms));
            Ok(format!("Waited {}ms", ms))
        }
        "task_complete" => {
            let summary = args.get("summary").and_then(|v| v.as_str()).unwrap_or("Task completed").to_string();
            Ok(format!("DONE:{}", summary))
        }
        _ => Err(format!("Unknown tool: {}", action.name)),
    }
}

#[tauri::command]
pub async fn start_computer_use(
    app: AppHandle,
    goal: String,
    provider_id: String,
    model_id: String,
    max_steps: Option<u32>,
    permission_mode: Option<String>,
    state: State<'_, ProviderState>,
    loop_state: State<'_, AgentLoopState>,
) -> Result<String, String> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let max = match max_steps {
        Some(0) => u32::MAX,
        Some(n) => n,
        None => 50,
    };
    let perm = permission_mode.unwrap_or_else(|| "supervised".to_string());

    let provider = {
        let providers = state.0.lock().map_err(|e| e.to_string())?;
        providers.iter().find(|p| p.id == provider_id).cloned()
    };

    let provider = provider.ok_or_else(|| "Provider not found. Configure a Vision-capable model first.".to_string())?;

    {
        let mut sessions = loop_state.0.lock().map_err(|e| e.to_string())?;
        sessions.insert(session_id.clone(), SessionData {
            running: true,
            current_step: 0,
            max_steps: max,
            permission_mode: perm.clone(),
            approved_actions: Mutex::new(Vec::new()),
        });
    }

    let sid = session_id.clone();
    let app_clone = app.clone();
    let loop_state_clone = AgentLoopState(Arc::clone(&loop_state.0));

    tokio::spawn(async move {
        if let Err(e) = run_agent_loop(
            &app_clone,
            &sid,
            &goal,
            &provider,
            &model_id,
            max,
            &perm,
            &loop_state_clone,
        ).await {
            let _ = app_clone.emit("computer-use:error", AgentError {
                session_id: sid.clone(),
                step: 0,
                error: e.to_string(),
            });
        }
        if let Ok(mut sessions) = loop_state_clone.0.lock() {
            if let Some(s) = sessions.get_mut(&sid) {
                s.running = false;
            }
        }
    });

    Ok(session_id)
}

async fn run_agent_loop(
    app: &AppHandle,
    session_id: &str,
    goal: &str,
    provider: &crate::commands::provider::Provider,
    model_id: &str,
    max_steps: u32,
    permission_mode: &str,
    loop_state: &AgentLoopState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    let endpoint = provider.api_endpoint.trim_end_matches('/');
    let api_key = &provider.api_key;
    let is_anthropic = provider.channel_type == crate::commands::channel_types::CHANNEL_ANTHROPIC;

    let tools = build_tool_definitions();
    let mut history: Vec<serde_json::Value> = Vec::new();

    for step in 1..=max_steps {
        {
            let sessions = loop_state.0.lock().map_err(|e| e.to_string())?;
            if let Some(s) = sessions.get(session_id) {
                if !s.running { break; }
            } else {
                break;
            }
        }

        let capture = match crate::commands::desktop::capture_clean_screenshot(None) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("[agent_loop] screenshot failed: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                continue;
            }
        };

        let screenshot_b64 = &capture.jpeg_base64;
        let disp_w = capture.display_width;
        let disp_h = capture.display_height;
        let phys_w = capture.physical_width;
        let phys_h = capture.physical_height;
        let scale_x = phys_w as f64 / disp_w as f64;
        let scale_y = phys_h as f64 / disp_h as f64;

        if step == 1 {
            let sys_prompt = build_system_prompt(disp_w, disp_h);
            history.insert(0, serde_json::json!({"role": "system", "content": sys_prompt}));
        }

        let step_text = if step == 1 {
            format!("Goal: {}\nThis is the current screen ({}x{} px). Observe carefully and determine the first action.", goal, disp_w, disp_h)
        } else {
            format!("Step {}. Screen: {}x{} px. Goal: {}\nObserve the result of the previous action. What is the next action?", step, disp_w, disp_h, goal)
        };

        let user_content = if is_anthropic {
            serde_json::json!([
                {"type": "image", "source": {"type": "base64", "media_type": "image/jpeg", "data": screenshot_b64}},
                {"type": "text", "text": step_text}
            ])
        } else {
            serde_json::json!([
                {"type": "image_url", "image_url": {"url": format!("data:image/jpeg;base64,{}", screenshot_b64), "detail": "auto"}},
                {"type": "text", "text": step_text}
            ])
        };

        history.push(serde_json::json!({"role": "user", "content": user_content}));

        // Aggressive history management: only keep system + last 4 messages (2 turns).
        // Older image payloads are extremely expensive for the API.
        if history.len() > 5 {
            let sys = history[0].clone();
            let tail = history[history.len() - 4..].to_vec();
            history = vec![sys];
            history.extend(tail);
        }

        let response = if is_anthropic {
            call_anthropic_vision(&client, endpoint, api_key, model_id, &history, &tools).await?
        } else {
            call_openai_vision(&client, endpoint, api_key, model_id, &history, &tools).await?
        };

        let (thought, tool_calls) = parse_model_response(&response, is_anthropic);

        app.emit("computer-use:step", AgentStep {
            session_id: session_id.to_string(),
            step,
            screenshot_base64: screenshot_b64.clone(),
            action_description: thought.clone(),
            tool_calls: tool_calls.clone(),
            status: "executing".to_string(),
        })?;

        if let Ok(mut sessions) = loop_state.0.lock() {
            if let Some(s) = sessions.get_mut(session_id) {
                s.current_step = step;
            }
        }

        let mut task_done = false;
        let mut exec_results = Vec::new();

        for tc in &tool_calls {
            if tc.name == "task_complete" {
                let summary = tc.arguments.get("summary").and_then(|v| v.as_str()).unwrap_or("Done").to_string();
                app.emit("computer-use:complete", AgentComplete {
                    session_id: session_id.to_string(),
                    total_steps: step,
                    summary,
                })?;
                task_done = true;
                break;
            }

            let needs_approval = match permission_mode {
                "supervised" => true,
                "semi-auto" => is_dangerous_action(tc),
                _ => false,
            };

            app.emit("computer-use:action", ActionRequest {
                session_id: session_id.to_string(),
                step,
                action: tc.clone(),
                needs_approval,
            })?;

            if needs_approval {
                let approved = wait_for_approval(loop_state, session_id, 30_000).await;
                if !approved {
                    exec_results.push(format!("Action {} was rejected by user", tc.name));
                    continue;
                }
            }

            match execute_tool_call(tc, scale_x, scale_y) {
                Ok(result) => exec_results.push(result),
                Err(e) => exec_results.push(format!("Error: {}", e)),
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }

        if task_done { break; }

        history.push(serde_json::json!({
            "role": "assistant",
            "content": format!("Thought: {}\nActions executed. Results: {}", thought, exec_results.join("; "))
        }));

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    Ok(())
}

async fn call_openai_vision(
    client: &Client,
    endpoint: &str,
    api_key: &str,
    model_id: &str,
    history: &[serde_json::Value],
    tools: &serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/v1/chat/completions", endpoint);
    let messages: Vec<serde_json::Value> = history.to_vec();

    let body = serde_json::json!({
        "model": model_id,
        "messages": messages,
        "tools": tools,
        "tool_choice": "auto",
        "max_tokens": 2048,
        "temperature": 0.1
    });

    let resp = client.post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Vision API error {}: {}", status, body).into());
    }

    let val: serde_json::Value = resp.json().await?;
    Ok(val)
}

async fn call_anthropic_vision(
    client: &Client,
    endpoint: &str,
    api_key: &str,
    model_id: &str,
    history: &[serde_json::Value],
    tools: &serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/v1/messages", endpoint);

    let anthropic_tools: Vec<serde_json::Value> = tools.as_array().unwrap_or(&vec![]).iter().map(|t| {
        let f = t.get("function").unwrap_or(t);
        serde_json::json!({
            "name": f.get("name").and_then(|v| v.as_str()).unwrap_or(""),
            "description": f.get("description").and_then(|v| v.as_str()).unwrap_or(""),
            "input_schema": f.get("parameters").unwrap_or(&serde_json::json!({}))
        })
    }).collect();

    let (sys_text, msgs): (String, Vec<&serde_json::Value>) = {
        let mut sys = String::new();
        let mut non_sys = Vec::new();
        for m in history {
            if m.get("role").and_then(|v| v.as_str()) == Some("system") {
                if let Some(c) = m.get("content").and_then(|v| v.as_str()) {
                    sys = c.to_string();
                }
            } else {
                non_sys.push(m);
            }
        }
        (sys, non_sys)
    };

    let body = serde_json::json!({
        "model": model_id,
        "system": sys_text,
        "messages": msgs,
        "tools": anthropic_tools,
        "max_tokens": 2048,
        "temperature": 0.1
    });

    let resp = client.post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Anthropic Vision API error {}: {}", status, body).into());
    }

    let val: serde_json::Value = resp.json().await?;
    Ok(val)
}

fn parse_model_response(response: &serde_json::Value, is_anthropic: bool) -> (String, Vec<ToolCallInfo>) {
    let mut thought = String::new();
    let mut tool_calls = Vec::new();

    if is_anthropic {
        if let Some(content) = response.get("content").and_then(|c| c.as_array()) {
            for block in content {
                let btype = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match btype {
                    "text" => {
                        if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                            thought.push_str(text);
                        }
                    }
                    "tool_use" => {
                        let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                        let input = block.get("input").cloned().unwrap_or(serde_json::json!({}));
                        tool_calls.push(ToolCallInfo { name, arguments: input });
                    }
                    _ => {}
                }
            }
        }
    } else {
        if let Some(choice) = response.get("choices").and_then(|c| c.get(0)) {
            let msg = choice.get("message").unwrap_or(choice);

            if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                thought = content.to_string();
            }

            if let Some(tcs) = msg.get("tool_calls").and_then(|t| t.as_array()) {
                for tc in tcs {
                    let func = tc.get("function").unwrap_or(tc);
                    let name = func.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                    let args_str = func.get("arguments").and_then(|a| a.as_str()).unwrap_or("{}");
                    let arguments = serde_json::from_str(args_str).unwrap_or(serde_json::json!({}));
                    tool_calls.push(ToolCallInfo { name, arguments });
                }
            }
        }

        // Fallback: parse JSON from thought text if no tool_calls
        if tool_calls.is_empty() && !thought.is_empty() {
            if let Some(start) = thought.find('{') {
                if let Some(end) = thought.rfind('}') {
                    let json_str = &thought[start..=end];
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                        if let Some(action) = val.get("action") {
                            let name = action.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                            let arguments = action.get("arguments").cloned().unwrap_or(serde_json::json!({}));
                            if !name.is_empty() {
                                tool_calls.push(ToolCallInfo { name, arguments });
                            }
                        }
                        if let Some(t) = val.get("thought").and_then(|t| t.as_str()) {
                            thought = t.to_string();
                        }
                    }
                }
            }
        }
    }

    (thought, tool_calls)
}

async fn wait_for_approval(loop_state: &AgentLoopState, session_id: &str, timeout_ms: u64) -> bool {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed().as_millis() > timeout_ms as u128 {
            return false;
        }
        if let Ok(sessions) = loop_state.0.lock() {
            if let Some(session) = sessions.get(session_id) {
                if let Ok(mut approvals) = session.approved_actions.lock() {
                    if let Some(approved) = approvals.pop() {
                        return approved;
                    }
                }
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
}

#[tauri::command]
pub fn approve_action(
    session_id: String,
    approved: bool,
    state: State<'_, AgentLoopState>,
) -> Result<(), String> {
    let sessions = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(session) = sessions.get(&session_id) {
        let mut approvals = session.approved_actions.lock().map_err(|e| e.to_string())?;
        approvals.push(approved);
    }
    Ok(())
}

#[tauri::command]
pub fn stop_computer_use(
    session_id: String,
    state: State<'_, AgentLoopState>,
) -> Result<(), String> {
    let mut sessions = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(session) = sessions.get_mut(&session_id) {
        session.running = false;
    }
    Ok(())
}

#[tauri::command]
pub fn get_computer_use_status(
    session_id: String,
    state: State<'_, AgentLoopState>,
) -> Result<SessionStatus, String> {
    let sessions = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(session) = sessions.get(&session_id) {
        Ok(SessionStatus {
            session_id,
            running: session.running,
            current_step: session.current_step,
            max_steps: session.max_steps,
        })
    } else {
        Err("Session not found".to_string())
    }
}
