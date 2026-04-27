use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

use super::channel_types;

fn pick_one_key(raw: &str) -> String {
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelSettings {
    #[serde(default)]
    pub supports_vision: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub channel_type: u32,
    pub api_key: String,
    pub api_endpoint: String,
    pub models: Vec<String>,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub supports_vision: bool,
    #[serde(default)]
    pub model_settings: std::collections::HashMap<String, ModelSettings>,
}

pub struct ProviderState(pub Mutex<Vec<Provider>>);

fn providers_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("providers.json"))
}

fn save_to_disk(app: &AppHandle, providers: &[Provider]) -> Result<(), String> {
    let path = providers_path(app)?;
    let json = serde_json::to_string_pretty(providers).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_from_disk(app: &AppHandle) -> Vec<Provider> {
    let path = match providers_path(app) {
        Ok(p) => p,
        Err(_) => return vec![],
    };
    if !path.exists() {
        return vec![];
    }
    let providers: Vec<Provider> = match std::fs::read_to_string(&path) {
        Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
        Err(_) => return vec![],
    };

    providers
}


#[tauri::command]
pub fn get_providers(state: State<'_, ProviderState>) -> Result<Vec<Provider>, String> {
    let providers = state.0.lock().map_err(|e| e.to_string())?;
    Ok(providers.clone())
}

#[tauri::command]
pub fn add_provider(
    app: AppHandle,
    state: State<'_, ProviderState>,
    provider: Provider,
) -> Result<Provider, String> {
    let mut providers = state.0.lock().map_err(|e| e.to_string())?;

    let mut p = provider;
    if p.id.is_empty() {
        p.id = uuid::Uuid::new_v4().to_string();
    }
    if p.api_endpoint.is_empty() {
        let default_url = channel_types::get_default_base_url(p.channel_type);
        if !default_url.is_empty() {
            p.api_endpoint = default_url.to_string();
        }
    }
    if p.models.is_empty() {
        p.models = channel_types::get_default_models(p.channel_type);
    }
    let now = chrono::Utc::now().timestamp_millis();
    if p.created_at == 0 { p.created_at = now; }
    if p.updated_at == 0 { p.updated_at = now; }

    providers.push(p.clone());
    save_to_disk(&app, &providers)?;
    Ok(p)
}

#[tauri::command]
pub fn update_provider(
    app: AppHandle,
    state: State<'_, ProviderState>,
    id: String,
    data: serde_json::Value,
) -> Result<(), String> {
    let mut providers = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(p) = providers.iter_mut().find(|p| p.id == id) {
        if let Some(v) = data.get("name").and_then(|v| v.as_str()) { p.name = v.to_string(); }
        if let Some(v) = data.get("channelType").and_then(|v| v.as_u64()) { p.channel_type = v as u32; }
        if let Some(v) = data.get("apiKey").and_then(|v| v.as_str()) { p.api_key = v.to_string(); }
        if let Some(v) = data.get("apiEndpoint").and_then(|v| v.as_str()) { p.api_endpoint = v.to_string(); }
        if let Some(v) = data.get("enabled").and_then(|v| v.as_bool()) { p.enabled = v; }
        if let Some(v) = data.get("supportsVision").and_then(|v| v.as_bool()) { p.supports_vision = v; }
        if let Some(arr) = data.get("models").and_then(|v| v.as_array()) {
            p.models = arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
        }
        if let Some(ms) = data.get("modelSettings") {
            if let Ok(parsed) = serde_json::from_value::<std::collections::HashMap<String, ModelSettings>>(ms.clone()) {
                p.model_settings = parsed;
            }
        }
        p.updated_at = chrono::Utc::now().timestamp_millis();
        save_to_disk(&app, &providers)?;
    }
    Ok(())
}

#[tauri::command]
pub fn remove_provider(
    app: AppHandle,
    state: State<'_, ProviderState>,
    id: String,
) -> Result<(), String> {
    let mut providers = state.0.lock().map_err(|e| e.to_string())?;
    providers.retain(|p| p.id != id);
    save_to_disk(&app, &providers)?;
    Ok(())
}

#[tauri::command]
pub async fn test_connection(
    _app: AppHandle,
    id: String,
    state: State<'_, ProviderState>,
) -> Result<serde_json::Value, String> {
    let provider = {
        let providers = state.0.lock().map_err(|e| e.to_string())?;
        providers.iter().find(|p| p.id == id).cloned()
    };

    let prov = match provider {
        Some(p) => p,
        None => return Ok(serde_json::json!({ "success": false, "message": "Provider not found" })),
    };

    if prov.api_key.is_empty() || prov.api_endpoint.is_empty() {
        return Ok(serde_json::json!({ "success": false, "message": "请先配置 API Key 和 Endpoint" }));
    }

    let start = std::time::Instant::now();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let active_key = pick_one_key(&prov.api_key);
    let auth_headers = channel_types::build_auth_headers(prov.channel_type, &active_key);

    let defaults = channel_types::get_default_models(prov.channel_type);
    let test_model = prov.models.first()
        .or_else(|| defaults.first())
        .map(|s| s.to_string());

    // If no model available, skip chat test and go straight to models endpoint
    let test_model = match test_model {
        Some(m) => m,
        None => {
            log::info!("[test_connection] no model available, trying /models endpoint directly");
            let urls = channel_types::build_models_url(prov.channel_type, &prov.api_endpoint, &prov.api_key);
            for url in &urls {
                log::info!("[test_connection] trying models URL: {}", url);
                let mut req = client.get(url);
                for (k, v) in &auth_headers {
                    req = req.header(k, v);
                }
                match req.send().await {
                    Ok(resp) => {
                        let elapsed = start.elapsed().as_millis();
                        if resp.status().is_success() {
                            return Ok(serde_json::json!({ "success": true, "message": format!("连接成功 ✓ ({}ms)", elapsed), "elapsed_ms": elapsed }));
                        }
                        if resp.status().as_u16() == 401 || resp.status().as_u16() == 403 {
                            return Ok(serde_json::json!({ "success": false, "message": format!("API Key 无效 ({}ms)", elapsed), "elapsed_ms": elapsed }));
                        }
                    }
                    Err(e) => {
                        log::warn!("[test_connection] models request failed: {}", e);
                    }
                }
            }
            let elapsed = start.elapsed().as_millis();
            return Ok(serde_json::json!({ "success": false, "message": format!("连接失败 ({}ms): 无法连接到服务器", elapsed), "elapsed_ms": elapsed }));
        }
    };

    let chat_url = channel_types::build_chat_url(prov.channel_type, &prov.api_endpoint, &test_model, &active_key);
    log::info!("[test_connection] chat test: url={}, model={}", chat_url, test_model);

    let body = serde_json::json!({
        "model": test_model,
        "messages": [{"role": "user", "content": "Hi"}],
        "max_tokens": 5,
        "stream": false
    });

    let mut req = client.post(&chat_url).header("Content-Type", "application/json");
    for (k, v) in &auth_headers {
        req = req.header(k, v);
    }

    match req.json(&body).send().await {
        Ok(resp) => {
            let elapsed = start.elapsed().as_millis();
            let status = resp.status().as_u16();
            if status == 401 || status == 403 {
                return Ok(serde_json::json!({ "success": false, "message": format!("API Key 无效 (401/403) ({}ms)", elapsed), "elapsed_ms": elapsed }));
            }
            let resp_body = resp.text().await.unwrap_or_default();
            log::info!("[test_connection] chat response: status={}, elapsed={}ms, body={}",
                status, elapsed, &resp_body[..resp_body.len().min(300)]);

            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&resp_body) {
                if let Some(em) = val.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
                    return Ok(serde_json::json!({ "success": false, "message": format!("API error: {} ({}ms)", em, elapsed), "elapsed_ms": elapsed }));
                }
                let base_status = val.get("base_resp").and_then(|b| b.get("status_code")).and_then(|c| c.as_i64());
                if base_status.is_some() && base_status != Some(0) {
                    let sm = val.get("base_resp").and_then(|b| b.get("status_msg")).and_then(|s| s.as_str()).unwrap_or("unknown");
                    return Ok(serde_json::json!({ "success": false, "message": format!("API error: {} ({}ms)", sm, elapsed), "elapsed_ms": elapsed }));
                }
                if val.get("choices").is_some() || val.get("id").is_some() || base_status == Some(0) {
                    return Ok(serde_json::json!({ "success": true, "message": format!("连接成功 ✓ ({}ms)", elapsed), "elapsed_ms": elapsed }));
                }
            }

            if status >= 200 && status < 300 {
                return Ok(serde_json::json!({ "success": true, "message": format!("连接成功 ✓ ({}ms)", elapsed), "elapsed_ms": elapsed }));
            }

            let msg = if resp_body.len() > 200 { resp_body[..200].to_string() } else { resp_body };
            Ok(serde_json::json!({ "success": false, "message": format!("HTTP {} — {} ({}ms)", status, msg, elapsed), "elapsed_ms": elapsed }))
        }
        Err(e) => {
            let elapsed = start.elapsed().as_millis();
            log::warn!("[test_connection] chat request failed ({}ms): {}", elapsed, e);
            // Chat request failed — try /v1/models as fallback
            let urls = channel_types::build_models_url(prov.channel_type, &prov.api_endpoint, &prov.api_key);
            for url in &urls {
                log::info!("[test_connection] trying models fallback: {}", url);
                let mut req = client.get(url);
                for (k, v) in &auth_headers {
                    req = req.header(k, v);
                }
                if let Ok(resp) = req.send().await {
                    let elapsed2 = start.elapsed().as_millis();
                    if resp.status().is_success() {
                        return Ok(serde_json::json!({ "success": true, "message": format!("连接成功 (models) ✓ ({}ms)", elapsed2), "elapsed_ms": elapsed2 }));
                    }
                    if resp.status().as_u16() == 401 || resp.status().as_u16() == 403 {
                        return Ok(serde_json::json!({ "success": false, "message": format!("API Key 无效 ({}ms)", elapsed2), "elapsed_ms": elapsed2 }));
                    }
                }
            }
            let elapsed_final = start.elapsed().as_millis();
            Ok(serde_json::json!({ "success": false, "message": format!("连接失败 ({}ms): {}", elapsed_final, e), "elapsed_ms": elapsed_final }))
        }
    }
}

