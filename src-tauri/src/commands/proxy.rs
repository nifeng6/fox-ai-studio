use std::net::SocketAddr;
use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use tokio::sync::{oneshot, RwLock};

use crate::commands::provider::ProviderState;
use crate::proxy::server::ProxyState;
use crate::proxy::types::{UpstreamModel, UpstreamProvider};

/// Holds the running proxy server handle so we can stop it later
pub struct ApiServerHandle {
    pub shutdown_tx: Option<oneshot::Sender<()>>,
    pub port: u16,
    pub running: bool,
}

pub struct ApiServerState(pub std::sync::Mutex<ApiServerHandle>);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiServerStatus {
    pub running: bool,
    pub port: u16,
    pub url: String,
}

#[tauri::command]
pub async fn start_api_server(
    port: u16,
    api_key: String,
    default_provider_id: String,
    server_state: State<'_, ApiServerState>,
    provider_state: State<'_, ProviderState>,
) -> Result<ApiServerStatus, String> {
    {
        let handle = server_state.0.lock().map_err(|e| e.to_string())?;
        if handle.running {
            return Err("API server is already running".to_string());
        }
    }

    let providers_data = {
        let providers = provider_state.0.lock().map_err(|e| e.to_string())?;
        providers
            .iter()
            .filter(|p| p.enabled && !p.api_key.is_empty())
            .map(|p| UpstreamProvider {
                id: p.id.clone(),
                name: p.name.clone(),
                channel_type: p.channel_type,
                api_key: p.api_key.clone(),
                api_endpoint: p.api_endpoint.clone(),
                models: p.models.iter().map(|mid| UpstreamModel {
                    id: mid.clone(),
                    name: mid.clone(),
                }).collect(),
            })
            .collect::<Vec<_>>()
    };

    if providers_data.is_empty() {
        return Err("No enabled providers with API keys configured".to_string());
    }

    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let proxy_state = ProxyState {
        providers: Arc::new(RwLock::new(providers_data)),
        default_provider_id: Arc::new(RwLock::new(default_provider_id)),
        api_key: Arc::new(RwLock::new(api_key)),
        http_client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| e.to_string())?,
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    tokio::spawn(async move {
        if let Err(e) = crate::proxy::server::start_server(addr, proxy_state, shutdown_rx).await {
            log::error!("API proxy server error: {}", e);
        }
    });

    {
        let mut handle = server_state.0.lock().map_err(|e| e.to_string())?;
        handle.shutdown_tx = Some(shutdown_tx);
        handle.port = port;
        handle.running = true;
    }

    Ok(ApiServerStatus {
        running: true,
        port,
        url: format!("http://127.0.0.1:{}", port),
    })
}

#[tauri::command]
pub fn stop_api_server(
    server_state: State<'_, ApiServerState>,
) -> Result<ApiServerStatus, String> {
    let mut handle = server_state.0.lock().map_err(|e| e.to_string())?;
    if !handle.running {
        return Ok(ApiServerStatus {
            running: false,
            port: handle.port,
            url: String::new(),
        });
    }

    if let Some(tx) = handle.shutdown_tx.take() {
        let _ = tx.send(());
    }
    handle.running = false;

    log::info!("API proxy server stopped");

    Ok(ApiServerStatus {
        running: false,
        port: handle.port,
        url: String::new(),
    })
}

#[tauri::command]
pub fn get_api_server_status(
    server_state: State<'_, ApiServerState>,
) -> Result<ApiServerStatus, String> {
    let handle = server_state.0.lock().map_err(|e| e.to_string())?;
    Ok(ApiServerStatus {
        running: handle.running,
        port: handle.port,
        url: if handle.running {
            format!("http://127.0.0.1:{}", handle.port)
        } else {
            String::new()
        },
    })
}

#[tauri::command]
pub async fn update_api_server_providers(
    _server_state: State<'_, ApiServerState>,
    _provider_state: State<'_, ProviderState>,
) -> Result<(), String> {
    Ok(())
}
