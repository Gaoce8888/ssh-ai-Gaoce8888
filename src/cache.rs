use std::sync::Arc;
use std::time::{Duration, Instant};
use std::hash::Hash;
use lru::LruCache;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use std::num::NonZeroUsize;

/// 通用缓存条目
#[derive(Clone, Debug)]
struct CacheEntry<V> {
    value: V,
    created_at: Instant,
    last_accessed: Instant,
    access_count: u64,
}

/// 高性能缓存系统
pub struct Cache<K, V> 
where 
    K: Hash + Eq + Clone,
    V: Clone,
{
    inner: Arc<RwLock<LruCache<K, CacheEntry<V>>>>,
    ttl: Duration,
    max_size: usize,
}

impl<K, V> Cache<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            inner: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(1000).unwrap())
            ))),
            ttl,
            max_size: capacity,
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.inner.write();
        
        if let Some(entry) = cache.get_mut(key) {
            // 检查是否过期
            if entry.created_at.elapsed() > self.ttl {
                cache.pop(key);
                return None;
            }
            
            // 更新访问信息
            entry.last_accessed = Instant::now();
            entry.access_count += 1;
            
            Some(entry.value.clone())
        } else {
            None
        }
    }

    pub fn put(&self, key: K, value: V) {
        let entry = CacheEntry {
            value,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 0,
        };
        
        let mut cache = self.inner.write();
        cache.put(key, entry);
    }

    pub fn remove(&self, key: &K) -> Option<V> {
        let mut cache = self.inner.write();
        cache.pop(key).map(|entry| entry.value)
    }

    pub fn clear(&self) {
        let mut cache = self.inner.write();
        cache.clear();
    }

    pub fn len(&self) -> usize {
        let cache = self.inner.read();
        cache.len()
    }

    pub fn is_empty(&self) -> bool {
        let cache = self.inner.read();
        cache.is_empty()
    }

    /// 清理过期条目
    pub fn cleanup(&self) {
        let mut cache = self.inner.write();
        let now = Instant::now();
        
        // 收集过期的键
        let expired_keys: Vec<K> = cache
            .iter()
            .filter(|(_, entry)| now.duration_since(entry.created_at) > self.ttl)
            .map(|(k, _)| k.clone())
            .collect();
        
        // 删除过期条目
        for key in expired_keys {
            cache.pop(&key);
        }
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> CacheStats {
        let cache = self.inner.read();
        let total_entries = cache.len();
        let mut total_hits = 0u64;
        let mut oldest_entry = Instant::now();
        
        for (_, entry) in cache.iter() {
            total_hits += entry.access_count;
            if entry.created_at < oldest_entry {
                oldest_entry = entry.created_at;
            }
        }
        
        CacheStats {
            total_entries,
            total_hits,
            capacity: self.max_size,
            ttl_seconds: self.ttl.as_secs(),
            oldest_entry_age: oldest_entry.elapsed().as_secs(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_hits: u64,
    pub capacity: usize,
    pub ttl_seconds: u64,
    pub oldest_entry_age: u64,
}

/// AI响应缓存
pub type AIResponseCache = Cache<String, String>;

/// SSH命令结果缓存
pub type CommandCache = Cache<String, Vec<u8>>;

/// 创建AI响应缓存
pub fn create_ai_cache(capacity: usize) -> AIResponseCache {
    Cache::new(capacity, Duration::from_secs(3600)) // 1小时TTL
}

/// 创建命令结果缓存
pub fn create_command_cache(capacity: usize) -> CommandCache {
    Cache::new(capacity, Duration::from_secs(300)) // 5分钟TTL
}

/// 缓存键生成器
pub fn generate_cache_key(components: &[&str]) -> String {
    components.join(":")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic_operations() {
        let cache: Cache<String, String> = Cache::new(10, Duration::from_secs(60));
        
        // 测试插入和获取
        cache.put("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
        
        // 测试删除
        cache.remove(&"key1".to_string());
        assert_eq!(cache.get(&"key1".to_string()), None);
        
        // 测试清空
        cache.put("key2".to_string(), "value2".to_string());
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_lru_eviction() {
        let cache: Cache<i32, String> = Cache::new(2, Duration::from_secs(60));
        
        cache.put(1, "one".to_string());
        cache.put(2, "two".to_string());
        cache.put(3, "three".to_string()); // 应该驱逐 1
        
        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&2), Some("two".to_string()));
        assert_eq!(cache.get(&3), Some("three".to_string()));
    }
}