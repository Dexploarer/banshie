use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::sleep;
use tracing::{warn, debug, info};

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
    pub burst_capacity: u32,
    pub cleanup_interval: Duration,
    pub cooldown_minutes: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            requests_per_hour: 1000,
            requests_per_day: 10000,
            burst_capacity: 10,
            cleanup_interval: Duration::from_secs(300),
            cooldown_minutes: 5,
        }
    }
}

/// Endpoint-specific rate limits
impl RateLimitConfig {
    pub fn for_trading() -> Self {
        Self {
            requests_per_minute: 5,
            requests_per_hour: 50,
            requests_per_day: 200,
            burst_capacity: 2,
            cleanup_interval: Duration::from_secs(300),
            cooldown_minutes: 10,
        }
    }
    
    pub fn for_portfolio() -> Self {
        Self {
            requests_per_minute: 20,
            requests_per_hour: 200,
            requests_per_day: 1000,
            burst_capacity: 5,
            cleanup_interval: Duration::from_secs(300),
            cooldown_minutes: 2,
        }
    }
    
    pub fn for_market_data() -> Self {
        Self {
            requests_per_minute: 30,
            requests_per_hour: 500,
            requests_per_day: 2000,
            burst_capacity: 10,
            cleanup_interval: Duration::from_secs(300),
            cooldown_minutes: 1,
        }
    }
}

#[derive(Debug)]
struct UserRateLimit {
    tokens: Arc<Semaphore>,
    last_refill: Instant,
    total_requests: u64,
    blocked_requests: u64,
}

impl UserRateLimit {
    fn new(config: &RateLimitConfig) -> Self {
        Self {
            tokens: Arc::new(Semaphore::new(config.burst_capacity as usize)),
            last_refill: Instant::now(),
            total_requests: 0,
            blocked_requests: 0,
        }
    }
    
    async fn try_acquire(&mut self, config: &RateLimitConfig) -> Result<(), RateLimitError> {
        self.total_requests += 1;
        
        // Refill tokens based on time elapsed
        self.refill_tokens(config).await;
        
        // Try to acquire a token (non-blocking)
        match self.tokens.try_acquire() {
            Ok(_permit) => {
                debug!("Rate limit check passed for user");
                Ok(())
            }
            Err(_) => {
                self.blocked_requests += 1;
                warn!("Rate limit exceeded for user, blocking request");
                Err(RateLimitError::RateLimitExceeded)
            }
        }
    }
    
    async fn refill_tokens(&mut self, config: &RateLimitConfig) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        
        if elapsed >= Duration::from_secs(60) {
            // Calculate how many tokens to add (1 token per minute)
            let minutes_elapsed = elapsed.as_secs() / 60;
            let tokens_to_add = minutes_elapsed as usize;
            
            if tokens_to_add > 0 {
                // Add tokens up to burst capacity
                let current_permits = self.tokens.available_permits();
                let max_permits = config.burst_capacity as usize;
                let permits_to_add = std::cmp::min(tokens_to_add, max_permits - current_permits);
                
                if permits_to_add > 0 {
                    self.tokens.add_permits(permits_to_add);
                    debug!("Added {} tokens to rate limiter", permits_to_add);
                }
                
                self.last_refill = now;
            }
        }
    }
}

pub struct UserRateLimiter {
    config: RateLimitConfig,
    users: Arc<RwLock<HashMap<String, UserRateLimit>>>,
    last_cleanup: Arc<RwLock<Instant>>,
}

