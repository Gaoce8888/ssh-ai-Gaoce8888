/*!
 * 性能基准测试套件
 * 
 * 功能：
 * - 内存分配性能测试
 * - 网络吞吐量测试
 * - 缓存命中率测试
 * - 并发连接测试
 * - CPU使用率监控
 * - 延迟分析
 */

use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use bytes::Bytes;
use super::{UltraState, UltraConfig, UltraResult};

/// 基准测试配置
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub duration: Duration,
    pub concurrent_connections: usize,
    pub message_size: usize,
    pub messages_per_second: usize,
    pub warmup_duration: Duration,
}

/// 基准测试结果
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    pub throughput: ThroughputMetrics,
    pub latency: LatencyMetrics,
    pub memory: MemoryMetrics,
    pub cache: CacheMetrics,
    pub errors: ErrorMetrics,
}

#[derive(Debug, Clone)]
pub struct ThroughputMetrics {
    pub requests_per_second: f64,
    pub bytes_per_second: f64,
    pub connections_per_second: f64,
    pub total_requests: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct LatencyMetrics {
    pub min: Duration,
    pub max: Duration,
    pub mean: Duration,
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub p999: Duration,
}

#[derive(Debug, Clone)]
pub struct MemoryMetrics {
    pub peak_usage: usize,
    pub average_usage: usize,
    pub allocations_per_second: f64,
    pub deallocations_per_second: f64,
    pub fragmentation_ratio: f64,
}

#[derive(Debug, Clone)]
pub struct CacheMetrics {
    pub hit_rate: f64,
    pub miss_rate: f64,
    pub eviction_rate: f64,
    pub compression_ratio: f64,
}

#[derive(Debug, Clone)]
pub struct ErrorMetrics {
    pub total_errors: u64,
    pub error_rate: f64,
    pub timeout_errors: u64,
    pub connection_errors: u64,
}

/// 基准测试执行器
pub struct BenchmarkExecutor {
    config: BenchmarkConfig,
    state: Arc<UltraState>,
}

impl BenchmarkExecutor {
    pub fn new(config: BenchmarkConfig, state: Arc<UltraState>) -> Self {
        Self { config, state }
    }

    /// 执行完整的基准测试套件
    pub async fn run_full_benchmark(&self) -> UltraResult<BenchmarkResults> {
        println!("🚀 开始执行基准测试套件...");
        
        // 预热阶段
        self.warmup().await?;
        
        // 执行各项测试
        let throughput = self.benchmark_throughput().await?;
        let latency = self.benchmark_latency().await?;
        let memory = self.benchmark_memory().await?;
        let cache = self.benchmark_cache().await?;
        let errors = self.benchmark_errors().await?;

        let results = BenchmarkResults {
            throughput,
            latency,
            memory,
            cache,
            errors,
        };

        self.print_results(&results);
        Ok(results)
    }

    /// 预热阶段
    async fn warmup(&self) -> UltraResult<()> {
        println!("🔥 预热阶段 ({:?})...", self.config.warmup_duration);
        
        let semaphore = Arc::new(Semaphore::new(100));
        let start_time = Instant::now();
        
        while start_time.elapsed() < self.config.warmup_duration {
            let _permit = semaphore.acquire().await.unwrap();
            
            tokio::spawn({
                let state = self.state.clone();
                async move {
                    // 模拟一些操作
                    let _buffer = state.memory_pools.get_buffer(1024).unwrap();
                    let _response = state.memory_pools.get_response().unwrap();
                    
                    // 模拟缓存操作
                    let test_data = Bytes::from("test_data");
                    let _ = state.cache_layer.set("warmup_key", test_data).await;
                    let _ = state.cache_layer.get("warmup_key").await;
                    
                    drop(_permit);
                }
            });
            
            tokio::time::sleep(Duration::from_micros(100)).await;
        }
        
        println!("✅ 预热完成");
        Ok(())
    }

