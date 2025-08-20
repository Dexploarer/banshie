use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use chrono::{DateTime, Utc, Duration};

use crate::errors::{BotError, Result};

/// Jupiter API authentication manager
#[derive(Clone)]
pub struct JupiterAuthManager {
    api_keys: RwLock<HashMap<String, ApiKeyConfig>>,
    usage_tracker: RwLock<UsageTracker>,
}

/// API key configuration with metadata
#[derive(Debug, Clone)]
pub struct ApiKeyConfig {
    pub key: String,
    pub tier: ApiTierLevel,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub daily_usage: u64,
    pub monthly_usage: u64,
    pub rate_limits: RateLimits,
    pub is_active: bool,
    pub description: Option<String>,
}

/// API tier levels with different capabilities
#[derive(Debug, Clone, PartialEq)]
pub enum ApiTierLevel {
    Lite,
    Ultra,
    Pro { credits_remaining: u64 },
    Enterprise { custom_limits: CustomLimits },
}

/// Rate limits for different tiers
#[derive(Debug, Clone)]
pub struct RateLimits {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
    pub concurrent_requests: u32,
    pub quote_cache_ttl_seconds: u32,
}

/// Custom limits for enterprise tier
#[derive(Debug, Clone)]
pub struct CustomLimits {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
    pub concurrent_requests: u32,
    pub priority_routing: bool,
    pub dedicated_infrastructure: bool,
}

/// Usage tracking for analytics and billing
#[derive(Debug)]
struct UsageTracker {
    daily_usage: HashMap<String, HashMap<String, u64>>, // date -> endpoint -> count
    monthly_usage: HashMap<String, HashMap<String, u64>>, // month -> endpoint -> count
    last_reset: DateTime<Utc>,
}

/// Authentication request for Jupiter API
#[derive(Debug, Serialize)]
pub struct AuthRequest {
    pub email: String,
    pub project_name: String,
    pub use_case: String,
    pub expected_volume: ExpectedVolume,
}

#[derive(Debug, Serialize)]
pub enum ExpectedVolume {
    Low,      // < 1K requests/day
    Medium,   // 1K-10K requests/day
    High,     // 10K-100K requests/day
    Enterprise, // > 100K requests/day
}

/// Authentication response from Jupiter
#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub api_key: String,
    pub tier: String,
    pub rate_limits: ApiRateLimits,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiRateLimits {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            requests_per_minute: 10,
            requests_per_hour: 100,
            requests_per_day: 1000,
            concurrent_requests: 5,
            quote_cache_ttl_seconds: 30,
        }
    }
}

impl ApiTierLevel {
    /// Get rate limits for this tier
    pub fn rate_limits(&self) -> RateLimits {
        match self {
            ApiTierLevel::Lite => RateLimits {
                requests_per_minute: 10,
                requests_per_hour: 100,
                requests_per_day: 1000,
                concurrent_requests: 5,
                quote_cache_ttl_seconds: 30,
            },
            ApiTierLevel::Ultra => RateLimits {
                requests_per_minute: 60,
                requests_per_hour: 1000,
                requests_per_day: 10000,
                concurrent_requests: 20,
                quote_cache_ttl_seconds: 15,
            },
            ApiTierLevel::Pro { .. } => RateLimits {
                requests_per_minute: 600,
                requests_per_hour: 10000,
                requests_per_day: 100000,
                concurrent_requests: 100,
                quote_cache_ttl_seconds: 5,
            },
            ApiTierLevel::Enterprise { custom_limits } => RateLimits {
                requests_per_minute: custom_limits.requests_per_minute,
                requests_per_hour: custom_limits.requests_per_hour,
                requests_per_day: custom_limits.requests_per_day,
                concurrent_requests: custom_limits.concurrent_requests,
                quote_cache_ttl_seconds: 1, // Minimal cache for enterprise
            },
        }
    }
    
    /// Check if tier supports feature
    pub fn supports_feature(&self, feature: ApiFeature) -> bool {
        match feature {
            ApiFeature::BasicQuoting => true,
            ApiFeature::PriorityRouting => matches!(self, ApiTierLevel::Pro { .. } | ApiTierLevel::Enterprise { .. }),
            ApiFeature::AdvancedAnalytics => matches!(self, ApiTierLevel::Ultra | ApiTierLevel::Pro { .. } | ApiTierLevel::Enterprise { .. }),
            ApiFeature::CustomSlippage => matches!(self, ApiTierLevel::Pro { .. } | ApiTierLevel::Enterprise { .. }),
            ApiFeature::DedicatedSupport => matches!(self, ApiTierLevel::Enterprise { .. }),
            ApiFeature::WhiteGloveOnboarding => matches!(self, ApiTierLevel::Enterprise { .. }),
        }
    }
}

