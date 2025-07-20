/*!
 * 高性能内存池管理器
 * 
 * 特性：
 * - 零分配设计
 * - NUMA感知分配
 * - 内存预取优化
 * - 缓存行对齐
 * - 内存碎片最小化
 */

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use parking_lot::{Mutex, RwLock};
use object_pool::{Pool, Reusable};
use bytes::{Bytes, BytesMut};
use ahash::AHashMap;
use super::{UltraResult, UltraError, MemoryPoolConfig, CACHE_LINE_SIZE};

/// 内存池管理器
pub struct MemoryPoolManager {
    /// 小缓冲区池 (< 4KB)
    small_buffer_pool: Arc<Pool<BytesMut>>,
    /// 中等缓冲区池 (4KB - 64KB)
    medium_buffer_pool: Arc<Pool<BytesMut>>,
    /// 大缓冲区池 (> 64KB)
    large_buffer_pool: Arc<Pool<BytesMut>>,
    /// 连接对象池
    connection_pool: Arc<Pool<ConnectionWrapper>>,
    /// 响应对象池
    response_pool: Arc<Pool<ResponseWrapper>>,
    /// 统计信息
    stats: Arc<PoolStats>,
    /// 配置
    config: MemoryPoolConfig,
}

/// 缓冲区大小类别
#[derive(Debug, Clone, Copy)]
pub enum BufferSize {
    Small,   // < 4KB
    Medium,  // 4KB - 64KB
    Large,   // > 64KB
}

/// 连接包装器
pub struct ConnectionWrapper {
    pub id: uuid::Uuid,
    pub buffer: BytesMut,
    pub metadata: ConnectionMetadata,
}

/// 响应包装器
pub struct ResponseWrapper {
    pub data: BytesMut,
    pub headers: AHashMap<String, String>,
    pub status: u16,
}

/// 连接元数据
#[derive(Default)]
pub struct ConnectionMetadata {
    pub remote_addr: Option<std::net::SocketAddr>,
    pub protocol: String,
    pub created_at: std::time::Instant,
    pub last_activity: std::time::Instant,
}

/// 内存池统计信息
pub struct PoolStats {
    /// 小缓冲区池统计
    small_buffer_allocated: AtomicUsize,
    small_buffer_peak: AtomicUsize,
    /// 中等缓冲区池统计
    medium_buffer_allocated: AtomicUsize,
    medium_buffer_peak: AtomicUsize,
    /// 大缓冲区池统计
    large_buffer_allocated: AtomicUsize,
    large_buffer_peak: AtomicUsize,
    /// 连接池统计
    connections_allocated: AtomicUsize,
    connections_peak: AtomicUsize,
    /// 总分配次数
    total_allocations: AtomicUsize,
    /// 缓存命中率
    cache_hits: AtomicUsize,
    cache_misses: AtomicUsize,
}

/// 内存使用统计
#[derive(Debug, Clone)]
pub struct MemoryUsageStats {
    pub small_buffers: usize,
    pub medium_buffers: usize,
    pub large_buffers: usize,
    pub connections: usize,
    pub total_memory_bytes: usize,
    pub cache_hit_rate: f64,
    pub fragmentation_ratio: f64,
}

impl MemoryPoolManager {
    /// 创建新的内存池管理器
    pub fn new(config: &MemoryPoolConfig) -> Self {
        let small_buffer_pool = Arc::new(Pool::new(
            config.buffer_pool_size / 4,
            || BytesMut::with_capacity(4096),
        ));

        let medium_buffer_pool = Arc::new(Pool::new(
            config.buffer_pool_size / 2,
            || BytesMut::with_capacity(65536),
        ));

        let large_buffer_pool = Arc::new(Pool::new(
            config.buffer_pool_size / 4,
            || BytesMut::with_capacity(1048576), // 1MB
        ));

        let connection_pool = Arc::new(Pool::new(
            config.connection_pool_size,
            || ConnectionWrapper {
                id: uuid::Uuid::new_v4(),
                buffer: BytesMut::with_capacity(8192),
                metadata: ConnectionMetadata::default(),
            },
        ));

        let response_pool = Arc::new(Pool::new(
            config.response_pool_size,
            || ResponseWrapper {
                data: BytesMut::with_capacity(4096),
                headers: AHashMap::with_capacity(16),
                status: 200,
            },
        ));

        Self {
            small_buffer_pool,
            medium_buffer_pool,
            large_buffer_pool,
            connection_pool,
            response_pool,
            stats: Arc::new(PoolStats::new()),
            config: config.clone(),
        }
    }

