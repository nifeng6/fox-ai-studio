use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::oneshot;

use crate::commands::notification::send_channel_notification;
use crate::commands::notification::ChannelNotification;

// ── Data Structures ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformConfig {
    pub key: String,
    pub platform_id: String,
    pub name: String,
    pub webhook_url: String,
    pub secret: Option<String>,
    pub enabled: bool,
    pub auto_create_task: bool,
    pub allowed_users: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayStatus {
    pub running: bool,
    pub port: u16,
    pub connected_platforms: Vec<String>,
}

struct ParsedMessage {
    platform: String,
    user_id: String,
    user_name: String,
    content: String,
    reply_url: Option<String>,
}

// ── State ──

pub struct GatewayHandle {
    shutdown_tx: Option<oneshot::Sender<()>>,
    port: u16,
    running: bool,
}

pub struct GatewayState(pub Arc<Mutex<GatewayHandle>>);

pub struct PlatformConfigsState(pub Arc<Mutex<HashMap<String, PlatformConfig>>>);

// ── Persistence ──

fn get_platform_configs_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| format!("App dir: {}", e))?;
    let configs_dir = data_dir.join("platform_configs");
    if !configs_dir.exists() {
        std::fs::create_dir_all(&configs_dir).map_err(|e| format!("Create dir: {}", e))?;
    }
    Ok(configs_dir)
}

fn save_platform_config_to_disk(config: &PlatformConfig, configs_dir: &std::path::PathBuf) -> Result<(), String> {
    let path = configs_dir.join(format!("{}.json", config.key));
    let json = serde_json::to_string_pretty(config).map_err(|e| format!("Serialize: {}", e))?;
    std::fs::write(path, json).map_err(|e| format!("Write: {}", e))
}

fn load_platform_configs_from_disk(configs_dir: &std::path::PathBuf) -> Vec<PlatformConfig> {
    let mut configs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(configs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(config) = serde_json::from_str::<PlatformConfig>(&json) {
                        configs.push(config);
                    }
                }
            }
        }
    }
    configs
}

// ── Message Parsing ──

