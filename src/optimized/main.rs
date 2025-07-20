use std::sync::Arc;
use std::net::SocketAddr;
use warp::Filter;
use tokio::sync::RwLock;
use dashmap::DashMap;
use uuid::Uuid;
use tracing::{info, error};
use parking_lot::Mutex;
use lru::LruCache;
use std::num::NonZeroUsize;
use metrics::{counter, gauge, histogram};

mod models;
mod ssh;
mod websocket;
mod ai;
mod connection_pool;
mod cache;
mod config;

use models::*;
use websocket::handle_websocket;
use connection_pool::ConnectionPool;
use cache::ResponseCache;
use config::Config;

// 使用读写锁替代互斥锁，提升并发读性能
type Sessions = Arc<DashMap<Uuid, Arc<RwLock<ssh::SSHSession>>>>;
type Cache = Arc<Mutex<LruCache<String, Vec<u8>>>>;

#[tokio::main(flavor = "multi_thread", worker_threads = 0)] // 自动检测CPU核心数
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化配置
    let config = Config::load()?;
    
    // 初始化高性能日志
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();

    // 初始化metrics
    init_metrics();

    // 创建全局状态
    let sessions: Sessions = Arc::new(DashMap::with_capacity(1000));
    let connection_pool = Arc::new(ConnectionPool::new(config.pool_size));
    let response_cache = Arc::new(ResponseCache::new(config.cache_size));
    
    // 创建LRU缓存用于静态文件
    let static_cache: Cache = Arc::new(Mutex::new(
        LruCache::new(NonZeroUsize::new(100).unwrap())
    ));

    // 静态文件服务，添加缓存和压缩
    let static_files = warp::fs::dir("static")
        .with(warp::compression::gzip())
        .with(warp::compression::br())
        .map(move |reply: warp::fs::File| {
            counter!("http_requests_total", 1, "endpoint" => "static");
            warp::reply::with_header(
                reply,
                "Cache-Control",
                "public, max-age=3600, immutable"
            )
        });

    // WebSocket路由 - 添加连接池支持
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(with_sessions(sessions.clone()))
        .and(with_connection_pool(connection_pool.clone()))
        .and_then(handle_websocket_with_pool);

    // AI路由 - 添加缓存支持
    let ai_route = warp::path!("api" / "ai" / "chat")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_sessions(sessions.clone()))
        .and(with_cache(response_cache.clone()))
        .and_then(handle_ai_chat_cached);

    // 健康检查端点
    let health = warp::path!("health")
        .map(|| {
            gauge!("sessions_active", sessions.len() as f64);
            warp::reply::json(&serde_json::json!({
                "status": "healthy",
                "sessions": sessions.len()
            }))
        });

    // Prometheus metrics端点
    let metrics_route = warp::path!("metrics")
        .map(|| {
            let encoder = metrics_exporter_prometheus::PrometheusBuilder::new()
                .build()
                .unwrap();
            encoder.render()
        });

    // 组合所有路由
    let routes = static_files
        .or(ws_route)
        .or(ai_route)
        .or(health)
        .or(metrics_route)
        .with(warp::cors().allow_any_origin())
        .with(warp::trace::request());

    // 配置服务器
    let addr: SocketAddr = format!("0.0.0.0:{}", config.port).parse()?;
    info!("Server starting on {} with {} workers", addr, num_cpus::get());

    // 启动服务器，启用HTTP/2
    warp::serve(routes)
        .tls()
        .cert_path(&config.tls_cert)
        .key_path(&config.tls_key)
        .run(addr)
        .await;

    Ok(())
}

fn with_sessions(sessions: Sessions) -> impl Filter<Extract = (Sessions,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || sessions.clone())
}

fn with_connection_pool(pool: Arc<ConnectionPool>) -> impl Filter<Extract = (Arc<ConnectionPool>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || pool.clone())
}

fn with_cache(cache: Arc<ResponseCache>) -> impl Filter<Extract = (Arc<ResponseCache>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || cache.clone())
}

async fn handle_websocket_with_pool(
    ws: warp::ws::Ws,
    sessions: Sessions,
    pool: Arc<ConnectionPool>,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(ws.on_upgrade(move |socket| {
        let start = std::time::Instant::now();
        counter!("websocket_connections_total", 1);
        gauge!("websocket_connections_active", 1.0);
        
        websocket::client_connection_optimized(socket, sessions, pool)
            .inspect(move |_| {
                histogram!("websocket_connection_duration", start.elapsed());
                gauge!("websocket_connections_active", -1.0);
            })
    }))
}

async fn handle_ai_chat_cached(
    request: AIRequest,
    sessions: Sessions,
    cache: Arc<ResponseCache>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let cache_key = format!("{:?}", request);
    
    // 尝试从缓存获取
    if let Some(cached_response) = cache.get(&cache_key).await {
        counter!("ai_cache_hits", 1);
        return Ok(warp::reply::json(&cached_response));
    }
    
    counter!("ai_cache_misses", 1);
    
    // 处理请求
    match ai::process_ai_request_optimized(request, sessions).await {
        Ok(response) => {
            // 缓存响应
            cache.set(cache_key, response.clone()).await;
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            error!("AI request failed: {}", e);
            counter!("ai_errors_total", 1);
            Ok(warp::reply::json(&serde_json::json!({
                "error": e.to_string()
            })))
        }
    }
}

fn init_metrics() {
    // 注册自定义metrics
    metrics::describe_counter!("http_requests_total", "Total HTTP requests");
    metrics::describe_counter!("websocket_connections_total", "Total WebSocket connections");
    metrics::describe_gauge!("websocket_connections_active", "Active WebSocket connections");
    metrics::describe_gauge!("sessions_active", "Active SSH sessions");
    metrics::describe_counter!("ai_cache_hits", "AI response cache hits");
    metrics::describe_counter!("ai_cache_misses", "AI response cache misses");
    metrics::describe_counter!("ai_errors_total", "Total AI processing errors");
    metrics::describe_histogram!("websocket_connection_duration", "WebSocket connection duration");
}
