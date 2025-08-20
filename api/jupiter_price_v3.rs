use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use chrono::{DateTime, Utc, Duration};

use crate::errors::{BotError, Result};
use crate::api::jupiter_auth::{JupiterAuthManager, ApiTierLevel};

/// Jupiter Price API V3 client with enhanced caching
#[derive(Clone)]
pub struct JupiterPriceV3Client {
    client: Client,
    auth_manager: Arc<JupiterAuthManager>,
    base_url: String,
    price_cache: Arc<RwLock<PriceCache>>,
}

/// Enhanced price data with V3 features
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PriceDataV3 {
    #[serde(rename = "usdPrice")]
    pub usd_price: f64,
    #[serde(rename = "blockId")]
    pub block_id: u64,
    pub decimals: u8,
    #[serde(rename = "priceChange24h", skip_serializing_if = "Option::is_none")]
    pub price_change_24h: Option<f64>,
    #[serde(rename = "volume24h", skip_serializing_if = "Option::is_none")]
    pub volume_24h: Option<u64>,
    #[serde(rename = "lastTradedPrice", skip_serializing_if = "Option::is_none")]
    pub last_traded_price: Option<f64>,
    #[serde(rename = "lastTradedAt", skip_serializing_if = "Option::is_none")]
    pub last_traded_at: Option<DateTime<Utc>>,
}

/// Comprehensive price response
#[derive(Debug, Clone, Deserialize)]
pub struct PriceResponseV3 {
    #[serde(flatten)]
    pub prices: HashMap<String, PriceDataV3>,
    #[serde(rename = "timeTaken", skip_serializing_if = "Option::is_none")]
    pub time_taken: Option<f64>,
    #[serde(rename = "contextSlot", skip_serializing_if = "Option::is_none")]
    pub context_slot: Option<u64>,
}

/// Historical price data request
#[derive(Debug, Serialize)]
pub struct HistoricalPriceRequest {
    pub id: String,
    pub vs_token: Option<String>, // Default: USDC
    pub timeframe: Timeframe,
    pub limit: Option<u32>, // Max 1000
}

/// Available timeframes for historical data
#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Timeframe {
    #[serde(rename = "1m")]
    OneMinute,
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "15m")]
    FifteenMinutes,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "4h")]
    FourHours,
    #[serde(rename = "1d")]
    OneDay,
}

/// Historical price data point
#[derive(Debug, Deserialize)]
pub struct HistoricalPricePoint {
    pub timestamp: DateTime<Utc>,
    #[serde(rename = "priceUsd")]
    pub price_usd: f64,
    #[serde(rename = "volume24h")]
    pub volume_24h: Option<u64>,
}

/// Historical price response
#[derive(Debug, Deserialize)]
pub struct HistoricalPriceResponse {
    pub data: Vec<HistoricalPricePoint>,
    #[serde(rename = "timeTaken")]
    pub time_taken: Option<f64>,
}

/// Price comparison data
#[derive(Debug, Clone)]
pub struct PriceComparison {
    pub token_mint: String,
    pub current_price: f64,
    pub price_24h_ago: Option<f64>,
    pub price_change_24h: Option<f64>,
    pub price_change_percent_24h: Option<f64>,
    pub volume_24h: Option<u64>,
    pub last_updated: DateTime<Utc>,
}

/// Price alert configuration
#[derive(Debug, Clone, Serialize)]
pub struct PriceAlert {
    pub token_mint: String,
    pub user_id: i64,
    pub alert_type: AlertType,
    pub target_price: f64,
    pub current_price: f64,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub enum AlertType {
    Above,
    Below,
    PercentageChange { threshold_percent: f64 },
}

/// Price cache with intelligent invalidation
#[derive(Debug)]
struct PriceCache {
    prices: HashMap<String, CachedPrice>,
    last_cleanup: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct CachedPrice {
    data: PriceDataV3,
    cached_at: DateTime<Utc>,
    access_count: u32,
    tier_ttl: Duration,
}

impl JupiterPriceV3Client {
    /// Create new Price API V3 client
    pub fn new(auth_manager: Arc<JupiterAuthManager>) -> Self {
        info!("📈 Initializing Jupiter Price API V3 client");
        
        Self {
            client: Client::new(),
            auth_manager,
            base_url: "https://api.jup.ag".to_string(), // Will be updated based on tier
            price_cache: Arc::new(RwLock::new(PriceCache {
                prices: HashMap::new(),
                last_cleanup: Utc::now(),
            })),
        }
    }
    
