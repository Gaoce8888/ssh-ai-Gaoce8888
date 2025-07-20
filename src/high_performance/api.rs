use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{AppState, models::*};

// 配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigData {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    pub ai_config: Option<AIConfig>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub endpoint: Option<String>,
}

// 获取所有配置
pub async fn get_configs(State(state): State<AppState>) -> Result<Json<Vec<ConfigData>>, StatusCode> {
    // 从缓存获取
    let configs = state.config_cache
        .iter()
        .map(|(_, v)| v)
        .collect::<Vec<_>>();
    
    Ok(Json(configs))
}

// 获取单个配置
pub async fn get_config(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<ConfigData>, StatusCode> {
    if let Some(config) = state.config_cache.get(&id).await {
        Ok(Json(config))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// 保存配置
pub async fn save_config(
    State(state): State<AppState>,
    Json(mut config): Json<ConfigData>,
) -> Result<Json<ConfigData>, StatusCode> {
    // 生成ID如果没有
    if config.id.is_empty() {
        config.id = Uuid::new_v4().to_string();
    }
    
    // 更新时间戳
    config.updated_at = chrono::Utc::now();
    if config.created_at.timestamp() == 0 {
        config.created_at = chrono::Utc::now();
    }
    
    // 存入缓存
    state.config_cache.insert(config.id.clone(), config.clone()).await;
    
    // 异步持久化到磁盘
    let config_clone = config.clone();
    tokio::spawn(async move {
        persist_config(&config_clone).await;
    });
    
    Ok(Json(config))
}

// 删除配置
pub async fn delete_config(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> StatusCode {
    state.config_cache.remove(&id).await;
    
    // 异步删除磁盘文件
    tokio::spawn(async move {
        delete_config_file(&id).await;
    });
    
    StatusCode::NO_CONTENT
}

// AI聊天API
pub async fn ai_chat(
    State(state): State<AppState>,
    Json(request): Json<AIRequest>,
) -> Result<Json<AIResponse>, StatusCode> {
    // 生成缓存键
    let cache_key = format!("{}-{}", request.session_id.unwrap_or_default(), request.message);
    
    // 检查缓存
    if let Some(cached) = state.ai_cache.get(&cache_key).await {
        return Ok(Json(AIResponse {
            response: cached,
            cached: true,
            ..Default::default()
        }));
    }
    
    // 处理AI请求
    match process_ai_request(request, &state).await {
        Ok(response) => {
            // 缓存响应
            state.ai_cache.insert(cache_key, response.clone()).await;
            
            Ok(Json(AIResponse {
                response,
                cached: false,
                ..Default::default()
            }))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// AI请求处理
async fn process_ai_request(request: AIRequest, state: &AppState) -> Result<String, anyhow::Error> {
    // 使用高性能HTTP客户端
    use crate::ai::AIProcessor;
    
    let processor = AIProcessor::new();
    processor.process(request).await
}

// 持久化配置到磁盘
async fn persist_config(config: &ConfigData) -> Result<(), std::io::Error> {
    use tokio::fs;
    use tokio::io::AsyncWriteExt;
    
    let config_dir = "static/configs";
    fs::create_dir_all(config_dir).await?;
    
    let file_path = format!("{}/{}.json", config_dir, config.id);
    let json = serde_json::to_string_pretty(config)?;
    
    let mut file = fs::File::create(file_path).await?;
    file.write_all(json.as_bytes()).await?;
    file.sync_all().await?;
    
    Ok(())
}

// 删除配置文件
async fn delete_config_file(id: &str) -> Result<(), std::io::Error> {
    use tokio::fs;
    
    let file_path = format!("static/configs/{}.json", id);
    fs::remove_file(file_path).await?;
    
    Ok(())
}

// 加载所有配置到缓存
pub async fn load_configs_to_cache(cache: &moka::future::Cache<String, ConfigData>) -> Result<(), anyhow::Error> {
    use tokio::fs;
    
    let config_dir = "static/configs";
    fs::create_dir_all(config_dir).await?;
    
    let mut entries = fs::read_dir(config_dir).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        if let Some(name) = entry.file_name().to_str() {
            if name.ends_with(".json") {
                let content = fs::read_to_string(entry.path()).await?;
                if let Ok(config) = serde_json::from_str::<ConfigData>(&content) {
                    cache.insert(config.id.clone(), config).await;
                }
            }
        }
    }
    
    Ok(())
}

#[derive(Default, Serialize)]
pub struct AIResponse {
    pub response: String,
    pub cached: bool,
    pub tokens_used: Option<usize>,
    pub model: Option<String>,
    pub latency_ms: Option<u64>,
}
