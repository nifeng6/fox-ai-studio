use std::collections::HashMap;
use std::sync::Mutex;

// ── Data Structures ──

#[derive(Debug, Clone)]
pub struct ScreenshotDiff {
    pub similarity: f64,
    pub changed: bool,
    pub change_description: String,
}

#[derive(Debug, Clone)]
pub struct AdaptiveWaitConfig {
    pub min_wait_ms: u64,
    pub max_wait_ms: u64,
    pub poll_interval_ms: u64,
    pub similarity_threshold: f64,
    pub max_polls: u32,
}

impl Default for AdaptiveWaitConfig {
    fn default() -> Self {
        AdaptiveWaitConfig {
            min_wait_ms: 1000,
            max_wait_ms: 30000,
            poll_interval_ms: 1500,
            similarity_threshold: 0.95,
            max_polls: 15,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FailureRecord {
    pub tool_name: String,
    pub error: String,
    pub step: u32,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub enum RetryDecision {
    Retry { delay_ms: u64 },
    SwitchStrategy { suggestion: String },
    GiveUp { reason: String },
}

#[derive(Debug, Clone, Default)]
struct SessionReflection {
    failure_history: Vec<FailureRecord>,
    last_screenshot_hash: Option<String>,
    consecutive_same_action_failures: u32,
}

// ── State Manager ──

pub struct ReflectionStateManager {
    sessions: Mutex<HashMap<String, SessionReflection>>,
}

impl ReflectionStateManager {
    pub fn new() -> Self {
        ReflectionStateManager {
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

// ── Screenshot Comparison ──

/// Compare two base64-encoded screenshots to determine if the screen changed.
/// Fast path: exact base64 equality → SHA256 hash → downsampled pixel comparison.
pub fn compare_screenshots(prev_b64: &str, curr_b64: &str) -> ScreenshotDiff {
    // Fast path: exact equality
    if prev_b64 == curr_b64 {
        return ScreenshotDiff {
            similarity: 1.0,
            changed: false,
            change_description: "完全相同".to_string(),
        };
    }

    // Decode both images and compare at low resolution
    let prev_img = decode_jpeg_base64(prev_b64);
    let curr_img = decode_jpeg_base64(curr_b64);

    let (prev_small, curr_small) = match (prev_img, curr_img) {
        (Some(p), Some(c)) => (p, c),
        _ => {
            // If we can't decode, compare hash strings as fallback
            use std::hash::{Hash, Hasher};
            let mut h1 = std::collections::hash_map::DefaultHasher::new();
            let mut h2 = std::collections::hash_map::DefaultHasher::new();
            prev_b64.hash(&mut h1);
            curr_b64.hash(&mut h2);
            let same = h1.finish() == h2.finish();
            return ScreenshotDiff {
                similarity: if same { 1.0 } else { 0.5 },
                changed: !same,
                change_description: if same { "哈希相同" } else { "哈希不同" }.to_string(),
            };
        }
    };

    // Compare at low resolution (160x120)
    let target_w = 160u32;
    let target_h = 120u32;
    let p_resized = resize_for_comparison(&prev_small, target_w, target_h);
    let c_resized = resize_for_comparison(&curr_small, target_w, target_h);

    let mut diff_pixels = 0u32;
    let total_pixels = target_w * target_h;
    let tolerance = 15i32; // per-channel tolerance

    for y in 0..target_h {
        for x in 0..target_w {
            let p = p_resized.get_pixel(x, y);
            let c = c_resized.get_pixel(x, y);
            let dr = (p[0] as i32 - c[0] as i32).abs();
            let dg = (p[1] as i32 - c[1] as i32).abs();
            let db = (p[2] as i32 - c[2] as i32).abs();
            if dr > tolerance || dg > tolerance || db > tolerance {
                diff_pixels += 1;
            }
        }
    }

    let similarity = 1.0 - (diff_pixels as f64 / total_pixels as f64);
    let changed = similarity < 0.95;

    let change_description = if similarity > 0.99 {
        "几乎无变化".to_string()
    } else if similarity > 0.95 {
        "微小变化".to_string()
    } else if similarity > 0.8 {
        format!("明显变化 (相似度: {:.0}%)", similarity * 100.0)
    } else {
        format!("剧烈变化 (相似度: {:.0}%)", similarity * 100.0)
    };

    ScreenshotDiff {
        similarity,
        changed,
        change_description,
    }
}

fn decode_jpeg_base64(b64: &str) -> Option<image::DynamicImage> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    image::load_from_memory(&bytes).ok()
}

fn resize_for_comparison(img: &image::DynamicImage, w: u32, h: u32) -> image::RgbaImage {
    image::imageops::resize(img, w, h, image::imageops::FilterType::Nearest)
}

// ── Adaptive Wait ──

/// Instead of a fixed wait, poll the screen until it changes.
/// Returns true if screen changed within limits, false if timed out.
pub fn adaptive_wait(
    prev_screenshot_b64: &str,
    config: &AdaptiveWaitConfig,
) -> bool {
    let mut polls = 0u32;
    let mut waited = 0u64;

    // Minimum wait before first poll
    std::thread::sleep(std::time::Duration::from_millis(config.min_wait_ms));
    waited += config.min_wait_ms;

    while polls < config.max_polls && waited < config.max_wait_ms {
        match crate::commands::desktop::capture_clean_screenshot(None) {
            Ok(capture) => {
                let diff = compare_screenshots(prev_screenshot_b64, &capture.jpeg_base64);
                if diff.changed {
                    log::info!("[reflection] Screen changed after {}ms: {}", waited, diff.change_description);
                    return true;
                }
            }
            Err(e) => {
                log::warn!("[reflection] Screenshot failed during adaptive wait: {}", e);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(config.poll_interval_ms));
        waited += config.poll_interval_ms;
        polls += 1;
    }

    log::info!("[reflection] Adaptive wait timed out after {}ms ({} polls)", waited, polls);
    false
}

// ── Failure Pattern Detection ──

/// Record a tool execution failure for pattern detection
pub fn record_failure(
    state: &ReflectionStateManager,
    session_id: &str,
    tool_name: &str,
    error: &str,
    step: u32,
) {
    let mut sessions = state.sessions.lock().unwrap();
    let session = sessions.entry(session_id.to_string()).or_insert_with(SessionReflection::default);

    // Check if same tool as last failure
    if let Some(last) = session.failure_history.last() {
        if last.tool_name == tool_name {
            session.consecutive_same_action_failures += 1;
        } else {
            session.consecutive_same_action_failures = 1;
        }
    } else {
        session.consecutive_same_action_failures = 1;
    }

    session.failure_history.push(FailureRecord {
        tool_name: tool_name.to_string(),
        error: error.to_string(),
        step,
        timestamp: chrono::Utc::now().timestamp_millis(),
    });

    log::warn!(
        "[reflection] Recorded failure #{} for '{}' at step {}: {}",
        session.consecutive_same_action_failures,
        tool_name, step, error
    );
}

/// Check if there's a failure pattern for this session
pub fn check_failure_pattern(
    state: &ReflectionStateManager,
    session_id: &str,
) -> Option<String> {
    let sessions = state.sessions.lock().unwrap();
    let session = sessions.get(session_id)?;

    if session.consecutive_same_action_failures >= 3 {
        let last = session.failure_history.last()?;
        Some(format!(
            "工具 '{}' 连续失败 {} 次，建议切换策略",
            last.tool_name,
            session.consecutive_same_action_failures
        ))
    } else {
        None
    }
}

/// Get the count of consecutive same-action failures
pub fn get_consecutive_failures(
    state: &ReflectionStateManager,
    session_id: &str,
) -> u32 {
    let sessions = state.sessions.lock().unwrap();
    sessions.get(session_id)
        .map(|s| s.consecutive_same_action_failures)
        .unwrap_or(0)
}

/// Suggest an alternative strategy based on the failed tool
pub fn suggest_strategy(tool_name: &str) -> String {
    match tool_name {
        "mouse_click" | "mouse_double_click" => {
            "尝试用键盘快捷键代替鼠标点击，或使用 open_application 打开程序".to_string()
        }
        "mouse_drag" => {
            "检查起点和终点坐标是否精确，尝试调整坐标后重试，或使用键盘操作代替拖拽".to_string()
        }
        "keyboard_type" => {
            "确保输入框已获得焦点（先点击输入框），尝试分步输入或使用剪贴板粘贴".to_string()
        }
        "open_application" => {
            "检查应用名称是否正确，尝试用开始菜单搜索或 desktop 图标双击打开".to_string()
        }
        _ => {
            "等待后重试，或换一种完全不同的方法完成任务".to_string()
        }
    }
}

/// Make a retry decision based on failure history
pub fn make_retry_decision(
    state: &ReflectionStateManager,
    session_id: &str,
    tool_name: &str,
    global_consecutive_failures: u32,
) -> RetryDecision {
    let consecutive = get_consecutive_failures(state, session_id);
    let total = global_consecutive_failures.max(consecutive);

    if total < 3 {
        // Retry with exponential backoff: 500ms, 1s, 2s, 4s, capped at 10s
        let delay = (500u64 * 2u64.pow(total)).min(10_000);
        RetryDecision::Retry { delay_ms: delay }
    } else if total < 5 {
        // Switch strategy
        let suggestion = suggest_strategy(tool_name);
        RetryDecision::SwitchStrategy { suggestion }
    } else {
        // Give up current approach
        RetryDecision::GiveUp {
            reason: format!("工具 '{}' 连续失败 {} 次，当前方法不可行", tool_name, total),
        }
    }
}

/// Build a reflection prompt to inject into the LLM's next step
pub fn build_reflection_prompt(
    state: &ReflectionStateManager,
    session_id: &str,
) -> String {
    let sessions = state.sessions.lock().unwrap();
    let session = match sessions.get(session_id) {
        Some(s) => s,
        None => return String::new(),
    };

    if session.failure_history.is_empty() {
        return String::new();
    }

    let mut prompt = String::from("\n\n<反思与调整>\n最近执行遇到了问题，请仔细分析并调整策略：\n");

    // Summarize recent failures (last 5)
    let recent: Vec<_> = session.failure_history.iter().rev().take(5).collect();
    for (i, f) in recent.iter().enumerate() {
        prompt.push_str(&format!(
            "{}. 步骤{}: 工具 '{}' 失败 — {}\n",
            i + 1,
            f.step,
            f.tool_name,
            f.error
        ));
    }

    if session.consecutive_same_action_failures >= 3 {
        let last = session.failure_history.last().unwrap();
        let suggestion = suggest_strategy(&last.tool_name);
        prompt.push_str(&format!(
            "\n⚠️ 工具 '{}' 已连续失败 {} 次！必须切换策略：{}\n",
            last.tool_name,
            session.consecutive_same_action_failures,
            suggestion
        ));
    }

    prompt.push_str("</反思与调整>");
    prompt
}

/// Update the cached screenshot hash for dedup tracking
pub fn update_screenshot_hash(
    state: &ReflectionStateManager,
    session_id: &str,
    screenshot_b64: &str,
) {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    screenshot_b64.hash(&mut hasher);
    let hash = format!("{:016x}", hasher.finish());

    let mut sessions = state.sessions.lock().unwrap();
    let session = sessions.entry(session_id.to_string()).or_insert_with(SessionReflection::default);
    session.last_screenshot_hash = Some(hash);
}

/// Check if the current screenshot is the same as the previous one (screen frozen)
pub fn is_screen_frozen(
    state: &ReflectionStateManager,
    session_id: &str,
    screenshot_b64: &str,
) -> bool {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    screenshot_b64.hash(&mut hasher);
    let hash = format!("{:016x}", hasher.finish());

    let sessions = state.sessions.lock().unwrap();
    if let Some(session) = sessions.get(session_id) {
        session.last_screenshot_hash.as_ref() == Some(&hash)
    } else {
        false
    }
}

/// Clear session reflection data
pub fn clear_session(
    state: &ReflectionStateManager,
    session_id: &str,
) {
    let mut sessions = state.sessions.lock().unwrap();
    sessions.remove(session_id);
}
