// SSH-AI Terminal 库
// 提供核心功能模块的公共接口

use std::sync::Arc;
use dashmap::DashMap;
use uuid::Uuid;

pub mod models;
pub mod ssh;
pub mod websocket;
pub mod ai;
pub mod config;
pub mod performance;
pub mod cache;
pub mod connection_pool;

// 重新导出常用类型
pub use models::*;
pub use config::ConfigManager;

// Sessions 类型定义
pub type Sessions = Arc<DashMap<Uuid, Arc<tokio::sync::Mutex<ssh::SSHSession>>>>;

// 版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");