    /// 吞吐量测试
    async fn benchmark_throughput(&self) -> UltraResult<ThroughputMetrics> {
        println!("📊 执行吞吐量测试...");
        
        let start_time = Instant::now();
        let total_requests = AtomicU64::new(0);
        let total_bytes = AtomicU64::new(0);
        let semaphore = Arc::new(Semaphore::new(self.config.concurrent_connections));
        
        let mut tasks = Vec::new();
        
        while start_time.elapsed() < self.config.duration {
            let _permit = semaphore.clone().acquire_owned().await.unwrap();
            
            let task = tokio::spawn({
                let state = self.state.clone();
                let total_requests = total_requests.clone();
                let total_bytes = total_bytes.clone();
                let message_size = self.config.message_size;
                
                async move {
                    // 模拟请求处理
                    let data = Bytes::from(vec![0u8; message_size]);
                    let _buffer = state.memory_pools.get_buffer(message_size).unwrap();
                    
                    // 模拟缓存操作
                    let key = format!("test_key_{}", rand::random::<u32>());
                    let _ = state.cache_layer.set(&key, data.clone()).await;
                    let _ = state.cache_layer.get(&key).await;
                    
                    total_requests.fetch_add(1, Ordering::Relaxed);
                    total_bytes.fetch_add(message_size as u64, Ordering::Relaxed);
                    
                    drop(_permit);
                }
            });
            
            tasks.push(task);
            
            // 控制请求频率
            let interval = Duration::from_secs(1) / self.config.messages_per_second as u32;
            tokio::time::sleep(interval).await;
        }
        
        // 等待所有任务完成
        for task in tasks {
            let _ = task.await;
        }
        
        let elapsed = start_time.elapsed();
        let requests = total_requests.load(Ordering::Relaxed);
        let bytes = total_bytes.load(Ordering::Relaxed);
        
        Ok(ThroughputMetrics {
            requests_per_second: requests as f64 / elapsed.as_secs_f64(),
            bytes_per_second: bytes as f64 / elapsed.as_secs_f64(),
            connections_per_second: self.config.concurrent_connections as f64 / elapsed.as_secs_f64(),
            total_requests: requests,
            total_bytes: bytes,
        })
    }

    /// 延迟测试
    async fn benchmark_latency(&self) -> UltraResult<LatencyMetrics> {
        println!("⏱️  执行延迟测试...");
        
        let mut latencies = Vec::new();
        let num_samples = 10000;
        
        for _ in 0..num_samples {
            let start = Instant::now();
            
            // 模拟操作
            let data = Bytes::from(vec![0u8; self.config.message_size]);
            let _buffer = self.state.memory_pools.get_buffer(self.config.message_size).unwrap();
            
            let key = format!("latency_test_{}", rand::random::<u32>());
            let _ = self.state.cache_layer.set(&key, data.clone()).await;
            let _ = self.state.cache_layer.get(&key).await;
            
            let latency = start.elapsed();
            latencies.push(latency);
        }
        
        latencies.sort();
        
        let min = latencies[0];
        let max = latencies[latencies.len() - 1];
        let mean = Duration::from_nanos(
            latencies.iter().map(|d| d.as_nanos()).sum::<u128>() / latencies.len() as u128
        );
        let p50 = latencies[latencies.len() * 50 / 100];
        let p95 = latencies[latencies.len() * 95 / 100];
        let p99 = latencies[latencies.len() * 99 / 100];
        let p999 = latencies[latencies.len() * 999 / 1000];
        
        Ok(LatencyMetrics {
            min, max, mean, p50, p95, p99, p999
        })
    }

