use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ToolEntry {
    pub name: &'static str,
    pub toolset: &'static str,
    pub description: &'static str,
    pub parameters: serde_json::Value,
}

pub struct ToolRegistry {
    tools: Vec<ToolEntry>,
}

impl ToolRegistry {
    fn new() -> Self {
        let mut r = Self { tools: Vec::new() };
        r.register_computer_use();
        r.register_terminal();
        r.register_file();
        r.register_web();
        r.register_vision();
        r.register_code_execution();
        r.register_skills();
        r.register_planning();
        r
    }

    fn reg(&mut self, name: &'static str, toolset: &'static str, desc: &'static str, params: serde_json::Value) {
        self.tools.push(ToolEntry { name, toolset, description: desc, parameters: params });
    }

    fn register_computer_use(&mut self) {
        self.reg("screenshot", "computer_use",
            "Take a screenshot of the current screen and return it for vision analysis",
            serde_json::json!({"type":"object","properties":{},"required":[]}));
        self.reg("mouse_click", "computer_use",
            "Click at screen coordinates",
            serde_json::json!({"type":"object","properties":{
                "x":{"type":"integer","description":"X pixel coordinate"},
                "y":{"type":"integer","description":"Y pixel coordinate"},
                "button":{"type":"string","enum":["left","right","middle"],"default":"left"}
            },"required":["x","y"]}));
        self.reg("mouse_double_click", "computer_use",
            "Double click at screen coordinates",
            serde_json::json!({"type":"object","properties":{
                "x":{"type":"integer"},"y":{"type":"integer"}
            },"required":["x","y"]}));
        self.reg("mouse_move", "computer_use",
            "Move mouse cursor to coordinates",
            serde_json::json!({"type":"object","properties":{
                "x":{"type":"integer"},"y":{"type":"integer"}
            },"required":["x","y"]}));
        self.reg("mouse_drag", "computer_use",
            "Drag from one point to another",
            serde_json::json!({"type":"object","properties":{
                "from_x":{"type":"integer"},"from_y":{"type":"integer"},
                "to_x":{"type":"integer"},"to_y":{"type":"integer"}
            },"required":["from_x","from_y","to_x","to_y"]}));
        self.reg("mouse_scroll", "computer_use",
            "Scroll at screen coordinates",
            serde_json::json!({"type":"object","properties":{
                "x":{"type":"integer"},"y":{"type":"integer"},
                "direction":{"type":"string","enum":["up","down"]},
                "amount":{"type":"integer","default":3}
            },"required":["x","y","direction","amount"]}));
        self.reg("keyboard_type", "computer_use",
            "Type a text string via keyboard",
            serde_json::json!({"type":"object","properties":{
                "text":{"type":"string"}
            },"required":["text"]}));
        self.reg("keyboard_key", "computer_use",
            "Press a single key with optional modifiers (enter, tab, escape, backspace, delete, up, down, left, right, etc.)",
            serde_json::json!({"type":"object","properties":{
                "key":{"type":"string","description":"Key name"},
                "modifiers":{"type":"array","items":{"type":"string"},"description":"Optional modifier keys: ctrl, alt, shift, meta"}
            },"required":["key"]}));
        self.reg("keyboard_hotkey", "computer_use",
            "Press a key combination simultaneously (e.g. ctrl+c)",
            serde_json::json!({"type":"object","properties":{
                "keys":{"type":"array","items":{"type":"string"},"description":"Keys to press together"}
            },"required":["keys"]}));
        self.reg("open_application", "computer_use",
            "Open an application by name using the OS launcher",
            serde_json::json!({"type":"object","properties":{
                "name":{"type":"string","description":"Application name to open"}
            },"required":["name"]}));
        self.reg("action_sequence", "computer_use",
            "Execute a sequence of mouse/keyboard actions atomically with smooth human-like transitions. Use for multi-step operations like drag-and-drop, chess piece moves, etc.",
            serde_json::json!({"type":"object","properties":{
                "steps":{"type":"array","description":"Array of action steps. Each step: {action, ...params}. Actions: move(x,y), click(x,y,button?), press(x?,y?,button?), release(x?,y?,button?), type(text), key(key,modifiers?), wait(ms)","items":{"type":"object"}}
            },"required":["steps"]}));
        self.reg("wait", "computer_use",
            "Wait for a specified number of milliseconds",
            serde_json::json!({"type":"object","properties":{
                "ms":{"type":"integer","description":"Milliseconds to wait"}
            },"required":["ms"]}));
    }