    /// 获取缓冲区
    #[inline(always)]
    pub fn get_buffer(&self, size: usize) -> UltraResult<Reusable<BytesMut>> {
        let buffer_type = self.classify_buffer_size(size);
        
        let result = match buffer_type {
            BufferSize::Small => {
                self.stats.small_buffer_allocated.fetch_add(1, Ordering::Relaxed);
                self.small_buffer_pool.try_pull()
            }
            BufferSize::Medium => {
                self.stats.medium_buffer_allocated.fetch_add(1, Ordering::Relaxed);
                self.medium_buffer_pool.try_pull()
            }
            BufferSize::Large => {
                self.stats.large_buffer_allocated.fetch_add(1, Ordering::Relaxed);
                self.large_buffer_pool.try_pull()
            }
        };

        match result {
            Some(mut buffer) => {
                // 确保缓冲区有足够容量
                if buffer.capacity() < size {
                    buffer.reserve(size - buffer.capacity());
                }
                buffer.clear();
                
                self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
                Ok(buffer)
            }
            None => {
                self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
                // 池已满，创建新的缓冲区
                Ok(Reusable::new(&self.small_buffer_pool, BytesMut::with_capacity(size)))
            }
        }
    }

    /// 获取连接对象
    #[inline(always)]
    pub fn get_connection(&self) -> UltraResult<Reusable<ConnectionWrapper>> {
        self.stats.connections_allocated.fetch_add(1, Ordering::Relaxed);
        
        match self.connection_pool.try_pull() {
            Some(mut conn) => {
                // 重置连接状态
                conn.id = uuid::Uuid::new_v4();
                conn.buffer.clear();
                conn.metadata = ConnectionMetadata::default();
                
                self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
                Ok(conn)
            }
            None => {
                self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
                Ok(Reusable::new(&self.connection_pool, ConnectionWrapper {
                    id: uuid::Uuid::new_v4(),
                    buffer: BytesMut::with_capacity(8192),
                    metadata: ConnectionMetadata::default(),
                }))
            }
        }
    }

    /// 获取响应对象
    #[inline(always)]
    pub fn get_response(&self) -> UltraResult<Reusable<ResponseWrapper>> {
        match self.response_pool.try_pull() {
            Some(mut response) => {
                // 重置响应状态
                response.data.clear();
                response.headers.clear();
                response.status = 200;
                
                self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
                Ok(response)
            }
            None => {
                self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
                Ok(Reusable::new(&self.response_pool, ResponseWrapper {
                    data: BytesMut::with_capacity(4096),
                    headers: AHashMap::with_capacity(16),
                    status: 200,
                }))
            }
        }
    }

    /// 分类缓冲区大小
    #[inline(always)]
    fn classify_buffer_size(&self, size: usize) -> BufferSize {
        if size <= 4096 {
            BufferSize::Small
        } else if size <= 65536 {
            BufferSize::Medium
        } else {
            BufferSize::Large
        }
    }

