use warp::ws::{WebSocket, Message};
use futures_util::{StreamExt, SinkExt, stream::SplitSink};
use tokio::sync::mpsc;
use uuid::Uuid;
use tracing::{info, error, debug};
use std::sync::Arc;
use tokio::time::{timeout, Duration};

use crate::{models::*, ssh::SSHSession, AppState};

/// 优化的 WebSocket 连接处理
pub async fn client_connection_optimized(ws: WebSocket, state: AppState) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (tx, mut rx) = mpsc::channel(100);
    let session_id = Uuid::new_v4();
    
    info!("新的 WebSocket 连接建立: {}", session_id);
    state.performance.add_connection();

    // 发送任务 - 处理从内部到客户端的消息
    let tx_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(Message::text(msg)).await.is_err() {
                break;
            }
        }
    });

    // 接收任务 - 处理从客户端到内部的消息
    let rx_state = state.clone();
    let rx_tx = tx.clone();
    let rx_task = tokio::spawn(async move {
        while let Some(result) = ws_rx.next().await {
            match result {
                Ok(msg) => {
                    if let Ok(text) = msg.to_str() {
                        if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(text) {
                            handle_message_optimized(ws_msg, &rx_state, &rx_tx, session_id).await;
                        }
                    }
                }
                Err(e) => {
                    error!("WebSocket 错误: {}", e);
                    break;
                }
            }
        }
    });

    // 等待任务完成
    tokio::select! {
        _ = tx_task => debug!("发送任务结束"),
        _ = rx_task => debug!("接收任务结束"),
    }

    // 清理资源
    cleanup_session(&state, session_id).await;
    state.performance.remove_connection();
    info!("WebSocket 连接关闭: {}", session_id);
}

async fn handle_message_optimized(
    msg: WebSocketMessage,
    state: &AppState,
    tx: &mpsc::Sender<String>,
    session_id: Uuid,
) {
    state.performance.record_request();
    
    match msg.msg_type.as_str() {
        "connect" => handle_connect_optimized(msg, state, tx, session_id).await,
        "command" => handle_command_optimized(msg, state, tx, session_id).await,
        "disconnect" => handle_disconnect_optimized(state, tx, session_id).await,
        "ping" => handle_ping(tx).await,
        _ => {
            error!("未知消息类型: {}", msg.msg_type);
            send_error(tx, "未知消息类型").await;
        }
    }
}

