use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::interval;
use tracing::{info, warn};
use serde::{Serialize, Deserialize};

/// 性能指标收集器
pub struct PerformanceMonitor {
    start_time: Instant,
    request_count: AtomicU64,
    active_connections: AtomicUsize,
    total_bytes_processed: AtomicU64,
    error_count: AtomicU64,
    ssh_sessions: AtomicUsize,
    ai_requests: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PerformanceMetrics {
    pub uptime_seconds: f64,
    pub requests_per_second: f64,
    pub active_connections: usize,
    pub total_requests: u64,
    pub total_bytes_processed: u64,
    pub error_rate: f64,
    pub ssh_sessions: usize,
    pub ai_requests: u64,
    pub cache_hit_rate: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

impl PerformanceMonitor {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            start_time: Instant::now(),
            request_count: AtomicU64::new(0),
            active_connections: AtomicUsize::new(0),
            total_bytes_processed: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            ssh_sessions: AtomicUsize::new(0),
            ai_requests: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
        })
    }

    pub fn record_request(&self) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_bytes(&self, bytes: u64) {
        self.total_bytes_processed.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_connection(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn remove_connection(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn add_ssh_session(&self) {
        self.ssh_sessions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn remove_ssh_session(&self) {
        self.ssh_sessions.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn record_ai_request(&self) {
        self.ai_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_metrics(&self) -> PerformanceMetrics {
        let uptime = self.start_time.elapsed().as_secs_f64();
        let total_requests = self.request_count.load(Ordering::Relaxed);
        let total_errors = self.error_count.load(Ordering::Relaxed);
        let cache_hits = self.cache_hits.load(Ordering::Relaxed);
        let cache_misses = self.cache_misses.load(Ordering::Relaxed);
        let total_cache_ops = cache_hits + cache_misses;

        PerformanceMetrics {
            uptime_seconds: uptime,
            requests_per_second: if uptime > 0.0 { total_requests as f64 / uptime } else { 0.0 },
            active_connections: self.active_connections.load(Ordering::Relaxed),
            total_requests,
            total_bytes_processed: self.total_bytes_processed.load(Ordering::Relaxed),
            error_rate: if total_requests > 0 { 
                (total_errors as f64 / total_requests as f64) * 100.0 
            } else { 
                0.0 
            },
            ssh_sessions: self.ssh_sessions.load(Ordering::Relaxed),
            ai_requests: self.ai_requests.load(Ordering::Relaxed),
            cache_hit_rate: if total_cache_ops > 0 {
                (cache_hits as f64 / total_cache_ops as f64) * 100.0
            } else {
                0.0
            },
            memory_usage_mb: get_memory_usage(),
            cpu_usage_percent: get_cpu_usage(),
        }
    }

    /// 启动定期性能报告
    pub fn start_reporting(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let metrics = self.get_metrics();
                info!(
                    "性能报告 - 运行时间: {:.0}s, RPS: {:.2}, 活跃连接: {}, 错误率: {:.2}%, 缓存命中率: {:.2}%",
                    metrics.uptime_seconds,
                    metrics.requests_per_second,
                    metrics.active_connections,
                    metrics.error_rate,
                    metrics.cache_hit_rate
                );

                // 检查异常情况
                if metrics.error_rate > 10.0 {
                    warn!("错误率过高: {:.2}%", metrics.error_rate);
                }
                if metrics.memory_usage_mb > 2048.0 {
                    warn!("内存使用过高: {:.2}MB", metrics.memory_usage_mb);
                }
            }
        });
    }
}

fn get_memory_usage() -> f64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<f64>() {
                            return kb / 1024.0;
                        }
                    }
                }
            }
        }
    }
    0.0
}

fn get_cpu_usage() -> f64 {
    // 简化的CPU使用率计算
    // 在生产环境中应该使用更精确的方法
    0.0
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new().as_ref().clone()
    }
}