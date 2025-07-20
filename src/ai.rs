use reqwest;
use serde_json::json;
use tracing::{error, info};
use thiserror::Error;

use crate::{Sessions, models::*};

#[derive(Error, Debug)]
pub enum AIError {
    #[error("请求失败: {0}")]
    RequestFailed(String),
    #[error("响应格式无效")]
    InvalidResponse,
    #[error("不支持的AI提供商: {0}")]
    UnsupportedProvider(String),
}

pub async fn process_ai_request(
    request: AIRequest,
    sessions: Sessions,
) -> Result<AIResponse, AIError> {
    let provider = request.ai_config.provider.as_deref().unwrap_or("openai");
    
    match provider {
        "openai" => process_openai_request(request, sessions).await,
        "claude" => process_claude_request(request, sessions).await,
        _ => Err(AIError::UnsupportedProvider(provider.to_string())),
    }
}

async fn process_openai_request(
    request: AIRequest,
    sessions: Sessions,
) -> Result<AIResponse, AIError> {
    let endpoint = request.ai_config.endpoint
        .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string());
    
    let system_prompt = request.ai_config.system_prompt
        .unwrap_or_else(|| "你是一个有用的系统管理员助手。帮助用户安全地执行服务器命令。".to_string());
    
    let mut messages = vec![
        json!({
            "role": "system",
            "content": system_prompt
        }),
        json!({
            "role": "user",
            "content": request.message
        })
    ];
    
    if let Some(session_id) = request.session_id {
        if let Some(_session) = sessions.get(&session_id) {
            messages.insert(1, json!({
                "role": "system",
                "content": "用户有一个活跃的SSH会话。"
            }));
        }
    }

    info!("发送OpenAI请求: 模型={}, 消息长度={}", request.ai_config.model, messages.len());
    
    let client = reqwest::Client::new();
    let response = client.post(&endpoint)
        .header("Authorization", format!("Bearer {}", request.ai_config.api_key))
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": request.ai_config.model,
            "messages": messages,
            "temperature": request.ai_config.temperature.unwrap_or(0.7),
            "max_tokens": request.ai_config.max_tokens.unwrap_or(2048)
        }))
        .send()
        .await
        .map_err(|e| AIError::RequestFailed(e.to_string()))?;
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        error!("OpenAI API错误: {}", error_text);
        return Err(AIError::RequestFailed(error_text));
    }
    
    let result: serde_json::Value = response.json()
        .await
        .map_err(|e| {
            error!("解析OpenAI响应失败: {}", e);
            AIError::InvalidResponse
        })?;
    
    let ai_response = result["choices"][0]["message"]["content"]
        .as_str()
        .ok_or(AIError::InvalidResponse)?
        .to_string();
    
    let command = extract_command(&ai_response);
    
    Ok(AIResponse {
        response: ai_response,
        command,
    })
}

async fn process_claude_request(
    request: AIRequest,
    sessions: Sessions,
) -> Result<AIResponse, AIError> {
    let endpoint = request.ai_config.endpoint
        .unwrap_or_else(|| "https://api.anthropic.com/v1/messages".to_string());
    
    let system_prompt = request.ai_config.system_prompt
        .unwrap_or_else(|| "你是一个有用的系统管理员助手。帮助用户安全地执行服务器命令。".to_string());
    
    let mut user_content = request.message.clone();
    
    if let Some(session_id) = request.session_id {
        if let Some(_session) = sessions.get(&session_id) {
            user_content = format!("用户有一个活跃的SSH会话。\n\n{}", user_content);
        }
    }

    info!("发送Claude请求: 模型={}", request.ai_config.model);
    
    let client = reqwest::Client::new();
    let response = client.post(&endpoint)
        .header("x-api-key", &request.ai_config.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": request.ai_config.model,
            "max_tokens": request.ai_config.max_tokens.unwrap_or(2048),
            "temperature": request.ai_config.temperature.unwrap_or(0.7),
            "system": system_prompt,
            "messages": [
                {
                    "role": "user",
                    "content": user_content
                }
            ]
        }))
        .send()
        .await
        .map_err(|e| AIError::RequestFailed(e.to_string()))?;
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        error!("Claude API错误: {}", error_text);
        return Err(AIError::RequestFailed(error_text));
    }
    
    let result: serde_json::Value = response.json()
        .await
        .map_err(|e| {
            error!("解析Claude响应失败: {}", e);
            AIError::InvalidResponse
        })?;
    
    let ai_response = result["content"][0]["text"]
        .as_str()
        .ok_or(AIError::InvalidResponse)?
        .to_string();
    
    let command = extract_command(&ai_response);
    
    Ok(AIResponse {
        response: ai_response,
        command,
    })
}

fn extract_command(response: &str) -> Option<String> {
    if let Some(start) = response.find("```bash") {
        let start = start + 7;
        if let Some(end) = response[start..].find("```") {
            return Some(response[start..start + end].trim().to_string());
        }
    } else if let Some(start) = response.find("```sh") {
        let start = start + 5;
        if let Some(end) = response[start..].find("```") {
            return Some(response[start..start + end].trim().to_string());
        }
    } else if let Some(start) = response.find("```") {
        let start = start + 3;
        if let Some(end) = response[start..].find("```") {
            let content = response[start..start + end].trim();
            if !content.contains('\n') && (content.starts_with("sudo") || 
                content.starts_with("apt") || content.starts_with("yum") ||
                content.starts_with("docker") || content.starts_with("systemctl")) {
                return Some(content.to_string());
            }
        }
    }
    
    None
}
