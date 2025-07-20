use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use tracing::{info, warn, error};
use metrics::{counter, gauge};
use std::time::{Duration, Instant};
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use sha2::{Sha256, Digest};
use once_cell::sync::Lazy;

use crate::models::*;
use crate::config::AuthConfig;

pub struct AuthManager {
    config: AuthConfig,
    sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
    rate_limit: Arc<RwLock<HashMap<String, RateLimitInfo>>>,
    jwt_secret: String,
}

#[derive(Debug, Clone)]
struct SessionInfo {
    user_id: String,
    created_at: Instant,
    last_activity: Instant,
    is_active: bool,
}

#[derive(Debug, Clone)]
struct RateLimitInfo {
    attempts: u32,
    last_attempt: Instant,
    locked_until: Option<Instant>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String, // user_id
    exp: usize,  // expiration time
    iat: usize,  // issued at
    session_id: String,
}

impl AuthManager {
    pub fn new(config: &AuthConfig) -> Self {
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "default-secret-key-change-in-production".to_string());
        
        let manager = Self {
            config: config.clone(),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            rate_limit: Arc::new(RwLock::new(HashMap::new())),
            jwt_secret,
        };

        // 启动会话清理任务
        let manager_clone = manager.clone();
        tokio::spawn(async move {
            manager_clone.cleanup_sessions().await;
        });

        info!("Auth manager initialized");
        manager
    }

    pub async fn authenticate(&self, username: &str, password: &str) -> Result<AuthResult, AuthError> {
        // 检查速率限制
        if self.is_rate_limited(username).await {
            counter!("auth_rate_limited", 1);
            return Err(AuthError::RateLimited);
        }

        // 验证凭据
        if username == self.config.username && self.verify_password(password, &self.config.password) {
            self.reset_rate_limit(username).await;
            let session_id = Uuid::new_v4().to_string();
            let token = self.create_jwt_token(username, &session_id)?;
            
            // 创建会话
            self.create_session(&session_id, username).await;
            
            counter!("auth_success", 1);
            Ok(AuthResult {
                token,
                session_id,
                expires_in: self.config.session_timeout,
            })
        } else {
            self.increment_rate_limit(username).await;
            counter!("auth_failure", 1);
            Err(AuthError::InvalidCredentials)
        }
    }

    pub async fn verify_token(&self, token: &str) -> Result<TokenInfo, AuthError> {
        let claims = self.decode_jwt_token(token)?;
        
        // 检查会话是否有效
        if !self.is_session_valid(&claims.session_id).await {
            return Err(AuthError::InvalidSession);
        }

        // 更新最后活动时间
        self.update_session_activity(&claims.session_id).await;
        
        Ok(TokenInfo {
            user_id: claims.sub,
            session_id: claims.session_id,
            expires_at: claims.exp,
        })
    }

    pub async fn verify_request(&self, request: &AIRequest) -> Result<(), AuthError> {
        // 这里可以根据需要实现更复杂的请求验证逻辑
        // 例如检查用户权限、API配额等
        
        if request.message.is_empty() {
            return Err(AuthError::InvalidRequest);
        }

        Ok(())
    }

    pub async fn logout(&self, session_id: &str) -> Result<(), AuthError> {
        let mut sessions = self.sessions.write();
        if sessions.remove(session_id).is_some() {
            counter!("auth_logout", 1);
            info!("User logged out from session: {}", session_id);
        }
        Ok(())
    }

    pub fn get_stats(&self) -> AuthStats {
        let sessions = self.sessions.read();
        let active_sessions = sessions.values().filter(|s| s.is_active).count();
        
        AuthStats {
            total_sessions: sessions.len(),
            active_sessions,
            max_sessions: self.config.max_attempts * 10, // 估算最大会话数
        }
    }

    fn verify_password(&self, input_password: &str, stored_password: &str) -> bool {
        // 在实际应用中，应该使用 bcrypt 或其他安全的哈希算法
        // 这里使用简单的 SHA256 作为示例
        let mut hasher = Sha256::new();
        hasher.update(input_password.as_bytes());
        let result = format!("{:x}", hasher.finalize());
        
        result == stored_password || input_password == stored_password
    }

    fn create_jwt_token(&self, user_id: &str, session_id: &str) -> Result<String, AuthError> {
        let now = chrono::Utc::now();
        let expires_at = now + chrono::Duration::seconds(self.config.session_timeout as i64);
        
        let claims = Claims {
            sub: user_id.to_string(),
            exp: expires_at.timestamp() as usize,
            iat: now.timestamp() as usize,
            session_id: session_id.to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref())
        ).map_err(|_| AuthError::TokenCreationFailed)
    }

    fn decode_jwt_token(&self, token: &str) -> Result<Claims, AuthError> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_ref()),
            &Validation::default()
        ).map_err(|_| AuthError::InvalidToken)?;

        Ok(token_data.claims)
    }

    async fn create_session(&self, session_id: &str, user_id: &str) {
        let mut sessions = self.sessions.write();
        let session_info = SessionInfo {
            user_id: user_id.to_string(),
            created_at: Instant::now(),
            last_activity: Instant::now(),
            is_active: true,
        };
        
        sessions.insert(session_id.to_string(), session_info);
        gauge!("auth_sessions_active", sessions.len() as f64);
    }

    async fn is_session_valid(&self, session_id: &str) -> bool {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(session_id) {
            if !session.is_active {
                return false;
            }
            
            let timeout = Duration::from_secs(self.config.session_timeout);
            if session.last_activity.elapsed() > timeout {
                return false;
            }
            
            true
        } else {
            false
        }
    }

    async fn update_session_activity(&self, session_id: &str) {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_activity = Instant::now();
        }
    }

    async fn is_rate_limited(&self, username: &str) -> bool {
        let rate_limits = self.rate_limit.read();
        if let Some(info) = rate_limits.get(username) {
            if let Some(locked_until) = info.locked_until {
                if Instant::now() < locked_until {
                    return true;
                }
            }
        }
        false
    }

    async fn increment_rate_limit(&self, username: &str) {
        let mut rate_limits = self.rate_limit.write();
        let info = rate_limits.entry(username.to_string()).or_insert(RateLimitInfo {
            attempts: 0,
            last_attempt: Instant::now(),
            locked_until: None,
        });

        info.attempts += 1;
        info.last_attempt = Instant::now();

        if info.attempts >= self.config.max_attempts {
            info.locked_until = Some(Instant::now() + Duration::from_secs(self.config.lockout_duration));
            warn!("User {} locked due to too many failed attempts", username);
        }
    }

    async fn reset_rate_limit(&self, username: &str) {
        let mut rate_limits = self.rate_limit.write();
        rate_limits.remove(username);
    }

    async fn cleanup_sessions(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5分钟清理一次
        
        loop {
            interval.tick().await;
            
            let mut sessions = self.sessions.write();
            let now = Instant::now();
            let timeout = Duration::from_secs(self.config.session_timeout);
            
            let expired: Vec<String> = sessions
                .iter()
                .filter(|(_, session)| {
                    !session.is_active || now.duration_since(session.last_activity) > timeout
                })
                .map(|(id, _)| id.clone())
                .collect();

            for session_id in expired {
                sessions.remove(&session_id);
                counter!("auth_sessions_expired", 1);
            }

            if !expired.is_empty() {
                info!("Cleaned up {} expired sessions", expired.len());
                gauge!("auth_sessions_active", sessions.len() as f64);
            }
        }
    }
}

