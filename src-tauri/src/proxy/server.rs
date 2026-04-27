use std::sync::Arc;
use std::net::SocketAddr;

use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Router,
};
use futures_util::stream::Stream;
use reqwest::Client;
use serde_json::json;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;

use super::converter;
use super::types::*;

// ─── Shared State ────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ProxyState {
    pub providers: Arc<RwLock<Vec<UpstreamProvider>>>,
    pub default_provider_id: Arc<RwLock<String>>,
    pub api_key: Arc<RwLock<String>>,
    pub http_client: Client,
}

// ─── Server lifecycle ────────────────────────────────────────────────

pub async fn start_server(
    addr: SocketAddr,
    state: ProxyState,
    shutdown_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<(), String> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(openai_chat_completions))
        .route("/chat/completions", post(openai_chat_completions))
        .route("/v1/messages", post(anthropic_messages))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind {}: {}", addr, e))?;

    log::info!("API proxy server listening on {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
            log::info!("API proxy server shutting down");
        })
        .await
        .map_err(|e| format!("Server error: {}", e))
}

// ─── Auth helper ─────────────────────────────────────────────────────

fn check_auth(headers: &HeaderMap, expected_key: &str) -> Result<(), (StatusCode, String)> {
    if expected_key.is_empty() {
        return Ok(());
    }
    let auth = headers
        .get("authorization")
        .or_else(|| headers.get("x-api-key"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let token = auth.strip_prefix("Bearer ").unwrap_or(auth);
    if token == expected_key {
        Ok(())
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            serde_json::to_string(&ErrorResponse {
                error: ApiError {
                    message: "Invalid API key".to_string(),
                    error_type: "authentication_error".to_string(),
                    code: Some("invalid_api_key".to_string()),
                },
            })
            .unwrap(),
        ))
    }
}

fn resolve_provider<'a>(
    providers: &'a [UpstreamProvider],
    default_id: &str,
    model_id: &str,
) -> Option<&'a UpstreamProvider> {
    // First try to find a provider that has this exact model
    if let Some(p) = providers.iter().find(|p| p.models.iter().any(|m| m.id == model_id)) {
        return Some(p);
    }
    // Fall back to the default provider
    if let Some(p) = providers.iter().find(|p| p.id == default_id) {
        return Some(p);
    }
    // Fall back to the first available provider
    providers.first()
}

// ─── Handlers ────────────────────────────────────────────────────────

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok", "service": "fox-ai-proxy" }))
}

async fn list_models(
    headers: HeaderMap,
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let key = state.api_key.read().await;
    check_auth(&headers, &key)?;
    drop(key);

    let providers = state.providers.read().await;
    let models: Vec<ModelInfo> = providers
        .iter()
        .flat_map(|p| {
            p.models.iter().map(move |m| ModelInfo {
                id: m.id.clone(),
                object: "model".to_string(),
                created: chrono::Utc::now().timestamp(),
                owned_by: p.name.clone(),
            })
        })
        .collect();

    Ok(Json(ModelListResponse {
        object: "list".to_string(),
        data: models,
    }))
}

/// POST /v1/chat/completions — OpenAI-compatible endpoint
async fn openai_chat_completions(
    headers: HeaderMap,
    State(state): State<ProxyState>,
    Json(req): Json<OpenAIChatRequest>,
) -> Result<Response, (StatusCode, String)> {
    let key = state.api_key.read().await;
    check_auth(&headers, &key)?;
    drop(key);

    let providers = state.providers.read().await;
    let default_id = state.default_provider_id.read().await;
    let provider = resolve_provider(&providers, &default_id, &req.model)
        .ok_or((
            StatusCode::BAD_REQUEST,
            json_error("No provider available for this model", "invalid_request"),
        ))?
        .clone();
    drop(default_id);
    drop(providers);

    let upstream_format = ApiFormat::detect_by_channel(provider.channel_type);
    let is_stream = req.stream.unwrap_or(false);

    match upstream_format {
        ApiFormat::OpenAI => {
            forward_openai_to_openai(&state.http_client, &provider, &req, is_stream).await
        }
        ApiFormat::Anthropic => {
            forward_openai_to_anthropic(&state.http_client, &provider, &req, is_stream).await
        }
    }
}

/// POST /v1/messages — Anthropic-compatible endpoint
async fn anthropic_messages(
    headers: HeaderMap,
    State(state): State<ProxyState>,
    Json(req): Json<AnthropicRequest>,
) -> Result<Response, (StatusCode, String)> {
    let key = state.api_key.read().await;
    check_auth(&headers, &key)?;
    drop(key);

    let providers = state.providers.read().await;
    let default_id = state.default_provider_id.read().await;
    let provider = resolve_provider(&providers, &default_id, &req.model)
        .ok_or((
            StatusCode::BAD_REQUEST,
            json_error("No provider available for this model", "invalid_request"),
        ))?
        .clone();
    drop(default_id);
    drop(providers);

    let upstream_format = ApiFormat::detect_by_channel(provider.channel_type);
    let is_stream = req.stream.unwrap_or(false);

    match upstream_format {
        ApiFormat::Anthropic => {
            forward_anthropic_to_anthropic(&state.http_client, &provider, &req, is_stream).await
        }
        ApiFormat::OpenAI => {
            forward_anthropic_to_openai(&state.http_client, &provider, &req, is_stream).await
        }
    }
}

