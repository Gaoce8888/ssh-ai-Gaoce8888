use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // 服务器配置
    pub port: u16,
    pub host: String,
    pub workers: Option<usize>,
    
    // TLS配置
    pub tls_cert: String,
    pub tls_key: String,
    
    // 连接池配置
    pub pool_size: usize,
    pub pool_idle_timeout: u64,
    
    // 缓存配置
    pub cache_size: usize,
    pub cache_ttl: u64,
    
    // AI配置
    pub ai_providers: Vec<AIProviderConfig>,
    
    // 性能配置
    pub max_concurrent_connections: usize,
    pub websocket_buffer_size: usize,
    pub ssh_buffer_size: usize,
    
    // 监控配置
    pub metrics_enabled: bool,
    pub metrics_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIProviderConfig {
    pub name: String,
    pub api_key: String,
    pub endpoint: String,
    pub model: String,
    pub max_tokens: usize,
    pub timeout: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "0.0.0.0".to_string(),
            workers: None, // 自动检测
            
            tls_cert: "cert.pem".to_string(),
            tls_key: "key.pem".to_string(),
            
            pool_size: 100,
            pool_idle_timeout: 300,
            
            cache_size: 1000,
            cache_ttl: 300,
            
            ai_providers: vec![],
            
            max_concurrent_connections: 1000,
            websocket_buffer_size: 256,
            ssh_buffer_size: 65536,
            
            metrics_enabled: true,
            metrics_port: 9090,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        // 尝试从环境变量加载
        if let Ok(config_path) = std::env::var("CONFIG_PATH") {
            return Self::from_file(&config_path);
        }
        
        // 尝试从默认位置加载
        let paths = vec![
            "config.toml",
            "/etc/ssh-ai-terminal/config.toml",
            "/opt/ssh-ai-terminal/config.toml",
        ];
        
        for path in paths {
            if Path::new(path).exists() {
                return Self::from_file(path);
            }
        }
        
        // 使用默认配置
        Ok(Self::default())
    }
    
    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
    
    pub fn save(&self, path: &str) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

// 运行时配置热重载
pub struct ConfigWatcher {
    config: Arc<parking_lot::RwLock<Config>>,
    path: String,
}

impl ConfigWatcher {
    pub fn new(path: String) -> Result<Self> {
        let config = Config::from_file(&path)?;
        Ok(Self {
            config: Arc::new(parking_lot::RwLock::new(config)),
            path,
        })
    }
    
    pub fn get(&self) -> Config {
        self.config.read().clone()
    }
    
    pub async fn watch(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        
        loop {
            interval.tick().await;
            
            if let Ok(new_config) = Config::from_file(&self.path) {
                *self.config.write() = new_config;
                tracing::info!("Configuration reloaded");
            }
        }
    }
}
