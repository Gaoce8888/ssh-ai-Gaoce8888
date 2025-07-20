use metrics::{counter, gauge, histogram};
use std::time::Instant;
use tracing::info;
use serde::Serialize;
use std::sync::Arc;
use parking_lot::Mutex;
use std::collections::HashMap;

pub struct PerformanceMonitor {
    start_time: Instant,
    request_stats: Arc<Mutex<RequestStats>>,
    system_stats: Arc<Mutex<SystemStats>>,
}

#[derive(Debug, Clone)]
struct RequestStats {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    average_response_time: f64,
    response_times: Vec<f64>,
}

#[derive(Debug, Clone)]
struct SystemStats {
    memory_usage: u64,
    cpu_usage: f64,
    active_connections: u64,
    cache_hit_rate: f64,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            request_stats: Arc::new(Mutex::new(RequestStats {
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                average_response_time: 0.0,
                response_times: Vec::new(),
            })),
            system_stats: Arc::new(Mutex::new(SystemStats {
                memory_usage: 0,
                cpu_usage: 0.0,
                active_connections: 0,
                cache_hit_rate: 0.0,
            })),
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

        // 更新内部统计
        let mut stats = self.request_stats.lock();
        stats.total_requests += 1;
        stats.response_times.push(duration);
        
        if status < 400 {
            stats.successful_requests += 1;
        } else {
            stats.failed_requests += 1;
        }

        // 保持最近1000个响应时间用于计算平均值
        if stats.response_times.len() > 1000 {
            stats.response_times.remove(0);
        }
        
        stats.average_response_time = stats.response_times.iter().sum::<f64>() / stats.response_times.len() as f64;
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
        
        let mut stats = self.system_stats.lock();
        if delta > 0 {
            stats.active_connections += delta as u64;
        } else {
            stats.active_connections = stats.active_connections.saturating_sub((-delta) as u64);
        }
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
                let memory_usage = stat.vsize;
                gauge!("process_memory_bytes", memory_usage as f64);
                gauge!("process_cpu_usage", stat.utime as f64);
                
                let mut stats = self.system_stats.lock();
                stats.memory_usage = memory_usage;
                stats.cpu_usage = stat.utime as f64;
            }
        }
    }
    
    pub fn update_cache_hit_rate(&self, hit_rate: f64) {
        let mut stats = self.system_stats.lock();
        stats.cache_hit_rate = hit_rate;
        gauge!("cache_hit_rate", hit_rate);
    }
    
    pub fn uptime(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    pub fn get_system_stats(&self) -> SystemMetrics {
        self.update_memory_usage();
        
        let request_stats = self.request_stats.lock();
        let system_stats = self.system_stats.lock();
        
        SystemMetrics {
            uptime: self.uptime(),
            total_requests: request_stats.total_requests,
            successful_requests: request_stats.successful_requests,
            failed_requests: request_stats.failed_requests,
            average_response_time: request_stats.average_response_time,
            memory_usage_mb: system_stats.memory_usage as f64 / 1024.0 / 1024.0,
            cpu_usage_percent: system_stats.cpu_usage,
            active_connections: system_stats.active_connections,
            cache_hit_rate: system_stats.cache_hit_rate,
        }
    }

    pub fn get_health_status(&self) -> HealthStatus {
        let stats = self.get_system_stats();
        
        let is_healthy = stats.average_response_time < 1.0 
            && stats.memory_usage_mb < 1024.0 
            && stats.cache_hit_rate > 0.5;
        
        HealthStatus {
            status: if is_healthy { "healthy".to_string() } else { "degraded".to_string() },
            uptime: stats.uptime,
            version: env!("CARGO_PKG_VERSION").to_string(),
            metrics: stats,
        }
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
    monitor.get_health_status()
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
    pub uptime: f64,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub active_connections: u64,
    pub cache_hit_rate: f64,
}

impl SystemMetrics {
    pub fn error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.failed_requests as f64 / self.total_requests as f64
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.successful_requests as f64 / self.total_requests as f64
        }
    }

    pub fn requests_per_second(&self) -> f64 {
        if self.uptime == 0.0 {
            0.0
        } else {
            self.total_requests as f64 / self.uptime
        }
    }
}