// ─── Forward: OpenAI client → OpenAI upstream (passthrough) ──────────

async fn forward_openai_to_openai(
    client: &Client,
    provider: &UpstreamProvider,
    req: &OpenAIChatRequest,
    is_stream: bool,
) -> Result<Response, (StatusCode, String)> {
    let url = format!(
        "{}/v1/chat/completions",
        provider.api_endpoint.trim_end_matches('/')
    );

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", provider.api_key))
        .header("Content-Type", "application/json")
        .json(req)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, json_error(&e.to_string(), "upstream_error")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err((
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
            body,
        ));
    }

    if is_stream {
        let stream = resp.bytes_stream().map(|chunk| {
            chunk
                .map(|b| Event::default().data(String::from_utf8_lossy(&b).to_string()))
                .map_err(|e| {
                    log::error!("Stream error: {}", e);
                    std::io::Error::new(std::io::ErrorKind::Other, e)
                })
        });
        Ok(Sse::new(stream).keep_alive(KeepAlive::default()).into_response())
    } else {
        let body = resp.text().await.unwrap_or_default();
        Ok((StatusCode::OK, [("content-type", "application/json")], body).into_response())
    }
}

// ─── Forward: OpenAI client → Anthropic upstream (convert) ───────────

async fn forward_openai_to_anthropic(
    client: &Client,
    provider: &UpstreamProvider,
    req: &OpenAIChatRequest,
    is_stream: bool,
) -> Result<Response, (StatusCode, String)> {
    let anthropic_req = converter::openai_request_to_anthropic(req);
    let url = format!(
        "{}/v1/messages",
        provider.api_endpoint.trim_end_matches('/')
    );

    let resp = client
        .post(&url)
        .header("x-api-key", &provider.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&anthropic_req)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, json_error(&e.to_string(), "upstream_error")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err((
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
            body,
        ));
    }

    if is_stream {
        let model = req.model.clone();
        let chunk_id = format!("chatcmpl-{}", uuid::Uuid::new_v4());
        let byte_stream = resp.bytes_stream();

        let converted = stream_anthropic_to_openai_sse(byte_stream, chunk_id, model);
        Ok(Sse::new(converted).keep_alive(KeepAlive::default()).into_response())
    } else {
        let body = resp.text().await.unwrap_or_default();
        let anthropic_resp: AnthropicResponse = serde_json::from_str(&body)
            .map_err(|e| (StatusCode::BAD_GATEWAY, json_error(&format!("Parse error: {}", e), "upstream_error")))?;
        let openai_resp = converter::anthropic_response_to_openai(&anthropic_resp, &req.model);
        let json = serde_json::to_string(&openai_resp).unwrap_or_default();
        Ok((StatusCode::OK, [("content-type", "application/json")], json).into_response())
    }
}

// ─── Forward: Anthropic client → Anthropic upstream (passthrough) ────

async fn forward_anthropic_to_anthropic(
    client: &Client,
    provider: &UpstreamProvider,
    req: &AnthropicRequest,
    is_stream: bool,
) -> Result<Response, (StatusCode, String)> {
    let url = format!(
        "{}/v1/messages",
        provider.api_endpoint.trim_end_matches('/')
    );

    let resp = client
        .post(&url)
        .header("x-api-key", &provider.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(req)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, json_error(&e.to_string(), "upstream_error")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err((
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
            body,
        ));
    }

    if is_stream {
        let stream = resp.bytes_stream().map(|chunk| {
            chunk
                .map(|b| Event::default().data(String::from_utf8_lossy(&b).to_string()))
                .map_err(|e| {
                    log::error!("Stream error: {}", e);
                    std::io::Error::new(std::io::ErrorKind::Other, e)
                })
        });
        Ok(Sse::new(stream).keep_alive(KeepAlive::default()).into_response())
    } else {
        let body = resp.text().await.unwrap_or_default();
        Ok((StatusCode::OK, [("content-type", "application/json")], body).into_response())
    }
}

// ─── Forward: Anthropic client → OpenAI upstream (convert) ───────────

async fn forward_anthropic_to_openai(
    client: &Client,
    provider: &UpstreamProvider,
    req: &AnthropicRequest,
    is_stream: bool,
) -> Result<Response, (StatusCode, String)> {
    let openai_req = converter::anthropic_request_to_openai(req);
    let url = format!(
        "{}/v1/chat/completions",
        provider.api_endpoint.trim_end_matches('/')
    );

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", provider.api_key))
        .header("Content-Type", "application/json")
        .json(&openai_req)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, json_error(&e.to_string(), "upstream_error")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err((
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
            body,
        ));
    }

    if is_stream {
        let model = req.model.clone();
        let byte_stream = resp.bytes_stream();
        let converted = stream_openai_to_anthropic_sse(byte_stream, model);
        Ok(Sse::new(converted).keep_alive(KeepAlive::default()).into_response())
    } else {
        let body = resp.text().await.unwrap_or_default();
        let openai_resp: OpenAIChatResponse = serde_json::from_str(&body)
            .map_err(|e| (StatusCode::BAD_GATEWAY, json_error(&format!("Parse error: {}", e), "upstream_error")))?;
        let anthropic_resp = converter::openai_response_to_anthropic(&openai_resp, &req.model);
        let json = serde_json::to_string(&anthropic_resp).unwrap_or_default();
        Ok((StatusCode::OK, [("content-type", "application/json")], json).into_response())
    }
}

