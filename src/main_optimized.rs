use std::sync::Arc;
use std::net::SocketAddr;
use warp::Filter;
use tokio::sync::RwLock;
use dashmap::DashMap;
use uuid::Uuid;
use tracing::{info, error};
use anyhow::Result;

mod models;
mod ssh;
mod websocket;
mod ai;
mod config;
mod performance;
mod cache;
mod connection_pool;

use models::*;
use websocket::handle_websocket;
use performance::PerformanceMonitor;
use cache::{create_ai_cache, create_command_cache};
use connection_pool::{ConnectionPool, PoolConfig};

type Sessions = Arc<DashMap<Uuid, Arc<RwLock<ssh::SSHSession>>>>;

#[derive(Clone)]
struct AppState {
    sessions: Sessions,
    performance: Arc<PerformanceMonitor>,
    ai_cache: Arc<cache::AIResponseCache>,
    command_cache: Arc<cache::CommandCache>,
    connection_pool: Arc<ConnectionPool>,
    config: Arc<config::Config>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志系统
    init_logging();

    // 加载配置
    let config = load_config().await?;
    let port = config.server.port;
    let address = config.server.address.clone();

    // 初始化性能监控
    let performance = PerformanceMonitor::new();
    performance.clone().start_reporting();

    // 初始化缓存
    let ai_cache = Arc::new(create_ai_cache(config.cache.capacity));
    let command_cache = Arc::new(create_command_cache(config.cache.capacity));

    // 初始化连接池
    let pool_config = PoolConfig {
        max_size: config.ssh.max_sessions,
        min_idle: 5,
        max_idle_time: std::time::Duration::from_secs(config.ssh.keep_alive as u64),
        max_lifetime: std::time::Duration::from_secs(config.ssh.timeout as u64),
        connection_timeout: std::time::Duration::from_secs(10),
    };
    let connection_pool = Arc::new(ConnectionPool::new(pool_config));

    // 创建应用状态
    let app_state = AppState {
        sessions: Arc::new(DashMap::with_capacity(1000)),
        performance,
        ai_cache,
        command_cache,
        connection_pool,
        config: Arc::new(config),
    };

    // 启动定期清理任务
    start_cleanup_tasks(app_state.clone());

    // 配置路由
    let routes = configure_routes(app_state);

    // 启动服务器
    let addr: SocketAddr = format!("{}:{}", address, port).parse()?;
    info!("SSH-AI Terminal 服务器启动在 http://{}", addr);
    
    warp::serve(routes)
        .run(addr)
        .await;

    Ok(())
}

fn init_logging() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ssh_ai_terminal=info,warp=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn load_config() -> Result<config::Config> {
    let config_manager = config::ConfigManager::new("config.json").await?;
    Ok(config_manager.get().await?)
}

fn configure_routes(state: AppState) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // 静态文件服务（带缓存头）
    let static_files = warp::fs::dir("static")
        .with(warp::compression::gzip())
        .with(warp::compression::br())
        .with(warp::reply::with::header("Cache-Control", "public, max-age=3600"));

    // WebSocket路由
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(with_state(state.clone()))
        .and_then(handle_websocket_optimized);

    // AI路由
    let ai_route = warp::path!("api" / "ai" / "chat")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(handle_ai_request);

    // 健康检查路由
    let health_route = warp::path!("health")
        .and(with_state(state.clone()))
        .and_then(handle_health_check);

    // 性能指标路由
    let metrics_route = warp::path!("metrics")
        .and(with_state(state.clone()))
        .and_then(handle_metrics);

    // 组合所有路由
    static_files
        .or(ws_route)
        .or(ai_route)
        .or(health_route)
        .or(metrics_route)
        .with(warp::cors().allow_any_origin())
        .with(warp::log("http"))
        .recover(handle_rejection)
}

fn with_state(state: AppState) -> impl Filter<Extract = (AppState,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || state.clone())
}

async fn handle_websocket_optimized(
    ws: warp::ws::Ws,
    state: AppState,
) -> Result<impl warp::Reply, warp::Rejection> {
    state.performance.add_connection();
    Ok(ws.on_upgrade(move |socket| async move {
        websocket::client_connection_optimized(socket, state).await;
    }))
}

async fn handle_ai_request(
    request: AIRequest,
    state: AppState,
) -> Result<impl warp::Reply, warp::Rejection> {
    state.performance.record_request();
    state.performance.record_ai_request();

    // 检查缓存
    let cache_key = cache::generate_cache_key(&[&request.message, &request.session_id.to_string()]);
    
    if let Some(cached_response) = state.ai_cache.get(&cache_key) {
        state.performance.record_cache_hit();
        return Ok(warp::reply::json(&AIResponse {
            response: cached_response,
            success: true,
            error: None,
        }));
    }

    state.performance.record_cache_miss();

    // 处理AI请求
    match ai::process_ai_request(request, state.sessions.clone()).await {
        Ok(response) => {
            // 缓存响应
            state.ai_cache.put(cache_key, response.response.clone());
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            state.performance.record_error();
            Ok(warp::reply::json(&AIResponse {
                response: String::new(),
                success: false,
                error: Some(e.to_string()),
            }))
        }
    }
}

async fn handle_health_check(state: AppState) -> Result<impl warp::Reply, warp::Rejection> {
    let metrics = state.performance.get_metrics();
    let pool_stats = state.connection_pool.get_stats().await;
    
    let health = serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime": metrics.uptime_seconds,
        "metrics": metrics,
        "connection_pool": {
            "current_size": pool_stats.current_size,
            "current_idle": pool_stats.current_idle,
            "total_created": pool_stats.total_created,
        }
    });

    Ok(warp::reply::json(&health))
}

async fn handle_metrics(state: AppState) -> Result<impl warp::Reply, warp::Rejection> {
    let metrics = state.performance.get_metrics();
    Ok(warp::reply::json(&metrics))
}

async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, std::convert::Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = warp::http::StatusCode::NOT_FOUND;
        message = "NOT_FOUND";
    } else if let Some(_) = err.find::<warp::filters::body::BodyDeserializeError>() {
        code = warp::http::StatusCode::BAD_REQUEST;
        message = "BAD_REQUEST";
    } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        code = warp::http::StatusCode::METHOD_NOT_ALLOWED;
        message = "METHOD_NOT_ALLOWED";
    } else {
        error!("unhandled rejection: {:?}", err);
        code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
        message = "INTERNAL_SERVER_ERROR";
    }

    let json = warp::reply::json(&serde_json::json!({
        "code": code.as_u16(),
        "message": message,
    }));

    Ok(warp::reply::with_status(json, code))
}

fn start_cleanup_tasks(state: AppState) {
    // 定期清理缓存
    let cache_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            cache_state.ai_cache.cleanup();
            cache_state.command_cache.cleanup();
            info!("缓存清理完成");
        }
    });

    // 定期清理过期会话
    let session_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            let mut expired_sessions = Vec::new();
            
            for entry in session_state.sessions.iter() {
                let session_id = *entry.key();
                // 这里可以添加会话过期检查逻辑
                // if session.is_expired() {
                //     expired_sessions.push(session_id);
                // }
            }
            
            for session_id in expired_sessions {
                session_state.sessions.remove(&session_id);
                session_state.performance.remove_ssh_session();
            }
        }
    });
}