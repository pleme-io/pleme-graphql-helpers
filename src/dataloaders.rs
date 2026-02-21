//! DataLoader utilities for batch loading
///
/// Implements the DataLoader pattern for preventing N+1 query problems.
/// See: https://github.com/graphql/dataloader

use async_trait::async_trait;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Batch loader trait for loading multiple items at once
#[async_trait]
pub trait BatchLoader<K, V>: Send + Sync
where
    K: Send + Sync + Clone + Eq + Hash,
    V: Send + Sync + Clone,
{
    /// Load batch of items by keys
    ///
    /// This method should fetch all items for the given keys in a single
    /// database query or API call to avoid N+1 problems.
    async fn load_batch(&self, keys: &[K]) -> HashMap<K, V>;
}

/// DataLoader with caching and batching
///
/// Automatically batches requests within a single GraphQL query and caches
/// results to prevent duplicate loads.
pub struct DataLoader<K, V, L>
where
    K: Send + Sync + Clone + Eq + Hash + 'static,
    V: Send + Sync + Clone + 'static,
    L: BatchLoader<K, V> + 'static,
{
    loader: Arc<L>,
    cache: Arc<Mutex<HashMap<K, V>>>,
}

impl<K, V, L> DataLoader<K, V, L>
where
    K: Send + Sync + Clone + Eq + Hash + 'static,
    V: Send + Sync + Clone + 'static,
    L: BatchLoader<K, V> + 'static,
{
    /// Create new DataLoader with a batch loader
    pub fn new(loader: L) -> Self {
        Self {
            loader: Arc::new(loader),
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Load a single item by key
    ///
    /// Checks cache first, then falls back to batch loading if needed.
    pub async fn load(&self, key: K) -> Option<V> {
        // Check cache first
        {
            let cache = self.cache.lock().await;
            if let Some(value) = cache.get(&key) {
                return Some(value.clone());
            }
        }

        // Cache miss - load from batch loader
        let keys = vec![key.clone()];
        let results = self.loader.load_batch(&keys).await;

        // Update cache
        {
            let mut cache = self.cache.lock().await;
            for (k, v) in results.iter() {
                cache.insert(k.clone(), v.clone());
            }
        }

        results.get(&key).cloned()
    }

    /// Load multiple items by keys
    ///
    /// Batches keys that aren't in cache and loads them together.
    pub async fn load_many(&self, keys: Vec<K>) -> HashMap<K, V> {
        let mut result = HashMap::new();
        let mut uncached_keys = Vec::new();

        // Check cache for each key
        {
            let cache = self.cache.lock().await;
            for key in keys {
                if let Some(value) = cache.get(&key) {
                    result.insert(key, value.clone());
                } else {
                    uncached_keys.push(key);
                }
            }
        }

        // Load uncached keys in batch
        if !uncached_keys.is_empty() {
            let batch_results = self.loader.load_batch(&uncached_keys).await;

            // Update cache and result
            {
                let mut cache = self.cache.lock().await;
                for (k, v) in batch_results.iter() {
                    cache.insert(k.clone(), v.clone());
                    result.insert(k.clone(), v.clone());
                }
            }
        }

        result
    }

    /// Clear the cache
    pub async fn clear(&self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
    }

    /// Prime the cache with a value
    ///
    /// Useful for seeding the cache with data you already have.
    pub async fn prime(&self, key: K, value: V) {
        let mut cache = self.cache.lock().await;
        cache.insert(key, value);
    }
}

impl<K, V, L> Clone for DataLoader<K, V, L>
where
    K: Send + Sync + Clone + Eq + Hash + 'static,
    V: Send + Sync + Clone + 'static,
    L: BatchLoader<K, V> + 'static,
{
    fn clone(&self) -> Self {
        Self {
            loader: self.loader.clone(),
            cache: self.cache.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestLoader;

    #[async_trait]
    impl BatchLoader<String, String> for TestLoader {
        async fn load_batch(&self, keys: &[String]) -> HashMap<String, String> {
            keys.iter()
                .map(|k| (k.clone(), format!("value-{}", k)))
                .collect()
        }
    }

    #[tokio::test]
    async fn test_dataloader_single_load() {
        let loader = DataLoader::new(TestLoader);
        let value = loader.load("key1".to_string()).await;
        assert_eq!(value, Some("value-key1".to_string()));
    }

    #[tokio::test]
    async fn test_dataloader_caching() {
        let loader = DataLoader::new(TestLoader);

        // First load
        let value1 = loader.load("key1".to_string()).await;
        assert_eq!(value1, Some("value-key1".to_string()));

        // Second load should hit cache
        let value2 = loader.load("key1".to_string()).await;
        assert_eq!(value2, Some("value-key1".to_string()));
    }

    #[tokio::test]
    async fn test_dataloader_batch_load() {
        let loader = DataLoader::new(TestLoader);

        let keys = vec!["key1".to_string(), "key2".to_string(), "key3".to_string()];
        let results = loader.load_many(keys).await;

        assert_eq!(results.len(), 3);
        assert_eq!(results.get("key1"), Some(&"value-key1".to_string()));
        assert_eq!(results.get("key2"), Some(&"value-key2".to_string()));
        assert_eq!(results.get("key3"), Some(&"value-key3".to_string()));
    }

    #[tokio::test]
    async fn test_dataloader_prime() {
        let loader = DataLoader::new(TestLoader);

        // Prime cache with value
        loader.prime("key1".to_string(), "custom-value".to_string()).await;

        // Load should return primed value
        let value = loader.load("key1".to_string()).await;
        assert_eq!(value, Some("custom-value".to_string()));
    }

    #[tokio::test]
    async fn test_dataloader_clear() {
        let loader = DataLoader::new(TestLoader);

        // Load and cache a value
        loader.load("key1".to_string()).await;

        // Clear cache
        loader.clear().await;

        // Next load should fetch again (but we can't verify that without instrumentation)
        let value = loader.load("key1".to_string()).await;
        assert_eq!(value, Some("value-key1".to_string()));
    }
}