    fn register_terminal(&mut self) {
        self.reg("terminal", "terminal",
            "Execute a shell command in a persistent terminal session. Supports background execution and custom working directory.",
            serde_json::json!({"type":"object","properties":{
                "command":{"type":"string","description":"The shell command to execute"},
                "background":{"type":"boolean","description":"Run in background without waiting for completion","default":false},
                "timeout":{"type":"integer","description":"Timeout in seconds (default 30)","default":30},
                "workdir":{"type":"string","description":"Working directory for the command"}
            },"required":["command"]}));
        self.reg("process", "terminal",
            "Manage running processes: list, kill, or check status",
            serde_json::json!({"type":"object","properties":{
                "action":{"type":"string","enum":["list","kill","status"],"description":"Action to perform"},
                "pid":{"type":"integer","description":"Process ID (required for kill/status)"}
            },"required":["action"]}));
    }

    fn register_file(&mut self) {
        self.reg("read_file", "file",
            "Read the contents of a file at the given path",
            serde_json::json!({"type":"object","properties":{
                "path":{"type":"string","description":"Absolute file path to read"}
            },"required":["path"]}));
        self.reg("write_file", "file",
            "Write content to a file (creates directories if needed, overwrites existing)",
            serde_json::json!({"type":"object","properties":{
                "path":{"type":"string","description":"Absolute file path"},
                "content":{"type":"string","description":"Content to write"}
            },"required":["path","content"]}));
        self.reg("patch", "file",
            "Apply a search-and-replace patch to a file. More precise than write_file for targeted edits.",
            serde_json::json!({"type":"object","properties":{
                "path":{"type":"string","description":"Absolute file path to patch"},
                "search":{"type":"string","description":"Exact text to find in the file"},
                "replace":{"type":"string","description":"Text to replace the search string with"}
            },"required":["path","search","replace"]}));
        self.reg("search_files", "file",
            "Search for text patterns in files within a directory (recursive grep)",
            serde_json::json!({"type":"object","properties":{
                "directory":{"type":"string","description":"Directory to search in"},
                "pattern":{"type":"string","description":"Text or regex pattern to search for"},
                "file_pattern":{"type":"string","description":"Glob pattern for file names (e.g. *.py, *.rs)","default":"*"}
            },"required":["directory","pattern"]}));
    }

    fn register_web(&mut self) {
        self.reg("web_search", "web",
            "Search the web using the configured search engine and return results",
            serde_json::json!({"type":"object","properties":{
                "query":{"type":"string","description":"Search query"},
                "num_results":{"type":"integer","description":"Number of results to return","default":5}
            },"required":["query"]}));
        self.reg("web_extract", "web",
            "Fetch a URL and extract its main content as clean text",
            serde_json::json!({"type":"object","properties":{
                "url":{"type":"string","description":"URL to extract content from"}
            },"required":["url"]}));
        self.reg("fetch_url", "web",
            "Fetch raw content of a URL via HTTP GET",
            serde_json::json!({"type":"object","properties":{
                "url":{"type":"string","description":"The URL to fetch"}
            },"required":["url"]}));
    }

    fn register_vision(&mut self) {
        self.reg("vision_analyze", "vision",
            "Analyze an image using a dedicated vision AI model. Accepts local file paths (e.g. C:/Users/xxx/photo.jpg), data URIs, or HTTP URLs. Use this for ANY image analysis task — never try to analyze images with code.",
            serde_json::json!({"type":"object","properties":{
                "image_url":{"type":"string","description":"Local file path (e.g. C:/Desktop/img.png), base64 data URI, or HTTP URL of the image"},
                "question":{"type":"string","description":"What to analyze, e.g. 'What text is visible?', 'Describe this image', 'What objects are in this photo?'"}
            },"required":["image_url","question"]}));
    }

