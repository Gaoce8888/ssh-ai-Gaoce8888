// 高性能版本模块整合

pub mod api;
pub mod websocket;
pub mod ssh;
pub mod ai;

// 配置管理模块
pub mod config_manager {
    use serde::{Deserialize, Serialize};
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Config {
        pub server: ServerConfig,
        pub performance: PerformanceConfig,
        pub ai_providers: Vec<AIProviderConfig>,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ServerConfig {
        pub port: u16,
        pub host: String,
        pub workers: usize,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PerformanceConfig {
        pub pool_size: usize,
        pub cache_size: usize,
        pub ai_cache_size: usize,
        pub max_connections: usize,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AIProviderConfig {
        pub name: String,
        pub api_key: String,
        pub model: String,
    }
    
    impl Default for Config {
        fn default() -> Self {
            Self {
                server: ServerConfig {
                    port: 8080,
                    host: "0.0.0.0".to_string(),
                    workers: 0,
                },
                performance: PerformanceConfig {
                    pool_size: 100,
                    cache_size: 10000,
                    ai_cache_size: 1000,
                    max_connections: 1000,
                },
                ai_providers: vec![],
            }
        }
    }
}

// 配置加载
pub mod config {
    use super::config_manager::Config;
    use std::path::Path;
    
    pub fn load_config() -> anyhow::Result<Config> {
        // 尝试从文件加载
        if Path::new("config.toml").exists() {
            let content = std::fs::read_to_string("config.toml")?;
            let config: Config = toml::from_str(&content)?;
            return Ok(config);
        }
        
        // 使用默认配置
        Ok(Config::default())
    }
}

// 必要的依赖
pub use lazy_static;
pub use async_trait;
