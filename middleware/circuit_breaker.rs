use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{warn, info, debug};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,    // Normal operation
    Open,      // Circuit is open, failing fast
    HalfOpen,  // Testing if service has recovered
}

#[derive(Debug)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,     // Number of failures to open circuit
    pub timeout: Duration,          // Time to wait before trying again
    pub success_threshold: u32,     // Successes needed in half-open to close
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            timeout: Duration::from_secs(30),
            success_threshold: 3,
        }
    }
}

pub struct CircuitBreaker {
    name: String,
    config: CircuitBreakerConfig,
    state: RwLock<CircuitState>,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    last_failure_time: AtomicU64,
    total_requests: AtomicU32,
    total_failures: AtomicU32,
}

impl CircuitBreaker {
    pub fn new(name: String, config: CircuitBreakerConfig) -> Self {
        info!("Circuit breaker '{}' initialized with config: {:?}", name, config);
        
        Self {
            name,
            config,
            state: RwLock::new(CircuitState::Closed),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure_time: AtomicU64::new(0),
            total_requests: AtomicU32::new(0),
            total_failures: AtomicU32::new(0),
        }
    }
    
    /// Execute a function with circuit breaker protection
    pub async fn execute<F, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: std::future::Future<Output = Result<T, E>>,
    {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        
        // Check if circuit should be closed due to timeout
        self.check_timeout().await;
        
        let state = *self.state.read().await;
        
        match state {
            CircuitState::Open => {
                debug!("Circuit breaker '{}' is OPEN - failing fast", self.name);
                return Err(CircuitBreakerError::CircuitOpen);
            }
            CircuitState::HalfOpen => {
                debug!("Circuit breaker '{}' is HALF-OPEN - testing", self.name);
            }
            CircuitState::Closed => {
                debug!("Circuit breaker '{}' is CLOSED - normal operation", self.name);
            }
        }
        
        // Execute the operation
        match operation.await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(error) => {
                self.on_failure().await;
                Err(CircuitBreakerError::OperationFailed(error))
            }
        }
    }
    
    async fn check_timeout(&self) {
        let state = *self.state.read().await;
        if state == CircuitState::Open {
            let last_failure = self.last_failure_time.load(Ordering::Relaxed);
            let now = Instant::now().elapsed().as_millis() as u64;
            
            if now.saturating_sub(last_failure) >= self.config.timeout.as_millis() as u64 {
                info!("Circuit breaker '{}' timeout expired, transitioning to HALF-OPEN", self.name);
                *self.state.write().await = CircuitState::HalfOpen;
                self.success_count.store(0, Ordering::Relaxed);
            }
        }
    }
    
    async fn on_success(&self) {
        let state = *self.state.read().await;
        
        match state {
            CircuitState::HalfOpen => {
                let successes = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if successes >= self.config.success_threshold {
                    info!("Circuit breaker '{}' transitioning to CLOSED after {} successes", 
                          self.name, successes);
                    *self.state.write().await = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::Relaxed);
                    self.success_count.store(0, Ordering::Relaxed);
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::Open => {
                // Should not reach here, but reset just in case
                self.failure_count.store(0, Ordering::Relaxed);
            }
        }
    }
    
    async fn on_failure(&self) {
        self.total_failures.fetch_add(1, Ordering::Relaxed);
        self.last_failure_time.store(
            Instant::now().elapsed().as_millis() as u64,
            Ordering::Relaxed
        );
        
        let state = *self.state.read().await;
        
        match state {
            CircuitState::Closed => {
                let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                if failures >= self.config.failure_threshold {
                    warn!("Circuit breaker '{}' transitioning to OPEN after {} failures", 
                          self.name, failures);
                    *self.state.write().await = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                warn!("Circuit breaker '{}' transitioning back to OPEN due to failure in half-open state", 
                      self.name);
                *self.state.write().await = CircuitState::Open;
            }
            CircuitState::Open => {
                // Already open, just update failure time
            }
        }
    }
    
    /// Get current circuit breaker state
    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }
    
    /// Get circuit breaker metrics
    pub async fn metrics(&self) -> CircuitBreakerMetrics {
        CircuitBreakerMetrics {
            name: self.name.clone(),
            state: *self.state.read().await,
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_failures: self.total_failures.load(Ordering::Relaxed),
            failure_rate: {
                let requests = self.total_requests.load(Ordering::Relaxed);
                let failures = self.total_failures.load(Ordering::Relaxed);
                if requests > 0 {
                    (failures as f64 / requests as f64) * 100.0
                } else {
                    0.0
                }
            }
        }
    }
    
    /// Force circuit breaker to open (for testing/maintenance)
    pub async fn force_open(&self) {
        warn!("Force opening circuit breaker '{}'", self.name);
        *self.state.write().await = CircuitState::Open;
    }
    
    /// Force circuit breaker to close (for recovery)
    pub async fn force_close(&self) {
        info!("Force closing circuit breaker '{}'", self.name);
        *self.state.write().await = CircuitState::Closed;
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerMetrics {
    pub name: String,
    pub state: CircuitState,
    pub total_requests: u32,
    pub total_failures: u32,
    pub failure_rate: f64,
}

#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    CircuitOpen,
    OperationFailed(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::CircuitOpen => write!(f, "Circuit breaker is open"),
            CircuitBreakerError::OperationFailed(e) => write!(f, "Operation failed: {}", e),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CircuitBreakerError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CircuitBreakerError::CircuitOpen => None,
            CircuitBreakerError::OperationFailed(e) => Some(e),
        }
    }
}