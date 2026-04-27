use super::types::*;

// ─── OpenAI → Anthropic ──────────────────────────────────────────────

pub fn openai_request_to_anthropic(req: &OpenAIChatRequest) -> AnthropicRequest {
    let mut system_prompt: Option<String> = None;
    let mut messages: Vec<AnthropicMessage> = Vec::new();

    for msg in &req.messages {
        if msg.role == "system" {
            let text = extract_openai_text(&msg.content);
            system_prompt = Some(match system_prompt {
                Some(existing) => format!("{}\n{}", existing, text),
                None => text,
            });
            continue;
        }

        let role = match msg.role.as_str() {
            "assistant" => "assistant",
            _ => "user",
        };

        let content = convert_openai_content_to_anthropic(&msg.content);
        messages.push(AnthropicMessage {
            role: role.to_string(),
            content,
        });
    }

    AnthropicRequest {
        model: req.model.clone(),
        messages,
        max_tokens: req.max_tokens.unwrap_or(4096),
        temperature: req.temperature,
        top_p: req.top_p,
        stream: req.stream,
        system: system_prompt,
        stop_sequences: match &req.stop {
            Some(serde_json::Value::String(s)) => Some(vec![s.clone()]),
            Some(serde_json::Value::Array(arr)) => Some(
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect(),
            ),
            _ => None,
        },
    }
}

pub fn anthropic_response_to_openai(resp: &AnthropicResponse, model: &str) -> OpenAIChatResponse {
    let text = resp
        .content
        .iter()
        .filter_map(|block| match block {
            AnthropicResponseBlock::Text { text } => Some(text.as_str()),
        })
        .collect::<Vec<_>>()
        .join("");

    let finish_reason = match resp.stop_reason.as_deref() {
        Some("end_turn") => Some("stop".to_string()),
        Some("max_tokens") => Some("length".to_string()),
        Some("stop_sequence") => Some("stop".to_string()),
        other => other.map(String::from),
    };

    let usage = resp.usage.as_ref().map(|u| OpenAIUsage {
        prompt_tokens: u.input_tokens,
        completion_tokens: u.output_tokens,
        total_tokens: u.input_tokens + u.output_tokens,
    });

    OpenAIChatResponse {
        id: format!("chatcmpl-{}", &resp.id),
        object: "chat.completion".to_string(),
        created: chrono::Utc::now().timestamp(),
        model: model.to_string(),
        choices: vec![OpenAIChoice {
            index: 0,
            message: Some(OpenAIMessage {
                role: "assistant".to_string(),
                content: OpenAIContent::Text(text),
            }),
            delta: None,
            finish_reason,
        }],
        usage,
    }
}

// ─── Anthropic → OpenAI ──────────────────────────────────────────────

pub fn anthropic_request_to_openai(req: &AnthropicRequest) -> OpenAIChatRequest {
    let mut messages: Vec<OpenAIMessage> = Vec::new();

    if let Some(sys) = &req.system {
        messages.push(OpenAIMessage {
            role: "system".to_string(),
            content: OpenAIContent::Text(sys.clone()),
        });
    }

    for msg in &req.messages {
        let content = convert_anthropic_content_to_openai(&msg.content);
        messages.push(OpenAIMessage {
            role: msg.role.clone(),
            content,
        });
    }

    OpenAIChatRequest {
        model: req.model.clone(),
        messages,
        temperature: req.temperature,
        max_tokens: Some(req.max_tokens),
        top_p: req.top_p,
        stream: req.stream,
        stop: req
            .stop_sequences
            .as_ref()
            .map(|s| serde_json::Value::Array(s.iter().map(|v| serde_json::Value::String(v.clone())).collect())),
        presence_penalty: None,
        frequency_penalty: None,
    }
}

