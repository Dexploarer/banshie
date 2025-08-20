use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tracing::{info, warn};
use anyhow::Result;

/// Rate limiter for API calls with per-endpoint and global limits
#[derive(Clone)]
pub struct ApiRateLimiter {
    /// Global rate limit across all endpoints
    global_semaphore: Arc<Semaphore>,
    /// Per-endpoint rate limiters
    endpoint_limiters: Arc<Mutex<HashMap<String, EndpointLimiter>>>,
    /// Configuration
    config: RateLimitConfig,
}

#[derive(Clone)]
pub struct RateLimitConfig {
    /// Maximum global requests per second
    pub global_rps: usize,
    /// Maximum requests per endpoint per minute
    pub endpoint_rpm: usize,
    /// Burst allowance (temporary spike tolerance)
    pub burst_size: usize,
    /// Cooldown period after hitting limits
    pub cooldown_duration: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            global_rps: 10,        // 10 requests per second globally
            endpoint_rpm: 60,      // 60 requests per minute per endpoint
            burst_size: 5,         // Allow burst of 5 extra requests
            cooldown_duration: Duration::from_secs(60),
        }
    }
}

struct EndpointLimiter {
    /// Request timestamps for sliding window
    request_times: Vec<Instant>,
    /// Last cleanup time
    last_cleanup: Instant,
    /// Number of requests in current burst
    burst_count: usize,
    /// Cooldown until time
    cooldown_until: Option<Instant>,
}

impl ApiRateLimiter {
    /// Create a new rate limiter with default config
    pub fn new() -> Self {
        Self::with_config(RateLimitConfig::default())
    }
    
    /// Create a new rate limiter with custom config
    pub fn with_config(config: RateLimitConfig) -> Self {
        Self {
            global_semaphore: Arc::new(Semaphore::new(config.global_rps)),
            endpoint_limiters: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }
    
    /// Check if request is allowed and update counters
    pub async fn check_rate_limit(&self, endpoint: &str) -> Result<RateLimitToken> {
        // Check global rate limit
        let global_permit = self.global_semaphore
            .try_acquire()
            .map_err(|_| anyhow::anyhow!("Global rate limit exceeded"))?;
        
        // Check endpoint-specific rate limit
        let mut limiters = self.endpoint_limiters.lock().await;
        let limiter = limiters.entry(endpoint.to_string())
            .or_insert_with(|| EndpointLimiter {
                request_times: Vec::new(),
                last_cleanup: Instant::now(),
                burst_count: 0,
                cooldown_until: None,
            });
        
        // Check if in cooldown
        if let Some(cooldown_until) = limiter.cooldown_until {
            if Instant::now() < cooldown_until {
                let remaining = cooldown_until.duration_since(Instant::now());
                return Err(anyhow::anyhow!(
                    "Rate limit cooldown for endpoint '{}': {} seconds remaining",
                    endpoint,
                    remaining.as_secs()
                ));
            }
            limiter.cooldown_until = None;
            limiter.burst_count = 0;
        }
        
        let now = Instant::now();
        let window_start = now - Duration::from_secs(60);
        
        // Clean up old timestamps
        if now.duration_since(limiter.last_cleanup) > Duration::from_secs(10) {
            limiter.request_times.retain(|&t| t > window_start);
            limiter.last_cleanup = now;
        }
        
        // Check endpoint rate limit
        let recent_requests = limiter.request_times
            .iter()
            .filter(|&&t| t > window_start)
            .count();
        
        if recent_requests >= self.config.endpoint_rpm {
            // Check burst allowance
            if limiter.burst_count < self.config.burst_size {
                limiter.burst_count += 1;
                warn!(
                    "Using burst allowance for endpoint '{}': {}/{}",
                    endpoint, limiter.burst_count, self.config.burst_size
                );
            } else {
                // Enter cooldown
                limiter.cooldown_until = Some(now + self.config.cooldown_duration);
                return Err(anyhow::anyhow!(
                    "Rate limit exceeded for endpoint '{}': entering {} second cooldown",
                    endpoint,
                    self.config.cooldown_duration.as_secs()
                ));
            }
        }
        
        // Record this request
        limiter.request_times.push(now);
        
        Ok(RateLimitToken {
            _global_permit: global_permit,
            endpoint: endpoint.to_string(),
            acquired_at: now,
        })
    }
    
    /// Get current usage stats for an endpoint
    pub async fn get_usage_stats(&self, endpoint: &str) -> EndpointStats {
        let limiters = self.endpoint_limiters.lock().await;
        
        if let Some(limiter) = limiters.get(endpoint) {
            let now = Instant::now();
            let window_start = now - Duration::from_secs(60);
            let recent_requests = limiter.request_times
                .iter()
                .filter(|&&t| t > window_start)
                .count();
            
            EndpointStats {
                endpoint: endpoint.to_string(),
                requests_in_window: recent_requests,
                limit: self.config.endpoint_rpm,
                burst_used: limiter.burst_count,
                burst_limit: self.config.burst_size,
                in_cooldown: limiter.cooldown_until.is_some(),
                cooldown_remaining: limiter.cooldown_until
                    .map(|until| {
                        if until > now {
                            until.duration_since(now)
                        } else {
                            Duration::ZERO
                        }
                    }),
            }
        } else {
            EndpointStats {
                endpoint: endpoint.to_string(),
                requests_in_window: 0,
                limit: self.config.endpoint_rpm,
                burst_used: 0,
                burst_limit: self.config.burst_size,
                in_cooldown: false,
                cooldown_remaining: None,
            }
        }
    }
    
    /// Reset rate limits for an endpoint
    pub async fn reset_endpoint(&self, endpoint: &str) {
        let mut limiters = self.endpoint_limiters.lock().await;
        limiters.remove(endpoint);
        info!("Reset rate limits for endpoint: {}", endpoint);
    }
    
    /// Reset all rate limits
    pub async fn reset_all(&self) {
        let mut limiters = self.endpoint_limiters.lock().await;
        limiters.clear();
        info!("Reset all rate limits");
    }
}

/// Token representing an approved rate limit check
pub struct RateLimitToken {
    _global_permit: tokio::sync::SemaphorePermit<'static>,
    pub endpoint: String,
    pub acquired_at: Instant,
}

impl RateLimitToken {
    /// Get the age of this token
    pub fn age(&self) -> Duration {
        Instant::now().duration_since(self.acquired_at)
    }
}

/// Statistics for an endpoint
#[derive(Debug, Clone)]
pub struct EndpointStats {
    pub endpoint: String,
    pub requests_in_window: usize,
    pub limit: usize,
    pub burst_used: usize,
    pub burst_limit: usize,
    pub in_cooldown: bool,
    pub cooldown_remaining: Option<Duration>,
}

impl EndpointStats {
    /// Check if endpoint is near rate limit
    pub fn is_near_limit(&self) -> bool {
        self.requests_in_window as f64 > (self.limit as f64 * 0.8)
    }
    
