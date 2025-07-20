use async_ssh2_tokio::{Session, Channel};
use tokio::sync::mpsc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use anyhow::{Result, Context};
use bytes::{Bytes, BytesMut};
use tracing::{info, error, debug, instrument};

const BUFFER_SIZE: usize = 65536; // 64KB buffer
const CHANNEL_BUFFER_SIZE: usize = 1024;

pub struct SSHSession {
    session: Arc<Session>,
    channel: Channel,
    tx: mpsc::Sender<Bytes>,
}

impl SSHSession {
    #[instrument(skip(password))]
    pub async fn new_optimized(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<(Self, mpsc::Receiver<Bytes>)> {
        // 创建TCP连接
        let tcp = tokio::net::TcpStream::connect((host, port))
            .await
            .context("Failed to connect to SSH server")?;
        
        // 优化TCP设置
        tcp.set_nodelay(true)?;
        let _ = tcp.set_recv_buffer_size(BUFFER_SIZE);
        let _ = tcp.set_send_buffer_size(BUFFER_SIZE);
        
        // 创建SSH会话
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.set_blocking(false);
        session.set_compress(true); // 启用压缩
        
        session.handshake().await
            .context("SSH handshake failed")?;
        
        // 认证
        session.userauth_password(username, password).await
            .context("Authentication failed")?;
        
        // 创建通道
        let mut channel = session.channel_session().await
            .context("Failed to create channel")?;
        
        // 请求PTY
        channel.request_pty(
            "xterm-256color",
            None,
            Some((80, 24, 640, 480))
        ).await?;
        
        channel.shell().await?;
        
        // 创建高性能数据通道
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let session = Arc::new(session);
        
        // 启动读取任务
        let tx_clone = tx.clone();
        let mut channel_clone = channel.clone();
        tokio::spawn(async move {
            let mut buffer = BytesMut::with_capacity(BUFFER_SIZE);
            buffer.resize(BUFFER_SIZE, 0);
            
            loop {
                match channel_clone.read(&mut buffer).await {
                    Ok(0) => {
                        debug!("SSH channel closed");
                        break;
                    }
                    Ok(n) => {
                        let data = Bytes::copy_from_slice(&buffer[..n]);
                        if tx_clone.send(data).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("SSH read error: {}", e);
                        break;
                    }
                }
            }
        });
        
        Ok((
            Self {
                session,
                channel,
                tx,
            },
            rx,
        ))
    }
    
    #[instrument(skip(self, data))]
    pub fn write_bytes(&mut self, data: &[u8]) -> Result<()> {
        // 使用异步写入
        let mut channel = self.channel.clone();
        let data = data.to_vec();
        
        tokio::spawn(async move {
            if let Err(e) = channel.write_all(&data).await {
                error!("Failed to write to SSH channel: {}", e);
            }
        });
        
        Ok(())
    }
    
    pub fn write(&mut self, data: &str) -> Result<()> {
        self.write_bytes(data.as_bytes())
    }
    
    pub fn is_alive(&self) -> bool {
        !self.channel.eof() && self.session.authenticated()
    }
    
    pub async fn close(&mut self) -> Result<()> {
        self.channel.close().await?;
        self.channel.wait_close().await?;
        Ok(())
    }
}

// SSH连接池键生成
pub fn generate_pool_key(host: &str, port: u16, username: &str) -> String {
    format!("{}@{}:{}", username, host, port)
}

// SSH会话管理器
pub struct SSHManager {
    sessions: dashmap::DashMap<String, Arc<tokio::sync::RwLock<SSHSession>>>,
    max_sessions: usize,
}

impl SSHManager {
    pub fn new(max_sessions: usize) -> Self {
        Self {
            sessions: dashmap::DashMap::new(),
            max_sessions,
        }
    }
    
    pub async fn get_or_create_session(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<Arc<tokio::sync::RwLock<SSHSession>>> {
        let key = generate_pool_key(host, port, username);
        
        // 检查现有会话
        if let Some(session) = self.sessions.get(&key) {
            let is_alive = {
                let ssh = session.read().await;
                ssh.is_alive()
            };
            
            if is_alive {
                info!("Reusing existing SSH session for {}", key);
                return Ok(session.clone());
            } else {
                // 移除死亡的会话
                drop(session);
                self.sessions.remove(&key);
            }
        }
        
        // 检查会话限制
        if self.sessions.len() >= self.max_sessions {
            // 清理死亡的会话
            self.cleanup_dead_sessions().await;
            
            if self.sessions.len() >= self.max_sessions {
                anyhow::bail!("Maximum number of SSH sessions reached");
            }
        }
        
        // 创建新会话
        info!("Creating new SSH session for {}", key);
        let (session, _rx) = SSHSession::new_optimized(host, port, username, password).await?;
        let session = Arc::new(tokio::sync::RwLock::new(session));
        
        self.sessions.insert(key, session.clone());
        Ok(session)
    }
    
    async fn cleanup_dead_sessions(&self) {
        let mut dead_keys = Vec::new();
        
        for entry in self.sessions.iter() {
            let is_alive = {
                let ssh = entry.value().read().await;
                ssh.is_alive()
            };
            
            if !is_alive {
                dead_keys.push(entry.key().clone());
            }
        }
        
        for key in dead_keys {
            self.sessions.remove(&key);
            debug!("Removed dead SSH session: {}", key);
        }
    }
}