/// Available API features
#[derive(Debug, Clone, Copy)]
pub enum ApiFeature {
    BasicQuoting,
    PriorityRouting,
    AdvancedAnalytics,
    CustomSlippage,
    DedicatedSupport,
    WhiteGloveOnboarding,
}

impl JupiterAuthManager {
    /// Create new authentication manager
    pub fn new() -> Self {
        info!("🔐 Initializing Jupiter API authentication manager");
        
        Self {
            api_keys: RwLock::new(HashMap::new()),
            usage_tracker: RwLock::new(UsageTracker {
                daily_usage: HashMap::new(),
                monthly_usage: HashMap::new(),
                last_reset: Utc::now(),
            }),
        }
    }
    
    /// Add API key to the manager
    pub async fn add_api_key(
        &self,
        key_id: String,
        config: ApiKeyConfig,
    ) -> Result<()> {
        let mut keys = self.api_keys.write().await;
        
        if keys.contains_key(&key_id) {
            warn!("🔐 API key {} already exists, updating configuration", key_id);
        } else {
            info!("🔐 Adding new API key: {} (tier: {:?})", key_id, config.tier);
        }
        
        keys.insert(key_id, config);
        Ok(())
    }
    
    /// Get API key configuration
    pub async fn get_api_key(&self, key_id: &str) -> Option<ApiKeyConfig> {
        let keys = self.api_keys.read().await;
        keys.get(key_id).cloned()
    }
    
    /// Select best API key for a request
    pub async fn select_best_key(&self, endpoint: &str) -> Result<Option<ApiKeyConfig>> {
        let keys = self.api_keys.read().await;
        
        if keys.is_empty() {
            return Ok(None); // Use Lite tier
        }
        
        // Find the highest tier key that's not rate limited
        let mut best_key: Option<ApiKeyConfig> = None;
        
        for (key_id, config) in keys.iter() {
            if !config.is_active {
                continue;
            }
            
            // Check if key is rate limited
            if self.is_rate_limited(key_id, endpoint).await {
                continue;
            }
            
            // Select based on tier priority
            match (&best_key, &config.tier) {
                (None, _) => best_key = Some(config.clone()),
                (Some(current), ApiTierLevel::Enterprise { .. }) => best_key = Some(config.clone()),
                (Some(current), ApiTierLevel::Pro { .. }) if !matches!(current.tier, ApiTierLevel::Enterprise { .. }) => {
                    best_key = Some(config.clone());
                },
                (Some(current), ApiTierLevel::Ultra) if matches!(current.tier, ApiTierLevel::Lite) => {
                    best_key = Some(config.clone());
                },
                _ => {} // Keep current best
            }
        }
        
        // Update last used timestamp
        if let Some(key) = &best_key {
            let key_id = keys.iter()
                .find(|(_, config)| config.key == key.key)
                .map(|(id, _)| id.clone());
                
            if let Some(id) = key_id {
                drop(keys); // Release read lock
                self.mark_key_used(&id).await;
            }
        }
        
        Ok(best_key)
    }
    
    /// Check if API key is rate limited
    async fn is_rate_limited(&self, key_id: &str, endpoint: &str) -> bool {
        let usage = self.usage_tracker.read().await;
        let today = Utc::now().format("%Y-%m-%d").to_string();
        
        if let Some(daily_usage) = usage.daily_usage.get(&today) {
            if let Some(endpoint_usage) = daily_usage.get(endpoint) {
                let keys = self.api_keys.read().await;
                if let Some(config) = keys.get(key_id) {
                    return *endpoint_usage >= config.rate_limits.requests_per_day as u64;
                }
            }
        }
        
        false
    }
    
    /// Mark API key as used
    async fn mark_key_used(&self, key_id: &str) {
        let mut keys = self.api_keys.write().await;
        if let Some(config) = keys.get_mut(key_id) {
            config.last_used = Some(Utc::now());
        }
    }
    
