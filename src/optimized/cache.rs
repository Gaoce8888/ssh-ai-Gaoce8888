use std::sync::Arc;
use lru::LruCache;
use parking_lot::Mutex;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

#[derive(Clone)]
pub struct ResponseCache {
    cache: Arc<Mutex<LruCache<u64, CachedResponse>>>,
    ttl: Duration,
}

#[derive(Clone)]
struct CachedResponse {
    data: serde_json::Value,
    created_at: Instant,
}

impl ResponseCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(capacity).unwrap()
            ))),
            ttl: Duration::from_secs(300), // 5分钟TTL
        }
    }

    pub async fn get(&self, key: &str) -> Option<serde_json::Value> {
        let hash = self.hash_key(key);
        let mut cache = self.cache.lock();
        
        if let Some(cached) = cache.get(&hash) {
            if cached.created_at.elapsed() < self.ttl {
                return Some(cached.data.clone());
            } else {
                cache.pop(&hash);
            }
        }
        
        None
    }

    pub async fn set(&self, key: String, value: serde_json::Value) {
        let hash = self.hash_key(&key);
        let cached = CachedResponse {
            data: value,
            created_at: Instant::now(),
        };
        
        self.cache.lock().put(hash, cached);
    }

    pub fn clear(&self) {
        self.cache.lock().clear();
    }

    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.lock();
        CacheStats {
            size: cache.len(),
            capacity: cache.cap().get(),
        }
    }

    fn hash_key(&self, key: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
}

// AI响应缓存的特殊实现
pub struct AIResponseCache {
    cache: ResponseCache,
}

impl AIResponseCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: ResponseCache::new(capacity),
        }
    }

    pub async fn get_ai_response(&self, prompt: &str, context: &str) -> Option<String> {
        let key = format!("{}:{}", prompt, context);
        self.cache.get(&key).await
            .and_then(|v| v.as_str().map(String::from))
    }

    pub async fn set_ai_response(&self, prompt: String, context: String, response: String) {
        let key = format!("{}:{}", prompt, context);
        self.cache.set(key, serde_json::Value::String(response)).await;
    }
}