async fn handle_connect_optimized(
    msg: WebSocketMessage,
    state: &AppState,
    tx: &mpsc::Sender<String>,
    session_id: Uuid,
) {
    if let Some(connect_msg) = msg.connect {
        info!("尝试连接到 SSH: {}:{}", connect_msg.host, connect_msg.port);
        
        // 尝试从连接池获取连接
        match state.connection_pool.get_connection(
            &connect_msg.host,
            connect_msg.port,
            &connect_msg.username,
            &connect_msg.password,
        ).await {
            Ok(pooled_conn) => {
                // 创建 SSH 会话
                match create_ssh_session_from_pooled(pooled_conn, tx.clone()).await {
                    Ok(session) => {
                        state.sessions.insert(session_id, session);
                        state.performance.add_ssh_session();
                        
                        let response = WebSocketMessage {
                            msg_type: "connected".to_string(),
                            session_id: Some(session_id),
                            connect: None,
                            command: None,
                            data: Some("SSH 连接成功".to_string()),
                            error: None,
                        };
                        
                        if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                            error!("发送连接成功消息失败: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("创建 SSH 会话失败: {}", e);
                        send_error(tx, &format!("连接失败: {}", e)).await;
                    }
                }
            }
            Err(e) => {
                error!("从连接池获取连接失败: {}", e);
                send_error(tx, &format!("连接失败: {}", e)).await;
            }
        }
    }
}

async fn handle_command_optimized(
    msg: WebSocketMessage,
    state: &AppState,
    tx: &mpsc::Sender<String>,
    session_id: Uuid,
) {
    if let Some(command_msg) = msg.command {
        if let Some(session_id_from_msg) = msg.session_id {
            // 检查命令缓存
            let cache_key = crate::cache::generate_cache_key(&[
                &session_id_from_msg.to_string(),
                &command_msg.command,
            ]);
            
            if let Some(cached_result) = state.command_cache.get(&cache_key) {
                state.performance.record_cache_hit();
                
                let response = WebSocketMessage {
                    msg_type: "output".to_string(),
                    session_id: Some(session_id_from_msg),
                    connect: None,
                    command: None,
                    data: Some(String::from_utf8_lossy(&cached_result).to_string()),
                    error: None,
                };
                
                if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
                    error!("发送缓存命令输出失败: {}", e);
                }
                return;
            }
            
            state.performance.record_cache_miss();
            
            // 执行命令
            if let Some(session_entry) = state.sessions.get(&session_id_from_msg) {
                let session = session_entry.clone();
                let command = command_msg.command.clone();
                let tx_clone = tx.clone();
                let cache = state.command_cache.clone();
                
                // 使用超时执行命令
                tokio::spawn(async move {
                    match timeout(
                        Duration::from_secs(30),
                        execute_command(session, &command)
                    ).await {
                        Ok(Ok(output)) => {
                            // 缓存结果
                            cache.put(cache_key, output.as_bytes().to_vec());
                            
                            let response = WebSocketMessage {
                                msg_type: "output".to_string(),
                                session_id: Some(session_id_from_msg),
                                connect: None,
                                command: None,
                                data: Some(output),
                                error: None,
                            };
                            
                            if let Err(e) = tx_clone.send(serde_json::to_string(&response).unwrap()).await {
                                error!("发送命令输出失败: {}", e);
                            }
                        }
                        Ok(Err(e)) => {
                            error!("执行命令失败: {}", e);
                            send_error(&tx_clone, &format!("命令执行失败: {}", e)).await;
                        }
                        Err(_) => {
                            error!("命令执行超时");
                            send_error(&tx_clone, "命令执行超时").await;
                        }
                    }
                });
            } else {
                send_error(tx, "会话不存在").await;
            }
        }
    }
}

async fn handle_disconnect_optimized(
    state: &AppState,
    tx: &mpsc::Sender<String>,
    session_id: Uuid,
) {
    cleanup_session(state, session_id).await;
    
    let response = WebSocketMessage {
        msg_type: "disconnected".to_string(),
        session_id: Some(session_id),
        connect: None,
        command: None,
        data: Some("SSH 连接已断开".to_string()),
        error: None,
    };
    
    if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
        error!("发送断开连接消息失败: {}", e);
    }
}

async fn handle_ping(tx: &mpsc::Sender<String>) {
    let response = WebSocketMessage {
        msg_type: "pong".to_string(),
        session_id: None,
        connect: None,
        command: None,
        data: Some("pong".to_string()),
        error: None,
    };
    
    if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
        error!("发送 pong 消息失败: {}", e);
    }
}

async fn send_error(tx: &mpsc::Sender<String>, error_msg: &str) {
    let response = WebSocketMessage {
        msg_type: "error".to_string(),
        session_id: None,
        connect: None,
        command: None,
        data: None,
        error: Some(error_msg.to_string()),
    };
    
    if let Err(e) = tx.send(serde_json::to_string(&response).unwrap()).await {
        error!("发送错误消息失败: {}", e);
    }
}

async fn cleanup_session(state: &AppState, session_id: Uuid) {
    if state.sessions.remove(&session_id).is_some() {
        state.performance.remove_ssh_session();
        info!("清理会话: {}", session_id);
    }
}

async fn create_ssh_session_from_pooled(
    pooled_conn: crate::connection_pool::PooledConnection,
    tx: mpsc::Sender<String>,
) -> Result<Arc<tokio::sync::RwLock<SSHSession>>, Box<dyn std::error::Error>> {
    // 这里需要根据实际的 SSHSession 结构来实现
    // 暂时返回一个占位符
    todo!("实现从池化连接创建 SSH 会话")
}

async fn execute_command(
    session: Arc<tokio::sync::RwLock<SSHSession>>,
    command: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // 这里需要根据实际的 SSHSession 结构来实现
    // 暂时返回一个占位符
    todo!("实现命令执行")
}