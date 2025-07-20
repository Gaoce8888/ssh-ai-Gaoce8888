use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;
use tracing::{info, error, instrument};
use futures_util::future::join_all;

use crate::{Sessions, models::*, cache::AIResponseCache};

// AI客户端池
lazy_static::lazy_static! {
    static ref AI_CLIENT: Client = Client::builder()
        .timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(90))
        .http2_prior_knowledge()
        .gzip(true)
        .build()
        .unwrap();
}

// AI提供商trait
#[async_trait::async_trait]
trait AIProvider: Send + Sync {
    async fn complete(&self, prompt: &str, context: &str) -> Result<String>;
    fn name(&self) -> &str;
}

// OpenAI提供商
struct OpenAIProvider {
    api_key: String,
    model: String,
    endpoint: String,
}

#[async_trait::async_trait]
impl AIProvider for OpenAIProvider {
    async fn complete(&self, prompt: &str, context: &str) -> Result<String> {
        let request = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": context},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.7,
            "max_tokens": 1000,
            "stream": false
        });

        let response = AI_CLIENT
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;

        let response_json: serde_json::Value = response.json().await?;
        
        response_json["choices"][0]["message"]["content"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))
    }
    
    fn name(&self) -> &str {
        "OpenAI"
    }
}

// Claude提供商
struct ClaudeProvider {
    api_key: String,
    model: String,
    endpoint: String,
}

#[async_trait::async_trait]
impl AIProvider for ClaudeProvider {
    async fn complete(&self, prompt: &str, context: &str) -> Result<String> {
        let request = serde_json::json!({
            "model": self.model,
            "max_tokens": 1000,
            "messages": [
                {"role": "user", "content": format!("{}\n\n{}", context, prompt)}
            ]
        });

        let response = AI_CLIENT
            .post(&self.endpoint)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Claude")?;

        let response_json: serde_json::Value = response.json().await?;
        
        response_json["content"][0]["text"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))
    }
    
    fn name(&self) -> &str {
        "Claude"
    }
}

// AI管理器
pub struct AIManager {
    providers: Vec<Box<dyn AIProvider>>,
    cache: AIResponseCache,
    fallback_enabled: bool,
}

impl AIManager {
    pub fn new(config: &crate::config::Config) -> Self {
        let mut providers: Vec<Box<dyn AIProvider>> = Vec::new();
        
        for provider_config in &config.ai_providers {
            match provider_config.name.as_str() {
                "openai" => {
                    providers.push(Box::new(OpenAIProvider {
                        api_key: provider_config.api_key.clone(),
                        model: provider_config.model.clone(),
                        endpoint: provider_config.endpoint.clone(),
                    }));
                }
                "claude" => {
                    providers.push(Box::new(ClaudeProvider {
                        api_key: provider_config.api_key.clone(),
                        model: provider_config.model.clone(),
                        endpoint: provider_config.endpoint.clone(),
                    }));
                }
                _ => {}
            }
        }
        
        Self {
            providers,
            cache: AIResponseCache::new(config.cache_size),
            fallback_enabled: true,
        }
    }
    
    #[instrument(skip(self))]
    pub async fn complete(&self, prompt: &str, context: &str) -> Result<String> {
        // 检查缓存
        if let Some(cached) = self.cache.get_ai_response(prompt, context).await {
            info!("AI response served from cache");
            return Ok(cached);
        }
        
        // 尝试主提供商
        if let Some(primary) = self.providers.first() {
            match primary.complete(prompt, context).await {
                Ok(response) => {
                    // 缓存成功的响应
                    self.cache.set_ai_response(
                        prompt.to_string(), 
                        context.to_string(), 
                        response.clone()
                    ).await;
                    
                    return Ok(response);
                }
                Err(e) => {
                    error!("Primary AI provider {} failed: {}", primary.name(), e);
                    
                    if !self.fallback_enabled {
                        return Err(e);
                    }
                }
            }
        }
        
        // 降级到备用提供商
        for provider in self.providers.iter().skip(1) {
            match provider.complete(prompt, context).await {
                Ok(response) => {
                    info!("AI response from fallback provider: {}", provider.name());
                    
                    // 缓存成功的响应
                    self.cache.set_ai_response(
                        prompt.to_string(), 
                        context.to_string(), 
                        response.clone()
                    ).await;
                    
                    return Ok(response);
                }
                Err(e) => {
                    error!("Fallback AI provider {} failed: {}", provider.name(), e);
                }
            }
        }
        
        Err(anyhow::anyhow!("All AI providers failed"))
    }
    
    // 并行请求多个提供商，返回最快的响应
    pub async fn race_complete(&self, prompt: &str, context: &str) -> Result<String> {
        if self.providers.is_empty() {
            return Err(anyhow::anyhow!("No AI providers configured"));
        }
        
        let futures: Vec<_> = self.providers.iter()
            .map(|provider| {
                let prompt = prompt.to_string();
                let context = context.to_string();
                async move {
                    provider.complete(&prompt, &context).await
                }
            })
            .collect();
        
        // 使用select!宏等待第一个成功的响应
        let (result, _index, _remaining) = futures_util::future::select_all(futures).await;
        
        result
    }
}

#[instrument(skip(sessions))]
pub async fn process_ai_request_optimized(
    request: AIRequest,
    sessions: Sessions,
) -> Result<serde_json::Value> {
    // 获取SSH会话上下文
    let context = if let Some(session_id) = request.session_id {
        if let Some(session) = sessions.get(&session_id) {
            let ssh = session.read().await;
            format!("SSH session active for: {}", session_id)
        } else {
            "No active SSH session".to_string()
        }
    } else {
        "General AI assistance".to_string()
    };
    
    // 使用全局AI管理器处理请求
    let ai_manager = AI_MANAGER.get().unwrap();
    let response = ai_manager.complete(&request.message, &context).await?;
    
    Ok(serde_json::json!({
        "response": response,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

// 全局AI管理器
use once_cell::sync::OnceCell;
static AI_MANAGER: OnceCell<AIManager> = OnceCell::new();

pub fn init_ai_manager(config: &crate::config::Config) {
    AI_MANAGER.set(AIManager::new(config)).unwrap();
}
