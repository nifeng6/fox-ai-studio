use std::collections::HashMap;

pub const CHANNEL_OPENAI: u32 = 1;
pub const CHANNEL_AZURE: u32 = 3;
pub const CHANNEL_OLLAMA: u32 = 4;
pub const CHANNEL_CUSTOM: u32 = 8;
pub const CHANNEL_ANTHROPIC: u32 = 14;
pub const CHANNEL_BAIDU: u32 = 15;
pub const CHANNEL_ZHIPU: u32 = 16;
pub const CHANNEL_ALI: u32 = 17;
pub const CHANNEL_OPENROUTER: u32 = 20;
pub const CHANNEL_TENCENT: u32 = 23;
pub const CHANNEL_GEMINI: u32 = 24;
pub const CHANNEL_MOONSHOT: u32 = 25;
pub const CHANNEL_ZHIPU_V4: u32 = 26;
pub const CHANNEL_PERPLEXITY: u32 = 27;
pub const CHANNEL_LINGYI: u32 = 31;
pub const CHANNEL_COHERE: u32 = 34;
pub const CHANNEL_MINIMAX: u32 = 35;
pub const CHANNEL_SILICONFLOW: u32 = 40;
pub const CHANNEL_MISTRAL: u32 = 42;
pub const CHANNEL_DEEPSEEK: u32 = 43;
pub const CHANNEL_VOLCENGINE: u32 = 45;
pub const CHANNEL_XAI: u32 = 48;
pub const CHANNEL_COZE: u32 = 49;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AuthScheme {
    Bearer,
    XApiKey,
    GoogleApiKey,
    AzureApiKey,
    QueryKeyParam,
}

pub struct ChannelDef {
    pub base_url: &'static str,
    pub chat_path: ChatPath,
    pub auth: AuthScheme,
    pub models_path: &'static str,
    pub models: &'static [&'static str],
}

#[derive(Clone, Copy)]
pub enum ChatPath {
    /// Standard: {base}/v1/chat/completions
    OpenAI,
    /// Anthropic: {base}/v1/messages
    Anthropic,
    /// Gemini native: {base}/v1beta/models/{model}:streamGenerateContent?alt=sse&key={key}
    Gemini,
    /// Azure: {base}/openai/deployments/{model}/chat/completions?api-version=2024-06-01
    Azure,
    /// MiniMax proprietary: {base}/v1/text/chatcompletion_v2
    MiniMax,
}

