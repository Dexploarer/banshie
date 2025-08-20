use super::strategies::{CacheStrategy, TtlCache, LruCache, CacheStats, CacheError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, debug, warn};
use serde::{Serialize, Deserialize, de::DeserializeOwned};

/// Cache manager that handles multiple cache layers and strategies
pub struct CacheManager {
    // Layer 1: In-memory caches for frequently accessed data
    token_price_cache: Arc<dyn CacheStrategy<String, f64>>,
    balance_cache: Arc<dyn CacheStrategy<String, serde_json::Value>>,
    position_cache: Arc<dyn CacheStrategy<String, Vec<serde_json::Value>>>,
    
    // Layer 2: Quote caches with short TTL
    jupiter_quote_cache: Arc<dyn CacheStrategy<String, serde_json::Value>>,
    
    // Layer 3: User data caches
    user_rebate_cache: Arc<dyn CacheStrategy<String, serde_json::Value>>,
    
    // Global stats
    global_stats: Arc<RwLock<GlobalCacheStats>>,
}

#[derive(Debug, Clone, Default)]
pub struct GlobalCacheStats {
    pub total_hits: u64,
    pub total_misses: u64,
    pub total_entries: usize,
    pub global_hit_rate: f64,
    pub layers: HashMap<String, CacheStats>,
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub token_price_ttl: Duration,
    pub balance_ttl: Duration,
    pub position_ttl: Duration,
    pub quote_ttl: Duration,
    pub rebate_ttl: Duration,
    pub max_capacity: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            token_price_ttl: Duration::from_secs(30),  // Token prices change frequently
            balance_ttl: Duration::from_secs(10),      // Balance updates often
            position_ttl: Duration::from_secs(15),     // Positions change with trades
            quote_ttl: Duration::from_secs(5),         // Quotes are very short-lived
            rebate_ttl: Duration::from_secs(60),       // Rebate stats update less frequently
            max_capacity: 10000,                       // 10k entries per cache
        }
    }
}

