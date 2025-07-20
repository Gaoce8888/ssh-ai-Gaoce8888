use ssh2::Session;
use std::io::{Read, Write};
use tokio::sync::mpsc;
use tracing::{info, error, warn};
use thiserror::Error;
use std::sync::Arc;
use std::time::Duration;

#[derive(Error, Debug)]
pub enum SSHError {
    #[error("连接失败: {0}")]
    ConnectionFailed(String),
    #[error("认证失败: 用户名或密码错误")]
    AuthenticationFailed,
    #[error("通道创建失败")]
    ChannelCreationFailed,
    #[error("网络超时: 无法连接到 {host}:{port}")]
    NetworkTimeout { host: String, port: u16 },
    #[error("握手失败: SSH协议握手失败")]
    HandshakeFailed,
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),
}

pub struct SSHSession {
    #[allow(dead_code)] // 企业级项目中可能需要直接访问session
    session: Session,
    channel: ssh2::Channel,
}

impl SSHSession {
    pub async fn new(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<(Arc<tokio::sync::Mutex<Self>>, mpsc::Receiver<String>), SSHError> {
        info!("尝试连接到 SSH 服务器: {}:{}", host, port);
        
        // 尝试TCP连接，设置超时
        let tcp = match std::net::TcpStream::connect_timeout(
            &format!("{}:{}", host, port).parse().unwrap(),
            Duration::from_secs(10)
        ) {
            Ok(stream) => {
                info!("TCP连接成功建立到 {}:{}", host, port);
                stream
            }
            Err(e) => {
                error!("TCP连接失败到 {}:{} - {}", host, port, e);
                return Err(SSHError::NetworkTimeout { 
                    host: host.to_string(), 
                    port 
                });
            }
        };
        
        // 设置TCP流为非阻塞模式
        tcp.set_nonblocking(false)
            .map_err(|e| SSHError::ConnectionFailed(format!("设置TCP流失败: {}", e)))?;
        
        let mut session = Session::new()
            .map_err(|e| SSHError::ConnectionFailed(format!("创建SSH会话失败: {}", e)))?;
        
        session.set_tcp_stream(tcp);
        
        info!("开始SSH握手...");
        session.handshake()
            .map_err(|e| {
                error!("SSH握手失败: {}", e);
                SSHError::HandshakeFailed
            })?;
        
        info!("SSH握手成功，开始认证用户: {}", username);
        session.userauth_password(username, password)
            .map_err(|e| {
                error!("SSH认证失败: {}", e);
                SSHError::AuthenticationFailed
            })?;
        
        if !session.authenticated() {
            warn!("SSH认证检查失败");
            return Err(SSHError::AuthenticationFailed);
        }
        
        info!("SSH认证成功");
        let mut channel = session.channel_session()
            .map_err(|e| {
                error!("创建SSH通道失败: {}", e);
                SSHError::ChannelCreationFailed
            })?;
        
        channel.request_pty("xterm", None, None)
            .map_err(|e| {
                error!("请求PTY失败: {}", e);
                SSHError::ChannelCreationFailed
            })?;
        
        channel.shell()
            .map_err(|e| {
                error!("启动Shell失败: {}", e);
                SSHError::ChannelCreationFailed
            })?;
        
        session.set_blocking(false);
        
        let (tx, rx) = mpsc::channel(100);
        
        let ssh_session = Arc::new(tokio::sync::Mutex::new(SSHSession {
            session,
            channel,
        }));
        
        let ssh_clone = ssh_session.clone();
        tokio::spawn(async move {
            let mut buffer = [0u8; 4096];
            loop {
                let data = {
                    let mut ssh = ssh_clone.lock().await;
                    match ssh.channel.read(&mut buffer) {
                        Ok(0) => {
                            info!("SSH通道已关闭");
                            break;
                        }
                        Ok(n) => Some(String::from_utf8_lossy(&buffer[..n]).to_string()),
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => None,
                        Err(e) => {
                            error!("SSH读取错误: {}", e);
                            break;
                        }
                    }
                };
                
                if let Some(data) = data {
                    if tx.send(data).await.is_err() {
                        break;
                    }
                }
                
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        });
        
        info!("SSH会话创建完成");
        Ok((ssh_session, rx))
    }

    pub fn write(&mut self, data: &str) -> Result<(), SSHError> {
        self.channel.write_all(data.as_bytes())?;
        self.channel.flush()?;
        Ok(())
    }

    #[allow(dead_code)] // 企业级项目中的命令执行功能
    pub fn execute_command(&mut self, command: &str) -> Result<String, SSHError> {
        let mut channel = self.session.channel_session()
            .map_err(|_| SSHError::ChannelCreationFailed)?;
        
        channel.exec(command)
            .map_err(|_| SSHError::ChannelCreationFailed)?;
        
        let mut output = String::new();
        channel.read_to_string(&mut output)?;
        
        Ok(output)
    }
}
