use opentelemetry::{
    global, trace::{TraceContextExt, Tracer},
    Context, Key, KeyValue,
};
use opentelemetry_sdk::{
    trace::{self, RandomIdGenerator, Sampler},
    Resource,
};
use opentelemetry_otlp::WithExportConfig;
use tracing::{info, warn, error, Span};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

/// Configuration for telemetry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub service_name: String,
    pub service_version: String,
    pub environment: String,
    pub otlp_endpoint: Option<String>,
    pub jaeger_endpoint: Option<String>,
    pub sampling_rate: f64,
    pub batch_timeout_seconds: u64,
    pub max_export_batch_size: usize,
    pub enable_console: bool,
    pub enable_file_logging: bool,
    pub log_level: String,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            service_name: "solana-trading-bot".to_string(),
            service_version: "0.2.0".to_string(),
            environment: "development".to_string(),
            otlp_endpoint: None,
            jaeger_endpoint: None,
            sampling_rate: 0.1,
            batch_timeout_seconds: 5,
            max_export_batch_size: 512,
            enable_console: true,
            enable_file_logging: true,
            log_level: "info".to_string(),
        }
    }
}

/// Telemetry service for distributed tracing and observability
pub struct TelemetryService {
    config: TelemetryConfig,
    tracer: Box<dyn Tracer + Send + Sync>,
    spans: Arc<RwLock<HashMap<String, SpanData>>>,
}

#[derive(Debug, Clone)]
struct SpanData {
    span_id: String,
    trace_id: String,
    operation_name: String,
    start_time: DateTime<Utc>,
    duration_ms: Option<u64>,
    tags: HashMap<String, String>,
    logs: Vec<LogEntry>,
}

#[derive(Debug, Clone)]
struct LogEntry {
    timestamp: DateTime<Utc>,
    level: String,
    message: String,
}

