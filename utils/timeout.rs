use std::future::Future;
use std::time::Duration;
use tokio::time::timeout;
use anyhow::Result;
use tracing::{warn, error};

/// Configuration for timeout handling
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Default timeout for API calls
    pub default_timeout: Duration,
    /// Timeout for critical operations (trades, transfers)
    pub critical_timeout: Duration,
    /// Timeout for non-critical operations (price checks, analytics)
    pub non_critical_timeout: Duration,
    /// Number of retries on timeout
    pub retry_count: u32,
    /// Backoff multiplier for retries
    pub backoff_multiplier: f64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
            critical_timeout: Duration::from_secs(60),
            non_critical_timeout: Duration::from_secs(15),
            retry_count: 3,
            backoff_multiplier: 2.0,
        }
    }
}

/// Execute a future with timeout and retry logic
pub async fn with_timeout<F, T>(
    future: F,
    duration: Duration,
    operation_name: &str,
) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    match timeout(duration, future).await {
        Ok(result) => result,
        Err(_) => {
            error!("Operation '{}' timed out after {:?}", operation_name, duration);
            Err(anyhow::anyhow!(
                "Operation '{}' timed out after {} seconds",
                operation_name,
                duration.as_secs()
            ))
        }
    }
}

/// Execute a future with timeout and automatic retry
pub async fn with_timeout_retry<F, Fut, T>(
    mut operation: F,
    config: &TimeoutConfig,
    operation_name: &str,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut retry_delay = Duration::from_millis(100);
    
    for attempt in 0..=config.retry_count {
        if attempt > 0 {
            warn!(
                "Retrying operation '{}' (attempt {}/{})",
                operation_name,
                attempt + 1,
                config.retry_count + 1
            );
            tokio::time::sleep(retry_delay).await;
            retry_delay = Duration::from_secs_f64(
                retry_delay.as_secs_f64() * config.backoff_multiplier
            );
        }
        
        match timeout(config.default_timeout, operation()).await {
            Ok(Ok(result)) => return Ok(result),
            Ok(Err(e)) => {
                // Operation failed but didn't timeout
                if attempt == config.retry_count {
                    error!(
                        "Operation '{}' failed after {} attempts: {}",
                        operation_name,
                        config.retry_count + 1,
                        e
                    );
                    return Err(e);
                }
            }
            Err(_) => {
                // Timeout occurred
                if attempt == config.retry_count {
                    error!(
                        "Operation '{}' timed out after {} attempts",
                        operation_name,
                        config.retry_count + 1
                    );
                    return Err(anyhow::anyhow!(
                        "Operation '{}' timed out after {} attempts",
                        operation_name,
                        config.retry_count + 1
                    ));
                }
            }
        }
    }
    
    unreachable!()
}

/// Wrapper for API clients with automatic timeout handling
pub struct TimeoutClient<T> {
    inner: T,
    config: TimeoutConfig,
}

impl<T> TimeoutClient<T> {
    pub fn new(client: T) -> Self {
        Self {
            inner: client,
            config: TimeoutConfig::default(),
        }
    }
    
    pub fn with_config(client: T, config: TimeoutConfig) -> Self {
        Self {
            inner: client,
            config,
        }
    }
    
    /// Execute a critical operation with extended timeout
    pub async fn execute_critical<F, Fut, R>(
        &self,
        operation_name: &str,
        f: F,
    ) -> Result<R>
    where
        F: FnOnce(&T) -> Fut,
        Fut: Future<Output = Result<R>>,
    {
        with_timeout(
            f(&self.inner),
            self.config.critical_timeout,
            operation_name,
        ).await
    }
    
    /// Execute a non-critical operation with shorter timeout
    pub async fn execute_non_critical<F, Fut, R>(
        &self,
        operation_name: &str,
        f: F,
    ) -> Result<R>
    where
        F: FnOnce(&T) -> Fut,
        Fut: Future<Output = Result<R>>,
    {
        with_timeout(
            f(&self.inner),
            self.config.non_critical_timeout,
            operation_name,
        ).await
    }
    
    /// Execute an operation with default timeout
    pub async fn execute<F, Fut, R>(
        &self,
        operation_name: &str,
        f: F,
    ) -> Result<R>
    where
        F: FnOnce(&T) -> Fut,
        Fut: Future<Output = Result<R>>,
    {
        with_timeout(
            f(&self.inner),
            self.config.default_timeout,
            operation_name,
        ).await
    }
}

/// Macro for easy timeout wrapping
#[macro_export]
macro_rules! timeout_operation {
    ($future:expr, $timeout_secs:expr, $op_name:expr) => {
        $crate::utils::timeout::with_timeout(
            $future,
            std::time::Duration::from_secs($timeout_secs),
            $op_name,
        )
    };
}

/// Helper function to create adaptive timeout based on operation type
pub fn adaptive_timeout(operation_type: OperationType) -> Duration {
    match operation_type {
        OperationType::Trade => Duration::from_secs(60),
        OperationType::Transfer => Duration::from_secs(60),
        OperationType::PriceCheck => Duration::from_secs(10),
        OperationType::BalanceCheck => Duration::from_secs(15),
        OperationType::Analytics => Duration::from_secs(20),
        OperationType::TokenCreation => Duration::from_secs(90),
        OperationType::WebSearch => Duration::from_secs(30),
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OperationType {
    Trade,
    Transfer,
    PriceCheck,
    BalanceCheck,
    Analytics,
    TokenCreation,
    WebSearch,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_successful_operation() {
        async fn quick_operation() -> Result<String> {
            Ok("success".to_string())
        }
        
        let result = with_timeout(
            quick_operation(),
            Duration::from_secs(1),
            "test_operation",
        ).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }
    
    #[tokio::test]
    async fn test_timeout() {
        async fn slow_operation() -> Result<String> {
            tokio::time::sleep(Duration::from_secs(2)).await;
            Ok("success".to_string())
        }
        
        let result = with_timeout(
            slow_operation(),
            Duration::from_millis(100),
            "test_operation",
        ).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }
    
    #[tokio::test]
    async fn test_retry_logic() {
        let mut attempt = 0;
        
        let operation = || {
            attempt += 1;
            async move {
                if attempt < 2 {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    Err(anyhow::anyhow!("timeout"))
                } else {
                    Ok("success".to_string())
                }
            }
        };
        
        let config = TimeoutConfig {
            default_timeout: Duration::from_millis(100),
            retry_count: 2,
            ..Default::default()
        };
        
        let result = with_timeout_retry(
            operation,
            &config,
            "test_operation",
        ).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }
    
    #[test]
    fn test_adaptive_timeout() {
        assert_eq!(adaptive_timeout(OperationType::Trade), Duration::from_secs(60));
        assert_eq!(adaptive_timeout(OperationType::PriceCheck), Duration::from_secs(10));
        assert_eq!(adaptive_timeout(OperationType::TokenCreation), Duration::from_secs(90));
    }
}