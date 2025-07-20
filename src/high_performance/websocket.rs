use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;
use parking_lot::RwLock;
use flume;

use crate::{AppState, models::*};

// 消息批处理大小
const BATCH_SIZE: usize = 32;
const CHANNEL_SIZE: usize = 1024;

pub async fn handle_connection(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    
    // 使用flume channel获得更好的性能
    let (tx, rx) = flume::bounded(CHANNEL_SIZE);
    
    // 发送任务 - 批量处理
    let mut send_task = tokio::spawn(async move {
        let mut rx = rx.into_stream();
        let mut buffer = Vec::with_capacity(BATCH_SIZE);
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(5));
        
        loop {
            tokio::select! {
                Some(msg) = rx.next() => {
                    buffer.push(msg);
                    
                    // 批量发送
                    if buffer.len() >= BATCH_SIZE {
                        if let Err(_) = send_batch(&mut sender, &mut buffer).await {
                            break;
                        }
                    }
                }
                _ = interval.tick() => {
                    // 定时刷新
                    if !buffer.is_empty() {
                        if let Err(_) = send_batch(&mut sender, &mut buffer).await {
                            break;
                        }
                    }
                }
                else => break,
            }
        }
    });
    
    // 接收处理
    while let Some(msg) = receiver.next().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if let Ok(ws_msg) = simd_json::from_str::<WebSocketMessage>(&text) {
                        tokio::spawn(handle_message(ws_msg, state.clone(), tx.clone()));
                    }
                }
                Message::Binary(data) => {
                    // 处理二进制消息
                    if let Ok(ws_msg) = simd_json::from_slice::<WebSocketMessage>(&data) {
                        tokio::spawn(handle_message(ws_msg, state.clone(), tx.clone()));
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    }
    
    // 清理
    send_task.abort();
}

async fn send_batch(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    buffer: &mut Vec<String>,
) -> Result<(), axum::Error> {
    if buffer.len() == 1 {
        sender.send(Message::Text(buffer.remove(0))).await?;
    } else {
        // 合并为数组
        let combined = format!("[{}]", buffer.join(","));
        sender.send(Message::Text(combined)).await?;
        buffer.clear();
    }
    Ok(())
}

async fn handle_message(
    msg: WebSocketMessage,
    state: AppState,
    tx: flume::Sender<String>,
) -> anyhow::Result<()> {
    match msg {
        WebSocketMessage::Connect { host, port, username, password } => {
            // 尝试从连接池获取
            let pool_key = format!("{}@{}:{}", username, host, port);
            
            if let Some(session) = state.connection_pool.get(&pool_key).await {
                let session_id = Uuid::new_v4();
                state.sessions.insert(session_id, session);
                
                let response = WebSocketResponse::Connected { session_id };
                let _ = tx.send_async(simd_json::to_string(&response)?).await;
                return Ok(());
            }
            
            // 创建新连接
            match crate::ssh::SSHSession::new_async(&host, port, &username, &password).await {
                Ok((session, mut rx)) => {
                    let session_id = Uuid::new_v4();
                    let session = Arc::new(RwLock::new(session));
                    
                    // 数据转发任务
                    let tx_clone = tx.clone();
                    tokio::spawn(async move {
                        let mut buffer = Vec::with_capacity(4096);
                        
                        while let Some(data) = rx.recv().await {
                            buffer.extend_from_slice(&data);
                            
                            // 批量发送
                            if buffer.len() >= 1024 || rx.is_empty() {
                                let response = WebSocketResponse::Data {
                                    data: String::from_utf8_lossy(&buffer).into_owned(),
                                };
                                if let Ok(json) = simd_json::to_string(&response) {
                                    let _ = tx_clone.send_async(json).await;
                                }
                                buffer.clear();
                            }
                        }
                    });
                    
                    // 加入连接池
                    state.connection_pool.insert(pool_key, session.clone()).await;
                    state.sessions.insert(session_id, session);
                    
                    let response = WebSocketResponse::Connected { session_id };
                    let _ = tx.send_async(simd_json::to_string(&response)?).await;
                }
                Err(e) => {
                    let response = WebSocketResponse::Error {
                        message: format!("Connection failed: {}", e),
                    };
                    let _ = tx.send_async(simd_json::to_string(&response)?).await;
                }
            }
        }
        WebSocketMessage::Data { session_id, data } => {
            if let Some(session) = state.sessions.get(&session_id) {
                let result = {
                    let mut ssh = session.write();
                    ssh.write_bytes(data.as_bytes())
                };
                
                if let Err(e) = result {
                    let response = WebSocketResponse::Error {
                        message: format!("Write failed: {}", e),
                    };
                    let _ = tx.send_async(simd_json::to_string(&response)?).await;
                }
            }
        }
        WebSocketMessage::BatchData { session_id, data } => {
            if let Some(session) = state.sessions.get(&session_id) {
                let mut ssh = session.write();
                for cmd in data {
                    if let Err(e) = ssh.write_bytes(cmd.as_bytes()) {
                        let response = WebSocketResponse::Error {
                            message: format!("Batch write failed: {}", e),
                        };
                        let _ = tx.send_async(simd_json::to_string(&response)?).await;
                        break;
                    }
                }
            }
        }
        WebSocketMessage::Disconnect { session_id } => {
            state.sessions.remove(&session_id);
            let response = WebSocketResponse::Disconnected;
            let _ = tx.send_async(simd_json::to_string(&response)?).await;
        }
        WebSocketMessage::Ping => {
            let response = WebSocketResponse::Pong;
            let _ = tx.send_async(simd_json::to_string(&response)?).await;
        }
        _ => {}
    }
    
    Ok(())
}
