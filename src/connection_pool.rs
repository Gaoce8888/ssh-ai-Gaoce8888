use std::sync::Arc;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tracing::{info, warn, debug};
use thiserror::Error;
use ssh2::Session;

#[derive(Error, Debug)]
pub enum PoolError {
    #[error("连接池已满")]
    PoolFull,
    #[error("无可用连接")]
    NoConnectionAvailable,
    #[error("连接创建失败: {0}")]
    ConnectionCreationFailed(String),
    #[error("连接已失效")]
    ConnectionExpired,
}

/// SSH连接包装器
pub struct PooledConnection {
    pub session: Session,
    pub created_at: Instant,
    pub last_used: Instant,
    pub use_count: u64,
    pub host: String,
    pub port: u16,
}

impl PooledConnection {
    pub fn new(session: Session, host: String, port: u16) -> Self {
        Self {
            session,
            created_at: Instant::now(),
            last_used: Instant::now(),
            use_count: 0,
            host,
            port,
        }
    }

    pub fn is_expired(&self, max_age: Duration) -> bool {
        self.created_at.elapsed() > max_age
    }

    pub fn is_idle_too_long(&self, max_idle: Duration) -> bool {
        self.last_used.elapsed() > max_idle
    }

    pub fn mark_used(&mut self) {
        self.last_used = Instant::now();
        self.use_count += 1;
    }
}

/// SSH连接池配置
#[derive(Clone, Debug)]
pub struct PoolConfig {
    pub max_size: usize,
    pub min_idle: usize,
    pub max_idle_time: Duration,
    pub max_lifetime: Duration,
    pub connection_timeout: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 50,
            min_idle: 5,
            max_idle_time: Duration::from_secs(300), // 5分钟
            max_lifetime: Duration::from_secs(3600), // 1小时
            connection_timeout: Duration::from_secs(10),
        }
    }
}

/// SSH连接池
pub struct ConnectionPool {
    connections: Arc<Mutex<VecDeque<PooledConnection>>>,
    semaphore: Arc<Semaphore>,
    config: PoolConfig,
    stats: Arc<Mutex<PoolStats>>,
}

#[derive(Debug, Default)]
pub struct PoolStats {
    pub total_created: u64,
    pub total_destroyed: u64,
    pub total_checkouts: u64,
    pub total_returns: u64,
    pub current_size: usize,
    pub current_idle: usize,
}

