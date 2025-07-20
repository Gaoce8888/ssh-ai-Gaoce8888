use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    #[serde(rename = "connect")]
    Connect {
        host: String,
        port: u16,
        username: String,
        password: String,
    },
    #[serde(rename = "data")]
    Data {
        session_id: Uuid,
        data: String,
    },
    #[serde(rename = "disconnect")]
    Disconnect {
        session_id: Uuid,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum WebSocketResponse {
    #[serde(rename = "connected")]
    Connected { session_id: Uuid },
    #[serde(rename = "data")]
    Data { data: String },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "disconnected")]
    Disconnected,
}

#[derive(Debug, Deserialize)]
pub struct AIRequest {
    pub message: String,
    pub session_id: Option<Uuid>,
    pub ai_config: AIConfig,
}

#[derive(Debug, Deserialize)]
pub struct AIConfig {
    pub provider: Option<String>,
    #[serde(rename = "apiKey")]
    pub api_key: String,
    pub model: String,
    pub endpoint: Option<String>,
    #[serde(rename = "systemPrompt")]
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    #[serde(rename = "maxTokens")]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct AIResponse {
    pub response: String,
    pub command: Option<String>,
}
