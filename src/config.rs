use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub cache: CacheConfig,
    pub ai: AIConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub address: String,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
    pub max_size: u64,
    pub sync_interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub capacity: usize,
    pub ttl: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    pub providers: Vec<String>,
    pub timeout: u64,
    pub retry_count: u8,
}

#[derive(Debug)]
pub struct ConfigManager {
    config: Arc<RwLock<Config>>,
    #[allow(dead_code)] // 企业级项目中需要跟踪配置文件路径
    path: String,
}

impl ConfigManager {
    pub async fn new(path: &str) -> Result<Self> {
        let config = Self::load_config(path)?;
        Ok(ConfigManager {
            config: Arc::new(RwLock::new(config)),
            path: path.to_string(),
        })
    }

    fn load_config(path: &str) -> Result<Config> {
        let path = Path::new(path);
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let config: Config = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            // 使用默认配置
            Ok(Config {
                server: ServerConfig {
                    port: 8080,
                    address: "127.0.0.1".to_string(),
                    log_level: "info".to_string(),
                },
                database: DatabaseConfig {
                    path: "data/db".to_string(),
                    max_size: 1024 * 1024 * 1024, // 1GB
                    sync_interval: 60, // 60 seconds
                },
                cache: CacheConfig {
                    capacity: 10000,
                    ttl: 3600, // 1 hour
                },
                ai: AIConfig {
                    providers: vec!["openai".to_string(), "anthropic".to_string()],
                    timeout: 30, // 30 seconds
                    retry_count: 3,
                },
            })
        }
    }

    pub async fn get(&self) -> Result<Config> {
        Ok(self.config.read().await.clone())
    }

    #[allow(dead_code)] // 企业级项目中的动态配置更新功能
    pub async fn update(&self, new_config: Config) -> Result<()> {
        let mut config = self.config.write().await;
        *config = new_config.clone();
        
        // 保存到文件
        let json = serde_json::to_string_pretty(&new_config)?;
        std::fs::write(&self.path, json)?;
        
        Ok(())
    }

    #[allow(dead_code)] // 企业级项目中的配置重载功能
    pub async fn reload(&self) -> Result<()> {
        let config = Self::load_config(&self.path)?;
        let mut current = self.config.write().await;
        *current = config;
        Ok(())
    }
}
