use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelNotification {
    pub platform_id: String,
    pub webhook_url: String,
    pub secret: Option<String>,
    pub content: String,
}

fn build_feishu_body(content: &str) -> serde_json::Value {
    serde_json::json!({
        "msg_type": "text",
        "content": { "text": content }
    })
}

fn build_dingtalk_body(content: &str) -> serde_json::Value {
    serde_json::json!({
        "msgtype": "text",
        "text": { "content": content }
    })
}

fn build_discord_body(content: &str) -> serde_json::Value {
    serde_json::json!({ "content": content })
}

fn build_slack_body(content: &str) -> serde_json::Value {
    serde_json::json!({ "text": content })
}

fn build_telegram_body(content: &str, webhook_url: &str) -> (String, serde_json::Value) {
    let chat_id = webhook_url
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_string();
    let base = webhook_url
        .rfind('/')
        .map(|i| &webhook_url[..i])
        .unwrap_or(webhook_url);
    let url = format!("{}/sendMessage", base);
    (url, serde_json::json!({ "chat_id": chat_id, "text": content, "parse_mode": "Markdown" }))
}

fn sign_dingtalk(secret: &str, timestamp: i64) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let string_to_sign = format!("{}\n{}", timestamp, secret);
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(string_to_sign.as_bytes());
    let result = mac.finalize();
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(result.into_bytes())
}

#[tauri::command]
pub async fn send_channel_notification(notification: ChannelNotification) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let platform = notification.platform_id.as_str();
    let content = &notification.content;

    let (url, body) = match platform {
        "feishu" => (notification.webhook_url.clone(), build_feishu_body(content)),
        "dingtalk" => {
            let mut url = notification.webhook_url.clone();
            if let Some(secret) = &notification.secret {
                if !secret.is_empty() {
                    let ts = chrono::Utc::now().timestamp_millis();
                    let sign = sign_dingtalk(secret, ts);
                    let sep = if url.contains('?') { '&' } else { '?' };
                    url = format!("{}{}timestamp={}&sign={}", url, sep, ts, urlencoding::encode(&sign));
                }
            }
            (url, build_dingtalk_body(content))
        }
        "discord" => (notification.webhook_url.clone(), build_discord_body(content)),
        "slack" => (notification.webhook_url.clone(), build_slack_body(content)),
        "telegram" => {
            let (url, body) = build_telegram_body(content, &notification.webhook_url);
            (url, body)
        }
        "webhook" | _ => {
            (notification.webhook_url.clone(), serde_json::json!({ "text": content, "content": content }))
        }
    };

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    if status.is_success() {
        Ok(text)
    } else {
        Err(format!("HTTP {}: {}", status, text))
    }
}

#[tauri::command]
pub async fn test_channel_webhook(
    platform_id: String,
    webhook_url: String,
    secret: Option<String>,
) -> Result<String, String> {
    let test_content = "🦊 Fox AI 测试消息 - 通知渠道连接成功！";
    send_channel_notification(ChannelNotification {
        platform_id,
        webhook_url,
        secret,
        content: test_content.to_string(),
    })
    .await
}
