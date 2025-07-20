use std::sync::Arc;
use std::net::SocketAddr;
use warp::Filter;
use tokio::sync::Mutex;
use dashmap::DashMap;
use uuid::Uuid;
use tracing::{info, error};

mod models;
mod ssh;
mod websocket;
mod ai;
mod config;
// 新增优化模块
mod performance;
mod cache;
mod connection_pool;

use models::*;
use websocket::handle_websocket;

type Sessions = Arc<DashMap<Uuid, Arc<Mutex<ssh::SSHSession>>>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Load configuration
    let config_manager = match config::ConfigManager::new("config.json").await {
        Ok(cm) => cm,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            return;
        }
    };

    let config = match config_manager.get().await {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to get configuration: {}", e);
            return;
        }
    };
    let port = config.server.port;
    let address = config.server.address;

    let sessions: Sessions = Arc::new(DashMap::new());

    let static_files = warp::fs::dir("static");

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(with_sessions(sessions.clone()))
        .and_then(handle_websocket);

    let ai_route = warp::path!("api" / "ai" / "chat")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_sessions(sessions.clone()))
        .and_then(handle_ai_chat);

    let routes = ws_route
        .or(ai_route)
        .or(static_files);

    let addr: SocketAddr = format!("{}:{}", address, port).parse().unwrap();
    info!("Server starting on {}", addr);

    warp::serve(routes)
        .run(addr)
        .await;
}

fn with_sessions(sessions: Sessions) -> impl Filter<Extract = (Sessions,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || sessions.clone())
}

async fn handle_ai_chat(
    request: AIRequest,
    sessions: Sessions,
) -> Result<impl warp::Reply, warp::Rejection> {
    match ai::process_ai_request(request, sessions).await {
        Ok(response) => Ok(warp::reply::json(&response)),
        Err(e) => {
            error!("AI request failed: {}", e);
            Ok(warp::reply::json(&serde_json::json!({
                "error": e.to_string()
            })))
        }
    }
}
