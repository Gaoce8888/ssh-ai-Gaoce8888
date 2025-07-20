use std::sync::Arc;
use tokio::sync::RwLock;
use dashmap::DashMap;
use lru::LruCache;
use parking_lot::Mutex;
use std::time::{Duration, Instant};
use tracing::{info, debug};

use crate::ssh::SSHSession;

pub struct ConnectionPool {
    connections: DashMap<String, PooledConnection>,
    max_size: usize,
    max_idle_time: Duration,
    lru: Arc<Mutex<LruCache<String, ()>>>,
}

struct PooledConnection {
    session: Arc<RwLock<SSHSession>>,
    last_used: Instant,
    usage_count: usize,
}

impl ConnectionPool {
    pub fn new(max_size: usize) -> Self {
        Self {
            connections: DashMap::new(),
            max_size,
            max_idle_time: Duration::from_secs(300), // 5分钟空闲超时
            lru: Arc::new(Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(max_size).unwrap()
            ))),
        }
    }

    pub async fn get(&self, key: &str) -> Option<Arc<RwLock<SSHSession>>> {
        // 更新LRU
        {
            let mut lru = self.lru.lock();
            lru.get(key);
        }

        if let Some(mut entry) = self.connections.get_mut(key) {
            let now = Instant::now();
            
            // 检查连接是否过期
            if now.duration_since(entry.last_used) > self.max_idle_time {
                drop(entry);
                self.connections.remove(key);
                self.lru.lock().pop(key);
                debug!("Removed expired connection: {}", key);
                return None;
            }

            // 检查连接是否仍然有效
            let session = entry.session.clone();
            let is_alive = {
                let ssh = session.read().await;
                ssh.is_alive()
            };

            if !is_alive {
                drop(entry);
                self.connections.remove(key);
                self.lru.lock().pop(key);
                debug!("Removed dead connection: {}", key);
                return None;
            }

            // 更新使用信息
            entry.last_used = now;
            entry.usage_count += 1;
            
            info!("Reusing connection from pool: {} (used {} times)", key, entry.usage_count);
            Some(session)
        } else {
            None
        }
    }

    pub async fn insert(&self, key: String, session: Arc<RwLock<SSHSession>>) {
        // 检查容量
        if self.connections.len() >= self.max_size {
            // 使用LRU策略移除最少使用的连接
            if let Some((lru_key, _)) = self.lru.lock().pop_lru() {
                self.connections.remove(&lru_key);
                debug!("Evicted connection from pool: {}", lru_key);
            }
        }

        let pooled = PooledConnection {
            session,
            last_used: Instant::now(),
            usage_count: 1,
        };

        self.connections.insert(key.clone(), pooled);
        self.lru.lock().put(key.clone(), ());
        
        info!("Added connection to pool: {} (pool size: {})", key, self.connections.len());
    }

    pub async fn cleanup(&self) {
        let now = Instant::now();
        let mut expired_keys = Vec::new();

        // 收集过期的连接
        for entry in self.connections.iter() {
            if now.duration_since(entry.last_used) > self.max_idle_time {
                expired_keys.push(entry.key().clone());
            }
        }

        // 移除过期连接
        for key in expired_keys {
            self.connections.remove(&key);
            self.lru.lock().pop(&key);
            debug!("Cleaned up expired connection: {}", key);
        }

        info!("Connection pool cleanup complete. Current size: {}", self.connections.len());
    }

    pub fn stats(&self) -> PoolStats {
        let mut total_usage = 0;
        let mut max_usage = 0;
        let mut min_usage = usize::MAX;

        for entry in self.connections.iter() {
            total_usage += entry.usage_count;
            max_usage = max_usage.max(entry.usage_count);
            min_usage = min_usage.min(entry.usage_count);
        }

        let size = self.connections.len();
        
        PoolStats {
            size,
            avg_usage: if size > 0 { total_usage / size } else { 0 },
            max_usage,
            min_usage: if min_usage == usize::MAX { 0 } else { min_usage },
        }
    }
}

pub struct PoolStats {
    pub size: usize,
    pub avg_usage: usize,
    pub max_usage: usize,
    pub min_usage: usize,
}

// 后台清理任务
pub async fn start_cleanup_task(pool: Arc<ConnectionPool>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    
    loop {
        interval.tick().await;
        pool.cleanup().await;
    }
}
