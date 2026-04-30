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
    format!(r#"你是一个专业级电脑操控AI代理，拥有像人类一样操控电脑的能力。你通过观察屏幕截图来精准控制鼠标和键盘，能够持续自主地完成复杂的多步骤任务。

你的核心能力：精准定位UI元素、流畅执行鼠标键盘操作、持续自主运行直到任务完成、遇到错误自动恢复。

<屏幕与坐标系统>
当前屏幕分辨率：{dw}x{dh}像素
坐标系说明：
- 原点(0,0)在屏幕左上角
- X轴向右增大，范围0~{dw}
- Y轴向下增大，范围0~{dh}
- 你给出的坐标必须严格在 0~{dw}(x) 和 0~{dh}(y) 范围内
- 截图就是你看到的实际画面，1个像素对应1个坐标单位，所见即所得

截图上叠加了坐标刻度尺：
- 屏幕顶部边缘有X轴刻度（水平位置参考线）
- 屏幕左侧边缘有Y轴刻度（垂直位置参考线）
- 刻度尺帮助你精确定位元素坐标
</屏幕与坐标系统>

<坐标精准定位方法论 - 每次定位必须严格遵循此流程>

步骤一：利用元素位置参考定位
每一步消息中都会提供<窗口和元素位置参考>，这是系统通过Accessibility API获取的真实窗口/控件位置信息，包含名称和中心坐标(center_x, center_y)。这是最可靠的定位方式！
- 如果你要点击的元素在参考列表中，直接使用其center_x和center_y作为坐标
- 例如：参考列表中有 "腾讯QQ" at (960, 540)，则点击QQ窗口中心就用 mouse_click x=960 y=540

步骤二：利用坐标刻度尺交叉定位
如果目标元素不在参考列表中，利用截图边缘的刻度尺：
- 找到目标元素，从该元素向左画一条水平线到左侧Y轴刻度尺，读取Y坐标
- 从该元素向上画一条垂直线到顶部X轴刻度尺，读取X坐标
- 两线交叉点就是目标元素的坐标

步骤三：相对偏移精确定位
当你知道某个参考元素的坐标，目标在其附近时：
- 目标在参考元素右边N像素：x = 参考x + N
- 目标在参考元素下方N像素：y = 参考y + N
- 桌面图标一般排列：第一列中心x约40-60，图标间距约90-100像素（竖向）
- 任务栏高度约40-50像素，在屏幕底部
- 窗口标题栏高度约30-35像素，关闭/最大化/最小化按钮在标题栏右侧
- 标题栏按钮从右到左依次是：关闭(距右边约20-45)、最大化(距右边约45-70)、最小化(距右边约70-95)

步骤四：元素类型适配
- 图标/小按钮（约32x32像素）：取其边界框的中心点坐标
- 文字标签/链接：取文字的水平中心x和垂直中间y
- 大区域（窗口/面板）：点击其标题区域或中心
- 输入框：点击其中心偏左位置（文字输入起始处）
- 下拉菜单项：取该项的中心坐标
- 复选框/单选按钮：取其图标中心
- 棋子等游戏元素：取该元素视觉中心，必须非常精确（偏差不超过5像素）

关键原则：宁可多花一秒仔细观察刻度尺和参考信息，也不要凭感觉猜坐标。坐标偏差10像素就可能导致点击到完全不同的元素。
</坐标精准定位方法论>

<持续任务执行模式>
你是一个能够持续自主运行的Agent，不是一次性问答机器人。对于复杂任务：

1.【任务分解】：将大任务拆分为可执行的原子步骤序列
2.【逐步执行】：每一步只做一件事，然后观察结果
3.【状态追踪】：在思考中维护当前任务进度，明确"已完成什么、正在做什么、接下来做什么"
4.【验证确认】：每步操作后通过新截图验证结果，确认成功再继续
5.【自动恢复】：遇到错误不要放弃，分析原因并尝试替代方案
6.【持续运行】：不要因为遇到困难就调用task_complete，除非真正完成了目标或确认无法继续
7.【等待策略】：对于需要等待的场景（对方下棋、页面加载、程序启动），使用wait工具等待足够时间后继续，不要提前放弃

典型持续任务示例 - 下象棋：
1. 观察棋盘，识别当前局势
2. 决定要移动的棋子和目标位置
3. 精确定位棋子中心坐标和目标位置中心坐标
4. 执行mouse_drag从棋子位置拖到目标位置
5. wait等待对方下棋（可能需要等待5-15秒）
6. 观察对方是否已经下棋（截图对比）
7. 如果对方还没下，继续wait
8. 对方下棋后，分析新局势，重复步骤2-7
9. 棋局结束时调用task_complete报告结果
</持续任务执行模式>

<核心工作流 - 每一步必须严格遵循>
1.【观察】仔细查看截图，识别所有可见的UI元素。特别关注：当前活跃窗口、按钮、文字、图标、输入框、菜单项等
2.【定位】按照<坐标精准定位方法论>的四个步骤，确定目标元素的中心像素坐标(x,y)
3.【思考】在回复中说明你的观察结果、定位依据和执行计划
4.【执行】调用一个工具来执行操作
5.【验证】下一轮你会收到新的截图来确认操作结果
</核心工作流>

<关键规则>
1. 每轮只调用一个工具。执行后会自动获得新截图来验证结果
2. 坐标必须是单个整数，绝对不能返回数组、范围或字符串
3. 优先使用<窗口和元素位置参考>中的坐标，这是最精确的定位方式
4. 打开应用程序优先使用open_application工具，比在桌面找图标更可靠
5. 如果无法精确定位目标元素，优先使用键盘快捷键或搜索方式替代
6. 切换窗口用keyboard_hotkey(["alt","tab"])，关闭窗口用keyboard_hotkey(["alt","f4"])
7. 拖拽操作（如移动棋子、拖动文件）：必须精确获取起点和终点的中心坐标。对于棋子，要先找到棋子的视觉中心作为from坐标，再找到目标格子的中心作为to坐标
8. 如果连续3次在同一操作上失败，换一种方法（如用键盘代替鼠标，用搜索代替浏览）
9. 对于需要等待的操作（如打开应用、加载页面、等待对手），使用wait工具等待足够时间
10. 输入文字前，必须先点击目标输入框使其获得焦点
11. 截图中鼠标光标的位置也提供了定位参考
12. 在你的思考中要明确说明定位依据，如"根据元素参考，QQ窗口中心在(960,540)"或"根据刻度尺，目标x约在800处，y约在300处"
</关键规则>

<坐标格式要求 - 极其重要>
x和y参数必须各是一个整数，例如：
正确：{{"x": 960, "y": 540}}
正确：{{"x": 54, "y": 60}}
错误：{{"x": "27,54"}} — 不能是字符串
错误：{{"x": [27,54]}} — 不能是数组
永远只返回一个整数作为x的值，一个整数作为y的值
</坐标格式要求>

<常见操作策略>
- 打开应用：优先用open_application(name)，比在桌面找图标更可靠
- 文件管理：用keyboard_hotkey(["win","e"])打开文件资源管理器
- 搜索功能：用keyboard_hotkey(["ctrl","f"])
- 复制粘贴：先keyboard_hotkey(["ctrl","c"])，再keyboard_hotkey(["ctrl","v"])
- 全选：keyboard_hotkey(["ctrl","a"])
- 撤销：keyboard_hotkey(["ctrl","z"])
- 保存：keyboard_hotkey(["ctrl","s"])
- 右键菜单：mouse_click指定button="right"
- 滚动页面：mouse_scroll，amount=3约等于滚轮一格
- 输入中文：直接用keyboard_type输入中文文本
- 切换输入法：keyboard_hotkey(["ctrl","shift"])或["win","space"]
- 下棋/拖拽：用mouse_drag，必须精准定位起点（棋子视觉中心）和终点（目标格子中心）。棋子中心通常在格子正中央
- 打开开始菜单：keyboard_key("meta")或keyboard_hotkey(["win"])
- 运行对话框：keyboard_hotkey(["win","r"])
- 任务管理器：keyboard_hotkey(["ctrl","shift","escape"])
- 截图工具：keyboard_hotkey(["win","shift","s"])
- 鼠标悬停预览：用mouse_move移动到目标位置
</常见操作策略>

<错误恢复策略>
- 点击没有反应：重新观察截图，用元素位置参考重新确认坐标，或尝试键盘快捷键
- 找不到目标元素：尝试滚动页面，或使用搜索功能(Ctrl+F)定位
- 窗口被遮挡：先切换窗口(Alt+Tab)或最小化遮挡窗口
- 输入框无法输入：先点击输入框确保获得焦点，等待200ms后再输入
- 程序未响应：等待更长时间(2-3秒)，或关闭后重新打开
- 拖拽没到位：检查起点和终点坐标是否准确，可能需要调整坐标后重试
- 点击了错误元素：观察新截图确认错误，然后用正确坐标重试，或用Esc取消当前操作
- 目标元素在屏幕外：滚动页面或调整窗口位置后重试
</错误恢复策略>"#,
    dw = display_w,
    dh = display_h,
    )
}

