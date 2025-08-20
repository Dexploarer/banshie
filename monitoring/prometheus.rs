use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramOpts, HistogramVec, Opts, Registry,
};
use std::sync::Arc;
use tracing::{info, error};

use crate::errors::{BotError, Result};

/// Prometheus metrics collector for comprehensive monitoring
#[derive(Clone)]
pub struct PrometheusMetrics {
    registry: Arc<Registry>,
    
    // Application metrics
    pub http_requests_total: CounterVec,
    pub http_request_duration: HistogramVec,
    pub active_connections: Gauge,
    
    // Trading metrics
    pub trading_operations_total: CounterVec,
    pub trading_operations_failed: Counter,
    pub trading_slippage: HistogramVec,
    pub trading_volume_usd: CounterVec,
    pub trading_pnl_total: GaugeVec,
    pub active_positions: Gauge,
    
    // Jupiter API metrics
    pub jupiter_api_requests_total: CounterVec,
    pub jupiter_api_request_duration: HistogramVec,
    pub jupiter_api_errors: Counter,
    pub jupiter_api_rate_limit_remaining: Gauge,
    
    // Solana RPC metrics
    pub solana_rpc_requests_total: CounterVec,
    pub solana_rpc_request_duration: HistogramVec,
    pub solana_rpc_errors: Counter,
    pub solana_network_tps: Gauge,
    pub solana_slot_height: Gauge,
    
    // Database metrics
    pub database_queries_total: CounterVec,
    pub database_query_duration: HistogramVec,
    pub database_connections_active: Gauge,
    pub database_connections_idle: Gauge,
    
    // Cache metrics
    pub cache_requests_total: CounterVec,
    pub cache_hits_total: Counter,
    pub cache_misses_total: Counter,
    pub cache_size_bytes: Gauge,
    pub cache_evictions_total: Counter,
    
    // System metrics
    pub memory_usage_bytes: Gauge,
    pub cpu_usage_percent: Gauge,
    pub disk_usage_bytes: GaugeVec,
    pub network_bytes_total: CounterVec,
    
    // Business metrics
    pub users_total: Gauge,
    pub users_active_daily: Gauge,
    pub wallets_created_total: Counter,
    pub revenue_usd_total: Counter,
}