    fn register_code_execution(&mut self) {
        self.reg("execute_code", "code_execution",
            "Execute a code snippet in an isolated environment. Supports Python, JavaScript, and Shell.",
            serde_json::json!({"type":"object","properties":{
                "code":{"type":"string","description":"Code to execute"},
                "language":{"type":"string","enum":["python","javascript","shell"],"description":"Programming language","default":"python"}
            },"required":["code"]}));
    }

    fn register_skills(&mut self) {
        self.reg("skills_list", "skills",
            "List all available skills with their names, descriptions, and categories",
            serde_json::json!({"type":"object","properties":{
                "category":{"type":"string","description":"Optional category filter"}
            },"required":[]}));
        self.reg("skill_view", "skills",
            "View the full content and instructions of a specific skill by name",
            serde_json::json!({"type":"object","properties":{
                "name":{"type":"string","description":"Name of the skill to view"}
            },"required":["name"]}));
        self.reg("skill_manage", "skills",
            "Create, edit, patch, or delete skills. After completing a complex task (5+ tool calls), save the approach as a skill so you can reuse it next time. Use 'patch' with old_string/new_string for precise edits.",
            serde_json::json!({"type":"object","properties":{
                "action":{"type":"string","enum":["create","edit","patch","delete"],"description":"Action: create, edit (full overwrite), patch (search-replace), delete"},
                "name":{"type":"string","description":"Skill name"},
                "description":{"type":"string","description":"What this skill does and when to use it"},
                "trigger":{"type":"string","description":"Comma-separated trigger keywords"},
                "content":{"type":"string","description":"Full skill instructions (for create/edit)"},
                "old_string":{"type":"string","description":"For patch: the text to find in skill instructions"},
                "new_string":{"type":"string","description":"For patch: the replacement text"}
            },"required":["action","name"]}));
    }

    fn register_planning(&mut self) {
        self.reg("task_complete", "planning",
            "Declare that the current task is finished with a summary",
            serde_json::json!({"type":"object","properties":{
                "summary":{"type":"string","description":"Brief summary of what was accomplished"}
            },"required":["summary"]}));
        self.reg("todo", "planning",
            "Manage a task list: add, complete, or list items for complex multi-step tasks",
            serde_json::json!({"type":"object","properties":{
                "action":{"type":"string","enum":["add","complete","list","clear"],"description":"Action to perform"},
                "text":{"type":"string","description":"Todo item text (for add)"},
                "index":{"type":"integer","description":"Item index (for complete)"}
            },"required":["action"]}));
        self.reg("session_search", "planning",
            "Search the current conversation history for specific information. Useful in long conversations to find earlier context, instructions, or results without losing track.",
            serde_json::json!({"type":"object","properties":{
                "query":{"type":"string","description":"Search query — keywords or phrases to find in conversation history"},
                "max_results":{"type":"integer","description":"Maximum number of matching messages to return (default: 5)"}
            },"required":["query"]}));
        self.reg("memory", "planning",
            "Manage persistent memory. Use 'add' to save new knowledge/preferences/insights. Use 'replace' to update an existing memory entry by content match. Use 'remove' to delete a memory. Target 'memory' for notes, 'user' for user profile.",
            serde_json::json!({"type":"object","properties":{
                "action":{"type":"string","enum":["add","replace","remove"],"description":"Action: add new, replace existing, or remove a memory entry"},
                "target":{"type":"string","enum":["memory","user"],"description":"Target: 'memory' for knowledge notes, 'user' for user profile info. Default: memory"},
                "content":{"type":"string","description":"The knowledge/information content. For 'replace': the new content. For 'remove': the content to match and delete."},
                "old_content":{"type":"string","description":"For 'replace' action only: the old content to find and replace"},
                "category":{"type":"string","enum":["preference","fact","workflow","correction","insight"],"description":"Category of the memory"},
                "update_profile":{"type":"object","description":"For target='user': update user profile fields","properties":{
                    "name":{"type":"string"},
                    "preferences":{"type":"object","additionalProperties":{"type":"string"}},
                    "notes":{"type":"string"}
                }}
            },"required":["action","content"]}));
    }