impl TelemetryService {
    /// Create new telemetry service
    pub async fn new(config: TelemetryConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Initialize OpenTelemetry
        let tracer = init_telemetry(&config).await?;
        
        Ok(Self {
            config,
            tracer,
            spans: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Start a new span for tracing
    pub async fn start_span(&self, operation_name: &str) -> String {
        let span = self.tracer.start(operation_name);
        let span_context = span.span_context();
        
        let span_id = format!("{:016x}", span_context.span_id().to_u64());
        let trace_id = format!("{:032x}", span_context.trace_id().to_u128());
        
        let span_data = SpanData {
            span_id: span_id.clone(),
            trace_id,
            operation_name: operation_name.to_string(),
            start_time: Utc::now(),
            duration_ms: None,
            tags: HashMap::new(),
            logs: Vec::new(),
        };
        
        let mut spans = self.spans.write().await;
        spans.insert(span_id.clone(), span_data);
        
        info!("Started span: {} for operation: {}", span_id, operation_name);
        span_id
    }
    
    /// Add tag to span
    pub async fn add_span_tag(&self, span_id: &str, key: &str, value: &str) {
        let mut spans = self.spans.write().await;
        if let Some(span_data) = spans.get_mut(span_id) {
            span_data.tags.insert(key.to_string(), value.to_string());
        }
    }
    
    /// Add log to span
    pub async fn add_span_log(&self, span_id: &str, level: &str, message: &str) {
        let mut spans = self.spans.write().await;
        if let Some(span_data) = spans.get_mut(span_id) {
            span_data.logs.push(LogEntry {
                timestamp: Utc::now(),
                level: level.to_string(),
                message: message.to_string(),
            });
        }
    }
    
    /// Finish span
    pub async fn finish_span(&self, span_id: &str) {
        let mut spans = self.spans.write().await;
        if let Some(span_data) = spans.get_mut(span_id) {
            let duration = Utc::now()
                .signed_duration_since(span_data.start_time)
                .num_milliseconds() as u64;
            
            span_data.duration_ms = Some(duration);
            
            info!("Finished span: {} ({}ms) for operation: {}", 
                span_id, duration, span_data.operation_name);
        }
    }
    
    /// Trace trading operation
    pub async fn trace_trade(
        &self,
        user_id: &str,
        token: &str,
        action: &str,
        amount: f64,
    ) -> String {
        let span_id = self.start_span("trade_execution").await;
        
        self.add_span_tag(&span_id, "user.id", user_id).await;
        self.add_span_tag(&span_id, "trade.token", token).await;
        self.add_span_tag(&span_id, "trade.action", action).await;
        self.add_span_tag(&span_id, "trade.amount", &amount.to_string()).await;
        self.add_span_tag(&span_id, "component", "trading").await;
        
        span_id
    }
    
    /// Trace API call
    pub async fn trace_api_call(
        &self,
        endpoint: &str,
        method: &str,
        user_agent: Option<&str>,
    ) -> String {
        let span_id = self.start_span("api_call").await;
        
        self.add_span_tag(&span_id, "http.url", endpoint).await;
        self.add_span_tag(&span_id, "http.method", method).await;
        
        if let Some(ua) = user_agent {
            self.add_span_tag(&span_id, "http.user_agent", ua).await;
        }
        
        self.add_span_tag(&span_id, "component", "api").await;
        
        span_id
    }
    
    /// Trace MEV protection
    pub async fn trace_mev_protection(
        &self,
        strategy: &str,
        bundle_id: Option<&str>,
        tip_amount: f64,
    ) -> String {
        let span_id = self.start_span("mev_protection").await;
        
        self.add_span_tag(&span_id, "mev.strategy", strategy).await;
        self.add_span_tag(&span_id, "mev.tip_amount", &tip_amount.to_string()).await;
        
        if let Some(bundle) = bundle_id {
            self.add_span_tag(&span_id, "mev.bundle_id", bundle).await;
        }
        
        self.add_span_tag(&span_id, "component", "mev").await;
        
        span_id
    }
    
    /// Trace wallet operation
    pub async fn trace_wallet_operation(
        &self,
        wallet_address: &str,
        operation: &str,
        token: Option<&str>,
    ) -> String {
        let span_id = self.start_span("wallet_operation").await;
        
        self.add_span_tag(&span_id, "wallet.address", &wallet_address[..8]).await;
        self.add_span_tag(&span_id, "wallet.operation", operation).await;
        
        if let Some(t) = token {
            self.add_span_tag(&span_id, "wallet.token", t).await;
        }
        
        self.add_span_tag(&span_id, "component", "wallet").await;
        
        span_id
    }
    
    /// Trace command processing
    pub async fn trace_command(
        &self,
        command: &str,
        user_id: &str,
        chat_id: &str,
    ) -> String {
        let span_id = self.start_span("command_processing").await;
        
        self.add_span_tag(&span_id, "bot.command", command).await;
        self.add_span_tag(&span_id, "bot.user_id", user_id).await;
        self.add_span_tag(&span_id, "bot.chat_id", chat_id).await;
        self.add_span_tag(&span_id, "component", "bot").await;
        
        span_id
    }
    
    /// Record error in span
    pub async fn record_error(
        &self,
        span_id: &str,
        error: &dyn std::error::Error,
        error_type: &str,
    ) {
        self.add_span_tag(span_id, "error", "true").await;
        self.add_span_tag(span_id, "error.type", error_type).await;
        self.add_span_log(span_id, "error", &error.to_string()).await;
        
        error!("Error in span {}: {} ({})", span_id, error, error_type);
    }
    
    /// Get span data
    pub async fn get_span(&self, span_id: &str) -> Option<SpanData> {
        let spans = self.spans.read().await;
        spans.get(span_id).cloned()
    }
    
    /// Get all active spans
    pub async fn get_active_spans(&self) -> Vec<SpanData> {
        let spans = self.spans.read().await;
        spans.values()
            .filter(|span| span.duration_ms.is_none())
            .cloned()
            .collect()
    }
    
    /// Get telemetry statistics
    pub async fn get_telemetry_stats(&self) -> TelemetryStats {
        let spans = self.spans.read().await;
        
        let total_spans = spans.len();
        let active_spans = spans.values()
            .filter(|span| span.duration_ms.is_none())
            .count();
        let completed_spans = total_spans - active_spans;
        
        let avg_duration = spans.values()
            .filter_map(|span| span.duration_ms)
            .sum::<u64>() as f64 / completed_spans.max(1) as f64;
        
        let operations = spans.values()
            .map(|span| span.operation_name.clone())
            .collect::<std::collections::HashSet<_>>();
        
        TelemetryStats {
            total_spans,
            active_spans,
            completed_spans,
            average_duration_ms: avg_duration,
            unique_operations: operations.len(),
        }
    }
    
    /// Clean up old spans
    pub async fn cleanup_old_spans(&self, max_age_hours: i64) {
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours);
        let mut spans = self.spans.write().await;
        
        let initial_count = spans.len();
        spans.retain(|_, span| span.start_time > cutoff);
        let removed = initial_count - spans.len();
        
        if removed > 0 {
            info!("Cleaned up {} old spans (older than {} hours)", removed, max_age_hours);
        }
    }
}

/// Initialize OpenTelemetry and tracing
pub async fn init_telemetry(config: &TelemetryConfig) -> Result<Box<dyn Tracer + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
    // Create resource with service information
    let resource = Resource::new(vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("service.version", config.service_version.clone()),
        KeyValue::new("environment", config.environment.clone()),
    ]);
    
