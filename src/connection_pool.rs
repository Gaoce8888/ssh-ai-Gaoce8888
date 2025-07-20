use std::sync::Arc;
use tokio::sync::Semaphore;
use parking_lot::Mutex;
use std::collections::HashMap;
use uuid::Uuid;
use tracing::{info, warn, error};
use metrics::{gauge, counter};
use std::time::{Duration, Instant};

pub struct ConnectionPool {
    max_connections: usize,
    semaphore: Arc<Semaphore>,
    connections: Arc<Mutex<HashMap<Uuid, PooledConnection>>>,
    cleanup_interval: Duration,
}

struct PooledConnection {
    session_id: Uuid,
    created_at: Instant,
    last_used: Instant,
    is_active: bool,
}

impl ConnectionPool {
    pub fn new(max_connections: usize) -> Self {
        let pool = Self {
            max_connections,
            semaphore: Arc::new(Semaphore::new(max_connections)),
            connections: Arc::new(Mutex::new(HashMap::new())),
            cleanup_interval: Duration::from_secs(300), // 5分钟清理一次
        };

        // 启动清理任务
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            pool_clone.cleanup_task().await;
        });

        info!("Connection pool initialized with max connections: {}", max_connections);
        pool
    }

    pub async fn acquire(&self, session_id: Uuid) -> Result<ConnectionGuard, PoolError> {
        let permit = self.semaphore.acquire().await.map_err(|_| PoolError::PoolExhausted)?;
        
        let mut connections = self.connections.lock();
        
        // 检查是否已存在连接
        if let Some(conn) = connections.get_mut(&session_id) {
            conn.last_used = Instant::now();
            conn.is_active = true;
            counter!("connection_pool_reuse", 1);
            return Ok(ConnectionGuard {
                session_id,
                _permit: permit,
                pool: self.clone(),
            });
        }

        // 创建新连接
        let pooled_conn = PooledConnection {
            session_id,
            created_at: Instant::now(),
            last_used: Instant::now(),
            is_active: true,
        };

        connections.insert(session_id, pooled_conn);
        gauge!("connection_pool_size", connections.len() as f64);
        counter!("connection_pool_created", 1);

        info!("Created new connection for session: {}", session_id);
        
        Ok(ConnectionGuard {
            session_id,
            _permit: permit,
            pool: self.clone(),
        })
    }

    pub fn release(&self, session_id: Uuid) {
        let mut connections = self.connections.lock();
        if let Some(conn) = connections.get_mut(&session_id) {
            conn.is_active = false;
            conn.last_used = Instant::now();
            counter!("connection_pool_released", 1);
        }
    }

    pub fn remove(&self, session_id: Uuid) {
        let mut connections = self.connections.lock();
        if connections.remove(&session_id).is_some() {
            gauge!("connection_pool_size", connections.len() as f64);
            counter!("connection_pool_removed", 1);
            info!("Removed connection for session: {}", session_id);
        }
    }

    pub fn get_stats(&self) -> PoolStats {
        let connections = self.connections.lock();
        let active_connections = connections.values().filter(|c| c.is_active).count();
        let total_connections = connections.len();
        
        PoolStats {
            total_connections,
            active_connections,
            max_connections: self.max_connections,
            available_permits: self.semaphore.available_permits(),
        }
    }

    async fn cleanup_task(&self) {
        let mut interval = tokio::time::interval(self.cleanup_interval);
        
        loop {
            interval.tick().await;
            self.cleanup_expired_connections().await;
        }
    }

    async fn cleanup_expired_connections(&self) {
        let mut connections = self.connections.lock();
        let now = Instant::now();
        let timeout = Duration::from_secs(1800); // 30分钟超时
        
        let expired: Vec<Uuid> = connections
            .iter()
            .filter(|(_, conn)| {
                !conn.is_active && now.duration_since(conn.last_used) > timeout
            })
            .map(|(id, _)| *id)
            .collect();

        for session_id in expired {
            connections.remove(&session_id);
            counter!("connection_pool_cleanup", 1);
        }

        if !expired.is_empty() {
            info!("Cleaned up {} expired connections", expired.len());
            gauge!("connection_pool_size", connections.len() as f64);
        }
    }
}

impl Clone for ConnectionPool {
    fn clone(&self) -> Self {
        Self {
            max_connections: self.max_connections,
            semaphore: self.semaphore.clone(),
            connections: self.connections.clone(),
            cleanup_interval: self.cleanup_interval,
        }
    }
}

pub struct ConnectionGuard {
    session_id: Uuid,
    _permit: tokio::sync::SemaphorePermit<'static>,
    pool: ConnectionPool,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.pool.release(self.session_id);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("Connection pool exhausted")]
    PoolExhausted,
    #[error("Connection not found")]
    ConnectionNotFound,
    #[error("Connection timeout")]
    Timeout,
}

#[derive(Debug, serde::Serialize)]
pub struct PoolStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub max_connections: usize,
    pub available_permits: usize,
}

impl PoolStats {
    pub fn utilization_rate(&self) -> f64 {
        if self.max_connections == 0 {
            0.0
        } else {
            self.active_connections as f64 / self.max_connections as f64
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.utilization_rate() < 0.9 && self.available_permits > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_connection_pool_basic() {
        let pool = ConnectionPool::new(5);
        
        let session_id = Uuid::new_v4();
        let guard = pool.acquire(session_id).await.unwrap();
        
        let stats = pool.get_stats();
        assert_eq!(stats.total_connections, 1);
        assert_eq!(stats.active_connections, 1);
        
        drop(guard);
        
        let stats = pool.get_stats();
        assert_eq!(stats.active_connections, 0);
    }

    #[tokio::test]
    async fn test_connection_pool_limit() {
        let pool = ConnectionPool::new(2);
        
        let session1 = Uuid::new_v4();
        let session2 = Uuid::new_v4();
        let session3 = Uuid::new_v4();
        
        let guard1 = pool.acquire(session1).await.unwrap();
        let guard2 = pool.acquire(session2).await.unwrap();
        
        // 第三个连接应该失败
        let result = pool.acquire(session3).await;
        assert!(result.is_err());
        
        drop(guard1);
        drop(guard2);
    }

    #[tokio::test]
    async fn test_connection_reuse() {
        let pool = ConnectionPool::new(5);
        let session_id = Uuid::new_v4();
        
        {
            let _guard1 = pool.acquire(session_id).await.unwrap();
        }
        
        {
            let _guard2 = pool.acquire(session_id).await.unwrap();
        }
        
        let stats = pool.get_stats();
        assert_eq!(stats.total_connections, 1); // 应该重用同一个连接
    }
}