    pub fn get_definitions(&self, enabled_tools: &[String]) -> Vec<serde_json::Value> {
        self.tools.iter()
            .filter(|t| enabled_tools.iter().any(|e| e == t.name))
            .map(|t| serde_json::json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters
                }
            }))
            .collect()
    }

    pub fn get_all_tool_names(&self) -> Vec<&'static str> {
        self.tools.iter().map(|t| t.name).collect()
    }

    pub fn get_toolset_names(&self, toolset: &str) -> Vec<&'static str> {
        self.tools.iter().filter(|t| t.toolset == toolset).map(|t| t.name).collect()
    }

    pub fn get_toolsets(&self) -> Vec<&'static str> {
        let mut sets: Vec<&str> = self.tools.iter().map(|t| t.toolset).collect();
        sets.sort();
        sets.dedup();
        sets
    }
}

static REGISTRY: std::sync::LazyLock<ToolRegistry> = std::sync::LazyLock::new(ToolRegistry::new);

pub fn get_registry() -> &'static ToolRegistry {
    &REGISTRY
}

pub fn build_chat_tool_definitions(enabled_tools: &[String]) -> serde_json::Value {
    serde_json::json!(REGISTRY.get_definitions(enabled_tools))
}

// ── Agent-loop tools (handled on frontend, not in Rust execute_tool) ──
const FRONTEND_TOOLS: &[&str] = &[
    "skills_list", "skill_view", "skill_manage", "todo",
    "vision_analyze", "web_search", "memory", "session_search",
];

pub fn is_frontend_tool(name: &str) -> bool {
    FRONTEND_TOOLS.contains(&name)
}

// ── Window management for screenshots ──
#[cfg(target_os = "windows")]
mod win_helper {
    use std::ffi::c_void;
    type HWND = *mut c_void;

    extern "system" {
        fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> i32;
        fn IsWindowVisible(hWnd: HWND) -> i32;
        fn EnumWindows(
            lpEnumFunc: unsafe extern "system" fn(HWND, isize) -> i32,
            lParam: isize,
        ) -> i32;
    }

    // Re-use the GetWindowThreadProcessId from selection.rs would conflict,
    // so we use GetCurrentProcessId + manual PID check via a different approach.
    extern "system" {
        fn GetCurrentProcessId() -> u32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetWindowThreadProcessId_local(hWnd: HWND, lpdwProcessId: *mut u32) -> u32;
    }

    // We can't redeclare GetWindowThreadProcessId without conflict, so use
    // the raw LoadLibrary/GetProcAddress approach or just use process ID from std.
    fn get_window_pid(hwnd: HWND) -> u32 {
        // Use inline asm-free approach: call the function via a type alias
        type GWTPIDFn = unsafe extern "system" fn(HWND, *mut u32) -> u32;
        let func: GWTPIDFn = unsafe {
            let lib = LoadLibraryA(b"user32.dll\0".as_ptr());
            if lib.is_null() { return 0; }
            let proc = GetProcAddress(lib, b"GetWindowThreadProcessId\0".as_ptr());
            if proc.is_null() { return 0; }
            std::mem::transmute(proc)
        };
        let mut pid: u32 = 0;
        unsafe { func(hwnd, &mut pid); }
        pid
    }

    extern "system" {
        fn LoadLibraryA(lpLibFileName: *const u8) -> *mut c_void;
        fn GetProcAddress(hModule: *mut c_void, lpProcName: *const u8) -> *mut c_void;
    }

    unsafe extern "system" fn minimize_cb(hwnd: HWND, lparam: isize) -> i32 {
        let target_pid = lparam as u32;
        let wpid = get_window_pid(hwnd);
        if wpid == target_pid && IsWindowVisible(hwnd) != 0 {
            ShowWindow(hwnd, 6); // SW_MINIMIZE
        }
        1
    }

    unsafe extern "system" fn restore_cb(hwnd: HWND, lparam: isize) -> i32 {
        let target_pid = lparam as u32;
        let wpid = get_window_pid(hwnd);
        if wpid == target_pid {
            ShowWindow(hwnd, 9); // SW_RESTORE
        }
        1
    }

    pub fn minimize_own_windows() {
        let pid = unsafe { GetCurrentProcessId() };
        unsafe { EnumWindows(minimize_cb, pid as isize); }
        log::info!("[screenshot] minimized own windows (pid={})", pid);
    }

    pub fn restore_own_windows() {
        let pid = unsafe { GetCurrentProcessId() };
        unsafe { EnumWindows(restore_cb, pid as isize); }
        log::info!("[screenshot] restored own windows (pid={})", pid);
    }
}

