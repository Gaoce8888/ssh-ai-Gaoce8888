use warp::ws::{WebSocket, Message};
use futures_util::{StreamExt, SinkExt, stream::SplitSink};
use tokio::sync::{mpsc, RwLock, Semaphore};
use std::sync::Arc;
use uuid::Uuid;
use tracing::{info, error, instrument};
use bytes::Bytes;
use parking_lot::Mutex;

use crate::{Sessions, models::*, ssh::SSHSession, connection_pool::ConnectionPool};

// 消息批处理缓冲区
const MESSAGE_BUFFER_SIZE: usize = 256;
const MAX_CONCURRENT_OPERATIONS: usize = 100;

pub async fn client_connection_optimized(
    ws: WebSocket,
    sessions: Sessions,
    pool: Arc<ConnectionPool>,
) {
    let (ws_tx, mut ws_rx) = ws.split();
    let ws_tx = Arc::new(Mutex::new(ws_tx));
    
    // 使用更大的channel缓冲区减少背压
    let (tx, mut rx) = mpsc::channel(MESSAGE_BUFFER_SIZE);
    
    // 限制并发操作
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_OPERATIONS));
    
    // 发送任务 - 批量发送优化
    let ws_tx_clone = ws_tx.clone();
    let send_task = tokio::spawn(async move {
        let mut buffer = Vec::with_capacity(10);
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(10));
        
        loop {
            tokio::select! {
                Some(msg) = rx.recv() => {
                    buffer.push(msg);
                    
                    // 批量发送
                    if buffer.len() >= 10 {
                        send_batch(&ws_tx_clone, &mut buffer).await;
                    }
                }
                _ = interval.tick() => {
                    // 定时刷新缓冲区
                    if !buffer.is_empty() {
                        send_batch(&ws_tx_clone, &mut buffer).await;
                    }
                }
                else => break,
            }
        }
    });

    // 接收任务 - 并行处理
    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(msg) => {
                let sessions = sessions.clone();
                let tx = tx.clone();
                let semaphore = semaphore.clone();
                let pool = pool.clone();
                
                // 异步处理消息，不阻塞接收
                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    if let Ok(text) = msg.to_str() {
                        if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(text) {
                            handle_message_optimized(ws_msg, &sessions, &tx, &pool).await;
                        }
                    }
                });
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
        }
    }

    // 清理
    send_task.abort();
    info!("WebSocket connection closed");
}

async fn send_batch(
    ws_tx: &Arc<Mutex<SplitSink<WebSocket, Message>>>,
    buffer: &mut Vec<String>,
) {
    if buffer.is_empty() {
        return;
    }
    
    // 合并小消息
    let combined = if buffer.len() == 1 {
        buffer.remove(0)
    } else {
        format!("[{}]", buffer.join(","))
    };
    
    let mut tx = ws_tx.lock();
    if let Err(e) = tx.send(Message::text(combined)).await {
        error!("Failed to send batch: {}", e);
    }
    
    buffer.clear();
}

#[instrument(skip(sessions, tx, pool))]
async fn handle_message_optimized(
    msg: WebSocketMessage,
    sessions: &Sessions,
    tx: &mpsc::Sender<String>,
    pool: &ConnectionPool,
) {
    match msg {
        WebSocketMessage::Connect { host, port, username, password } => {
            // 尝试从连接池获取现有连接
            let conn_key = format!("{}:{}@{}", username, host, port);
            
            if let Some(existing_session) = pool.get(&conn_key).await {
                let session_id = Uuid::new_v4();
                sessions.insert(session_id, existing_session);
                
                let response = WebSocketResponse::Connected { session_id };
                let _ = tx.send(serde_json::to_string(&response).unwrap()).await;
                return;
            }
            
            // 创建新连接
            match SSHSession::new_optimized(&host, port, &username, &password).await {
                Ok((ssh_session, mut rx)) => {
                    let session_id = Uuid::new_v4();
                    let ssh_session = Arc::new(RwLock::new(ssh_session));
                    
                    // 优化的数据转发
                    let tx_clone = tx.clone();
                    tokio::spawn(async move {
                        let mut buffer = Vec::with_capacity(4096);
                        
                        while let Some(data) = rx.recv().await {
                            buffer.extend_from_slice(&data);
                            
                            // 批量发送数据
                            if buffer.len() >= 1024 || rx.is_empty() {
                                let response = WebSocketResponse::Data { 
                                    data: String::from_utf8_lossy(&buffer).into_owned() 
                                };
                                if tx_clone.send(serde_json::to_string(&response).unwrap()).await.is_err() {
                                    break;
                                }
                                buffer.clear();
                            }
                        }
                    });
                    
                    // 添加到连接池
                    pool.insert(conn_key, ssh_session.clone()).await;
                    sessions.insert(session_id, ssh_session);
                    
                    let response = WebSocketResponse::Connected { session_id };
                    let _ = tx.send(serde_json::to_string(&response).unwrap()).await;
                }
                Err(e) => {
                    let response = WebSocketResponse::Error { 
                        message: format!("Connection failed: {}", e) 
                    };
                    let _ = tx.send(serde_json::to_string(&response).unwrap()).await;
                }
            }
        }
        WebSocketMessage::Data { session_id, data } => {
            if let Some(session) = sessions.get(&session_id) {
                // 使用读写锁的写锁
                let mut ssh = session.write().await;
                if let Err(e) = ssh.write_bytes(data.as_bytes()) {
                    let response = WebSocketResponse::Error { 
                        message: format!("Write failed: {}", e) 
                    };
                    let _ = tx.send(serde_json::to_string(&response).unwrap()).await;
                }
            }
        }
        WebSocketMessage::Disconnect { session_id } => {
            sessions.remove(&session_id);
            let response = WebSocketResponse::Disconnected;
            let _ = tx.send(serde_json::to_string(&response).unwrap()).await;
        }
    }
}
