use async_ssh2_tokio::{client::{Client, Channel}, Session};
use bytes::Bytes;
use flume;
use parking_lot::RwLock;
use std::sync::Arc;
use dashmap::DashMap;
use std::time::{Duration, Instant};

const BUFFER_SIZE: usize = 128 * 1024; // 128KB缓冲区
const POOL_CLEANUP_INTERVAL: Duration = Duration::from_secs(60);
const MAX_IDLE_TIME: Duration = Duration::from_secs(300);

pub struct SSHSession {
    client: Client,
    channel: Channel,
    last_activity: Instant,
}

impl SSHSession {
    pub async fn new_async(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> anyhow::Result<(Self, flume::Receiver<Bytes>)> {
        // 建立连接
        let mut client = Client::connect(
            (host, port),
            username,
            async_ssh2_tokio::client::AuthMethod::Password(password.to_string()),
        ).await?;
        
        // 创建shell通道
        let channel = client.get_channel().await?;
        channel.request_pty(
            "xterm-256color",
            80,
            24,
            640,
            480,
            None
        ).await?;
        channel.request_shell().await?;
        
        // 创建数据通道
        let (tx, rx) = flume::bounded(256);
        
        // 启动读取任务
        let mut channel_clone = channel.clone();
        tokio::spawn(async move {
            let mut buffer = vec![0u8; BUFFER_SIZE];
            
            loop {
                match channel_clone.read(&mut buffer).await {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = Bytes::copy_from_slice(&buffer[..n]);
                        if tx.send_async(data).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        
        Ok((
            Self {
                client,
                channel,
                last_activity: Instant::now(),
            },
            rx,
        ))
    }
    
    pub fn write_bytes(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.last_activity = Instant::now();
        
        // 异步写入
        let mut channel = self.channel.clone();
        let data = data.to_vec();
        
        tokio::spawn(async move {
            let _ = channel.write_all(&data).await;
            let _ = channel.flush().await;
        });
        
        Ok(())
    }
    
    pub fn is_alive(&self) -> bool {
        !self.channel.is_closed()
    }
    
    pub fn last_activity(&self) -> Instant {
        self.last_activity
    }
}

// 高性能连接池
pub struct ConnectionPool {
    connections: Arc<DashMap<String, PoolEntry>>,
    max_size: usize,
}

struct PoolEntry {
    connection: Arc<RwLock<SSHSession>>,
    created_at: Instant,
    usage_count: usize,
}

impl ConnectionPool {
    pub fn new(max_size: usize) -> Self {
        let pool = Self {
            connections: Arc::new(DashMap::with_capacity_and_hasher(
                max_size,
                ahash::RandomState::new(),
            )),
            max_size,
        };
        
        // 启动清理任务
        let connections = pool.connections.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(POOL_CLEANUP_INTERVAL);
            loop {
                interval.tick().await;
                Self::cleanup_expired(&connections).await;
            }
        });
        
        pool
    }
    
    pub async fn get(&self, key: &str) -> Option<Arc<RwLock<SSHSession>>> {
        if let Some(mut entry) = self.connections.get_mut(key) {
            let is_alive = {
                let session = entry.connection.read();
                session.is_alive() && 
                session.last_activity().elapsed() < MAX_IDLE_TIME
            };
            
            if is_alive {
                entry.usage_count += 1;
                Some(entry.connection.clone())
            } else {
                drop(entry);
                self.connections.remove(key);
                None
            }
        } else {
            None
        }
    }
    
    pub async fn insert(&self, key: String, connection: Arc<RwLock<SSHSession>>) {
        // LRU驱逐
        if self.connections.len() >= self.max_size {
            self.evict_lru();
        }
        
        let entry = PoolEntry {
            connection,
            created_at: Instant::now(),
            usage_count: 1,
        };
        
        self.connections.insert(key, entry);
    }
    
    pub fn active_connections(&self) -> usize {
        self.connections.len()
    }
    
    pub async fn cleanup(&self) {
        Self::cleanup_expired(&self.connections).await;
    }
    
    async fn cleanup_expired(connections: &DashMap<String, PoolEntry>) {
        let mut expired = Vec::new();
        
        for entry in connections.iter() {
            let should_remove = {
                let session = entry.connection.read();
                !session.is_alive() || session.last_activity().elapsed() > MAX_IDLE_TIME
            };
            
            if should_remove {
                expired.push(entry.key().clone());
            }
        }
        
        for key in expired {
            connections.remove(&key);
        }
    }
    
    fn evict_lru(&self) {
        if let Some(oldest) = self.connections
            .iter()
            .min_by_key(|entry| entry.created_at)
            .map(|entry| entry.key().clone())
        {
            self.connections.remove(&oldest);
        }
    }
}