    /// Record API usage
    pub async fn record_usage(&self, key_id: &str, endpoint: &str) {
        let mut usage = self.usage_tracker.write().await;
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let this_month = Utc::now().format("%Y-%m").to_string();
        
        // Record daily usage
        usage.daily_usage
            .entry(today)
            .or_insert_with(HashMap::new)
            .entry(endpoint.to_string())
            .and_modify(|count| *count += 1)
            .or_insert(1);
        
        // Record monthly usage
        usage.monthly_usage
            .entry(this_month)
            .or_insert_with(HashMap::new)
            .entry(endpoint.to_string())
            .and_modify(|count| *count += 1)
            .or_insert(1);
        
        // Update API key daily usage
        let mut keys = self.api_keys.write().await;
        if let Some(config) = keys.get_mut(key_id) {
            config.daily_usage += 1;
        }
    }
    
    /// Get usage statistics
    pub async fn get_usage_stats(&self, key_id: &str) -> Result<UsageStats> {
        let keys = self.api_keys.read().await;
        let usage = self.usage_tracker.read().await;
        
        let config = keys.get(key_id)
            .ok_or_else(|| BotError::config(format!("API key {} not found", key_id)))?;
        
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let this_month = Utc::now().format("%Y-%m").to_string();
        
        let daily_usage = usage.daily_usage
            .get(&today)
            .map(|endpoints| endpoints.values().sum())
            .unwrap_or(0);
            
        let monthly_usage = usage.monthly_usage
            .get(&this_month)
            .map(|endpoints| endpoints.values().sum())
            .unwrap_or(0);
        
        Ok(UsageStats {
            daily_usage,
            monthly_usage,
            daily_limit: config.rate_limits.requests_per_day as u64,
            monthly_limit: match &config.tier {
                ApiTierLevel::Pro { credits_remaining } => *credits_remaining,
                _ => config.rate_limits.requests_per_day as u64 * 30,
            },
            tier: config.tier.clone(),
        })
    }
    
    /// Clean up old usage data
    pub async fn cleanup_old_usage(&self) {
        let mut usage = self.usage_tracker.write().await;
        let cutoff_daily = Utc::now() - Duration::days(30);
        let cutoff_monthly = Utc::now() - Duration::days(365);
        
        // Clean daily usage
        usage.daily_usage.retain(|date_str, _| {
            if let Ok(date) = DateTime::parse_from_str(&format!("{} 00:00:00 +0000", date_str), "%Y-%m-%d %H:%M:%S %z") {
                date.with_timezone(&Utc) > cutoff_daily
            } else {
                false
            }
        });
        
        // Clean monthly usage  
        usage.monthly_usage.retain(|month_str, _| {
            if let Ok(date) = DateTime::parse_from_str(&format!("{}-01 00:00:00 +0000", month_str), "%Y-%m-%d %H:%M:%S %z") {
                date.with_timezone(&Utc) > cutoff_monthly
            } else {
                false
            }
        });
        
        info!("🧹 Cleaned up old usage data");
    }
}

/// Usage statistics for reporting
#[derive(Debug, Serialize)]
pub struct UsageStats {
    pub daily_usage: u64,
    pub monthly_usage: u64,
    pub daily_limit: u64,
    pub monthly_limit: u64,
    pub tier: ApiTierLevel,
}

/// Helper function to create API key config from environment
pub fn create_api_key_from_env(env_key: &str, tier: ApiTierLevel) -> Result<ApiKeyConfig> {
    let key = std::env::var(env_key)
        .map_err(|_| BotError::config(format!("Environment variable {} not set", env_key)))?;
    
    Ok(ApiKeyConfig {
        key,
        tier: tier.clone(),
        created_at: Utc::now(),
        last_used: None,
        daily_usage: 0,
        monthly_usage: 0,
        rate_limits: tier.rate_limits(),
        is_active: true,
        description: Some(format!("Auto-created from {}", env_key)),
    })
}

/// Helper function to register for Jupiter API access
pub async fn register_for_api_access(request: AuthRequest) -> Result<AuthResponse> {
    let client = reqwest::Client::new();
    
    // Note: This is a placeholder for the actual Jupiter API registration endpoint
    // The actual endpoint and process may vary
    let response = client
        .post("https://portal.jup.ag/api/register")
        .json(&request)
        .send()
        .await
        .map_err(|e| BotError::jupiter_api(format!("Registration request failed: {}", e)))?;
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(BotError::jupiter_api(format!(
            "Registration failed: {}", error_text
        )).into());
    }
    
    let auth_response = response
        .json::<AuthResponse>()
        .await
        .map_err(|e| BotError::jupiter_api(format!("Failed to parse registration response: {}", e)))?;
    
    info!("✅ Successfully registered for Jupiter API access: tier {}", auth_response.tier);
    
    Ok(auth_response)
}