#[tauri::command]
pub async fn get_models(
    _app: AppHandle,
    id: String,
    state: State<'_, ProviderState>,
) -> Result<Vec<String>, String> {
    let provider = {
        let providers = state.0.lock().map_err(|e| e.to_string())?;
        providers.iter().find(|p| p.id == id).cloned()
    };

    let prov = match provider {
        Some(p) => p,
        None => return Ok(vec![]),
    };

    if prov.api_key.is_empty() || prov.api_endpoint.is_empty() {
        return Ok(channel_types::get_default_models(prov.channel_type));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let active_key = pick_one_key(&prov.api_key);
    let urls = channel_types::build_models_url(prov.channel_type, &prov.api_endpoint, &active_key);
    let auth_headers = channel_types::build_auth_headers(prov.channel_type, &active_key);

    log::info!("[get_models] channel_type={}, endpoint={}, urls={:?}", prov.channel_type, prov.api_endpoint, urls);

    for url in &urls {
        let mut req = client.get(url);
        for (k, v) in &auth_headers {
            req = req.header(k, v);
        }

        match req.send().await {
            Ok(r) => {
                let status = r.status();
                if !status.is_success() {
                    log::warn!("[get_models] {} returned {}", url, status);
                    continue;
                }
                if let Ok(body) = r.text().await {
                    log::info!("[get_models] response body ({}B): {}", body.len(), &body[..body.len().min(500)]);
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body) {
                        let models = parse_models_response(&val, prov.channel_type);
                        if !models.is_empty() {
                            return Ok(models);
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!("[get_models] request to {} failed: {}", url, e);
            }
        }
    }

    // Only return built-in defaults if the endpoint matches the known default for this channel type
    let default_base = channel_types::get_default_base_url(prov.channel_type);
    let is_default_endpoint = !default_base.is_empty()
        && prov.api_endpoint.trim_end_matches('/').starts_with(default_base.trim_end_matches('/'));

    if is_default_endpoint {
        let defaults = channel_types::get_default_models(prov.channel_type);
        if !defaults.is_empty() {
            return Ok(defaults);
        }
    }

    // Custom endpoint: return existing models (don't overwrite with wrong defaults)
    if !prov.models.is_empty() {
        return Ok(prov.models);
    }

    Ok(vec![])
}

fn parse_models_response(val: &serde_json::Value, channel_type: u32) -> Vec<String> {
    if channel_type == channel_types::CHANNEL_GEMINI {
        if let Some(arr) = val.get("models").and_then(|d| d.as_array()) {
            return arr.iter().filter_map(|item| {
                let name = item.get("name").and_then(|v| v.as_str())?;
                Some(name.strip_prefix("models/").unwrap_or(name).to_string())
            }).collect();
        }
    }

    if channel_type == channel_types::CHANNEL_OLLAMA {
        if let Some(arr) = val.get("models").and_then(|d| d.as_array()) {
            return arr.iter().filter_map(|item| {
                item.get("name").and_then(|v| v.as_str()).map(|s| s.to_string())
            }).collect();
        }
    }

    let arr = val.get("data").and_then(|d| d.as_array())
        .or_else(|| val.get("models").and_then(|d| d.as_array()))
        .or_else(|| val.as_array());

    match arr {
        Some(items) => items.iter().filter_map(|item| {
            let id = item.get("id").and_then(|v| v.as_str())
                .or_else(|| item.get("model").and_then(|v| v.as_str()))
                .or_else(|| item.as_str())?;
            Some(id.to_string())
        }).collect(),
        None => vec![],
    }
}
