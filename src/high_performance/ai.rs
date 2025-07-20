use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use futures::future::join_all;

use crate::models::AIRequest;

// 复用HTTP客户端
lazy_static::lazy_static! {
    static ref HTTP_CLIENT: Client = Client::builder()
        .pool_max_idle_per_host(20)
        .pool_idle_timeout(Duration::from_secs(90))
        .timeout(Duration::from_secs(30))
        .http2_prior_knowledge()
        .gzip(true)
        .brotli(true)
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
}

pub struct AIProcessor {
    providers: Vec<Box<dyn AIProvider + Send + Sync>>,
}

#[async_trait::async_trait]
trait AIProvider {
    async fn process(&self, prompt: &str, context: &str) -> anyhow::Result<String>;
    fn name(&self) -> &str;
}

struct OpenAIProvider {
    api_key: String,
    model: String,
}

#[async_trait::async_trait]
impl AIProvider for OpenAIProvider {
    async fn process(&self, prompt: &str, context: &str) -> anyhow::Result<String> {
        let request_body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": context},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.7,
            "max_tokens": 1000,
            "stream": false
        });

        let response = HTTP_CLIENT
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        
        Ok(json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }
    
    fn name(&self) -> &str {
        "OpenAI"
    }
}

struct ClaudeProvider {
    api_key: String,
    model: String,
}

#[async_trait::async_trait]
impl AIProvider for ClaudeProvider {
    async fn process(&self, prompt: &str, context: &str) -> anyhow::Result<String> {
        let request_body = json!({
            "model": self.model,
            "max_tokens": 1000,
            "messages": [
                {"role": "user", "content": format!("{}\n\n{}", context, prompt)}
            ]
        });

        let response = HTTP_CLIENT
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request_body)
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        
        Ok(json["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }
    
    fn name(&self) -> &str {
        "Claude"
    }
}

impl AIProcessor {
    pub fn new() -> Self {
        let mut providers: Vec<Box<dyn AIProvider + Send + Sync>> = Vec::new();
        
        // 从环境变量加载配置
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            providers.push(Box::new(OpenAIProvider {
                api_key,
                model: std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string()),
            }));
        }
        
        if let Ok(api_key) = std::env::var("CLAUDE_API_KEY") {
            providers.push(Box::new(ClaudeProvider {
                api_key,
                model: std::env::var("CLAUDE_MODEL").unwrap_or_else(|_| "claude-3-haiku-20240307".to_string()),
            }));
        }
        
        Self { providers }
    }
    
    pub async fn process(&self, request: AIRequest) -> anyhow::Result<String> {
        let context = request.context.unwrap_or_else(|| "You are an SSH terminal assistant.".to_string());
        
        // 尝试主提供商
        if let Some(primary) = self.providers.first() {
            match primary.process(&request.message, &context).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    tracing::error!("Primary AI provider failed: {}", e);
                }
            }
        }
        
        // 降级到备用提供商
        for provider in self.providers.iter().skip(1) {
            match provider.process(&request.message, &context).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    tracing::error!("Fallback AI provider {} failed: {}", provider.name(), e);
                }
            }
        }
        
        Err(anyhow::anyhow!("All AI providers failed"))
    }
    
    // 并行请求多个提供商，返回最快的
    pub async fn race_process(&self, request: AIRequest) -> anyhow::Result<String> {
        let context = request.context.unwrap_or_else(|| "You are an SSH terminal assistant.".to_string());
        
        let futures: Vec<_> = self.providers
            .iter()
            .map(|provider| {
                let message = request.message.clone();
                let context = context.clone();
                async move {
                    provider.process(&message, &context).await
                }
            })
            .collect();
        
        // 使用select!等待第一个成功的
        let (result, _, _) = futures_util::future::select_all(futures).await;
        
        result
    }
}