fn parse_feishu_message(body: &serde_json::Value) -> Option<ParsedMessage> {
    // Handle URL verification challenge
    if body.get("challenge").is_some() {
        return None;
    }
    let event = body.get("event")?;
    let msg_type = event.get("msg_type")?.as_str()?;
    if msg_type != "text" {
        return None;
    }
    let sender = event.get("sender")?.get("sender_id")?;
    let user_id = sender.get("user_id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    let content_str = event.get("message")?.get("content")?.as_str()?;
    let content_json: serde_json::Value = serde_json::from_str(content_str).ok()?;
    let text = content_json.get("text")?.as_str()?.to_string()?;

    Some(ParsedMessage {
        platform: "feishu".to_string(),
        user_id,
        user_name: String::new(),
        content: text,
        reply_url: None,
    })
}

fn parse_dingtalk_message(body: &serde_json::Value) -> Option<ParsedMessage> {
    let msg_type = body.get("msgtype")?.as_str()?;
    if msg_type != "text" {
        return None;
    }
    let content = body.get("text")?.get("content")?.as_str()?.to_string()?;
    let user_id = body.get("senderStaffId")
        .or_else(|| body.get("senderId"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    Some(ParsedMessage {
        platform: "dingtalk".to_string(),
        user_id,
        user_name: String::new(),
        content,
        reply_url: None,
    })
}

fn parse_wecom_message(body: &serde_json::Value) -> Option<ParsedMessage> {
    let content = body.get("Content")?.as_str()?.to_string()?;
    let user_id = body.get("FromUserName")?.as_str()?.to_string().unwrap_or_default();

    Some(ParsedMessage {
        platform: "wecom".to_string(),
        user_id,
        user_name: String::new(),
        content,
        reply_url: None,
    })
}

fn parse_generic_webhook(body: &serde_json::Value) -> Option<ParsedMessage> {
    let content = body.get("text")
        .or_else(|| body.get("content"))
        .or_else(|| body.get("message"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if content.is_empty() {
        return None;
    }

    Some(ParsedMessage {
        platform: "webhook".to_string(),
        user_id: "unknown".to_string(),
        user_name: String::new(),
        content,
        reply_url: None,
    })
}

fn parse_incoming_message(platform: &str, body: &serde_json::Value) -> Option<ParsedMessage> {
    match platform {
        "feishu" => parse_feishu_message(body),
        "dingtalk" => parse_dingtalk_message(body),
        "wecom" => parse_wecom_message(body),
        _ => parse_generic_webhook(body),
    }
}

// ── HTTP Server ──

async fn handle_incoming_message(
    platform: String,
    body: serde_json::Value,
    app: AppHandle,
    configs_state: PlatformConfigsState,
) -> axum::Json<serde_json::Value> {
    log::info!("[gateway] Received {} message", platform);

    // Handle Feishu URL verification challenge
    if platform == "feishu" {
        if let Some(challenge) = body.get("challenge").and_then(|v| v.as_str()) {
            let token = body.get("token").and_then(|v| v.as_str()).unwrap_or("");
            log::info!("[gateway] Feishu challenge received");
            return axum::Json(serde_json::json!({
                "challenge": challenge,
                "token": token
            }));
        }
    }

    let parsed = match parse_incoming_message(&platform, &body) {
        Some(p) => p,
        None => return axum::Json(serde_json::json!({"status": "ignored"})),
    };

    log::info!("[gateway] Parsed message from {} user={}: {}", parsed.platform, parsed.user_id, parsed.content);

    // Check if user is allowed
    let configs = configs_state.0.lock().unwrap();
    let config = configs.values().find(|c| c.platform_id == platform && c.enabled);
    if let Some(cfg) = config {
        if let Some(allowed) = &cfg.allowed_users {
            if !allowed.is_empty() && !allowed.contains(&parsed.user_id) {
                log::warn!("[gateway] User {} not in allowed list", parsed.user_id);
                return axum::Json(serde_json::json!({"status": "forbidden"}));
            }
        }

        // Auto-create task if configured
        if cfg.auto_create_task {
            let _ = app.emit("gateway:incoming_message", serde_json::json!({
                "platform": parsed.platform,
                "userId": parsed.user_id,
                "content": parsed.content,
            }));

            // Send confirmation back to platform
            let confirm_msg = format!("🦊 已收到指令，正在执行：{}", parsed.content);
            let notification = ChannelNotification {
                platform_id: cfg.platform_id.clone(),
                webhook_url: cfg.webhook_url.clone(),
                secret: cfg.secret.clone(),
                content: confirm_msg,
            };
            drop(configs);
            let _ = send_channel_notification(notification).await;
        }
    } else {
        log::warn!("[gateway] No enabled config for platform {}", platform);
    }

    axum::Json(serde_json::json!({"status": "ok"}))
}

async fn health_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "service": "fox-ai-gateway",
        "version": "0.1.0"
    }))
}

pub async fn start_gateway_server(
    port: u16,
    app: AppHandle,
    configs_state: PlatformConfigsState,
    mut shutdown_rx: oneshot::Receiver<()>,
) -> Result<(), String> {
    let app_feishu = app.clone();
    let configs_feishu = PlatformConfigsState(Arc::clone(&configs_state.0));
    let app_dingtalk = app.clone();
    let configs_dingtalk = PlatformConfigsState(Arc::clone(&configs_state.0));
    let app_wecom = app.clone();
    let configs_wecom = PlatformConfigsState(Arc::clone(&configs_state.0));
    let app_webhook = app.clone();
    let configs_webhook = PlatformConfigsState(Arc::clone(&configs_state.0));

    let app_cors = tower_http::cors::CorsLayer::permissive();

    let app = axum::Router::new()
        .route("/gateway/health", axum::routing::get(health_handler))
        .route("/gateway/feishu", axum::routing::post(move |body: axum::Json<serde_json::Value>| {
            let app = app_feishu.clone();
            let configs = PlatformConfigsState(Arc::clone(&configs_feishu.0));
            async move { handle_incoming_message("feishu".to_string(), body.0, app, configs).await }
        }))
        .route("/gateway/dingtalk", axum::routing::post(move |body: axum::Json<serde_json::Value>| {
            let app = app_dingtalk.clone();
            let configs = PlatformConfigsState(Arc::clone(&configs_dingtalk.0));
            async move { handle_incoming_message("dingtalk".to_string(), body.0, app, configs).await }
        }))
        .route("/gateway/wecom", axum::routing::post(move |body: axum::Json<serde_json::Value>| {
            let app = app_wecom.clone();
            let configs = PlatformConfigsState(Arc::clone(&configs_wecom.0));
            async move { handle_incoming_message("wecom".to_string(), body.0, app, configs).await }
        }))
        .route("/gateway/webhook", axum::routing::post(move |body: axum::Json<serde_json::Value>| {
            let app = app_webhook.clone();
            let configs = PlatformConfigsState(Arc::clone(&configs_webhook.0));
            async move { handle_incoming_message("webhook".to_string(), body.0, app, configs).await }
        }))
        .layer(app_cors);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| format!("Bind {}: {}", port, e))?;

    log::info!("[gateway] HTTP server listening on port {}", port);

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
            log::info!("[gateway] Server shutting down");
        })
        .await
        .map_err(|e| format!("Server error: {}", e))
}

