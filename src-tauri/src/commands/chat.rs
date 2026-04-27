use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use reqwest::Client;
use futures_util::StreamExt;
use std::sync::Mutex;
use once_cell::sync::Lazy;

use crate::commands::provider::ProviderState;
use crate::commands::channel_types::{self, ChatPath};
use crate::commands::tools;

static ABORTED_MESSAGES: Lazy<Mutex<std::collections::HashSet<String>>> =
    Lazy::new(|| Mutex::new(std::collections::HashSet::new()));

// ── Message types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrlData },
    Image { source: ImageSourceData },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageUrlData {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageSourceData {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
}

impl ChatMessage {
    pub fn text_only(&self) -> String {
        match &self.content {
            MessageContent::Text(s) => s.clone(),
            MessageContent::Parts(parts) => {
                parts.iter().filter_map(|p| match p {
                    ContentPart::Text { text } => Some(text.clone()),
                    _ => None,
                }).collect::<Vec<_>>().join("\n")
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatOptions {
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub tools_enabled: Option<bool>,
    pub enabled_tool_ids: Option<Vec<String>>,
}

// ── Event payloads ─────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamChunk {
    pub message_id: String,
    pub chunk: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamThinking {
    pub message_id: String,
    pub chunk: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamToolCall {
    pub message_id: String,
    pub tool_call_id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamEnd {
    pub message_id: String,
    pub has_tool_calls: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamError {
    pub message_id: String,
    pub error: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    pub message_id: String,
    pub tool_call_id: String,
    pub name: String,
    pub result: String,
    pub success: bool,
}

// ── Commands ───────────────────────────────────────────────────────

#[tauri::command]
pub async fn send_chat_message(
    app: AppHandle,
    provider_id: String,
    model_id: String,
    messages: Vec<ChatMessage>,
    message_id: String,
    options: Option<ChatOptions>,
    state: State<'_, ProviderState>,
) -> Result<(), String> {
    let provider = {
        let providers = state.0.lock().map_err(|e| e.to_string())?;
        providers.iter().find(|p| p.id == provider_id).cloned()
            .or_else(|| {
                log::warn!("[chat] provider_id={} not found, searching for model={}", provider_id, model_id);
                providers.iter().find(|p| p.enabled && !p.api_key.is_empty() && p.models.iter().any(|m| m == &model_id)).cloned()
            })
            .or_else(|| {
                log::warn!("[chat] no provider with model={}, using first enabled", model_id);
                providers.iter().find(|p| p.enabled && !p.api_key.is_empty()).cloned()
            })
    };

    let app_clone = app.clone();
    let msg_id = message_id.clone();
    clear_aborted(&msg_id);

    tokio::spawn(async move {
        let result = if let Some(prov) = provider {
            log::info!("[chat] using provider: id={}, name={}, channel_type={}, model={}", prov.id, prov.name, prov.channel_type, model_id);
            do_stream(&app_clone, &prov, &model_id, &messages, &msg_id, options).await
        } else {
            log::warn!("[chat] NO provider found — using fallback");
            do_fallback_stream(&app_clone, &messages, &msg_id).await
        };

        if let Err(e) = result {
            log::error!("[chat] stream error for msg_id={}: {}", msg_id, e);
            let _ = app_clone.emit("chat:stream-error", StreamError {
                message_id: msg_id.clone(),
                error: e.to_string(),
            });
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn execute_chat_tool(
    name: String,
    arguments: serde_json::Value,
    tool_call_id: String,
    message_id: String,
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let tool = tools::ToolCallInfo {
        id: tool_call_id.clone(),
        name: name.clone(),
        arguments,
    };

    let (success, result_text) = match tools::execute_tool(&tool).await {
        Ok(r) => (true, r),
        Err(e) => (false, format!("Error: {}", e)),
    };

    let _ = app.emit("chat:tool-result", ToolResult {
        message_id: message_id.clone(),
        tool_call_id: tool_call_id.clone(),
        name: name.clone(),
        result: result_text.clone(),
        success,
    });

    Ok(serde_json::json!({
        "toolCallId": tool_call_id,
        "name": name,
        "result": result_text,
        "success": success
    }))
}

#[tauri::command]
pub async fn abort_chat(message_id: String) -> Result<(), String> {
    log::info!("[chat] abort requested for msg_id={}", message_id);
    if let Ok(mut set) = ABORTED_MESSAGES.lock() {
        set.insert(message_id);
    }
    Ok(())
}

fn is_aborted(message_id: &str) -> bool {
    ABORTED_MESSAGES.lock().map(|s| s.contains(message_id)).unwrap_or(false)
}

fn clear_aborted(message_id: &str) {
    if let Ok(mut set) = ABORTED_MESSAGES.lock() {
        set.remove(message_id);
    }
}

// ── Internal streaming ─────────────────────────────────────────────

async fn pick_api_key(raw: &str) -> String {
    let keys: Vec<&str> = raw.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if keys.is_empty() {
        return raw.to_string();
    }
    if keys.len() == 1 {
        return keys[0].to_string();
    }
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos() as usize)
        .unwrap_or(0);
    keys[seed % keys.len()].to_string()
}

async fn do_stream(
    app: &AppHandle,
    provider: &crate::commands::provider::Provider,
    model_id: &str,
    messages: &[ChatMessage],
    message_id: &str,
    options: Option<ChatOptions>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::builder()
        .connect_timeout(std::time::Duration::from_secs(120))
        .build()?;

    let ct = provider.channel_type;
    let base = &provider.api_endpoint;
    let key = pick_api_key(&provider.api_key).await;
    let key = key.as_str();

    match channel_types::get_chat_path(ct) {
        ChatPath::Anthropic => stream_anthropic(app, &client, ct, base, key, model_id, messages, message_id, &options).await,
        ChatPath::Gemini => stream_gemini(app, &client, ct, base, key, model_id, messages, message_id, &options).await,
        _ => stream_openai_compat(app, &client, ct, base, key, model_id, messages, message_id, &options).await,
    }
}

// ── Message building ───────────────────────────────────────────────

fn extract_text_content(msg: &ChatMessage) -> String {
    match &msg.content {
        MessageContent::Text(s) => s.clone(),
        MessageContent::Parts(parts) => {
            parts.iter().filter_map(|p| match p {
                ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            }).collect::<Vec<_>>().join("\n")
        }
    }
}

fn build_openai_messages(messages: &[ChatMessage]) -> Vec<serde_json::Value> {
    let mut system_parts: Vec<String> = Vec::new();
    let mut non_system: Vec<serde_json::Value> = Vec::new();

    for m in messages {
        if m.role == "tool" {
            let content_val = match &m.content {
                MessageContent::Text(s) => serde_json::json!(s),
                MessageContent::Parts(parts) => {
                    let arr: Vec<serde_json::Value> = parts.iter().map(|p| match p {
                        ContentPart::Text { text } => serde_json::json!({"type": "text", "text": text}),
                        ContentPart::ImageUrl { image_url } => serde_json::json!({
                            "type": "image_url",
                            "image_url": { "url": &image_url.url, "detail": image_url.detail.as_deref().unwrap_or("auto") }
                        }),
                        ContentPart::Image { source } => serde_json::json!({
                            "type": "image_url",
                            "image_url": { "url": format!("data:{};base64,{}", source.media_type, source.data) }
                        }),
                    }).collect();
                    serde_json::json!(arr)
                }
            };
            let mut msg = serde_json::json!({
                "role": "tool",
                "content": content_val,
            });
            if let Some(tcid) = &m.tool_call_id {
                msg["tool_call_id"] = serde_json::json!(tcid);
            }
            non_system.push(msg);
            continue;
        }

        // assistant messages with tool_calls need special handling
        if m.role == "assistant" {
            if let Some(tcs) = &m.tool_calls {
                if !tcs.is_empty() {
                    let content = extract_text_content(m);
                    let mut msg = serde_json::json!({"role": "assistant"});
                    if !content.is_empty() {
                        msg["content"] = serde_json::json!(content);
                    } else {
                        msg["content"] = serde_json::Value::Null;
                    }
                    // Sanitize tool_calls: fix invalid arguments JSON (e.g. doubled strings)
                    let sanitized: Vec<serde_json::Value> = tcs.iter().map(|tc| {
                        let mut tc = tc.clone();
                        if let Some(func) = tc.get_mut("function") {
                            if let Some(args_str) = func.get("arguments").and_then(|a| a.as_str()) {
                                if !args_str.is_empty() && serde_json::from_str::<serde_json::Value>(args_str).is_err() {
                                    if let Some(pos) = args_str.find("}{") {
                                        let first = &args_str[..pos + 1];
                                        if serde_json::from_str::<serde_json::Value>(first).is_ok() {
                                            func["arguments"] = serde_json::json!(first);
                                        } else {
                                            func["arguments"] = serde_json::json!("{}");
                                        }
                                    } else {
                                        func["arguments"] = serde_json::json!("{}");
                                    }
                                }
                            }
                        }
                        tc
                    }).collect();
                    msg["tool_calls"] = serde_json::json!(sanitized);
                    non_system.push(msg);
                    continue;
                }
            }
        }

        if m.role == "system" {
            let text = extract_text_content(m);
            if !text.is_empty() {
                system_parts.push(text);
            }
            continue;
        }

        let content_val = match &m.content {
            MessageContent::Text(s) => serde_json::json!(s),
            MessageContent::Parts(parts) => {
                let arr: Vec<serde_json::Value> = parts.iter().map(|p| match p {
                    ContentPart::Text { text } => serde_json::json!({"type": "text", "text": text}),
                    ContentPart::ImageUrl { image_url } => serde_json::json!({
                        "type": "image_url",
                        "image_url": { "url": &image_url.url, "detail": image_url.detail.as_deref().unwrap_or("auto") }
                    }),
                    ContentPart::Image { source } => serde_json::json!({
                        "type": "image_url",
                        "image_url": { "url": format!("data:{};base64,{}", source.media_type, source.data) }
                    }),
                }).collect();
                serde_json::json!(arr)
            }
        };
        non_system.push(serde_json::json!({"role": m.role, "content": content_val}));
    }

    let mut result: Vec<serde_json::Value> = Vec::new();
    if !system_parts.is_empty() {
        result.push(serde_json::json!({"role": "system", "content": system_parts.join("\n\n")}));
    }

    // Validate: collect valid tool_call IDs from assistant messages,
    // then strip any tool-role messages whose tool_call_id is orphaned
    let mut valid_tc_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for msg in &non_system {
        if msg.get("role").and_then(|r| r.as_str()) == Some("assistant") {
            if let Some(tcs) = msg.get("tool_calls").and_then(|t| t.as_array()) {
                for tc in tcs {
                    if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
                        valid_tc_ids.insert(id.to_string());
                    }
                }
            }
        }
    }

    for msg in non_system {
        if msg.get("role").and_then(|r| r.as_str()) == Some("tool") {
            let tcid = msg.get("tool_call_id").and_then(|i| i.as_str()).unwrap_or("");
            if tcid.is_empty() || !valid_tc_ids.contains(tcid) {
                log::warn!("[build_messages] stripping orphaned tool message with tool_call_id={}", tcid);
                continue;
            }
        }
        result.push(msg);
    }

    result
}

// ── OpenAI-compatible streaming with tool_calls ────────────────────

async fn stream_openai_compat(
    app: &AppHandle,
    client: &Client,
    channel_type: u32,
    base_url: &str,
    api_key: &str,
    model_id: &str,
    messages: &[ChatMessage],
    message_id: &str,
    options: &Option<ChatOptions>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = channel_types::build_chat_url(channel_type, base_url, model_id, api_key);
    let auth_headers = channel_types::build_auth_headers(channel_type, api_key);
    let api_messages = build_openai_messages(messages);

    let tools_enabled = options.as_ref().and_then(|o| o.tools_enabled).unwrap_or(false);
    let enabled_ids = options.as_ref().and_then(|o| o.enabled_tool_ids.clone()).unwrap_or_default();

    let mut body = serde_json::json!({
        "model": model_id,
        "messages": api_messages,
        "stream": true
    });

    if tools_enabled && !enabled_ids.is_empty() {
        let tool_defs = tools::build_chat_tool_definitions(&enabled_ids);
        if let Some(arr) = tool_defs.as_array() {
            if !arr.is_empty() {
                body["tools"] = tool_defs;
                body["tool_choice"] = serde_json::json!("auto");
            }
        }
    }

    if let Some(opts) = options {
        if let Some(t) = opts.temperature { body["temperature"] = serde_json::json!(t); }
        if let Some(m) = opts.max_tokens { body["max_tokens"] = serde_json::json!(m); }
        if let Some(p) = opts.top_p { body["top_p"] = serde_json::json!(p); }
    }

    let tool_count = body.get("tools").and_then(|t| t.as_array()).map(|a| a.len()).unwrap_or(0);
    log::info!("[stream] url={}, tools_enabled={}, tool_count={}, messages={}", url, tools_enabled, tool_count, api_messages.len());

    let has_tools_in_body = body.get("tools").is_some();
    let mut req = client.post(&url).header("Content-Type", "application/json");
    for (k, v) in &auth_headers {
        req = req.header(k, v);
    }
    let resp = req.json(&body).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_body = resp.text().await.unwrap_or_default();
        let err_lower = err_body.to_lowercase();
        // If error is about tools/functions and we sent tools, retry without them
        if has_tools_in_body && (err_lower.contains("tool") || err_lower.contains("function")
            || err_lower.contains("unsupported") || status.as_u16() == 400)
        {
            log::warn!("[stream] tools rejected ({}), retrying without tools: {}", status, &err_body[..err_body.len().min(200)]);
            body.as_object_mut().map(|o| { o.remove("tools"); o.remove("tool_choice"); });
            let mut req2 = client.post(&url).header("Content-Type", "application/json");
            for (k, v) in &auth_headers {
                req2 = req2.header(k, v);
            }
            let resp2 = req2.json(&body).send().await?;
            if !resp2.status().is_success() {
                let s2 = resp2.status();
                let e2 = resp2.text().await.unwrap_or_default();
                return Err(format!("API error {}: {}", s2, &e2[..e2.len().min(500)]).into());
            }
            // Continue with resp2 below — jump into the streaming path
            return stream_openai_response(app, resp2, message_id).await;
        }
        return Err(format!("API error {}: {}", status, &err_body[..err_body.len().min(500)]).into());
    }

    stream_openai_response(app, resp, message_id).await
}

async fn stream_openai_response(
    app: &AppHandle,
    resp: reqwest::Response,
    message_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();
    let mut first_chunk_logged = false;
    let mut content_sent = false;
    let mut total_content = String::new();
    let mut last_delta_content = String::new();
    let mut total_reasoning = String::new();
    let mut last_delta_reasoning = String::new();

    let mut tc_map: std::collections::HashMap<u32, (String, String, String)> = std::collections::HashMap::new();

    while let Some(chunk_result) = stream.next().await {
        if is_aborted(message_id) {
            log::info!("[stream] aborted for msg_id={}", message_id);
            clear_aborted(message_id);
            app.emit("chat:stream-end", StreamEnd {
                message_id: message_id.to_string(),
                has_tool_calls: false,
            })?;
            return Ok(());
        }
        let bytes = chunk_result?;
        let raw = String::from_utf8_lossy(&bytes);

        if !first_chunk_logged {
            first_chunk_logged = true;
            if !raw.starts_with("data:") && !raw.trim().is_empty() {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(raw.trim()) {
                    let sc = val.get("base_resp").and_then(|b| b.get("status_code")).and_then(|c| c.as_i64());
                    if sc.is_some() && sc != Some(0) {
                        let sm = val.get("base_resp").and_then(|b| b.get("status_msg")).and_then(|s| s.as_str()).unwrap_or("");
                        return Err(format!("API error (code {}): {}", sc.unwrap_or(-1), sm).into());
                    }
                    if let Some(em) = val.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
                        return Err(format!("API error: {}", em).into());
                    }
                }
            }
        }

        buffer.push_str(&raw);

        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].to_string();
            buffer = buffer[pos + 1..].to_string();
            let line = line.trim();
            if line.is_empty() { continue; }

            if let Some(data) = line.strip_prefix("data: ") {
                if data.trim() == "[DONE]" {
                    emit_accumulated_tool_calls(app, &mut tc_map, message_id)?;
                    app.emit("chat:stream-end", StreamEnd {
                        message_id: message_id.to_string(),
                        has_tool_calls: !tc_map.is_empty(),
                    })?;
                    return Ok(());
                }

                if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
                    let choice = val.get("choices").and_then(|c| c.get(0));
                    if let Some(ch) = choice {
                        // Prefer delta (streaming); only fallback to message if no delta
                        let (d_obj, is_delta) = if let Some(d) = ch.get("delta") {
                            (Some(d), true)
                        } else if let Some(m) = ch.get("message") {
                            (Some(m), false)
                        } else {
                            (None, false)
                        };
                        if let Some(d) = d_obj {
                            let content = d.get("content").and_then(|t| t.as_str()).unwrap_or("");
                            if !content.is_empty() {
                                if is_delta {
                                    // Detect cumulative-delta pattern: if this delta's content
                                    // starts with what we've already accumulated, it's cumulative
                                    // (some APIs send full text each time instead of incremental diffs)
                                    if !total_content.is_empty() && content.starts_with(&total_content) {
                                        let new_part = &content[total_content.len()..];
                                        if !new_part.is_empty() {
                                            app.emit("chat:stream-chunk", StreamChunk {
                                                message_id: message_id.to_string(),
                                                chunk: new_part.to_string(),
                                            })?;
                                        }
                                        total_content = content.to_string();
                                    } else if content == last_delta_content {
                                        // Exact duplicate of previous chunk, skip
                                    } else {
                                        // Normal incremental delta
                                        app.emit("chat:stream-chunk", StreamChunk {
                                            message_id: message_id.to_string(),
                                            chunk: content.to_string(),
                                        })?;
                                        total_content.push_str(content);
                                    }
                                    last_delta_content = content.to_string();
                                } else if !content_sent {
                                    // Non-streaming message: emit once only
                                    if total_content.is_empty() || content != total_content {
                                        content_sent = true;
                                        let emit_content = if !total_content.is_empty() && content.starts_with(&total_content) {
                                            content[total_content.len()..].to_string()
                                        } else if total_content.is_empty() {
                                            content.to_string()
                                        } else {
                                            String::new()
                                        };
                                        if !emit_content.is_empty() {
                                            app.emit("chat:stream-chunk", StreamChunk {
                                                message_id: message_id.to_string(),
                                                chunk: emit_content,
                                            })?;
                                        }
                                    }
                                }
                            }

                            let reasoning = d.get("reasoning_content").and_then(|t| t.as_str()).unwrap_or("");
                            if !reasoning.is_empty() {
                                if is_delta {
                                    if !total_reasoning.is_empty() && reasoning.starts_with(&total_reasoning) {
                                        let new_part = &reasoning[total_reasoning.len()..];
                                        if !new_part.is_empty() {
                                            app.emit("chat:stream-thinking", StreamThinking {
                                                message_id: message_id.to_string(),
                                                chunk: new_part.to_string(),
                                            })?;
                                        }
                                        total_reasoning = reasoning.to_string();
                                    } else if reasoning == last_delta_reasoning {
                                        // duplicate reasoning chunk, skip
                                    } else {
                                        app.emit("chat:stream-thinking", StreamThinking {
                                            message_id: message_id.to_string(),
                                            chunk: reasoning.to_string(),
                                        })?;
                                        total_reasoning.push_str(reasoning);
                                    }
                                    last_delta_reasoning = reasoning.to_string();
                                } else if total_reasoning.is_empty() {
                                    app.emit("chat:stream-thinking", StreamThinking {
                                        message_id: message_id.to_string(),
                                        chunk: reasoning.to_string(),
                                    })?;
                                    total_reasoning = reasoning.to_string();
                                }
                            }

                            if let Some(tcs) = d.get("tool_calls").and_then(|t| t.as_array()) {
                                for tc in tcs {
                                    let idx = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as u32;
                                    let entry = tc_map.entry(idx).or_insert_with(|| {
                                        (String::new(), String::new(), String::new())
                                    });
                                    if let Some(id_str) = tc.get("id").and_then(|i| i.as_str()) {
                                        if !id_str.is_empty() {
                                            entry.0 = id_str.to_string();
                                        }
                                    }
                                    if let Some(f) = tc.get("function") {
                                        if let Some(n) = f.get("name").and_then(|n| n.as_str()) {
                                            if !n.is_empty() { entry.1 = n.to_string(); }
                                        }
                                        if let Some(a) = f.get("arguments") {
                                            if is_delta {
                                                if let Some(s) = a.as_str() {
                                                    entry.2.push_str(s);
                                                } else if !a.is_null() {
                                                    entry.2.push_str(&a.to_string());
                                                }
                                            } else {
                                                if let Some(s) = a.as_str() {
                                                    entry.2 = s.to_string();
                                                } else if !a.is_null() {
                                                    entry.2 = a.to_string();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Stream ended without [DONE]
    emit_accumulated_tool_calls(app, &mut tc_map, message_id)?;
    app.emit("chat:stream-end", StreamEnd {
        message_id: message_id.to_string(),
        has_tool_calls: !tc_map.is_empty(),
    })?;
    Ok(())
}

fn fix_doubled_json(s: &mut String) {
    if s.contains("}{") && serde_json::from_str::<serde_json::Value>(s).is_err() {
        if let Some(pos) = s.find("}{") {
            let first_half = &s[..pos + 1];
            if serde_json::from_str::<serde_json::Value>(first_half).is_ok() {
                *s = first_half.to_string();
            }
        }
    }
}

fn emit_accumulated_tool_calls(
    app: &AppHandle,
    tc_map: &mut std::collections::HashMap<u32, (String, String, String)>,
    message_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for (idx, entry) in tc_map.iter_mut() {
        if entry.0.is_empty() {
            entry.0 = format!("tc_{}_{}", message_id, idx);
        }
        fix_doubled_json(&mut entry.2);
        log::info!("[tool_call] id={}, name={}, args={}", entry.0, entry.1, &entry.2[..entry.2.len().min(200)]);
        let _ = app.emit("chat:stream-tool-call", StreamToolCall {
            message_id: message_id.to_string(),
            tool_call_id: entry.0.clone(),
            name: entry.1.clone(),
            arguments: entry.2.clone(),
        });
    }
    Ok(())
}

// ── Anthropic streaming ────────────────────────────────────────────

async fn stream_anthropic(
    app: &AppHandle,
    client: &Client,
    channel_type: u32,
    base_url: &str,
    api_key: &str,
    model_id: &str,
    messages: &[ChatMessage],
    message_id: &str,
    options: &Option<ChatOptions>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = channel_types::build_chat_url(channel_type, base_url, model_id, api_key);
    let auth_headers = channel_types::build_auth_headers(channel_type, api_key);

    let mut system_text = String::new();
    let api_messages: Vec<serde_json::Value> = messages.iter().filter_map(|m| {
        if m.role == "system" {
            if !system_text.is_empty() { system_text.push('\n'); }
            system_text.push_str(&m.text_only());
            None
        } else {
            let content_val = match &m.content {
                MessageContent::Text(s) => serde_json::json!(s),
                MessageContent::Parts(parts) => {
                    let arr: Vec<serde_json::Value> = parts.iter().map(|p| match p {
                        ContentPart::Text { text } => serde_json::json!({"type": "text", "text": text}),
                        ContentPart::ImageUrl { image_url } => {
                            if let Some(rest) = image_url.url.strip_prefix("data:") {
                                if let Some(idx) = rest.find(";base64,") {
                                    let media = &rest[..idx];
                                    let data = &rest[idx + 8..];
                                    return serde_json::json!({"type": "image", "source": {"type": "base64", "media_type": media, "data": data}});
                                }
                            }
                            serde_json::json!({"type": "text", "text": format!("[image: {}]", image_url.url)})
                        }
                        ContentPart::Image { source } => serde_json::json!({"type": "image", "source": {"type": &source.source_type, "media_type": &source.media_type, "data": &source.data}}),
                    }).collect();
                    serde_json::json!(arr)
                }
            };
            Some(serde_json::json!({"role": m.role, "content": content_val}))
        }
    }).collect();

    let max_tokens = options.as_ref().and_then(|o| o.max_tokens).unwrap_or(4096);
    let mut body = serde_json::json!({
        "model": model_id,
        "messages": api_messages,
        "max_tokens": max_tokens,
        "stream": true
    });
    if !system_text.is_empty() { body["system"] = serde_json::json!(system_text); }
    if let Some(opts) = options {
        if let Some(t) = opts.temperature { body["temperature"] = serde_json::json!(t); }
        if let Some(p) = opts.top_p { body["top_p"] = serde_json::json!(p); }
    }

    let mut req = client.post(&url).header("Content-Type", "application/json");
    for (k, v) in &auth_headers { req = req.header(k, v); }
    let resp = req.json(&body).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let b = resp.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, b).into());
    }

    let mut stream_r = resp.bytes_stream();
    let mut buffer = String::new();
    while let Some(chunk_result) = stream_r.next().await {
        if is_aborted(message_id) {
            log::info!("[anthropic] aborted for msg_id={}", message_id);
            clear_aborted(message_id);
            app.emit("chat:stream-end", StreamEnd { message_id: message_id.to_string(), has_tool_calls: false })?;
            return Ok(());
        }
        let bytes = chunk_result?;
        buffer.push_str(&String::from_utf8_lossy(&bytes));
        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].to_string();
            buffer = buffer[pos + 1..].to_string();
            let line = line.trim();
            if line.is_empty() { continue; }
            if let Some(data) = line.strip_prefix("data: ") {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
                    let et = val.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    match et {
                        "content_block_delta" => {
                            if let Some(text) = val.get("delta").and_then(|d| d.get("text")).and_then(|t| t.as_str()) {
                                if !text.is_empty() {
                                    app.emit("chat:stream-chunk", StreamChunk { message_id: message_id.to_string(), chunk: text.to_string() })?;
                                }
                            }
                        }
                        "message_stop" => {
                            app.emit("chat:stream-end", StreamEnd { message_id: message_id.to_string(), has_tool_calls: false })?;
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    app.emit("chat:stream-end", StreamEnd { message_id: message_id.to_string(), has_tool_calls: false })?;
    Ok(())
}

// ── Gemini streaming ───────────────────────────────────────────────

async fn stream_gemini(
    app: &AppHandle,
    client: &Client,
    channel_type: u32,
    base_url: &str,
    api_key: &str,
    model_id: &str,
    messages: &[ChatMessage],
    message_id: &str,
    options: &Option<ChatOptions>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = channel_types::build_chat_url(channel_type, base_url, model_id, api_key);
    let mut system_text = String::new();
    let mut contents: Vec<serde_json::Value> = Vec::new();
    for m in messages {
        if m.role == "system" { if !system_text.is_empty() { system_text.push('\n'); } system_text.push_str(&m.text_only()); continue; }
        let role = if m.role == "assistant" { "model" } else { "user" };
        let parts = match &m.content {
            MessageContent::Text(s) => vec![serde_json::json!({"text": s})],
            MessageContent::Parts(ps) => ps.iter().map(|p| match p {
                ContentPart::Text { text } => serde_json::json!({"text": text}),
                ContentPart::ImageUrl { image_url } => {
                    if let Some(rest) = image_url.url.strip_prefix("data:") {
                        if let Some(idx) = rest.find(";base64,") {
                            return serde_json::json!({"inline_data": {"mime_type": &rest[..idx], "data": &rest[idx + 8..]}});
                        }
                    }
                    serde_json::json!({"text": format!("[image: {}]", image_url.url)})
                }
                ContentPart::Image { source } => serde_json::json!({"inline_data": {"mime_type": &source.media_type, "data": &source.data}}),
            }).collect(),
        };
        contents.push(serde_json::json!({"role": role, "parts": parts}));
    }
    let mut body = serde_json::json!({"contents": contents});
    if !system_text.is_empty() { body["system_instruction"] = serde_json::json!({"parts": [{"text": system_text}]}); }
    let mut gc = serde_json::Map::new();
    if let Some(opts) = options {
        if let Some(t) = opts.temperature { gc.insert("temperature".into(), serde_json::json!(t)); }
        if let Some(m) = opts.max_tokens { gc.insert("maxOutputTokens".into(), serde_json::json!(m)); }
        if let Some(p) = opts.top_p { gc.insert("topP".into(), serde_json::json!(p)); }
    }
    if !gc.is_empty() { body["generationConfig"] = serde_json::Value::Object(gc); }
    let resp = client.post(&url).header("Content-Type", "application/json").json(&body).send().await?;
    if !resp.status().is_success() { let s = resp.status(); let b = resp.text().await.unwrap_or_default(); return Err(format!("Gemini API error {}: {}", s, b).into()); }
    let mut stream_r = resp.bytes_stream();
    let mut buffer = String::new();
    while let Some(cr) = stream_r.next().await {
        if is_aborted(message_id) {
            log::info!("[gemini] aborted for msg_id={}", message_id);
            clear_aborted(message_id);
            app.emit("chat:stream-end", StreamEnd { message_id: message_id.to_string(), has_tool_calls: false })?;
            return Ok(());
        }
        buffer.push_str(&String::from_utf8_lossy(&cr?));
        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].to_string(); buffer = buffer[pos + 1..].to_string(); let line = line.trim(); if line.is_empty() { continue; }
            if let Some(data) = line.strip_prefix("data: ") {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(text) = val.get("candidates").and_then(|c| c.get(0)).and_then(|c| c.get("content")).and_then(|c| c.get("parts")).and_then(|p| p.get(0)).and_then(|p| p.get("text")).and_then(|t| t.as_str()) {
                        if !text.is_empty() { app.emit("chat:stream-chunk", StreamChunk { message_id: message_id.to_string(), chunk: text.to_string() })?; }
                    }
                }
            }
        }
    }
    app.emit("chat:stream-end", StreamEnd { message_id: message_id.to_string(), has_tool_calls: false })?;
    Ok(())
}

// ── Fallback ───────────────────────────────────────────────────────

async fn do_fallback_stream(
    app: &AppHandle,
    messages: &[ChatMessage],
    message_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response_text = format!(
        "\u{26a0}\u{fe0f} 尚未配置模型提供商。\n\n请前往 **设置 \u{2192} 模型服务** 添加 API Key 和 Endpoint。\n\n收到 {} 条消息。",
        messages.len()
    );
    for chunk in response_text.chars().collect::<Vec<_>>().chunks(5) {
        let text: String = chunk.iter().collect();
        app.emit("chat:stream-chunk", StreamChunk { message_id: message_id.to_string(), chunk: text })?;
        tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;
    }
    app.emit("chat:stream-end", StreamEnd { message_id: message_id.to_string(), has_tool_calls: false })?;
    Ok(())
}
