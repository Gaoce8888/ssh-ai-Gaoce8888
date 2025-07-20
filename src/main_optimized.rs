use std::sync::Arc;
use std::net::SocketAddr;
use warp::Filter;
use tokio::sync::RwLock;
use dashmap::DashMap;
use uuid::Uuid;
use tracing::{info, error, warn};
use parking_lot::Mutex;
use lru::LruCache;
use std::num::NonZeroUsize;
use metrics::{counter, gauge, histogram};
use std::time::Instant;
use once_cell::sync::Lazy;

mod models;
mod ssh;
mod websocket;
mod ai;
mod config;
mod cache;
mod connection_pool;
mod performance;
mod security;

use models::*;
use websocket::handle_websocket;
use cache::ResponseCache;
use connection_pool::ConnectionPool;
use performance::PerformanceMonitor;
use security::AuthManager;

// 全局性能监控器
static PERFORMANCE_MONITOR: Lazy<Arc<PerformanceMonitor>> = Lazy::new(|| {
    Arc::new(PerformanceMonitor::new())
});

// 使用读写锁替代互斥锁，提升并发读性能
type Sessions = Arc<DashMap<Uuid, Arc<RwLock<ssh::SSHSession>>>>;
type Cache = Arc<Mutex<LruCache<String, Vec<u8>>>>;

#[tokio::main(flavor = "multi_thread", worker_threads = 0)] // 自动检测CPU核心数
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化配置
    let config = match config::ConfigManager::new("config.json").await {
        Ok(cm) => cm,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            return Err(e.into());
        }
    };
    
    let config_data = match config.get().await {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to get configuration: {}", e);
            return Err(e.into());
        }
    };
    
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
    let connection_pool = Arc::new(ConnectionPool::new(config_data.ssh.max_sessions));
    let response_cache = Arc::new(ResponseCache::new(config_data.cache.capacity));
    let auth_manager = Arc::new(AuthManager::new(&config_data.auth));
    
    // 创建LRU缓存用于静态文件
    let static_cache: Cache = Arc::new(Mutex::new(
        LruCache::new(NonZeroUsize::new(100).unwrap())
    ));

    info!("Starting SSH AI Terminal with optimizations");
    info!("CPU cores: {}", num_cpus::get());
    info!("Max connections: {}", config_data.server.max_connections);

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
        .and(with_auth_manager(auth_manager.clone()))
        .and_then(handle_websocket_with_pool);

    // AI路由 - 添加缓存支持
    let ai_route = warp::path!("api" / "ai" / "chat")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_sessions(sessions.clone()))
        .and(with_cache(response_cache.clone()))
        .and(with_auth_manager(auth_manager.clone()))
        .and_then(handle_ai_chat_cached);

    // 健康检查端点
    let health = warp::path!("health")
        .map(move || {
            gauge!("sessions_active", sessions.len() as f64);
            warp::reply::json(&serde_json::json!({
                "status": "healthy",
                "sessions": sessions.len(),
                "uptime": PERFORMANCE_MONITOR.uptime(),
                "version": env!("CARGO_PKG_VERSION")
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

    // 系统状态端点
    let status_route = warp::path!("status")
        .map(move || {
            let stats = PERFORMANCE_MONITOR.get_system_stats();
            warp::reply::json(&stats)
        });

    // 组合所有路由
    let routes = static_files
        .or(ws_route)
        .or(ai_route)
        .or(health)
        .or(metrics_route)
        .or(status_route)
        .with(warp::cors().allow_any_origin())
        .with(warp::trace::request())
        .with(performance_middleware());

    // 配置服务器
    let addr: SocketAddr = format!("{}:{}", config_data.server.address, config_data.server.port).parse()?;
    info!("Server starting on {} with {} workers", addr, num_cpus::get());

    // 启动服务器
    if config_data.server.tls.enabled {
        warp::serve(routes)
            .tls()
            .cert_path(&config_data.server.tls.cert_path)
            .key_path(&config_data.server.tls.key_path)
            .run(addr)
            .await;
    } else {
        warp::serve(routes)
            .run(addr)
            .await;
    }

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

fn with_auth_manager(auth: Arc<AuthManager>) -> impl Filter<Extract = (Arc<AuthManager>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || auth.clone())
}

async fn handle_websocket_with_pool(
    ws: warp::ws::Ws,
    sessions: Sessions,
    pool: Arc<ConnectionPool>,
    auth: Arc<AuthManager>,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(ws.on_upgrade(move |socket| {
        let start = Instant::now();
        counter!("websocket_connections_total", 1);
        gauge!("websocket_connections_active", 1.0);
        
        websocket::client_connection_optimized(socket, sessions, pool, auth)
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
    auth: Arc<AuthManager>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let start = Instant::now();
    
    // 验证认证
    if let Err(e) = auth.verify_request(&request).await {
        counter!("ai_auth_failures", 1);
        return Ok(warp::reply::json(&serde_json::json!({
            "error": "Authentication failed",
            "details": e.to_string()
        })));
    }
    
    let cache_key = format!("ai:{}:{}", request.session_id, request.message);
    
    // 尝试从缓存获取
    if let Some(cached_response) = cache.get(&cache_key).await {
        counter!("ai_cache_hits", 1);
        histogram!("ai_request_duration", start.elapsed());
        return Ok(warp::reply::json(&cached_response));
    }
    
    counter!("ai_cache_misses", 1);
    
    // 处理请求
    match ai::process_ai_request_optimized(request, sessions).await {
        Ok(response) => {
            // 缓存响应
            cache.set(cache_key, response.clone()).await;
            histogram!("ai_request_duration", start.elapsed());
            counter!("ai_requests_success", 1);
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            error!("AI request failed: {}", e);
            counter!("ai_errors_total", 1);
            histogram!("ai_request_duration", start.elapsed());
            Ok(warp::reply::json(&serde_json::json!({
                "error": e.to_string()
            })))
        }
    }
}

fn performance_middleware() -> impl Filter<Extract = (), Error = warp::Rejection> + Clone {
    warp::any()
        .map(|| Instant::now())
        .and(warp::path::full())
        .and(warp::method())
        .and(warp::reply::with::default(warp::http::StatusCode::OK))
        .map(|start: Instant, path: warp::path::FullPath, method: warp::http::Method, status: warp::http::StatusCode| {
            let duration = start.elapsed();
            
            counter!("http_requests_total", 1,
                "method" => method.to_string(),
                "path" => path.as_str().to_string(),
                "status" => status.as_u16().to_string()
            );
            
            histogram!("http_request_duration_seconds", duration.as_secs_f64(),
                "method" => method.to_string(),
                "path" => path.as_str().to_string()
            );
        })
        .untuple_one()
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
    metrics::describe_counter!("ai_requests_success", "Successful AI requests");
    metrics::describe_counter!("ai_auth_failures", "AI authentication failures");
    metrics::describe_histogram!("websocket_connection_duration", "WebSocket connection duration");
    metrics::describe_histogram!("ai_request_duration", "AI request processing duration");
    metrics::describe_histogram!("http_request_duration_seconds", "HTTP request duration");
}