    // Configure tracer
    let mut tracer_builder = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_resource(resource)
        .with_id_generator(RandomIdGenerator::default())
        .with_sampler(Sampler::TraceIdRatioBased(config.sampling_rate));
    
    // Add OTLP exporter if endpoint provided
    if let Some(endpoint) = &config.otlp_endpoint {
        let exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(endpoint);
        
        let batch_processor = trace::BatchSpanProcessor::builder(
            exporter.build_span_exporter()?,
            opentelemetry_sdk::runtime::Tokio,
        )
        .with_batch_config(
            trace::BatchConfig::default()
                .with_max_export_batch_size(config.max_export_batch_size)
                .with_max_export_timeout(std::time::Duration::from_secs(config.batch_timeout_seconds))
        )
        .build();
        
        tracer_builder = tracer_builder.with_span_processor(batch_processor);
    }
    
    let provider = tracer_builder.build();
    global::set_tracer_provider(provider.clone());
    
    let tracer = provider.tracer("solana-trading-bot");
    
    // Initialize tracing subscriber
    let mut layers = Vec::new();
    
    if config.enable_console {
        let console_layer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true);
        layers.push(console_layer.boxed());
    }
    
    // Add OpenTelemetry layer
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer.clone());
    layers.push(otel_layer.boxed());
    
    tracing_subscriber::registry()
        .with(layers)
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level))
        )
        .init();
    
    info!("Telemetry initialized with service: {}", config.service_name);
    
    Ok(Box::new(tracer))
}

/// Telemetry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryStats {
    pub total_spans: usize,
    pub active_spans: usize,
    pub completed_spans: usize,
    pub average_duration_ms: f64,
    pub unique_operations: usize,
}

/// Macro for convenient span creation
#[macro_export]
macro_rules! trace_span {
    ($telemetry:expr, $operation:expr, { $($key:expr => $value:expr),* }) => {{
        let span_id = $telemetry.start_span($operation).await;
        $(
            $telemetry.add_span_tag(&span_id, $key, $value).await;
        )*
        span_id
    }};
}

/// Macro for tracing function execution
#[macro_export]
macro_rules! trace_function {
    ($telemetry:expr, $func:expr) => {{
        let span_id = $telemetry.start_span(stringify!($func)).await;
        let result = $func.await;
        $telemetry.finish_span(&span_id).await;
        result
    }};
}