impl PrometheusMetrics {
    /// Create new Prometheus metrics collector
    pub fn new() -> Result<Self> {
        let registry = Arc::new(Registry::new());
        
        // Application metrics
        let http_requests_total = CounterVec::new(
            Opts::new(
                "http_requests_total",
                "Total number of HTTP requests"
            ),
            &["method", "status", "endpoint"]
        ).map_err(|e| BotError::config(format!("Failed to create http_requests_total metric: {}", e)))?;
        
        let http_request_duration = HistogramVec::new(
            HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request duration in seconds"
            ).buckets(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
            &["method", "endpoint"]
        ).map_err(|e| BotError::config(format!("Failed to create http_request_duration metric: {}", e)))?;
        
        let active_connections = Gauge::new(
            "active_connections",
            "Number of active connections"
        ).map_err(|e| BotError::config(format!("Failed to create active_connections metric: {}", e)))?;
        
        // Trading metrics
        let trading_operations_total = CounterVec::new(
            Opts::new(
                "trading_operations_total",
                "Total number of trading operations"
            ),
            &["operation_type", "token_pair", "status"]
        ).map_err(|e| BotError::config(format!("Failed to create trading_operations_total metric: {}", e)))?;
        
        let trading_operations_failed = Counter::new(
            "trading_operations_failed_total",
            "Total number of failed trading operations"
        ).map_err(|e| BotError::config(format!("Failed to create trading_operations_failed metric: {}", e)))?;
        
        let trading_slippage = HistogramVec::new(
            HistogramOpts::new(
                "trading_slippage_percent",
                "Trading slippage percentage"
            ).buckets(vec![0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0]),
            &["token_pair", "operation_type"]
        ).map_err(|e| BotError::config(format!("Failed to create trading_slippage metric: {}", e)))?;
        
        let trading_volume_usd = CounterVec::new(
            Opts::new(
                "trading_volume_usd_total",
                "Total trading volume in USD"
            ),
            &["token_pair", "operation_type"]
        ).map_err(|e| BotError::config(format!("Failed to create trading_volume_usd metric: {}", e)))?;
        
        let trading_pnl_total = GaugeVec::new(
            Opts::new(
                "trading_pnl_usd_total",
                "Total profit and loss in USD"
            ),
            &["token_pair", "user_id"]
        ).map_err(|e| BotError::config(format!("Failed to create trading_pnl_total metric: {}", e)))?;
        
        let active_positions = Gauge::new(
            "active_positions_total",
            "Number of active trading positions"
        ).map_err(|e| BotError::config(format!("Failed to create active_positions metric: {}", e)))?;
        
        // Jupiter API metrics
        let jupiter_api_requests_total = CounterVec::new(
            Opts::new(
                "jupiter_api_requests_total",
                "Total Jupiter API requests"
            ),
            &["endpoint", "status"]
        ).map_err(|e| BotError::config(format!("Failed to create jupiter_api_requests_total metric: {}", e)))?;
        
        let jupiter_api_request_duration = HistogramVec::new(
            HistogramOpts::new(
                "jupiter_api_request_duration_seconds",
                "Jupiter API request duration"
            ).buckets(vec![0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0]),
            &["endpoint"]
        ).map_err(|e| BotError::config(format!("Failed to create jupiter_api_request_duration metric: {}", e)))?;
        
        let jupiter_api_errors = Counter::new(
            "jupiter_api_errors_total",
            "Total Jupiter API errors"
        ).map_err(|e| BotError::config(format!("Failed to create jupiter_api_errors metric: {}", e)))?;
        
        let jupiter_api_rate_limit_remaining = Gauge::new(
            "jupiter_api_rate_limit_remaining",
            "Jupiter API rate limit remaining"
        ).map_err(|e| BotError::config(format!("Failed to create jupiter_api_rate_limit_remaining metric: {}", e)))?;
        
        // Solana RPC metrics
        let solana_rpc_requests_total = CounterVec::new(
            Opts::new(
                "solana_rpc_requests_total",
                "Total Solana RPC requests"
            ),
            &["method", "status"]
        ).map_err(|e| BotError::config(format!("Failed to create solana_rpc_requests_total metric: {}", e)))?;
        
        let solana_rpc_request_duration = HistogramVec::new(
            HistogramOpts::new(
                "solana_rpc_request_duration_seconds",
                "Solana RPC request duration"
            ).buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0]),
            &["method"]
        ).map_err(|e| BotError::config(format!("Failed to create solana_rpc_request_duration metric: {}", e)))?;
        
        let solana_rpc_errors = Counter::new(
            "solana_rpc_errors_total",
            "Total Solana RPC errors"
        ).map_err(|e| BotError::config(format!("Failed to create solana_rpc_errors metric: {}", e)))?;
        
        let solana_network_tps = Gauge::new(
            "solana_network_tps",
            "Solana network transactions per second"
        ).map_err(|e| BotError::config(format!("Failed to create solana_network_tps metric: {}", e)))?;
        
        let solana_slot_height = Gauge::new(
            "solana_slot_height",
            "Current Solana slot height"
        ).map_err(|e| BotError::config(format!("Failed to create solana_slot_height metric: {}", e)))?;
        
        // Database metrics
        let database_queries_total = CounterVec::new(
            Opts::new(
                "database_queries_total",
                "Total database queries"
            ),
            &["operation", "table"]
        ).map_err(|e| BotError::config(format!("Failed to create database_queries_total metric: {}", e)))?;
        
        let database_query_duration = HistogramVec::new(
            HistogramOpts::new(
                "database_query_duration_seconds",
                "Database query duration"
            ).buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]),
            &["operation", "table"]
        ).map_err(|e| BotError::config(format!("Failed to create database_query_duration metric: {}", e)))?;
        
        let database_connections_active = Gauge::new(
            "database_connections_active",
            "Active database connections"
        ).map_err(|e| BotError::config(format!("Failed to create database_connections_active metric: {}", e)))?;
        
        let database_connections_idle = Gauge::new(
            "database_connections_idle",
            "Idle database connections"
        ).map_err(|e| BotError::config(format!("Failed to create database_connections_idle metric: {}", e)))?;
        
        // Cache metrics
        let cache_requests_total = CounterVec::new(
            Opts::new(
                "cache_requests_total",
                "Total cache requests"
            ),
            &["cache_type", "operation"]
        ).map_err(|e| BotError::config(format!("Failed to create cache_requests_total metric: {}", e)))?;
        
        let cache_hits_total = Counter::new(
            "cache_hits_total",
            "Total cache hits"
        ).map_err(|e| BotError::config(format!("Failed to create cache_hits_total metric: {}", e)))?;
        
        let cache_misses_total = Counter::new(
            "cache_misses_total",
            "Total cache misses"
        ).map_err(|e| BotError::config(format!("Failed to create cache_misses_total metric: {}", e)))?;
        
        let cache_size_bytes = Gauge::new(
            "cache_size_bytes",
            "Cache size in bytes"
        ).map_err(|e| BotError::config(format!("Failed to create cache_size_bytes metric: {}", e)))?;
        
        let cache_evictions_total = Counter::new(
            "cache_evictions_total",
            "Total cache evictions"
        ).map_err(|e| BotError::config(format!("Failed to create cache_evictions_total metric: {}", e)))?;
        
        // System metrics
        let memory_usage_bytes = Gauge::new(
            "memory_usage_bytes",
            "Memory usage in bytes"
        ).map_err(|e| BotError::config(format!("Failed to create memory_usage_bytes metric: {}", e)))?;
        
        let cpu_usage_percent = Gauge::new(
            "cpu_usage_percent",
            "CPU usage percentage"
        ).map_err(|e| BotError::config(format!("Failed to create cpu_usage_percent metric: {}", e)))?;
        
        let disk_usage_bytes = GaugeVec::new(
            Opts::new(
                "disk_usage_bytes",
                "Disk usage in bytes"
            ),
            &["mountpoint"]
        ).map_err(|e| BotError::config(format!("Failed to create disk_usage_bytes metric: {}", e)))?;
        
        let network_bytes_total = CounterVec::new(
            Opts::new(
                "network_bytes_total",
                "Total network bytes"
            ),
            &["direction", "interface"]
        ).map_err(|e| BotError::config(format!("Failed to create network_bytes_total metric: {}", e)))?;
        
        // Business metrics
        let users_total = Gauge::new(
            "users_total",
            "Total number of users"
        ).map_err(|e| BotError::config(format!("Failed to create users_total metric: {}", e)))?;
        
        let users_active_daily = Gauge::new(
            "users_active_daily",
            "Daily active users"
        ).map_err(|e| BotError::config(format!("Failed to create users_active_daily metric: {}", e)))?;
        
        let wallets_created_total = Counter::new(
            "wallets_created_total",
            "Total wallets created"
        ).map_err(|e| BotError::config(format!("Failed to create wallets_created_total metric: {}", e)))?;
        
        let revenue_usd_total = Counter::new(
            "revenue_usd_total",
            "Total revenue in USD"
        ).map_err(|e| BotError::config(format!("Failed to create revenue_usd_total metric: {}", e)))?;
        
        // Register all metrics
        registry.register(Box::new(http_requests_total.clone()))?;
        registry.register(Box::new(http_request_duration.clone()))?;
        registry.register(Box::new(active_connections.clone()))?;
        
        registry.register(Box::new(trading_operations_total.clone()))?;
        registry.register(Box::new(trading_operations_failed.clone()))?;
        registry.register(Box::new(trading_slippage.clone()))?;
        registry.register(Box::new(trading_volume_usd.clone()))?;
        registry.register(Box::new(trading_pnl_total.clone()))?;
        registry.register(Box::new(active_positions.clone()))?;
        
        registry.register(Box::new(jupiter_api_requests_total.clone()))?;
        registry.register(Box::new(jupiter_api_request_duration.clone()))?;
        registry.register(Box::new(jupiter_api_errors.clone()))?;
        registry.register(Box::new(jupiter_api_rate_limit_remaining.clone()))?;
        
        registry.register(Box::new(solana_rpc_requests_total.clone()))?;
        registry.register(Box::new(solana_rpc_request_duration.clone()))?;
        registry.register(Box::new(solana_rpc_errors.clone()))?;
        registry.register(Box::new(solana_network_tps.clone()))?;
        registry.register(Box::new(solana_slot_height.clone()))?;
        
        registry.register(Box::new(database_queries_total.clone()))?;
        registry.register(Box::new(database_query_duration.clone()))?;
        registry.register(Box::new(database_connections_active.clone()))?;
        registry.register(Box::new(database_connections_idle.clone()))?;
        
        registry.register(Box::new(cache_requests_total.clone()))?;
        registry.register(Box::new(cache_hits_total.clone()))?;
        registry.register(Box::new(cache_misses_total.clone()))?;
        registry.register(Box::new(cache_size_bytes.clone()))?;
        registry.register(Box::new(cache_evictions_total.clone()))?;
        
        registry.register(Box::new(memory_usage_bytes.clone()))?;
        registry.register(Box::new(cpu_usage_percent.clone()))?;
        registry.register(Box::new(disk_usage_bytes.clone()))?;
        registry.register(Box::new(network_bytes_total.clone()))?;
        
        registry.register(Box::new(users_total.clone()))?;
        registry.register(Box::new(users_active_daily.clone()))?;
        registry.register(Box::new(wallets_created_total.clone()))?;
        registry.register(Box::new(revenue_usd_total.clone()))?;
        
        info!("📊 Prometheus metrics initialized with {} collectors", registry.metric_families().len());
        
        Ok(Self {
            registry,
            http_requests_total,
            http_request_duration,
            active_connections,
            trading_operations_total,
            trading_operations_failed,
            trading_slippage,
            trading_volume_usd,
            trading_pnl_total,
            active_positions,
            jupiter_api_requests_total,
            jupiter_api_request_duration,
            jupiter_api_errors,
            jupiter_api_rate_limit_remaining,
            solana_rpc_requests_total,
            solana_rpc_request_duration,
            solana_rpc_errors,
            solana_network_tps,
            solana_slot_height,
            database_queries_total,
            database_query_duration,
            database_connections_active,
            database_connections_idle,
            cache_requests_total,
            cache_hits_total,
            cache_misses_total,
            cache_size_bytes,
            cache_evictions_total,
            memory_usage_bytes,
            cpu_usage_percent,
            disk_usage_bytes,
            network_bytes_total,
            users_total,
            users_active_daily,
            wallets_created_total,
            revenue_usd_total,
        })
    }
    
    /// Get the registry for serving metrics
    pub fn registry(&self) -> Arc<Registry> {
        self.registry.clone()
    }
    
    /// Record HTTP request metrics
    pub fn record_http_request(&self, method: &str, status: u16, endpoint: &str, duration: f64) {
        self.http_requests_total
            .with_label_values(&[method, &status.to_string(), endpoint])
            .inc();
        self.http_request_duration
            .with_label_values(&[method, endpoint])
            .observe(duration);
    }
    
    /// Record trading operation metrics
    pub fn record_trading_operation(&self, operation_type: &str, token_pair: &str, success: bool, slippage: f64, volume_usd: f64) {
        let status = if success { "success" } else { "failed" };
        self.trading_operations_total
            .with_label_values(&[operation_type, token_pair, status])
            .inc();
        
        if !success {
            self.trading_operations_failed.inc();
        }
        
        self.trading_slippage
            .with_label_values(&[token_pair, operation_type])
            .observe(slippage);
        
        self.trading_volume_usd
            .with_label_values(&[token_pair, operation_type])
            .inc_by(volume_usd);
    }
    
    /// Update cache hit rate
    pub fn record_cache_operation(&self, cache_type: &str, hit: bool) {
        let operation = if hit { "hit" } else { "miss" };
        self.cache_requests_total
            .with_label_values(&[cache_type, operation])
            .inc();
        
        if hit {
            self.cache_hits_total.inc();
        } else {
            self.cache_misses_total.inc();
        }
    }
    
    /// Calculate and return cache hit rate
    pub fn get_cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits_total.get();
        let misses = self.cache_misses_total.get();
        let total = hits + misses;
        
        if total > 0.0 {
            hits / total
        } else {
            0.0
        }
    }
}