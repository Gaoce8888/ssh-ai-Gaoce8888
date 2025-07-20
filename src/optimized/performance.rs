use metrics::{counter, gauge, histogram};
use std::time::Instant;
use tracing::info;

pub struct PerformanceMonitor {
    start_time: Instant,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }
    
    pub fn record_request(&self, endpoint: &str, status: u16, duration: f64) {
        counter!("http_requests_total", 1, 
            "endpoint" => endpoint.to_string(),
            "status" => status.to_string()
        );
        
        histogram!("http_request_duration_seconds", duration,
            "endpoint" => endpoint.to_string()
        );
    }
    
    pub fn record_websocket_message(&self, message_type: &str, size: usize) {
        counter!("websocket_messages_total", 1,
            "type" => message_type.to_string()
        );
        
        histogram!("websocket_message_size_bytes", size as f64,
            "type" => message_type.to_string()
        );
    }
    
    pub fn update_active_connections(&self, delta: i64) {
        gauge!("active_connections", delta as f64);
    }
    
    pub fn record_ssh_operation(&self, operation: &str, success: bool, duration: f64) {
        counter!("ssh_operations_total", 1,
            "operation" => operation.to_string(),
            "success" => success.to_string()
        );
        
        if success {
            histogram!("ssh_operation_duration_seconds", duration,
                "operation" => operation.to_string()
            );
        }
    }
    
    pub fn record_ai_request(&self, provider: &str, success: bool, duration: f64) {
        counter!("ai_requests_total", 1,
            "provider" => provider.to_string(),
            "success" => success.to_string()
        );
        
        if success {
            histogram!("ai_request_duration_seconds", duration,
                "provider" => provider.to_string()
            );
        }
    }
    
    pub fn record_cache_operation(&self, operation: &str, hit: bool) {
        counter!("cache_operations_total", 1,
            "operation" => operation.to_string(),
            "hit" => hit.to_string()
        );
    }
    
    pub fn update_memory_usage(&self) {
        if let Ok(process) = procfs::process::Process::myself() {
            if let Ok(stat) = process.stat() {
                gauge!("process_memory_bytes", stat.vsize as f64);
                gauge!("process_cpu_usage", stat.utime as f64);
            }
        }
    }
    
    pub fn uptime(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }
}

// 请求追踪中间件
pub fn request_metrics() -> impl warp::Filter<Extract = (), Error = warp::Rejection> + Clone {
    warp::any()
        .map(|| Instant::now())
        .and(warp::path::full())
        .and(warp::method())
        .and(warp::reply::with::default(warp::http::StatusCode::OK))
        .map(|start: Instant, path: warp::path::FullPath, method: warp::http::Method, status: warp::http::StatusCode| {
            let duration = start.elapsed().as_secs_f64();
            
            counter!("http_requests_total", 1,
                "method" => method.to_string(),
                "path" => path.as_str().to_string(),
                "status" => status.as_u16().to_string()
            );
            
            histogram!("http_request_duration_seconds", duration,
                "method" => method.to_string(),
                "path" => path.as_str().to_string()
            );
        })
        .untuple_one()
}

// 系统健康检查
pub async fn health_check() -> HealthStatus {
    let monitor = PERFORMANCE_MONITOR.get().unwrap();
    
    HealthStatus {
        status: "healthy".to_string(),
        uptime: monitor.uptime(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        metrics: SystemMetrics {
            active_connections: get_metric_value("active_connections"),
            total_requests: get_metric_value("http_requests_total"),
            cache_hit_rate: calculate_cache_hit_rate(),
            memory_usage_mb: get_memory_usage_mb(),
            cpu_usage_percent: get_cpu_usage_percent(),
        },
    }
}

#[derive(serde::Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub uptime: f64,
    pub version: String,
    pub metrics: SystemMetrics,
}

#[derive(serde::Serialize)]
pub struct SystemMetrics {
    pub active_connections: f64,
    pub total_requests: f64,
    pub cache_hit_rate: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

fn get_metric_value(name: &str) -> f64 {
    // 实际实现需要从metrics registry获取
    0.0
}

fn calculate_cache_hit_rate() -> f64 {
    // 计算缓存命中率
    0.0
}

fn get_memory_usage_mb() -> f64 {
    if let Ok(process) = procfs::process::Process::myself() {
        if let Ok(stat) = process.stat() {
            return (stat.vsize as f64) / 1024.0 / 1024.0;
        }
    }
    0.0
}

fn get_cpu_usage_percent() -> f64 {
    // 获取CPU使用率
    0.0
}

// 全局性能监控器
use once_cell::sync::OnceCell;
static PERFORMANCE_MONITOR: OnceCell<PerformanceMonitor> = OnceCell::new();

pub fn init_performance_monitor() {
    PERFORMANCE_MONITOR.set(PerformanceMonitor::new()).unwrap();
}