    /// Get usage percentage
    pub fn usage_percentage(&self) -> f64 {
        (self.requests_in_window as f64 / self.limit as f64) * 100.0
    }
}

/// Rate-limited API client wrapper
pub struct RateLimitedClient<T> {
    inner: T,
    rate_limiter: ApiRateLimiter,
}

impl<T> RateLimitedClient<T> {
    pub fn new(client: T, rate_limiter: ApiRateLimiter) -> Self {
        Self {
            inner: client,
            rate_limiter,
        }
    }
    
    /// Execute a rate-limited request
    pub async fn execute<F, R>(&self, endpoint: &str, f: F) -> Result<R>
    where
        F: FnOnce(&T) -> R,
    {
        // Acquire rate limit token
        let _token = self.rate_limiter.check_rate_limit(endpoint).await?;
        
        // Execute the request
        Ok(f(&self.inner))
    }
    
    /// Get the inner client
    pub fn inner(&self) -> &T {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = ApiRateLimiter::new();
        
        // Should allow initial requests
        assert!(limiter.check_rate_limit("test").await.is_ok());
        assert!(limiter.check_rate_limit("test").await.is_ok());
        
        // Check stats
        let stats = limiter.get_usage_stats("test").await;
        assert_eq!(stats.requests_in_window, 2);
        assert!(!stats.in_cooldown);
    }
    
    #[tokio::test]
    async fn test_burst_allowance() {
        let config = RateLimitConfig {
            global_rps: 100,
            endpoint_rpm: 2, // Very low for testing
            burst_size: 2,
            cooldown_duration: Duration::from_secs(1),
        };
        
        let limiter = ApiRateLimiter::with_config(config);
        
        // Fill up normal quota
        assert!(limiter.check_rate_limit("test").await.is_ok());
        assert!(limiter.check_rate_limit("test").await.is_ok());
        
        // Should use burst
        assert!(limiter.check_rate_limit("test").await.is_ok());
        assert!(limiter.check_rate_limit("test").await.is_ok());
        
        // Should hit limit
        assert!(limiter.check_rate_limit("test").await.is_err());
    }
}