    /// Get current prices for multiple tokens
    pub async fn get_prices(&self, token_mints: Vec<String>) -> Result<PriceResponseV3> {
        if token_mints.is_empty() {
            return Err(BotError::validation("Token mints cannot be empty".to_string()).into());
        }
        
        if token_mints.len() > 100 {
            return Err(BotError::validation("Maximum 100 tokens per request".to_string()).into());
        }
        
        // Check cache first
        let cache_results = self.check_price_cache(&token_mints).await;
        let uncached_tokens: Vec<String> = token_mints.iter()
            .filter(|mint| !cache_results.contains_key(*mint))
            .cloned()
            .collect();
        
        let mut final_prices = cache_results;
        
        // Fetch uncached prices
        if !uncached_tokens.is_empty() {
            let api_key_config = self.auth_manager.select_best_key("price").await?;
            let base_url = match &api_key_config {
                Some(config) => match config.tier {
                    ApiTierLevel::Lite => "https://lite-api.jup.ag".to_string(),
                    _ => "https://api.jup.ag".to_string(),
                },
                None => "https://lite-api.jup.ag".to_string(),
            };
            
            let url = format!("{}/price/v3", base_url);
            let ids = uncached_tokens.join(",");
            
            debug!("📈 Fetching prices for {} tokens from API", uncached_tokens.len());
            
            let mut request = self.client
                .get(&url)
                .query(&[("ids", &ids)]);
                
            // Add authentication if available
            if let Some(config) = &api_key_config {
                request = request.header("Authorization", format!("Bearer {}", config.key));
            }
            
            let response = request
                .send()
                .await
                .map_err(|e| BotError::jupiter_api(format!("Price request failed: {}", e)))?;
                
            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                return Err(BotError::jupiter_api(format!(
                    "Price API failed with status {}: {}", status, error_text
                )).into());
            }
            
            let api_response: PriceResponseV3 = response
                .json()
                .await
                .map_err(|e| BotError::jupiter_api(format!("Failed to parse price response: {}", e)))?;
                
            // Cache the results
            self.cache_prices(&api_response.prices, &api_key_config).await;
            
            // Record usage
            if let Some(config) = &api_key_config {
                let key_id = format!("key_{}", &config.key[..8]);
                self.auth_manager.record_usage(&key_id, "price").await;
            }
            
            final_prices.extend(api_response.prices);
        }
        
        info!("📈 Retrieved prices for {} tokens ({} from cache, {} from API)", 
            token_mints.len(), 
            token_mints.len() - uncached_tokens.len(),
            uncached_tokens.len()
        );
        
        Ok(PriceResponseV3 {
            prices: final_prices,
            time_taken: None,
            context_slot: None,
        })
    }
    
    /// Get historical price data
    pub async fn get_historical_prices(
        &self,
        request: HistoricalPriceRequest,
    ) -> Result<HistoricalPriceResponse> {
        let api_key_config = self.auth_manager.select_best_key("historical_price").await?;
        
        // Historical data requires Ultra tier or above
        if let Some(config) = &api_key_config {
            if matches!(config.tier, ApiTierLevel::Lite) {
                return Err(BotError::jupiter_api(
                    "Historical price data requires Ultra tier or above".to_string()
                ).into());
            }
        } else {
            return Err(BotError::jupiter_api(
                "Historical price data requires API authentication".to_string()
            ).into());
        }
        
        let base_url = "https://api.jup.ag";
        let url = format!("{}/price/v3/historical", base_url);
        
        let mut req = self.client
            .get(&url)
            .query(&request);
            
        if let Some(config) = &api_key_config {
            req = req.header("Authorization", format!("Bearer {}", config.key));
        }
        
        let response = req
            .send()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Historical price request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(BotError::jupiter_api(format!(
                "Historical price API failed with status {}: {}", status, error_text
            )).into());
        }
        
        let historical_data = response
            .json::<HistoricalPriceResponse>()
            .await
            .map_err(|e| BotError::jupiter_api(format!("Failed to parse historical price response: {}", e)))?;
            
        // Record usage
        if let Some(config) = &api_key_config {
            let key_id = format!("key_{}", &config.key[..8]);
            self.auth_manager.record_usage(&key_id, "historical_price").await;
        }
        
        debug!("📈 Retrieved {} historical price points for {}", 
            historical_data.data.len(), request.id);
        