impl ConnectionPool {
    pub fn new(config: PoolConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_size));
        
        Self {
            connections: Arc::new(Mutex::new(VecDeque::new())),
            semaphore,
            config,
            stats: Arc::new(Mutex::new(PoolStats::default())),
        }
    }

    /// 获取连接
    pub async fn get_connection(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<PooledConnection, PoolError> {
        // 首先尝试从池中获取现有连接
        let mut connections = self.connections.lock().await;
        let mut stats = self.stats.lock().await;
        
        // 查找匹配的空闲连接
        let mut index = None;
        for (i, conn) in connections.iter().enumerate() {
            if conn.host == host && conn.port == port {
                if !conn.is_expired(self.config.max_lifetime) 
                    && !conn.is_idle_too_long(self.config.max_idle_time) {
                    index = Some(i);
                    break;
                }
            }
        }
        
        if let Some(i) = index {
            let mut conn = connections.remove(i).unwrap();
            conn.mark_used();
            stats.total_checkouts += 1;
            stats.current_idle = connections.len();
            debug!("从连接池获取连接: {}:{}", host, port);
            return Ok(conn);
        }
        
        drop(connections);
        drop(stats);
        
        // 如果没有可用连接，创建新连接
        let _permit = self.semaphore
            .acquire()
            .await
            .map_err(|_| PoolError::PoolFull)?;
        
        self.create_connection(host, port, username, password).await
    }

    /// 归还连接
    pub async fn return_connection(&self, mut conn: PooledConnection) {
        let mut connections = self.connections.lock().await;
        let mut stats = self.stats.lock().await;
        
        // 检查连接是否仍然有效
        if !conn.is_expired(self.config.max_lifetime) {
            conn.last_used = Instant::now();
            connections.push_back(conn);
            stats.total_returns += 1;
            stats.current_idle = connections.len();
            debug!("连接归还到池中");
        } else {
            stats.total_destroyed += 1;
            warn!("连接已过期，丢弃");
        }
        
        // 清理过期连接
        self.cleanup_expired(&mut connections, &mut stats);
    }

    /// 创建新连接
    async fn create_connection(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<PooledConnection, PoolError> {
        info!("创建新的SSH连接: {}:{}", host, port);
        
        // 使用tokio的异步任务来处理阻塞的SSH连接
        let host_clone = host.to_string();
        let username_clone = username.to_string();
        let password_clone = password.to_string();
        
        let session = tokio::task::spawn_blocking(move || {
            let tcp = std::net::TcpStream::connect_timeout(
                &format!("{}:{}", host_clone, port).parse().unwrap(),
                Duration::from_secs(10)
            ).map_err(|e| PoolError::ConnectionCreationFailed(e.to_string()))?;
            
            let mut sess = Session::new()
                .map_err(|e| PoolError::ConnectionCreationFailed(e.to_string()))?;
            
            sess.set_tcp_stream(tcp);
            sess.handshake()
                .map_err(|e| PoolError::ConnectionCreationFailed(e.to_string()))?;
            
            sess.userauth_password(&username_clone, &password_clone)
                .map_err(|e| PoolError::ConnectionCreationFailed(e.to_string()))?;
            
            Ok::<Session, PoolError>(sess)
        })
        .await
        .map_err(|e| PoolError::ConnectionCreationFailed(e.to_string()))??;
        
        let mut stats = self.stats.lock().await;
        stats.total_created += 1;
        stats.current_size += 1;
        
        Ok(PooledConnection::new(session, host.to_string(), port))
    }

    /// 清理过期连接
    fn cleanup_expired(&self, connections: &mut VecDeque<PooledConnection>, stats: &mut PoolStats) {
        let before_count = connections.len();
        
        connections.retain(|conn| {
            let should_keep = !conn.is_expired(self.config.max_lifetime) 
                && !conn.is_idle_too_long(self.config.max_idle_time);
            if !should_keep {
                stats.total_destroyed += 1;
                stats.current_size -= 1;
            }
            should_keep
        });
        
        let removed = before_count - connections.len();
        if removed > 0 {
            debug!("清理了 {} 个过期连接", removed);
        }
        
        stats.current_idle = connections.len();
    }

    /// 获取连接池统计信息
    pub async fn get_stats(&self) -> PoolStats {
        let stats = self.stats.lock().await;
        stats.clone()
    }

    /// 清空连接池
    pub async fn clear(&self) {
        let mut connections = self.connections.lock().await;
        let mut stats = self.stats.lock().await;
        
        let count = connections.len();
        connections.clear();
        
        stats.total_destroyed += count as u64;
        stats.current_size = 0;
        stats.current_idle = 0;
        
        info!("清空连接池，关闭了 {} 个连接", count);
    }

    /// 预热连接池
    pub async fn warmup(&self, host: &str, port: u16, username: &str, password: &str) {
        let warmup_count = self.config.min_idle.min(self.config.max_size);
        info!("预热连接池，创建 {} 个连接", warmup_count);
        
        for _ in 0..warmup_count {
            match self.create_connection(host, port, username, password).await {
                Ok(conn) => {
                    self.return_connection(conn).await;
                }
                Err(e) => {
                    warn!("预热连接失败: {}", e);
                    break;
                }
            }
        }
    }
}

impl Clone for PoolStats {
    fn clone(&self) -> Self {
        Self {
            total_created: self.total_created,
            total_destroyed: self.total_destroyed,
            total_checkouts: self.total_checkouts,
            total_returns: self.total_returns,
            current_size: self.current_size,
            current_idle: self.current_idle,
        }
    }
}