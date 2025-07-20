use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use dashmap::DashMap;
use moka::future::Cache;
use parking_lot::RwLock;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::{info, Level};
use uuid::Uuid;

mod api;
mod config_manager;
mod models;
mod ssh;
mod websocket;
mod ai;

use models::*;

// 使用 mimalloc 作为全局分配器
#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// 使用 jemalloc 作为全局分配器（Linux）
#[cfg(all(feature = "jemalloc", not(target_env = "msvc")))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

// 应用状态
#[derive(Clone)]
struct AppState {
    sessions: Arc<DashMap<Uuid, Arc<RwLock<ssh::SSHSession>>>>,
    config_cache: Arc<Cache<String, ConfigData>>,
    ai_cache: Arc<Cache<String, String>>,
    connection_pool: Arc<ssh::ConnectionPool>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    init_tracing();

    // 加载配置
    let config = config::load_config()?;

    // 创建应用状态
    let state = AppState {
        sessions: Arc::new(DashMap::with_capacity_and_hasher(1000, ahash::RandomState::new())),
        config_cache: Arc::new(
            Cache::builder()
                .max_capacity(10_000)
                .time_to_live(Duration::from_secs(300))
                .build(),
        ),
        ai_cache: Arc::new(
            Cache::builder()
                .max_capacity(1_000)
                .time_to_live(Duration::from_secs(600))
                .build(),
        ),
        connection_pool: Arc::new(ssh::ConnectionPool::new(config.pool_size)),
    };

    // 启动后台任务
    start_background_tasks(state.clone());

    // 构建路由
    let app = Router::new()
        // WebSocket端点
        .route("/ws", get(websocket_handler))
        // API端点 - 原生Rust实现，取代PHP
        .route("/api/configs", get(api::get_configs).post(api::save_config))
        .route("/api/configs/:id", get(api::get_config).delete(api::delete_config))
        .route("/api/ai/chat", post(api::ai_chat))
        .route("/api/health", get(health_check))
        .route("/api/metrics", get(metrics_handler))
        // 静态文件服务
        .nest_service(
            "/",
            ServeDir::new("static")
                .precompressed_gzip()
                .precompressed_br()
                .precompressed_deflate(),
        )
        // 中间件
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any),
                )
                .timeout(Duration::from_secs(30)),
        )
        .with_state(state);

    // 绑定地址
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("High-Performance SSH AI Terminal starting on {}", addr);

    // 使用hyper的高性能配置
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .tcp_nodelay(true)
    .await?;

    Ok(())
}

// WebSocket处理器
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| websocket::handle_connection(socket, state))
}

// 健康检查
async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let stats = HealthStats {
        status: "healthy".to_string(),
        active_sessions: state.sessions.len(),
        cache_size: state.config_cache.entry_count() as usize,
        pool_connections: state.connection_pool.active_connections(),
        uptime: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };
    Json(stats)
}

// 性能指标
async fn metrics_handler(State(state): State<AppState>) -> impl IntoResponse {
    let metrics = collect_metrics(&state).await;
    Json(metrics)
}

// 初始化日志
fn init_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ssh_ai_terminal=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();
}

// 启动后台任务
fn start_background_tasks(state: AppState) {
    // 连接池清理任务
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            state.connection_pool.cleanup().await;
        }
    });
}

// 收集性能指标
async fn collect_metrics(state: &AppState) -> Metrics {
    Metrics {
        active_sessions: state.sessions.len(),
        cache_entries: state.config_cache.entry_count() as usize,
        ai_cache_entries: state.ai_cache.entry_count() as usize,
        pool_connections: state.connection_pool.active_connections(),
        memory_usage_mb: get_memory_usage_mb(),
        cpu_usage_percent: get_cpu_usage(),
    }
}

#[derive(serde::Serialize)]
struct HealthStats {
    status: String,
    active_sessions: usize,
    cache_size: usize,
    pool_connections: usize,
    uptime: u64,
}

#[derive(serde::Serialize)]
struct Metrics {
    active_sessions: usize,
    cache_entries: usize,
    ai_cache_entries: usize,
    pool_connections: usize,
    memory_usage_mb: f64,
    cpu_usage_percent: f64,
}

fn get_memory_usage_mb() -> f64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(process) = procfs::process::Process::myself() {
            if let Ok(stat) = process.stat() {
                return (stat.vsize as f64) / 1024.0 / 1024.0;
            }
        }
    }
    0.0
}

fn get_cpu_usage() -> f64 {
    // 实现CPU使用率获取
    0.0
}