        Ok(historical_data)
    }
    
    /// Get price comparison with 24h change
    pub async fn get_price_comparison(&self, token_mint: &str) -> Result<PriceComparison> {
        let current_price_resp = self.get_prices(vec![token_mint.to_string()]).await?;
        let current_data = current_price_resp.prices
            .get(token_mint)
            .ok_or_else(|| BotError::jupiter_api(format!("Price not found for token {}", token_mint)))?;
        
        let price_change_24h = current_data.price_change_24h;
        let price_change_percent_24h = price_change_24h.map(|change| {
            if current_data.usd_price > 0.0 {
                (change / (current_data.usd_price - change)) * 100.0
            } else {
                0.0
            }
        });
        
        let price_24h_ago = price_change_24h.map(|change| current_data.usd_price - change);
        
        Ok(PriceComparison {
            token_mint: token_mint.to_string(),
            current_price: current_data.usd_price,
            price_24h_ago,
            price_change_24h,
            price_change_percent_24h,
            volume_24h: current_data.volume_24h,
            last_updated: Utc::now(),
        })
    }
    
    /// Check if price alert should trigger
    pub fn should_trigger_alert(&self, alert: &PriceAlert, current_price: f64) -> bool {
        match alert.alert_type {
            AlertType::Above => current_price >= alert.target_price,
            AlertType::Below => current_price <= alert.target_price,
            AlertType::PercentageChange { threshold_percent } => {
                let change_percent = ((current_price - alert.current_price) / alert.current_price) * 100.0;
                change_percent.abs() >= threshold_percent
            }
        }
    }
    
    /// Check cache for prices
    async fn check_price_cache(&self, token_mints: &[String]) -> HashMap<String, PriceDataV3> {
        let mut cache = self.price_cache.write().await;
        let mut results = HashMap::new();
        
        for mint in token_mints {
            if let Some(cached) = cache.prices.get_mut(mint) {
                let age = Utc::now().signed_duration_since(cached.cached_at);
                
                if age < cached.tier_ttl {
                    cached.access_count += 1;
                    results.insert(mint.clone(), cached.data.clone());
                }
            }
        }
        
        // Periodic cleanup
        if Utc::now().signed_duration_since(cache.last_cleanup) > Duration::minutes(5) {
            self.cleanup_cache(&mut cache).await;
        }
        
        results
    }
    
    /// Cache price data with tier-appropriate TTL
    async fn cache_prices(
        &self,
        prices: &HashMap<String, PriceDataV3>,
        api_key_config: &Option<crate::api::jupiter_auth::ApiKeyConfig>,
    ) {
        let mut cache = self.price_cache.write().await;
        
        let tier_ttl = match api_key_config {
            Some(config) => match config.tier {
                ApiTierLevel::Lite => Duration::seconds(30),
                ApiTierLevel::Ultra => Duration::seconds(15),
                ApiTierLevel::Pro { .. } => Duration::seconds(5),
                ApiTierLevel::Enterprise { .. } => Duration::seconds(1),
            },
            None => Duration::seconds(30),
        };
        
        for (mint, price_data) in prices {
            cache.prices.insert(mint.clone(), CachedPrice {
                data: price_data.clone(),
                cached_at: Utc::now(),
                access_count: 1,
                tier_ttl,
            });
        }
    }
    
    /// Clean up expired cache entries
    async fn cleanup_cache(&self, cache: &mut PriceCache) {
        let now = Utc::now();
        let cutoff = now - Duration::minutes(10);
        
        let before_count = cache.prices.len();
        cache.prices.retain(|_, cached| {
            cached.cached_at > cutoff || cached.access_count > 5
        });
        
        cache.last_cleanup = now;
        
        let cleaned = before_count - cache.prices.len();
        if cleaned > 0 {
            debug!("🧹 Cleaned {} expired price cache entries", cleaned);
        }
    }
    
    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> CacheStats {
        let cache = self.price_cache.read().await;
        let total_entries = cache.prices.len();
        let avg_access_count = if total_entries > 0 {
            cache.prices.values().map(|c| c.access_count).sum::<u32>() as f64 / total_entries as f64
        } else {
            0.0
        };
        
        CacheStats {
            total_entries,
            avg_access_count,
            last_cleanup: cache.last_cleanup,
        }
    }
}

/// Cache statistics for monitoring
#[derive(Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub avg_access_count: f64,
    pub last_cleanup: DateTime<Utc>,
}