fn build_tool_definitions() -> serde_json::Value {
    serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "mouse_move",
                "description": "将鼠标光标移动到屏幕坐标(x,y)处，不点击。用于悬停预览、定位光标、为后续操作做准备。x和y必须各是一个整数。",
                "parameters": {"type": "object", "properties": {
                    "x": {"type": "integer", "description": "目标X坐标(单个整数)"},
                    "y": {"type": "integer", "description": "目标Y坐标(单个整数)"}
                }, "required": ["x","y"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "mouse_click",
                "description": "在屏幕坐标(x,y)处点击鼠标。先移动到目标位置再点击。x和y必须各是一个整数。默认左键。",
                "parameters": {"type": "object", "properties": {
                    "x": {"type": "integer", "description": "X坐标(单个整数)"},
                    "y": {"type": "integer", "description": "Y坐标(单个整数)"},
                    "button": {"type": "string", "enum": ["left","right","middle"], "description": "鼠标按键，默认left"}
                }, "required": ["x","y"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "mouse_double_click",
                "description": "在屏幕坐标(x,y)处双击鼠标左键。用于打开文件、启动程序、打开桌面图标等。x和y必须各是一个整数。",
                "parameters": {"type": "object", "properties": {
                    "x": {"type": "integer", "description": "X坐标(单个整数)"},
                    "y": {"type": "integer", "description": "Y坐标(单个整数)"}
                }, "required": ["x","y"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "mouse_drag",
                "description": "从起点坐标按住鼠标左键拖拽到终点坐标，然后释放。用于移动棋子、拖动文件、调整窗口大小等。关键：from坐标必须是源元素的视觉中心，to坐标必须是目标位置的视觉中心。所有坐标必须是单个整数。例如下棋：from坐标是棋子中心，to坐标是目标格子中心。",
                "parameters": {"type": "object", "properties": {
                    "from_x": {"type": "integer", "description": "起点X坐标（源元素视觉中心）"},
                    "from_y": {"type": "integer", "description": "起点Y坐标（源元素视觉中心）"},
                    "to_x": {"type": "integer", "description": "终点X坐标（目标位置视觉中心）"},
                    "to_y": {"type": "integer", "description": "终点Y坐标（目标位置视觉中心）"}
                }, "required": ["from_x","from_y","to_x","to_y"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "mouse_scroll",
                "description": "在指定位置滚动鼠标滚轮。direction为up或down，amount为滚动次数(1次约等于滚轮一格)。",
                "parameters": {"type": "object", "properties": {
                    "x": {"type": "integer", "description": "X坐标"},
                    "y": {"type": "integer", "description": "Y坐标"},
                    "direction": {"type": "string", "enum": ["up","down"], "description": "滚动方向"},
                    "amount": {"type": "integer", "description": "滚动量，默认3"}
                }, "required": ["x","y","direction","amount"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "keyboard_type",
                "description": "输入文字字符串。支持中英文和各种符号。输入前请确保光标已在目标输入框中。",
                "parameters": {"type": "object", "properties": {
                    "text": {"type": "string", "description": "要输入的文字"}
                }, "required": ["text"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "keyboard_key",
                "description": "按下单个按键，可带修饰键。常用键：enter, tab, escape, backspace, delete, up, down, left, right, home, end, space, meta(win)。",
                "parameters": {"type": "object", "properties": {
                    "key": {"type": "string", "description": "按键名称"},
                    "modifiers": {"type": "array", "items": {"type": "string"}, "description": "修饰键列表：ctrl, alt, shift, meta(win)"}
                }, "required": ["key"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "keyboard_hotkey",
                "description": "同时按下组合键。例如：复制[\"ctrl\",\"c\"]，粘贴[\"ctrl\",\"v\"]，切换窗口[\"alt\",\"tab\"]，关闭窗口[\"alt\",\"f4\"]，打开资源管理器[\"win\",\"e\"]。",
                "parameters": {"type": "object", "properties": {
                    "keys": {"type": "array", "items": {"type": "string"}, "description": "组合键列表，如[\"ctrl\",\"c\"]"}
                }, "required": ["keys"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "open_application",
                "description": "通过名称打开应用程序，比在桌面找图标更可靠。例如：\"记事本\"、\"计算器\"、\"Chrome\"、\"Word\"、\"Excel\"。",
                "parameters": {"type": "object", "properties": {
                    "name": {"type": "string", "description": "应用程序名称"}
                }, "required": ["name"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "wait",
                "description": "等待指定毫秒数。用于等待程序打开、页面加载、动画完成等。打开应用后建议等待1000-2000ms。",
                "parameters": {"type": "object", "properties": {
                    "ms": {"type": "integer", "description": "等待毫秒数，建议500-3000"}
                }, "required": ["ms"]}
            }
        },
        {
            "type": "function",
            "function": {
                "name": "task_complete",
                "description": "任务完成时调用，附带完成总结。",
                "parameters": {"type": "object", "properties": {
                    "summary": {"type": "string", "description": "任务完成总结"}
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

/// Fix coordinates that come as strings (e.g. "27,54") or arrays by extracting the first valid integer.
fn fix_coordinate(val: &serde_json::Value) -> i64 {
    // Direct integer
    if let Some(n) = val.as_i64() {
        return n;
    }
    // String that might contain a number or comma-separated numbers
    if let Some(s) = val.as_str() {
        // Try parsing as direct number
        if let Ok(n) = s.parse::<i64>() {
            return n;
        }
        // Try extracting first number from comma-separated (e.g. "27, 54" → 27)
        for part in s.split(&[',', ' ', '[', ']', '"']) {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                if let Ok(n) = trimmed.parse::<i64>() {
                    return n;
                }
            }
        }
    }
    // Array: take first element
    if let Some(arr) = val.as_array() {
        if let Some(first) = arr.first() {
            return fix_coordinate(first);
        }
    }
    0
}

/// Fix tool call arguments - ensure coordinates are proper integers.
fn fix_tool_call_args(action: &mut ToolCallInfo) {
    let coord_keys = ["x", "y", "from_x", "from_y", "to_x", "to_y"];
    for key in &coord_keys {
        if let Some(val) = action.arguments.get(key) {
            if !val.is_i64() {
                let fixed = fix_coordinate(val);
                log::warn!(
                    "[agent_loop] Fixed coordinate '{}' from {} to {}",
                    key, val, fixed
                );
                action.arguments.as_object_mut().unwrap().insert(
                    key.to_string(),
                    serde_json::Value::Number(fixed.into())
                );
            }
        }
    }
}

fn execute_tool_call(action: &mut ToolCallInfo, sx: f64, sy: f64) -> Result<String, String> {
    // Fix any malformed coordinates before execution
    fix_tool_call_args(action);

    let args = &action.arguments;
    let sm = ScreenMapping::new(sx, sy);

    match action.name.as_str() {
        "mouse_move" => {
            let x = sm.x(args.get("x").and_then(|v| v.as_i64()).unwrap_or(0));
            let y = sm.y(args.get("y").and_then(|v| v.as_i64()).unwrap_or(0));
            log::info!("[agent_loop] mouse_move logical=({},{})", x, y);
            let mut e = new_agent_enigo()?;
            smooth_move_to(&mut e, x, y)?;
            Ok(format!("已移动鼠标到({},{})", x, y))
        }
        "mouse_click" => {
            let x = sm.x(args.get("x").and_then(|v| v.as_i64()).unwrap_or(0));
            let y = sm.y(args.get("y").and_then(|v| v.as_i64()).unwrap_or(0));
            log::info!("[agent_loop] mouse_click logical=({},{})", x, y);
            let mut e = new_agent_enigo()?;
            smooth_move_to(&mut e, x, y)?;
            std::thread::sleep(std::time::Duration::from_millis(80));
            let btn = match args.get("button").and_then(|v| v.as_str()) {
                Some("right") => EnigoButton::Right,
                Some("middle") => EnigoButton::Middle,
                _ => EnigoButton::Left,
            };
            e.button(btn, EnigoDir::Click).map_err(|e| e.to_string())?;
            Ok(format!("已在({},{})处点击鼠标", x, y))
        }
        "mouse_double_click" => {
            let x = sm.x(args.get("x").and_then(|v| v.as_i64()).unwrap_or(0));
            let y = sm.y(args.get("y").and_then(|v| v.as_i64()).unwrap_or(0));
            let mut e = new_agent_enigo()?;
            smooth_move_to(&mut e, x, y)?;
            std::thread::sleep(std::time::Duration::from_millis(60));
            e.button(EnigoButton::Left, EnigoDir::Click).map_err(|e| e.to_string())?;
            std::thread::sleep(std::time::Duration::from_millis(80));
            e.button(EnigoButton::Left, EnigoDir::Click).map_err(|e| e.to_string())?;
            Ok(format!("已在({},{})处双击鼠标", x, y))
        }
        "mouse_drag" => {
            let fx = sm.x(args.get("from_x").and_then(|v| v.as_i64()).unwrap_or(0));
            let fy = sm.y(args.get("from_y").and_then(|v| v.as_i64()).unwrap_or(0));
            let tx = sm.x(args.get("to_x").and_then(|v| v.as_i64()).unwrap_or(0));
            let ty = sm.y(args.get("to_y").and_then(|v| v.as_i64()).unwrap_or(0));
            let mut e = new_agent_enigo()?;
            smooth_move_to(&mut e, fx, fy)?;
            std::thread::sleep(std::time::Duration::from_millis(80));
            e.button(EnigoButton::Left, EnigoDir::Press).map_err(|er| er.to_string())?;
            std::thread::sleep(std::time::Duration::from_millis(60));
            let steps = 20;
            let dx = (tx - fx) as f64;
            let dy = (ty - fy) as f64;
            for i in 1..=steps {
                let t = i as f64 / steps as f64;
                let ease = t * t * (3.0 - 2.0 * t);
                let cx = fx + (dx * ease).round() as i32;
                let cy = fy + (dy * ease).round() as i32;
                e.move_mouse(cx, cy, EnigoCoord::Abs).map_err(|er| er.to_string())?;
                std::thread::sleep(std::time::Duration::from_millis(12));
            }
            e.move_mouse(tx, ty, EnigoCoord::Abs).map_err(|er| er.to_string())?;
            std::thread::sleep(std::time::Duration::from_millis(50));
            e.button(EnigoButton::Left, EnigoDir::Release).map_err(|er| er.to_string())?;
            Ok(format!("已从({},{})拖拽到({},{})", fx, fy, tx, ty))
        }
        "mouse_scroll" => {
            let x = sm.x(args.get("x").and_then(|v| v.as_i64()).unwrap_or(0));
            let y = sm.y(args.get("y").and_then(|v| v.as_i64()).unwrap_or(0));
            let mut e = new_agent_enigo()?;
            smooth_move_to(&mut e, x, y)?;
            std::thread::sleep(std::time::Duration::from_millis(50));
            let dir = args.get("direction").and_then(|v| v.as_str()).unwrap_or("down");
            let amt = args.get("amount").and_then(|v| v.as_i64()).unwrap_or(3) as i32;
            let scroll_val = if dir == "up" { amt } else { -amt };
            e.scroll(scroll_val, enigo::Axis::Vertical).map_err(|er| er.to_string())?;
            Ok(format!("已在({},{})处向{}滚动{}", x, y, if dir == "up" { "上" } else { "下" }, amt))
        }
        "keyboard_type" => {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
            crate::commands::input::keyboard_type(text.clone())?;
            let display = if text.len() > 50 { &text[..50] } else { &text };
            Ok(format!("已输入文字：{}", display))
        }
        "keyboard_key" => {
            let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let mods = args.get("modifiers").and_then(|v| v.as_array()).map(|arr| {
                arr.iter().filter_map(|m| m.as_str().map(String::from)).collect()
            });
            crate::commands::input::keyboard_key(key.clone(), mods)?;
            Ok(format!("已按键：{}", key))
        }
        "keyboard_hotkey" => {
            let keys: Vec<String> = args.get("keys").and_then(|v| v.as_array()).map(|arr| {
                arr.iter().filter_map(|k| k.as_str().map(String::from)).collect()
            }).unwrap_or_default();
            crate::commands::input::keyboard_hotkey(keys.clone())?;
            Ok(format!("已按组合键：{}", keys.join("+")))
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
            // Wait for application to start
            std::thread::sleep(std::time::Duration::from_millis(2000));
            Ok(format!("已打开应用：{}", name))
        }
        "wait" => {
            let ms = args.get("ms").and_then(|v| v.as_u64()).unwrap_or(500);
            std::thread::sleep(std::time::Duration::from_millis(ms));
            Ok(format!("已等待{}毫秒", ms))
        }
        "task_complete" => {
            let summary = args.get("summary").and_then(|v| v.as_str()).unwrap_or("Task completed").to_string();
            Ok(format!("DONE:{}", summary))
        }
        _ => Err(format!("未知工具：{}", action.name)),
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
        None => 200, // Increased from 50 to 200 for sustained tasks like chess games
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
    let mut consecutive_failures = 0u32;

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

            // Few-shot example 1: Double-click desktop icon using element reference
            history.push(serde_json::json!({
                "role": "user",
                "content": "请双击桌面上的\"此电脑\"图标来打开它。"
            }));
            history.push(serde_json::json!({
                "role": "assistant",
                "content": "我观察到截面上有桌面图标。根据元素位置参考，\"此电脑\"在坐标(54,60)附近。我需要双击这个图标。",
                "tool_calls": [{
                    "id": "fewshot_1",
                    "type": "function",
                    "function": {
                        "name": "mouse_double_click",
                        "arguments": "{\"x\": 54, \"y\": 60}"
                    }
                }]
            }));
            history.push(serde_json::json!({
                "role": "tool",
                "tool_call_id": "fewshot_1",
                "content": "已在(54, 60)处双击鼠标。"
            }));

            // Few-shot example 2: Drag chess piece
            history.push(serde_json::json!({
                "role": "user",
                "content": "第2步。屏幕：1920x1080像素。任务目标：帮我下棋\n请观察上一步操作的结果，决定下一步操作。\n\n<窗口和元素位置参考>\n\"中国象棋\" at (960,500) size 800x600\n</窗口和元素位置参考>"
            }));
            history.push(serde_json::json!({
                "role": "assistant",
                "content": "我观察到象棋窗口已打开。当前我需要走炮。根据截图观察，炮的视觉中心大约在(720,340)，目标位置中心大约在(720,200)。我将拖拽炮到目标位置。",
                "tool_calls": [{
                    "id": "fewshot_2",
                    "type": "function",
                    "function": {
                        "name": "mouse_drag",
                        "arguments": "{\"from_x\": 720, \"from_y\": 340, \"to_x\": 720, \"to_y\": 200}"
                    }
                }]
            }));
            history.push(serde_json::json!({
                "role": "tool",
                "tool_call_id": "fewshot_2",
                "content": "已从(720,340)拖拽到(720,200)"
            }));
        }

        let step_text = if step == 1 {
            let mut text = format!("任务目标：{}\n这是当前屏幕截图({}x{}像素)。请按照<坐标精准定位方法论>仔细观察截图和坐标刻度尺，识别屏幕上的元素，决定第一步操作。\n重要：优先使用<窗口和元素位置参考>中的坐标来定位目标。坐标必须是单个整数。", goal, disp_w, disp_h);
            // Add accessibility element info as coordinate references (increased limit to 3000)
            if let Ok(elements) = crate::commands::desktop::get_screen_elements() {
                if !elements.is_empty() {
                    text.push_str("\n\n<窗口和元素位置参考>\n");
                    let elided = if elements.len() > 4000 { &elements[..4000] } else { &elements };
                    text.push_str(elided);
                    text.push_str("\n</窗口和元素位置参考>");
                    text.push_str("\n重要：[桌面图标] 的坐标是最精确的，直接使用其坐标即可。其他元素的坐标也是精确的。优先在参考列表中查找目标元素的坐标。");
                }
            }
            text
        } else {
            let mut text = format!("第{}步。屏幕：{}x{}像素。任务目标：{}\n请观察上一步操作的结果，决定下一步操作。\n重要提醒：\n- 优先使用下方元素位置参考中的坐标来定位，这是最精确的定位方式\n- [桌面图标] 的坐标是精确的屏幕坐标，直接使用\n- 如果上一步操作失败了，分析失败原因并换一种方法\n- 如果任务还没完成，继续执行下一步，不要放弃\n- 在思考中说明你的定位依据（参考哪个元素的坐标、或根据刻度尺估算）", step, disp_w, disp_h, goal);
            // Add accessibility info on later steps too (increased limit to 4000 for desktop icons)
            if let Ok(elements) = crate::commands::desktop::get_screen_elements() {
                if !elements.is_empty() {
                    text.push_str("\n\n<窗口和元素位置参考>\n");
                    let elided = if elements.len() > 4000 { &elements[..4000] } else { &elements };
                    text.push_str(elided);
                    text.push_str("\n</窗口和元素位置参考>");
                }
            }
            // Add mouse cursor position for reference
            text.push_str(&format!("\n当前鼠标位置：({}, {})" , capture.cursor_display_x, capture.cursor_display_y));
            text
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

        // History management: keep system + few-shot examples + recent turns.
        // Few-shot messages are at index 1..7 (2 examples * 3 messages each = 6).
        // After that, keep last 20 messages (10 turns) for context - longer for sustained tasks.
        let few_shot_count = 6; // 2 few-shot examples * 3 messages each
        let max_recent = 20; // Increased from 8 to 20 for sustained task context
        let max_history = 1 + few_shot_count + max_recent;
        if history.len() > max_history {
            let sys = history[0].clone();
            let few_shot: Vec<_> = history[1..1+few_shot_count].to_vec();
            let tail = history[history.len() - max_recent..].to_vec();
            history = vec![sys];
            history.extend(few_shot);
            history.extend(tail);
        }

        let response = if is_anthropic {
            call_anthropic_vision(&client, endpoint, api_key, model_id, &history, &tools).await?
        } else {
            call_openai_vision(&client, endpoint, api_key, model_id, &history, &tools).await?
        };

        let (thought, mut tool_calls) = parse_model_response(&response, is_anthropic);

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

        for tc in &mut tool_calls {
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
                    exec_results.push(format!("操作 {} 被用户拒绝", tc.name));
                    continue;
                }
            }

            match execute_tool_call(tc, scale_x, scale_y) {
                Ok(result) => {
                    exec_results.push(result);
                    consecutive_failures = 0;
                }
                Err(e) => {
                    exec_results.push(format!("操作出错：{}", e));
                    consecutive_failures += 1;
                    if consecutive_failures >= 5 {
                        log::warn!("[agent_loop] Too many consecutive failures, stopping");
                        app.emit("computer-use:error", AgentError {
                            session_id: session_id.to_string(),
                            step,
                            error: "连续5次操作失败，已自动停止".to_string(),
                        })?;
                        break;
                    }
                }
            }

            // Wait for UI to update after action (increased waits for better reliability)
            let wait_ms = match tc.name.as_str() {
                "open_application" => 2000,
                "mouse_double_click" => 1000,
                "mouse_drag" => 800,
                "keyboard_hotkey" => 600,
                "mouse_click" => 500,
                "keyboard_type" => 400,
                "mouse_move" => 300,
                _ => 400,
            };
            tokio::time::sleep(tokio::time::Duration::from_millis(wait_ms)).await;
        }

        if task_done { break; }

        // Build assistant message with tool calls for proper conversation format
        if !tool_calls.is_empty() {
            let tool_calls_json: Vec<serde_json::Value> = tool_calls.iter().enumerate().map(|(i, tc)| {
                serde_json::json!({
                    "id": format!("tc_{}_{}", step, i),
                    "type": "function",
                    "function": {
                        "name": tc.name,
                        "arguments": serde_json::to_string(&tc.arguments).unwrap_or_default()
                    }
                })
            }).collect();

            history.push(serde_json::json!({
                "role": "assistant",
                "content": if thought.is_empty() { "" } else { &thought },
                "tool_calls": tool_calls_json
            }));

            // Add tool results
            for (i, result) in exec_results.iter().enumerate() {
                history.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": format!("tc_{}_{}", step, i),
                    "content": result
                }));
            }
        } else {
            // No tool calls, just assistant text
            history.push(serde_json::json!({
                "role": "assistant",
                "content": if thought.is_empty() { "我需要更多观察" } else { &thought }
            }));
        }

        // Additional wait before next screenshot to let UI settle
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
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
    // Try /chat/completions first (for proxies like One API), then /v1/chat/completions
    let urls = [
        format!("{}/chat/completions", endpoint),
        format!("{}/v1/chat/completions", endpoint),
    ];

    let messages: Vec<serde_json::Value> = history.to_vec();
    let body = serde_json::json!({
        "model": model_id,
        "messages": messages,
        "tools": tools,
        "tool_choice": "auto",
        "max_tokens": 4096,
        "temperature": 0.2
    });

    let mut last_error = String::new();
    for url in &urls {
        let resp = client.post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send().await;

        match resp {
            Ok(resp) => {
                let status = resp.status();
                let body_text = resp.text().await.unwrap_or_default();
                if status.is_success() {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_text) {
                        return Ok(val);
                    }
                    return Err(format!("Invalid JSON response: {}", &body_text[..body_text.len().min(500)]).into());
                }
                // If we get 404, try next URL
                if status.as_u16() == 404 {
                    last_error = format!("API 404 at {}", url);
                    continue;
                }
                return Err(format!("Vision API error {}: {}", status, &body_text[..body_text.len().min(500)]).into());
            }
            Err(e) => {
                last_error = e.to_string();
                continue;
            }
        }
    }

    Err(format!("All API endpoints failed: {}", last_error).into())
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
        "max_tokens": 4096,
        "temperature": 0.2
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