// ─── SSE stream converters ───────────────────────────────────────────

/// Parse Anthropic SSE stream → emit OpenAI SSE chunks
fn stream_anthropic_to_openai_sse(
    byte_stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
    chunk_id: String,
    model: String,
) -> impl Stream<Item = Result<Event, std::io::Error>> + Send {
    futures_util::stream::unfold(
        (Box::pin(byte_stream), String::new(), chunk_id, model, false),
        |(mut stream, mut buf, cid, model, done)| async move {
            if done {
                return None;
            }
            loop {
                match stream.next().await {
                    Some(Ok(bytes)) => {
                        buf.push_str(&String::from_utf8_lossy(&bytes));
                        while let Some(pos) = buf.find("\n\n") {
                            let event_block = buf[..pos].to_string();
                            buf = buf[pos + 2..].to_string();

                            if let Some(data_line) = event_block
                                .lines()
                                .find(|l| l.starts_with("data: "))
                                .map(|l| &l[6..])
                            {
                                if let Ok(val) = serde_json::from_str::<serde_json::Value>(data_line) {
                                    if val.get("type").and_then(|t| t.as_str()) == Some("content_block_delta") {
                                        if let Some(text) = val
                                            .get("delta")
                                            .and_then(|d| d.get("text"))
                                            .and_then(|t| t.as_str())
                                        {
                                            let sse = converter::anthropic_delta_to_openai_sse(
                                                &cid, &model, text, None,
                                            );
                                            return Some((
                                                Ok(Event::default().data(sse)),
                                                (stream, buf, cid, model, false),
                                            ));
                                        }
                                    } else if val.get("type").and_then(|t| t.as_str()) == Some("message_stop") {
                                        let sse = converter::anthropic_delta_to_openai_sse(
                                            &cid, &model, "", Some("end_turn"),
                                        );
                                        return Some((
                                            Ok(Event::default().data(format!("{sse}data: [DONE]\n\n"))),
                                            (stream, buf, cid, model, true),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    Some(Err(e)) => {
                        return Some((
                            Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
                            (stream, buf, cid, model, true),
                        ));
                    }
                    None => {
                        let sse = format!("data: [DONE]\n\n");
                        return Some((
                            Ok(Event::default().data(sse)),
                            (stream, buf, cid, model, true),
                        ));
                    }
                }
            }
        },
    )
}

/// Parse OpenAI SSE stream → emit Anthropic SSE events
fn stream_openai_to_anthropic_sse(
    byte_stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
    model: String,
) -> impl Stream<Item = Result<Event, std::io::Error>> + Send {
    let index: u32 = 0;
    futures_util::stream::unfold(
        (Box::pin(byte_stream), String::new(), model, index, false),
        |(mut stream, mut buf, model, idx, done)| async move {
            if done {
                return None;
            }
            loop {
                match stream.next().await {
                    Some(Ok(bytes)) => {
                        buf.push_str(&String::from_utf8_lossy(&bytes));
                        while let Some(pos) = buf.find("\n\n") {
                            let line_block = buf[..pos].to_string();
                            buf = buf[pos + 2..].to_string();

                            for line in line_block.lines() {
                                if let Some(data) = line.strip_prefix("data: ") {
                                    if data.trim() == "[DONE]" {
                                        let stop_event = format!(
                                            "event: message_stop\ndata: {{\"type\":\"message_stop\"}}\n\n"
                                        );
                                        return Some((
                                            Ok(Event::default().data(stop_event)),
                                            (stream, buf, model, idx, true),
                                        ));
                                    }
                                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
                                        if let Some(text) = val
                                            .get("choices")
                                            .and_then(|c| c.get(0))
                                            .and_then(|c| c.get("delta"))
                                            .and_then(|d| d.get("content"))
                                            .and_then(|c| c.as_str())
                                        {
                                            if !text.is_empty() {
                                                let sse = converter::openai_delta_to_anthropic_sse(text, idx);
                                                return Some((
                                                    Ok(Event::default().data(sse)),
                                                    (stream, buf, model, idx, false),
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Some(Err(e)) => {
                        return Some((
                            Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
                            (stream, buf, model, idx, true),
                        ));
                    }
                    None => {
                        return Some((
                            Ok(Event::default().data("event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n".to_string())),
                            (stream, buf, model, idx, true),
                        ));
                    }
                }
            }
        },
    )
}

fn json_error(message: &str, error_type: &str) -> String {
    serde_json::to_string(&ErrorResponse {
        error: ApiError {
            message: message.to_string(),
            error_type: error_type.to_string(),
            code: None,
        },
    })
    .unwrap_or_default()
}