fn channel_registry() -> HashMap<u32, ChannelDef> {
    let mut m = HashMap::new();

    m.insert(CHANNEL_OPENAI, ChannelDef {
        base_url: "https://api.openai.com",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/v1/models",
        models: &[
            "gpt-4o", "gpt-4o-mini", "gpt-4o-2024-11-20", "gpt-4-turbo",
            "gpt-4", "gpt-3.5-turbo", "o1", "o1-mini", "o1-preview",
            "o3-mini", "chatgpt-4o-latest",
        ],
    });

    m.insert(CHANNEL_AZURE, ChannelDef {
        base_url: "",
        chat_path: ChatPath::Azure,
        auth: AuthScheme::AzureApiKey,
        models_path: "/openai/models?api-version=2024-06-01",
        models: &[
            "gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "gpt-4", "gpt-35-turbo",
        ],
    });

    m.insert(CHANNEL_OLLAMA, ChannelDef {
        base_url: "http://localhost:11434",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/api/tags",
        models: &[],
    });

    m.insert(CHANNEL_CUSTOM, ChannelDef {
        base_url: "",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/v1/models",
        models: &[],
    });

    m.insert(CHANNEL_ANTHROPIC, ChannelDef {
        base_url: "https://api.anthropic.com",
        chat_path: ChatPath::Anthropic,
        auth: AuthScheme::XApiKey,
        models_path: "/v1/models",
        models: &[
            "claude-sonnet-4-20250514", "claude-3-7-sonnet-20250219",
            "claude-3-5-sonnet-20241022", "claude-3-5-haiku-20241022",
            "claude-3-opus-20240229", "claude-3-haiku-20240307",
        ],
    });

    m.insert(CHANNEL_BAIDU, ChannelDef {
        base_url: "https://aip.baidubce.com",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/v1/models",
        models: &[
            "ernie-4.0-8k", "ernie-4.0-turbo-8k", "ernie-3.5-8k",
            "ernie-speed-128k", "ernie-lite-8k", "ernie-tiny-8k",
        ],
    });

    m.insert(CHANNEL_ZHIPU, ChannelDef {
        base_url: "https://open.bigmodel.cn",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/api/paas/v4/models",
        models: &[
            "glm-4-plus", "glm-4-long", "glm-4-air", "glm-4-airx",
            "glm-4-flash", "glm-4-flashx", "glm-4v-plus", "glm-4v",
        ],
    });

    m.insert(CHANNEL_ALI, ChannelDef {
        base_url: "https://dashscope.aliyuncs.com",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/compatible-mode/v1/models",
        models: &[
            "qwen-max", "qwen-plus", "qwen-turbo", "qwen-long",
            "qwen-vl-max", "qwen-vl-plus", "qwen2.5-72b-instruct",
            "qwen2.5-32b-instruct", "qwen2.5-14b-instruct", "qwen2.5-7b-instruct",
        ],
    });

    m.insert(CHANNEL_OPENROUTER, ChannelDef {
        base_url: "https://openrouter.ai/api",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/v1/models",
        models: &[
            "openai/gpt-4o", "anthropic/claude-sonnet-4-20250514",
            "google/gemini-2.5-pro-preview", "meta-llama/llama-3.1-405b-instruct",
            "deepseek/deepseek-chat",
        ],
    });

    m.insert(CHANNEL_TENCENT, ChannelDef {
        base_url: "https://hunyuan.tencentcloudapi.com",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/v1/models",
        models: &[
            "hunyuan-pro", "hunyuan-standard", "hunyuan-lite",
            "hunyuan-turbo", "hunyuan-vision",
        ],
    });

    m.insert(CHANNEL_GEMINI, ChannelDef {
        base_url: "https://generativelanguage.googleapis.com",
        chat_path: ChatPath::Gemini,
        auth: AuthScheme::GoogleApiKey,
        models_path: "/v1beta/models",
        models: &[
            "gemini-2.5-pro-preview-05-06", "gemini-2.5-flash-preview-04-17",
            "gemini-2.0-flash", "gemini-2.0-flash-lite",
            "gemini-1.5-pro", "gemini-1.5-flash",
        ],
    });

    m.insert(CHANNEL_MOONSHOT, ChannelDef {
        base_url: "https://api.moonshot.cn",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/v1/models",
        models: &[
            "moonshot-v1-8k", "moonshot-v1-32k", "moonshot-v1-128k",
        ],
    });

    m.insert(CHANNEL_ZHIPU_V4, ChannelDef {
        base_url: "https://open.bigmodel.cn/api/paas/v4",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/models",
        models: &[
            "glm-4-plus", "glm-4-long", "glm-4-air", "glm-4-airx",
            "glm-4-flash", "glm-4-flashx", "glm-4v-plus",
        ],
    });

    m.insert(CHANNEL_PERPLEXITY, ChannelDef {
        base_url: "https://api.perplexity.ai",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/v1/models",
        models: &[
            "sonar-pro", "sonar", "sonar-reasoning-pro",
            "sonar-reasoning", "sonar-deep-research",
        ],
    });

    m.insert(CHANNEL_LINGYI, ChannelDef {
        base_url: "https://api.lingyiwanwu.com",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/v1/models",
        models: &[
            "yi-lightning", "yi-large", "yi-large-turbo",
            "yi-medium", "yi-spark",
        ],
    });

    m.insert(CHANNEL_COHERE, ChannelDef {
        base_url: "https://api.cohere.ai",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/compatibility/v1/models",
        models: &[
            "command-r-plus", "command-r", "command-light",
        ],
    });

    m.insert(CHANNEL_MINIMAX, ChannelDef {
        base_url: "https://api.minimax.chat",
        chat_path: ChatPath::MiniMax,
        auth: AuthScheme::Bearer,
        models_path: "/v1/models",
        models: &[
            "MiniMax-M2.7", "MiniMax-M2.7-highspeed",
            "MiniMax-M2.5", "MiniMax-M2.5-highspeed",
            "MiniMax-M2.1", "MiniMax-M2.1-highspeed",
            "MiniMax-M2", "MiniMax-Text-01",
            "abab6.5-chat", "abab6.5s-chat", "abab6-chat",
            "abab5.5-chat", "abab5.5s-chat",
            "speech-2.5-hd-preview", "speech-2.5-turbo-preview",
            "speech-02-hd", "speech-02-turbo",
            "speech-01-hd", "speech-01-turbo",
            "image-01", "image-01-live",
        ],
    });

    m.insert(CHANNEL_SILICONFLOW, ChannelDef {
        base_url: "https://api.siliconflow.cn",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/v1/models",
        models: &[
            "deepseek-ai/DeepSeek-V3", "deepseek-ai/DeepSeek-R1",
            "Qwen/Qwen2.5-72B-Instruct", "Pro/Qwen/Qwen2.5-7B-Instruct",
            "THUDM/glm-4-9b-chat",
        ],
    });

    m.insert(CHANNEL_MISTRAL, ChannelDef {
        base_url: "https://api.mistral.ai",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/v1/models",
        models: &[
            "mistral-large-latest", "mistral-medium-latest",
            "mistral-small-latest", "open-mixtral-8x22b",
            "codestral-latest",
        ],
    });

    m.insert(CHANNEL_DEEPSEEK, ChannelDef {
        base_url: "https://api.deepseek.com",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/v1/models",
        models: &[
            "deepseek-chat", "deepseek-reasoner",
        ],
    });

    m.insert(CHANNEL_VOLCENGINE, ChannelDef {
        base_url: "https://ark.cn-beijing.volces.com",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/api/v3/models",
        models: &[
            "doubao-pro-32k", "doubao-pro-128k", "doubao-lite-32k",
        ],
    });

    m.insert(CHANNEL_XAI, ChannelDef {
        base_url: "https://api.x.ai",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/v1/models",
        models: &[
            "grok-3", "grok-3-mini", "grok-2", "grok-2-vision",
        ],
    });

    m.insert(CHANNEL_COZE, ChannelDef {
        base_url: "https://api.coze.cn",
        chat_path: ChatPath::OpenAI,
        auth: AuthScheme::Bearer,
        models_path: "/v1/models",
        models: &[],
    });

    m
}