// ── Tauri Commands ──

#[tauri::command]
pub async fn start_platform_gateway(
    app: AppHandle,
    port: Option<u16>,
    state: State<'_, GatewayState>,
    configs_state: State<'_, PlatformConfigsState>,
) -> Result<GatewayStatus, String> {
    let gateway_port = port.unwrap_or(23334);

    let mut handle = state.0.lock().map_err(|e| e.to_string())?;
    if handle.running {
        return Err("Gateway already running".to_string());
    }

    let (tx, rx) = oneshot::channel();
    handle.shutdown_tx = Some(tx);
    handle.port = gateway_port;
    handle.running = true;

    let app_clone = app.clone();
    let configs_clone = PlatformConfigsState(Arc::clone(&configs_state.0));
    let state_clone = GatewayState(Arc::clone(&state.0));

    tokio::spawn(async move {
        if let Err(e) = start_gateway_server(gateway_port, app_clone, configs_clone, rx).await {
            log::error!("[gateway] Server error: {}", e);
        }
        let mut h = state_clone.0.lock().unwrap();
        h.running = false;
    });

    // Load connected platforms
    let configs = configs_state.0.lock().unwrap();
    let connected: Vec<String> = configs.values()
        .filter(|c| c.enabled)
        .map(|c| c.platform_id.clone())
        .collect();

    log::info!("[gateway] Started on port {}", gateway_port);
    Ok(GatewayStatus {
        running: true,
        port: gateway_port,
        connected_platforms: connected,
    })
}

#[tauri::command]
pub async fn stop_platform_gateway(
    state: State<'_, GatewayState>,
) -> Result<GatewayStatus, String> {
    let mut handle = state.0.lock().map_err(|e| e.to_string())?;
    if !handle.running {
        return Err("Gateway not running".to_string());
    }

    if let Some(tx) = handle.shutdown_tx.take() {
        let _ = tx.send(());
    }
    handle.running = false;

    log::info!("[gateway] Stopped");
    Ok(GatewayStatus {
        running: false,
        port: handle.port,
        connected_platforms: Vec::new(),
    })
}

#[tauri::command]
pub fn configure_platform(
    app: AppHandle,
    config: PlatformConfig,
    state: State<'_, PlatformConfigsState>,
) -> Result<PlatformConfig, String> {
    let configs_dir = get_platform_configs_dir(&app)?;
    save_platform_config_to_disk(&config, &configs_dir)?;

    let mut configs = state.0.lock().map_err(|e| e.to_string())?;
    configs.insert(config.key.clone(), config.clone());

    log::info!("[gateway] Configured platform: {} ({})", config.name, config.platform_id);
    Ok(config)
}

#[tauri::command]
pub async fn test_platform(
    platform_id: String,
    webhook_url: String,
    secret: Option<String>,
) -> Result<String, String> {
    let test_content = "🦊 Fox AI 平台网关测试 - 连接成功！";
    let notification = ChannelNotification {
        platform_id,
        webhook_url,
        secret,
        content: test_content.to_string(),
    };
    send_channel_notification(notification).await
}
