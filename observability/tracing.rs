use opentelemetry::trace::{TraceError, Tracer};
use std::sync::Arc;
use tracing::{info, warn};

use crate::{
    telemetry::{TelemetryService, TelemetryConfig},
    errors::Result,
};

/// Enhanced tracing setup for comprehensive observability
pub struct TracingSetup {
    telemetry_service: Arc<TelemetryService>,
}

impl TracingSetup {
    /// Initialize tracing with OpenTelemetry integration
    pub async fn init() -> Result<Self> {
        // Load configuration from environment
        let mut config = TelemetryConfig::default();
        
        // Customize configuration based on environment
        if let Ok(env) = std::env::var("ENVIRONMENT") {
            config.environment = env;
            
            // Adjust sampling for production
            if config.environment == "production" {
                config.sampling_ratio = 0.1; // 10% sampling in production
                config.enable_stdout = false; // Disable stdout in production
            }
        }
        
        // Configure resource attributes for better identification
        config.resource_attributes.insert(
            "service.instance.id".to_string(),
            std::env::var("HOSTNAME").unwrap_or_else(|_| {
                format!("instance-{}", uuid::Uuid::new_v4())
            })
        );
        
        config.resource_attributes.insert(
            "deployment.environment".to_string(),
            config.environment.clone()
        );
        
        // Initialize telemetry service
        let telemetry_service = Arc::new(TelemetryService::init(config)?);
        
        info!("🔭 Enhanced tracing setup complete");
        
        Ok(Self {
            telemetry_service,
        })
    }
    
    /// Get reference to telemetry service
    pub fn telemetry(&self) -> Arc<TelemetryService> {
        self.telemetry_service.clone()
    }
    
    /// Create a trading operation span with enhanced context
    pub fn trace_trading_operation(&self, operation: &str) -> TracingContext {
        TracingContext::new(
            self.telemetry_service.create_trading_span(operation, None),
            self.telemetry_service.clone()
        )
    }
    
    /// Create a Jupiter API call span with enhanced context  
    pub fn trace_jupiter_call(&self, endpoint: &str, method: &str) -> TracingContext {
        TracingContext::new(
            self.telemetry_service.create_jupiter_span(endpoint, method),
            self.telemetry_service.clone()
        )
    }
    
    /// Create a database operation span with enhanced context
    pub fn trace_database_operation(&self, operation: &str, table: Option<&str>) -> TracingContext {
        TracingContext::new(
            self.telemetry_service.create_database_span(operation, table),
            self.telemetry_service.clone()
        )
    }
    
    /// Shutdown tracing gracefully
    pub async fn shutdown(&self) -> Result<()> {
        self.telemetry_service.shutdown().await
    }
}

/// Enhanced tracing context that automatically records metrics and errors
pub struct TracingContext {
    span: tracing::Span,
    telemetry_service: Arc<TelemetryService>,
    start_time: std::time::Instant,
}

impl TracingContext {
    fn new(span: tracing::Span, telemetry_service: Arc<TelemetryService>) -> Self {
        Self {
            span,
            telemetry_service,
            start_time: std::time::Instant::now(),
        }
    }
    
    /// Enter the span context
    pub fn enter(&self) -> tracing::span::Entered<'_> {
        self.span.enter()
    }
    
    /// Add custom attributes to the span
    pub fn add_attributes(&self, attributes: &[(&str, &str)]) {
        self.telemetry_service.add_span_attributes(attributes);
    }
    
    /// Record an error in the span
    pub fn record_error(&self, error: &dyn std::error::Error) {
        self.telemetry_service.record_error(error);
    }
    
    /// Record success and duration
    pub fn record_success(&self) {
        let duration = self.start_time.elapsed();
        self.span.record("success", true);
        self.span.record("duration_ms", duration.as_millis() as u64);
    }
    
    /// Record trading-specific metrics
    pub fn record_trading_metrics(&self, amount: f64, token_pair: &str, slippage: f64) {
        self.span.record("trading.amount", amount);
        self.span.record("trading.token_pair", token_pair);
        self.span.record("trading.slippage", slippage);
    }
    
    /// Record API response metrics
    pub fn record_api_metrics(&self, status_code: u16, response_size: usize) {
        self.span.record("http.status_code", status_code as i64);
        self.span.record("http.response_size_bytes", response_size as i64);
    }
    
    /// Record database metrics
    pub fn record_database_metrics(&self, rows_affected: i64, query_duration: std::time::Duration) {
        self.span.record("db.rows_affected", rows_affected);
        self.span.record("db.query_duration_ms", query_duration.as_millis() as u64);
    }
}

impl Drop for TracingContext {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed();
        self.span.record("duration_ms", duration.as_millis() as u64);
    }
}

/// Convenience macros for enhanced tracing
#[macro_export]
macro_rules! trace_trading {
    ($tracer:expr, $operation:expr, $code:block) => {
        {
            let context = $tracer.trace_trading_operation($operation);
            let _guard = context.enter();
            let result = $code;
            
            match &result {
                Ok(_) => context.record_success(),
                Err(e) => context.record_error(e.as_ref()),
            }
            
            result
        }
    };
}

#[macro_export]
macro_rules! trace_jupiter {
    ($tracer:expr, $endpoint:expr, $method:expr, $code:block) => {
        {
            let context = $tracer.trace_jupiter_call($endpoint, $method);
            let _guard = context.enter();
            let result = $code;
            
            match &result {
                Ok(_) => context.record_success(),
                Err(e) => context.record_error(e.as_ref()),
            }
            
            result
        }
    };
}

#[macro_export]
macro_rules! trace_database {
    ($tracer:expr, $operation:expr, $table:expr, $code:block) => {
        {
            let context = $tracer.trace_database_operation($operation, Some($table));
            let _guard = context.enter();
            let result = $code;
            
            match &result {
                Ok(_) => context.record_success(),
                Err(e) => context.record_error(e.as_ref()),
            }
            
            result
        }
    };
}