impl Clone for AuthManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            sessions: self.sessions.clone(),
            rate_limit: self.rate_limit.clone(),
            jwt_secret: self.jwt_secret.clone(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Invalid token")]
    InvalidToken,
    #[error("Token creation failed")]
    TokenCreationFailed,
    #[error("Invalid session")]
    InvalidSession,
    #[error("Rate limited")]
    RateLimited,
    #[error("Invalid request")]
    InvalidRequest,
    #[error("Session expired")]
    SessionExpired,
}

#[derive(Debug, Serialize)]
pub struct AuthResult {
    pub token: String,
    pub session_id: String,
    pub expires_in: u64,
}

#[derive(Debug, Serialize)]
pub struct TokenInfo {
    pub user_id: String,
    pub session_id: String,
    pub expires_at: usize,
}

#[derive(Debug, Serialize)]
pub struct AuthStats {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub max_sessions: usize,
}

impl AuthStats {
    pub fn session_utilization(&self) -> f64 {
        if self.max_sessions == 0 {
            0.0
        } else {
            self.active_sessions as f64 / self.max_sessions as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_authentication_success() {
        let config = AuthConfig {
            enabled: true,
            username: "test".to_string(),
            password: "test123".to_string(),
            session_timeout: 3600,
            max_attempts: 5,
            lockout_duration: 300,
        };

        let auth_manager = AuthManager::new(&config);
        let result = auth_manager.authenticate("test", "test123").await;
        
        assert!(result.is_ok());
        let auth_result = result.unwrap();
        assert!(!auth_result.token.is_empty());
        assert!(!auth_result.session_id.is_empty());
    }

    #[tokio::test]
    async fn test_authentication_failure() {
        let config = AuthConfig {
            enabled: true,
            username: "test".to_string(),
            password: "test123".to_string(),
            session_timeout: 3600,
            max_attempts: 5,
            lockout_duration: 300,
        };

        let auth_manager = AuthManager::new(&config);
        let result = auth_manager.authenticate("test", "wrong_password").await;
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::InvalidCredentials));
    }

    #[tokio::test]
    async fn test_token_verification() {
        let config = AuthConfig {
            enabled: true,
            username: "test".to_string(),
            password: "test123".to_string(),
            session_timeout: 3600,
            max_attempts: 5,
            lockout_duration: 300,
        };

        let auth_manager = AuthManager::new(&config);
        let auth_result = auth_manager.authenticate("test", "test123").await.unwrap();
        
        let token_info = auth_manager.verify_token(&auth_result.token).await.unwrap();
        assert_eq!(token_info.user_id, "test");
        assert_eq!(token_info.session_id, auth_result.session_id);
    }
}