pub fn openai_response_to_anthropic(
    resp: &OpenAIChatResponse,
    model: &str,
) -> AnthropicResponse {
    let text = resp
        .choices
        .first()
        .and_then(|c| c.message.as_ref())
        .map(|m| extract_openai_text(&m.content))
        .unwrap_or_default();

    let stop_reason = resp
        .choices
        .first()
        .and_then(|c| c.finish_reason.as_deref())
        .map(|r| match r {
            "stop" => "end_turn",
            "length" => "max_tokens",
            other => other,
        })
        .map(String::from);

    let usage = resp.usage.as_ref().map(|u| AnthropicUsage {
        input_tokens: u.prompt_tokens,
        output_tokens: u.completion_tokens,
    });

    AnthropicResponse {
        id: resp.id.replace("chatcmpl-", "msg_"),
        response_type: "message".to_string(),
        role: "assistant".to_string(),
        model: model.to_string(),
        content: vec![AnthropicResponseBlock::Text { text }],
        stop_reason,
        usage,
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────

fn extract_openai_text(content: &OpenAIContent) -> String {
    match content {
        OpenAIContent::Text(s) => s.clone(),
        OpenAIContent::Parts(parts) => parts
            .iter()
            .filter_map(|p| match p {
                OpenAIContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(""),
    }
}

fn convert_openai_content_to_anthropic(content: &OpenAIContent) -> AnthropicContent {
    match content {
        OpenAIContent::Text(s) => AnthropicContent::Text(s.clone()),
        OpenAIContent::Parts(parts) => {
            let blocks: Vec<AnthropicContentBlock> = parts
                .iter()
                .map(|p| match p {
                    OpenAIContentPart::Text { text } => {
                        AnthropicContentBlock::Text { text: text.clone() }
                    }
                    OpenAIContentPart::ImageUrl { image_url } => {
                        AnthropicContentBlock::Image {
                            source: AnthropicImageSource {
                                source_type: "url".to_string(),
                                media_type: "image/png".to_string(),
                                data: image_url.url.clone(),
                            },
                        }
                    }
                })
                .collect();
            AnthropicContent::Blocks(blocks)
        }
    }
}

fn convert_anthropic_content_to_openai(content: &AnthropicContent) -> OpenAIContent {
    match content {
        AnthropicContent::Text(s) => OpenAIContent::Text(s.clone()),
        AnthropicContent::Blocks(blocks) => {
            let text = blocks
                .iter()
                .filter_map(|b| match b {
                    AnthropicContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("");
            OpenAIContent::Text(text)
        }
    }
}

// ─── SSE Stream conversion helpers ───────────────────────────────────

/// Convert an Anthropic SSE text delta into an OpenAI SSE chunk string
pub fn anthropic_delta_to_openai_sse(
    chunk_id: &str,
    model: &str,
    text: &str,
    finish_reason: Option<&str>,
) -> String {
    let chunk = OpenAIStreamChunk {
        id: chunk_id.to_string(),
        object: "chat.completion.chunk".to_string(),
        created: chrono::Utc::now().timestamp(),
        model: model.to_string(),
        choices: vec![OpenAIChoice {
            index: 0,
            message: None,
            delta: Some(OpenAIDelta {
                role: None,
                content: if text.is_empty() { None } else { Some(text.to_string()) },
            }),
            finish_reason: finish_reason.map(|r| match r {
                "end_turn" => "stop".to_string(),
                "max_tokens" => "length".to_string(),
                other => other.to_string(),
            }),
        }],
    };
    format!("data: {}\n\n", serde_json::to_string(&chunk).unwrap_or_default())
}

/// Convert an OpenAI SSE delta into an Anthropic SSE event string
pub fn openai_delta_to_anthropic_sse(text: &str, index: u32) -> String {
    let delta = AnthropicStreamContentDelta {
        event_type: "content_block_delta".to_string(),
        index,
        delta: AnthropicTextDelta {
            delta_type: "text_delta".to_string(),
            text: text.to_string(),
        },
    };
    format!(
        "event: content_block_delta\ndata: {}\n\n",
        serde_json::to_string(&delta).unwrap_or_default()
    )
}
