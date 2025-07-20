use serde::{Deserialize, Serialize};
use uuid::Uuid;

// WebSocket消息类型 - 优化序列化
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebSocketMessage {
    Connect {
        host: String,
        port: u16,
        username: String,
        #[serde(skip_serializing)]
        password: String,
    },
    Data {
        session_id: Uuid,
        data: String,
    },
    Disconnect {
        session_id: Uuid,
    },
    // 新增批量操作支持
    BatchData {
        session_id: Uuid,
        data: Vec<String>,
    },
    // 心跳消息
    Ping,
    Pong,
}

// WebSocket响应类型 - 优化序列化
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebSocketResponse {
    Connected {
        session_id: Uuid,
    },
    Data {
        data: String,
    },
    Error {
        message: String,
    },
    Disconnected,
    // 新增批量响应
    BatchData {
        data: Vec<String>,
    },
    // 状态更新
    Status {
        session_id: Uuid,
        connected: bool,
        latency_ms: u64,
    },
    Pong,
}

// AI请求类型 - 支持更多功能
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIRequest {
    pub message: String,
    pub session_id: Option<Uuid>,
    pub context: Option<String>,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    pub stream: Option<bool>,
}

// AI响应类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIResponse {
    pub response: String,
    pub tokens_used: Option<usize>,
    pub model: Option<String>,
    pub cached: bool,
}

// SSH配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSHConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(skip_serializing)]
    pub password: Option<String>,
    #[serde(skip_serializing)]
    pub private_key: Option<String>,
    pub timeout: Option<u64>,
    pub keepalive_interval: Option<u64>,
    pub compression: Option<bool>,
}

// 会话信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: Uuid,
    pub host: String,
    pub username: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

// 错误类型
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("SSH connection error: {0}")]
    SSHError(String),
    
    #[error("WebSocket error: {0}")]
    WebSocketError(String),
    
    #[error("AI service error: {0}")]
    AIError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Internal server error: {0}")]
    InternalError(String),
}

impl warp::reject::Reject for AppError {}

// 性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub active_connections: u64,
    pub total_connections: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub ai_requests: u64,
    pub ai_cache_hits: u64,
    pub average_response_time_ms: f64,
    pub uptime_seconds: u64,
}

// 批量操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult<T> {
    pub successful: Vec<T>,
    pub failed: Vec<BatchError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchError {
    pub index: usize,
    pub error: String,
}

// 命令历史
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHistory {
    pub session_id: Uuid,
    pub command: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub exit_code: Option<i32>,
}

// 会话统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub session_id: Uuid,
    pub duration_seconds: u64,
    pub commands_executed: u64,
    pub data_transferred_bytes: u64,
    pub ai_interactions: u64,
    pub error_count: u64,
}