impl UserRateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        info!("User rate limiter initialized: {} requests/minute, {} burst capacity", 
              config.requests_per_minute, config.burst_capacity);
        
        Self {
            config,
            users: Arc::new(RwLock::new(HashMap::new())),
            last_cleanup: Arc::new(RwLock::new(Instant::now())),
        }
    }
    
    /// Check if user can make a request
    pub async fn check_rate_limit(&self, user_id: &str) -> Result<(), RateLimitError> {
        // Periodically clean up old entries
        self.maybe_cleanup().await;
        
        let mut users = self.users.write().await;
        let user_limit = users.entry(user_id.to_string())
            .or_insert_with(|| UserRateLimit::new(&self.config));
        
        user_limit.try_acquire(&self.config).await
    }
    
    /// Check rate limit with custom config
    pub async fn check_rate_limit_with_config(&self, user_id: &str, config: &RateLimitConfig) -> Result<(), RateLimitError> {
        // Periodically clean up old entries
        self.maybe_cleanup().await;
        
        let mut users = self.users.write().await;
        let user_limit = users.entry(user_id.to_string())
            .or_insert_with(|| UserRateLimit::new(config));
        
        user_limit.try_acquire(config).await
    }
    
    /// Check rate limit with automatic retry after delay
    pub async fn check_rate_limit_with_retry(
        &self, 
        user_id: &str,
        max_retries: u32,
        base_delay: Duration
    ) -> Result<(), RateLimitError> {
        let mut retries = 0;
        
        loop {
            match self.check_rate_limit(user_id).await {
                Ok(()) => return Ok(()),
                Err(RateLimitError::RateLimitExceeded) if retries < max_retries => {
                    retries += 1;
                    let delay = base_delay * retries;
                    warn!("Rate limit exceeded for user {}, retrying in {:?} (attempt {}/{})", 
                          user_id, delay, retries, max_retries);
                    sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
    
    /// Add tokens to a specific user (for premium users or special circumstances)
    pub async fn add_tokens(&self, user_id: &str, tokens: u32) {
        let mut users = self.users.write().await;
        if let Some(user_limit) = users.get_mut(user_id) {
            let available = user_limit.tokens.available_permits();
            let to_add = std::cmp::min(tokens as usize, self.config.burst_capacity as usize - available);
            
            if to_add > 0 {
                user_limit.tokens.add_permits(to_add);
                info!("Added {} tokens to user {}", to_add, user_id);
            }
        }
    }
    
    /// Get rate limiting statistics for a user
    pub async fn get_user_stats(&self, user_id: &str) -> Option<UserRateStats> {
        let users = self.users.read().await;
        users.get(user_id).map(|limit| {
            UserRateStats {
                user_id: user_id.to_string(),
                available_tokens: limit.tokens.available_permits() as u32,
                total_requests: limit.total_requests,
                blocked_requests: limit.blocked_requests,
                block_rate: if limit.total_requests > 0 {
                    (limit.blocked_requests as f64 / limit.total_requests as f64) * 100.0
                } else {
                    0.0
                }
            }
        })
    }
    
    /// Get global rate limiting statistics
    pub async fn get_global_stats(&self) -> GlobalRateStats {
        let users = self.users.read().await;
        let mut total_requests = 0;
        let mut total_blocked = 0;
        let active_users = users.len() as u32;
        
        for limit in users.values() {
            total_requests += limit.total_requests;
            total_blocked += limit.blocked_requests;
        }
        
        GlobalRateStats {
            active_users,
            total_requests,
            total_blocked,
            global_block_rate: if total_requests > 0 {
                (total_blocked as f64 / total_requests as f64) * 100.0
            } else {
                0.0
            }
        }
    }
    
    /// Clean up inactive users
    async fn maybe_cleanup(&self) {
        let mut last_cleanup = self.last_cleanup.write().await;
        let now = Instant::now();
        
        if now.duration_since(*last_cleanup) >= self.config.cleanup_interval {
            let mut users = self.users.write().await;
            let initial_count = users.len();
            
            // Remove users who haven't made requests in the last hour
            users.retain(|_user_id, limit| {
                now.duration_since(limit.last_refill) <= Duration::from_secs(3600)
            });
            
            let removed = initial_count - users.len();
            if removed > 0 {
                info!("Cleaned up {} inactive users from rate limiter", removed);
            }
            
            *last_cleanup = now;
        }
    }
    
    /// Clear all rate limits (for testing/admin purposes)
    pub async fn clear_all(&self) {
        let mut users = self.users.write().await;
        users.clear();
        info!("Cleared all rate limits");
    }
    
    /// Set custom rate limit for specific user
    pub async fn set_user_limit(&self, user_id: &str, custom_config: RateLimitConfig) {
        let mut users = self.users.write().await;
        users.insert(user_id.to_string(), UserRateLimit::new(&custom_config));
        info!("Set custom rate limit for user {}: {:?}", user_id, custom_config);
    }
}

#[derive(Debug, Clone)]
pub struct UserRateStats {
    pub user_id: String,
    pub available_tokens: u32,
    pub total_requests: u64,
    pub blocked_requests: u64,
    pub block_rate: f64,
}

#[derive(Debug, Clone)]
pub struct GlobalRateStats {
    pub active_users: u32,
    pub total_requests: u64,
    pub total_blocked: u64,
    pub global_block_rate: f64,
}

#[derive(Debug, Clone)]
pub enum RateLimitError {
    RateLimitExceeded,
    InternalError(String),
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimitError::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            RateLimitError::InternalError(msg) => write!(f, "Rate limiter error: {}", msg),
        }
    }
}

impl std::error::Error for RateLimitError {}