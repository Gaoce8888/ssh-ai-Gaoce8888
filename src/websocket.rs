use warp::ws::{WebSocket, Message};
use futures_util::{StreamExt, SinkExt};
use tokio::sync::mpsc;
use uuid::Uuid;
use tracing::{info, error};

use crate::{Sessions, models::*, ssh::SSHSession};

pub async fn handle_websocket(
    ws: warp::ws::Ws,
    sessions: Sessions,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(ws.on_upgrade(move |socket| client_connection(socket, sessions)))
}

async fn client_connection(ws: WebSocket, sessions: Sessions) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (tx, mut rx) = mpsc::channel(100);

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(Message::text(msg)).await.is_err() {
                break;
            }
        }
    });

    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(msg) => {
                if let Ok(text) = msg.to_str() {
                    if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(text) {
                        handle_message(ws_msg, &sessions, &tx).await;
                    }
                }
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
        }
    }

    info!("WebSocket connection closed");
}

async fn handle_message(
    msg: WebSocketMessage,
    sessions: &Sessions,
    tx: &mpsc::Sender<String>,
) {
    match msg {
        WebSocketMessage::Connect { host, port, username, password } => {
            match SSHSession::new(&host, port, &username, &password).await {
                Ok((ssh_session, mut rx)) => {
                    let session_id = Uuid::new_v4();
                    
                    let tx_clone = tx.clone();
                    tokio::spawn(async move {
                        while let Some(data) = rx.recv().await {
                            let response = WebSocketResponse::Data { data };
                            if tx_clone.send(serde_json::to_string(&response).unwrap()).await.is_err() {
                                break;
                            }
                        }
                    });
                    
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
                let mut ssh = session.lock().await;
                if let Err(e) = ssh.write(&data) {
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
