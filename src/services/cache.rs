use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

struct CachedValue<V> {
    value: V,
    expires_at: Instant,
}

pub struct TtlCache<K, V> {
    state: Mutex<HashMap<K, CachedValue<V>>>,
}

impl<K, V> TtlCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    pub fn new() -> Self {
        Self {
            state: Mutex::new(HashMap::new()),
        }
    }

    pub async fn get(&self, key: &K) -> Option<V> {
        let mut state = self.state.lock().await;
        match state.get(key) {
            Some(cached) if cached.expires_at > Instant::now() => Some(cached.value.clone()),
            Some(_) => {
                state.remove(key);
                None
            }
            None => None,
        }
    }

    pub async fn set(&self, key: K, value: V, ttl: Duration) {
        let mut state = self.state.lock().await;
        state.insert(
            key,
            CachedValue {
                value,
                expires_at: Instant::now() + ttl,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::TtlCache;
    use std::time::Duration;

    #[tokio::test]
    async fn returns_value_before_ttl_expires() {
        let cache = TtlCache::<String, i32>::new();
        let key = "k1".to_string();

        cache.set(key.clone(), 10, Duration::from_millis(100)).await;

        let value = cache.get(&key).await;
        assert_eq!(value, Some(10));
    }

    #[tokio::test]
    async fn returns_none_after_ttl_expires() {
        let cache = TtlCache::<String, i32>::new();
        let key = "k1".to_string();

        cache.set(key.clone(), 10, Duration::from_millis(10)).await;
        tokio::time::sleep(Duration::from_millis(20)).await;

        let value = cache.get(&key).await;
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn overwrites_existing_key_with_new_value_and_ttl() {
        let cache = TtlCache::<String, i32>::new();
        let key = "k1".to_string();

        cache.set(key.clone(), 10, Duration::from_millis(50)).await;
        cache.set(key.clone(), 20, Duration::from_millis(200)).await;
        tokio::time::sleep(Duration::from_millis(80)).await;

        let value = cache.get(&key).await;
        assert_eq!(value, Some(20));
    }
}