use std::sync::OnceLock;

static REGISTRY: OnceLock<HashMap<u32, ChannelDef>> = OnceLock::new();

fn reg() -> &'static HashMap<u32, ChannelDef> {
    REGISTRY.get_or_init(channel_registry)
}

pub fn get_channel_def(channel_type: u32) -> Option<&'static ChannelDef> {
    reg().get(&channel_type)
}

pub fn get_default_base_url(channel_type: u32) -> &'static str {
    reg().get(&channel_type).map(|d| d.base_url).unwrap_or("")
}

pub fn get_default_models(channel_type: u32) -> Vec<String> {
    reg().get(&channel_type)
        .map(|d| d.models.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default()
}

/// Build the full chat completion URL based on channel type.
pub fn build_chat_url(channel_type: u32, base_url: &str, model_id: &str, api_key: &str) -> String {
    let def = reg().get(&channel_type);
    let chat_path = def.map(|d| d.chat_path).unwrap_or(ChatPath::OpenAI);
    let base = base_url.trim_end_matches('/');

    match chat_path {
        ChatPath::OpenAI => build_openai_chat_url(channel_type, base),
        ChatPath::Anthropic => format!("{}/v1/messages", base),
        ChatPath::MiniMax => format!("{}/v1/text/chatcompletion_v2", base),
        ChatPath::Gemini => {
            let version = if base.contains("/v1beta") { "" } else { "/v1beta" };
            format!("{}{}/models/{}:streamGenerateContent?alt=sse&key={}",
                base, version, model_id, api_key)
        }
        ChatPath::Azure => {
            format!("{}/openai/deployments/{}/chat/completions?api-version=2024-06-01",
                base, model_id)
        }
    }
}

/// Build OpenAI-compatible chat URL, handling per-channel path prefixes.
fn build_openai_chat_url(channel_type: u32, base: &str) -> String {
    match channel_type {
        CHANNEL_ALI => format!("{}/compatible-mode/v1/chat/completions", base),
        CHANNEL_COHERE => format!("{}/compatibility/v1/chat/completions", base),
        CHANNEL_VOLCENGINE => format!("{}/api/v3/chat/completions", base),
        CHANNEL_ZHIPU_V4 => format!("{}/chat/completions", base),
        _ => {
            // If the base URL already contains a path with /chat/completions, use it directly
            if base.contains("/chat/completions") {
                return base.to_string();
            }
            // Detect any version prefix like /v1, /v2, /v3, /v4, etc.
            let last_segment = base.rsplit('/').next().unwrap_or("");
            let has_version = last_segment.len() >= 2
                && last_segment.starts_with('v')
                && last_segment[1..].chars().all(|c| c.is_ascii_digit());
            if has_version {
                format!("{}/chat/completions", base)
            } else {
                format!("{}/v1/chat/completions", base)
            }
        }
    }
}

/// Build the models listing URL for a channel.
pub fn build_models_url(channel_type: u32, base_url: &str, api_key: &str) -> Vec<String> {
    let def = reg().get(&channel_type);
    let base = base_url.trim_end_matches('/');

    if let Some(d) = def {
        match d.chat_path {
            ChatPath::Gemini => {
                let version = if base.contains("/v1beta") { "" } else { "/v1beta" };
                vec![format!("{}{}/models?key={}", base, version, api_key)]
            }
            ChatPath::Azure => {
                vec![format!("{}/openai/models?api-version=2024-06-01", base)]
            }
            _ => {
                if channel_type == CHANNEL_CUSTOM {
                    // For custom endpoints, detect version prefix and append /models
                    let last_segment = base.rsplit('/').next().unwrap_or("");
                    let has_version = last_segment.len() >= 2
                        && last_segment.starts_with('v')
                        && last_segment[1..].chars().all(|c| c.is_ascii_digit());
                    if has_version {
                        vec![format!("{}/models", base)]
                    } else {
                        vec![format!("{}/v1/models", base)]
                    }
                } else {
                    vec![format!("{}{}", base, d.models_path)]
                }
            }
        }
    } else {
        vec![
            format!("{}/v1/models", base),
        ]
    }
}

/// Build auth headers for a channel type.
pub fn build_auth_headers(channel_type: u32, api_key: &str) -> Vec<(String, String)> {
    let def = reg().get(&channel_type);
    let scheme = def.map(|d| d.auth).unwrap_or(AuthScheme::Bearer);

    match scheme {
        AuthScheme::Bearer => {
            vec![("Authorization".to_string(), format!("Bearer {}", api_key))]
        }
        AuthScheme::XApiKey => {
            vec![
                ("x-api-key".to_string(), api_key.to_string()),
                ("anthropic-version".to_string(), "2023-06-01".to_string()),
            ]
        }
        AuthScheme::GoogleApiKey => {
            vec![("x-goog-api-key".to_string(), api_key.to_string())]
        }
        AuthScheme::AzureApiKey => {
            vec![("api-key".to_string(), api_key.to_string())]
        }
        AuthScheme::QueryKeyParam => {
            vec![]
        }
    }
}

/// Get the ChatPath classification for dispatching streaming logic.
pub fn get_chat_path(channel_type: u32) -> ChatPath {
    reg().get(&channel_type).map(|d| d.chat_path).unwrap_or(ChatPath::OpenAI)
}

pub fn list_all_channel_types() -> Vec<(u32, &'static str)> {
    vec![
        (CHANNEL_OPENAI, "OpenAI"),
        (CHANNEL_AZURE, "Azure OpenAI"),
        (CHANNEL_OLLAMA, "Ollama"),
        (CHANNEL_CUSTOM, "Custom/兼容接口"),
        (CHANNEL_ANTHROPIC, "Anthropic"),
        (CHANNEL_BAIDU, "百度文心"),
        (CHANNEL_ZHIPU, "智谱 AI"),
        (CHANNEL_ALI, "阿里通义"),
        (CHANNEL_OPENROUTER, "OpenRouter"),
        (CHANNEL_TENCENT, "腾讯混元"),
        (CHANNEL_GEMINI, "Google Gemini"),
        (CHANNEL_MOONSHOT, "Moonshot/月之暗面"),
        (CHANNEL_ZHIPU_V4, "智谱 GLM-4"),
        (CHANNEL_PERPLEXITY, "Perplexity"),
        (CHANNEL_LINGYI, "零一万物"),
        (CHANNEL_COHERE, "Cohere"),
        (CHANNEL_MINIMAX, "MiniMax"),
        (CHANNEL_SILICONFLOW, "SiliconFlow/硅基流动"),
        (CHANNEL_MISTRAL, "Mistral AI"),
        (CHANNEL_DEEPSEEK, "DeepSeek"),
        (CHANNEL_VOLCENGINE, "火山引擎/豆包"),
        (CHANNEL_XAI, "xAI/Grok"),
        (CHANNEL_COZE, "Coze/扣子"),
    ]
}