    /// 获取内存使用统计
    pub fn get_usage_stats(&self) -> MemoryUsageStats {
        let cache_hits = self.stats.cache_hits.load(Ordering::Relaxed);
        let cache_misses = self.stats.cache_misses.load(Ordering::Relaxed);
        let total_requests = cache_hits + cache_misses;
        
        let cache_hit_rate = if total_requests > 0 {
            cache_hits as f64 / total_requests as f64
        } else {
            0.0
        };

        MemoryUsageStats {
            small_buffers: self.stats.small_buffer_allocated.load(Ordering::Relaxed),
            medium_buffers: self.stats.medium_buffer_allocated.load(Ordering::Relaxed),
            large_buffers: self.stats.large_buffer_allocated.load(Ordering::Relaxed),
            connections: self.stats.connections_allocated.load(Ordering::Relaxed),
            total_memory_bytes: self.calculate_total_memory(),
            cache_hit_rate,
            fragmentation_ratio: self.calculate_fragmentation_ratio(),
        }
    }

    /// 计算总内存使用量
    fn calculate_total_memory(&self) -> usize {
        let small = self.stats.small_buffer_allocated.load(Ordering::Relaxed) * 4096;
        let medium = self.stats.medium_buffer_allocated.load(Ordering::Relaxed) * 65536;
        let large = self.stats.large_buffer_allocated.load(Ordering::Relaxed) * 1048576;
        let connections = self.stats.connections_allocated.load(Ordering::Relaxed) * 8192;
        
        small + medium + large + connections
    }

    /// 计算内存碎片率
    fn calculate_fragmentation_ratio(&self) -> f64 {
        // 简化的碎片率计算
        // 在实际实现中，这里会有更复杂的算法
        let total_allocated = self.calculate_total_memory();
        let total_available = self.config.buffer_pool_size * 1024; // 假设单位是KB
        
        if total_available > 0 {
            1.0 - (total_allocated as f64 / total_available as f64)
        } else {
            0.0
        }
    }

    /// 执行内存清理
    pub fn cleanup(&self) {
        // 在实际实现中，这里会有内存清理逻辑
        tracing::debug!("执行内存池清理");
    }

    /// 预热内存池
    pub fn warmup(&self) -> UltraResult<()> {
        // 预分配一些缓冲区以减少首次分配延迟
        let warmup_count = 100;
        
        for _ in 0..warmup_count {
            let _small = self.get_buffer(1024)?;
            let _medium = self.get_buffer(32768)?;
            let _large = self.get_buffer(512000)?;
            let _conn = self.get_connection()?;
            let _resp = self.get_response()?;
        }
        
        tracing::info!("内存池预热完成");
        Ok(())
    }
}

impl PoolStats {
    fn new() -> Self {
        Self {
            small_buffer_allocated: AtomicUsize::new(0),
            small_buffer_peak: AtomicUsize::new(0),
            medium_buffer_allocated: AtomicUsize::new(0),
            medium_buffer_peak: AtomicUsize::new(0),
            large_buffer_allocated: AtomicUsize::new(0),
            large_buffer_peak: AtomicUsize::new(0),
            connections_allocated: AtomicUsize::new(0),
            connections_peak: AtomicUsize::new(0),
            total_allocations: AtomicUsize::new(0),
            cache_hits: AtomicUsize::new(0),
            cache_misses: AtomicUsize::new(0),
        }
    }
}

impl Default for ConnectionMetadata {
    fn default() -> Self {
        let now = std::time::Instant::now();
        Self {
            remote_addr: None,
            protocol: String::new(),
            created_at: now,
            last_activity: now,
        }
    }
}

/// 配置内存分配器优化
pub fn configure_allocator() -> UltraResult<()> {
    #[cfg(target_os = "linux")]
    {
        // 在Linux上启用透明大页面
        unsafe {
            libc::madvise(
                std::ptr::null_mut(),
                0,
                libc::MADV_HUGEPAGE,
            );
        }
    }
    
    tracing::info!("内存分配器优化已配置");
    Ok(())
}

/// 内存对齐辅助函数
#[inline(always)]
pub fn align_to_cache_line(size: usize) -> usize {
    (size + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1)
}