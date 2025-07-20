// SSH AI Terminal 优化版本模块导出
// 整合所有性能优化模块

pub mod models;
pub mod ssh;
pub mod websocket;
pub mod ai;
pub mod connection_pool;
pub mod cache;
pub mod config;
pub mod performance;

// 重新导出常用类型
pub use models::*;
pub use connection_pool::ConnectionPool;
pub use cache::{ResponseCache, AIResponseCache};
pub use config::{Config, ConfigWatcher};
pub use performance::{PerformanceMonitor, health_check};

// 版本信息
pub const VERSION: &str = "0.2.0-optimized";
pub const BUILD_DATE: &str = env!("BUILD_DATE");

// 初始化函数
pub async fn init_optimized_system(config: Config) -> anyhow::Result<()> {
    use tracing::info;
    
    info!("Initializing SSH AI Terminal v{} (Optimized)", VERSION);
    
    // 初始化性能监控
    performance::init_performance_monitor();
    
    // 初始化AI管理器
    ai::init_ai_manager(&config);
    
    // 设置系统限制
    set_system_limits()?;
    
    info!("System initialization complete");
    Ok(())
}

// 设置系统资源限制
fn set_system_limits() -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use rlimit::{Resource, setrlimit};
        
        // 设置文件描述符限制
        setrlimit(Resource::NOFILE, 65535, 65535)?;
        
        // 设置线程限制
        setrlimit(Resource::NPROC, 4096, 4096)?;
    }
    
    Ok(())
}

// 优雅关闭
pub async fn graceful_shutdown(signal: tokio::sync::oneshot::Receiver<()>) {
    use tracing::info;
    
    signal.await.ok();
    info!("Received shutdown signal, cleaning up...");
    
    // 这里可以添加清理逻辑
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    info!("Shutdown complete");
}

// 性能统计
pub struct SystemStats {
    pub version: String,
    pub uptime: std::time::Duration,
    pub total_connections: u64,
    pub active_connections: u64,
    pub cache_hit_rate: f64,
    pub average_response_time_ms: f64,
}

impl SystemStats {
    pub fn collect() -> Self {
        // 实现统计收集逻辑
        Self {
            version: VERSION.to_string(),
            uptime: std::time::Duration::from_secs(0),
            total_connections: 0,
            active_connections: 0,
            cache_hit_rate: 0.0,
            average_response_time_ms: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_system_init() {
        let config = Config::default();
        assert!(init_optimized_system(config).await.is_ok());
    }
}