// ── Tool execution ──

pub async fn execute_tool(tool: &ToolCallInfo) -> Result<String, String> {
    let args = &tool.arguments;
    match tool.name.as_str() {
        // ── Computer Use ──
        "screenshot" => {
            #[cfg(target_os = "windows")]
            {
                win_helper::minimize_own_windows();
                std::thread::sleep(std::time::Duration::from_millis(300));
            }

            let capture = crate::commands::desktop::capture_clean_screenshot(None)?;
            if capture.jpeg_base64.len() > 200 {
                let a11y_info = crate::commands::desktop::get_screen_elements()
                    .unwrap_or_default();
                let win_section = if a11y_info.is_empty() { String::new() }
                    else { format!("WINDOWS:{}", a11y_info.replace('\n', "|")) };

                Ok(format!(
                    "__SCREENSHOT__:{}x{}:{}x{}:CURSOR:{},{}:{}:data:image/jpeg;base64,{}",
                    capture.physical_width, capture.physical_height,
                    capture.display_width, capture.display_height,
                    capture.cursor_display_x, capture.cursor_display_y,
                    win_section.replace('\n', "\\n"),
                    capture.jpeg_base64
                ))
            } else {
                Err("Screenshot capture failed: empty result".into())
            }
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
            Ok(format!("Opened application: {}", name))
        }
        "action_sequence" => {
            let steps = args.get("steps")
                .and_then(|v| v.as_array())
                .map(|arr| arr.clone())
                .unwrap_or_default();
            crate::commands::input::action_sequence(steps)
        }
        "mouse_click" => {
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let btn = args.get("button").and_then(|v| v.as_str()).map(String::from);
            crate::commands::input::mouse_click(x, y, btn)?;
            Ok(format!("Clicked at ({}, {})", x, y))
        }
        "mouse_double_click" => {
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            crate::commands::input::mouse_double_click(x, y)?;
            Ok(format!("Double clicked at ({}, {})", x, y))
        }
        "mouse_move" => {
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            crate::commands::input::mouse_move(x, y)?;
            Ok(format!("Moved to ({}, {})", x, y))
        }
        "mouse_drag" => {
            let fx = args.get("from_x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let fy = args.get("from_y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let tx = args.get("to_x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let ty = args.get("to_y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            crate::commands::input::mouse_drag(fx, fy, tx, ty)?;
            Ok(format!("Dragged from ({},{}) to ({},{})", fx, fy, tx, ty))
        }
        "mouse_scroll" => {
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let dir = args.get("direction").and_then(|v| v.as_str()).unwrap_or("down").to_string();
            let amt = args.get("amount").and_then(|v| v.as_i64()).unwrap_or(3) as i32;
            crate::commands::input::mouse_scroll(x, y, dir, amt)?;
            Ok("Scrolled".into())
        }
        "keyboard_type" => {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
            crate::commands::input::keyboard_type(text.clone())?;
            Ok(format!("Typed: {}", if text.len() > 80 { &text[..80] } else { &text }))
        }
        "keyboard_key" => {
            let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let mods = args.get("modifiers").and_then(|v| v.as_array()).map(|arr| {
                arr.iter().filter_map(|m| m.as_str().map(String::from)).collect()
            });
            crate::commands::input::keyboard_key(key.clone(), mods)?;
            Ok(format!("Pressed: {}", key))
        }
        "keyboard_hotkey" => {
            let keys: Vec<String> = args.get("keys").and_then(|v| v.as_array()).map(|arr| {
                arr.iter().filter_map(|k| k.as_str().map(String::from)).collect()
            }).unwrap_or_default();
            crate::commands::input::keyboard_hotkey(keys.clone())?;
            Ok(format!("Hotkey: {}", keys.join("+")))
        }
        "wait" => {
            let ms = args.get("ms").and_then(|v| v.as_u64()).unwrap_or(500);
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
            Ok(format!("Waited {}ms", ms))
        }

        // ── Terminal ──
        "terminal" | "shell_execute" => {
            let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if cmd.is_empty() { return Err("Empty command".into()); }
            let workdir = args.get("workdir").and_then(|v| v.as_str());
            let timeout_secs = args.get("timeout").and_then(|v| v.as_u64()).unwrap_or(30);
            let background = args.get("background").and_then(|v| v.as_bool()).unwrap_or(false);

            let mut command = if cfg!(windows) {
                let mut c = tokio::process::Command::new("cmd");
                c.args(["/C", &cmd]);
                c
            } else {
                let mut c = tokio::process::Command::new("sh");
                c.args(["-c", &cmd]);
                c
            };

            if let Some(wd) = workdir {
                command.current_dir(wd);
            }
            command.stdout(std::process::Stdio::piped());
            command.stderr(std::process::Stdio::piped());

            if background {
                match command.spawn() {
                    Ok(child) => Ok(format!("Background process started (pid: {})", child.id().unwrap_or(0))),
                    Err(e) => Err(format!("Failed to spawn: {}", e)),
                }
            } else {
                let result = tokio::time::timeout(
                    std::time::Duration::from_secs(timeout_secs),
                    command.output()
                ).await;
                match result {
                    Ok(Ok(output)) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let exit = output.status.code().unwrap_or(-1);
                        let mut r = format!("exit_code: {}\n", exit);
                        if !stdout.is_empty() {
                            let s = if stdout.len() > 4000 { &stdout[..4000] } else { &stdout };
                            r.push_str(&format!("stdout:\n{}\n", s));
                        }
                        if !stderr.is_empty() {
                            let s = if stderr.len() > 2000 { &stderr[..2000] } else { &stderr };
                            r.push_str(&format!("stderr:\n{}\n", s));
                        }
                        Ok(r)
                    }
                    Ok(Err(e)) => Err(format!("Execution failed: {}", e)),
                    Err(_) => Err(format!("Command timed out after {}s", timeout_secs)),
                }
            }
        }
        "process" => {
            let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("list");
            match action {
                "list" => {
                    let cmd = if cfg!(windows) { "tasklist /FO CSV /NH" } else { "ps aux --sort=-%mem | head -20" };
                    let output = std::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" })
                        .args(if cfg!(windows) { vec!["/C", cmd] } else { vec!["-c", cmd] })
                        .output()
                        .map_err(|e| e.to_string())?;
                    let out = String::from_utf8_lossy(&output.stdout);
                    let truncated = if out.len() > 4000 { format!("{}...[truncated]", &out[..4000]) } else { out.to_string() };
                    Ok(truncated)
                }
                "kill" => {
                    let pid = args.get("pid").and_then(|v| v.as_i64()).ok_or("pid required")?;
                    let cmd = if cfg!(windows) { format!("taskkill /PID {} /F", pid) } else { format!("kill -9 {}", pid) };
                    let output = std::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" })
                        .args(if cfg!(windows) { vec!["/C", &cmd] } else { vec!["-c", &cmd] })
                        .output()
                        .map_err(|e| e.to_string())?;
                    Ok(String::from_utf8_lossy(&output.stdout).to_string() + &String::from_utf8_lossy(&output.stderr))
                }
                _ => Err(format!("Unknown process action: {}", action))
            }
        }

        // ── File ──
        "read_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            if path.is_empty() { return Err("Empty path".into()); }
            let content = std::fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;
            let truncated = if content.len() > 8000 { format!("{}...[truncated]", &content[..8000]) } else { content };
            Ok(truncated)
        }
        "write_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
            if path.is_empty() { return Err("Empty path".into()); }
            if let Some(parent) = std::path::Path::new(path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(path, content).map_err(|e| format!("Write error: {}", e))?;
            Ok(format!("Written {} bytes to {}", content.len(), path))
        }
        "patch" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let search = args.get("search").and_then(|v| v.as_str()).unwrap_or("");
            let replace = args.get("replace").and_then(|v| v.as_str()).unwrap_or("");
            if path.is_empty() { return Err("Empty path".into()); }
            if search.is_empty() { return Err("Empty search string".into()); }
            let content = std::fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;
            if !content.contains(search) {
                return Err(format!("Search string not found in {}", path));
            }
            let new_content = content.replacen(search, replace, 1);
            std::fs::write(path, &new_content).map_err(|e| format!("Write error: {}", e))?;
            Ok(format!("Patched {} (replaced {} chars with {} chars)", path, search.len(), replace.len()))
        }
        "search_files" => {
            let directory = args.get("directory").and_then(|v| v.as_str()).unwrap_or(".");
            let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
            let file_pattern = args.get("file_pattern").and_then(|v| v.as_str()).unwrap_or("*");
            if pattern.is_empty() { return Err("Empty search pattern".into()); }

            let cmd = if cfg!(windows) {
                format!("findstr /S /N /I /C:\"{}\" {}\\{}", pattern, directory, file_pattern)
            } else {
                format!("grep -rn --include='{}' '{}' '{}'", file_pattern, pattern, directory)
            };
            let output = std::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" })
                .args(if cfg!(windows) { vec!["/C", &cmd] } else { vec!["-c", &cmd] })
                .output()
                .map_err(|e| e.to_string())?;
            let out = String::from_utf8_lossy(&output.stdout);
            let truncated = if out.len() > 6000 { format!("{}...[truncated]", &out[..6000]) } else { out.to_string() };
            if truncated.is_empty() {
                Ok("No matches found.".to_string())
            } else {
                Ok(truncated)
            }
        }

        // ── Web ──
        "fetch_url" | "web_extract" => {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if url.is_empty() { return Err("Empty URL".into()); }
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .map_err(|e| e.to_string())?;
            let resp = client.get(&url).send().await.map_err(|e| format!("Fetch error: {}", e))?;
            let status = resp.status().as_u16();
            let body = resp.text().await.map_err(|e| e.to_string())?;
            let truncated = if body.len() > 8000 { format!("{}...[truncated]", &body[..8000]) } else { body };
            Ok(format!("HTTP {} — {}", status, truncated))
        }

        // ── Code Execution ──
        "execute_code" => {
            let code = args.get("code").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let lang = args.get("language").and_then(|v| v.as_str()).unwrap_or("python");
            if code.is_empty() { return Err("Empty code".into()); }

            let temp_dir = std::env::temp_dir();
            let ext = match lang { "python" => "py", "javascript" => "js", _ => "sh" };
            let temp_file = temp_dir.join(format!("fox_exec_{}.{}", std::process::id(), ext));
            std::fs::write(&temp_file, &code).map_err(|e| format!("Write temp: {}", e))?;

            let run_cmd = match lang {
                "python" => {
                    if cfg!(windows) { format!("python \"{}\"", temp_file.display()) }
                    else { format!("python3 \"{}\"", temp_file.display()) }
                }
                "javascript" => format!("node \"{}\"", temp_file.display()),
                _ => {
                    if cfg!(windows) { format!("cmd /C \"{}\"", temp_file.display()) }
                    else { format!("sh \"{}\"", temp_file.display()) }
                }
            };

            let result = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                tokio::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" })
                    .args(if cfg!(windows) { vec!["/C", &run_cmd] } else { vec!["-c", &run_cmd] })
                    .output()
            ).await;

            let _ = std::fs::remove_file(&temp_file);

            match result {
                Ok(Ok(output)) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let exit = output.status.code().unwrap_or(-1);
                    let mut r = format!("exit_code: {}\n", exit);
                    if !stdout.is_empty() {
                        let s = if stdout.len() > 4000 { &stdout[..4000] } else { &stdout };
                        r.push_str(&format!("stdout:\n{}\n", s));
                    }
                    if !stderr.is_empty() {
                        let s = if stderr.len() > 2000 { &stderr[..2000] } else { &stderr };
                        r.push_str(&format!("stderr:\n{}\n", s));
                    }
                    Ok(r)
                }
                Ok(Err(e)) => Err(format!("Execution failed: {}", e)),
                Err(_) => Err("Code execution timed out (30s)".into()),
            }
        }

        // ── Planning ──
        "task_complete" => {
            #[cfg(target_os = "windows")]
            win_helper::restore_own_windows();
            let summary = args.get("summary").and_then(|v| v.as_str()).unwrap_or("Done");
            Ok(format!("TASK_COMPLETE: {}", summary))
        }

        // ── Frontend-handled tools return a marker ──
        name if is_frontend_tool(name) => {
            Ok(format!("__FRONTEND_TOOL__:{}", serde_json::to_string(args).unwrap_or_default()))
        }

        other => Err(format!("Unknown tool: {}", other)),
    }
}