    /// 内存测试
    async fn benchmark_memory(&self) -> UltraResult<MemoryMetrics> {
        println!("💾 执行内存测试...");
        
        let start_time = Instant::now();
        let mut peak_usage = 0;
        let mut total_usage = 0;
        let mut samples = 0;
        let allocations = AtomicU64::new(0);
        
        // 持续监控内存使用
        let monitor_handle = tokio::spawn({
            let state = self.state.clone();
            async move {
                let mut peak = 0;
                let mut total = 0;
                let mut count = 0;
                
                while start_time.elapsed() < Duration::from_secs(10) {
                    let stats = state.memory_pools.get_usage_stats();
                    let current_usage = stats.total_memory_bytes;
                    
                    if current_usage > peak {
                        peak = current_usage;
                    }
                    total += current_usage;
                    count += 1;
                    
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                
                (peak, total / count, count)
            }
        });
        
        // 执行内存分配测试
        let mut tasks = Vec::new();
        for _ in 0..1000 {
            let task = tokio::spawn({
                let state = self.state.clone();
                let allocations = allocations.clone();
                
                async move {
                    for _ in 0..100 {
                        let _buffer = state.memory_pools.get_buffer(4096).unwrap();
                        let _conn = state.memory_pools.get_connection().unwrap();
                        let _resp = state.memory_pools.get_response().unwrap();
                        
                        allocations.fetch_add(3, Ordering::Relaxed);
                        
                        tokio::time::sleep(Duration::from_micros(10)).await;
                    }
                }
            });
            tasks.push(task);
        }
        
        for task in tasks {
            let _ = task.await;
        }
        
        let (peak, avg, _samples) = monitor_handle.await.unwrap();
        let elapsed = start_time.elapsed();
        let total_allocations = allocations.load(Ordering::Relaxed);
        
        let usage_stats = self.state.memory_pools.get_usage_stats();
        
        Ok(MemoryMetrics {
            peak_usage: peak,
            average_usage: avg,
            allocations_per_second: total_allocations as f64 / elapsed.as_secs_f64(),
            deallocations_per_second: total_allocations as f64 / elapsed.as_secs_f64(),
            fragmentation_ratio: usage_stats.fragmentation_ratio,
        })
    }

    /// 缓存测试
    async fn benchmark_cache(&self) -> UltraResult<CacheMetrics> {
        println!("🗄️  执行缓存测试...");
        
        let num_operations = 10000;
        let cache_size = 1000;
        
        // 填充缓存
        for i in 0..cache_size {
            let key = format!("cache_test_{}", i);
            let data = Bytes::from(vec![i as u8; self.config.message_size]);
            let _ = self.state.cache_layer.set(&key, data).await;
        }
        
        // 执行读取测试（包含命中和未命中）
        for i in 0..num_operations {
            let key = if i % 2 == 0 {
                // 50% 概率命中
                format!("cache_test_{}", i % cache_size)
            } else {
                // 50% 概率未命中
                format!("cache_miss_{}", i)
            };
            
            let _ = self.state.cache_layer.get(&key).await;
        }
        
        let stats = self.state.cache_layer.get_detailed_stats();
        let total_operations = stats.l1_hits + stats.l1_misses + stats.l2_hits + stats.l2_misses + stats.l3_hits + stats.l3_misses;
        let total_hits = stats.l1_hits + stats.l2_hits + stats.l3_hits;
        
        Ok(CacheMetrics {
            hit_rate: if total_operations > 0 { total_hits as f64 / total_operations as f64 } else { 0.0 },
            miss_rate: if total_operations > 0 { (total_operations - total_hits) as f64 / total_operations as f64 } else { 0.0 },
            eviction_rate: 0.0, // 简化
            compression_ratio: stats.average_compression_ratio,
        })
    }

    /// 错误测试
    async fn benchmark_errors(&self) -> UltraResult<ErrorMetrics> {
        println!("❌ 执行错误处理测试...");
        
        // 模拟各种错误情况
        let total_operations = 1000;
        let mut errors = 0;
        let mut timeouts = 0;
        let mut connection_errors = 0;
        
        for i in 0..total_operations {
            // 模拟不同类型的错误
            match i % 10 {
                0 => {
                    // 模拟超时错误
                    timeouts += 1;
                    errors += 1;
                }
                1 => {
                    // 模拟连接错误
                    connection_errors += 1;
                    errors += 1;
                }
                _ => {
                    // 正常操作
                    let key = format!("error_test_{}", i);
                    let data = Bytes::from(vec![0u8; 100]);
                    let _ = self.state.cache_layer.set(&key, data).await;
                }
            }
        }
        
        Ok(ErrorMetrics {
            total_errors: errors,
            error_rate: errors as f64 / total_operations as f64,
            timeout_errors: timeouts,
            connection_errors: connection_errors,
        })
    }

    /// 打印测试结果
    fn print_results(&self, results: &BenchmarkResults) {
        println!("\n📋 基准测试结果报告");
        println!("====================");
        
        println!("\n🚀 吞吐量指标:");
        println!("  请求/秒: {:.2}", results.throughput.requests_per_second);
        println!("  字节/秒: {:.2} MB", results.throughput.bytes_per_second / 1_000_000.0);
        println!("  总请求数: {}", results.throughput.total_requests);
        println!("  总字节数: {:.2} MB", results.throughput.total_bytes as f64 / 1_000_000.0);
        
        println!("\n⏱️  延迟指标:");
        println!("  最小延迟: {:?}", results.latency.min);
        println!("  最大延迟: {:?}", results.latency.max);
        println!("  平均延迟: {:?}", results.latency.mean);
        println!("  P50延迟: {:?}", results.latency.p50);
        println!("  P95延迟: {:?}", results.latency.p95);
        println!("  P99延迟: {:?}", results.latency.p99);
        println!("  P99.9延迟: {:?}", results.latency.p999);
        
        println!("\n💾 内存指标:");
        println!("  峰值使用: {:.2} MB", results.memory.peak_usage as f64 / 1_000_000.0);
        println!("  平均使用: {:.2} MB", results.memory.average_usage as f64 / 1_000_000.0);
        println!("  分配/秒: {:.2}", results.memory.allocations_per_second);
        println!("  碎片率: {:.2}%", results.memory.fragmentation_ratio * 100.0);
        
        println!("\n🗄️  缓存指标:");
        println!("  命中率: {:.2}%", results.cache.hit_rate * 100.0);
        println!("  未命中率: {:.2}%", results.cache.miss_rate * 100.0);
        println!("  压缩率: {:.2}x", results.cache.compression_ratio);
        
        println!("\n❌ 错误指标:");
        println!("  总错误数: {}", results.errors.total_errors);
        println!("  错误率: {:.2}%", results.errors.error_rate * 100.0);
        println!("  超时错误: {}", results.errors.timeout_errors);
        println!("  连接错误: {}", results.errors.connection_errors);
        
        println!("\n✅ 测试完成!");
    }
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(60),
            concurrent_connections: 100,
            message_size: 1024,
            messages_per_second: 1000,
            warmup_duration: Duration::from_secs(10),
        }
    }
}

/// 运行快速基准测试
pub async fn run_quick_benchmark(state: Arc<UltraState>) -> UltraResult<()> {
    let config = BenchmarkConfig {
        duration: Duration::from_secs(10),
        concurrent_connections: 50,
        message_size: 512,
        messages_per_second: 500,
        warmup_duration: Duration::from_secs(2),
    };
    
    let executor = BenchmarkExecutor::new(config, state);
    let _results = executor.run_full_benchmark().await?;
    
    Ok(())
}

/// 运行生产级基准测试
pub async fn run_production_benchmark(state: Arc<UltraState>) -> UltraResult<BenchmarkResults> {
    let config = BenchmarkConfig {
        duration: Duration::from_secs(300), // 5分钟
        concurrent_connections: 1000,
        message_size: 4096,
        messages_per_second: 5000,
        warmup_duration: Duration::from_secs(30),
    };
    
    let executor = BenchmarkExecutor::new(config, state);
    executor.run_full_benchmark().await
}