impl CacheManager {
    pub fn new(config: CacheConfig) -> Self {
        info!("Initializing comprehensive cache manager with config: {:?}", config);
        
        // Create different cache strategies for different use cases
        let token_price_cache: Arc<dyn CacheStrategy<String, f64>> = Arc::new(
            TtlCache::new(config.max_capacity, config.token_price_ttl)
                .with_cleanup_interval(Duration::from_secs(30))
        );
        
        let balance_cache: Arc<dyn CacheStrategy<String, serde_json::Value>> = Arc::new(
            TtlCache::new(config.max_capacity / 2, config.balance_ttl)
                .with_cleanup_interval(Duration::from_secs(15))
        );
        
        let position_cache: Arc<dyn CacheStrategy<String, Vec<serde_json::Value>>> = Arc::new(
            TtlCache::new(config.max_capacity / 2, config.position_ttl)
                .with_cleanup_interval(Duration::from_secs(20))
        );
        
        let jupiter_quote_cache: Arc<dyn CacheStrategy<String, serde_json::Value>> = Arc::new(
            LruCache::new(config.max_capacity / 4) // Quotes are frequent but short-lived
        );
        
        let user_rebate_cache: Arc<dyn CacheStrategy<String, serde_json::Value>> = Arc::new(
            TtlCache::new(config.max_capacity / 4, config.rebate_ttl)
                .with_cleanup_interval(Duration::from_secs(60))
        );
        
        info!("Cache manager initialized with 5 specialized cache layers");\n        \n        Self {\n            token_price_cache,\n            balance_cache,\n            position_cache,\n            jupiter_quote_cache,\n            user_rebate_cache,\n            global_stats: Arc::new(RwLock::new(GlobalCacheStats::default())),\n        }\n    }\n    \n    /// Cache token price with optimized key\n    pub async fn cache_token_price(&self, token_mint: &str, price: f64) -> Result<(), CacheError> {\n        let key = format!(\"price:{}\", token_mint);\n        debug!(\"Caching token price: {} = ${:.8}\", token_mint, price);\n        self.token_price_cache.set(key, price).await\n    }\n    \n    /// Get cached token price\n    pub async fn get_token_price(&self, token_mint: &str) -> Option<f64> {\n        let key = format!(\"price:{}\", token_mint);\n        if let Some(price) = self.token_price_cache.get(&key).await {\n            debug!(\"Cache hit for token price: {} = ${:.8}\", token_mint, price);\n            Some(price)\n        } else {\n            debug!(\"Cache miss for token price: {}\", token_mint);\n            None\n        }\n    }\n    \n    /// Cache user balance\n    pub async fn cache_balance<T: Serialize>(&self, user_wallet: &str, balance: &T) -> Result<(), CacheError> {\n        let key = format!(\"balance:{}\", user_wallet);\n        let value = serde_json::to_value(balance)\n            .map_err(|e| CacheError::SerializationError(e.to_string()))?;\n        debug!(\"Caching balance for wallet: {}\", user_wallet);\n        self.balance_cache.set(key, value).await\n    }\n    \n    /// Get cached balance\n    pub async fn get_balance<T: DeserializeOwned>(&self, user_wallet: &str) -> Option<T> {\n        let key = format!(\"balance:{}\", user_wallet);\n        if let Some(value) = self.balance_cache.get(&key).await {\n            match serde_json::from_value(value) {\n                Ok(balance) => {\n                    debug!(\"Cache hit for balance: {}\", user_wallet);\n                    Some(balance)\n                }\n                Err(e) => {\n                    warn!(\"Failed to deserialize cached balance: {}\", e);\n                    None\n                }\n            }\n        } else {\n            debug!(\"Cache miss for balance: {}\", user_wallet);\n            None\n        }\n    }\n    \n    /// Cache user positions\n    pub async fn cache_positions<T: Serialize>(&self, user_wallet: &str, positions: &[T]) -> Result<(), CacheError> {\n        let key = format!(\"positions:{}\", user_wallet);\n        let values: Result<Vec<serde_json::Value>, _> = positions.iter()\n            .map(|p| serde_json::to_value(p))\n            .collect();\n        let values = values.map_err(|e| CacheError::SerializationError(e.to_string()))?;\n        debug!(\"Caching {} positions for wallet: {}\", positions.len(), user_wallet);\n        self.position_cache.set(key, values).await\n    }\n    \n    /// Get cached positions\n    pub async fn get_positions<T: DeserializeOwned>(&self, user_wallet: &str) -> Option<Vec<T>> {\n        let key = format!(\"positions:{}\", user_wallet);\n        if let Some(values) = self.position_cache.get(&key).await {\n            let positions: Result<Vec<T>, _> = values.into_iter()\n                .map(|v| serde_json::from_value(v))\n                .collect();\n            match positions {\n                Ok(positions) => {\n                    debug!(\"Cache hit for {} positions: {}\", positions.len(), user_wallet);\n                    Some(positions)\n                }\n                Err(e) => {\n                    warn!(\"Failed to deserialize cached positions: {}\", e);\n                    None\n                }\n            }\n        } else {\n            debug!(\"Cache miss for positions: {}\", user_wallet);\n            None\n        }\n    }\n    \n    /// Cache Jupiter quote\n    pub async fn cache_jupiter_quote<T: Serialize>(\n        &self, \n        input_mint: &str, \n        output_mint: &str, \n        amount: u64, \n        slippage: u16,\n        quote: &T\n    ) -> Result<(), CacheError> {\n        let key = format!(\"quote:{}:{}:{}:{}\", input_mint, output_mint, amount, slippage);\n        let value = serde_json::to_value(quote)\n            .map_err(|e| CacheError::SerializationError(e.to_string()))?;\n        debug!(\"Caching Jupiter quote: {}\", key);\n        self.jupiter_quote_cache.set(key, value).await\n    }\n    \n    /// Get cached Jupiter quote\n    pub async fn get_jupiter_quote<T: DeserializeOwned>(\n        &self,\n        input_mint: &str,\n        output_mint: &str,\n        amount: u64,\n        slippage: u16\n    ) -> Option<T> {\n        let key = format!(\"quote:{}:{}:{}:{}\", input_mint, output_mint, amount, slippage);\n        if let Some(value) = self.jupiter_quote_cache.get(&key).await {\n            match serde_json::from_value(value) {\n                Ok(quote) => {\n                    debug!(\"Cache hit for Jupiter quote: {}\", key);\n                    Some(quote)\n                }\n                Err(e) => {\n                    warn!(\"Failed to deserialize cached quote: {}\", e);\n                    None\n                }\n            }\n        } else {\n            debug!(\"Cache miss for Jupiter quote: {}\", key);\n            None\n        }\n    }\n    \n    /// Cache user rebate stats\n    pub async fn cache_rebate_stats<T: Serialize>(&self, user_id: &str, stats: &T) -> Result<(), CacheError> {\n        let key = format!(\"rebate:{}\", user_id);\n        let value = serde_json::to_value(stats)\n            .map_err(|e| CacheError::SerializationError(e.to_string()))?;\n        debug!(\"Caching rebate stats for user: {}\", user_id);\n        self.user_rebate_cache.set(key, value).await\n    }\n    \n    /// Get cached rebate stats\n    pub async fn get_rebate_stats<T: DeserializeOwned>(&self, user_id: &str) -> Option<T> {\n        let key = format!(\"rebate:{}\", user_id);\n        if let Some(value) = self.user_rebate_cache.get(&key).await {\n            match serde_json::from_value(value) {\n                Ok(stats) => {\n                    debug!(\"Cache hit for rebate stats: {}\", user_id);\n                    Some(stats)\n                }\n                Err(e) => {\n                    warn!(\"Failed to deserialize cached rebate stats: {}\", e);\n                    None\n                }\n            }\n        } else {\n            debug!(\"Cache miss for rebate stats: {}\", user_id);\n            None\n        }\n    }\n    \n    /// Invalidate all caches for a user (after trade execution)\n    pub async fn invalidate_user_caches(&self, user_wallet: &str) {\n        let balance_key = format!(\"balance:{}\", user_wallet);\n        let positions_key = format!(\"positions:{}\", user_wallet);\n        \n        self.balance_cache.remove(&balance_key).await;\n        self.position_cache.remove(&positions_key).await;\n        \n        info!(\"Invalidated user caches for wallet: {}\", user_wallet);\n    }\n    \n    /// Clear all caches (for maintenance or testing)\n    pub async fn clear_all(&self) {\n        self.token_price_cache.clear().await;\n        self.balance_cache.clear().await;\n        self.position_cache.clear().await;\n        self.jupiter_quote_cache.clear().await;\n        self.user_rebate_cache.clear().await;\n        \n        info!(\"All cache layers cleared\");\n    }\n    \n    /// Get comprehensive cache statistics\n    pub async fn get_global_stats(&self) -> GlobalCacheStats {\n        let mut global_stats = self.global_stats.write().await;\n        let mut layers = HashMap::new();\n        \n        // Collect stats from all cache layers\n        layers.insert(\"token_prices\".to_string(), self.token_price_cache.stats().await);\n        layers.insert(\"balances\".to_string(), self.balance_cache.stats().await);\n        layers.insert(\"positions\".to_string(), self.position_cache.stats().await);\n        layers.insert(\"jupiter_quotes\".to_string(), self.jupiter_quote_cache.stats().await);\n        layers.insert(\"user_rebates\".to_string(), self.user_rebate_cache.stats().await);\n        \n        // Calculate global statistics\n        let mut total_hits = 0;\n        let mut total_misses = 0;\n        let mut total_entries = 0;\n        \n        for stats in layers.values() {\n            total_hits += stats.hits;\n            total_misses += stats.misses;\n            total_entries += stats.entries;\n        }\n        \n        let global_hit_rate = if total_hits + total_misses > 0 {\n            total_hits as f64 / (total_hits + total_misses) as f64 * 100.0\n        } else {\n            0.0\n        };\n        \n        global_stats.total_hits = total_hits;\n        global_stats.total_misses = total_misses;\n        global_stats.total_entries = total_entries;\n        global_stats.global_hit_rate = global_hit_rate;\n        global_stats.layers = layers;\n        \n        global_stats.clone()\n    }\n    \n    /// Health check for all cache layers\n    pub async fn health_check(&self) -> CacheHealthReport {\n        let stats = self.get_global_stats().await;\n        let mut issues = Vec::new();\n        \n        // Check for low hit rates\n        for (layer_name, layer_stats) in &stats.layers {\n            if layer_stats.hit_rate < 50.0 && layer_stats.hits + layer_stats.misses > 100 {\n                issues.push(format!(\"Low hit rate in {} layer: {:.1}%\", layer_name, layer_stats.hit_rate));\n            }\n        }\n        \n        // Check for capacity issues\n        if stats.total_entries > 40000 {\n            issues.push(\"High cache utilization detected\".to_string());\n        }\n        \n        let health = if issues.is_empty() {\n            CacheHealth::Healthy\n        } else if issues.len() <= 2 {\n            CacheHealth::Warning\n        } else {\n            CacheHealth::Critical\n        };\n        \n        CacheHealthReport {\n            health,\n            stats,\n            issues,\n        }\n    }\n}\n\n#[derive(Debug, Clone)]\npub struct CacheHealthReport {\n    pub health: CacheHealth,\n    pub stats: GlobalCacheStats,\n    pub issues: Vec<String>,\n}\n\n#[derive(Debug, Clone, PartialEq)]\npub enum CacheHealth {\n    Healthy,\n    Warning,\n    Critical,\n}