// 全局性能监控器
use once_cell::sync::OnceCell;
static PERFORMANCE_MONITOR: OnceCell<PerformanceMonitor> = OnceCell::new();

pub fn init_performance_monitor() {
    PERFORMANCE_MONITOR.set(PerformanceMonitor::new()).unwrap();
}

pub fn get_performance_monitor() -> Option<&'static PerformanceMonitor> {
    PERFORMANCE_MONITOR.get()
}

// 性能基准测试工具
pub struct BenchmarkRunner {
    monitor: Arc<PerformanceMonitor>,
}

impl BenchmarkRunner {
    pub fn new() -> Self {
        Self {
            monitor: Arc::new(PerformanceMonitor::new()),
        }
    }

    pub async fn run_benchmark<F, Fut>(&self, name: &str, iterations: usize, test_fn: F) -> BenchmarkResult
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send,
    {
        let mut durations = Vec::new();
        let mut errors = 0;

        for i in 0..iterations {
            let start = Instant::now();
            match test_fn().await {
                Ok(_) => {
                    let duration = start.elapsed();
                    durations.push(duration.as_secs_f64());
                }
                Err(_) => {
                    errors += 1;
                }
            }

            if i % 100 == 0 {
                info!("Benchmark {}: {}/{} iterations completed", name, i, iterations);
            }
        }

        let success_rate = if iterations == 0 {
            0.0
        } else {
            (iterations - errors) as f64 / iterations as f64
        };

        durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let avg_duration = durations.iter().sum::<f64>() / durations.len() as f64;
        let p50_duration = durations[durations.len() / 2];
        let p95_duration = durations[(durations.len() as f64 * 0.95) as usize];
        let p99_duration = durations[(durations.len() as f64 * 0.99) as usize];

        BenchmarkResult {
            name: name.to_string(),
            iterations,
            errors,
            success_rate,
            avg_duration,
            p50_duration,
            p95_duration,
            p99_duration,
            min_duration: durations[0],
            max_duration: durations[durations.len() - 1],
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: usize,
    pub errors: usize,
    pub success_rate: f64,
    pub avg_duration: f64,
    pub p50_duration: f64,
    pub p95_duration: f64,
    pub p99_duration: f64,
    pub min_duration: f64,
    pub max_duration: f64,
}

impl BenchmarkResult {
    pub fn print_summary(&self) {
        info!("Benchmark Results for '{}':", self.name);
        info!("  Iterations: {}", self.iterations);
        info!("  Success Rate: {:.2}%", self.success_rate * 100.0);
        info!("  Average Duration: {:.3}ms", self.avg_duration * 1000.0);
        info!("  P50 Duration: {:.3}ms", self.p50_duration * 1000.0);
        info!("  P95 Duration: {:.3}ms", self.p95_duration * 1000.0);
        info!("  P99 Duration: {:.3}ms", self.p99_duration * 1000.0);
        info!("  Min Duration: {:.3}ms", self.min_duration * 1000.0);
        info!("  Max Duration: {:.3}ms", self.max_duration * 1000.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_monitor() {
        let monitor = PerformanceMonitor::new();
        
        monitor.record_request("test", 200, 0.1);
        monitor.record_request("test", 404, 0.2);
        
        let stats = monitor.get_system_stats();
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.successful_requests, 1);
        assert_eq!(stats.failed_requests, 1);
    }

    #[tokio::test]
    async fn test_benchmark_runner() {
        let runner = BenchmarkRunner::new();
        
        let result = runner.run_benchmark("test", 10, || async {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            Ok(())
        }).await;
        
        assert_eq!(result.iterations, 10);
        assert_eq!(result.errors, 0);
        assert!(result.success_rate > 0.9);
    }
}